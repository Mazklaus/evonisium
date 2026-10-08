//! Le monde de l'étape 2 : une planète vivante peuplée de microbes.
//!
//! Un pas planétaire enchaîne :
//! 0. ordres : application des ordres dus (file d'ordres), entre deux pas ;
//! 1. planète lente : tectonique (tous les millions d'années), obliquité,
//!    relief, niveau de la mer et climat à l'équilibre ; les cellules qui
//!    passent de l'océan à la terre (ou l'inverse) échangent exactement leur
//!    contenu avec les réservoirs globaux ;
//! 2. écologie : dynamique rapide des populations couplée à la chimie de la
//!    couche d'eau et à ses échanges (en parallèle, cellule par cellule) ;
//!    les échanges mesurés sont appliqués exactement aux réservoirs, puis
//!    prolongés sur le reste du pas, équilibrés cellule par cellule en carbone
//!    et en phosphore, pendant que les boîtes globales tournent en sous-pas ;
//! 3. évolution : apparition puis fixation, tunnel stochastique, transfert
//!    horizontal, accélérateur en dernier recours ;
//! 4. migration vers les cellules océaniques voisines ;
//! 5. registres : lignées, innovations, seuils d'oxygène, glaciations,
//!    historique, état publié.
//!
//! Les phases parallèles ne lisent et n'écrivent que leur cellule ; tout ce
//! qui crée des identifiants est appliqué ensuite, dans l'ordre des cellules.
//! La même graine et le même registre d'ordres donnent donc la même histoire,
//! quel que soit le nombre de coeurs.

use crate::evolution::{evolve_cell, CellEvolution, EvolutionParams, EvolutionStats, Target};
use crate::history::{CellView, ClimateMode, History, Publication, PublishedState, Sample};
use crate::orders::{AppliedOrder, Order, OrderKind, OrderQueue};
use evo_core::events::{EventKind, EventLog, Origin};
use evo_core::flux::{Element, FluxRegistry};
use evo_core::rng::{rng_for, Stream};
use evo_genetics::genome::MARKER_LEN;
use evo_genetics::{
    Domain, DomainFamily, Gene, Genome, GenomeChangeCause, GenomeJournal, JournalEntry, LineageRegistry, MutationParams, OriginFixation,
    GENOME_CHANGE_CAUSE_COUNT,
};
use evo_life::community::{evaluate, substep, CellContext, Population};
use evo_life::metabolism::{
    domain_relations, photosynthesis_stage, FERMENTATION, METHANOGENESIS, PHOTOSYNTHESIS_PATHWAY, PHOTOSYNTHESIS_STAGES, REACTION_COUNT,
    RHODOPSIN_PATHWAY,
};
use evo_life::{growth_rates, pigment_colour, selection_coefficient, LightSpectrum, Phenotype, Physiology};
use evo_planet::generate::generate;
use evo_planet::{Gas, Planet, PlanetParams, WaterChemistry, WaterPool, TECTONIC_STEP_YEARS, WATER_POOLS, WATER_POOL_COUNT};
use rand::Rng;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Où déposer les cellules minimales au départ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seeding {
    /// Près des sources hydrothermales (choix par défaut du moteur).
    Vents,
    /// Dans toutes les cellules océaniques (mesures de charge).
    AllOcean,
}

/// Seuils d'oxygène atmosphérique signalés (fraction molaire). L'atmosphère
/// actuelle de la Terre en contient 0,21 ; la « grande oxydation » la fait
/// passer de moins de 10⁻⁶ à plus de 10⁻³.
pub const OXYGEN_THRESHOLDS: [f64; 5] = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2];

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    /// Niveau de subdivision de la grille (6 : 40 962 cellules).
    pub level: u32,
    pub planet: PlanetParams,
    /// Durée d'un pas planétaire au départ, années (les ordres de vitesse la
    /// changent).
    pub step_years: f64,
    /// Sous-pas écologiques par pas planétaire et leur durée, années.
    pub eco_substeps: usize,
    pub eco_dt_years: f64,
    pub physiology: Physiology,
    pub mutation: MutationParams,
    pub regime: OriginFixation,
    pub evolution: EvolutionParams,
    /// Part de la biomasse qui passe chaque année dans une cellule voisine.
    pub migration_rate: f64,
    /// Sous ce seuil de biomasse (mol de carbone), une population disparaît.
    pub extinction_biomass: f64,
    /// Biomasse de départ d'une population fondée par mutation ou migration.
    pub founder_biomass: f64,
    /// Biomasse phototrophe qui absorbe 63 % de la lumière, molC·m⁻².
    pub light_biomass_per_m2: f64,
    /// Nombre maximal de guildes suivies par cellule : au-delà, les plus
    /// petites populations sont retirées (exclusion compétitive que
    /// l'écologie quasi stationnaire n'a pas le temps de faire).
    pub max_populations_per_cell: usize,
    pub seeding: Seeding,
    /// Biomasse déposée par cellule au départ, mol de carbone.
    pub seed_biomass: f64,
    /// Intervalle de l'historique des grandeurs globales, années.
    pub history_every_years: f64,
}

impl WorldConfig {
    pub fn new(seed: u64, level: u32) -> Self {
        Self::with_planet(PlanetParams::earth_archean(), seed, level)
    }

    pub fn with_planet(planet: PlanetParams, seed: u64, level: u32) -> Self {
        Self {
            seed,
            level,
            planet,
            step_years: 100_000.0,
            eco_substeps: 30,
            eco_dt_years: 1.0 / 3650.0,
            physiology: Physiology::default(),
            mutation: MutationParams { reaction_count: REACTION_COUNT as u8, relations: domain_relations(), ..Default::default() },
            regime: OriginFixation::default(),
            evolution: EvolutionParams::default(),
            migration_rate: 1.0,
            extinction_biomass: 1.0,
            founder_biomass: 100.0,
            light_biomass_per_m2: 0.1,
            max_populations_per_cell: 12,
            seeding: Seeding::Vents,
            seed_biomass: 1e4,
            history_every_years: 1e6,
        }
    }
}

/// Compteurs cumulés depuis le début de la partie.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldStats {
    pub steps: u64,
    pub substitutions: u64,
    pub new_lineages: u64,
    pub colonisations: u64,
    pub migrant_replacements: u64,
    pub local_extinctions: u64,
    /// Lignées éteintes sans lignée fille (comptées, sans événement).
    pub leaf_extinctions: u64,
    /// Écart des électrons corrigé sur les flux extrapolés, mol d'équivalent
    /// O₂ cumulées (voir `ecology_phase`).
    pub redox_correction: f64,
    /// Génomes mutants construits et évalués (mutation, phénotype, r, s).
    pub genetic_evaluations: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    /// Modifications de génome fixées, par cause ([`GenomeChangeCause::index`]).
    pub fixed_changes_by_cause: [u64; GENOME_CHANGE_CAUSE_COUNT],
    /// Cellules passées de l'océan à la terre et inversement.
    pub cells_emerged: u64,
    pub cells_flooded: u64,
    /// Pas pendant lesquels l'accélérateur a agi.
    pub accelerator_steps: u64,
}

