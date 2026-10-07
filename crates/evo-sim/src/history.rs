//! Ce que le moteur montre au reste du jeu : historique échantillonné des
//! grandeurs globales et état publié à chaque pas (document « Interface du
//! moteur »).
//!
//! L'état publié est une photographie datée et immuable du monde, prise à la
//! fin de chaque pas. Le moteur garde la précédente : un affichage peut
//! interpoler entre les deux sans jamais lire l'état pendant qu'il change.
//!
//! [Simplification] Le double tampon partagé entre fils, les historiques par
//! région et par espèce et les points de sauvegarde arrivent à l'étape 3.

use std::fmt::Write as _;
use std::sync::Arc;

/// Un échantillon des grandeurs globales.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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

/// Ce qu'un affichage peut lire d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellView {
    pub elevation_m: f32,
    pub temperature_k: f32,
    pub is_ocean: bool,
    pub ice: bool,
    /// Biomasse vivante, mol de carbone.
    pub biomass: f32,
    /// Guilde dominante (signature métabolique), 0 sans vie.
    pub dominant_guild: u32,
    /// Couleur du pigment de la population phototrophe dominante, s'il y en a.
    pub pigment_rgb: Option<[u8; 3]>,
    /// O₂ dissous, mol·m⁻³.
    pub oxygen: f32,
    /// Plaque qui porte la cellule.
    pub plate: u16,
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
    pub cells: Vec<CellView>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
