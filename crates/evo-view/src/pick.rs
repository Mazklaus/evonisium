//! Sélection : quelle cellule se trouve sous le curseur.
//!
//! La grille subdivisée garde les sommets des niveaux précédents en tête de
//! liste : les 642 premières cellules forment la grille du niveau 3. On part
//! de la plus proche d'entre elles, puis on descend de voisin en voisin vers
//! le point visé ; sur une grille de Voronoï sphérique, cette descente finit
//! sur la cellule la plus proche (quelques microsecondes).

use evo_planet::grid::GeodesicGrid;

#[derive(Clone, Debug)]
pub struct CellLocator {
    coarse: usize,
}

impl CellLocator {
    pub fn new(grid: &GeodesicGrid) -> Self {
        let level = grid.level.min(3);
        Self { coarse: GeodesicGrid::cell_count_for_level(level).min(grid.len()) }
    }

    /// Cellule la plus proche d'une direction (pas forcément unitaire).
    pub fn locate(&self, grid: &GeodesicGrid, dir: [f64; 3]) -> usize {
        let n = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if !(n > 0.0) {
            return 0;
        }
        let p = [dir[0] / n, dir[1] / n, dir[2] / n];
        let score = |c: usize| {
            let q = grid.centers[c];
            p[0] * q[0] + p[1] * q[1] + p[2] * q[2]
        };
        let mut best = (0..self.coarse).max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap_or(0);
        loop {
            let next = grid.neighbours_of(best).max_by(|&a, &b| score(a).total_cmp(&score(b)));
            match next {
                Some(nb) if score(nb) > score(best) => best = nb,
                _ => return best,
            }
        }
    }
}

/// Intersection d'un rayon avec la sphère de rayon `r` centrée à l'origine :
/// point d'entrée, s'il existe.
pub fn ray_sphere(origin: [f64; 3], dir: [f64; 3], r: f64) -> Option<[f64; 3]> {
    let d2 = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
    let b = origin[0] * dir[0] + origin[1] * dir[1] + origin[2] * dir[2];
    let c = origin[0] * origin[0] + origin[1] * origin[1] + origin[2] * origin[2] - r * r;
    let disc = b * b - d2 * c;
    if disc < 0.0 || d2 == 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / d2;
    let t = if t >= 0.0 { t } else { (-b + disc.sqrt()) / d2 };
    (t >= 0.0).then(|| [origin[0] + t * dir[0], origin[1] + t * dir[1], origin[2] + t * dir[2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descent_finds_the_true_nearest_cell() {
        let grid = GeodesicGrid::new(5);
        let loc = CellLocator::new(&grid);
        let mut x = 0x1234_5678_u64;
        for _ in 0..2000 {
            let mut r = || {
                x = evo_core::rng::splitmix64(x);
                (x >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
            };
            let d = [r(), r(), r()];
            let got = loc.locate(&grid, d);
            let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let brute = (0..grid.len())
                .max_by(|&a, &b| {
                    let s = |c: usize| (grid.centers[c][0] * d[0] + grid.centers[c][1] * d[1] + grid.centers[c][2] * d[2]) / n;
                    s(a).total_cmp(&s(b))
                })
                .unwrap();
            assert_eq!(got, brute);
        }
    }

    #[test]
    fn ray_hits_the_near_side() {
        let p = ray_sphere([0.0, 0.0, 3.0], [0.0, 0.0, -1.0], 1.0).unwrap();
        assert!((p[2] - 1.0).abs() < 1e-12);
        assert!(ray_sphere([0.0, 2.0, 3.0], [0.0, 0.0, -1.0], 1.0).is_none());
    }
}