impl WorldStats {
    fn add_evolution(&mut self, o: &EvolutionStats) {
        self.substitutions += o.substitutions;
        self.genetic_evaluations += o.genetic_evaluations;
        self.tunnel_attempts += o.tunnel_attempts;
        self.tunnel_successes += o.tunnel_successes;
        for (a, b) in self.fixed_changes_by_cause.iter_mut().zip(o.fixed_by_cause) {
            *a += b;
        }
    }
}

/// Temps passé dans chaque phase d'un pas.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhaseTimings {
    pub planet: Duration,
    pub ecology: Duration,
    pub evolution: Duration,
    pub migration: Duration,
    pub bookkeeping: Duration,
}

impl PhaseTimings {
    pub fn total(&self) -> Duration {
        self.planet + self.ecology + self.evolution + self.migration + self.bookkeeping
    }

    pub fn add(&mut self, o: &PhaseTimings) {
        self.planet += o.planet;
        self.ecology += o.ecology;
        self.evolution += o.evolution;
        self.migration += o.migration;
        self.bookkeeping += o.bookkeeping;
    }

    pub fn divided(&self, n: u32) -> PhaseTimings {
        let n = n.max(1);
        PhaseTimings {
            planet: self.planet / n,
            ecology: self.ecology / n,
            evolution: self.evolution / n,
            migration: self.migration / n,
            bookkeeping: self.bookkeeping / n,
        }
    }
}

/// Suivi des innovations, de l'accélérateur et des seuils globaux.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    /// Étape la plus avancée jamais atteinte sur le chemin de la photosynthèse.
    pub best_stage: u8,
    /// Date de la dernière étape franchie (ou de l'arrivée de la vie).
    pub stage_since_years: Option<f64>,
    /// Date et événement de chaque étape franchie (index : étape).
    pub stage_years: [Option<f64>; 5],
    pub stage_events: [Option<u64>; 5],
    pub rhodopsin_event: Option<u64>,
    pub accelerator_on: bool,
    /// Seuils d'oxygène franchis.
    pub oxygen_level: usize,
    pub snowball: bool,
}

/// Immigrant retenu pour une guilde d'une cellule.
struct MigrationWinner {
    signature: u32,
    source: (usize, usize),
    selection: f64,
    rates: evo_life::GrowthRates,
    lineage: u32,
    genome: Arc<Genome>,
    phenotype: Arc<Phenotype>,
}

pub struct World {
    pub config: WorldConfig,
    pub planet: Planet,
    pub chemistry: Vec<WaterChemistry>,
    pub communities: Vec<Vec<Population>>,
    pub lineages: LineageRegistry,
    pub events: EventLog,
    pub journal: GenomeJournal,
    /// Registre de flux : carbone et phosphore entrés ou sortis du système
    /// suivi (volcanisme, altération, subduction, ordres du joueur).
    pub flux: FluxRegistry,
    pub orders: OrderQueue,
    pub history: History,
    pub publication: Publication,
    pub progress: Progress,
    /// Date de jeu à la fin du dernier pas, années.
    pub years: f64,
    pub paused: bool,
    /// Lignées suivies par le joueur (sans effet sur l'histoire).
    pub marked: Vec<u32>,
    /// Production brute d'O₂ par photosynthèse au dernier pas, mol·an⁻¹.
    pub oxygen_production: f64,
    /// Flux équilibrés de la surface vers les réservoirs au pas précédent,
    /// mol·an⁻¹ (amortissement du couplage, voir `ecology_phase`).
    previous_rates: Option<[f64; WATER_POOL_COUNT]>,
    pub stats: WorldStats,
    pub timings: PhaseTimings,
}

impl World {
    pub fn new(mut config: WorldConfig) -> Self {
        let planet = generate(config.planet.clone(), config.level, config.seed);
        // Les pigments sont jugés sous l'étoile de cette partie, dans l'eau.
        config.physiology.spectrum = LightSpectrum::new(planet.params.star_temperature_k, planet.params.mixed_layer_m);
        let targets = planet.exchange_targets(config.physiology.carbon_to_phosphorus);
        let chemistry: Vec<WaterChemistry> = (0..planet.cells.len()).map(|c| planet.equilibrium_chemistry(c, &targets)).collect();
        let n = planet.cells.len();
        let mut world = Self {
            history: History::new(config.history_every_years),
            config,
            planet,
            chemistry,
            communities: vec![Vec::new(); n],
            lineages: LineageRegistry::default(),
            events: EventLog::default(),
            journal: GenomeJournal::default(),
            flux: FluxRegistry::default(),
            orders: OrderQueue::default(),
            publication: Publication::default(),
            progress: Progress::default(),
            years: 0.0,
            paused: false,
            marked: Vec::new(),
            oxygen_production: 0.0,
            previous_rates: None,
            stats: WorldStats::default(),
            timings: PhaseTimings::default(),
        };
        world.flux.set_initial(Element::Carbon, world.total_carbon());
        world.flux.set_initial(Element::Phosphorus, world.total_phosphorus());
        world.record_history();
        world.publish();
        world
    }

    /// Monde rejoué : même configuration, même registre d'ordres.
    pub fn replay(config: WorldConfig, orders: &[Order]) -> Self {
        let mut w = Self::new(config);
        w.orders = OrderQueue::from_log(orders);
        w
    }

    pub fn years(&self) -> f64 {
        self.years
    }

