//! Liste unique des pools chimiques (section 8 du document Planète).
//!
//! C'est la référence commune du projet : la planète l'écrit, les autres
//! chantiers en lisent des sous-ensembles. L'eau de surface des cellules
//! océaniques en simule une partie ([`WATER_POOLS`]) ; l'atmosphère, l'océan
//! profond et les sédiments sont des réservoirs globaux
//! ([`crate::geochem::GlobalReservoirs`]).

/// Les 30 pools de la liste commune.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pool {
    N2,
    O2,
    Co2,
    Ch4,
    H2,
    WaterVapour,
    H2s,
    SulfurAerosols,
    N2o,
    O3,
    Dust,
    DissolvedInorganicCarbon,
    DissolvedOrganicCarbon,
    ParticulateOrganicMatter,
    Nitrate,
    Ammonium,
    Phosphate,
    FerrousIron,
    FerricIron,
    ManganeseDissolved,
    ManganeseOxides,
    Sulfate,
    DissolvedSilica,
    Calcium,
    TraceMetals,
    Carbonates,
    BuriedOrganicCarbon,
    PyriteGypsum,
    MethaneHydrates,
    MineralDeposits,
}

/// Pools de la couche d'eau de surface, en mol·m⁻³.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterPool {
    Dic = 0,
    Doc = 1,
    H2 = 2,
    Ch4 = 3,
    O2 = 4,
    Sulfate = 5,
    H2s = 6,
    /// Fer ferreux dissous (Fe²⁺).
    Fe2 = 7,
    /// Manganèse dissous (Mn²⁺).
    Mn2 = 8,
    /// Oxydes de fer en suspension, qui sédimentent (fer rubané).
    FeOx = 9,
    /// Oxydes de manganèse en suspension, qui sédimentent.
    MnOx = 10,
    /// Phosphate dissous.
    Po4 = 11,
}

pub const WATER_POOL_COUNT: usize = 12;

pub const WATER_POOLS: [WaterPool; WATER_POOL_COUNT] = [
    WaterPool::Dic,
    WaterPool::Doc,
    WaterPool::H2,
    WaterPool::Ch4,
    WaterPool::O2,
    WaterPool::Sulfate,
    WaterPool::H2s,
    WaterPool::Fe2,
    WaterPool::Mn2,
    WaterPool::FeOx,
    WaterPool::MnOx,
    WaterPool::Po4,
];

impl WaterPool {
    /// Pool correspondant dans la liste commune.
    pub fn common(self) -> Pool {
        match self {
            WaterPool::Dic => Pool::DissolvedInorganicCarbon,
            WaterPool::Doc => Pool::DissolvedOrganicCarbon,
            WaterPool::H2 => Pool::H2,
            WaterPool::Ch4 => Pool::Ch4,
            WaterPool::O2 => Pool::O2,
            WaterPool::Sulfate => Pool::Sulfate,
            WaterPool::H2s => Pool::H2s,
            WaterPool::Fe2 => Pool::FerrousIron,
            WaterPool::Mn2 => Pool::ManganeseDissolved,
            WaterPool::FeOx => Pool::FerricIron,
            WaterPool::MnOx => Pool::ManganeseOxides,
            WaterPool::Po4 => Pool::Phosphate,
        }
    }

    /// Atomes de carbone par unité, pour le registre de flux.
    pub fn carbon_atoms(self) -> f64 {
        match self {
            WaterPool::Dic | WaterPool::Doc | WaterPool::Ch4 => 1.0,
            _ => 0.0,
        }
    }

    /// Pouvoir oxydant par unité, en équivalents d'O₂ (référence : CO₂,
    /// sulfate, oxydes de fer et de manganèse, eau). Un matériau réduit a une
    /// valeur négative : CH₂O consomme une mole d'O₂, CH₄ deux, H₂S deux,
    /// H₂ une demie, Fe²⁺ un quart, Mn²⁺ une demie. La photosynthèse
    /// (CO₂ + H₂O → CH₂O + O₂) et toute réaction biologique conservent la
    /// somme.
    pub fn oxidant_equivalents(self) -> f64 {
        match self {
            WaterPool::O2 => 1.0,
            WaterPool::Doc => -1.0,
            WaterPool::Ch4 => -2.0,
            WaterPool::H2 => -0.5,
            WaterPool::H2s => -2.0,
            WaterPool::Fe2 => -0.25,
            WaterPool::Mn2 => -0.5,
            WaterPool::Dic | WaterPool::Sulfate | WaterPool::FeOx | WaterPool::MnOx | WaterPool::Po4 => 0.0,
        }
    }

    /// Atomes de phosphore par unité.
    pub fn phosphorus_atoms(self) -> f64 {
        match self {
            WaterPool::Po4 => 1.0,
            _ => 0.0,
        }
    }

    /// Gaz échangé avec l'atmosphère, et sa constante de Henry,
    /// mol·m⁻³·Pa⁻¹ (constantes physiques à 25 °C, pas des valeurs terrestres).
    pub fn henry(self) -> Option<f64> {
        match self {
            WaterPool::O2 => Some(1.3e-5),
            WaterPool::Ch4 => Some(1.4e-5),
            WaterPool::H2 => Some(7.8e-6),
            WaterPool::H2s => Some(1.0e-3),
            _ => None,
        }
    }

    pub fn is_gas(self) -> bool {
        self.henry().is_some()
    }

    pub fn label(self) -> &'static str {
        match self {
            WaterPool::Dic => "carbone inorganique dissous",
            WaterPool::Doc => "carbone organique dissous",
            WaterPool::H2 => "H2",
            WaterPool::Ch4 => "CH4",
            WaterPool::O2 => "O2",
            WaterPool::Sulfate => "sulfate",
            WaterPool::H2s => "H2S",
            WaterPool::Fe2 => "Fe2+",
            WaterPool::Mn2 => "Mn2+",
            WaterPool::FeOx => "oxydes de fer",
            WaterPool::MnOx => "oxydes de manganèse",
            WaterPool::Po4 => "phosphate",
        }
    }
}
