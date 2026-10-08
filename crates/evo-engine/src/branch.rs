//! Branches « avec et sans » (document Fonctionnalités, « Interventions » et
//! « Sauvegarde, branches et partage » ; porte de l'étape 4).
//!
//! Après une intervention, le joueur demande « et sans mon intervention ? ».
//! Le moteur reprend le point de sauvegarde écrit juste avant, lui redonne
//! tous les ordres reçus depuis sauf ceux que le joueur retire, et le fait
//! avancer en tâche de fond, sur la même graine, jusqu'à la date de la
//! partie principale. Le déterminisme fait le reste : sans ordre retiré, la
//! branche refait exactement l'histoire principale (test plus bas).
//!
//! La branche tourne sur son propre fil et son propre groupe de fils de
//! calcul. Deux allures, au choix du joueur : en même temps que la partie
//! (qui ralentit d'autant, les cœurs étant partagés) ou seulement pendant
//! les pauses. Elle n'écrit jamais dans la partie principale.

use evo_sim::history::{PublishedState, Sample};
use evo_sim::orders::Order;
use evo_sim::World;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Ce que demande une branche.
#[derive(Clone, Debug, PartialEq)]
pub struct BranchSpec {
    /// Point de sauvegarde d'où elle part.
    pub save: PathBuf,
    /// Ordres de la partie principale à retirer (identifiants).
    pub without: Vec<u64>,
    /// Fils de calcul de la branche.
    pub threads: usize,
}

/// Où en est la branche.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BranchStatus {
    /// Date atteinte, années (NaN tant que l'état n'est pas lu).
    pub years: f64,
    /// Date de départ (celle du point de sauvegarde).
    pub start_years: f64,
    /// Date à atteindre (celle de la partie principale).
    pub target_years: f64,
    pub steps: u64,
    pub running: bool,
    pub error: Option<String>,
}

impl BranchStatus {
    /// Avancement de 0 à 1.
    pub fn progress(&self) -> f64 {
        let span = self.target_years - self.start_years;
        if !self.years.is_finite() {
            return 0.0;
        }
        if span <= 0.0 {
            return 1.0;
        }
        ((self.years - self.start_years) / span).clamp(0.0, 1.0)
    }

    /// Vrai quand la branche a rattrapé la partie principale.
    pub fn caught_up(&self) -> bool {
        self.years.is_finite() && self.years >= self.target_years
    }
}

struct Shared {
    status: Mutex<BranchStatus>,
    state: Mutex<Option<Arc<PublishedState>>>,
    history: Mutex<Vec<Sample>>,
    /// Ordres de la partie principale reçus depuis le départ.
    orders: Mutex<Vec<Order>>,
    allowed: AtomicBool,
    stop: AtomicBool,
}

