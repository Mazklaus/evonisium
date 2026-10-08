//! La planète vivante : grille, tectonique, climat, réservoirs globaux, et
//! vecteur d'environnement par cellule.
//!
//! Contrat Planète → Organismes : unités SI, une valeur par cellule. La vie
//! occupe la couche d'eau de surface des cellules océaniques ; cette couche
//! échange avec l'atmosphère, l'océan profond et les sédiments (réservoirs
//! globaux), et reçoit les sources hydrothermales des dorsales.

use crate::climate::{ClimateState, Greenhouse};
use crate::geochem::{BoxContext, Gas, GlobalReservoirs};
use crate::grid::GeodesicGrid;
use crate::params::PlanetParams;
use crate::pools::{WaterPool, WATER_POOLS, WATER_POOL_COUNT};
use crate::tectonics::{sea_level, Tectonics};

/// Conditions physiques d'une cellule, recalculées à chaque pas.
#[derive(Clone, Debug, PartialEq)]
pub struct CellEnvironment {
    pub latitude_rad: f64,
    /// Altitude (positive) ou profondeur (négative) par rapport au niveau de la mer, m.
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
    /// Lumière utilisable par des pigments (350 à 1100 nm), moyennée sur la
    /// couche d'eau, W·m⁻².
    pub light_par_w_m2: f64,
    /// Ultraviolets nocifs reçus en surface, W·m⁻².
    pub uv_w_m2: f64,
    pub ph: f64,
    /// Salinité, g·kg⁻¹.
    pub salinity: f64,
    /// Pression au milieu de la couche, Pa.
    pub pressure_pa: f64,
    /// Apports hydrothermaux dans la couche de surface, mol·an⁻¹.
    pub vent_h2_supply: f64,
    pub vent_h2s_supply: f64,
    pub vent_fe_supply: f64,
    pub vent_mn_supply: f64,
    /// Couverture de glace, de 0 à 1.
    pub ice_cover: f64,
}

/// Concentrations de la couche d'eau, mol·m⁻³, indexées par [`WaterPool`].
pub type WaterChemistry = [f64; WATER_POOL_COUNT];

/// Cibles et vitesses d'échange de la couche de surface avec l'extérieur,
/// communes à toutes les cellules pour un pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExchangeTargets {
    pub target: [f64; WATER_POOL_COUNT],
    pub rate: [f64; WATER_POOL_COUNT],
    /// Rapport C/P de la matière organique qui sédimente.
    pub export_carbon_to_phosphorus: f64,
}

/// Cellule qui a changé d'état (océan ou terre) au dernier rafraîchissement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellChange {
    pub cell: usize,
    pub now_ocean: bool,
}

#[derive(Clone, Debug)]
pub struct Planet {
    pub params: PlanetParams,
    pub grid: GeodesicGrid,
    pub cells: Vec<CellEnvironment>,
    pub tectonics: Tectonics,
    pub climate: ClimateState,
    pub reservoirs: GlobalReservoirs,
    /// Niveau de la mer par rapport au niveau de référence des altitudes, m.
    pub sea_level_m: f64,
    /// Volume de l'océan profond, m³.
    pub deep_volume_m3: f64,
    /// Temps écoulé depuis le dernier pas tectonique, années.
    pub tectonic_clock: f64,
}

/// Pas tectonique, années (document Planète : 0,1 à 1 Ma).
pub const TECTONIC_STEP_YEARS: f64 = 1.0e6;

impl Planet {
    /// Activité volcanique et hydrothermale relative au départ.
    pub fn activity(&self, years: f64) -> f64 {
        let spreading = if self.tectonics.reference_spreading > 0.0 {
            (self.tectonics.last.spreading_rate() / self.tectonics.reference_spreading).clamp(0.2, 5.0)
        } else {
            1.0
        };
        self.params.internal_heat(years) * spreading
    }

