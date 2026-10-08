//! Diagnostics de surface : vents, pluie, rivières, lacs, nuages, courants
//! et vitesse des plaques (document Planète, section 14 ; document Vision,
//! « Sorties d'affichage de la planète »).
//!
//! Le climat reste celui du bilan d'énergie (module `climate`) : rien ici ne
//! le modifie. Ces champs servent à deux choses : l'affichage (globe, calques)
//! et l'eau des terres, où vivent les tapis microbiens des lacs et des sols
//! humides.
//!
//! [Simplification] Circulation paramétrée, pas résolue (décision du 7 octobre
//! 2026) :
//! - vents de surface en trois cellules par hémisphère (alizés, vents
//!   d'ouest, vents polaires), fonction de la seule latitude ;
//! - pluie par bandes de latitude (convergence équatoriale, déserts
//!   subtropicaux, fronts des moyennes latitudes), qui décroît avec la
//!   distance à la mer et suit la loi de Clausius-Clapeyron (7 % par kelvin),
//!   puis ramenée à ce que les mers évaporent ;
//! - rivières par plus grande pente, lacs aux points bas et le long des
//!   grands fleuves, sols humides proportionnels à la pluie ;
//! - nuages déduits de la pluie, sans effet sur l'albédo du climat ;
//! - courants de surface entraînés par le vent, déviés le long des côtes.

use crate::grid::{GeodesicGrid, Vec3};
use crate::params::PlanetParams;
use crate::tectonics::Tectonics;
use std::collections::VecDeque;

/// Ce qu'une cellule physique montre en plus de son environnement.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CellDisplay {
    /// Vent de surface, m·s⁻¹ (composantes vers l'est et vers le nord).
    pub wind_ms: [f32; 2],
    /// Courant de surface, m·s⁻¹ (est, nord), nul sur les terres.
    pub current_ms: [f32; 2],
    /// Vitesse de la plaque, cm·an⁻¹ (est, nord).
    pub plate_velocity_cm_yr: [f32; 2],
    /// Pluie, mm·an⁻¹.
    pub rain_mm_yr: f32,
    /// Couverture nuageuse approchée, de 0 à 1.
    pub cloud_cover: f32,
    /// Débit de la rivière qui quitte la cellule, m³·s⁻¹.
    pub river_flow_m3s: f32,
    /// Cellule où coule la rivière (`u32::MAX` : mer, ou lac sans exutoire).
    pub river_downstream: u32,
    /// Part de la cellule couverte d'eaux douces (lacs, zones humides).
    pub lake_fraction: f32,
}

/// Eau des terres d'une cellule, pour la vie.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LandWater {
    /// Part de la surface occupée par des lacs, des zones humides et des sols
    /// assez humides pour un tapis microbien.
    pub wet_fraction: f64,
    /// Renouvellement de l'eau des lacs par la pluie et les rivières, an⁻¹.
    pub flushing_per_year: f64,
}

/// Interpolation linéaire dans une table (latitude en degrés, valeur).
fn table(points: &[(f64, f64)], x: f64) -> f64 {
    let x = x.clamp(points[0].0, points[points.len() - 1].0);
    for w in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    points[points.len() - 1].1
}