    /// Cellule minimale (décision du 7 octobre 2026) : une chimioautotrophie
    /// simple (H₂ + CO₂), une fermentation rudimentaire, un gène de réparation
    /// de l'ADN et un transporteur d'électrons à hème (les cytochromes sont
    /// antérieurs à la photosynthèse ; leurs copies divergentes donnent les
    /// premiers pigments), adaptés à une eau tiède.
    pub fn minimal_cell(&self) -> Genome {
        let mut rng = rng_for(self.config.seed, Stream::Seeding, &[0]);
        let gene = |family, efficiency, affinity, absorption_nm| Gene {
            domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 12.0, absorption_nm },
            functional: true,
        };
        let mut marker = [0u8; MARKER_LEN];
        for b in marker.iter_mut() {
            *b = rng.random_range(0..4);
        }
        Genome {
            genes: vec![
                gene(DomainFamily::Catalytic(METHANOGENESIS), 1.0, 1.0, 500.0),
                gene(DomainFamily::Catalytic(FERMENTATION), 0.3, 0.5, 500.0),
                gene(DomainFamily::Repair, 1.0, 0.5, 500.0),
                gene(DomainFamily::Cytochrome, 0.5, 1.0, 420.0),
            ],
            marker,
        }
    }

    /// Dépose des cellules minimales ; le carbone et le phosphore de leur
    /// biomasse sont pris à la couche d'eau de chaque cellule.
    pub fn seed_life(&mut self) {
        self.seed_life_with(Origin::Engine, None);
    }

    fn seed_life_with(&mut self, origin: Origin, cause: Option<u64>) {
        let genome = Arc::new(self.minimal_cell());
        let phenotype = Arc::new(Phenotype::from_genome(&genome, &self.config.physiology));
        let cells: Vec<usize> = match self.config.seeding {
            Seeding::Vents => self.planet.vent_cells(),
            Seeding::AllOcean => self.planet.ocean_cells().collect(),
        };
        let first = cells.first().copied().unwrap_or(0) as u32;
        let lineage = self.lineages.found(None, self.years, first, phenotype.signature, genome.clone());
        let event = self.events.push_with(self.years, Some(first), EventKind::LifeSeeded { lineage }, origin, cause);
        let cp = self.config.physiology.carbon_to_phosphorus;
        for c in cells {
            let v = self.planet.cells[c].water_volume_m3;
            let chem = &mut self.chemistry[c];
            let b = self.config.seed_biomass.min(0.5 * chem[WaterPool::Dic as usize] * v).min(0.5 * chem[WaterPool::Po4 as usize] * v * cp);
            if b < self.config.extinction_biomass {
                continue;
            }
            chem[WaterPool::Dic as usize] -= b / v;
            chem[WaterPool::Po4 as usize] -= b / cp / v;
            self.communities[c].push(Population {
                lineage,
                genome: genome.clone(),
                phenotype: phenotype.clone(),
                biomass: b,
                rates: Default::default(),
            });
        }
        if self.progress.stage_since_years.is_none() {
            self.progress.stage_since_years = Some(self.years);
            self.progress.stage_years[0] = Some(self.years);
            self.progress.stage_events[0] = Some(event);
        }
    }

    pub fn biomass(&self) -> f64 {
        self.communities.iter().flatten().map(|p| p.biomass).sum()
    }

    /// Carbone total du système (couche d'eau, biomasse, réservoirs), mol.
    pub fn total_carbon(&self) -> f64 {
        self.planet.water_carbon(&self.chemistry) + self.biomass() + self.planet.reservoirs.carbon()
    }

    /// Phosphore total du système, mol.
    pub fn total_phosphorus(&self) -> f64 {
        self.planet.water_phosphorus(&self.chemistry)
            + self.biomass() / self.config.physiology.carbon_to_phosphorus
            + self.planet.reservoirs.phosphorus()
    }

    /// Écarts relatifs des bilans de carbone et de phosphore.
    pub fn carbon_balance_error(&self) -> f64 {
        self.flux.relative_error(Element::Carbon, self.total_carbon())
    }

    pub fn phosphorus_balance_error(&self) -> f64 {
        self.flux.relative_error(Element::Phosphorus, self.total_phosphorus())
    }

    /// Applique les ordres dus à la date courante.
    fn apply_orders(&mut self) {
        for order in self.orders.take_due(self.years) {
            let origin = if order.kind.is_intervention() { Origin::Player } else { Origin::Engine };
            let event = self.events.push_with(
                self.years,
                None,
                EventKind::OrderApplied { order: order.id, label: order.kind.label() },
                origin,
                None,
            );
            match order.kind {
                OrderKind::SetStepYears(y) => {
                    if y > 0.0 && y.is_finite() {
                        self.config.step_years = y;
                    }
                }
                OrderKind::Pause => self.paused = true,
                OrderKind::Resume => self.paused = false,
                OrderKind::SeedLife => self.seed_life_with(Origin::Player, Some(event)),
                OrderKind::AddPhosphate { moles } => {
                    let m = moles.max(0.0);
                    self.planet.reservoirs.deep_po4 += m;
                    self.flux.exchange(Element::Phosphorus, m);
                }
                OrderKind::InjectGas { gas, moles } => {
                    let r = &mut self.planet.reservoirs.atmosphere[gas as usize];
                    let m = moles.max(-*r);
                    *r += m;
                    if matches!(gas, Gas::Co2 | Gas::Ch4) {
                        self.flux.exchange(Element::Carbon, m);
                    }
                }
                OrderKind::MarkLineage { lineage } => {
                    if !self.marked.contains(&lineage) {
                        self.marked.push(lineage);
                    }
                }
            }
            self.orders.applied.push(AppliedOrder { order, step: self.stats.steps, years: self.years, event });
        }
    }

    /// Avance d'un pas planétaire (en pause, n'applique que les ordres dus).
    pub fn step(&mut self) -> PhaseTimings {
        let mut timings = PhaseTimings::default();
        let t0 = Instant::now();
        self.apply_orders();
        if self.paused {
            return timings;
        }
        let dt = self.config.step_years;
        let step_index = self.stats.steps;
        let years = self.years;

        // 1. Planète lente.
        self.planet_phase(years, dt, step_index);
        timings.planet = t0.elapsed();

        // 2. Écologie et cycles globaux.
        let t1 = Instant::now();
        self.ecology_phase(years, dt);
        timings.ecology = t1.elapsed();

        // 3. Évolution.
        let t2 = Instant::now();
        let modified = self.evolution_phase(years, dt, step_index);
        timings.evolution = t2.elapsed();

        // 4. Migration.
        let t3 = Instant::now();
        self.migrate(dt, step_index);
        self.trim_communities();
        timings.migration = t3.elapsed();

        // 5. Registres.
        let t4 = Instant::now();
        self.years = years + dt;
        self.bookkeeping(modified);
        self.stats.steps += 1;
        timings.bookkeeping = t4.elapsed();
        self.timings.add(&timings);
        timings
    }

    fn planet_phase(&mut self, years: f64, dt: f64, step_index: u64) {
        let seed = self.config.seed;
        let planet = &mut self.planet;
        planet.tectonic_clock += dt;
        if planet.tectonic_clock >= TECTONIC_STEP_YEARS {
            let heat = planet.params.internal_heat(years);
            let span = planet.tectonic_clock;
            planet.tectonic_clock = 0.0;
            if planet.tectonics.step(&planet.grid, &planet.params, years, span, heat, seed) {
                let plates = planet.tectonics.plates.len() as u32;
                self.events.push(years, None, EventKind::PlateReorganisation { plates });
            }
        }
        let mut rng = rng_for(seed, Stream::Climate, &[step_index]);
        let (u1, u2): (f64, f64) = (rng.random::<f64>().max(1e-300), rng.random());
        let gaussian = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        planet.climate.drift_obliquity(&planet.params, dt, gaussian);

        let before: Vec<(bool, f64)> = planet.cells.iter().map(|e| (e.is_ocean, e.water_volume_m3)).collect();
        let changes = planet.refresh(years);
        if changes.is_empty() {
            return;
        }
        let cp = self.config.physiology.carbon_to_phosphorus;
        let targets = self.planet.exchange_targets(cp);
        for ch in changes {
            let c = ch.cell;
            let (was_ocean, old_volume) = before[c];
            let new_volume = self.planet.cells[c].water_volume_m3;
            match (was_ocean, ch.now_ocean) {
                (true, false) => {
                    // La couche d'eau disparaît : son contenu et la biomasse
                    // rejoignent les réservoirs et les sédiments.
                    let moles = self.chemistry[c].map(|x| x * old_volume);
                    let biomass: f64 = self.communities[c].iter().map(|p| p.biomass).sum();
                    self.planet.reservoirs.absorb_layer(&moles, biomass, biomass / cp);
                    self.stats.local_extinctions += self.communities[c].len() as u64;
                    self.communities[c].clear();
                    self.chemistry[c] = [0.0; WATER_POOL_COUNT];
                    self.stats.cells_emerged += 1;
                }
                (false, true) => {
                    let wanted = self.planet.equilibrium_chemistry(c, &targets).map(|x| x * new_volume);
                    let got = self.planet.reservoirs.fill_layer(&wanted);
                    self.chemistry[c] = got.map(|m| m / new_volume);
                    self.stats.cells_flooded += 1;
                }
                (true, true) if old_volume > 0.0 && new_volume > 0.0 => {
                    // La couche change d'épaisseur : les moles restent.
                    let k = old_volume / new_volume;
                    for x in self.chemistry[c].iter_mut() {
                        *x *= k;
                    }
                }
                _ => {}
            }
        }
    }

    fn ecology_phase(&mut self, years: f64, dt: f64) {
        let cfg = &self.config;
        let planet = &self.planet;
        let cp = cfg.physiology.carbon_to_phosphorus;
        let targets = planet.exchange_targets(cp);
        let t_eco = cfg.eco_substeps as f64 * cfg.eco_dt_years;
        struct CellEco {
            exact: [f64; WATER_POOL_COUNT],
            rates: [f64; WATER_POOL_COUNT],
            oxygen: f64,
            extinctions: u64,
            redox_correction: f64,
        }
        let zero = || CellEco {
            exact: [0.0; WATER_POOL_COUNT],
            rates: [0.0; WATER_POOL_COUNT],
            oxygen: 0.0,
            extinctions: 0,
            redox_correction: 0.0,
        };
        let total = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter_mut())
            .enumerate()
            .map(|(c, (pops, chem))| {
                let mut r = zero();
                let env = &planet.cells[c];
                if !env.is_ocean {
                    return r;
                }
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                let start = *chem;
                for _ in 0..cfg.eco_substeps {
                    if !pops.is_empty() {
                        let o = substep(pops, &ctx, chem, cfg.eco_dt_years, &cfg.physiology);
                        r.oxygen += o.oxygen;
                        r.exact[WaterPool::Doc as usize] += o.sinking_carbon;
                        r.exact[WaterPool::Po4 as usize] += o.sinking_carbon / cp;
                    }
                    planet.exchange(c, chem, cfg.eco_dt_years, &targets, &mut r.exact);
                }
                // Flux à prolonger : ceux d'une couche à l'équilibre, où ce qui
                // s'accumule pendant l'écologie rapide serait sorti. Le simple
                // rattrapage d'une cellule vers ses cibles (après un changement
                // de l'atmosphère) ne doit pas être prolongé sur tout le pas.
                let v = env.water_volume_m3;
                let mut steady = r.exact;
                for i in 0..WATER_POOL_COUNT {
                    steady[i] += (chem[i] - start[i]) * v;
                }
                // Aucune boîte ne fournit de matière organique dissoute à la
                // couche : une cellule qui en a consommé son stock ne peut pas
                // en importer.
                steady[WaterPool::Doc as usize] = steady[WaterPool::Doc as usize].max(0.0);
                // Extinctions locales : la biomasse restante redevient matière
                // organique dissoute et phosphate.
                pops.retain(|p| {
                    if p.biomass < cfg.extinction_biomass {
                        chem[WaterPool::Doc as usize] += p.biomass / v;
                        chem[WaterPool::Po4 as usize] += p.biomass / cp / v;
                        r.extinctions += 1;
                        false
                    } else {
                        true
                    }
                });
                evaluate(pops, &ctx, chem, &cfg.physiology);
                r.rates = balanced_rates(&steady, t_eco);
                // Bilan des électrons d'une couche à l'équilibre : ce qu'elle
                // exporte de pouvoir réducteur vers les réservoirs est ce que
                // ses sources hydrothermales lui apportent, la vie ne faisant
                // que le déplacer. L'extrapolation et la fermeture du carbone
                // ne le garantissent pas ; l'écart est corrigé (voir
                // `close_electrons`) et compté.
                let expected: f64 = WATER_POOLS.iter().map(|&p| planet.vent_supply(env, p) * p.oxidant_equivalents()).sum();
                let actual: f64 = WATER_POOLS.iter().map(|&p| r.rates[p as usize] * p.oxidant_equivalents()).sum();
                r.redox_correction = (expected - actual).abs();
                close_electrons(&mut r.rates, expected - actual);
                r
            })
            .collect::<Vec<CellEco>>()
            // Somme dans l'ordre des cellules : le résultat ne dépend pas du
            // découpage entre fils.
            .into_iter()
            .fold(zero(), |mut a, b| {
                for i in 0..WATER_POOL_COUNT {
                    a.exact[i] += b.exact[i];
                    a.rates[i] += b.rates[i];
                }
                a.oxygen += b.oxygen;
                a.extinctions += b.extinctions;
                a.redox_correction += b.redox_correction;
                a
            });
        self.stats.local_extinctions += total.extinctions;
        self.stats.redox_correction += total.redox_correction * (dt - t_eco).max(0.0);
        self.oxygen_production = total.oxygen / t_eco;
        self.planet.reservoirs.oxygen.photosynthesis += self.oxygen_production * dt;
        self.planet.reservoirs.apply_exact(&self.planet.params, &total.exact);
        let ctx = self.planet.box_context(years);
        let rest = (dt - t_eco).max(0.0);
        // Les flux d'équilibré de la surface sont tenus constants sur tout le
        // pas, alors que l'atmosphère qu'ils modifient change la vie qui les
        // produit : avec des pas de 200 000 ans, ce couplage explicite oscille
        // d'un pas à l'autre (méthane abondant, puis nul, puis abondant). On
        // applique la moyenne des flux de ce pas et du précédent, schéma
        // amorti qui supprime l'oscillation de période 2 sans changer
        // l'équilibre. Les deux jeux de flux étant équilibrés en carbone et en
        // phosphore, leur moyenne l'est aussi.
        let rates = match self.previous_rates {
            Some(prev) => std::array::from_fn(|i| 0.5 * (total.rates[i] + prev[i])),
            None => total.rates,
        };
        self.previous_rates = Some(total.rates);
        self.planet.reservoirs.integrate(&self.planet.params, &ctx, &rates, rest, &mut self.flux);
    }

    /// Évolution ; renvoie les populations modifiées (cellule, indice) avec la
    /// cause de leur modification, pour la détection des innovations.
    fn evolution_phase(&mut self, years: f64, dt: f64, step_index: u64) -> Vec<(usize, usize, GenomeChangeCause)> {
        let cfg = &self.config;
        let planet = &self.planet;
        let accelerator = self.progress.accelerator_on;
        if accelerator {
            self.stats.accelerator_steps += 1;
        }
        let results: Vec<CellEvolution> = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter())
            .enumerate()
            .map(|(c, (pops, chem))| {
                let env = &planet.cells[c];
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                evolve_cell(c, pops, chem, &ctx, cfg, dt, step_index, accelerator)
            })
            .collect();
        let mut modified = Vec::new();
        for (c, r) in results.into_iter().enumerate() {
            self.stats.add_evolution(&r.stats);
            let mut founder_index = Vec::with_capacity(r.founders.len());
            for f in r.founders {
                let mut p = f.population;
                p.lineage = self.lineages.found(Some(f.parent), years, c as u32, p.signature(), p.genome.clone());
                self.events.push(
                    years,
                    Some(c as u32),
                    EventKind::NewLineage { lineage: p.lineage, parent: f.parent, signature: p.signature() },
                );
                self.stats.new_lineages += 1;
                founder_index.push(self.communities[c].len());
                self.communities[c].push(p);
            }
            for change in r.changes {
                let index = match change.target {
                    Target::Population(i) => i,
                    Target::Founder(k) => founder_index[k],
                };
                let lineage = self.communities[c][index].lineage;
                self.journal.record(JournalEntry { years, lineage, cell: c as u32, cause: change.cause, element: change.element });
                modified.push((c, index, change.cause));
            }
        }
        modified
    }

    /// Chaque cellule océanique regarde les écotypes de ses voisines et
    /// retient, par guilde, le meilleur immigrant qui réussit à s'installer.
    fn migrate(&mut self, dt: f64, step_index: u64) {
        let cfg = &self.config;
        let planet = &self.planet;
        let communities = &self.communities;
        let chemistry = &self.chemistry;
        let physio = &cfg.physiology;

        let winners: Vec<Vec<MigrationWinner>> = (0..communities.len())
            .into_par_iter()
            .map(|target| {
                let mut best: Vec<MigrationWinner> = Vec::new();
                let env = &planet.cells[target];
                if !env.is_ocean {
                    return best;
                }
                let residents = &communities[target];
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                let cond = ctx.conditions(CellContext::photo_biomass(residents));
                for src in planet.grid.neighbours_of(target) {
                    for (i, p) in communities[src].iter().enumerate() {
                        if p.rates.birth <= 0.0 {
                            continue;
                        }
                        let resident = residents.iter().find(|q| q.signature() == p.signature());
                        if resident.is_some_and(|q| Arc::ptr_eq(&q.genome, &p.genome)) {
                            continue;
                        }
                        let rates = growth_rates(&p.phenotype, &cond, &chemistry[target], physio);
                        let (s, ne) = match resident {
                            Some(q) => (selection_coefficient(&rates, &q.rates, physio), cfg.regime.effective_size(q.census(physio))),
                            None => (rates.r * rates.generation_time(physio), cfg.regime.ne_cap),
                        };
                        if s <= 0.0 {
                            continue;
                        }
                        let slot = best.iter().position(|w| w.signature == p.signature());
                        if slot.is_some_and(|k| best[k].selection >= s) {
                            continue;
                        }
                        let migrants = cfg.regime.effective_size(p.census(physio) * cfg.migration_rate * dt);
                        let mut rng = rng_for(cfg.seed, Stream::Migration, &[step_index, target as u64, src as u64, i as u64]);
                        if !cfg.regime.candidate_fixes(s, ne, migrants, &mut rng) {
                            continue;
                        }
                        let w = MigrationWinner {
                            signature: p.signature(),
                            source: (src, i),
                            selection: s,
                            rates,
                            lineage: p.lineage,
                            genome: p.genome.clone(),
                            phenotype: p.phenotype.clone(),
                        };
                        match slot {
                            Some(k) => best[k] = w,
                            None => best.push(w),
                        }
                    }
                }
                best
            })
            .collect();

        for (target, list) in winners.into_iter().enumerate() {
            for w in list {
                let (sc, si) = w.source;
                let (lineage, genome, phenotype) = (w.lineage, w.genome, w.phenotype);
                if let Some(q) = self.communities[target].iter_mut().find(|q| q.signature() == w.signature) {
                    q.genome = genome;
                    q.phenotype = phenotype;
                    q.lineage = lineage;
                    self.stats.migrant_replacements += 1;
                } else {
                    // La biomasse des colons est prise à la population source.
                    let give = self.config.founder_biomass.min(0.5 * self.communities[sc][si].biomass);
                    if give < self.config.extinction_biomass {
                        continue;
                    }
                    self.communities[sc][si].biomass -= give;
                    self.communities[target].push(Population { lineage, genome, phenotype, biomass: give, rates: w.rates });
                    self.stats.colonisations += 1;
                }
            }
        }
    }

    /// Retire les plus petites populations des cellules trop peuplées ; leur
    /// biomasse retourne à la couche d'eau (carbone organique et phosphate).
    fn trim_communities(&mut self) {
        let max = self.config.max_populations_per_cell.max(1);
        let cp = self.config.physiology.carbon_to_phosphorus;
        let planet = &self.planet;
        let removed: u64 = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter_mut())
            .enumerate()
            .map(|(c, (pops, chem))| {
                if pops.len() <= max {
                    return 0;
                }
                // Tri stable : à biomasse égale, l'ordre d'arrivée décide.
                let mut order: Vec<usize> = (0..pops.len()).collect();
                order.sort_by(|&a, &b| pops[b].biomass.total_cmp(&pops[a].biomass));
                let mut keep = vec![false; pops.len()];
                for &i in &order[..max] {
                    keep[i] = true;
                }
                let v = planet.cells[c].water_volume_m3;
                let mut i = 0;
                pops.retain(|p| {
                    let k = keep[i];
                    i += 1;
                    if !k {
                        chem[WaterPool::Doc as usize] += p.biomass / v;
                        chem[WaterPool::Po4 as usize] += p.biomass / cp / v;
                    }
                    k
                });
                (keep.len() - max) as u64
            })
            .sum();
        self.stats.local_extinctions += removed;
    }

    fn bookkeeping(&mut self, modified: Vec<(usize, usize, GenomeChangeCause)>) {
        let years = self.years;
        // Lignées éteintes.
        let mut present: Vec<u32> = self.communities.iter().flatten().map(|p| p.lineage).collect();
        present.sort_unstable();
        present.dedup();
        let (clades, leaves) = self.lineages.retire_absent(&present, years);
        self.stats.leaf_extinctions += leaves as u64;
        for id in clades {
            self.events.push(years, None, EventKind::LineageExtinct { lineage: id });
        }

        // Innovations : première apparition d'une étape sur la planète.
        let mut best: Option<(u8, usize, usize, GenomeChangeCause)> = None;
        for &(c, i, cause) in &modified {
            let Some(p) = self.communities[c].get(i) else { continue };
            let stage = photosynthesis_stage(&p.phenotype);
            if stage > self.progress.best_stage && best.is_none_or(|b| stage > b.0) {
                best = Some((stage, c, i, cause));
            }
            if p.phenotype.rhodopsin > 0.0 && self.progress.rhodopsin_event.is_none() {
                let id = self.events.push_with(
                    years,
                    Some(c as u32),
                    EventKind::Innovation { lineage: p.lineage, pathway: RHODOPSIN_PATHWAY, stage: 1, label: "rhodopsine" },
                    origin_of(cause),
                    None,
                );
                self.progress.rhodopsin_event = Some(id);
            }
        }
        if let Some((stage, c, i, cause)) = best {
            let lineage = self.communities[c][i].lineage;
            let previous = self.progress.stage_events[..stage as usize].iter().rev().find_map(|e| *e);
            let id = self.events.push_with(
                years,
                Some(c as u32),
                EventKind::Innovation { lineage, pathway: PHOTOSYNTHESIS_PATHWAY, stage, label: PHOTOSYNTHESIS_STAGES[stage as usize] },
                origin_of(cause),
                previous,
            );
            for s in (self.progress.best_stage as usize + 1)..=(stage as usize) {
                self.progress.stage_events[s] = Some(id);
                self.progress.stage_years[s] = Some(years);
            }
            self.progress.best_stage = stage;
            self.progress.stage_since_years = Some(years);
            if self.progress.accelerator_on {
                self.progress.accelerator_on = false;
                self.events.push_with(
                    years,
                    None,
                    EventKind::AcceleratorOff { pathway: PHOTOSYNTHESIS_PATHWAY },
                    Origin::Accelerator,
                    Some(id),
                );
            }
        }
        // Détecteur de stagnation : l'accélérateur n'agit qu'en dernier recours.
        let acc = &self.config.evolution.accelerator;
        if acc.enabled && !self.progress.accelerator_on && self.progress.best_stage < 4 {
            if let Some(since) = self.progress.stage_since_years {
                if years - since >= acc.patience_years && self.communities.iter().any(|v| !v.is_empty()) {
                    self.progress.accelerator_on = true;
                    let stage = self.progress.best_stage;
                    self.events.push_with(
                        years,
                        None,
                        EventKind::AcceleratorOn { pathway: PHOTOSYNTHESIS_PATHWAY, stage },
                        Origin::Accelerator,
                        None,
                    );
                }
            }
        }

        // Seuils d'oxygène de l'atmosphère, reliés à la photosynthèse oxygénique.
        let o2 = self.planet.reservoirs.mixing_ratio(Gas::O2);
        while self.progress.oxygen_level < OXYGEN_THRESHOLDS.len() && o2 >= OXYGEN_THRESHOLDS[self.progress.oxygen_level] {
            let cause = self.progress.stage_events[4];
            let level = OXYGEN_THRESHOLDS[self.progress.oxygen_level];
            self.events.push_with(years, None, EventKind::OxygenThreshold { mixing_ratio: level, rising: true }, Origin::Engine, cause);
            self.progress.oxygen_level += 1;
        }
        while self.progress.oxygen_level > 0 && o2 < 0.5 * OXYGEN_THRESHOLDS[self.progress.oxygen_level - 1] {
            self.progress.oxygen_level -= 1;
            let level = OXYGEN_THRESHOLDS[self.progress.oxygen_level];
            self.events.push(years, None, EventKind::OxygenThreshold { mixing_ratio: level, rising: false });
        }
        // Glaciations globales.
        let ice = self.planet.climate.ice_fraction;
        if !self.progress.snowball && ice > 0.95 {
            self.progress.snowball = true;
            self.events.push(years, None, EventKind::Snowball { ice_fraction: ice, starts: true });
        } else if self.progress.snowball && ice < 0.5 {
            self.progress.snowball = false;
            self.events.push(years, None, EventKind::Snowball { ice_fraction: ice, starts: false });
        }

        if self.history.due(years) {
            self.record_history();
        }
        self.publish();
    }

    /// Grandeurs globales à la date courante.
    pub fn sample(&self) -> Sample {
        let r = &self.planet.reservoirs;
        let mut guilds: Vec<u32> = self.communities.iter().flatten().map(|p| p.signature()).collect();
        guilds.sort_unstable();
        guilds.dedup();
        let stage = self.communities.iter().flatten().map(|p| photosynthesis_stage(&p.phenotype)).max().unwrap_or(0);
        Sample {
            years: self.years,
            o2_mixing: r.mixing_ratio(Gas::O2),
            co2_pa: self.planet.partial_pressure(Gas::Co2),
            ch4_ppb: r.mixing_ratio(Gas::Ch4) * 1e9,
            pressure_pa: r.pressure_pa(self.planet.params.gravity(), self.planet.params.surface_area()),
            mean_temperature_k: self.planet.climate.mean_temperature_k,
            ice_fraction: self.planet.climate.ice_fraction,
            ocean_fraction: self.planet.ocean_fraction(),
            biomass: self.biomass(),
            living_lineages: self.lineages.living_count(),
            guilds: guilds.len(),
            photosynthesis_stage: stage,
            o2_production: self.oxygen_production,
            o2_release: r.last.oxygen_release,
            o2_sinks: r.last.oxygen_sinks,
            organic_burial: r.last.organic_burial,
            accelerator_on: self.progress.accelerator_on,
        }
    }

    fn record_history(&mut self) {
        let s = self.sample();
        self.history.push(s);
    }

    /// Publie l'état du monde à la fin du pas (photographie immuable).
    fn publish(&mut self) {
        let globals = self.history.last().filter(|s| s.years == self.years).copied().unwrap_or_else(|| self.sample());
        let cells = (0..self.planet.cells.len())
            .map(|c| {
                let env = &self.planet.cells[c];
                let pops = &self.communities[c];
                let dominant = pops.iter().max_by(|a, b| a.biomass.total_cmp(&b.biomass));
                let photo = pops.iter().filter(|p| p.phenotype.pigment_nm.is_some()).max_by(|a, b| a.biomass.total_cmp(&b.biomass));
                CellView {
                    elevation_m: env.elevation_m as f32,
                    temperature_k: env.temperature_k as f32,
                    is_ocean: env.is_ocean,
                    ice: env.ice_cover > 0.5,
                    biomass: pops.iter().map(|p| p.biomass).sum::<f64>() as f32,
                    dominant_guild: dominant.map_or(0, |p| p.signature()),
                    pigment_rgb: photo.and_then(|p| p.phenotype.pigment_nm).map(pigment_colour),
                    oxygen: self.chemistry[c][WaterPool::O2 as usize] as f32,
                    plate: self.planet.tectonics.parcel_of(c).plate,
                }
            })
            .collect();
        self.publication.publish(PublishedState {
            step: self.stats.steps,
            years: self.years,
            step_years: self.config.step_years,
            climate_mode: ClimateMode::for_step(self.config.step_years),
            globals,
            cells,
        });
    }

    /// Empreinte de l'état complet (rejeu à l'identique).
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv::default();
        h.f(self.years);
        h.u(self.stats.steps);
        for chem in &self.chemistry {
            for &x in chem {
                h.f(x);
            }
        }
        for (c, pops) in self.communities.iter().enumerate() {
            h.u(c as u64);
            for p in pops {
                h.u(p.lineage as u64);
                h.u(p.signature() as u64);
                h.f(p.biomass);
                for g in &p.genome.genes {
                    h.f(g.domain.efficiency);
                    h.f(g.domain.absorption_nm);
                }
            }
        }
        let r = &self.planet.reservoirs;
        for &x in &r.atmosphere {
            h.f(x);
        }
        for x in [r.deep_fe2, r.deep_mn2, r.deep_po4, r.carbonate_c, r.organic_c, r.sediment_p, self.planet.climate.obliquity_rad] {
            h.f(x);
        }
        for e in &self.planet.cells {
            h.f(e.elevation_m);
            h.f(e.temperature_k);
        }
        h.u(self.events.events.len() as u64);
        h.u(self.lineages.records.len() as u64);
        h.u(self.journal.total());
        h.0
    }

    /// Résumé lisible de l'état du monde.
    pub fn summary(&self) -> Summary {
        let mut guilds: BTreeMap<u32, (usize, f64)> = BTreeMap::new();
        let (mut mismatch, mut weight) = (0.0, 0.0);
        for (c, pops) in self.communities.iter().enumerate() {
            for p in pops {
                let e = guilds.entry(p.signature()).or_default();
                e.0 += 1;
                e.1 += p.biomass;
                if let Some(t) = used_t_opt(p) {
                    mismatch += p.biomass * (t - self.planet.cells[c].temperature_k).abs();
                    weight += p.biomass;
                }
            }
        }
        let ocean: Vec<usize> = self.planet.ocean_cells().collect();
        let colonised = ocean.iter().filter(|&&c| !self.communities[c].is_empty()).count();
        let volume: f64 = ocean.iter().map(|&c| self.planet.cells[c].water_volume_m3).sum();
        let mut mean_chem = [0.0; WATER_POOL_COUNT];
        for &c in &ocean {
            let v = self.planet.cells[c].water_volume_m3;
            for (m, x) in mean_chem.iter_mut().zip(self.chemistry[c]) {
                *m += x * v / volume;
            }
        }
        Summary {
            years: self.years,
            ocean_cells: ocean.len(),
            colonised_cells: colonised,
            populations: self.communities.iter().map(Vec::len).sum(),
            biomass: self.biomass(),
            guilds,
            lineages_total: self.lineages.records.len(),
            lineages_living: self.lineages.living_count(),
            thermal_mismatch_k: if weight > 0.0 { mismatch / weight } else { f64::NAN },
            mean_genes: mean(self.communities.iter().flatten().map(|p| p.genome.genes.len() as f64)),
            mean_functional_genes: mean(self.communities.iter().flatten().map(|p| p.genome.functional_genes().count() as f64)),
            mean_chemistry: mean_chem,
            carbon_error: self.carbon_balance_error(),
            phosphorus_error: self.phosphorus_balance_error(),
            globals: self.sample(),
            stats: self.stats,
        }
    }

    /// Mémoire occupée par l'état de la simulation, en octets (estimation par
    /// la taille des structures ; les génomes partagés sont comptés une fois).
    pub fn memory_bytes(&self) -> MemoryReport {
        let mut seen = std::collections::HashSet::new();
        let mut genomes = 0usize;
        for p in self.communities.iter().flatten() {
            if seen.insert(Arc::as_ptr(&p.genome) as usize) {
                genomes += p.genome.memory_bytes() + std::mem::size_of::<Phenotype>() + p.phenotype.enzymes.capacity() * 40;
            }
        }
        let published = self.publication.current.as_ref().map_or(0, |s| s.cells.capacity() * std::mem::size_of::<CellView>());
        MemoryReport {
            planet: self.planet.memory_bytes() + self.chemistry.capacity() * std::mem::size_of::<WaterChemistry>(),
            populations: self
                .communities
                .iter()
                .map(|v| std::mem::size_of::<Vec<Population>>() + v.capacity() * std::mem::size_of::<Population>())
                .sum(),
            genomes,
            distinct_genomes: seen.len(),
            lineages: self.lineages.memory_bytes(),
            journal: self.journal.memory_bytes(),
            published: 2 * published,
        }
    }
}