/// Une branche qui tourne en tâche de fond.
pub struct Branch {
    pub spec: BranchSpec,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Branch {
    /// Lance une branche. `orders` est le registre complet de la partie
    /// principale ; seuls les ordres reçus après le point de sauvegarde
    /// sont rejoués, moins ceux de `spec.without`.
    pub fn start(spec: BranchSpec, orders: Vec<Order>, target_years: f64) -> io::Result<Branch> {
        let shared = Arc::new(Shared {
            status: Mutex::new(BranchStatus { years: f64::NAN, target_years, running: true, ..Default::default() }),
            state: Mutex::new(None),
            history: Mutex::new(Vec::new()),
            orders: Mutex::new(orders),
            allowed: AtomicBool::new(true),
            stop: AtomicBool::new(false),
        });
        let sh = shared.clone();
        let s = spec.clone();
        let thread = std::thread::Builder::new().name("evonisium-branche".into()).spawn(move || {
            if let Err(e) = run(&s, &sh) {
                sh.status.lock().unwrap_or_else(|p| p.into_inner()).error = Some(e.to_string());
            }
            sh.status.lock().unwrap_or_else(|p| p.into_inner()).running = false;
        })?;
        Ok(Branch { spec, shared, thread: Some(thread) })
    }

    pub fn status(&self) -> BranchStatus {
        self.shared.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Dernier état publié par la branche.
    pub fn state(&self) -> Option<Arc<PublishedState>> {
        self.shared.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Historique échantillonné de la branche.
    pub fn history(&self) -> Vec<Sample> {
        self.shared.history.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Date de la partie principale à rattraper, et ses ordres reçus depuis
    /// (le joueur a pu agir à nouveau) : la branche les rejoue aussi.
    pub fn follow(&self, target_years: f64, orders: Option<Vec<Order>>) {
        self.shared.status.lock().unwrap_or_else(|e| e.into_inner()).target_years = target_years;
        if let Some(o) = orders {
            *self.shared.orders.lock().unwrap_or_else(|e| e.into_inner()) = o;
        }
    }

    /// Autorise ou suspend l'avance (allure « pendant les pauses »).
    pub fn set_allowed(&self, allowed: bool) {
        self.shared.allowed.store(allowed, Ordering::Relaxed);
    }
}

impl Drop for Branch {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Ordres de la partie principale qu'une branche partie de `world` doit
/// recevoir : ceux que le point de sauvegarde ne connaissait pas, moins les
/// ordres retirés.
pub fn orders_for_branch(world: &World, main_log: &[Order], without: &[u64]) -> Vec<Order> {
    let known = world.orders.next_id();
    main_log.iter().filter(|o| o.id >= known && !without.contains(&o.id)).cloned().collect()
}

/// Prépare un monde de branche : il ne connaît plus les ordres retirés
/// (s'ils attendaient encore dans sa file) et n'est jamais en pause.
pub fn prepare(world: &mut World, without: &[u64]) {
    world.orders.withdraw(without);
    world.paused = false;
}

/// Avance un monde de branche d'un pas en ignorant les pauses de la partie
/// principale : une pause appliquée n'arrête que l'horloge de celle-ci.
pub fn advance(world: &mut World) {
    let before = world.stats.steps;
    world.step();
    if world.paused {
        world.paused = false;
        if world.stats.steps == before {
            // Le pas n'a fait qu'appliquer la pause : on le refait.
            world.step();
            world.paused = false;
        }
    }
}

fn run(spec: &BranchSpec, sh: &Shared) -> io::Result<()> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(spec.threads.max(1))
        .thread_name(|i| format!("evonisium-branche-{i}"))
        .build()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let mut world = pool.install(|| World::load_file(&spec.save))?;
    prepare(&mut world, &spec.without);
    let mut fed = 0usize;
    {
        let mut st = sh.status.lock().unwrap_or_else(|e| e.into_inner());
        st.start_years = world.years();
        st.years = world.years();
        st.steps = world.stats.steps;
    }
    publish(&mut world, sh);
    loop {
        if sh.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        // Nouveaux ordres de la partie principale.
        {
            let all = sh.orders.lock().unwrap_or_else(|e| e.into_inner());
            let fresh = orders_for_branch(&world, &all[fed.min(all.len())..], &spec.without);
            fed = all.len();
            drop(all);
            for o in fresh {
                world.orders.submit_with_id(o.id, o.due_years, o.kind);
            }
        }
        let target = sh.status.lock().unwrap_or_else(|e| e.into_inner()).target_years;
        if !sh.allowed.load(Ordering::Relaxed) || world.years() >= target {
            std::thread::sleep(Duration::from_millis(30));
            continue;
        }
        pool.install(|| advance(&mut world));
        publish(&mut world, sh);
    }
}

fn publish(world: &mut World, sh: &Shared) {
    if world.publication.current.is_none() {
        world.republish();
    }
    *sh.state.lock().unwrap_or_else(|e| e.into_inner()) = world.publication.current.clone();
    *sh.history.lock().unwrap_or_else(|e| e.into_inner()) = world.history.samples.clone();
    let mut st = sh.status.lock().unwrap_or_else(|e| e.into_inner());
    st.years = world.years();
    st.steps = world.stats.steps;
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_sim::orders::{Intervention, OrderKind};
    use evo_sim::WorldConfig;

    fn config() -> WorldConfig {
        let mut cfg = WorldConfig::new(17, 3);
        cfg.bio_level = 3;
        cfg.step_years = 2000.0;
        cfg.influence.sandbox = true;
        cfg
    }

    #[test]
    fn a_branch_without_removed_orders_replays_the_main_history() {
        let dir = std::env::temp_dir().join(format!("evo-branche-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("avant.evo");
        let mut main = World::new(config());
        main.orders.submit(0.0, OrderKind::SeedLife);
        for _ in 0..10 {
            main.step();
        }
        main.save_file(&save).unwrap();
        let t = main.years();
        // Après le point de sauvegarde : une pause, une reprise, une
        // intervention, un changement de pas.
        main.orders.submit(t, OrderKind::Pause);
        main.step();
        main.orders.submit(t, OrderKind::Resume);
        let impact = main.orders.submit(t, OrderKind::Intervene(Intervention::Impact { cell: 3, diameter_km: 20.0 }));
        main.orders.submit(t + 6000.0, OrderKind::SetStepYears(3000.0));
        while main.years() < t + 30_000.0 {
            main.step();
        }

        // Même histoire sans rien retirer.
        let mut same = World::load_file(&save).unwrap();
        prepare(&mut same, &[]);
        for o in orders_for_branch(&same, &main.orders.log(), &[]) {
            same.orders.submit_with_id(o.id, o.due_years, o.kind);
        }
        while same.years() < main.years() {
            advance(&mut same);
        }
        assert_eq!(same.years(), main.years());
        same.paused = main.paused;
        assert_eq!(same.state_hash(), main.state_hash(), "la branche refait l'histoire");

        // Sans l'impact, l'histoire diffère, et la branche en tâche de fond
        // rattrape la date demandée.
        let b =
            Branch::start(BranchSpec { save: save.clone(), without: vec![impact], threads: 1 }, main.orders.log(), main.years()).unwrap();
        for _ in 0..600 {
            if b.status().caught_up() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let st = b.status();
        assert!(st.caught_up(), "{st:?}");
        assert_eq!(st.progress(), 1.0);
        let state = b.state().unwrap();
        assert_eq!(state.years, main.years());
        assert!(state.disturbances.killed_biomass == 0.0);
        assert!(main.disturbances.killed_biomass > 0.0);
        assert!(!b.history().is_empty());
        drop(b);
        let _ = std::fs::remove_dir_all(dir);
    }
}
