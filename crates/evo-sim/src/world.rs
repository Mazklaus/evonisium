//! Le monde des étapes 2 et 3 : une planète vivante peuplée de microbes.
//!
//! Depuis l'étape 3, le vivant est calculé sur la grille du vivant
//! ([`BioGrid`], niveau 5 sous une planète de niveau 6) et évolue par dème
//! (module `evolution`). Les cellules du vivant ont une couche d'eau dès
//! qu'une de leurs cellules physiques en a une : mer, lacs ou sols humides.
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

use crate::disturbance::{self, Barrier, ClimateAnomaly, Disturbances};
use crate::evolution::{evolve_deme, habitat_class, EvolutionParams, EvolutionStats, Fixation};
use crate::history::{CellView, ClimateMode, EventView, History, Publication, PublishedState, Sample, SpeciesView};
use crate::influence::{InfluenceParams, InfluenceReserve, InfluenceView};
use crate::observation::{Focus, FocusCell, InterestZone, PopulationView, MAX_FOCUS_CELLS};
use crate::orders::{AppliedOrder, Intervention, Order, OrderKind, OrderQueue};
use crate::transitions::TransitionParams;
use evo_core::events::{EventKind, EventLog, Origin};
use evo_core::flux::{Element, FluxRegistry};
use evo_core::math::Det;
use evo_core::rng::{rng_for, Stream};
use evo_genetics::genome::MARKER_LEN;
use evo_genetics::{
    Domain, DomainFamily, Gene, Genome, GenomeChangeCause, GenomeJournal, JournalEntry, LineageRegistry, MutationParams, OriginFixation,
    GENOME_CHANGE_CAUSE_COUNT,
};
use evo_life::community::{capacities, evaluate, substep_with, CellContext, Population};
use evo_life::metabolism::{
    domain_relations, guild_label, photosynthesis_stage, FERMENTATION, METHANOGENESIS, PHOTOSYNTHESIS_PATHWAY, PHOTOSYNTHESIS_STAGES,
    REACTION_COUNT, RHODOPSIN_PATHWAY,
};
use evo_life::{growth_rates, pigment_colour, selection_coefficient, LightSpectrum, Phenotype, Physiology};
use evo_planet::generate::generate;
use evo_planet::{BioGrid, Gas, Planet, PlanetParams, WaterChemistry, WaterPool, TECTONIC_STEP_YEARS, WATER_POOLS, WATER_POOL_COUNT};
use rand::Rng;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Où déposer les cellules minimales au départ.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Seeding {
    /// Près des sources hydrothermales (choix par défaut du moteur).
    Vents,
    /// Dans toutes les cellules océaniques (mesures de charge).
    AllOcean,
}

/// Ordre d'éviction sous le plafond de populations par cellule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Eviction {
    /// La plus petite biomasse d'abord (règle de l'étape 3).
    Biomass,
    /// La plus basse fitness d'invasion d'abord : le r de chaque population
    /// dans la communauté résidente, ressources déjà consommées (étape 4).
    InvasionFitness,
}

/// Pas adaptatif : quand un pas s'achève sans événement notable et sans
/// variation rapide de l'oxygène, le suivant s'allonge d'un pas demandé,
/// jusqu'à `max_factor` fois ; le moindre événement notable le ramène au
/// pas demandé. La décision ne dépend que de l'état simulé : le rejeu reste
/// identique.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AdaptiveStep {
    pub max_factor: u32,
    /// Score d'intérêt à partir duquel un événement est notable.
    pub notable_interest: f64,
    /// Variation relative de l'oxygène de l'air tolérée sur un pas calme.
    pub max_oxygen_change: f64,
    /// Sous ce pas demandé (années), le pas reste fixe : le joueur regarde
    /// de près.
    pub min_step_years: f64,
}

impl Default for AdaptiveStep {
    fn default() -> Self {
        Self { max_factor: 3, notable_interest: 0.5, max_oxygen_change: 0.1, min_step_years: 10_000.0 }
    }
}

/// Seuils d'oxygène atmosphérique signalés (fraction molaire). L'atmosphère
/// actuelle de la Terre en contient 0,21 ; la « grande oxydation » la fait
/// passer de moins de 10⁻⁶ à plus de 10⁻³.
pub const OXYGEN_THRESHOLDS: [f64; 5] = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldConfig {
    pub seed: u64,
    /// Niveau de subdivision de la grille physique (6 : 40 962 cellules).
    pub level: u32,
    /// Niveau de la grille du vivant (par défaut un de moins que la planète :
    /// 10 242 cellules sous une planète de niveau 6).
    pub bio_level: u32,
    /// Niveau de la grille des dèmes (par défaut un de moins que le vivant :
    /// environ 4 cellules du vivant par maille).
    pub deme_level: Option<u32>,
    /// Réserve d'influence du joueur.
    pub influence: InfluenceParams,
    pub planet: PlanetParams,
    /// Durée d'un pas planétaire au départ, années (les ordres de vitesse la
    /// changent).
    pub step_years: f64,
    /// Allongement du pas aux périodes calmes (aucun si `None`).
    pub adaptive_step: Option<AdaptiveStep>,
    /// Sous-pas écologiques par pas planétaire et leur durée, années.
    pub eco_substeps: usize,
    pub eco_dt_years: f64,
    pub physiology: Physiology,
    pub mutation: MutationParams,
    pub regime: OriginFixation,
    pub evolution: EvolutionParams,
    /// Grandes transitions de l'étape 4 (endosymbiose, sexe).
    pub transitions: TransitionParams,
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
    /// Ordre d'éviction sous ce plafond.
    pub eviction: Eviction,
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
            bio_level: level.saturating_sub(1),
            deme_level: None,
            influence: InfluenceParams::default(),
            planet,
            step_years: 100_000.0,
            adaptive_step: Some(AdaptiveStep::default()),
            eco_substeps: 30,
            eco_dt_years: 1.0 / 3650.0,
            physiology: Physiology::default(),
            mutation: MutationParams { reaction_count: REACTION_COUNT as u8, relations: domain_relations(), ..Default::default() },
            regime: OriginFixation::default(),
            evolution: EvolutionParams::default(),
            transitions: TransitionParams::default(),
            migration_rate: 1.0,
            extinction_biomass: 1.0,
            founder_biomass: 100.0,
            light_biomass_per_m2: 0.1,
            max_populations_per_cell: 8,
            eviction: Eviction::Biomass,
            seeding: Seeding::Vents,
            seed_biomass: 1e4,
            history_every_years: 1e6,
        }
    }
}

/// Compteurs cumulés depuis le début de la partie.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorldStats {
    pub steps: u64,
    pub substitutions: u64,
    pub new_lineages: u64,
    pub colonisations: u64,
    pub migrant_replacements: u64,
    pub local_extinctions: u64,
    /// Lignées éteintes sans lignée fille (comptées, sans événement).
    pub leaf_extinctions: u64,
    /// Écart des électrons des flux prolongés des couches à leurs sources,
    /// mol d'équivalent O₂ cumulées : des arrondis seulement, rien n'est
    /// corrigé (voir `steady_rates`).
    pub redox_correction: f64,
    /// Pouvoir oxydant déplacé quand une boîte vide freine un prélèvement des
    /// couches de surface (voir `evo_planet::geochem::pair_throttled`), mol
    /// d'équivalent O₂.
    pub redox_throttled: f64,
    /// Part de ce pouvoir oxydant qu'aucun flux de la couche n'a pu reprendre
    /// (non appliquée, elle sort du système suivi et est inscrite au
    /// registre).
    pub redox_unpaired: f64,
    /// Génomes mutants construits et évalués (mutation, phénotype, r, s).
    pub genetic_evaluations: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    /// Substitutions avantageuses (coefficient de sélection positif) ; les
    /// autres sont neutres.
    pub adaptive_substitutions: u64,
    /// Mutants innovants apparus (tirage de Poisson), dont ceux ajoutés par
    /// l'accélérateur, et ceux qui ont été évalués.
    pub innovations_drawn: u64,
    pub innovations_accelerated: u64,
    pub innovations_evaluated: u64,
    /// Cellules du vivant peuplées, et cellules qui dépassaient le plafond
    /// de populations avant éviction, cumulées sur les pas.
    pub occupied_cell_steps: u64,
    pub saturated_cell_steps: u64,
    /// Cellules où l'éviction a retiré une population établie (plus que la
    /// biomasse d'un fondateur), et non seulement des arrivants du pas.
    pub established_eviction_cell_steps: u64,
    /// Parmi elles, celles où la population établie évincée croissait
    /// encore (r > 0).
    pub growing_eviction_cell_steps: u64,
    /// Modifications de génome fixées, par cause ([`GenomeChangeCause::index`]).
    pub fixed_changes_by_cause: [u64; GENOME_CHANGE_CAUSE_COUNT],
    /// Cellules passées de l'océan à la terre et inversement.
    pub cells_emerged: u64,
    pub cells_flooded: u64,
    /// Pas pendant lesquels l'accélérateur a agi.
    pub accelerator_steps: u64,
    /// Mutations réunies par recombinaison chez les sexués.
    pub recombinations: u64,
}

