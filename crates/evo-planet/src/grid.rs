//! Grille géodésique : icosaèdre subdivisé, dont chaque sommet est le centre
//! d'une cellule hexagonale (12 pentagones). Le niveau n donne 10·4ⁿ + 2
//! cellules : 40 962 au niveau 6 (environ 110 km sur Terre), 163 842 au
//! niveau 7 (environ 55 km).

use evo_core::math::Det;
use std::collections::HashMap;

pub type Vec3 = [f64; 3];

#[inline]
fn normalize(v: Vec3) -> Vec3 {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

#[inline]
fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Aire d'un triangle sphérique sur la sphère unité (formule de Van Oosterom).
fn spherical_triangle_area(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    let num = dot(a, cross(b, c)).abs();
    let den = 1.0 + dot(a, b) + dot(b, c) + dot(c, a);
    2.0 * num.datan2(den)
}

/// Nombre maximal de voisins d'une cellule.
pub const MAX_NEIGHBOURS: usize = 6;

#[derive(Clone, Debug)]
pub struct GeodesicGrid {
    pub level: u32,
    /// Centres des cellules, sur la sphère unité.
    pub centers: Vec<Vec3>,
    /// Voisins de chaque cellule ; `u32::MAX` marque une case vide (pentagones).
    pub neighbours: Vec<[u32; MAX_NEIGHBOURS]>,
    /// Aire de chaque cellule sur la sphère unité (somme = 4π).
    pub unit_areas: Vec<f64>,
}

impl GeodesicGrid {
    /// Nombre de cellules attendu pour un niveau de subdivision.
    pub fn cell_count_for_level(level: u32) -> usize {
        10 * 4usize.pow(level) + 2
    }

    pub fn new(level: u32) -> Self {
        let t = (1.0 + 5f64.sqrt()) / 2.0;
        let mut verts: Vec<Vec3> = [
            [-1.0, t, 0.0],
            [1.0, t, 0.0],
            [-1.0, -t, 0.0],
            [1.0, -t, 0.0],
            [0.0, -1.0, t],
            [0.0, 1.0, t],
            [0.0, -1.0, -t],
            [0.0, 1.0, -t],
            [t, 0.0, -1.0],
            [t, 0.0, 1.0],
            [-t, 0.0, -1.0],
            [-t, 0.0, 1.0],
        ]
        .into_iter()
        .map(normalize)
        .collect();
        let mut faces: Vec<[u32; 3]> = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        for _ in 0..level {
            let mut cache: HashMap<(u32, u32), u32> = HashMap::with_capacity(faces.len() * 3 / 2);
            let mut midpoint = |a: u32, b: u32, verts: &mut Vec<Vec3>| -> u32 {
                let key = if a < b { (a, b) } else { (b, a) };
                *cache.entry(key).or_insert_with(|| {
                    let (va, vb) = (verts[a as usize], verts[b as usize]);
                    verts.push(normalize([va[0] + vb[0], va[1] + vb[1], va[2] + vb[2]]));
                    (verts.len() - 1) as u32
                })
            };
            let mut next = Vec::with_capacity(faces.len() * 4);
            for &[a, b, c] in &faces {
                let ab = midpoint(a, b, &mut verts);
                let bc = midpoint(b, c, &mut verts);
                let ca = midpoint(c, a, &mut verts);
                next.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
            }
            faces = next;
        }

        let n = verts.len();
        let mut neighbours = vec![[u32::MAX; MAX_NEIGHBOURS]; n];
        let mut unit_areas = vec![0.0; n];
        let add = |nb: &mut Vec<[u32; MAX_NEIGHBOURS]>, a: u32, b: u32| {
            let row = &mut nb[a as usize];
            if !row.contains(&b) {
                let slot = row.iter().position(|&x| x == u32::MAX).expect("plus de 6 voisins");
                row[slot] = b;
            }
        };
        for &[a, b, c] in &faces {
            add(&mut neighbours, a, b);
            add(&mut neighbours, a, c);
            add(&mut neighbours, b, a);
            add(&mut neighbours, b, c);
            add(&mut neighbours, c, a);
            add(&mut neighbours, c, b);
            // Cellule duale barycentrique : chaque sommet reçoit un tiers du triangle.
            let area = spherical_triangle_area(verts[a as usize], verts[b as usize], verts[c as usize]) / 3.0;
            unit_areas[a as usize] += area;
            unit_areas[b as usize] += area;
            unit_areas[c as usize] += area;
        }

        Self { level, centers: verts, neighbours, unit_areas }
    }

    pub fn len(&self) -> usize {
        self.centers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.centers.is_empty()
    }

    /// Voisins réels d'une cellule.
    pub fn neighbours_of(&self, cell: usize) -> impl Iterator<Item = usize> + '_ {
        self.neighbours[cell].iter().filter(|&&x| x != u32::MAX).map(|&x| x as usize)
    }

    /// Latitude du centre, rad.
    pub fn latitude(&self, cell: usize) -> f64 {
        self.centers[cell][2].clamp(-1.0, 1.0).dasin()
    }

    /// Distance angulaire moyenne entre centres voisins, rad.
    pub fn mean_spacing(&self) -> f64 {
        let (mut sum, mut count) = (0.0, 0usize);
        for c in 0..self.len() {
            for nb in self.neighbours_of(c) {
                sum += dot(self.centers[c], self.centers[nb]).clamp(-1.0, 1.0).dacos();
                count += 1;
            }
        }
        sum / count as f64
    }

    /// Mémoire occupée par la grille, en octets.
    pub fn memory_bytes(&self) -> usize {
        self.centers.capacity() * std::mem::size_of::<Vec3>()
            + self.neighbours.capacity() * std::mem::size_of::<[u32; MAX_NEIGHBOURS]>()
            + self.unit_areas.capacity() * std::mem::size_of::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_counts_match_formula() {
        for level in 0..=5 {
            assert_eq!(GeodesicGrid::new(level).len(), GeodesicGrid::cell_count_for_level(level));
        }
    }

    #[test]
    fn twelve_pentagons_rest_hexagons_and_symmetric_links() {
        let g = GeodesicGrid::new(4);
        let mut pentagons = 0;
        for c in 0..g.len() {
            let k = g.neighbours_of(c).count();
            assert!(k == 5 || k == 6);
            if k == 5 {
                pentagons += 1;
            }
            for nb in g.neighbours_of(c) {
                assert!(g.neighbours_of(nb).any(|x| x == c), "lien non symétrique");
            }
        }
        assert_eq!(pentagons, 12);
    }

    #[test]
    fn areas_cover_sphere_and_are_nearly_equal() {
        let g = GeodesicGrid::new(5);
        let total: f64 = g.unit_areas.iter().sum();
        assert!((total - 4.0 * std::f64::consts::PI).abs() < 1e-9);
        let max = g.unit_areas.iter().cloned().fold(f64::MIN, f64::max);
        let min = g.unit_areas.iter().cloned().fold(f64::MAX, f64::min);
        assert!(max / min < 1.6, "rapport des aires {}", max / min);
    }

    #[test]
    fn level_six_spacing_is_about_110_km_on_earth() {
        let g = GeodesicGrid::new(6);
        assert_eq!(g.len(), 40_962);
        let km = g.mean_spacing() * 6371.0;
        assert!((100.0..130.0).contains(&km), "espacement {km} km");
    }
}
