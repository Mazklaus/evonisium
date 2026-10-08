//! Maillages du globe, construits une fois depuis la grille de simulation
//! (document « Globe 3D et rendu du monde », section « Géométrie »).
//!
//! - Vue lissée : les centres des cellules sont les sommets d'une icosphère ;
//!   les valeurs portées par les sommets sont interpolées sur les triangles.
//! - Vue en tuiles (« données brutes ») : le maillage dual, un hexagone ou un
//!   pentagone plat par cellule, montre la valeur exacte de chaque cellule.
//!
//! Les faces de l'icosphère sont retrouvées depuis le voisinage de la grille
//! (chaque triangle relie une cellule à deux voisins consécutifs) : le client
//! n'a besoin de rien d'autre que `GeodesicGrid`.

use evo_planet::grid::{GeodesicGrid, Vec3};

#[inline]
fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

#[inline]
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn normalize(v: Vec3) -> Vec3 {
    let n = dot(v, v).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

fn to_f32(v: Vec3) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

/// Voisins d'une cellule triés dans le sens trigonométrique vu de
/// l'extérieur de la sphère.
pub fn ordered_neighbours(grid: &GeodesicGrid, cell: usize) -> Vec<usize> {
    let c = grid.centers[cell];
    // Repère tangent : e1 vers le premier voisin, e2 = c × e1.
    let mut ns: Vec<usize> = grid.neighbours_of(cell).collect();
    if ns.is_empty() {
        return ns;
    }
    let first = sub(grid.centers[ns[0]], c);
    let e1 = normalize(sub(first, [c[0] * dot(first, c), c[1] * dot(first, c), c[2] * dot(first, c)]));
    let e2 = cross(c, e1);
    ns.sort_by(|&a, &b| {
        let angle = |n: usize| {
            let d = sub(grid.centers[n], c);
            dot(d, e2).atan2(dot(d, e1))
        };
        angle(a).total_cmp(&angle(b))
    });
    ns
}

/// Triangles de l'icosphère, orientés vers l'extérieur ; 20·4ⁿ au niveau n.
pub fn icosphere_triangles(grid: &GeodesicGrid) -> Vec<[u32; 3]> {
    let mut tris = Vec::with_capacity(20 * 4usize.pow(grid.level));
    for i in 0..grid.len() {
        let ns = ordered_neighbours(grid, i);
        for k in 0..ns.len() {
            let (a, b) = (ns[k], ns[(k + 1) % ns.len()]);
            // Chaque triangle est vu depuis ses trois sommets : on ne le garde
            // que depuis le plus petit indice.
            if i < a && i < b {
                tris.push([i as u32, a as u32, b as u32]);
            }
        }
    }
    tris
}

/// Maillage prêt à envoyer à la carte graphique : positions sur la sphère
/// unité, cellule portée par chaque sommet, triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlobeMesh {
    pub positions: Vec<[f32; 3]>,
    pub cells: Vec<u32>,
    pub indices: Vec<u32>,
}

impl GlobeMesh {
    /// Vue lissée : un sommet par cellule.
    pub fn smooth(grid: &GeodesicGrid) -> Self {
        let positions = grid.centers.iter().map(|&c| to_f32(c)).collect();
        let cells = (0..grid.len() as u32).collect();
        let indices = icosphere_triangles(grid).into_iter().flatten().collect();
        Self { positions, cells, indices }
    }

    /// Vue en tuiles : chaque cellule est un polygone plat dont les coins sont
    /// les centres des triangles qui l'entourent.
    pub fn tiles(grid: &GeodesicGrid) -> Self {
        let mut m = Self::default();
        for i in 0..grid.len() {
            let ns = ordered_neighbours(grid, i);
            let c = grid.centers[i];
            let centre = m.positions.len() as u32;
            m.positions.push(to_f32(c));
            m.cells.push(i as u32);
            for k in 0..ns.len() {
                let (a, b) = (grid.centers[ns[k]], grid.centers[ns[(k + 1) % ns.len()]]);
                let corner = normalize([c[0] + a[0] + b[0], c[1] + a[1] + b[1], c[2] + a[2] + b[2]]);
                m.positions.push(to_f32(corner));
                m.cells.push(i as u32);
            }
            let n = ns.len() as u32;
            for k in 0..n {
                m.indices.extend_from_slice(&[centre, centre + 1 + k, centre + 1 + (k + 1) % n]);
            }
        }
        m
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icosphere_has_the_expected_faces_all_facing_out() {
        for level in 0..=4 {
            let grid = GeodesicGrid::new(level);
            let tris = icosphere_triangles(&grid);
            assert_eq!(tris.len(), 20 * 4usize.pow(level), "niveau {level}");
            for t in &tris {
                let [a, b, c] = t.map(|i| grid.centers[i as usize]);
                let n = cross(sub(b, a), sub(c, a));
                assert!(dot(n, a) > 0.0, "triangle retourné au niveau {level}");
            }
            // Euler : V − E + F = 2 sur la sphère.
            let v = grid.len() as i64;
            let f = tris.len() as i64;
            assert_eq!(v - 3 * f / 2 + f, 2);
        }
    }

    #[test]
    fn tiles_cover_every_cell_with_twelve_pentagons() {
        let grid = GeodesicGrid::new(3);
        let m = GlobeMesh::tiles(&grid);
        let mut per_cell = vec![0usize; grid.len()];
        for &c in &m.cells {
            per_cell[c as usize] += 1;
        }
        assert_eq!(per_cell.iter().filter(|&&k| k == 6).count(), 12);
        assert!(per_cell.iter().all(|&k| k == 6 || k == 7));
        // Les tuiles pavent la sphère : 6 ou 5 triangles par cellule.
        assert_eq!(m.triangle_count(), 6 * grid.len() - 12);
        let smooth = GlobeMesh::smooth(&grid);
        assert_eq!(smooth.triangle_count(), 20 * 64);
    }
}
