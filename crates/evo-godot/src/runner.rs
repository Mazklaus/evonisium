//! Le moteur sur son propre fil.
//!
//! Le monde avance sur un fil dédié, avec un groupe de fils de calcul qui
//! laisse deux cœurs à l'affichage (architecture : 2 cœurs sur 8 réservés).
//! Après chaque pas, il publie une image figée (`Frame`) que le fil de rendu
//! lit sans jamais attendre la simulation. Le client n'écrit que par deux
//! canaux : la file d'ordres (qui change l'histoire et entre dans le rejeu)
//! et le canal d'observation (qui ne la change jamais).
//!
//! [Simplification] Adaptateur provisoire sur l'API de l'étape 2 : quand le
//! volet moteur de l'étape 3 publiera son service d'exécution (double
//! tampon, points de sauvegarde, réserve d'influence), ce module se réduira
//! à un appel de ce service.

use evo_core::events::{Event, EventKind, Origin};
use evo_sim::{OrderKind, Sample, Seeding, World, WorldConfig};
use evo_view::chronicle::{Action, StopRules};
use evo_view::frame::{lineages_of, Frame, LineageFrame};
use evo_view::save::{PlanetSpec, SaveFile};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Zone d'intérêt de la caméra (canal d'observation).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Observation {
    pub centre: [f64; 3],
    pub radius_rad: f64,
    /// Bande de zoom, 1 (orbite) à 6 (loupe).
    pub band: u8,
}

pub enum Command {
    /// Ordre pour la file (daté au début du prochain pas).
    Order(OrderKind),
    /// Vitesse visée, années de jeu par seconde réelle (cadence du fil, pas
    /// l'histoire : la durée du pas passe, elle, par un ordre).
    Pace(f64),
    Observe(Observation),
    SetSeeding(Seeding),
    Save {
        path: String,
        name: String,
    },
    Shutdown,
}

/// Ce que le fil de la simulation partage avec le client.
#[derive(Default)]
pub struct Shared {
    pub current: Mutex<Option<Arc<Frame>>>,
    pub previous: Mutex<Option<Arc<Frame>>>,
    /// Journal complet des événements, dans l'ordre.
    pub events: Mutex<Vec<Event>>,
    pub lineages: Mutex<Arc<Vec<LineageFrame>>>,
    pub history: Mutex<Vec<Sample>>,
    pub rules: Mutex<Option<StopRules>>,
    /// Messages pour le client (sauvegarde écrite, erreur).
    pub notices: Mutex<VecDeque<String>>,
    /// Vitesse réellement tenue, années par seconde (bits d'un f64).
    pub real_speed: AtomicU64,
    /// Rejeu d'un point de sauvegarde en cours : pas atteints et visés.
    pub loading: Mutex<Option<(u64, u64)>>,
    pub alive: AtomicBool,
    pub observation: Mutex<Observation>,
    /// Événement qui a provoqué la dernière pause automatique.
    pub auto_paused_by: Mutex<Option<u64>>,
}

impl Shared {
    pub fn real_speed(&self) -> f64 {
        f64::from_bits(self.real_speed.load(Ordering::Relaxed))
    }
}

/// Une partie en cours.
pub struct Runner {
    pub tx: Sender<Command>,
    pub shared: Arc<Shared>,
    pub spec: PlanetSpec,
    pub seed: u64,
    pub level: u32,
    handle: Option<JoinHandle<()>>,
}

/// Vitesse de départ : 100 ka/s.
pub const DEFAULT_SPEED: f64 = 1e5;
/// Pas par seconde visés : la durée du pas est vitesse / 10.
pub const STEPS_PER_SECOND: f64 = 10.0;

/// Durée de pas pour une vitesse (bornes du monde microbien).
pub fn step_years_for(speed: f64) -> f64 {
    (speed / STEPS_PER_SECOND).clamp(100.0, 130_000.0)
}

impl Runner {
    /// Nouvelle partie, en pause, sans vie.
    pub fn start(spec: PlanetSpec, seed: u64, level: u32, seeding: Seeding) -> Runner {
        let mut config = WorldConfig::with_planet(spec.to_params(), seed, level);
        config.seeding = seeding;
        Self::spawn(
            spec,
            seed,
            level,
            move || {
                let mut w = World::new(config);
                // La vitesse de départ est un ordre comme un autre : le rejeu la
                // retrouve dans le registre.
                w.orders.submit(0.0, OrderKind::SetStepYears(step_years_for(DEFAULT_SPEED)));
                w.orders.submit(0.0, OrderKind::Pause);
                w.step();
                w
            },
            None,
        )
    }