/// Température optimale moyenne des enzymes réellement utilisées (voie
/// active dans la cellule) : les gènes d'une voie sans substrat dérivent
/// librement et ne disent rien de l'adaptation.
fn used_t_opt(p: &Population) -> Option<f64> {
    let (mut w, mut t) = (0.0, 0.0);
    for e in &p.phenotype.enzymes {
        if p.rates.reaction[e.reaction as usize] > 0.0 {
            w += e.efficiency;
            t += e.efficiency * e.t_opt_k;
        }
    }
    (w > 0.0).then(|| t / w)
}

fn origin_of(cause: GenomeChangeCause) -> Origin {
    if cause == GenomeChangeCause::Accelerator {
        Origin::Accelerator
    } else {
        Origin::Engine
    }
}

/// Flux annuels d'une cellule à prolonger sur le reste du pas : la couche
/// d'eau est supposée à l'équilibre, donc ce qui entre en carbone en ressort
/// (même chose pour le phosphore). Le côté le plus fort est ramené au plus
/// faible ; l'O₂ libéré suit le carbone fixé quand les entrées dominent.
pub fn balanced_rates(moles: &[f64; WATER_POOL_COUNT], t_eco: f64) -> [f64; WATER_POOL_COUNT] {
    let mut r = moles.map(|m| m / t_eco);
    for atoms in [WaterPool::carbon_atoms as fn(WaterPool) -> f64, WaterPool::phosphorus_atoms] {
        let (mut out, mut inn) = (0.0, 0.0);
        for p in WATER_POOLS {
            let x = r[p as usize] * atoms(p);
            if x > 0.0 {
                out += x;
            } else {
                inn -= x;
            }
        }
        if out <= 0.0 && inn <= 0.0 {
            continue;
        }
        let carbon = atoms(WaterPool::Dic) > 0.0;
        if out > inn {
            let k = inn / out;
            for p in WATER_POOLS {
                if atoms(p) > 0.0 && r[p as usize] > 0.0 {
                    r[p as usize] *= k;
                }
            }
        } else {
            let k = out / inn;
            for p in WATER_POOLS {
                if atoms(p) > 0.0 && r[p as usize] < 0.0 {
                    r[p as usize] *= k;
                }
            }
            if carbon && r[WaterPool::O2 as usize] > 0.0 {
                r[WaterPool::O2 as usize] *= k;
            }
        }
    }
    r
}

