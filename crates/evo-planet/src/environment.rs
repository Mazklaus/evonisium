//! Vecteur d'environnement par cellule et chimie de la couche d'eau.
//!
//! Contrat Planète → Organismes : unités SI, une valeur par cellule et par
//! couche. L'étape 1 ne simule qu'une couche, l'eau de surface des cellules
//! océaniques ; les sols, la colonne profonde et le fond arrivent à l'étape 2.

use crate::grid::GeodesicGrid;
use crate::params::PlanetParams;
use crate::pools::{WaterPool, WATER_POOLS, WATER_POOL_COUNT};
use evo_core::flux::{Element, FluxRegistry};

/// Conditions physiques d'une cellule, fixes à l'étape 1.
#[derive(Clone, Debug, PartialEq)]
pub struct CellEnvironment {
    pub latitude_rad: f64,
    /// Altitude (positive) ou profondeur (négative) du fond, m.
    pub elevation_m: f64,
    pub is_ocean: bool,
    /// Aire de la cellule, m².
    pub area_m2: f64,
    /// Volume de la couche d'eau simulée, m³ (nul sur les terres).
    pub water_volume_m3: f64,
    /// Température moyenne annuelle, K.
    pub temperature_k: f64,
    /// Amplitude saisonnière, K.
    pub seasonal_amplitude_k: f64,
    /// Lumière utile à la photosynthèse, moyennée sur la couche, W·m⁻².
    pub light_par_w_m2: f64,
    /// Ultraviolets reçus en surface, W·m⁻².
    pub uv_w_m2: f64,
    pub ph: f64,
    /// Salinité, g·kg⁻¹.
    pub salinity: f64,
    /// Pression au milieu de la couche, Pa.
    pub pressure_pa: f64,
    /// Apports hydrothermaux, mol·an⁻¹.
    pub vent_h2_supply: f64,
    pub vent_h2s_supply: f64,
}

/// Concentrations de la couche d'eau, mol·m⁻³, indexées par [`WaterPool`].
pub type WaterChemistry = [f64; WATER_POOL_COUNT];

/// Planète de l'étape 1 : grille fixe et conditions fixes. La chimie de
/// l'eau, qui varie, est tenue à part (voir [`Planet::initial_chemistry`]) pour
/// que chaque cellule puisse être mise à jour en parallèle.
#[derive(Clone, Debug)]
pub struct Planet {
    pub params: PlanetParams,
    pub grid: GeodesicGrid,
    pub cells: Vec<CellEnvironment>,
}

impl Planet {
    /// Concentration d'équilibre avec l'atmosphère et l'océan profond, et
    /// vitesse de rappel vers cet équilibre, an⁻¹.
    pub fn exchange_target(&self, pool: WaterPool) -> (f64, f64) {
        match pool {
            WaterPool::Dic => (self.params.dic_equilibrium, 1.0),
            WaterPool::Sulfate => (self.params.sulfate_equilibrium, 0.2),
            // Sédimentation et export de la matière organique vers le fond.
            WaterPool::Doc => (0.0, 0.5),
            WaterPool::H2 => (self.params.h2_equilibrium, 10.0),
            // Gaz : l'atmosphère de l'étape 1 n'en contient pas (pas de stock
            // atmosphérique avant l'étape 2), ils s'échappent.
            _ => (0.0, 10.0),
        }
    }

    /// Fait évoluer la chimie d'une cellule pendant `dt` années sous l'effet
    /// des échanges avec l'extérieur de la couche (atmosphère, fond, sources
    /// hydrothermales), et inscrit le carbone échangé au registre de flux.
    pub fn exchange(&self, cell: usize, chem: &mut WaterChemistry, dt: f64, flux: &mut FluxRegistry) {
        let env = &self.cells[cell];
        if !env.is_ocean {
            return;
        }
        let v = env.water_volume_m3;
        for pool in WATER_POOLS {
            let i = pool as usize;
            let (target, rate) = self.exchange_target(pool);
            let supply = match pool {
                WaterPool::H2 => env.vent_h2_supply,
                WaterPool::H2s => env.vent_h2s_supply,
                _ => 0.0,
            } / v;
            // Solution exacte de dc/dt = rate·(target − c) + supply.
            let eq = target + supply / rate;
            let before = chem[i];
            chem[i] = eq + (before - eq) * (-rate * dt).exp();
            let c = pool.carbon_atoms();
            if c > 0.0 {
                flux.exchange(Element::Carbon, (chem[i] - before) * v * c);
            }
        }
    }

    /// Chimie sans vie : chaque pool à l'équilibre de ses échanges.
    pub fn initial_chemistry(&self) -> Vec<WaterChemistry> {
        self.cells
            .iter()
            .map(|env| {
                let mut chem: WaterChemistry = [0.0; WATER_POOL_COUNT];
                if env.is_ocean {
                    for pool in WATER_POOLS {
                        let (target, rate) = self.exchange_target(pool);
                        let supply = match pool {
                            WaterPool::H2 => env.vent_h2_supply,
                            WaterPool::H2s => env.vent_h2s_supply,
                            _ => 0.0,
                        } / env.water_volume_m3;
                        chem[pool as usize] = target + supply / rate;
                    }
                }
                chem
            })
            .collect()
    }

    /// Carbone total de la couche d'eau, mol.
    pub fn water_carbon(&self, chemistry: &[WaterChemistry]) -> f64 {
        self.cells
            .iter()
            .zip(chemistry)
            .filter(|(e, _)| e.is_ocean)
            .map(|(e, c)| WATER_POOLS.iter().map(|&p| c[p as usize] * p.carbon_atoms()).sum::<f64>() * e.water_volume_m3)
            .sum()
    }

    pub fn ocean_cells(&self) -> impl Iterator<Item = usize> + '_ {
        self.cells.iter().enumerate().filter(|(_, e)| e.is_ocean).map(|(i, _)| i)
    }

    pub fn memory_bytes(&self) -> usize {
        self.grid.memory_bytes() + self.cells.capacity() * std::mem::size_of::<CellEnvironment>()
    }
}
