//! La partie côté client : le moteur (`evo-engine`) et ce que le client
//! garde de lui.
//!
//! Le moteur tourne sur son propre fil et publie un état figé à chaque pas.
//! Le client n'écrit que par deux canaux : la file d'ordres (qui change
//! l'histoire) et la zone d'intérêt (qui ne la change jamais). Tout le reste
//! passe par des requêtes asynchrones : ce module les relance et relève leurs
//! réponses sans jamais attendre (une fois par image, depuis `pump`).

use evo_core::events::{Event, Origin};
use evo_engine::{Answer, CellDetail, Engine, InterestZone, NewGame, OrderKind, Query, Sample, When};
use evo_view::chronicle::{self, Action, StopRules};
use evo_view::frame::{LineageFrame, PlanetInfo};
use evo_view::save::{seeding_of, PlanetSpec, SaveFile, META_SUFFIX};
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Vitesse de départ : 100 ka/s.
pub const DEFAULT_SPEED: f64 = 1e5;
/// Pas par seconde visés : la durée du pas est vitesse / 10.
pub const STEPS_PER_SECOND: f64 = 10.0;
/// Événements relevés par requête.
const EVENTS_PER_POLL: usize = 500;

/// Durée de pas pour une vitesse (bornes du monde microbien).
pub fn step_years_for(speed: f64) -> f64 {
    (speed / STEPS_PER_SECOND).clamp(100.0, 130_000.0)
}

/// Fils de calcul du moteur : deux cœurs restent à l'affichage
/// (architecture : 2 cœurs sur 8 réservés).
pub fn engine_threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get()).saturating_sub(2).max(1)
}

pub fn planet_info(spec: &PlanetSpec, seed: u64, level: u32) -> Arc<PlanetInfo> {
    let p = spec.to_params();
    Arc::new(PlanetInfo { name: p.name.clone(), seed, level, radius_m: p.radius_m, star_temperature_k: p.star_temperature_k })
}

fn meta_path(save: &Path) -> PathBuf {
    let mut s = save.as_os_str().to_owned();
    s.push(META_SUFFIX);
    PathBuf::from(s)
}

/// Fiche d'un point de sauvegarde, sans ouvrir l'état.
pub fn read_meta(save: &Path) -> Result<SaveFile, String> {
    let text = std::fs::read_to_string(meta_path(save)).map_err(|e| format!("fiche illisible : {e}"))?;
    SaveFile::from_text(&text)
}

/// Une réponse attendue du moteur.
struct Pending<T> {
    rx: Option<Receiver<Answer>>,
    tag: T,
}

impl<T: Copy> Pending<T> {
    fn new(tag: T) -> Self {
        Self { rx: None, tag }
    }

    fn idle(&self) -> bool {
        self.rx.is_none()
    }

    fn ask(&mut self, engine: &Engine, q: Query, tag: T) {
        self.rx = Some(engine.query(q));
        self.tag = tag;
    }

