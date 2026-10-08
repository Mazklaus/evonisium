//! Géométrie de la grille, fixe pendant toute la partie : le client la lit
//! une fois pour construire son maillage.

use evo_planet::{BioGrid, GeodesicGrid};

/// Grille physique (cellules de l'état publié) et rattachement à la grille
/// du vivant.
#[derive(Clone, Debug, PartialEq)]
pub struct GridGeometry {
    /// Niveau de la grille physique (6 : 40 962 cellules).
    pub level: u32,
    /// Rayon de la planète, m.
    pub radius_m: f64,
    /// Centres des cellules sur la sphère unité (x, y, z ; z vers le pôle nord).
    pub centers: Vec<[f32; 3]>,
    /// Voisins de chaque cellule, `u32::MAX` pour une case vide (les 12
    /// pentagones n'ont que 5 voisins).
    pub neighbours: Vec<[u32; 6]>,
    /// Aire de chaque cellule, m².
    pub areas_m2: Vec<f32>,
    /// Triangles de l'icosaèdre subdivisé (indices de cellules), pour un
    /// maillage dont les sommets sont les centres des cellules.
    pub triangles: Vec<[u32; 3]>,
    /// Niveau de la grille du vivant et cellule du vivant de chaque cellule.
    pub bio_level: u32,
    pub bio_parent: Vec<u32>,
}

impl GridGeometry {
    pub fn new(grid: &GeodesicGrid, radius_m: f64, bio: &BioGrid) -> Self {
        let r2 = radius_m * radius_m;
        let mut triangles = Vec::with_capacity(2 * grid.len());
        // Chaque triangle est vu depuis son plus petit sommet, une seule fois.
        for a in 0..grid.len() {
            let nb: Vec<usize> = grid.neighbours_of(a).collect();
            for &b in &nb {
                for &c in &nb {
                    if b < c && a < b && grid.neighbours_of(b).any(|x| x == c) {
                        // Orientation vers l'extérieur.
                        let (pa, pb, pc) = (grid.centers[a], grid.centers[b], grid.centers[c]);
                        let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
                        let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
                        let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                        let out = n[0] * pa[0] + n[1] * pa[1] + n[2] * pa[2];
                        triangles.push(if out >= 0.0 { [a as u32, b as u32, c as u32] } else { [a as u32, c as u32, b as u32] });
                    }
                }
            }
        }
        Self {
            level: grid.level,
            radius_m,
            centers: grid.centers.iter().map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]).collect(),
            neighbours: grid.neighbours.clone(),
            areas_m2: grid.unit_areas.iter().map(|a| (a * r2) as f32).collect(),
            triangles,
            bio_level: bio.grid.level,
            bio_parent: bio.parent.clone(),
        }
    }

    pub fn len(&self) -> usize {
        self.centers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.centers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangles_cover_the_sphere_once() {
        let grid = GeodesicGrid::new(3);
        let bio = BioGrid::new(&grid, 2);
        let g = GridGeometry::new(&grid, 6.4e6, &bio);
        // Icosaèdre subdivisé : 20·4ⁿ faces.
        assert_eq!(g.triangles.len(), 20 * 4usize.pow(3));
        let area: f32 = g.areas_m2.iter().sum();
        let sphere = 4.0 * std::f64::consts::PI * 6.4e6f64 * 6.4e6;
        assert!(((area as f64) - sphere).abs() / sphere < 1e-4);
    }
}
