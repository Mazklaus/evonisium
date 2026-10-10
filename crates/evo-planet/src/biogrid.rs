//! Grille du vivant (document Vision, révision du budget du 8 octobre 2026).
//!
//! Pendant l'ère microbienne, le vivant est calculé sur une grille plus
//! grossière que la planète : niveau 5 (10 242 cellules d'environ 220 km)
//! sous une planète de niveau 6 (40 962 cellules). Les sommets d'une grille
//! géodésique de niveau n sont les premiers sommets de la grille de niveau
//! n + 1 : chaque cellule physique appartient à la cellule du vivant la plus
//! proche, qui en regroupe environ quatre et lit la moyenne de leurs
//! conditions.
//!
//! [Simplification] Un tapis microbien ne demande pas une résolution de
//! 110 km ; le vivant passe à la grille physique à l'arrivée de la vie
//! multicellulaire sur les terres (étape 4).

use crate::environment::CellEnvironment;
use crate::grid::GeodesicGrid;
use crate::tectonics::locate;

#[derive(Clone, Debug)]
pub struct BioGrid {
    pub grid: GeodesicGrid,
    /// Cellule du vivant de chaque cellule physique.
    pub parent: Vec<u32>,
    /// Cellules physiques de chaque cellule du vivant.
    pub children: Vec<Vec<u32>>,
    /// Conditions moyennes de chaque cellule du vivant.
    pub env: Vec<CellEnvironment>,
}