    /// Réponse arrivée, avec l'étiquette de la demande.
    fn take(&mut self) -> Option<(Answer, T)> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(a) => {
                self.rx = None;
                Some((a, self.tag))
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// Une partie en cours.
pub struct Game {
    pub engine: Engine,
    pub spec: PlanetSpec,
    pub seed: u64,
    pub level: u32,
    /// « sources » ou « mers ».
    pub seeding: String,
    pub sandbox: bool,
    pub planet: Arc<PlanetInfo>,
    /// Journal des événements relevés, dans l'ordre.
    pub events: Vec<Event>,
    pub lineages: Arc<Vec<LineageFrame>>,
    pub history: Vec<Sample>,
    pub rules: StopRules,
    /// Messages pour le client (sauvegarde écrite, erreur).
    pub notices: VecDeque<String>,
    /// Événement qui a provoqué la dernière pause automatique.
    pub auto_paused_by: Option<u64>,
    /// Dernier détail de cellule reçu, et le pas où il a été demandé.
    pub cell: Option<(CellDetail, u64)>,
    pub pace: f64,
    next_event: u64,
    events_rx: Pending<()>,
    lineages_rx: Pending<u64>,
    lineages_step: Option<u64>,
    history_rx: Pending<()>,
    history_step: Option<u64>,
    cell_rx: Pending<(u32, u64)>,
    saves: Vec<(String, Receiver<io::Result<()>>)>,
}

impl Game {
    /// Nouvelle partie, en pause, sans vie.
    pub fn start(spec: PlanetSpec, seed: u64, level: u32, seeding: &str, sandbox: bool) -> io::Result<Game> {
        let engine = Engine::new_game(NewGame {
            seed,
            preset: spec.preset.clone(),
            level,
            threads: engine_threads(),
            sandbox,
            planet: Some(spec.to_params()),
            seeding: seeding_of(seeding),
        })?;
        Ok(Self::wrap(engine, spec, seed, level, seeding.into(), sandbox))
    }

    /// Reprise d'un point de sauvegarde : l'état complet, sans rejeu.
    pub fn load(path: &Path) -> Result<Game, String> {
        let meta = read_meta(path)?;
        let engine = Engine::load(path, engine_threads()).map_err(|e| format!("reprise impossible : {e}"))?;
        let sandbox = meta.mode == "bac-a-sable";
        let mut g = Self::wrap(engine, meta.spec, meta.seed, meta.level, meta.seeding, sandbox);
        g.pace = g.engine.frame().current.step_years * STEPS_PER_SECOND;
        g.engine.set_throttle(Some(g.pace));
        Ok(g)
    }

    fn wrap(engine: Engine, spec: PlanetSpec, seed: u64, level: u32, seeding: String, sandbox: bool) -> Game {
        let planet = planet_info(&spec, seed, level);
        engine.set_throttle(Some(DEFAULT_SPEED));
        Game {
            engine,
            spec,
            seed,
            level,
            seeding,
            sandbox,
            planet,
            events: Vec::new(),
            lineages: Arc::default(),
            history: Vec::new(),
            rules: StopRules::profile("naturaliste"),
            notices: VecDeque::new(),
            auto_paused_by: None,
            cell: None,
            pace: DEFAULT_SPEED,
            next_event: 0,
            events_rx: Pending::new(()),
            lineages_rx: Pending::new(0),
            lineages_step: None,
            history_rx: Pending::new(()),
            history_step: None,
            cell_rx: Pending::new((0, 0)),
            saves: Vec::new(),
        }
    }

    pub fn order(&self, kind: OrderKind) -> u64 {
        self.engine.submit(When::Now, kind)
    }

    pub fn order_at(&self, years: f64, kind: OrderKind) -> u64 {
        self.engine.submit(When::At(years), kind)
    }

    /// Vitesse visée : le frein du moteur, et la durée du pas par un ordre
    /// (elle change l'histoire, donc le registre la garde).
    pub fn set_speed(&mut self, years_per_second: f64) {
        self.pace = years_per_second.max(1.0);
        self.engine.set_throttle(Some(self.pace));
        let old = self.engine.frame().current.step_years;
        let new = step_years_for(self.pace);
        if (new - old).abs() > 1e-9 {
            self.order(OrderKind::SetStepYears(new));
        }
    }

    pub fn observe(&self, zone: InterestZone) {
        self.engine.set_interest(Some(zone));
    }

    pub fn save(&mut self, path: &str, name: &str) {
        let f = self.engine.frame().current;
        let meta = SaveFile {
            name: name.into(),
            engine_version: env!("CARGO_PKG_VERSION").into(),
            spec: self.spec.clone(),
            seed: self.seed,
            level: self.level,
            seeding: self.seeding.clone(),
            steps: f.step,
            years: f.years,
            mode: if self.sandbox { "bac-a-sable" } else { "observateur" }.into(),
        };
        if let Err(e) = std::fs::write(meta_path(Path::new(path)), meta.to_text()) {
            self.notices.push_back(format!("erreur\t{path}\t{e}"));
            return;
        }
        self.saves.push((path.into(), self.engine.save(PathBuf::from(path))));
    }

    /// Demande le détail d'une cellule (inspecteur) ; la réponse arrive par
    /// `pump`.
    pub fn request_cell(&mut self, cell: u32, step: u64) {
        let fresh = self.cell.as_ref().is_some_and(|(d, s)| d.cell == cell && *s == step);
        let asked = !self.cell_rx.idle() && self.cell_rx.tag == (cell, step);
        if !fresh && !asked {
            self.cell_rx.ask(&self.engine, Query::Cell { cell }, (cell, step));
        }
    }

    /// Relève les réponses arrivées et relance les requêtes périodiques.
    /// Renvoie le nombre d'événements nouveaux.
    pub fn pump(&mut self) -> usize {
        let step = self.engine.status().steps;
        let mut new_events = 0;
        if let Some((Answer::Events(list), ())) = self.events_rx.take() {
            let full = list.len() >= EVENTS_PER_POLL;
            for v in list {
                // Les identifiants se suivent ; un doublon (bornes de la
                // requête) est ignoré.
                if v.id < self.next_event {
                    continue;
                }
                self.next_event = v.id + 1;
                let e = chronicle::from_view(&v);
                self.apply_rules(&e);
                self.events.push(e);
                new_events += 1;
            }
            if full {
                self.ask_events();
            }
        }
        if self.events_rx.idle() {
            self.ask_events();
        }
        if let Some((Answer::Lineages(list), at)) = self.lineages_rx.take() {
            self.lineages = Arc::new(list.iter().map(LineageFrame::from).collect());
            self.lineages_step = Some(at);
        }
        // L'arbre du vivant n'a pas besoin de chaque pas quand les lignées se
        // comptent par centaines de milliers.
        let every = if self.lineages.len() > 200_000 { 20 } else { 1 };
        if self.lineages_rx.idle() && self.lineages_step.is_none_or(|s| step >= s + every) {
            self.lineages_rx.ask(&self.engine, Query::Lineages { since_years: f64::NEG_INFINITY }, step);
        }
        if let Some((Answer::GlobalHistory(list), ())) = self.history_rx.take() {
            for s in list {
                if self.history.last().is_none_or(|l| s.years > l.years) {
                    self.history.push(s);
                }
            }
        }
        if self.history_rx.idle() && self.history_step != Some(step) {
            let from = self.history.last().map_or(f64::NEG_INFINITY, |s| s.years);
            self.history_rx.ask(&self.engine, Query::GlobalHistory { from_years: from }, ());
            self.history_step = Some(step);
        }
        if let Some((Answer::Cell(Some(d)), (_, at))) = self.cell_rx.take() {
            self.cell = Some((d, at));
        }
        self.saves.retain(|(path, rx)| match rx.try_recv() {
            Ok(r) => {
                self.notices.push_back(match r {
                    Ok(()) => format!("ok\t{path}"),
                    Err(e) => format!("erreur\t{path}\t{e}"),
                });
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
        });
        new_events
    }

    fn ask_events(&mut self) {
        // Identifiants à partir de `since_id` inclus.
        self.events_rx.ask(&self.engine, Query::Events { since_id: self.next_event, min_interest: 0.0, limit: EVENTS_PER_POLL }, ());
    }

    /// Règles d'arrêt : le client met le temps en pause par un ordre quand
    /// un événement le demande.
    fn apply_rules(&mut self, e: &Event) {
        if e.origin == Origin::Player || self.auto_paused_by.is_some() {
            return;
        }
        if self.rules.decide(e, self.pace) == Action::Pause && !self.engine.status().paused {
            self.auto_paused_by = Some(e.id);
            self.order(OrderKind::Pause);
        }
    }
}

/// Reprise d'une sauvegarde sur un fil à part : l'écran de chargement reste
/// vivant pendant la lecture de l'état.
pub struct Loading {
    result: Arc<Mutex<Option<Result<Game, String>>>>,
    handle: Option<JoinHandle<()>>,
}

impl Loading {
    pub fn start(path: PathBuf) -> Loading {
        let result = Arc::new(Mutex::new(None));
        let r = result.clone();
        let handle = std::thread::Builder::new()
            .name("evonisium-reprise".into())
            .spawn(move || {
                let g = Game::load(&path);
                *r.lock().unwrap_or_else(|e| e.into_inner()) = Some(g);
            })
            .ok();
        Loading { result, handle }
    }

    /// La partie, une fois chargée.
    pub fn take(&mut self) -> Option<Result<Game, String>> {
        let g = self.result.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        Some(g)
    }
}