/// Base locale (est, nord) au point `p` de la sphère unité.
fn local_basis(p: Vec3) -> (Vec3, Vec3) {
    let east = {
        let e = [-p[1], p[0], 0.0];
        let n = (e[0] * e[0] + e[1] * e[1]).sqrt();
        if n < 1e-12 {
            [1.0, 0.0, 0.0]
        } else {
            [e[0] / n, e[1] / n, 0.0]
        }
    };
    let north = [p[1] * east[2] - p[2] * east[1], p[2] * east[0] - p[0] * east[2], p[0] * east[1] - p[1] * east[0]];
    (east, north)
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Profil des vents zonaux (vers l'est positif), en part de la vitesse de
/// référence, selon la latitude absolue.
const ZONAL: [(f64, f64); 7] = [(0.0, -1.0), (30.0, 0.0), (45.0, 1.0), (60.0, 0.0), (75.0, -0.5), (89.0, -0.2), (90.0, 0.0)];
/// Composante méridienne (vers le pôle positif) : convergence vers
/// l'équateur dans la cellule de Hadley, vers le front polaire ensuite.
const MERIDIONAL: [(f64, f64); 7] = [(0.0, 0.0), (15.0, -0.3), (30.0, 0.0), (45.0, 0.2), (60.0, 0.0), (75.0, -0.1), (90.0, 0.0)];
/// Pluie de référence, mm·an⁻¹, d'une mer à 288 K.
const RAIN: [(f64, f64); 9] = [
    (0.0, 2200.0),
    (10.0, 1600.0),
    (20.0, 800.0),
    (30.0, 350.0),
    (40.0, 800.0),
    (50.0, 1100.0),
    (60.0, 800.0),
    (70.0, 400.0),
    (90.0, 150.0),
];

/// Calcule les diagnostics de surface de toutes les cellules physiques.
#[allow(clippy::too_many_arguments)]
pub fn diagnose(
    params: &PlanetParams,
    grid: &GeodesicGrid,
    tectonics: &Tectonics,
    elevation: &[f64],
    is_ocean: &[bool],
    temperature_k: &[f64],
    ice: &[bool],
    areas_m2: &[f64],
) -> (Vec<CellDisplay>, Vec<LandWater>) {
    let n = grid.len();
    let mut display = vec![CellDisplay { river_downstream: u32::MAX, ..Default::default() }; n];

    // Distance à la mer, km, par parcours en largeur sur la grille.
    let spacing_km = grid.mean_spacing() * params.radius_m / 1e3;
    let mut dist = vec![u32::MAX; n];
    let mut queue: VecDeque<usize> = (0..n).filter(|&c| is_ocean[c]).collect();
    for &c in &queue {
        dist[c] = 0;
    }
    while let Some(c) = queue.pop_front() {
        for nb in grid.neighbours_of(c) {
            if dist[nb] == u32::MAX {
                dist[nb] = dist[c] + 1;
                queue.push_back(nb);
            }
        }
    }

    // Vents, pluie brute.
    let mut rain = vec![0.0f64; n];
    for c in 0..n {
        let p = grid.centers[c];
        let lat = grid.latitude(c);
        let deg = lat.abs().to_degrees();
        let hemi = if lat >= 0.0 { 1.0 } else { -1.0 };
        let u = params.wind_reference_ms * table(&ZONAL, deg);
        let v = params.wind_reference_ms * table(&MERIDIONAL, deg) * hemi;
        display[c].wind_ms = [u as f32, v as f32];
        let moisture = if dist[c] == u32::MAX { 0.0 } else { (-(dist[c] as f64) * spacing_km / params.moisture_range_km).exp() };
        let cc = (0.07 * (temperature_k[c] - 288.0)).exp().clamp(0.02, 4.0);
        let frozen = if ice[c] { 0.3 } else { 1.0 };
        rain[c] = table(&RAIN, deg) * moisture * cc * frozen;
        let _ = p;
    }
    // La pluie totale ne peut dépasser ce que les mers évaporent (plus le
    // recyclage par les terres, compté dans la pluie de référence).
    let ocean_area: f64 = (0..n).filter(|&c| is_ocean[c]).map(|c| areas_m2[c]).sum();
    let evaporation: f64 = (0..n)
        .filter(|&c| is_ocean[c] && !ice[c])
        .map(|c| areas_m2[c] * params.ocean_evaporation_m_yr * (0.07 * (temperature_k[c] - 288.0)).exp().clamp(0.02, 4.0))
        .sum::<f64>()
        * 1e3;
    let total: f64 = (0..n).map(|c| rain[c] * areas_m2[c]).sum();
    if total > evaporation && total > 0.0 {
        let k = evaporation / total;
        for r in rain.iter_mut() {
            *r *= k;
        }
    }
    let _ = ocean_area;

    // Rivières : chaque cellule de terre s'écoule vers sa voisine la plus
    // basse ; le débit cumule l'écoulement de l'amont, traité des plus hautes
    // aux plus basses.
    let mut land: Vec<usize> = (0..n).filter(|&c| !is_ocean[c]).collect();
    land.sort_by(|&a, &b| elevation[b].total_cmp(&elevation[a]).then(a.cmp(&b)));
    let mut flow = vec![0.0f64; n];
    let mut sink_inflow = vec![0.0f64; n];
    for &c in &land {
        let runoff = rain[c] * 1e-3 * params.runoff_coefficient * areas_m2[c] / evo_core::units::SECONDS_PER_YEAR;
        flow[c] += runoff;
        let down = grid.neighbours_of(c).filter(|&nb| elevation[nb] < elevation[c]).min_by(|&a, &b| elevation[a].total_cmp(&elevation[b]));
        match down {
            Some(d) => {
                display[c].river_downstream = d as u32;
                if !is_ocean[d] {
                    flow[d] += flow[c];
                }
            }
            None => sink_inflow[c] = flow[c],
        }
        display[c].river_flow_m3s = flow[c] as f32;
    }

    // Lacs et sols humides.
    let mut water = vec![LandWater::default(); n];
    for &c in &land {
        if ice[c] {
            continue;
        }
        let rain_m = rain[c] * 1e-3;
        // Sols assez humides pour un tapis microbien : croûtes biologiques des
        // sols arides, tapis des zones humides.
        let soil = params.wet_soil_fraction * (rain_m / 1.0).min(1.0);
        // Lacs le long des grands fleuves et aux points bas sans exutoire.
        let q = flow[c];
        let river = 0.05 * q / (q + params.lake_flow_scale_m3s);
        let basin = if sink_inflow[c] > 0.0 { 0.3 * sink_inflow[c] / (sink_inflow[c] + params.lake_flow_scale_m3s) } else { 0.0 };
        let lakes = river + basin;
        let wet = (soil + lakes).min(0.5);
        let inflow_m_yr = q * evo_core::units::SECONDS_PER_YEAR / (areas_m2[c] * wet.max(1e-6));
        water[c] = LandWater { wet_fraction: wet, flushing_per_year: (inflow_m_yr / params.lake_layer_m).clamp(0.05, 10.0) };
        display[c].lake_fraction = lakes.min(0.5) as f32;
    }

    // Nuages, courants, plaques.
    for c in 0..n {
        let p = grid.centers[c];
        display[c].rain_mm_yr = rain[c] as f32;
        display[c].cloud_cover = (0.1 + 0.6 * rain[c] / (rain[c] + 1000.0)).min(0.9) as f32;
        let (east, north) = local_basis(p);
        if is_ocean[c] && !ice[c] {
            // Courant entraîné par le vent (quelques pour cent du vent),
            // privé de sa composante vers les côtes.
            let [u, v] = display[c].wind_ms;
            let mut cur = [0.03 * u as f64, 0.03 * v as f64];
            for nb in grid.neighbours_of(c) {
                if is_ocean[nb] {
                    continue;
                }
                let q = grid.centers[nb];
                let d = [dot(q, east), dot(q, north)];
                let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-12);
                let d = [d[0] / len, d[1] / len];
                let along = cur[0] * d[0] + cur[1] * d[1];
                if along > 0.0 {
                    cur[0] -= along * d[0];
                    cur[1] -= along * d[1];
                }
            }
            display[c].current_ms = [cur[0] as f32, cur[1] as f32];
        }
        // Vitesse de la plaque : ω × r (ω en rad·Ma⁻¹ autour du pôle).
        let plate = &tectonics.plates[tectonics.parcel_of(c).plate as usize];
        let w = plate.pole;
        let vel = [w[1] * p[2] - w[2] * p[1], w[2] * p[0] - w[0] * p[2], w[0] * p[1] - w[1] * p[0]];
        let k = plate.omega * params.radius_m / 1e6 * 100.0;
        display[c].plate_velocity_cm_yr = [(k * dot(vel, east)) as f32, (k * dot(vel, north)) as f32];
    }
    (display, water)
}
