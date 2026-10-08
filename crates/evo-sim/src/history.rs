//! Ce que le moteur montre au reste du jeu : historique échantillonné des
//! grandeurs globales et état publié à chaque pas (document « Interface du
//! moteur »).
//!
//! L'état publié est une photographie datée et immuable du monde, prise à la
//! fin de chaque pas. Le moteur garde la précédente : un affichage peut
//! interpoler entre les deux sans jamais lire l'état pendant qu'il change.
//!
//! Depuis l'étape 3, l'état publié porte aussi les sorties d'affichage de la
//! planète, les espèces vivantes, les événements du pas, la réserve
//! d'influence et le détail de la zone d'intérêt. Le partage entre fils est
//! fait par le crate `evo-engine`.

use crate::influence::InfluenceView;
use crate::observation::Focus;
use evo_core::events::Origin;
use std::fmt::Write as _;
use std::sync::Arc;

/// Un échantillon des grandeurs globales.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sample {
    pub years: f64,
    /// Fractions molaires de l'atmosphère.
    pub o2_mixing: f64,
    pub co2_pa: f64,
    pub ch4_ppb: f64,
    pub pressure_pa: f64,
    pub mean_temperature_k: f64,
    pub ice_fraction: f64,
    pub ocean_fraction: f64,
    /// Biomasse vivante, mol de carbone.
    pub biomass: f64,
    pub living_lineages: usize,
    /// Guildes métaboliques présentes (équivalent provisoire du nombre
    /// d'espèces chez les microbes).
    pub guilds: usize,
    /// Étape la plus avancée du chemin vers la photosynthèse présente.
    pub photosynthesis_stage: u8,
    /// Flux d'O₂, mol·an⁻¹ : production brute dans les cellules, libération
    /// nette vers l'atmosphère, puits globaux.
    pub o2_production: f64,
    pub o2_release: f64,
    pub o2_sinks: f64,
    /// Carbone organique enfoui, mol·an⁻¹.
    pub organic_burial: f64,
    pub accelerator_on: bool,
}

/// Historique échantillonné à intervalle fixe de temps de jeu.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct History {
    pub every_years: f64,
    pub samples: Vec<Sample>,
    next_years: f64,
}

impl History {
    pub fn new(every_years: f64) -> Self {
        Self { every_years, samples: Vec::new(), next_years: 0.0 }
    }

    /// Vrai si un échantillon est dû à la date `years`.
    pub fn due(&self, years: f64) -> bool {
        years >= self.next_years
    }

    pub fn push(&mut self, s: Sample) {
        while self.next_years <= s.years {
            self.next_years += self.every_years;
        }
        self.samples.push(s);
    }

    pub fn last(&self) -> Option<&Sample> {
        self.samples.last()
    }

    pub fn to_tsv(&self) -> String {
        let mut out = String::from(
            "annees\tO2_fraction\tCO2_Pa\tCH4_ppb\tpression_Pa\ttemperature_K\tglace\tocean\tbiomasse_molC\tlignees\tguildes\tetape_photosynthese\tO2_production\tO2_liberation\tO2_puits\tenfouissement_C\taccelerateur\n",
        );
        for s in &self.samples {
            let _ = writeln!(
                out,
                "{}\t{:.4e}\t{:.4e}\t{:.4e}\t{:.4e}\t{:.2}\t{:.3}\t{:.3}\t{:.4e}\t{}\t{}\t{}\t{:.3e}\t{:.3e}\t{:.3e}\t{:.3e}\t{}",
                s.years,
                s.o2_mixing,
                s.co2_pa,
                s.ch4_ppb,
                s.pressure_pa,
                s.mean_temperature_k,
                s.ice_fraction,
                s.ocean_fraction,
                s.biomass,
                s.living_lineages,
                s.guilds,
                s.photosynthesis_stage,
                s.o2_production,
                s.o2_release,
                s.o2_sinks,
                s.organic_burial,
                u8::from(s.accelerator_on)
            );
        }
        out
    }
}

