//! Génération d'une planète fixe à partir des paramètres et d'une graine.
//!
//! Simplification de l'étape 1 : le relief est une somme de reliefs aléatoires
//! lisses (pas encore de tectonique), le climat est un bilan d'énergie moyen par
//! latitude (pas de circulation), et les sources hydrothermales sont tirées au
//! hasard parmi les cellules océaniques profondes.

use crate::environment::{CellEnvironment, Planet};
use crate::grid::GeodesicGrid;
use crate::params::PlanetParams;
use evo_core::rng::{rng_for, Stream};
use rand::Rng;

/// Polynôme de Legendre d'ordre 2.
#[inline]
fn p2(x: f64) -> f64 {
    0.5 * (3.0 * x * x - 1.0)
}

/// Ensoleillement moyen annuel au sommet de l'atmosphère, W·m⁻², en fonction
/// de la latitude (approximation de North, 1975, avec s₂ déduit de l'obliquité).
pub fn annual_insolation(params: &PlanetParams, latitude: f64) -> f64 {
    let s2 = -0.625 * p2(params.obliquity_rad.cos());
    params.stellar_flux() / 4.0 * (1.0 + s2 * p2(latitude.sin()))
}

/// Relief : somme de bosses gaussiennes aléatoires sur la sphère, puis niveau
/// de la mer placé pour obtenir la part d'océan demandée.
fn elevations(grid: &GeodesicGrid, params: &PlanetParams, seed: u64) -> Vec<f64> {
    let mut rng = rng_for(seed, Stream::PlanetGeneration, &[1]);
    let bumps: Vec<([f64; 3], f64, f64)> = (0..96)
        .map(|_| {
            // Point uniforme sur la sphère.
            let z: f64 = rng.random_range(-1.0..1.0);
            let phi: f64 = rng.random_range(0.0..std::f64::consts::TAU);
            let r = (1.0 - z * z).sqrt();
            let width: f64 = rng.random_range(0.15..0.6);
            let height: f64 = rng.random_range(-1.0..1.0);
            ([r * phi.cos(), r * phi.sin(), z], width, height)
        })
        .collect();
    let raw: Vec<f64> = grid
        .centers
        .iter()
        .map(|c| {
            bumps
                .iter()
                .map(|(b, w, h)| {
                    let ang = (c[0] * b[0] + c[1] * b[1] + c[2] * b[2]).clamp(-1.0, 1.0).acos();
                    h * (-(ang / w).powi(2)).exp()
                })
                .sum()
        })
        .collect();
    let mut sorted = raw.clone();
    sorted.sort_by(f64::total_cmp);
    let idx = ((params.ocean_fraction * sorted.len() as f64) as usize).min(sorted.len() - 1);
    let sea = sorted[idx];
    let (lo, hi) = (sorted[0], sorted[sorted.len() - 1]);
    raw.iter()
        .map(|&v| if v < sea { -4000.0 * (sea - v) / (sea - lo).max(1e-9) - 200.0 } else { 3000.0 * (v - sea) / (hi - sea).max(1e-9) })
        .collect()
}

