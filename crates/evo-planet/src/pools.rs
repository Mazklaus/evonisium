//! Liste unique des pools chimiques (section 8 du document Planète).
//!
//! C'est la référence commune du projet : la planète l'écrit, les autres
//! chantiers en lisent des sous-ensembles. L'étape 1 ne simule qu'une partie
//! de la couche d'eau de surface ([`WATER_POOLS`]).

/// Les 28 pools de la liste commune.
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

/// Pools de la couche d'eau simulés à l'étape 1, en mol·m⁻³.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterPool {
    Dic = 0,
    Doc = 1,
    H2 = 2,
    Ch4 = 3,
    O2 = 4,
    Sulfate = 5,
    H2s = 6,
}

pub const WATER_POOL_COUNT: usize = 7;

pub const WATER_POOLS: [WaterPool; WATER_POOL_COUNT] =
    [WaterPool::Dic, WaterPool::Doc, WaterPool::H2, WaterPool::Ch4, WaterPool::O2, WaterPool::Sulfate, WaterPool::H2s];

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
        }
    }

    /// Atomes de carbone par molécule, pour le registre de flux.
    pub fn carbon_atoms(self) -> f64 {
        match self {
            WaterPool::Dic | WaterPool::Doc | WaterPool::Ch4 => 1.0,
            _ => 0.0,
        }
    }

    /// Gaz échangé avec l'atmosphère (vitesse d'échange rapide).
    pub fn is_gas(self) -> bool {
        matches!(self, WaterPool::H2 | WaterPool::Ch4 | WaterPool::O2 | WaterPool::H2s)
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
        }
    }
}