    /// Reprise d'un point de sauvegarde : rejeu jusqu'au pas sauvegardé.
    pub fn load(save: SaveFile) -> Runner {
        let target = save.steps;
        let mut config = save.config();
        config.seeding = if save.seeding == "mers" { Seeding::AllOcean } else { Seeding::Vents };
        let orders = save.orders.clone();
        Self::spawn(save.spec.clone(), save.seed, save.level, move || World::replay(config, &orders), Some(target))
    }

    fn spawn(spec: PlanetSpec, seed: u64, level: u32, make: impl FnOnce() -> World + Send + 'static, replay_to: Option<u64>) -> Runner {
        let (tx, rx) = channel();
        let shared = Arc::new(Shared::default());
        shared.alive.store(true, Ordering::SeqCst);
        *shared.rules.lock().unwrap() = Some(StopRules::profile("naturaliste"));
        if let Some(t) = replay_to {
            *shared.loading.lock().unwrap() = Some((0, t));
        }
        let sh = shared.clone();
        let spec_run = spec.clone();
        let handle = std::thread::Builder::new()
            .name("evonisium-moteur".into())
            .spawn(move || {
                let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
                // Deux cœurs pour l'affichage, au moins un pour le moteur.
                let pool = rayon::ThreadPoolBuilder::new().num_threads(cores.saturating_sub(2).max(1)).build().expect("fils de calcul");
                pool.install(|| run(make, replay_to, spec_run, rx, sh));
            })
            .expect("fil du moteur");
        Runner { tx, shared, spec, seed, level, handle: Some(handle) }
    }

    pub fn send(&self, c: Command) {
        let _ = self.tx.send(c);
    }

    pub fn frame(&self) -> Option<Arc<Frame>> {
        self.shared.current.lock().unwrap().clone()
    }

