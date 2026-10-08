//! Le fil de simulation et ce que le client en voit.

use crate::geometry::GridGeometry;
use crate::query::{Answer, Query, Store};
use evo_planet::PlanetParams;
use evo_sim::history::PublishedState;
use evo_sim::observation::InterestZone;
use evo_sim::orders::OrderKind;
use evo_sim::{Seeding, World, WorldConfig};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Paramètres d'une nouvelle partie.
#[derive(Clone, Debug, PartialEq)]
pub struct NewGame {
    pub seed: u64,
    /// Préréglage du monde : « terre », « ocean », « desert », « super-terre »,
    /// « petite », « sans-lune ».
    pub preset: String,
    /// Niveau de la grille physique (6 : 40 962 cellules ; la vie tourne un
    /// niveau en dessous).
    pub level: u32,
    /// Fils de calcul de la simulation.
    pub threads: usize,
    /// Bac à sable : interventions sans limite d'influence.
    pub sandbox: bool,
    /// Paramètres de planète réglés à l'écran de création ; s'ils sont
    /// donnés, ils remplacent le préréglage.
    pub planet: Option<PlanetParams>,
    /// Où l'ordre `SeedLife` dépose les cellules minimales.
    pub seeding: Seeding,
}

impl Default for NewGame {
    fn default() -> Self {
        Self { seed: 1, preset: "terre".into(), level: 6, threads: 6, sandbox: false, planet: None, seeding: Seeding::Vents }
    }
}

impl NewGame {
    /// Configuration du monde correspondante.
    pub fn config(&self) -> io::Result<WorldConfig> {
        let preset = PlanetParams::by_key(&self.preset).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("monde inconnu : {} (connus : {})", self.preset, PlanetParams::KEYS.join(", ")),
            )
        })?;
        let planet = self.planet.clone().unwrap_or(preset);
        let mut cfg = WorldConfig::with_planet(planet, self.seed, self.level);
        cfg.influence.sandbox = self.sandbox;
        cfg.seeding = self.seeding;
        Ok(cfg)
    }
}

/// Date d'effet d'un ordre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum When {
    /// Au prochain pas : date du moteur à la réception.
    Now,
    /// À une date de jeu, années.
    At(f64),
}

/// Les deux derniers états publiés.
#[derive(Clone, Debug)]
pub struct Frame {
    pub current: Arc<PublishedState>,
    pub previous: Option<Arc<PublishedState>>,
}

/// Santé du fil de simulation, pour le tableau de bord.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EngineStatus {
    pub years: f64,
    pub steps: u64,
    /// Durée du dernier pas, s (horloge murale).
    pub last_step_seconds: f64,
    /// Vitesse lissée, années de jeu par seconde.
    pub years_per_second: f64,
    pub paused: bool,
    /// Empreinte de l'état complet, tenue à jour quand la partie est en
    /// pause (vérification du rejeu ; voir aussi `Query::StateHash`).
    pub state_hash: Option<u64>,
    /// Dernière erreur du fil (base, sauvegarde).
    pub error: Option<String>,
    /// Faux une fois le fil arrêté.
    pub running: bool,
}

enum Command {
    Order { id: u64, when: When, kind: OrderKind },
    Interest(Option<InterestZone>),
    Throttle(Option<f64>),
    Query(Query, Sender<Answer>),
    Save(PathBuf, Sender<io::Result<()>>),
    Stop,
}

struct Shared {
    frame: RwLock<Frame>,
    status: Mutex<EngineStatus>,
}

/// Le moteur : un fil de simulation, son groupe de fils de calcul, sa base.
pub struct Engine {
    tx: Mutex<(Sender<Command>, u64)>,
    shared: Arc<Shared>,
    grid: Arc<GridGeometry>,
    thread: Option<JoinHandle<()>>,
    workdir: PathBuf,
}

static ENGINE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Chemin de la base qui accompagne un point de sauvegarde.
pub fn history_path(save: &Path) -> PathBuf {
    let mut s = save.as_os_str().to_owned();
    s.push(".histoire.sqlite");
    PathBuf::from(s)
}