    pub fn ocean_cells(&self) -> impl Iterator<Item = usize> + '_ {
        self.cells.iter().enumerate().filter(|(_, e)| e.is_ocean).map(|(i, _)| i)
    }

    /// Cellules portant une source hydrothermale.
    pub fn vent_cells(&self) -> Vec<usize> {
        (0..self.cells.len()).filter(|&c| self.cells[c].vent_h2_supply > 0.0).collect()
    }

    pub fn ocean_area(&self) -> f64 {
        self.cells.iter().filter(|e| e.is_ocean).map(|e| e.area_m2).sum()
    }

    pub fn ocean_fraction(&self) -> f64 {
        self.ocean_area() / self.params.surface_area()
    }

    /// Aire des terres émergées libres de glace, m².
    pub fn land_area(&self) -> f64 {
        self.cells.iter().filter(|e| !e.is_ocean).map(|e| e.area_m2 * (1.0 - e.ice_cover)).sum()
    }

    pub fn partial_pressure(&self, gas: Gas) -> f64 {
        self.reservoirs.partial_pressure(gas, self.params.gravity(), self.params.surface_area())
    }

    pub fn greenhouse(&self) -> Greenhouse {
        Greenhouse {
            co2_pa: self.partial_pressure(Gas::Co2),
            ch4_ppb: self.reservoirs.mixing_ratio(Gas::Ch4) * 1e9,
            o2_mixing: self.reservoirs.mixing_ratio(Gas::O2),
        }
    }

    /// Recalcule relief, niveau de la mer, climat et vecteur d'environnement
    /// de chaque cellule ; renvoie les cellules passées de la terre à
    /// l'océan ou l'inverse.
    pub fn refresh(&mut self, years: f64) -> Vec<CellChange> {
        let p = &self.params;
        let n = self.grid.len();
        let r2 = p.radius_m * p.radius_m;
        let areas: Vec<f64> = self.grid.unit_areas.iter().map(|a| a * r2).collect();
        let raw: Vec<f64> = (0..n).map(|c| self.tectonics.elevation_m(p, c)).collect();
        let ice_volume = 0.0;
        self.sea_level_m = sea_level(&raw, &areas, p.water_inventory_m * p.surface_area() - ice_volume);
        let elevation: Vec<f64> = raw.iter().map(|e| e - self.sea_level_m).collect();
        let is_ocean: Vec<bool> = elevation.iter().map(|&e| e < 0.0).collect();
        let g = self.greenhouse();
        let climate = self.climate.solve(p, &self.grid, &is_ocean, &elevation, years, &g);

        // Sources hydrothermales : axes des dorsales sous l'eau. Quand les
        // dorsales sont émergées (monde désertique, mers fermées dans les
        // bassins), la circulation hydrothermale passe par la croûte immergée
        // la plus jeune (sources hors axe, serpentinisation) : on garde au
        // moins une part fixe des cellules océaniques, les plus jeunes.
        let mut vents: Vec<usize> = (0..n).filter(|&c| is_ocean[c] && self.tectonics.parcel_of(c).age_myr < p.vent_crust_age_myr).collect();
        let ocean_count = is_ocean.iter().filter(|&&o| o).count();
        let min_vents = ((ocean_count as f64 * p.vent_min_ocean_share).ceil() as usize).min(ocean_count);
        if vents.len() < min_vents {
            let mut by_age: Vec<usize> = (0..n).filter(|&c| is_ocean[c]).collect();
            by_age.sort_by(|&a, &b| self.tectonics.parcel_of(a).age_myr.total_cmp(&self.tectonics.parcel_of(b).age_myr).then(a.cmp(&b)));
            by_age.truncate(min_vents);
            by_age.sort_unstable();
            vents = by_age;
        }
        let activity = self.activity(years);
        let per_vent = activity * p.vent_local_share / vents.len().max(1) as f64;

        let k = p.water_light_attenuation;
        let gravity = p.gravity();
        let surface_pressure = self.reservoirs.pressure_pa(gravity, p.surface_area());
        let mut changes = Vec::new();
        let mut surface_volume = 0.0;
        for c in 0..n {
            let lat = self.grid.latitude(c);
            let cl = climate[c];
            let ocean = is_ocean[c];
            let depth = (-elevation[c]).max(0.0);
            let layer = p.mixed_layer_m.min(depth.max(10.0));
            let ice_cover = if cl.ice { 1.0 } else { 0.0 };
            let transmission = if ocean { (1.0 - (-k * layer).exp()) / (k * layer) } else { 1.0 };
            let light = cl.insolation_w_m2 * (1.0 - p.albedo) * self.climate.light_share * transmission * (1.0 - 0.95 * ice_cover);
            let uv = cl.insolation_w_m2 * self.climate.uv_share * self.climate.uv_transmission * (1.0 - ice_cover);
            let is_vent = ocean && vents.binary_search(&c).is_ok();
            let volume = if ocean { areas[c] * layer } else { 0.0 };
            surface_volume += volume;
            let env = CellEnvironment {
                latitude_rad: lat,
                elevation_m: elevation[c],
                is_ocean: ocean,
                area_m2: areas[c],
                water_volume_m3: volume,
                temperature_k: cl.temperature_k,
                seasonal_amplitude_k: 40.0 * self.climate.obliquity_rad.sin() * lat.sin().abs() * if ocean { 0.3 } else { 1.0 },
                light_par_w_m2: light,
                uv_w_m2: uv,
                ph: p.ocean_ph,
                salinity: if ocean { p.salinity } else { 0.0 },
                pressure_pa: surface_pressure + if ocean { p.seawater_density * gravity * layer / 2.0 } else { 0.0 },
                vent_h2_supply: if is_vent { p.vent_h2_flux * per_vent } else { 0.0 },
                vent_h2s_supply: if is_vent { p.vent_h2s_flux * per_vent } else { 0.0 },
                vent_fe_supply: if is_vent { p.vent_fe_flux * per_vent } else { 0.0 },
                vent_mn_supply: if is_vent { p.vent_mn_flux * per_vent } else { 0.0 },
                ice_cover,
            };
            if let Some(old) = self.cells.get(c) {
                if old.is_ocean != ocean || (ocean && old.water_volume_m3 != volume) {
                    changes.push(CellChange { cell: c, now_ocean: ocean });
                }
            }
            if c < self.cells.len() {
                self.cells[c] = env;
            } else {
                self.cells.push(env);
            }
        }
        self.deep_volume_m3 = (p.water_inventory_m * p.surface_area() - surface_volume).max(1.0);
        changes
    }

    /// Cibles d'échange de la couche de surface pour l'état courant des
    /// réservoirs.
    pub fn exchange_targets(&self, export_carbon_to_phosphorus: f64) -> ExchangeTargets {
        let p = &self.params;
        let mut target = [0.0; WATER_POOL_COUNT];
        let mut rate = [0.0; WATER_POOL_COUNT];
        let co2 = self.partial_pressure(Gas::Co2);
        target[WaterPool::Dic as usize] = p.dic_equilibrium * (co2 / p.co2_pa).max(0.0).powf(p.dic_co2_exponent);
        rate[WaterPool::Dic as usize] = 1.0;
        // Sédimentation de la matière organique et des oxydes.
        rate[WaterPool::Doc as usize] = 0.5;
        rate[WaterPool::FeOx as usize] = 1.0;
        rate[WaterPool::MnOx as usize] = 1.0;
        for (pool, gas) in [(WaterPool::H2, Gas::H2), (WaterPool::Ch4, Gas::Ch4), (WaterPool::O2, Gas::O2)] {
            target[pool as usize] = pool.henry().unwrap_or(0.0) * self.partial_pressure(gas);
            rate[pool as usize] = 10.0;
        }
        // L'H₂S dégazé est perdu par l'atmosphère (oxydé ou lessivé).
        rate[WaterPool::H2s as usize] = 10.0;
        target[WaterPool::Sulfate as usize] = p.sulfate_equilibrium;
        rate[WaterPool::Sulfate as usize] = 0.2;
        let deep = self.deep_volume_m3.max(1.0);
        for (pool, moles) in [
            (WaterPool::Fe2, self.reservoirs.deep_fe2),
            (WaterPool::Mn2, self.reservoirs.deep_mn2),
            (WaterPool::Po4, self.reservoirs.deep_po4),
        ] {
            target[pool as usize] = moles.max(0.0) / deep;
            rate[pool as usize] = p.upwelling_rate;
        }
        ExchangeTargets { target, rate, export_carbon_to_phosphorus }
    }

    /// Chimie de départ d'une cellule océanique : chaque pool à l'équilibre
    /// de ses échanges.
    pub fn equilibrium_chemistry(&self, cell: usize, targets: &ExchangeTargets) -> WaterChemistry {
        let env = &self.cells[cell];
        let mut chem = [0.0; WATER_POOL_COUNT];
        if env.is_ocean {
            for pool in WATER_POOLS {
                let i = pool as usize;
                let rate = targets.rate[i] * self.gas_factor(env, pool);
                chem[i] = targets.target[i] + if rate > 0.0 { self.vent_supply(env, pool) / env.water_volume_m3 / rate } else { 0.0 };
            }
        }
        chem
    }

    fn vent_supply(&self, env: &CellEnvironment, pool: WaterPool) -> f64 {
        match pool {
            WaterPool::H2 => env.vent_h2_supply,
            WaterPool::H2s => env.vent_h2s_supply,
            WaterPool::Fe2 => env.vent_fe_supply,
            WaterPool::Mn2 => env.vent_mn_supply,
            _ => 0.0,
        }
    }

    /// La glace freine les échanges de gaz.
    fn gas_factor(&self, env: &CellEnvironment, pool: WaterPool) -> f64 {
        if pool.is_gas() || pool == WaterPool::Dic {
            1.0 - 0.9 * env.ice_cover
        } else {
            1.0
        }
    }

    /// Fait évoluer la chimie d'une cellule pendant `dt` années sous l'effet
    /// des échanges avec l'extérieur de la couche et des oxydations
    /// abiotiques. `out` cumule les moles sorties de la cellule vers chaque
    /// réservoir extérieur (négatif : entrées), sources hydrothermales exclues.
    pub fn exchange(&self, cell: usize, chem: &mut WaterChemistry, dt: f64, targets: &ExchangeTargets, out: &mut [f64; WATER_POOL_COUNT]) {
        let env = &self.cells[cell];
        if !env.is_ocean {
            return;
        }
        let v = env.water_volume_m3;
        for pool in WATER_POOLS {
            let i = pool as usize;
            let rate = targets.rate[i] * self.gas_factor(env, pool);
            let supply = self.vent_supply(env, pool);
            let before = chem[i];
            if rate > 0.0 {
                // Solution exacte de dc/dt = rate·(target − c) + supply/V.
                let eq = targets.target[i] + supply / v / rate;
                chem[i] = eq + (before - eq) * (-rate * dt).exp();
            } else {
                chem[i] += supply / v * dt;
            }
            out[i] += supply * dt - (chem[i] - before) * v;
            if pool == WaterPool::Doc {
                // La matière organique qui sédimente emporte son phosphore.
                let exported = (before - chem[i]).max(0.0);
                let po4 = WaterPool::Po4 as usize;
                let p = (exported / targets.export_carbon_to_phosphorus).min(chem[po4].max(0.0));
                chem[po4] -= p;
                out[po4] += p * v;
            }
        }
        // Oxydations abiotiques par l'O₂ dissous : fer (rapide), sulfure,
        // manganèse (lent sans microbes). Constantes en (mol·m⁻³)⁻¹·an⁻¹.
        let o2 = WaterPool::O2 as usize;
        for (reduced, oxidised, k, per_o2) in [
            (WaterPool::Fe2, Some(WaterPool::FeOx), 1e4, 4.0),
            (WaterPool::H2s, Some(WaterPool::Sulfate), 1e3, 0.5),
            (WaterPool::Mn2, Some(WaterPool::MnOx), 10.0, 2.0),
        ] {
            let (r, ox) = (chem[reduced as usize].max(0.0), chem[o2].max(0.0));
            if r <= 0.0 || ox <= 0.0 {
                continue;
            }
            let reacted = (r * -(-k * ox * dt).exp_m1()).min(ox * per_o2);
            chem[reduced as usize] -= reacted;
            chem[o2] -= reacted / per_o2;
            if let Some(p) = oxidised {
                chem[p as usize] += reacted;
            }
        }
    }

    /// Carbone total de la couche d'eau, mol.
    pub fn water_carbon(&self, chemistry: &[WaterChemistry]) -> f64 {
        self.water_sum(chemistry, |p| p.carbon_atoms())
    }

    /// Phosphore total de la couche d'eau, mol.
    pub fn water_phosphorus(&self, chemistry: &[WaterChemistry]) -> f64 {
        self.water_sum(chemistry, |p| p.phosphorus_atoms())
    }

    fn water_sum(&self, chemistry: &[WaterChemistry], atoms: impl Fn(WaterPool) -> f64) -> f64 {
        self.cells
            .iter()
            .zip(chemistry)
            .filter(|(e, _)| e.is_ocean)
            .map(|(e, c)| WATER_POOLS.iter().map(|&p| c[p as usize] * atoms(p)).sum::<f64>() * e.water_volume_m3)
            .sum()
    }

    /// Contexte des boîtes globales pour le pas courant.
    pub fn box_context(&self, years: f64) -> BoxContext {
        let ocean_unit: f64 = (0..self.cells.len()).filter(|&c| self.cells[c].is_ocean).map(|c| self.grid.unit_areas[c]).sum();
        let last = &self.tectonics.last;
        let subduction = if last.years > 0.0 && ocean_unit > 0.0 { last.subducted_area / ocean_unit / last.years } else { 0.0 };
        BoxContext {
            land_area_m2: self.land_area(),
            deep_volume_m3: self.deep_volume_m3,
            mean_temperature_k: self.climate.mean_temperature_k,
            activity: self.activity(years),
            subduction_per_year: subduction,
            gravity: self.params.gravity(),
            area_m2: self.params.surface_area(),
        }
    }

    pub fn memory_bytes(&self) -> usize {
        self.grid.memory_bytes() + self.cells.capacity() * std::mem::size_of::<CellEnvironment>() + self.tectonics.memory_bytes()
    }
}