/// Génère une planète fixe : grille, relief, climat moyen, chimie initiale.
pub fn generate(params: PlanetParams, level: u32, seed: u64) -> Planet {
    let grid = GeodesicGrid::new(level);
    let elev = elevations(&grid, &params, seed);
    let g = params.gravity();
    let r2 = params.radius_m * params.radius_m;
    let mean_t = params.mean_surface_temperature();
    let mean_absorbed = params.stellar_flux() / 4.0 * (1.0 - params.albedo);
    let k = params.water_light_attenuation;
    let h = params.mixed_layer_m;
    let mut vent_rng = rng_for(seed, Stream::PlanetGeneration, &[2]);

    let mut cells: Vec<CellEnvironment> = (0..grid.len())
        .map(|c| {
            let lat = grid.latitude(c);
            let q = annual_insolation(&params, lat);
            let is_ocean = elev[c] < 0.0;
            let mut t = mean_t + (q * (1.0 - params.albedo) - mean_absorbed) / params.heat_transport;
            if !is_ocean {
                t -= params.lapse_rate() * elev[c];
            }
            let amplitude = 40.0 * params.obliquity_rad.sin() * lat.sin().abs() * if is_ocean { 0.3 } else { 1.0 };
            let par_surface = q * (1.0 - params.albedo) * 0.43;
            let area = grid.unit_areas[c] * r2;
            CellEnvironment {
                latitude_rad: lat,
                elevation_m: elev[c],
                is_ocean,
                area_m2: area,
                water_volume_m3: if is_ocean { area * h } else { 0.0 },
                // L'eau de mer gèle vers 271 K ; la glace n'est pas simulée.
                temperature_k: if is_ocean { t.max(271.2) } else { t },
                seasonal_amplitude_k: amplitude,
                light_par_w_m2: if is_ocean { par_surface * (1.0 - (-k * h).exp()) / (k * h) } else { par_surface },
                uv_w_m2: q * (1.0 - params.albedo) * 0.05,
                ph: params.ocean_ph,
                salinity: if is_ocean { params.salinity } else { 0.0 },
                pressure_pa: params.surface_pressure_pa + if is_ocean { params.seawater_density * g * h / 2.0 } else { 0.0 },
                vent_h2_supply: 0.0,
                vent_h2s_supply: 0.0,
            }
        })
        .collect();

    // Sources hydrothermales parmi les fonds océaniques profonds.
    let deep: Vec<usize> = (0..cells.len()).filter(|&c| cells[c].elevation_m < -1500.0).collect();
    let vents: Vec<usize> = deep.iter().copied().filter(|_| vent_rng.random::<f64>() < params.vent_fraction).collect();
    let n = vents.len().max(1) as f64;
    for &c in &vents {
        cells[c].vent_h2_supply = params.vent_h2_flux / n;
        cells[c].vent_h2s_supply = params.vent_h2s_flux / n;
    }

    Planet { params, grid, cells }
}

impl Planet {
    /// Cellules portant une source hydrothermale.
    pub fn vent_cells(&self) -> Vec<usize> {
        (0..self.cells.len()).filter(|&c| self.cells[c].vent_h2_supply > 0.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::flux::{Element, FluxRegistry};

    #[test]
    fn generation_is_deterministic_and_seed_dependent() {
        let a = generate(PlanetParams::earth_archean(), 3, 7);
        let b = generate(PlanetParams::earth_archean(), 3, 7);
        let c = generate(PlanetParams::earth_archean(), 3, 8);
        assert_eq!(a.cells, b.cells);
        assert_ne!(a.cells, c.cells);
    }

    #[test]
    fn ocean_fraction_climate_and_vents() {
        let p = generate(PlanetParams::earth_archean(), 5, 1);
        let total: f64 = p.cells.iter().map(|c| c.area_m2).sum();
        let ocean: f64 = p.cells.iter().filter(|c| c.is_ocean).map(|c| c.area_m2).sum();
        assert!((ocean / total - 0.71).abs() < 0.03, "part d'océan {}", ocean / total);
        let eq = p.cells.iter().filter(|c| c.is_ocean && c.latitude_rad.abs() < 0.1).map(|c| c.temperature_k).fold(0.0, f64::max);
        let pole = p.cells.iter().filter(|c| c.latitude_rad.abs() > 1.4).map(|c| c.temperature_k).fold(f64::MAX, f64::min);
        assert!(eq > pole + 20.0, "équateur {eq} K, pôle {pole} K");
        assert!(!p.vent_cells().is_empty());
    }

    #[test]
    fn lifeless_chemistry_is_at_equilibrium_and_conserves_carbon() {
        let p = generate(PlanetParams::earth_archean(), 3, 1);
        let start = p.initial_chemistry();
        let mut chemistry = start.clone();
        let mut flux = FluxRegistry::default();
        flux.set_initial(Element::Carbon, p.water_carbon(&chemistry));
        for (c, chem) in chemistry.iter_mut().enumerate() {
            p.exchange(c, chem, 1.0, &mut flux);
            for (x, y) in chem.iter().zip(&start[c]) {
                assert!((x - y).abs() <= 1e-12 * y.abs().max(1e-12));
            }
        }
        assert!(flux.relative_error(Element::Carbon, p.water_carbon(&chemistry)) < 1e-12);
    }
}
