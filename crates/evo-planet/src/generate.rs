//! Génération d'une planète à partir des paramètres et d'une graine : grille,
//! plaques et continents, mise en route de la tectonique, atmosphère de
//! départ, climat d'équilibre.

use crate::climate::ClimateState;
use crate::environment::{Planet, TECTONIC_STEP_YEARS};
use crate::geochem::GlobalReservoirs;
use crate::grid::GeodesicGrid;
use crate::params::PlanetParams;
use crate::tectonics::Tectonics;

/// Pas tectoniques de mise en route : les dorsales et les zones de
/// subduction se forment avant le début de la partie.
const SPIN_UP_STEPS: u64 = 40;

pub fn generate(params: PlanetParams, level: u32, seed: u64) -> Planet {
    let grid = GeodesicGrid::new(level);
    let mut tectonics = Tectonics::new(&grid, &params, seed);
    let mut spreading = 0.0;
    for i in 0..SPIN_UP_STEPS {
        // Mise en route à rebours : la partie commence au temps 0.
        let t = (i as f64 - SPIN_UP_STEPS as f64) * TECTONIC_STEP_YEARS;
        tectonics.step(&grid, &params, t, TECTONIC_STEP_YEARS, 1.0, seed);
        if i >= SPIN_UP_STEPS - 10 {
            spreading += tectonics.last.spreading_rate() / 10.0;
        }
    }
    tectonics.reference_spreading = spreading;
    tectonics.last_reorganisation_years = 0.0;
    let water = params.water_inventory_m * params.surface_area();
    let n = grid.len();
    let mut planet = Planet {
        climate: ClimateState::new(&params, n),
        reservoirs: GlobalReservoirs::new(&params, water),
        params,
        grid,
        cells: Vec::with_capacity(n),
        tectonics,
        sea_level_m: 0.0,
        deep_volume_m3: water,
        tectonic_clock: 0.0,
    };
    planet.refresh(0.0);
    // L'état de glace de départ est celui de l'équilibre chaud.
    planet.refresh(0.0);
    planet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geochem::Gas;
    use crate::pools::WaterPool;
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
    fn earth_has_oceans_continents_climate_and_vents() {
        let p = generate(PlanetParams::earth_archean(), 5, 1);
        let f = p.ocean_fraction();
        assert!((0.55..0.85).contains(&f), "part d'océan {f}");
        let eq = p.cells.iter().filter(|c| c.is_ocean && c.latitude_rad.abs() < 0.1).map(|c| c.temperature_k).fold(0.0, f64::max);
        let pole = p.cells.iter().filter(|c| c.latitude_rad.abs() > 1.4).map(|c| c.temperature_k).fold(f64::MAX, f64::min);
        assert!(eq > pole + 20.0, "équateur {eq} K, pôle {pole} K");
        assert!((270.0..305.0).contains(&p.climate.mean_temperature_k), "{}", p.climate.mean_temperature_k);
        assert!(!p.vent_cells().is_empty());
        assert_eq!(p.reservoirs.mixing_ratio(Gas::O2), 0.0);
    }

    #[test]
    fn water_inventory_sets_the_ocean_fraction() {
        let ocean = generate(PlanetParams::ocean_world(), 4, 1);
        let desert = generate(PlanetParams::desert_world(), 4, 1);
        assert!(ocean.ocean_fraction() > 0.99, "{}", ocean.ocean_fraction());
        assert!(desert.ocean_fraction() < 0.4, "{}", desert.ocean_fraction());
        assert!(desert.ocean_fraction() > 0.0);
        // Dorsales émergées : les sources passent sur la croûte immergée la
        // plus jeune, la vie a toujours où commencer.
        assert!(!desert.vent_cells().is_empty());
        assert!(desert.vent_cells().iter().all(|&c| desert.cells[c].is_ocean));
    }

    #[test]
    fn lifeless_chemistry_is_at_equilibrium_and_conserves_carbon() {
        let p = generate(PlanetParams::earth_archean(), 3, 1);
        let targets = p.exchange_targets(106.0);
        let start: Vec<_> = (0..p.cells.len()).map(|c| p.equilibrium_chemistry(c, &targets)).collect();
        let mut chemistry = start.clone();
        let mut flux = FluxRegistry::default();
        flux.set_initial(Element::Carbon, p.water_carbon(&chemistry));
        let mut out = [0.0; crate::pools::WATER_POOL_COUNT];
        for (c, chem) in chemistry.iter_mut().enumerate() {
            p.exchange(c, chem, 1.0, &targets, &mut out);
            for (x, y) in chem.iter().zip(&start[c]) {
                assert!((x - y).abs() <= 1e-9 * y.abs().max(1e-12), "{x} contre {y}");
            }
        }
        assert!(out[WaterPool::Dic as usize].abs() < 1e-6 * p.water_carbon(&chemistry));
    }
}