impl WorldStats {
    fn add_evolution(&mut self, o: &EvolutionStats) {
        self.substitutions += o.substitutions;
        self.genetic_evaluations += o.genetic_evaluations;
        self.tunnel_attempts += o.tunnel_attempts;
        self.tunnel_successes += o.tunnel_successes;
        self.innovations_drawn += o.innovations_drawn;
        self.innovations_accelerated += o.innovations_accelerated;
        self.innovations_evaluated += o.innovations_evaluated;
        self.recombinations += o.recombinations;
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
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Multiple du pas demandé utilisé au prochain pas (pas adaptatif ;
    /// 0 ou 1 : le pas demandé).
    pub step_factor: u32,
    /// Première apparition de chaque étape de la complexité
    /// ([`COMPLEXITY_STAGES`]) : date et événement.
    pub complexity_years: [Option<f64>; COMPLEXITY_STAGE_COUNT],
    pub complexity_events: [Option<u64>; COMPLEXITY_STAGE_COUNT],
    /// Accélérateur des transitions de la complexité (endosymbiose
    /// facilitée, innovations plus fréquentes), et date du dernier progrès.
    pub complex_accelerator_on: bool,
    pub complexity_since_years: Option<f64>,
    /// Temps simulé écoulé depuis le dernier tour d'évolution, années : les
    /// tours se comptent sur le temps, pas sur les pas, pour que l'histoire
    /// ne dépende pas du pas choisi par le joueur.
    pub evolution_clock: f64,
}

/// Étapes de la complexité suivies et signalées (chemin des « cellules
/// complexes ») ; chacune est datée à sa première apparition sur la planète,
/// dans l'ordre où elle survient.
pub const COMPLEXITY_PATHWAY: &str = "cellule complexe";
pub const COMPLEXITY_STAGE_COUNT: usize = 8;
pub const COMPLEXITY_STAGES: [&str; COMPLEXITY_STAGE_COUNT] = [
    "phagotrophie",
    "eucaryote (endosymbiose)",
    "plaste",
    "reproduction sexuée",
    "colonie clonale",
    "deux types cellulaires",
    "eucaryote multicellulaire à deux types cellulaires",
    "eucaryote multicellulaire complexe hors de l'eau",
];

/// Étapes de la complexité atteintes par une population (bits, dans l'ordre
/// de [`COMPLEXITY_STAGES`]) ; `on_land` : sa cellule n'est pas océanique.
pub fn complexity_bits(p: &Phenotype, on_land: bool) -> u8 {
    let types = p.cell_types() >= 2 && p.is_multicellular();
    let mut bits = 0u8;
    bits |= u8::from(p.is_phagotroph());
    bits |= u8::from(p.is_eukaryote()) << 1;
    bits |= u8::from(p.plastids > 0) << 2;
    bits |= u8::from(p.sexual) << 3;
    bits |= u8::from(p.is_multicellular()) << 4;
    bits |= u8::from(types) << 5;
    bits |= u8::from(types && p.is_eukaryote()) << 6;
    bits |= u8::from(types && p.is_eukaryote() && on_land && p.is_terrestrial()) << 7;
    bits
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
    /// Grille du vivant ; `communities` et `chemistry` sont indexés par ses
    /// cellules.
    pub bio: BioGrid,
    /// Cellules du vivant de chaque dème, et dème de chaque cellule.
    pub demes: Vec<Vec<u32>>,
    pub deme_index: Vec<u32>,
    pub influence: InfluenceReserve,
    /// Barrières et anomalies climatiques posées par le joueur (étape 4).
    pub disturbances: Disturbances,
    /// Zone d'intérêt de la caméra (canal d'observation, hors histoire).
    pub interest: Option<InterestZone>,
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
    /// Flux de la surface vers les réservoirs au pas précédent, et apport
    /// des sources hydrothermales de surface qu'ils contiennent, mol·an⁻¹
    /// (amortissement du couplage, voir `ecology_phase`).
    previous_rates: Option<SurfaceRates>,
    pub stats: WorldStats,
    pub timings: PhaseTimings,
    /// Premier événement non encore publié.
    published_events: usize,
    /// Cellule d'origine de chaque signature (première lignée qui la porte),
    /// et lignées déjà parcourues ; recalculé après une reprise.
    species_origin: BTreeMap<u32, u32>,
    origin_scanned: usize,
}

impl World {
    /// Accélération de l'altération par la vie de la terre ferme : 1 sans
    /// elle, `biotic_weathering_max` quand toute la terre ferme est couverte
    /// (couverture d'une cellule : 1 − e^(−B/B₀), B₀ la biomasse qui absorbe
    /// 63 % de la lumière).
    pub fn biotic_weathering(&self) -> f64 {
        let b0 = self.config.light_biomass_per_m2;
        let (mut covered, mut total) = (0.0, 0.0);
        for (env, pops) in self.bio.env.iter().zip(&self.communities) {
            if env.is_ocean || env.dry_area_m2 <= 0.0 {
                continue;
            }
            let land: f64 = pops.iter().filter(|p| p.phenotype.is_terrestrial() && p.phenotype.phototroph).map(|p| p.biomass).sum();
            covered += env.dry_area_m2 * -(-land / (b0 * env.dry_area_m2)).dexp_m1();
            total += env.dry_area_m2;
        }
        let cover = if total > 0.0 { covered / total } else { 0.0 };
        1.0 + (self.planet.params.biotic_weathering_max - 1.0).max(0.0) * cover
    }

    pub fn new(mut config: WorldConfig) -> Self {
        let planet = generate(config.planet.clone(), config.level, config.seed);
        // Les pigments sont jugés sous l'étoile de cette partie, dans l'eau.
        config.physiology.spectrum = LightSpectrum::new(planet.params.star_temperature_k, planet.params.mixed_layer_m);
        config.bio_level = config.bio_level.min(config.level);
        let mut bio = BioGrid::new(&planet.grid, config.bio_level);
        bio.aggregate(&planet.cells);
        let targets = planet.exchange_targets(config.physiology.carbon_to_phosphorus);
        let chemistry: Vec<WaterChemistry> = bio.env.iter().map(|e| planet.equilibrium_chemistry(e, &targets)).collect();
        let n = bio.len();
        let mut world = Self::assemble(config, planet, bio, chemistry, vec![Vec::new(); n]);
        world.flux.set_initial(Element::Carbon, world.total_carbon());
        world.flux.set_initial(Element::Phosphorus, world.total_phosphorus());
        world.flux.set_initial(Element::Electrons, world.total_electrons());
        world.record_history();
        world.publish();
        world
    }

    /// Monde construit à partir de ses parties, compteurs à zéro (création
    /// d'une partie, reprise d'un point de sauvegarde).
    pub(crate) fn assemble(
        config: WorldConfig,
        planet: Planet,
        bio: BioGrid,
        chemistry: Vec<WaterChemistry>,
        communities: Vec<Vec<Population>>,
    ) -> Self {
        let deme_level = config.deme_level.unwrap_or(config.bio_level.saturating_sub(1)).min(config.bio_level);
        let deme_grid = BioGrid::new(&bio.grid, deme_level);
        let (demes, deme_index) = (deme_grid.children, deme_grid.parent);
        Self {
            history: History::new(config.history_every_years),
            influence: InfluenceReserve::new(&config.influence),
            disturbances: Disturbances::default(),
            config,
            planet,
            bio,
            demes,
            deme_index,
            interest: None,
            published_events: 0,
            species_origin: BTreeMap::new(),
            origin_scanned: 0,
            chemistry,
            communities,
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
        }
    }

    pub(crate) fn previous_rates(&self) -> Option<SurfaceRates> {
        self.previous_rates
    }

    pub(crate) fn set_previous_rates(&mut self, r: Option<SurfaceRates>) {
        self.previous_rates = r;
    }

    pub(crate) fn published_events(&self) -> usize {
        self.published_events
    }

    pub(crate) fn set_published_events(&mut self, n: usize) {
        self.published_events = n;
    }

    /// Republie l'état courant sans avancer (après une reprise, un ordre
    /// appliqué en pause ou un déplacement de la zone d'intérêt).
    pub fn republish(&mut self) {
        self.publish();
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
            organelles: Vec::new(),
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
            Seeding::Vents => self.bio.vent_cells(),
            Seeding::AllOcean => self.bio.wet_cells().collect(),
        };
        let first = cells.first().copied().unwrap_or(0) as u32;
        let lineage = self.lineages.found(None, self.years, first, phenotype.signature, genome.clone());
        let event = self.events.push_with(self.years, Some(first), EventKind::LifeSeeded { lineage }, origin, cause);
        let cp = self.config.physiology.carbon_to_phosphorus;
        for c in cells {
            let v = self.bio.env[c].water_volume_m3;
            let chem = &mut self.chemistry[c];
            let b = self.config.seed_biomass.min(0.5 * chem[WaterPool::Dic as usize] * v).min(0.5 * chem[WaterPool::Po4 as usize] * v * cp);
            if b < self.config.extinction_biomass {
                continue;
            }
            chem[WaterPool::Dic as usize] -= b / v;
            chem[WaterPool::Po4 as usize] -= b / cp / v;
            // Les cellules déposées apportent leur matière organique réduite
            // (CH₂O) de l'extérieur du système suivi.
            self.flux.exchange(Element::Electrons, -b);
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
        Planet::water_carbon(&self.bio.env, &self.chemistry) + self.biomass() + self.planet.reservoirs.carbon()
    }

    /// Pouvoir oxydant total du système, mol d'équivalent O₂ : couches d'eau,
    /// biomasse (CH₂O, −1 par carbone) et réservoirs.
    pub fn total_electrons(&self) -> f64 {
        Planet::water_electrons(&self.bio.env, &self.chemistry) - self.biomass() + self.planet.reservoirs.electrons()
    }

    /// Phosphore total du système, mol.
    pub fn total_phosphorus(&self) -> f64 {
        Planet::water_phosphorus(&self.bio.env, &self.chemistry)
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

    /// Écart relatif du bilan des électrons (pouvoir oxydant).
    pub fn electron_balance_error(&self) -> f64 {
        self.flux.relative_error(Element::Electrons, self.total_electrons())
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
                        self.progress.step_factor = 1;
                    }
                }
                OrderKind::Pause => self.paused = true,
                OrderKind::Resume => self.paused = false,
                OrderKind::SeedLife => self.seed_life_with(Origin::Player, Some(event)),
                OrderKind::Intervene(ref i) => {
                    if self.influence.try_spend(&self.config.influence, i.cost()) {
                        self.intervene(i, order.id);
                    } else {
                        self.events.push_with(
                            self.years,
                            Some(i.cell()),
                            EventKind::OrderRefused { order: order.id, reason: "réserve d'influence insuffisante".into() },
                            Origin::Player,
                            Some(event),
                        );
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

    /// Applique une intervention déjà payée.
    fn intervene(&mut self, i: &Intervention, order: u64) {
        match *i {
            Intervention::Fertilize { cell, radius_km, moles_p } => {
                let m = moles_p.max(0.0);
                if m <= 0.0 {
                    return;
                }
                // Cellules du vivant mouillées dans le rayon, au prorata de
                // leur volume d'eau ; sans eau, l'apport va à l'océan profond.
                let center = self.planet.grid.centers[(cell as usize).min(self.planet.grid.len() - 1)];
                let radius = radius_km.max(1.0) * 1e3 / self.planet.params.radius_m;
                let targets: Vec<usize> = self
                    .bio
                    .wet_cells()
                    .filter(|&b| {
                        let p = self.bio.grid.centers[b];
                        (p[0] * center[0] + p[1] * center[1] + p[2] * center[2]).clamp(-1.0, 1.0).dacos() <= radius
                    })
                    .collect();
                let volume: f64 = targets.iter().map(|&b| self.bio.env[b].water_volume_m3).sum();
                if volume > 0.0 {
                    for b in targets {
                        self.chemistry[b][WaterPool::Po4 as usize] += m / volume;
                    }
                } else {
                    self.planet.reservoirs.deep_po4 += m;
                }
                self.flux.exchange(Element::Phosphorus, m);
            }
            Intervention::Eruption { gas, moles, .. } => {
                // L'oxygène ne sort pas des volcans.
                if gas == Gas::O2 {
                    return;
                }
                let r = &mut self.planet.reservoirs.atmosphere[gas as usize];
                let m = moles.max(-*r);
                *r += m;
                if matches!(gas, Gas::Co2 | Gas::Ch4) {
                    self.flux.exchange(Element::Carbon, m);
                }
                let ox = match gas {
                    Gas::Ch4 => -2.0,
                    Gas::H2 => -0.5,
                    _ => 0.0,
                };
                self.flux.exchange(Element::Electrons, ox * m);
            }
            Intervention::Impact { cell, diameter_km } => self.impact(cell, diameter_km, order),
            Intervention::Isolate { cell, azimuth_deg, length_km, duration_years, sea } => {
                let center = self.planet.grid.centers[(cell as usize).min(self.planet.grid.len() - 1)];
                let half = 0.5 * length_km.max(1.0) * 1e3 / self.planet.params.radius_m;
                let azimuth = azimuth_deg.to_radians();
                self.disturbances.barriers.push(Barrier::new(center, azimuth, half, self.years + duration_years.max(0.0), sea, order));
            }
            Intervention::ClimatePulse { cell, radius_km, delta_k, rain_factor, duration_years } => {
                let center = self.planet.grid.centers[(cell as usize).min(self.planet.grid.len() - 1)];
                self.disturbances.anomalies.push(ClimateAnomaly {
                    center,
                    radius: radius_km.max(1.0) * 1e3 / self.planet.params.radius_m,
                    global: false,
                    delta_k: delta_k.clamp(-30.0, 30.0),
                    rain_factor: rain_factor.clamp(0.0, 10.0),
                    from_years: self.years,
                    until_years: self.years + duration_years.max(0.0),
                    order,
                });
            }
        }
    }

    /// Impact météoritique : la vie meurt dans le rayon de dévastation (sa
    /// matière retourne à l'eau), les carbonates touchés libèrent leur CO₂,
    /// et les poussières refroidissent la planète le temps de retomber.
    fn impact(&mut self, cell: u32, diameter_km: f64, order: u64) {
        let d = diameter_km.max(0.0);
        if d <= 0.0 {
            return;
        }
        let pc = (cell as usize).min(self.planet.grid.len() - 1);
        let center = self.planet.grid.centers[pc];
        let radius = disturbance::kill_radius_m(d) / self.planet.params.radius_m;
        let cp = self.config.physiology.carbon_to_phosphorus;
        let mut killed = 0.0;
        for b in 0..self.communities.len() {
            let f = disturbance::kill_fraction(disturbance::angle(self.bio.grid.centers[b], center), radius);
            if f <= 0.0 || self.communities[b].is_empty() {
                continue;
            }
            let v = self.bio.env[b].water_volume_m3;
            let mut dead = 0.0;
            for p in &mut self.communities[b] {
                let x = p.biomass * f;
                p.biomass -= x;
                dead += x;
            }
            if v > 0.0 {
                self.chemistry[b][WaterPool::Doc as usize] += dead / v;
                self.chemistry[b][WaterPool::Po4 as usize] += dead / cp / v;
            } else {
                // Sans eau, la matière morte rejoint les sédiments par la
                // même voie qu'une couche d'eau qui s'assèche.
                let moles = [0.0; WATER_POOL_COUNT];
                self.planet.reservoirs.absorb_layer(&moles, dead, dead / cp, &mut self.flux);
            }
            killed += dead;
        }
        // Cible : plate-forme marine (carbonates), grands fonds ou continent.
        let target = &self.planet.cells[pc];
        let carbonate = match (target.is_ocean, target.elevation_m > -1000.0) {
            (true, true) => 0.8,
            (true, false) => 0.1,
            (false, _) => 0.3,
        };
        let co2 = disturbance::impact_co2(d, carbonate);
        if co2 > 0.0 {
            self.planet.reservoirs.atmosphere[Gas::Co2 as usize] += co2;
            self.flux.exchange(Element::Carbon, co2);
        }
        self.disturbances.anomalies.push(ClimateAnomaly {
            center,
            radius: std::f64::consts::PI,
            global: true,
            delta_k: -disturbance::impact_cooling_k(d),
            rain_factor: 0.7,
            from_years: self.years,
            until_years: self.years + disturbance::impact_winter_years(d),
            order,
        });
        self.disturbances.killed_biomass += killed;
    }

    /// Zone d'intérêt de la caméra (canal d'observation). Ne change jamais
    /// l'histoire : seul l'état publié du prochain pas en tient compte.
    pub fn set_interest(&mut self, zone: Option<InterestZone>) {
        self.interest = zone;
    }

    /// Avance d'un pas planétaire (en pause, n'applique que les ordres dus).
    pub fn step(&mut self) -> PhaseTimings {
        let mut timings = PhaseTimings::default();
        let t0 = Instant::now();
        self.apply_orders();
        if self.paused {
            return timings;
        }
        let dt = self.next_step_years();
        let step_index = self.stats.steps;
        let years = self.years;
        let first_event = self.events.events.len();
        let oxygen_before = self.planet.reservoirs.mixing_ratio(Gas::O2);

        let debug = std::env::var_os("EVO_DEBUG_ELECTRONS").is_some();
        let check = |w: &World, what: &str| {
            if debug {
                let e = w.total_electrons() - w.flux.expected(Element::Electrons);
                eprintln!("{what}: écart {e:.6e}");
            }
        };
        check(self, "début");
        // 1. Planète lente.
        self.planet_phase(years, dt, step_index);
        check(self, "planète");
        timings.planet = t0.elapsed();

        // 2. Écologie et cycles globaux.
        let t1 = Instant::now();
        self.ecology_phase(years, dt);
        check(self, "écologie");
        timings.ecology = t1.elapsed();

        // 3. Évolution.
        let t2 = Instant::now();
        // Un tour par tranche de `round_years` de temps simulé, le reste
        // reporté au pas suivant ; sans durée de tour, un tour par pas.
        let (rounds, sub) = match self.config.evolution.round_years {
            Some(r) if r > 0.0 => {
                let pending = self.progress.evolution_clock + dt;
                let n = (pending / r + 1e-9).floor().max(0.0) as u64;
                self.progress.evolution_clock = (pending - n as f64 * r).max(0.0);
                (n, r)
            }
            _ => (1, dt),
        };
        if self.progress.accelerator_on || self.progress.complex_accelerator_on {
            self.stats.accelerator_steps += 1;
        }
        let mut modified = Vec::new();
        for round in 0..rounds {
            let start = (years + dt - (rounds - round) as f64 * sub).max(years);
            modified.extend(self.evolution_phase(start, sub, step_index, round));
        }
        check(self, "évolution");
        timings.evolution = t2.elapsed();

        // 4. Migration.
        let t3 = Instant::now();
        self.migrate(dt, step_index);
        self.trim_communities();
        check(self, "migration");
        timings.migration = t3.elapsed();

        // 5. Registres.
        let t4 = Instant::now();
        self.years = years + dt;
        self.influence.recharge(&self.config.influence, dt);
        self.disturbances.expire(self.years);
        self.bookkeeping(modified);
        self.adapt_step(first_event, oxygen_before);
        if std::env::var_os("EVO_DEBUG_O2").is_some() && (self.years / 2e7).floor() > (years / 2e7).floor() {
            let r = &self.planet.reservoirs;
            let b = r.oxygen;
            let (mut n, mut sum, mut silent, mut max) = (0.0f64, 0.0f64, 0.0f64, 0usize);
            for p in self.communities.iter().flatten() {
                n += p.biomass;
                sum += p.biomass * p.genome.genes.len() as f64;
                silent += p.biomass * p.genome.genes.iter().filter(|g| !g.functional).count() as f64;
                max = max.max(p.genome.genes.len());
            }
            eprintln!(
                "GENES {:.1} Ma moyenne pondérée {:.1} max {max} inactifs {:.0} %",
                self.years / 1e6,
                sum / n.max(1e-30),
                100.0 * silent / sum.max(1e-30)
            );
            if std::env::var_os("EVO_DEBUG_GENOMES").is_some() {
                // Le plus gros génome : familles, nombre et efficacité moyenne.
                let big: f64 = self.communities.iter().flatten().filter(|p| p.genome.genes.len() > 300).map(|p| p.biomass).sum();
                if let Some(p) = self.communities.iter().flatten().max_by_key(|p| p.genome.genes.len()) {
                    let mut fam: std::collections::BTreeMap<String, (usize, f64, f64)> = Default::default();
                    for g in p.genome.functional_genes() {
                        let e = fam.entry(format!("{:?}", g.domain.family)).or_default();
                        e.0 += 1;
                        e.1 += g.domain.efficiency;
                        e.2 = e.2.max(g.domain.efficiency);
                    }
                    let mut v: Vec<_> = fam.into_iter().collect();
                    v.sort_by_key(|(_, (n, _, _))| std::cmp::Reverse(*n));
                    let txt: Vec<String> =
                        v.iter().take(8).map(|(k, (n, s, m))| format!("{k} {n}×(moy {:.2}, max {m:.2})", s / *n as f64)).collect();
                    eprintln!(
                        "GENOMES {:.1} Ma part >300 gènes {:.1} % ; plus gros {} gènes, biomasse {:.1e}, taille {:.2} : {}",
                        self.years / 1e6,
                        100.0 * big / n.max(1e-30),
                        p.genome.genes.len(),
                        p.biomass,
                        p.phenotype.cell_size,
                        txt.join(", ")
                    );
                }
            }
            let pl = &self.planet;
            eprintln!(
                "TERRES {:.1} Ma terres {:.3} océan {:.3} niveau {:.0} m CO2 {:.0} Pa T {:.1} K bilans C {:.1e} P {:.1e} e {:.1e}",
                self.years / 1e6,
                pl.land_area() / pl.params.surface_area(),
                pl.ocean_fraction(),
                pl.sea_level_m,
                pl.partial_pressure(Gas::Co2),
                self.summary().globals.mean_temperature_k,
                self.carbon_balance_error(),
                self.phosphorus_balance_error(),
                self.electron_balance_error()
            );
            eprintln!(
                "O2 {:.1} Ma f={:.3e} orgC={:.4e} sedP={:.4e} deepP={:.4e} photo={:.4e} rel={:.4e} up={:.4e} deep={:.4e} red={:.4e} ch4={:.4e} femn={:.4e} ow={:.4e} sulf={:.4e} sea={:.4e}",
                self.years / 1e6, r.mixing_ratio(Gas::O2), r.organic_c, r.sediment_p, r.deep_po4, b.photosynthesis, b.surface_release,
                b.surface_uptake, b.deep_respiration, b.reduced_gases, b.methane, b.iron_manganese, b.oxidative_weathering, b.sulfide,
                b.seafloor_oxidation
            );
        }
        self.stats.steps += 1;
        timings.bookkeeping = t4.elapsed();
        self.timings.add(&timings);
        timings
    }

    /// Durée du prochain pas : le pas demandé, allongé aux périodes calmes.
    pub fn next_step_years(&self) -> f64 {
        let base = self.config.step_years;
        match self.config.adaptive_step {
            Some(a) if base >= a.min_step_years => base * self.progress.step_factor.clamp(1, a.max_factor.max(1)) as f64,
            _ => base,
        }
    }

    fn adapt_step(&mut self, first_event: usize, oxygen_before: f64) {
        let Some(a) = self.config.adaptive_step else { return };
        let notable = self.events.events[first_event..].iter().any(|e| e.interest >= a.notable_interest);
        let o2 = self.planet.reservoirs.mixing_ratio(Gas::O2);
        let change = (o2 - oxygen_before).abs() / o2.max(oxygen_before).max(1e-6);
        self.progress.step_factor =
            if notable || change > a.max_oxygen_change { 1 } else { (self.progress.step_factor.max(1) + 1).min(a.max_factor.max(1)) };
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
        let gaussian = (-2.0 * u1.dln()).sqrt() * (std::f64::consts::TAU * u2).dcos();
        planet.climate.drift_obliquity(&planet.params, dt, gaussian);

        let before: Vec<f64> = self.bio.env.iter().map(|e| e.water_volume_m3).collect();
        let changes = planet.refresh(years);
        self.bio.aggregate(&self.planet.cells);
        self.disturbances.apply_to(&mut self.bio, years, years + dt);
        if changes.is_empty() {
            return;
        }
        let cp = self.config.physiology.carbon_to_phosphorus;
        let targets = self.planet.exchange_targets(cp);
        for (c, &old_volume) in before.iter().enumerate() {
            let new_volume = self.bio.env[c].water_volume_m3;
            if old_volume == new_volume {
                continue;
            }
            match (old_volume > 0.0, new_volume > 0.0) {
                (true, false) => {
                    // La couche d'eau disparaît : son contenu et la biomasse
                    // rejoignent les réservoirs et les sédiments.
                    let moles = self.chemistry[c].map(|x| x * old_volume);
                    let biomass: f64 = self.communities[c].iter().map(|p| p.biomass).sum();
                    self.planet.reservoirs.absorb_layer(&moles, biomass, biomass / cp, &mut self.flux);
                    self.stats.local_extinctions += self.communities[c].len() as u64;
                    self.communities[c].clear();
                    self.chemistry[c] = [0.0; WATER_POOL_COUNT];
                    self.stats.cells_emerged += 1;
                }
                (false, true) => {
                    let wanted = self.planet.equilibrium_chemistry(&self.bio.env[c], &targets).map(|x| x * new_volume);
                    let got = self.planet.reservoirs.fill_layer(&wanted, &mut self.flux);
                    self.chemistry[c] = got.map(|m| m / new_volume);
                    self.stats.cells_flooded += 1;
                }
                (true, true) => {
                    // La couche change de volume : les moles restent.
                    let k = old_volume / new_volume;
                    for x in self.chemistry[c].iter_mut() {
                        *x *= k;
                    }
                }
                (false, false) => {}
            }
        }
    }

    fn ecology_phase(&mut self, years: f64, dt: f64) {
        let cfg = &self.config;
        let planet = &self.planet;
        let envs = &self.bio.env;
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
                let env = &envs[c];
                if env.water_volume_m3 <= 0.0 {
                    return r;
                }
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                let start = *chem;
                let biomass_start: f64 = pops.iter().map(|p| p.biomass).sum();
                // La température ne change pas pendant l'écologie du pas : les
                // capacités des enzymes se calculent une fois.
                let caps = capacities(pops, &ctx, &cfg.physiology);
                for _ in 0..cfg.eco_substeps {
                    if !pops.is_empty() {
                        let o = substep_with(pops, &caps, &ctx, chem, cfg.eco_dt_years, &cfg.physiology);
                        r.oxygen += o.oxygen;
                        r.exact[WaterPool::Doc as usize] += o.sinking_carbon;
                        r.exact[WaterPool::Po4 as usize] += o.sinking_carbon / cp;
                    }
                    planet.exchange(env, chem, cfg.eco_dt_years, &targets, &mut r.exact);
                }
                let v = env.water_volume_m3;
                let biomass_change = pops.iter().map(|p| p.biomass).sum::<f64>() - biomass_start;
                r.rates = steady_rates(&r.exact, &start, chem, v, biomass_change, cp, t_eco, cfg.physiology.oxygen_stress_half);
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
                // Bilan des électrons d'une couche à l'équilibre : ce qu'elle
                // exporte de pouvoir réducteur est exactement ce que ses
                // sources hydrothermales lui apportent. On le vérifie ; l'écart
                // n'est fait que d'arrondis.
                let expected: f64 = WATER_POOLS.iter().map(|&p| planet.vent_supply(env, p) * p.oxidant_equivalents()).sum();
                let actual: f64 = WATER_POOLS.iter().map(|&p| r.rates[p as usize] * p.oxidant_equivalents()).sum();
                r.redox_correction = (expected - actual).abs();
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
        // Les sources hydrothermales des couches de surface, pool par pool,
        // apportent leur pouvoir réducteur de l'extérieur du système suivi,
        // sur tout le pas. C'est la seule partie des flux prolongés qui
        // apporte des électrons.
        let vent_pools: [f64; WATER_POOL_COUNT] = std::array::from_fn(|i| {
            let pool = WATER_POOLS[i];
            self.bio.env.iter().filter(|e| e.water_volume_m3 > 0.0).map(|e| self.planet.vent_supply(e, pool)).sum()
        });
        let vents: f64 = WATER_POOLS.iter().map(|&p| vent_pools[p as usize] * p.oxidant_equivalents()).sum();
        // Pendant l'écologie rapide, elles sont mesurées ; le reste du pas,
        // les boîtes inscrivent ce qu'elles reçoivent des couches prolongées.
        self.flux.exchange(Element::Electrons, vents * t_eco.min(dt));
        let mut ctx = self.planet.box_context(years);
        ctx.biotic_weathering = self.biotic_weathering();
        self.planet.reservoirs.apply_exact(&self.planet.params, &ctx, &total.exact, t_eco, &mut self.flux);
        let rest = (dt - t_eco).max(0.0);
        // Les flux d'équilibre de la surface sont tenus constants sur tout le
        // pas, alors que l'atmosphère qu'ils modifient change la vie qui les
        // produit : avec des pas de 200 000 ans, ce couplage explicite oscille
        // d'un pas à l'autre (méthane abondant, puis nul, puis abondant). La
        // réponse de la vie appliquée est donc la moyenne de ce pas et du
        // précédent (schéma amorti qui supprime l'oscillation de période 2
        // sans changer l'équilibre), mais l'apport des sources est celui de ce
        // pas. Chaque jeu de flux a pour bilan d'électrons exactement ses
        // sources, et un carbone et un phosphore nuls : la moyenne aussi.
        let rates = match self.previous_rates {
            Some((prev, prev_vents)) => {
                std::array::from_fn(|i| 0.5 * (total.rates[i] - vent_pools[i]) + 0.5 * (prev[i] - prev_vents[i]) + vent_pools[i])
            }
            None => total.rates,
        };
        self.previous_rates = Some((total.rates, vent_pools));
        // Une boîte vide freine un prélèvement des couches ; ce qu'il
        // alimentait est freiné avec lui, à bilan d'électrons exact. Le
        // pouvoir oxydant ainsi déplacé est compté.
        let (moved, unpaired) = self.planet.reservoirs.integrate(&self.planet.params, &ctx, &rates, rest, &mut self.flux);
        self.stats.redox_throttled += moved;
        self.stats.redox_unpaired += unpaired;
        if debug_rates() {
            let names = ["DIC", "DOC", "H2", "CH4", "O2", "SO4", "H2S", "Fe2", "Mn2", "FeOx", "MnOx", "PO4"];
            let r: Vec<String> = rates.iter().zip(names).map(|(x, n)| format!("{n} {x:.2e}")).collect();
            eprintln!("{:.1} Ma : sources {:.3e} ; {}", years / 1e6, vents * rest, r.join(", "));
        }
    }

    /// Évolution par dème ; renvoie les populations modifiées (cellule,
    /// indice) avec la cause de leur modification, pour la détection des
    /// innovations.
    fn evolution_phase(&mut self, years: f64, dt: f64, step_index: u64, round: u64) -> Vec<(usize, usize, GenomeChangeCause)> {
        let cfg = &self.config;
        let accelerator = self.progress.accelerator_on;
        let complex = self.complexity_boost(years);
        let (communities, chemistry, envs) = (&self.communities, &self.chemistry, &self.bio.env);
        let results: Vec<(Vec<Fixation>, EvolutionStats)> = self
            .demes
            .par_iter()
            .enumerate()
            .map(|(d, cells)| evolve_deme(d, cells, communities, chemistry, envs, cfg, dt, years, step_index, round, accelerator, complex))
            .collect();
        let mut modified = Vec::new();
        for (fixations, stats) in results {
            self.stats.add_evolution(&stats);
            for f in fixations {
                if let Some(m) = self.apply_fixation(years, f) {
                    modified.push(m);
                }
            }
        }
        modified
    }

    /// Applique une fixation d'un dème : au génotype dans toutes les cellules
    /// du dème qui le portent, ou, pour une guilde nouvelle, dans la cellule
    /// représentative.
    fn apply_fixation(&mut self, years: f64, f: Fixation) -> Option<(usize, usize, GenomeChangeCause)> {
        let selection = f.selection;
        let (c, i) = f.rep;
        let resident = &self.communities[c][i];
        let parent_lineage = resident.lineage;
        let old = resident.genome.clone();
        let (index, ok) = if f.phenotype.signature == resident.signature() {
            // Même guilde : le génotype est remplacé partout dans le dème.
            let deme = &self.demes[self.deme_of(c)];
            for &cell in deme {
                if habitat_class(&self.bio.env[cell as usize]) != f.habitat {
                    continue;
                }
                for p in self.communities[cell as usize].iter_mut() {
                    if Arc::ptr_eq(&p.genome, &old) {
                        p.genome = f.genome.clone();
                        p.phenotype = f.phenotype.clone();
                        p.rates = f.rates;
                    }
                }
            }
            (i, true)
        } else if let Some(j) = self.communities[c].iter().position(|q| q.signature() == f.phenotype.signature) {
            let q = &mut self.communities[c][j];
            q.genome = f.genome;
            q.phenotype = f.phenotype;
            q.rates = f.rates;
            q.lineage = parent_lineage;
            (j, true)
        } else {
            // Nouvelle guilde : la biomasse fondatrice est prise au parent.
            let give = self.config.founder_biomass.min(0.5 * self.communities[c][i].biomass);
            if give < self.config.extinction_biomass {
                (0, false)
            } else {
                self.communities[c][i].biomass -= give;
                let signature = f.phenotype.signature;
                let lineage = self.lineages.found(Some(parent_lineage), years, c as u32, signature, f.genome.clone());
                self.events.push(years, Some(c as u32), EventKind::NewLineage { lineage, parent: parent_lineage, signature });
                self.stats.new_lineages += 1;
                self.communities[c].push(Population { lineage, genome: f.genome, phenotype: f.phenotype, biomass: give, rates: f.rates });
                (self.communities[c].len() - 1, true)
            }
        };
        if !ok {
            return None;
        }
        self.stats.substitutions += 1;
        if selection > 0.0 {
            self.stats.adaptive_substitutions += 1;
        }
        self.stats.fixed_changes_by_cause[f.cause.index()] += 1;
        let lineage = self.communities[c][index].lineage;
        self.journal.record(JournalEntry { years, lineage, cell: c as u32, cause: f.cause, element: f.element });
        for &(cause, element, adaptive) in &f.extra {
            self.stats.substitutions += 1;
            self.stats.adaptive_substitutions += adaptive as u64;
            self.stats.fixed_changes_by_cause[cause.index()] += 1;
            self.journal.record(JournalEntry { years, lineage, cell: c as u32, cause, element });
        }
        Some((c, index, f.cause))
    }

    /// Dème d'une cellule du vivant.
    pub fn deme_of(&self, bio_cell: usize) -> usize {
        self.deme_index[bio_cell] as usize
    }

    /// Chaque cellule océanique regarde les écotypes de ses voisines et
    /// retient, par guilde, le meilleur immigrant qui réussit à s'installer.
    fn migrate(&mut self, dt: f64, step_index: u64) {
        let cfg = &self.config;
        let bio = &self.bio;
        let communities = &self.communities;
        let chemistry = &self.chemistry;
        let physio = &cfg.physiology;
        let disturbances = &self.disturbances;

        let winners: Vec<Vec<MigrationWinner>> = (0..communities.len())
            .into_par_iter()
            .map(|target| {
                let mut best: Vec<MigrationWinner> = Vec::new();
                let env = &bio.env[target];
                if env.water_volume_m3 <= 0.0 {
                    return best;
                }
                let residents = &communities[target];
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                let cond = ctx.conditions(residents);
                for src in bio.grid.neighbours_of(target) {
                    if disturbances.blocked(bio.grid.centers[src], bio.grid.centers[target]) {
                        continue;
                    }
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

    /// Retire les populations les moins aptes des cellules trop peuplées ;
    /// leur biomasse retourne à la couche d'eau (carbone organique et
    /// phosphate).
    ///
    /// Garde-fous (document d'architecture, « Éviction sous le plafond ») :
    /// la dernière population d'une guilde n'est jamais retirée, quitte à
    /// dépasser le plafond, et l'éviction suit la fitness d'invasion (le r
    /// de chaque population dans la communauté résidente, ressources déjà
    /// consommées), de la plus basse à la plus haute, puis la biomasse et
    /// l'ordre d'arrivée (déterministe). La
    /// guilde est ici la voie principale ([`Phenotype::main_pathway`]) : neuf
    /// au plus, alors que les combinaisons de voies se comptent par centaines
    /// et videraient le plafond de son sens.
    fn trim_communities(&mut self) {
        let max = self.config.max_populations_per_cell.max(1);
        let cp = self.config.physiology.carbon_to_phosphorus;
        let envs = &self.bio.env;
        let founder = self.config.founder_biomass;
        let (light, physio, rule) = (self.config.light_biomass_per_m2, &self.config.physiology, self.config.eviction);
        let (removed, saturated, established, growing, occupied) = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter_mut())
            .enumerate()
            .map(|(c, (pops, chem))| {
                let occupied = u64::from(!pops.is_empty());
                if pops.len() <= max {
                    return (0, 0, 0, 0, occupied);
                }
                let guilds: Vec<Option<u32>> = pops.iter().map(|p| p.phenotype.guild_key()).collect();
                let mut guild: BTreeMap<Option<u32>, usize> = BTreeMap::new();
                for g in &guilds {
                    *guild.entry(*g).or_default() += 1;
                }
                // Fitness d'invasion : le taux de croissance de chaque
                // population dans la communauté telle qu'elle est, ressources
                // déjà consommées par les résidents. Un résident à l'équilibre
                // a un r proche de zéro ; un arrivant qui ne peut pas
                // s'installer, un r négatif. On évince d'abord le r le plus
                // bas ; à r égal, la plus petite biomasse, puis l'ordre
                // d'arrivée (tri stable).
                let mut order: Vec<usize> = (0..pops.len()).collect();
                if rule == Eviction::InvasionFitness {
                    let ctx = CellContext { env: &envs[c], light_biomass_per_m2: light };
                    evaluate(pops, &ctx, chem, physio);
                    order.sort_by(|&a, &b| pops[a].rates.r.total_cmp(&pops[b].rates.r).then(pops[a].biomass.total_cmp(&pops[b].biomass)));
                } else {
                    order.sort_by(|&a, &b| pops[a].biomass.total_cmp(&pops[b].biomass));
                }
                let mut keep = vec![true; pops.len()];
                let mut excess = pops.len() - max;
                for &i in &order {
                    if excess == 0 {
                        break;
                    }
                    let n = guild.get_mut(&guilds[i]).expect("guilde comptée");
                    if *n > 1 {
                        *n -= 1;
                        keep[i] = false;
                        excess -= 1;
                    }
                }
                let v = envs[c].water_volume_m3;
                let mut i = 0;
                let mut removed = 0;
                let mut established = 0;
                let mut growing = 0;
                pops.retain(|p| {
                    let k = keep[i];
                    i += 1;
                    if !k {
                        // Plus que la biomasse d'un fondateur : la population
                        // a grandi depuis son arrivée.
                        if p.biomass > founder {
                            established = 1;
                            if p.rates.r > 0.0 {
                                growing = 1;
                            }
                        }
                        chem[WaterPool::Doc as usize] += p.biomass / v;
                        chem[WaterPool::Po4 as usize] += p.biomass / cp / v;
                        removed += 1;
                    }
                    k
                });
                (removed, 1, established, growing, occupied)
            })
            .reduce(|| (0, 0, 0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3, a.4 + b.4));
        self.stats.local_extinctions += removed;
        self.stats.saturated_cell_steps += saturated;
        self.stats.established_eviction_cell_steps += established;
        self.stats.growing_eviction_cell_steps += growing;
        self.stats.occupied_cell_steps += occupied;
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
                    EventKind::Innovation { lineage: p.lineage, pathway: RHODOPSIN_PATHWAY.into(), stage: 1, label: "rhodopsine".into() },
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
                EventKind::Innovation {
                    lineage,
                    pathway: PHOTOSYNTHESIS_PATHWAY.into(),
                    stage,
                    label: PHOTOSYNTHESIS_STAGES[stage as usize].into(),
                },
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
                    EventKind::AcceleratorOff { pathway: PHOTOSYNTHESIS_PATHWAY.into() },
                    Origin::Accelerator,
                    Some(id),
                );
            }
        }
        self.complexity_bookkeeping(&modified);
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
                        EventKind::AcceleratorOn { pathway: PHOTOSYNTHESIS_PATHWAY.into(), stage },
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

        // Des multicellulaires complexes gagnent les terres : le vivant passe
        // à la résolution de la planète.
        if self.config.transitions.refine_on_land
            && self.progress.complexity_years[7].is_some()
            && self.config.bio_level < self.config.level
        {
            self.refine_life_grid();
        }

        if self.history.due(years) {
            self.record_history();
        }
        self.publish();
    }

    /// Passe la grille du vivant à la résolution de la grille physique
    /// (document d'architecture : vivant au niveau 6 quand les
    /// multicellulaires colonisent les terres). Chaque cellule fine reçoit la
    /// chimie de sa cellule grossière (des concentrations) et une part de ses
    /// populations au prorata de son volume d'eau. Les grilles sont
    /// emboîtées : une cellule grossière garde son numéro, si bien que les
    /// événements passés restent bien placés.
    pub fn refine_life_grid(&mut self) {
        let level = self.config.level;
        if self.config.bio_level >= level {
            return;
        }
        let before = [self.total_carbon(), self.total_phosphorus(), self.total_electrons()];
        let mut bio = BioGrid::new(&self.planet.grid, level);
        bio.aggregate(&self.planet.cells);
        let old_of: Vec<usize> = bio.children.iter().map(|kids| self.bio.parent[kids[0] as usize] as usize).collect();
        self.chemistry = old_of.iter().map(|&o| self.chemistry[o]).collect();
        self.communities = old_of
            .iter()
            .enumerate()
            .map(|(c, &o)| {
                let volume = self.bio.env[o].water_volume_m3;
                let share = if volume > 0.0 { bio.env[c].water_volume_m3 / volume } else { 0.0 };
                if share <= 0.0 {
                    return Vec::new();
                }
                self.communities[o]
                    .iter()
                    .map(|p| {
                        let mut q = p.clone();
                        q.biomass *= share;
                        q
                    })
                    .collect()
            })
            .collect();
        self.bio = bio;
        self.config.bio_level = level;
        let deme_level = self.config.deme_level.unwrap_or(level.saturating_sub(1)).min(level);
        let deme_grid = BioGrid::new(&self.bio.grid, deme_level);
        (self.demes, self.deme_index) = (deme_grid.children, deme_grid.parent);
        // Les arrondis du partage sont portés au bilan comme un échange.
        let after = [self.total_carbon(), self.total_phosphorus(), self.total_electrons()];
        for (k, e) in [Element::Carbon, Element::Phosphorus, Element::Electrons].into_iter().enumerate() {
            self.flux.exchange(e, after[k] - before[k]);
        }
    }

    /// Effet de l'accélérateur de la complexité au moment `years` : 1 quand
    /// il n'agit pas ; sinon son multiplicateur, décuplé à chaque
    /// `complexity_escalation_years` passées depuis sa mise en marche, au plus
    /// `complexity_max_boost`. [Simplification] Accélération déclarée : sur
    /// les petits mondes, les hôtes phagotrophes sont trop peu nombreux pour
    /// garder un endosymbiote dans des délais terrestres.
    fn complexity_boost(&self, years: f64) -> f64 {
        let acc = &self.config.evolution.accelerator;
        if !self.progress.complex_accelerator_on {
            return 1.0;
        }
        let Some(oxygenic) = self.progress.stage_years[4] else { return acc.boost };
        let since = self.progress.complexity_since_years.unwrap_or(oxygenic).max(oxygenic);
        let running = (years - since - acc.complexity_patience_years).max(0.0);
        let escalation = 10f64.dpowf(running / acc.complexity_escalation_years.max(1.0));
        (acc.boost * escalation).min(acc.complexity_max_boost.max(acc.boost))
    }

    /// Étapes de la complexité : première apparition sur la planète, et
    /// accélérateur des transitions quand la complexité stagne (seulement
    /// une fois la photosynthèse oxygénique apparue : avant, c'est elle que
    /// l'accélérateur de la photosynthèse surveille).
    fn complexity_bookkeeping(&mut self, modified: &[(usize, usize, GenomeChangeCause)]) {
        let years = self.years;
        let mut reached = [None::<(usize, usize, GenomeChangeCause)>; COMPLEXITY_STAGE_COUNT];
        for &(c, i, cause) in modified {
            let Some(p) = self.communities[c].get(i) else { continue };
            let bits = complexity_bits(&p.phenotype, !self.bio.env[c].is_ocean);
            for (k, slot) in reached.iter_mut().enumerate() {
                if bits & (1 << k) != 0 && self.progress.complexity_years[k].is_none() && slot.is_none() {
                    *slot = Some((c, i, cause));
                }
            }
        }
        // Une colonie existante qui gagne la terre ferme ne passe pas par
        // une fixation : on la cherche parmi les populations des terres.
        if self.progress.complexity_years[7].is_none() && self.progress.complexity_years[6].is_some() && reached[7].is_none() {
            'land: for (c, pops) in self.communities.iter().enumerate() {
                if self.bio.env[c].is_ocean {
                    continue;
                }
                for (i, p) in pops.iter().enumerate() {
                    if complexity_bits(&p.phenotype, true) & (1 << 7) != 0 {
                        reached[7] = Some((c, i, GenomeChangeCause::SpontaneousMutation(evo_genetics::MutationKind::Point)));
                        break 'land;
                    }
                }
            }
        }
        let mut any = None;
        for (k, slot) in reached.iter().enumerate() {
            let Some((c, i, cause)) = *slot else { continue };
            let lineage = self.communities[c][i].lineage;
            let previous = self.progress.complexity_events.iter().rev().find_map(|e| *e);
            let id = self.events.push_with(
                years,
                Some(c as u32),
                EventKind::Innovation {
                    lineage,
                    pathway: COMPLEXITY_PATHWAY.into(),
                    stage: k as u8 + 1,
                    label: COMPLEXITY_STAGES[k].into(),
                },
                origin_of(cause),
                previous,
            );
            self.progress.complexity_years[k] = Some(years);
            self.progress.complexity_events[k] = Some(id);
            any = Some(id);
        }
        if let Some(id) = any {
            self.progress.complexity_since_years = Some(years);
            if self.progress.complex_accelerator_on {
                self.progress.complex_accelerator_on = false;
                self.events.push_with(
                    years,
                    None,
                    EventKind::AcceleratorOff { pathway: COMPLEXITY_PATHWAY.into() },
                    Origin::Accelerator,
                    Some(id),
                );
            }
        }
        let acc = &self.config.evolution.accelerator;
        let Some(oxygenic) = self.progress.stage_years[4] else { return };
        // Le but suivi : un eucaryote multicellulaire à deux types cellulaires.
        if !acc.enabled || self.progress.complex_accelerator_on || self.progress.complexity_years[6].is_some() {
            return;
        }
        let since = self.progress.complexity_since_years.unwrap_or(oxygenic).max(oxygenic);
        if years - since >= acc.complexity_patience_years {
            self.progress.complex_accelerator_on = true;
            let stage = self.progress.complexity_years.iter().rposition(Option::is_some).map_or(0, |k| k as u8 + 1);
            self.events.push_with(
                years,
                None,
                EventKind::AcceleratorOn { pathway: COMPLEXITY_PATHWAY.into(), stage },
                Origin::Accelerator,
                None,
            );
        }
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

    /// Publie l'état du monde à la fin du pas (photographie immuable). Ne
    /// modifie rien de ce qui fait l'histoire.
    fn publish(&mut self) {
        let globals = self.history.last().filter(|s| s.years == self.years).copied().unwrap_or_else(|| self.sample());
        // Résumé par cellule du vivant, recopié sur ses cellules physiques.
        struct BioSummary {
            biomass: f64,
            dominant: u32,
            pigment: Option<[u8; 3]>,
            stage: u8,
            oxygen: f32,
        }
        let bio: Vec<BioSummary> = (0..self.bio.len())
            .map(|b| {
                let pops = &self.communities[b];
                let dominant = pops.iter().max_by(|x, y| x.biomass.total_cmp(&y.biomass));
                let photo = pops.iter().filter(|p| p.phenotype.pigment_nm.is_some()).max_by(|x, y| x.biomass.total_cmp(&y.biomass));
                BioSummary {
                    biomass: pops.iter().map(|p| p.biomass).sum(),
                    dominant: dominant.map_or(0, |p| p.signature()),
                    pigment: photo.and_then(|p| p.phenotype.pigment_nm).map(pigment_colour),
                    stage: pops.iter().map(|p| photosynthesis_stage(&p.phenotype)).max().unwrap_or(0),
                    oxygen: self.chemistry[b][WaterPool::O2 as usize] as f32,
                }
            })
            .collect();
        let cells = (0..self.planet.cells.len())
            .map(|c| {
                let env = &self.planet.cells[c];
                let b = self.bio.parent[c] as usize;
                let s = &bio[b];
                let d = self.planet.display.get(c).copied().unwrap_or_default();
                let water = self.bio.env[b].water_area_m2;
                CellView {
                    elevation_m: env.elevation_m as f32,
                    temperature_k: env.temperature_k as f32,
                    is_ocean: env.is_ocean,
                    ice: env.ice_cover > 0.5,
                    ice_cover: env.ice_cover as f32,
                    light_w_m2: env.light_par_w_m2 as f32,
                    biomass: s.biomass as f32,
                    biomass_per_m2: if water > 0.0 { (s.biomass / water) as f32 } else { 0.0 },
                    dominant_guild: s.dominant,
                    pigment_rgb: s.pigment,
                    photosynthesis_stage: s.stage,
                    oxygen: s.oxygen,
                    plate: self.planet.tectonics.parcel_of(c).plate,
                    bio_cell: b as u32,
                    wind_ms: d.wind_ms,
                    current_ms: d.current_ms,
                    plate_velocity_cm_yr: d.plate_velocity_cm_yr,
                    rain_mm_yr: d.rain_mm_yr,
                    cloud_cover: d.cloud_cover,
                    river_flow_m3s: d.river_flow_m3s,
                    river_downstream: d.river_downstream,
                    lake_fraction: d.lake_fraction,
                }
            })
            .collect();
        let new_events =
            self.events.events[self.published_events.min(self.events.events.len())..].iter().map(EventView::from_event).collect();
        self.published_events = self.events.events.len();
        self.scan_species_origins();
        let species = self.species();
        let p = &self.config.influence;
        self.publication.publish(PublishedState {
            step: self.stats.steps,
            years: self.years,
            step_years: self.config.step_years,
            climate_mode: ClimateMode::for_step(self.next_step_years()),
            globals,
            cells,
            bio_level: self.config.bio_level,
            bio_cells: self.bio.len() as u32,
            species,
            new_events,
            influence: InfluenceView {
                points: self.influence.points as f32,
                max: p.max as f32,
                recharge_per_myr: p.recharge_per_myr as f32,
                sandbox: p.sandbox,
            },
            paused: self.paused,
            focus: self.focus(),
            disturbances: self.disturbances.clone(),
        });
    }

    /// Espèces vivantes (guildes), par biomasse décroissante.
    /// Met à jour la cellule d'origine de chaque signature avec les
    /// lignées fondées depuis le dernier appel.
    fn scan_species_origins(&mut self) {
        for r in &self.lineages.records[self.origin_scanned..] {
            self.species_origin.entry(r.signature).or_insert(r.origin_cell);
        }
        self.origin_scanned = self.lineages.records.len();
    }

    pub fn species(&self) -> Vec<SpeciesView> {
        // Vue, biomasse de la population la plus abondante, génotypes vus
        // (adresses), et cette population.
        type Entry<'a> = (SpeciesView, f64, Vec<usize>, Option<&'a Population>);
        let mut map: BTreeMap<u32, Entry> = BTreeMap::new();
        for (b, pops) in self.communities.iter().enumerate() {
            for p in pops {
                let sig = p.signature();
                let e = map.entry(sig).or_insert_with(|| {
                    (
                        SpeciesView {
                            id: sig,
                            signature: sig,
                            name: guild_label(sig),
                            phototroph: p.phenotype.phototroph,
                            origin_bio_cell: self.species_origin.get(&sig).copied().unwrap_or(b as u32),
                            ..Default::default()
                        },
                        0.0,
                        Vec::new(),
                        None,
                    )
                });
                let v = &mut e.0;
                v.biomass += p.biomass;
                v.cells += 1;
                v.photosynthesis_stage = v.photosynthesis_stage.max(photosynthesis_stage(&p.phenotype));
                let key = Arc::as_ptr(&p.genome) as usize;
                if !e.2.contains(&key) {
                    e.2.push(key);
                }
                // Cellule, pigment et organisation de la population la plus
                // abondante.
                if p.biomass > e.1 {
                    e.1 = p.biomass;
                    v.peak_bio_cell = b as u32;
                    if let Some(nm) = p.phenotype.pigment_nm {
                        v.pigment_rgb = Some(pigment_colour(nm));
                    }
                    v.organisation = crate::history::Organisation::of(&p.phenotype);
                    e.3 = Some(p);
                }
            }
        }
        let mut list: Vec<SpeciesView> = map
            .into_values()
            .map(|(mut v, _, eco, peak)| {
                v.ecotypes = eco.len() as u32;
                v.body_plan = peak.map(|p| Arc::new(evo_life::body_plan(&p.genome, &p.phenotype, &self.config.physiology)));
                v
            })
            .collect();
        list.sort_by(|a, b| b.biomass.total_cmp(&a.biomass).then(a.id.cmp(&b.id)));
        list
    }

    /// Détail des cellules du vivant dans la zone d'intérêt.
    fn focus(&self) -> Focus {
        let Some(zone) = self.interest else { return Focus::default() };
        let n = self.planet.grid.len();
        let center = self.planet.grid.centers[(zone.center_cell as usize).min(n - 1)];
        let radius = zone.radius_km.max(1.0) * 1e3 / self.planet.params.radius_m;
        let mut near: Vec<(f64, usize)> = (0..self.bio.len())
            .filter_map(|b| {
                let p = self.bio.grid.centers[b];
                let d = (p[0] * center[0] + p[1] * center[1] + p[2] * center[2]).clamp(-1.0, 1.0).dacos();
                (d <= radius).then_some((d, b))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        near.truncate(MAX_FOCUS_CELLS);
        let physio = &self.config.physiology;
        let cells = near
            .into_iter()
            .map(|(_, b)| FocusCell {
                bio_cell: b as u32,
                populations: self.communities[b].iter().map(population_view).collect(),
                chemistry: self.chemistry[b].iter().map(|&x| x as f32).collect(),
            })
            .collect();
        let _ = physio;
        Focus { zone: Some(zone), cells }
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
        for x in [
            r.deep_fe2,
            r.deep_mn2,
            r.deep_po4,
            r.carbonate_c,
            r.organic_c,
            r.sediment_p,
            r.iron_reduced,
            r.manganese_reduced,
            self.planet.climate.obliquity_rad,
            self.influence.points,
            self.config.step_years,
        ] {
            h.f(x);
        }
        h.u(u64::from(self.paused));
        for e in &self.planet.cells {
            h.f(e.elevation_m);
            h.f(e.temperature_k);
        }
        h.u(self.events.events.len() as u64);
        h.u(self.lineages.records.len() as u64);
        h.u(self.journal.total());
        h.u(self.disturbances.barriers.len() as u64);
        h.u(self.disturbances.anomalies.len() as u64);
        h.f(self.disturbances.killed_biomass);
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
                    mismatch += p.biomass * (t - self.bio.env[c].temperature_k).abs();
                    weight += p.biomass;
                }
            }
        }
        let ocean: Vec<usize> = self.bio.wet_cells().collect();
        let lakes = ocean.iter().filter(|&&c| !self.bio.env[c].is_ocean).count();
        let lake_biomass: f64 =
            ocean.iter().filter(|&&c| !self.bio.env[c].is_ocean).map(|&c| self.communities[c].iter().map(|p| p.biomass).sum::<f64>()).sum();
        let colonised = ocean.iter().filter(|&&c| !self.communities[c].is_empty()).count();
        let volume: f64 = ocean.iter().map(|&c| self.bio.env[c].water_volume_m3).sum();
        let mut mean_chem = [0.0; WATER_POOL_COUNT];
        for &c in &ocean {
            let v = self.bio.env[c].water_volume_m3;
            for (m, x) in mean_chem.iter_mut().zip(self.chemistry[c]) {
                *m += x * v / volume;
            }
        }
        Summary {
            years: self.years,
            ocean_cells: ocean.len(),
            lake_cells: lakes,
            lake_biomass,
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
            electron_error: self.electron_balance_error(),
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
            planet: self.planet.memory_bytes()
                + self.bio.memory_bytes()
                + self.chemistry.capacity() * std::mem::size_of::<WaterChemistry>(),
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
/// Vue d'une population pour l'affichage.
pub fn population_view(p: &Population) -> PopulationView {
    PopulationView {
        lineage: p.lineage,
        species: p.signature(),
        biomass: p.biomass as f32,
        growth_per_year: p.rates.r as f32,
        birth_per_year: p.rates.birth as f32,
        genes: p.genome.genes.len() as u16,
        pigment_rgb: p.phenotype.pigment_nm.map(pigment_colour),
        pigment_nm: p.phenotype.pigment_nm.map(|x| x as f32),
        phototroph: p.phenotype.phototroph,
        photosynthesis_stage: photosynthesis_stage(&p.phenotype),
    }
}

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

fn debug_rates() -> bool {
    std::env::var_os("EVO_DEBUG_RATES").is_some()
}

/// Flux annuels de la surface vers les réservoirs, et apport des sources
/// hydrothermales de surface qu'ils contiennent, pool par pool, mol·an⁻¹.
pub type SurfaceRates = ([f64; WATER_POOL_COUNT], [f64; WATER_POOL_COUNT]);

/// Flux annuels d'une couche d'eau à prolonger sur le reste du pas, quand
/// elle est supposée à l'équilibre.
///
/// Pendant l'écologie rapide (`t_eco` années), la couche a échangé `exact`
/// moles avec l'extérieur (sorties positives, apports des sources compris),
/// sa chimie est passée de `start` à `end` (volume `volume`) et sa biomasse a
/// varié de `biomass_change` moles de carbone. À l'équilibre, ni la chimie ni
/// la biomasse ne varient : ce qui s'y est accumulé serait sorti. La chimie
/// accumulée sort telle quelle ; la biomasse accumulée sort comme matière
/// organique avec son phosphore (une couche à l'équilibre exporte sa
/// production nette). Chaque terme garde le carbone, le phosphore et les
/// électrons : le carbone et le phosphore exportés sont nuls et le pouvoir
/// oxydant exporté est exactement celui des sources, sans correction.
///
/// Une biomasse qui a fondu peut laisser un export de matière organique
/// négatif, qu'aucune boîte ne fournit. La fonte retranchée l'est alors
/// selon la décomposition qui l'a produite, en sens inverse : respiration
/// (CH₂O + O₂ → CO₂) dans la part oxique de la couche, fermentation
/// méthanogène (2 CH₂O → CH₄ + CO₂) dans la part anoxique. Ces deux
/// réactions gardent aussi les trois bilans.
#[allow(clippy::too_many_arguments)]
pub fn steady_rates(
    exact: &[f64; WATER_POOL_COUNT],
    start: &WaterChemistry,
    end: &WaterChemistry,
    volume: f64,
    biomass_change: f64,
    carbon_to_phosphorus: f64,
    t_eco: f64,
    oxygen_half: f64,
) -> [f64; WATER_POOL_COUNT] {
    let mut m: [f64; WATER_POOL_COUNT] = std::array::from_fn(|i| exact[i] + (end[i] - start[i]) * volume);
    m[WaterPool::Doc as usize] += biomass_change;
    m[WaterPool::Po4 as usize] += biomass_change / carbon_to_phosphorus;
    let deficit = -m[WaterPool::Doc as usize];
    if deficit > 0.0 {
        let o2 = end[WaterPool::O2 as usize].max(0.0);
        let oxic = o2 / (o2 + oxygen_half);
        let (a, b) = (deficit * oxic, deficit * (1.0 - oxic));
        m[WaterPool::Doc as usize] = 0.0;
        m[WaterPool::Dic as usize] -= a + 0.5 * b;
        m[WaterPool::O2 as usize] += a;
        m[WaterPool::Ch4 as usize] -= 0.5 * b;
    }
    // Le carbone et le phosphore exportés sont nuls aux arrondis près ; ces
    // arrondis (différences de grands stocks, prolongées sur tout le pas)
    // vont au carbone inorganique et au phosphate, sans effet sur les
    // électrons.
    let carbon: f64 = WATER_POOLS.iter().map(|&p| m[p as usize] * p.carbon_atoms()).sum();
    m[WaterPool::Dic as usize] -= carbon;
    let phosphorus: f64 = WATER_POOLS.iter().map(|&p| m[p as usize] * p.phosphorus_atoms()).sum();
    m[WaterPool::Po4 as usize] -= phosphorus;
    m.map(|x| x / t_eco)
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
    /// Cellules du vivant qui ont une couche d'eau, dont celles faites
    /// surtout d'eaux douces (lacs et sols humides) et leur biomasse.
    pub ocean_cells: usize,
    pub lake_cells: usize,
    pub lake_biomass: f64,
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
    pub electron_error: f64,
    pub globals: Sample,
    pub stats: WorldStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn small_world(seed: u64) -> World {
        let mut cfg = WorldConfig::new(seed, 3);
        // Vivant à la résolution de la planète : sur une grille aussi petite,
        // la grille plus grossière ne laisserait presque rien à coloniser.
        cfg.bio_level = 3;
        cfg.step_years = 1000.0;
        // Un tour d'évolution par pas : à cette échelle, un tour par 300 ka
        // n'en ferait aucun.
        cfg.evolution.round_years = Some(1000.0);
        let mut w = World::new(cfg);
        w.seed_life();
        w
    }

    #[test]
    fn refining_the_life_grid_keeps_mass_and_life() {
        let mut cfg = WorldConfig::new(11, 4);
        cfg.bio_level = 3;
        cfg.step_years = 1000.0;
        let mut w = World::new(cfg);
        w.seed_life();
        for _ in 0..20 {
            w.step();
        }
        let biomass = w.biomass();
        let coarse = w.bio.len();
        w.refine_life_grid();
        assert_eq!(w.config.bio_level, 4);
        assert!(w.bio.len() > 3 * coarse);
        assert_eq!(w.communities.len(), w.bio.len());
        assert!((w.biomass() - biomass).abs() <= 1e-9 * biomass);
        for _ in 0..10 {
            w.step();
        }
        assert!(w.carbon_balance_error() < 1e-9, "carbone {}", w.carbon_balance_error());
        assert!(w.phosphorus_balance_error() < 1e-9);
        assert!(w.biomass() > 0.0);
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
            assert!(w.electron_balance_error() < 1e-9, "électrons : écart {}", w.electron_balance_error());
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
        // colonisation : après 60 pas, l'écart dépend encore beaucoup de la
        // graine (de 0,3 à 1,9 fois celui de l'ancêtre sur 24 graines) ;
        // après 260, il est sous 0,75 pour toutes celles essayées.
        for _ in 0..240 {
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
            // Le dème partage ses génotypes entre cellules voisines de
            // températures proches : l'adaptation est un peu moins fine que
            // cellule par cellule (0,75 à l'étape 2).
            end.thermal_mismatch_k < 0.85 * ancestral,
            "écart thermique {} K contre {} K pour l'ancêtre",
            end.thermal_mismatch_k,
            ancestral
        );
    }

    #[test]
    fn impacts_barriers_and_climate_pulses_act_and_replay() {
        let mut cfg = WorldConfig::new(5, 3);
        cfg.bio_level = 3;
        cfg.step_years = 1000.0;
        cfg.influence.sandbox = true;
        let mut a = World::new(cfg.clone());
        a.orders.submit(0.0, OrderKind::SeedLife);
        for _ in 0..30 {
            a.step();
        }
        // Cellule la plus peuplée : cible de l'impact.
        let (target, _) = a
            .communities
            .iter()
            .enumerate()
            .map(|(c, p)| (c, p.iter().map(|x| x.biomass).sum::<f64>()))
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .unwrap();
        let cell = a.bio.children[target][0];
        let before = a.biomass();
        let carbon = a.total_carbon();
        let t = a.years;
        a.orders.submit(t, OrderKind::Intervene(Intervention::Impact { cell, diameter_km: 10.0 }));
        a.orders.submit(
            t,
            OrderKind::Intervene(Intervention::Isolate { cell, azimuth_deg: 0.0, length_km: 3000.0, duration_years: 50_000.0, sea: true }),
        );
        a.orders.submit(
            t,
            OrderKind::Intervene(Intervention::ClimatePulse {
                cell,
                radius_km: 2000.0,
                delta_k: 8.0,
                rain_factor: 0.5,
                duration_years: 20_000.0,
            }),
        );
        a.step();
        assert!(a.disturbances.killed_biomass > 0.0);
        // La part tuée dépend de la biomasse prise dans le rayon de destruction
        // (3 % avec les colonies de l'étape 4, 5 % et plus avant).
        assert!(a.disturbances.killed_biomass > 0.01 * before, "l'impact tue : {} sur {before}", a.disturbances.killed_biomass);
        // Le carbone tué reste dans le système ; le CO₂ libéré est inscrit.
        assert!(a.carbon_balance_error() < 1e-9 && a.phosphorus_balance_error() < 1e-9, "bilan");
        assert!(a.total_carbon() > carbon);
        assert_eq!(a.disturbances.barriers.len(), 1);
        // L'hiver d'impact est fini, la poussée climatique dure encore.
        assert_eq!(a.disturbances.anomalies.len(), 1);
        for _ in 0..60 {
            a.step();
        }
        assert!(a.disturbances.is_empty(), "tout expire");
        let mut b = World::replay(cfg, &a.orders.log());
        for _ in 0..91 {
            b.step();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn replay_from_seed_and_orders_gives_the_same_state() {
        let mut cfg = WorldConfig::new(11, 3);
        cfg.step_years = 50_000.0;
        let mut a = World::new(cfg.clone());
        a.orders.submit(0.0, OrderKind::SeedLife);
        a.orders.submit(150_000.0, OrderKind::SetStepYears(20_000.0));
        a.orders.submit(200_000.0, OrderKind::Intervene(Intervention::Fertilize { cell: 0, radius_km: 3000.0, moles_p: 1e14 }));
        a.orders.submit(240_000.0, OrderKind::Intervene(Intervention::Eruption { cell: 5, gas: Gas::Co2, moles: 1e16 }));
        a.orders.submit(260_000.0, OrderKind::Pause);
        a.orders.submit(260_000.0, OrderKind::Resume);
        a.orders.submit(300_000.0, OrderKind::MarkLineage { lineage: 0 });
        for _ in 0..20 {
            a.step();
        }
        assert_eq!(a.orders.applied.len(), 7);
        assert!(a.carbon_balance_error() < 1e-9 && a.phosphorus_balance_error() < 1e-9);
        // Un ordre soumis en cours de partie est rejoué à la même date.
        let late = a.orders.submit(a.years, OrderKind::Intervene(Intervention::Eruption { cell: 9, gas: Gas::Ch4, moles: 1e13 }));
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
    fn the_population_cap_never_evicts_the_last_of_a_guild() {
        let mut cfg = WorldConfig::new(5, 3);
        cfg.step_years = 100_000.0;
        cfg.max_populations_per_cell = 2;
        // Innovations fréquentes : il faut plusieurs guildes par cellule.
        cfg.evolution.innovation_probability = 1e-9;
        let mut w = World::new(cfg);
        w.seed_life();
        for _ in 0..60 {
            w.step();
            for pops in &w.communities {
                let guilds: BTreeSet<_> = pops.iter().map(|p| p.phenotype.guild_key()).collect();
                // Au-dessus du plafond, il ne reste qu'une population par guilde.
                assert!(pops.len() <= 2 || guilds.len() == pops.len(), "{} populations pour {} guildes", pops.len(), guilds.len());
            }
        }
        assert!(w.stats.saturated_cell_steps > 0, "le plafond n'a jamais servi");
        assert!(w.stats.saturated_cell_steps <= w.stats.occupied_cell_steps);
        assert!(w.stats.innovations_evaluated <= w.stats.innovations_drawn);
    }

    #[test]
    fn a_long_step_runs_one_evolution_round_per_slice() {
        let run = |step: f64, round: Option<f64>| {
            let mut cfg = WorldConfig::new(8, 3);
            cfg.step_years = step;
            cfg.evolution.round_years = round;
            cfg.adaptive_step = None;
            let mut w = World::new(cfg);
            w.seed_life();
            for _ in 0..10 {
                w.step();
            }
            (w.stats.substitutions, w.state_hash())
        };
        // Un tour d'au plus le pas : rien ne change.
        assert_eq!(run(100_000.0, None).1, run(100_000.0, Some(100_000.0)).1);
        // Deux tours par pas : bien plus de substitutions qu'avec un seul.
        assert!(run(200_000.0, Some(100_000.0)).0 > run(200_000.0, None).0);
    }

    #[test]
    fn calm_periods_lengthen_the_step_and_replay_identically() {
        let run = || {
            let mut w = World::new(WorldConfig::new(8, 3));
            w.seed_life();
            let mut spans = Vec::new();
            for _ in 0..40 {
                let before = w.years;
                let first = w.events.events.len();
                w.step();
                let notable = w.events.events[first..].iter().any(|e| e.interest >= 0.5);
                spans.push((w.years - before, notable));
            }
            (spans, w.state_hash())
        };
        let (spans, hash) = run();
        assert_eq!(run().1, hash);
        assert!(spans.iter().all(|&(dt, _)| [1e5, 2e5, 3e5].iter().any(|x| (dt - x).abs() < 1e-6)));
        assert!(spans.iter().any(|&(dt, _)| dt > 2.5e5), "aucun pas allongé");
        // Après un événement notable, le pas suivant revient au pas demandé.
        for w in spans.windows(2) {
            if w[0].1 {
                assert!((w[1].0 - 1e5).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn the_camera_never_changes_history() {
        // Deux parcours de caméra différents, même histoire au bit près.
        let mut a = small_world(4);
        let mut b = small_world(4);
        for i in 0..12u32 {
            a.set_interest(Some(InterestZone { center_cell: i * 37 % 642, radius_km: 500.0 + 300.0 * i as f64, zoom_band: (i % 4) as u8 }));
            if i % 3 == 0 {
                b.set_interest(None);
            } else {
                b.set_interest(Some(InterestZone { center_cell: 600 - i * 11, radius_km: 8000.0, zoom_band: 1 }));
            }
            a.step();
            b.step();
            assert_eq!(a.state_hash(), b.state_hash(), "pas {i}");
        }
        assert_eq!(a.events.events, b.events.events);
        assert!(!a.publication.current.as_ref().unwrap().focus.cells.is_empty());
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
    fn steady_rates_keep_carbon_phosphorus_and_electrons() {
        let ox = |m: &[f64; WATER_POOL_COUNT], f: fn(WaterPool) -> f64| WATER_POOLS.iter().map(|&p| m[p as usize] * f(p)).sum::<f64>();
        let cp = 106.0;
        // Couche qui reçoit 3 mol d'H₂ de ses sources, dont la biomasse
        // croît (cas 1) ou fond (cas 2, export organique négatif).
        for (biomass_change, doc_out) in [(5.0, 1.0), (-40.0, 2.0)] {
            let mut exact = [0.0; WATER_POOL_COUNT];
            let (mut start, mut end) = ([0.0; WATER_POOL_COUNT], [0.0; WATER_POOL_COUNT]);
            start[WaterPool::O2 as usize] = 1e-4;
            end[WaterPool::O2 as usize] = 1e-4;
            // Bilan de la couche : sources = sorties + accumulation + biomasse.
            exact[WaterPool::Doc as usize] = doc_out;
            exact[WaterPool::Po4 as usize] = doc_out / cp;
            end[WaterPool::H2 as usize] = 0.5;
            // Carbone, phosphore et électrons de la biomasse pris à l'eau.
            exact[WaterPool::Dic as usize] = -(biomass_change + doc_out);
            exact[WaterPool::Po4 as usize] -= biomass_change / cp + doc_out / cp;
            exact[WaterPool::H2 as usize] = 3.0 - 0.5 - 2.0 * (biomass_change + doc_out);
            let r = steady_rates(&exact, &start, &end, 1.0, biomass_change, cp, 2.0, 1e-3);
            assert!(ox(&r, WaterPool::carbon_atoms).abs() < 1e-12, "carbone {r:?}");
            assert!(ox(&r, WaterPool::phosphorus_atoms).abs() < 1e-12, "phosphore {r:?}");
            assert!((ox(&r, WaterPool::oxidant_equivalents) - 3.0 * -0.5 / 2.0).abs() < 1e-12, "électrons {r:?}");
            assert!(r[WaterPool::Doc as usize] >= 0.0);
        }
    }
}