/// Corrige de `delta` équivalents d'O₂ par an des flux équilibrés en carbone
/// et en phosphore, sans toucher à ces bilans. Trop de pouvoir réducteur
/// exporté (`delta` < 0) : une part du méthane sort oxydée en CO₂, puis l'H₂
/// exporté baisse, puis l'O₂ exporté. Trop d'oxydant : l'O₂ exporté baisse
/// (ou l'O₂ importé augmente).
pub fn close_electrons(rates: &mut [f64; WATER_POOL_COUNT], mut delta: f64) {
    if delta < 0.0 {
        let ch4 = WaterPool::Ch4 as usize;
        let x = rates[ch4].max(0.0).min(-delta / 2.0);
        rates[ch4] -= x;
        rates[WaterPool::Dic as usize] += x;
        delta += 2.0 * x;
        let h2 = WaterPool::H2 as usize;
        let y = rates[h2].max(0.0).min(-delta / 0.5);
        rates[h2] -= y;
        delta += 0.5 * y;
    }
    rates[WaterPool::O2 as usize] += delta;
}

#[derive(Default)]
struct Fnv(u64);

impl Fnv {
    fn u(&mut self, x: u64) {
        if self.0 == 0 {
            self.0 = 0xcbf2_9ce4_8422_2325;
        }
        for b in x.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn f(&mut self, x: f64) {
        self.u(x.to_bits());
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryReport {
    pub planet: usize,
    pub populations: usize,
    pub genomes: usize,
    pub distinct_genomes: usize,
    pub lineages: usize,
    pub journal: usize,
    pub published: usize,
}

#[derive(Clone, Debug)]
pub struct Summary {
    pub years: f64,
    pub ocean_cells: usize,
    pub colonised_cells: usize,
    pub populations: usize,
    pub biomass: f64,
    /// Par guilde (signature) : nombre de populations et biomasse.
    pub guilds: BTreeMap<u32, (usize, f64)>,
    pub lineages_total: usize,
    pub lineages_living: usize,
    /// Écart moyen entre l'optimum thermique des enzymes et la température
    /// locale, pondéré par la biomasse, K.
    pub thermal_mismatch_k: f64,
    /// Nombre moyen de gènes par population, dont fonctionnels.
    pub mean_genes: f64,
    pub mean_functional_genes: f64,
    pub mean_chemistry: WaterChemistry,
    pub carbon_error: f64,
    pub phosphorus_error: f64,
    pub globals: Sample,
    pub stats: WorldStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_world(seed: u64) -> World {
        let mut cfg = WorldConfig::new(seed, 3);
        cfg.step_years = 1000.0;
        let mut w = World::new(cfg);
        w.seed_life();
        w
    }

    #[test]
    fn same_seed_same_history() {
        let mut a = small_world(9);
        let mut b = small_world(9);
        for _ in 0..15 {
            a.step();
            b.step();
        }
        assert_eq!(a.stats, b.stats);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.events.events, b.events.events);
    }

    #[test]
    fn determinism_does_not_depend_on_thread_count() {
        let run = |threads: usize| {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
            pool.install(|| {
                let mut cfg = WorldConfig::new(4, 3);
                cfg.step_years = 200_000.0;
                let mut w = World::new(cfg);
                w.seed_life();
                for _ in 0..12 {
                    w.step();
                }
                (w.state_hash(), w.stats)
            })
        };
        assert_eq!(run(1), run(4));
    }

    #[test]
    fn carbon_and_phosphorus_are_conserved_through_tectonics() {
        let mut cfg = WorldConfig::new(3, 3);
        cfg.step_years = 250_000.0;
        let mut w = World::new(cfg);
        w.seed_life();
        let tectonic_steps = w.planet.tectonics.steps;
        for _ in 0..24 {
            w.step();
            assert!(w.carbon_balance_error() < 1e-9, "carbone : écart {}", w.carbon_balance_error());
            assert!(w.phosphorus_balance_error() < 1e-9, "phosphore : écart {}", w.phosphorus_balance_error());
        }
        assert!(w.planet.tectonics.steps >= tectonic_steps + 5, "la tectonique a tourné");
    }

    #[test]
    fn life_spreads_and_adapts_to_local_temperature() {
        let mut w = small_world(1);
        let start = w.summary();
        for _ in 0..20 {
            w.step();
        }
        let end = w.summary();
        assert!(end.colonised_cells > 3 * start.colonised_cells, "{} → {}", start.colonised_cells, end.colonised_cells);
        assert!(end.stats.substitutions > 0);
        assert_eq!(w.journal.total(), end.stats.substitutions);
        // L'adaptation thermique demande plus de générations que la
        // colonisation.
        for _ in 0..40 {
            w.step();
        }
        let end = w.summary();
        // Écart à la température locale des enzymes utilisées, comparé à
        // celui qu'aurait gardé l'ancêtre (optimum à 300 K) aux mêmes endroits.
        let (mut a, mut b) = (0.0, 0.0);
        for (c, pops) in w.communities.iter().enumerate() {
            for p in pops.iter().filter(|p| used_t_opt(p).is_some()) {
                a += p.biomass * (300.0 - w.planet.cells[c].temperature_k).abs();
                b += p.biomass;
            }
        }
        let ancestral = a / b;
        assert!(
            end.thermal_mismatch_k < 0.75 * ancestral,
            "écart thermique {} K contre {} K pour l'ancêtre",
            end.thermal_mismatch_k,
            ancestral
        );
    }

    #[test]
    fn replay_from_seed_and_orders_gives_the_same_state() {
        let mut cfg = WorldConfig::new(11, 3);
        cfg.step_years = 50_000.0;
        let mut a = World::new(cfg.clone());
        a.orders.submit(0.0, OrderKind::SeedLife);
        a.orders.submit(150_000.0, OrderKind::SetStepYears(20_000.0));
        a.orders.submit(200_000.0, OrderKind::AddPhosphate { moles: 1e14 });
        a.orders.submit(240_000.0, OrderKind::InjectGas { gas: Gas::Co2, moles: 1e16 });
        a.orders.submit(260_000.0, OrderKind::Pause);
        a.orders.submit(260_000.0, OrderKind::Resume);
        a.orders.submit(300_000.0, OrderKind::MarkLineage { lineage: 0 });
        for _ in 0..20 {
            a.step();
        }
        assert_eq!(a.orders.applied.len(), 7);
        assert!(a.carbon_balance_error() < 1e-9 && a.phosphorus_balance_error() < 1e-9);
        // Un ordre soumis en cours de partie est rejoué à la même date.
        let late = a.orders.submit(a.years, OrderKind::InjectGas { gas: Gas::Ch4, moles: 1e13 });
        for _ in 0..5 {
            a.step();
        }
        assert!(a.orders.applied.iter().any(|o| o.order.id == late));

        let mut b = World::replay(cfg.clone(), &a.orders.log());
        for _ in 0..25 {
            b.step();
        }
        assert_eq!(a.years, b.years);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.orders.applied, b.orders.applied);

        // Sans les ordres, l'histoire diffère.
        let mut c = World::new(cfg);
        c.seed_life();
        for _ in 0..25 {
            c.step();
        }
        assert_ne!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn publication_keeps_the_previous_state() {
        let mut w = small_world(5);
        w.step();
        let first = w.publication.current.clone().unwrap();
        w.step();
        let prev = w.publication.previous.clone().unwrap();
        assert!(Arc::ptr_eq(&first, &prev));
        let cur = w.publication.current.clone().unwrap();
        assert!(cur.years > prev.years);
        assert_eq!(cur.cells.len(), w.planet.cells.len());
        assert!(cur.cells.iter().any(|c| c.biomass > 0.0));
    }

    #[test]
    fn electron_closure_keeps_carbon_and_reaches_target() {
        let ox = |r: &[f64; WATER_POOL_COUNT]| WATER_POOLS.iter().map(|&p| r[p as usize] * p.oxidant_equivalents()).sum::<f64>();
        let carbon = |r: &[f64; WATER_POOL_COUNT]| WATER_POOLS.iter().map(|&p| r[p as usize] * p.carbon_atoms()).sum::<f64>();
        let mut r = [0.0; WATER_POOL_COUNT];
        r[WaterPool::Ch4 as usize] = 3.0;
        r[WaterPool::Dic as usize] = -3.0;
        r[WaterPool::H2 as usize] = 1.0;
        for target in [-4.0, -6.5, 1.0] {
            let mut x = r;
            let delta = target - ox(&x);
            close_electrons(&mut x, delta);
            assert!((ox(&x) - target).abs() < 1e-12, "{target}");
            assert!(carbon(&x).abs() < 1e-12);
            assert!(x[WaterPool::Ch4 as usize] >= 0.0 && x[WaterPool::H2 as usize] >= 0.0);
        }
    }

    #[test]
    fn balanced_rates_close_carbon_and_phosphorus() {
        let mut m = [0.0; WATER_POOL_COUNT];
        m[WaterPool::Dic as usize] = -10.0;
        m[WaterPool::Doc as usize] = 4.0;
        m[WaterPool::Ch4 as usize] = 2.0;
        m[WaterPool::O2 as usize] = 9.0;
        m[WaterPool::Po4 as usize] = 0.5;
        let r = balanced_rates(&m, 2.0);
        let c: f64 = WATER_POOLS.iter().map(|&p| r[p as usize] * p.carbon_atoms()).sum();
        assert!(c.abs() < 1e-12);
        assert!((r[WaterPool::Dic as usize] + 3.0).abs() < 1e-12);
        assert!((r[WaterPool::O2 as usize] - 2.7).abs() < 1e-12);
        assert_eq!(r[WaterPool::Po4 as usize], 0.0);
    }
}