    pub fn previous(&self) -> Option<Arc<Frame>> {
        self.shared.previous.lock().unwrap().clone()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

struct State {
    pace: f64,
    seeding_label: &'static str,
    events_seen: usize,
    lineages_refreshed_at: u64,
    speed_window: VecDeque<(Instant, f64)>,
}

fn publish(world: &World, shared: &Shared, st: &mut State) {
    let frame = Arc::new(Frame::from_world(world));
    {
        let mut cur = shared.current.lock().unwrap();
        let old = cur.replace(frame);
        *shared.previous.lock().unwrap() = old;
    }
    let new_events = &world.events.events[st.events_seen..];
    if !new_events.is_empty() {
        shared.events.lock().unwrap().extend_from_slice(new_events);
    }
    st.events_seen = world.events.events.len();
    let n = world.lineages.records.len() as u64;
    // L'arbre du vivant n'a pas besoin de chaque pas quand les lignées se
    // comptent par centaines de milliers.
    let every = if n > 200_000 { 20 } else { 1 };
    if world.stats.steps >= st.lineages_refreshed_at + every || world.paused {
        *shared.lineages.lock().unwrap() = Arc::new(lineages_of(world));
        st.lineages_refreshed_at = world.stats.steps;
    }
    let mut h = shared.history.lock().unwrap();
    let have = h.len();
    if have < world.history.samples.len() {
        h.extend_from_slice(&world.history.samples[have..]);
    }
}

fn save_text(world: &World, spec: &PlanetSpec, name: &str, seeding: &str) -> String {
    SaveFile {
        name: name.into(),
        engine_version: env!("CARGO_PKG_VERSION").into(),
        spec: spec.clone(),
        seed: world.config.seed,
        level: world.config.level,
        seeding: seeding.into(),
        steps: world.stats.steps,
        years: world.years,
        mode: "observateur".into(),
        orders: world.orders.log(),
    }
    .to_text()
}

/// Suite à donner après une commande.
enum Flow {
    Continue,
    Stop,
}

fn handle(c: Command, world: &mut World, shared: &Shared, st: &mut State, spec: &PlanetSpec) -> Flow {
    match c {
        Command::Shutdown => {
            shared.alive.store(false, Ordering::SeqCst);
            return Flow::Stop;
        }
        Command::Order(kind) => {
            world.orders.submit(world.years, kind);
        }
        Command::Pace(p) => st.pace = p.max(1.0),
        Command::Observe(o) => *shared.observation.lock().unwrap() = o,
        Command::SetSeeding(s) => {
            // Seulement avant le dépôt des premières cellules.
            if world.lineages.records.is_empty() {
                world.config.seeding = s;
                st.seeding_label = if s == Seeding::AllOcean { "mers" } else { "sources" };
            }
        }
        Command::Save { path, name } => {
            let text = save_text(world, spec, &name, st.seeding_label);
            let msg = match std::fs::write(&path, text) {
                Ok(()) => format!("ok\t{path}"),
                Err(e) => format!("erreur\t{path}\t{e}"),
            };
            shared.notices.lock().unwrap().push_back(msg);
        }
    }
    Flow::Continue
}

fn run(make: impl FnOnce() -> World, replay_to: Option<u64>, spec: PlanetSpec, rx: Receiver<Command>, shared: Arc<Shared>) {
    let mut world = make();
    let mut st = State {
        pace: DEFAULT_SPEED,
        seeding_label: if world.config.seeding == Seeding::AllOcean { "mers" } else { "sources" },
        events_seen: 0,
        lineages_refreshed_at: 0,
        speed_window: VecDeque::new(),
    };
    // Rejeu d'un point de sauvegarde : au plus vite, sans cadence.
    if let Some(target) = replay_to {
        while world.stats.steps < target {
            world.step();
            *shared.loading.lock().unwrap() = Some((world.stats.steps, target));
            if let Ok(Command::Shutdown) = rx.try_recv() {
                shared.alive.store(false, Ordering::SeqCst);
                return;
            }
        }
        *shared.loading.lock().unwrap() = None;
        // On reprend en pause : le joueur relance quand il veut.
        if !world.paused {
            world.orders.submit(world.years, OrderKind::Pause);
            world.step();
        }
    }
    publish(&world, &shared, &mut st);
    loop {
        // 1. Les commandes : sans attendre si le monde avance, avec une
        //    attente courte s'il est en pause.
        if world.paused && world.orders.pending().is_empty() {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(c) => {
                    if let Flow::Stop = handle(c, &mut world, &shared, &mut st, &spec) {
                        return;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        while let Ok(c) = rx.try_recv() {
            if let Flow::Stop = handle(c, &mut world, &shared, &mut st, &spec) {
                return;
            }
        }
        if world.paused && world.orders.pending().is_empty() {
            st.speed_window.clear();
            shared.real_speed.store(0f64.to_bits(), Ordering::Relaxed);
            continue;
        }
        // 2. Un pas (en pause, il n'applique que les ordres dus).
        let t0 = Instant::now();
        let was_paused = world.paused;
        let before = world.years;
        world.step();
        // 3. Règles d'arrêt : le moteur se met lui-même en pause sur un
        //    événement qui le demande, par un ordre, donc dans le rejeu.
        if !world.paused {
            let rules = shared.rules.lock().unwrap().clone();
            if let Some(rules) = rules {
                let new = &world.events.events[st.events_seen..];
                if let Some(e) = new.iter().find(|e| e.origin != Origin::Player && rules.decide(e, st.pace) == Action::Pause) {
                    *shared.auto_paused_by.lock().unwrap() = Some(e.id);
                    world.orders.submit(world.years, OrderKind::Pause);
                    world.step();
                }
            }
        }
        publish(&world, &shared, &mut st);
        // 4. Cadence : un pas de `step_years` doit durer step_years / pace.
        let advanced = world.years - before;
        if !was_paused && advanced > 0.0 {
            let target = Duration::from_secs_f64((advanced / st.pace).min(2.0));
            let spent = t0.elapsed();
            if spent < target {
                // Attente interruptible : une commande réveille le fil.
                match rx.recv_timeout(target - spent) {
                    Ok(c) => {
                        if let Flow::Stop = handle(c, &mut world, &shared, &mut st, &spec) {
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
            st.speed_window.push_back((Instant::now(), advanced));
            while st.speed_window.len() > 20 {
                st.speed_window.pop_front();
            }
            if st.speed_window.len() > 1 {
                let dt = st.speed_window.back().unwrap().0.duration_since(st.speed_window.front().unwrap().0).as_secs_f64();
                let years: f64 = st.speed_window.iter().skip(1).map(|x| x.1).sum();
                if dt > 0.0 {
                    shared.real_speed.store((years / dt).to_bits(), Ordering::Relaxed);
                }
            }
        }
    }
}

/// Nombre d'événements d'un type, pour les tests.
pub fn count_kind(events: &[Event], pred: impl Fn(&EventKind) -> bool) -> usize {
    events.iter().filter(|e| pred(&e.kind)).count()
}