impl BioGrid {
    /// Grille du vivant de niveau `level` sous la grille physique `physical`.
    pub fn new(physical: &GeodesicGrid, level: u32) -> Self {
        let level = level.min(physical.level);
        let grid = GeodesicGrid::new(level);
        let mut parent = Vec::with_capacity(physical.len());
        let mut last = 0usize;
        for c in 0..physical.len() {
            let p = if c < grid.len() { c } else { locate(&grid, last, physical.centers[c]) };
            last = p;
            parent.push(p as u32);
        }
        let mut children = vec![Vec::new(); grid.len()];
        for (c, &p) in parent.iter().enumerate() {
            children[p as usize].push(c as u32);
        }
        Self { grid, parent, children, env: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.grid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    /// Recalcule les conditions de chaque cellule du vivant depuis les
    /// cellules physiques. Les cellules d'eau comptent seules pour la couche
    /// d'eau (volume, lumière, température, chimie) : la vie microbienne vit
    /// dans l'eau des cellules filles qui en ont. Sans eau, la moyenne porte
    /// sur toutes les filles.
    pub fn aggregate(&mut self, physical: &[CellEnvironment]) {
        let env: Vec<CellEnvironment> = (0..self.grid.len())
            .map(|b| {
                let kids = &self.children[b];
                let wet: Vec<&CellEnvironment> = kids.iter().map(|&c| &physical[c as usize]).filter(|e| e.water_volume_m3 > 0.0).collect();
                let all: Vec<&CellEnvironment> = kids.iter().map(|&c| &physical[c as usize]).collect();
                let (set, weight): (&[&CellEnvironment], fn(&CellEnvironment) -> f64) =
                    if wet.is_empty() { (&all, |e| e.area_m2) } else { (&wet, |e| e.water_volume_m3) };
                let total: f64 = set.iter().map(|e| weight(e)).sum::<f64>().max(f64::MIN_POSITIVE);
                let mean = |f: fn(&CellEnvironment) -> f64| set.iter().map(|e| weight(e) * f(e)).sum::<f64>() / total;
                let sum = |f: fn(&CellEnvironment) -> f64| set.iter().map(|e| f(e)).sum::<f64>();
                let ocean_volume: f64 = wet.iter().filter(|e| e.is_ocean).map(|e| e.water_volume_m3).sum();
                let wet_volume: f64 = wet.iter().map(|e| e.water_volume_m3).sum();
                CellEnvironment {
                    latitude_rad: self.grid.latitude(b),
                    elevation_m: mean(|e| e.elevation_m),
                    // Une cellule du vivant est marine si l'essentiel de son
                    // eau est de la mer ; sinon ses eaux sont des lacs.
                    is_ocean: wet_volume > 0.0 && ocean_volume >= 0.5 * wet_volume,
                    area_m2: all.iter().map(|e| e.area_m2).sum(),
                    water_volume_m3: wet_volume,
                    water_area_m2: sum(|e| e.water_area_m2),
                    temperature_k: mean(|e| e.temperature_k),
                    seasonal_amplitude_k: mean(|e| e.seasonal_amplitude_k),
                    light_par_w_m2: mean(|e| e.light_par_w_m2),
                    uv_w_m2: mean(|e| e.uv_w_m2),
                    ph: mean(|e| e.ph),
                    salinity: mean(|e| e.salinity),
                    pressure_pa: mean(|e| e.pressure_pa),
                    vent_h2_supply: sum(|e| e.vent_h2_supply),
                    vent_h2s_supply: sum(|e| e.vent_h2s_supply),
                    vent_fe_supply: sum(|e| e.vent_fe_supply),
                    vent_mn_supply: sum(|e| e.vent_mn_supply),
                    ice_cover: mean(|e| e.ice_cover),
                    dry_area_m2: all.iter().map(|e| e.dry_area_m2).sum(),
                    land_light_par_w_m2: weighted(&all, |e| e.dry_area_m2, |e| e.land_light_par_w_m2),
                    moisture: weighted(&all, |e| e.dry_area_m2, |e| e.moisture),
                    phosphorus_supply: all.iter().map(|e| e.phosphorus_supply).sum(),
                    soil_phosphate: weighted(&all, |e| e.dry_area_m2, |e| e.soil_phosphate),
                    flushing_per_year: mean(|e| e.flushing_per_year),
                    rain_mm_yr: mean(|e| e.rain_mm_yr),
                }
            })
            .collect();
        self.env = env;
    }

    /// Cellules du vivant qui ont une couche d'eau.
    pub fn wet_cells(&self) -> impl Iterator<Item = usize> + '_ {
        self.env.iter().enumerate().filter(|(_, e)| e.water_volume_m3 > 0.0).map(|(i, _)| i)
    }

    /// Cellules du vivant portant une source hydrothermale.
    pub fn vent_cells(&self) -> Vec<usize> {
        (0..self.env.len()).filter(|&c| self.env[c].vent_h2_supply > 0.0).collect()
    }

    pub fn memory_bytes(&self) -> usize {
        self.grid.memory_bytes()
            + self.parent.capacity() * 4
            + self.children.iter().map(|v| v.capacity() * 4 + 24).sum::<usize>()
            + self.env.capacity() * std::mem::size_of::<CellEnvironment>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coarse_vertices_nest_in_the_fine_grid() {
        let fine = GeodesicGrid::new(4);
        let bio = BioGrid::new(&fine, 3);
        assert_eq!(bio.len(), GeodesicGrid::cell_count_for_level(3));
        for b in 0..bio.len() {
            assert_eq!(fine.centers[b], bio.grid.centers[b]);
            assert!(bio.children[b].contains(&(b as u32)));
        }
        let sizes: Vec<usize> = bio.children.iter().map(Vec::len).collect();
        assert_eq!(sizes.iter().sum::<usize>(), fine.len());
        assert!(sizes.iter().all(|&s| (1..=7).contains(&s)), "{:?}", sizes.iter().max());
        let same = BioGrid::new(&fine, 4);
        assert!(same.parent.iter().enumerate().all(|(c, &p)| p as usize == c));
    }
}

/// Moyenne de `f` pondérée par `w` (zéro sans poids).
fn weighted(set: &[&CellEnvironment], w: fn(&CellEnvironment) -> f64, f: fn(&CellEnvironment) -> f64) -> f64 {
    let total: f64 = set.iter().map(|e| w(e)).sum();
    if total <= 0.0 {
        return 0.0;
    }
    set.iter().map(|e| w(e) * f(e)).sum::<f64>() / total
}