impl Engine {
    /// Crée une planète et lance la simulation (en pause : le client envoie
    /// `SeedLife` puis `Resume`, ou `Resume` seul).
    pub fn new_game(game: NewGame) -> io::Result<Engine> {
        let cfg = game.config()?;
        let pool = pool(game.threads)?;
        let mut world = pool.install(|| World::new(cfg));
        world.paused = true;
        world.republish();
        Engine::start(world, pool, None)
    }

    /// Reprend une partie depuis un point de sauvegarde, avec son
    /// historique s'il est à côté.
    pub fn load(path: &Path, threads: usize) -> io::Result<Engine> {
        let pool = pool(threads)?;
        let world = pool.install(|| World::load_file(path))?;
        let db = history_path(path);
        Engine::start(world, pool, db.exists().then_some(db))
    }

    fn start(world: World, pool: rayon::ThreadPool, history: Option<PathBuf>) -> io::Result<Engine> {
        let n = ENGINE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let workdir = std::env::temp_dir().join(format!("evonisium-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&workdir)?;
        let db = workdir.join("histoire.sqlite");
        if let Some(src) = history {
            std::fs::copy(src, &db)?;
        }
        let store = Store::open(&db, &world)?;
        let grid = Arc::new(GridGeometry::new(&world.planet.grid, world.planet.params.radius_m, &world.bio));
        let frame = frame_of(&world);
        let shared = Arc::new(Shared {
            frame: RwLock::new(frame),
            status: Mutex::new(EngineStatus {
                years: world.years(),
                steps: world.stats.steps,
                paused: world.paused,
                running: true,
                ..Default::default()
            }),
        });
        let (tx, rx) = channel();
        let next_id = world.orders.next_id();
        let sh = shared.clone();
        let thread = std::thread::Builder::new().name("evonisium-simulation".into()).spawn(move || run(world, pool, store, rx, sh))?;
        Ok(Engine { tx: Mutex::new((tx, next_id)), shared, grid, thread: Some(thread), workdir })
    }

    /// Géométrie de la grille (fixe pendant la partie).
    pub fn grid(&self) -> Arc<GridGeometry> {
        self.grid.clone()
    }

    /// Les deux derniers états publiés.
    pub fn frame(&self) -> Frame {
        self.shared.frame.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn status(&self) -> EngineStatus {
        self.shared.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Ajoute un ordre à la file et renvoie son identifiant (celui que porte
    /// l'événement `OrderRefused` s'il est refusé).
    pub fn submit(&self, when: When, kind: OrderKind) -> u64 {
        let mut g = self.tx.lock().unwrap_or_else(|e| e.into_inner());
        let id = g.1;
        g.1 += 1;
        let _ = g.0.send(Command::Order { id, when, kind });
        id
    }

    /// Zone d'intérêt de la caméra (canal d'observation : jamais dans
    /// l'histoire).
    pub fn set_interest(&self, zone: Option<InterestZone>) {
        self.send(Command::Interest(zone));
    }

    /// Plafond de vitesse en années de jeu par seconde (horloge murale, hors
    /// histoire) ; `None` : aussi vite que possible.
    pub fn set_throttle(&self, max_years_per_second: Option<f64>) {
        self.send(Command::Throttle(max_years_per_second));
    }

    /// Requête asynchrone ; la réponse arrive au plus un pas plus tard.
    pub fn query(&self, q: Query) -> Receiver<Answer> {
        let (tx, rx) = channel();
        self.send(Command::Query(q, tx));
        rx
    }

    /// Écrit un point de sauvegarde (et son historique à côté) à la fin du
    /// pas en cours.
    pub fn save(&self, path: PathBuf) -> Receiver<io::Result<()>> {
        let (tx, rx) = channel();
        self.send(Command::Save(path, tx));
        rx
    }

    fn send(&self, c: Command) {
        let _ = self.tx.lock().unwrap_or_else(|e| e.into_inner()).0.send(c);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.send(Command::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let _ = std::fs::remove_dir_all(&self.workdir);
    }
}

fn pool(threads: usize) -> io::Result<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .thread_name(|i| format!("evonisium-calcul-{i}"))
        .build()
        .map_err(|e| io::Error::other(e.to_string()))
}

fn frame_of(world: &World) -> Frame {
    let p = &world.publication;
    Frame { current: p.current.clone().expect("le monde publie dès sa création"), previous: p.previous.clone() }
}

/// Boucle du fil de simulation.
fn run(mut world: World, pool: rayon::ThreadPool, mut store: Store, rx: Receiver<Command>, shared: Arc<Shared>) {
    let mut throttle: Option<f64> = None;
    let mut speed = 0.0f64;
    let set_error = |e: String| shared.status.lock().unwrap_or_else(|p| p.into_inner()).error = Some(e);
    'outer: loop {
        // Commandes reçues : en pause, on attend ; sinon on ne fait que vider.
        let mut commands = Vec::new();
        if world.paused {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(c) => commands.push(c),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        commands.extend(rx.try_iter());
        let mut dirty = false;
        for c in commands {
            match handle(&mut world, &mut store, &mut throttle, c) {
                Flow::Stop => break 'outer,
                Flow::Dirty => dirty = true,
                Flow::Quiet => {}
            }
        }
        if world.paused && !dirty {
            continue;
        }
        let t0 = Instant::now();
        let was_paused = world.paused;
        let steps = world.stats.steps;
        pool.install(|| world.step());
        if world.stats.steps == steps {
            // Pas en pause : seuls les ordres dus ont été appliqués.
            world.republish();
        }
        if let Err(e) = store.record(&world) {
            set_error(e.to_string());
        }
        *shared.frame.write().unwrap_or_else(|e| e.into_inner()) = frame_of(&world);
        let elapsed = t0.elapsed().as_secs_f64();
        let advanced = if world.stats.steps > steps { world.config.step_years } else { 0.0 };
        if advanced > 0.0 {
            let inst = advanced / elapsed.max(1e-6);
            speed = if speed == 0.0 || was_paused { inst } else { 0.8 * speed + 0.2 * inst };
        }
        {
            let mut st = shared.status.lock().unwrap_or_else(|e| e.into_inner());
            st.years = world.years();
            st.steps = world.stats.steps;
            st.paused = world.paused;
            st.state_hash = world.paused.then(|| world.state_hash());
            if advanced > 0.0 {
                st.last_step_seconds = elapsed;
                st.years_per_second = speed;
            }
        }
        // Freinage : le pas suivant ne commence pas avant son heure.
        if let (Some(max), true) = (throttle, advanced > 0.0) {
            let wanted = advanced / max;
            if wanted > elapsed {
                match rx.recv_timeout(Duration::from_secs_f64(wanted - elapsed)) {
                    Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                    Ok(c) => {
                        if let Flow::Stop = handle(&mut world, &mut store, &mut throttle, c) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        }
    }
    shared.status.lock().unwrap_or_else(|e| e.into_inner()).running = false;
}

enum Flow {
    Stop,
    /// L'état publié doit être refait (ordre reçu, zone d'intérêt en pause).
    Dirty,
    Quiet,
}

fn handle(world: &mut World, store: &mut Store, throttle: &mut Option<f64>, c: Command) -> Flow {
    match c {
        Command::Stop => return Flow::Stop,
        Command::Order { id, when, kind } => {
            let due = match when {
                When::Now => world.years(),
                When::At(y) => y,
            };
            world.orders.submit_with_id(id, due, kind);
            return Flow::Dirty;
        }
        Command::Interest(z) => {
            world.set_interest(z);
            if world.paused {
                return Flow::Dirty;
            }
        }
        Command::Throttle(t) => *throttle = t.filter(|v| *v > 0.0),
        Command::Query(q, reply) => {
            let _ = reply.send(store.answer(world, &q));
        }
        Command::Save(path, reply) => {
            let r = world.save_file(&path).and_then(|_| store.backup_to(&history_path(&path)));
            let _ = reply.send(r);
        }
    }
    Flow::Quiet
}