/// Ce qu'un affichage peut lire d'une cellule physique. Les champs du vivant
/// sont ceux de sa cellule du vivant (`bio_cell`), répétés sur ses cellules
/// physiques.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellView {
    /// Altitude (positive) ou profondeur (négative) sous le niveau de la mer, m.
    pub elevation_m: f32,
    pub temperature_k: f32,
    pub is_ocean: bool,
    pub ice: bool,
    /// Couverture de glace, de 0 à 1.
    pub ice_cover: f32,
    /// Lumière utilisable par des pigments, W·m⁻².
    pub light_w_m2: f32,
    /// Biomasse vivante de la cellule du vivant, mol de carbone.
    pub biomass: f32,
    /// Biomasse par m² d'eau, mol de carbone (pour les calques).
    pub biomass_per_m2: f32,
    /// Guilde dominante (signature métabolique), 0 sans vie. C'est aussi
    /// l'identifiant d'espèce du vivant microbien (voir [`SpeciesView`]).
    pub dominant_guild: u32,
    /// Couleur du pigment de la population phototrophe dominante, s'il y en a.
    pub pigment_rgb: Option<[u8; 3]>,
    /// Étape la plus avancée du chemin vers la photosynthèse dans la cellule.
    pub photosynthesis_stage: u8,
    /// O₂ dissous, mol·m⁻³.
    pub oxygen: f32,
    /// Plaque qui porte la cellule.
    pub plate: u16,
    /// Cellule du vivant qui contient la cellule physique.
    pub bio_cell: u32,
    // — Sorties d'affichage de la planète (lecture seule, sans effet sur
    //   l'histoire) —
    /// Vent de surface et courant marin, m·s⁻¹ (est, nord).
    pub wind_ms: [f32; 2],
    pub current_ms: [f32; 2],
    /// Vitesse de la plaque, cm·an⁻¹ (est, nord).
    pub plate_velocity_cm_yr: [f32; 2],
    pub rain_mm_yr: f32,
    pub cloud_cover: f32,
    /// Débit sortant, m³·s⁻¹, et cellule aval (`u32::MAX` : mer ou lac fermé).
    pub river_flow_m3s: f32,
    pub river_downstream: u32,
    /// Part de la cellule couverte d'eaux douces.
    pub lake_fraction: f32,
}

/// Une espèce vivante. Pendant l'ère microbienne, une espèce est une guilde
/// métabolique, identifiée par sa signature (l'ensemble des réactions
/// qu'elle catalyse) ; ses écotypes sont les génotypes de ses populations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeciesView {
    pub id: u32,
    /// Nom généré à partir du métabolisme (« méthanogènes », …).
    pub name: String,
    /// Biomasse totale, mol de carbone.
    pub biomass: f64,
    /// Cellules du vivant occupées.
    pub cells: u32,
    /// Écotypes (génotypes distincts).
    pub ecotypes: u32,
    pub phototroph: bool,
    pub photosynthesis_stage: u8,
    /// Couleur du pigment le plus abondant de l'espèce, s'il y en a.
    pub pigment_rgb: Option<[u8; 3]>,
    /// Cellule du vivant où l'espèce est la plus abondante.
    pub peak_bio_cell: u32,
}

/// Un événement, tel que la chronique et les alertes le montrent.
#[derive(Clone, Debug, PartialEq)]
pub struct EventView {
    pub id: u64,
    pub years: f64,
    /// Cellule du vivant concernée, s'il y en a une.
    pub cell: Option<u32>,
    pub type_name: &'static str,
    /// Libellé en français.
    pub label: String,
    pub origin: Origin,
    pub cause: Option<u64>,
    pub interest: f32,
}

impl EventView {
    pub fn from_event(e: &evo_core::events::Event) -> Self {
        Self {
            id: e.id,
            years: e.years,
            cell: e.cell,
            type_name: e.kind.type_name(),
            label: e.kind.describe(),
            origin: e.origin,
            cause: e.cause,
            interest: e.interest as f32,
        }
    }
}

/// Photographie datée et immuable du monde à la fin d'un pas.
#[derive(Clone, Debug, PartialEq)]
pub struct PublishedState {
    pub step: u64,
    /// Date de jeu à la fin du pas, années.
    pub years: f64,
    pub step_years: f64,
    /// Mode du climat pour ce pas.
    pub climate_mode: ClimateMode,
    pub globals: Sample,
    /// Une vue par cellule physique, dans l'ordre de la grille.
    pub cells: Vec<CellView>,
    /// Niveau de la grille du vivant et nombre de ses cellules.
    pub bio_level: u32,
    pub bio_cells: u32,
    /// Espèces vivantes, par biomasse décroissante.
    pub species: Vec<SpeciesView>,
    /// Événements inscrits pendant ce pas.
    pub new_events: Vec<EventView>,
    pub influence: InfluenceView,
    pub paused: bool,
    /// Détail de la zone d'intérêt de la caméra.
    pub focus: Focus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClimateMode {
    /// Pas d'au moins 10 000 ans : climat d'équilibre, cycles orbitaux lissés.
    #[default]
    Equilibrium,
    /// Pas plus courts : climat progressif.
    Progressive,
}

impl ClimateMode {
    /// Seuil du document Planète (section 2) : 10 000 ans de jeu par pas.
    pub const THRESHOLD_YEARS: f64 = 10_000.0;

    pub fn for_step(step_years: f64) -> Self {
        if step_years >= Self::THRESHOLD_YEARS {
            ClimateMode::Equilibrium
        } else {
            ClimateMode::Progressive
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ClimateMode::Equilibrium => "équilibre",
            ClimateMode::Progressive => "progressif",
        }
    }
}

/// Les deux derniers états publiés.
#[derive(Clone, Debug, Default)]
pub struct Publication {
    pub current: Option<Arc<PublishedState>>,
    pub previous: Option<Arc<PublishedState>>,
}

impl Publication {
    pub fn publish(&mut self, state: PublishedState) {
        self.previous = self.current.take();
        self.current = Some(Arc::new(state));
    }
}
