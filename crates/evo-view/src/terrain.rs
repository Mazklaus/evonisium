//! Régions et paysages, incrément G3 du globe (document Globe 3D, bandes de
//! zoom Z3 et Z4) : sous la résolution de la simulation, chaque triangle de
//! l'icosphère autour du point regardé est subdivisé en quatre, récursivement
//! (arbre quaternaire), et reçoit un relief fractal.
//!
//! - L'altitude de base est une moyenne lissée de celles des cellules
//!   voisines (noyau gaussien), sans les facettes d'une interpolation
//!   linéaire par triangle.
//! - Le détail est un bruit fractal (somme d'octaves d'un bruit de valeur 3D)
//!   dont l'amplitude est la rugosité de la cellule ; sa moyenne sur chaque
//!   cellule est retirée : le détail n'élève ni n'abaisse une cellule, son
//!   altitude moyenne reste à peu près celle que la simulation connaît (au
//!   lissage de la base près).
//! - Le bruit est haché depuis la graine de la partie et la position : la
//!   même région se redessine toujours à l'identique, quel que soit le point
//!   d'où on y arrive.
//!
//! [Simplification] La rugosité n'est pas lue d'une roche et d'un âge
//! simulés (le moteur ne publie ni l'un ni l'autre par cellule) : elle vient
//! du contraste d'altitude avec les voisines (reliefs jeunes et actifs), de
//! l'altitude des terres, et la glace l'adoucit (érosion glaciaire).

use crate::layers::texel_uv;
use crate::mesh::icosphere_triangles;
use evo_morph::canvas::mix64;
use evo_planet::grid::GeodesicGrid;
use std::collections::HashMap;

/// Relief d'une cellule tel que la région le lit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellRelief {
    pub elevation_m: f32,
    /// Amplitude du détail fractal, m.
    pub roughness_m: f32,
}

/// Rugosité d'une cellule : voir l'en-tête du module.
pub fn roughness(elevation_m: f32, neighbour_contrast_m: f32, ice_cover: f32, is_ocean: bool) -> f32 {
    let base = if is_ocean { 60.0 } else { 150.0 };
    let r = base + 0.3 * neighbour_contrast_m + 0.12 * elevation_m.max(0.0);
    r.min(2500.0) * (1.0 - 0.5 * ice_cover.clamp(0.0, 1.0))
}

/// Relief de toutes les cellules d'une grille, depuis l'altitude, la glace et
/// la mer de chaque cellule.
pub fn reliefs(grid: &GeodesicGrid, elevation: &[f32], ice: &[f32], ocean: &[bool]) -> Vec<CellRelief> {
    (0..grid.len())
        .map(|i| {
            let e = elevation[i];
            let contrast = grid.neighbours_of(i).map(|n| (elevation[n] - e).abs()).fold(0.0f32, f32::max);
            CellRelief { elevation_m: e, roughness_m: roughness(e, contrast, ice[i], ocean[i]) }
        })
        .collect()
}

/// Maillage d'une région : directions sur la sphère unité, altitude fine de
/// chaque sommet, cellule la plus proche (données du globe), triangles
/// tournés vers l'extérieur.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Region {
    pub positions: Vec<[f32; 3]>,
    pub heights: Vec<f32>,
    pub cells: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    /// Texels des deux cellules suivantes (b puis c) et poids des trois
    /// cellules (la plus proche d'abord) : le shader mêle leurs données au
    /// lieu de montrer les bords des cellules en escalier.
    pub uvs_bc: Vec<[f32; 4]>,
    pub weights: Vec<[f32; 3]>,
    /// Gradient de l'altitude sur la sphère unité (m par unité de longueur,
    /// vecteur tangent) : le shader en tire des normales lissées, sans les
    /// facettes des triangles.
    pub gradients: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub centre: usize,
    /// Rayon angulaire du disque entièrement couvert, en radians.
    pub covered_radius: f32,
}

/// Triangles de l'icosphère d'une grille, à garder d'une région à l'autre.
pub struct Triangles {
    pub tris: Vec<[u32; 3]>,
    /// Triangles touchant chaque cellule.
    pub by_cell: Vec<Vec<u32>>,
}

impl Triangles {
    pub fn new(grid: &GeodesicGrid) -> Self {
        let tris = icosphere_triangles(grid);
        let mut by_cell = vec![Vec::new(); grid.len()];
        for (i, t) in tris.iter().enumerate() {
            for &c in t {
                by_cell[c as usize].push(i as u32);
            }
        }
        Self { tris, by_cell }
    }
}

fn lattice(seed: u64, x: i64, y: i64, z: i64) -> f32 {
    let h = mix64(seed ^ mix64(x as u64 ^ mix64(y as u64 ^ mix64(z as u64))));
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Bruit de valeur 3D lissé, dans [0, 1].
fn value_noise3(seed: u64, p: [f64; 3]) -> f32 {
    let i = [p[0].floor(), p[1].floor(), p[2].floor()];
    let f = [(p[0] - i[0]) as f32, (p[1] - i[1]) as f32, (p[2] - i[2]) as f32];
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy, sz) = (s(f[0]), s(f[1]), s(f[2]));
    let (x, y, z) = (i[0] as i64, i[1] as i64, i[2] as i64);
    let l = |dx: i64, dy: i64, dz: i64| lattice(seed, x + dx, y + dy, z + dz);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(l(0, 0, 0), l(1, 0, 0), sx);
    let x10 = lerp(l(0, 1, 0), l(1, 1, 0), sx);
    let x01 = lerp(l(0, 0, 1), l(1, 0, 1), sx);
    let x11 = lerp(l(0, 1, 1), l(1, 1, 1), sx);
    lerp(lerp(x00, x10, sy), lerp(x01, x11, sy), sz)
}

/// Bruit fractal centré (moyenne nulle), amplitude totale ~1.
fn fractal(seed: u64, dir: [f64; 3], base_freq: f64, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut freq = base_freq;
    for o in 0..octaves {
        // Crêtes : un relief plus anguleux qu'un bruit lisse, comme des
        // versants entaillés.
        let n = value_noise3(seed ^ (o as u64) << 56, [dir[0] * freq, dir[1] * freq, dir[2] * freq]);
        let ridged = 1.0 - (2.0 * n - 1.0).abs();
        sum += amp * (ridged - 0.5);
        norm += amp * 0.5;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm.max(1e-6)
}

/// Sommet en construction : direction et poids des cellules qui
/// l'interpolent.
#[derive(Clone)]
struct Vert {
    dir: [f64; 3],
    weights: Vec<(u32, f32)>,
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

fn merge(a: &[(u32, f32)], b: &[(u32, f32)]) -> Vec<(u32, f32)> {
    let mut out: Vec<(u32, f32)> = a.iter().map(|&(c, w)| (c, w * 0.5)).collect();
    for &(c, w) in b {
        match out.iter_mut().find(|e| e.0 == c) {
            Some(e) => e.1 += w * 0.5,
            None => out.push((c, w * 0.5)),
        }
    }
    out
}

/// Altitude et rugosité de base d'un sommet : moyenne des cellules proches
/// (celles qui l'interpolent et leurs voisines), pondérée par un noyau
/// gaussien de largeur ~0,6 espacement de cellule.
fn smooth_base(grid: &GeodesicGrid, relief: &[CellRelief], v: &Vert, spacing: f64) -> (f32, f32) {
    let mut cells: Vec<u32> = Vec::with_capacity(24);
    for &(c, _) in &v.weights {
        cells.push(c);
        cells.extend(grid.neighbours_of(c as usize).map(|n| n as u32));
    }
    cells.sort_unstable();
    cells.dedup();
    let sigma2 = (0.6 * spacing) * (0.6 * spacing);
    let (mut h, mut r, mut wsum) = (0.0f64, 0.0f64, 0.0f64);
    for c in cells {
        let p = grid.centers[c as usize];
        let d = [p[0] - v.dir[0], p[1] - v.dir[1], p[2] - v.dir[2]];
        let w = (-(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) / sigma2).exp();
        h += w * relief[c as usize].elevation_m as f64;
        r += w * relief[c as usize].roughness_m as f64;
        wsum += w;
    }
    if wsum <= 0.0 {
        return (0.0, 0.0);
    }
    ((h / wsum) as f32, (r / wsum) as f32)
}

/// Construit la région de `rings` anneaux de cellules autour de `centre`,
/// subdivisée `depth` fois.
pub fn region(grid: &GeodesicGrid, tris: &Triangles, relief: &[CellRelief], centre: usize, rings: usize, depth: u32, seed: u64) -> Region {
    // Cellules des anneaux, par parcours en largeur.
    let mut ring_of: HashMap<u32, usize> = HashMap::new();
    ring_of.insert(centre as u32, 0);
    let mut frontier = vec![centre];
    for r in 1..=rings {
        let mut next = Vec::new();
        for &c in &frontier {
            for n in grid.neighbours_of(c) {
                if let std::collections::hash_map::Entry::Vacant(e) = ring_of.entry(n as u32) {
                    e.insert(r);
                    next.push(n);
                }
            }
        }
        frontier = next;
    }
    let mut chosen: Vec<u32> = Vec::new();
    for &c in ring_of.keys() {
        for &t in &tris.by_cell[c as usize] {
            let tri = tris.tris[t as usize];
            if tri.iter().all(|v| ring_of.contains_key(v)) {
                chosen.push(t);
            }
        }
    }
    chosen.sort_unstable();
    chosen.dedup();

    // Subdivision avec sommets partagés (pas de fissures entre triangles).
    let mut verts: Vec<Vert> = Vec::new();
    let mut corner: HashMap<u32, u32> = HashMap::new();
    let mut corner_of = |c: u32, verts: &mut Vec<Vert>| -> u32 {
        *corner.entry(c).or_insert_with(|| {
            verts.push(Vert { dir: grid.centers[c as usize], weights: vec![(c, 1.0)] });
            verts.len() as u32 - 1
        })
    };
    let mut faces: Vec<[u32; 3]> = chosen.iter().map(|&t| tris.tris[t as usize]).map(|t| t.map(|c| corner_of(c, &mut verts))).collect();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    for _ in 0..depth {
        let mut next = Vec::with_capacity(faces.len() * 4);
        for f in &faces {
            let mut mid = |a: u32, b: u32, verts: &mut Vec<Vert>| -> u32 {
                let key = (a.min(b), a.max(b));
                *mids.entry(key).or_insert_with(|| {
                    let (va, vb) = (&verts[a as usize], &verts[b as usize]);
                    let dir = normalize([va.dir[0] + vb.dir[0], va.dir[1] + vb.dir[1], va.dir[2] + vb.dir[2]]);
                    let weights = merge(&va.weights, &vb.weights);
                    verts.push(Vert { dir, weights });
                    verts.len() as u32 - 1
                })
            };
            let ab = mid(f[0], f[1], &mut verts);
            let bc = mid(f[1], f[2], &mut verts);
            let ca = mid(f[2], f[0], &mut verts);
            next.push([f[0], ab, ca]);
            next.push([ab, f[1], bc]);
            next.push([ca, bc, f[2]]);
            next.push([ab, bc, ca]);
        }
        faces = next;
    }

    // Relief : base interpolée et détail fractal, centré par cellule.
    let spacing = (4.0 * std::f64::consts::PI / grid.len() as f64).sqrt();
    let base_freq = 1.0 / spacing;
    let octaves = depth + 3;
    let mut out = Region { centre, ..Default::default() };
    let mut detail = Vec::with_capacity(verts.len());
    let mut cell_sum: HashMap<u32, (f32, f32)> = HashMap::new();
    for v in &verts {
        // Base lissée : un noyau gaussien sur les cellules voisines plutôt
        // que l'interpolation linéaire par triangle, dont les facettes se
        // verraient à l'estompage.
        let (base, rough) = smooth_base(grid, relief, v, spacing);
        let mut ranked = v.weights.clone();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let nearest = ranked.first().map(|e| e.0).unwrap_or(centre as u32);
        let pick = |k: usize| ranked.get(k).copied().unwrap_or((nearest, 0.0));
        let (b, c) = (pick(1), pick(2));
        let total = (pick(0).1 + b.1 + c.1).max(1e-6);
        let (ub, uc) = (texel_uv(b.0 as usize, grid.len()), texel_uv(c.0 as usize, grid.len()));
        out.uvs_bc.push([ub[0], ub[1], uc[0], uc[1]]);
        out.weights.push([pick(0).1 / total, b.1 / total, c.1 / total]);
        let n = rough * fractal(seed, v.dir, base_freq, octaves);
        let e = cell_sum.entry(nearest).or_insert((0.0, 0.0));
        e.0 += n;
        e.1 += 1.0;
        detail.push(n);
        out.positions.push([v.dir[0] as f32, v.dir[1] as f32, v.dir[2] as f32]);
        out.heights.push(base);
        out.cells.push(nearest);
        out.uvs.push(texel_uv(nearest as usize, grid.len()));
    }
    for i in 0..verts.len() {
        let (s, n) = cell_sum[&out.cells[i]];
        out.heights[i] += detail[i] - s / n;
    }
    // Triangles orientés vers l'extérieur.
    for f in faces {
        let p = |i: u32| verts[i as usize].dir;
        let (a, b, c) = (p(f[0]), p(f[1]), p(f[2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let nrm = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        if nrm[0] * a[0] + nrm[1] * a[1] + nrm[2] * a[2] >= 0.0 {
            out.indices.extend_from_slice(&f);
        } else {
            out.indices.extend_from_slice(&[f[0], f[2], f[1]]);
        }
    }
    // Gradients aux sommets : moyenne, pondérée par l'aire, des gradients
    // des faces qui les touchent.
    let mut grad = vec![[0.0f64; 3]; out.positions.len()];
    let mut area = vec![0.0f64; out.positions.len()];
    for t in out.indices.chunks(3) {
        let p = |i: u32| verts[i as usize].dir;
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let d = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
        let (a11, a12, a22) = (d(e1, e1), d(e1, e2), d(e2, e2));
        let det = a11 * a22 - a12 * a12;
        if det.abs() < 1e-30 {
            continue;
        }
        let h = |i: u32| out.heights[i as usize] as f64;
        let (dh1, dh2) = (h(t[1]) - h(t[0]), h(t[2]) - h(t[0]));
        let alpha = (dh1 * a22 - dh2 * a12) / det;
        let beta = (dh2 * a11 - dh1 * a12) / det;
        let g = [alpha * e1[0] + beta * e2[0], alpha * e1[1] + beta * e2[1], alpha * e1[2] + beta * e2[2]];
        let w = det.sqrt() / 2.0;
        for &i in t {
            for k in 0..3 {
                grad[i as usize][k] += g[k] * w;
            }
            area[i as usize] += w;
        }
    }
    out.gradients = grad
        .iter()
        .zip(&area)
        .map(|(g, a)| if *a > 0.0 { [(g[0] / a) as f32, (g[1] / a) as f32, (g[2] / a) as f32] } else { [0.0; 3] })
        .collect();
    // Disque couvert : jusqu'à l'avant-dernier anneau.
    out.covered_radius = (rings.saturating_sub(1) as f64 * spacing * 0.9) as f32;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(level: u32) -> (GeodesicGrid, Vec<CellRelief>) {
        let grid = GeodesicGrid::new(level);
        let n = grid.len();
        let elevation: Vec<f32> = (0..n).map(|i| ((grid.centers[i][0] * 5.0).sin() * 3000.0) as f32).collect();
        let ocean: Vec<bool> = elevation.iter().map(|e| *e < 0.0).collect();
        let r = reliefs(&grid, &elevation, &vec![0.0; n], &ocean);
        (grid, r)
    }

    #[test]
    fn region_keeps_cell_means_and_is_reproducible() {
        let (grid, relief) = world(4);
        let tris = Triangles::new(&grid);
        let a = region(&grid, &tris, &relief, 100, 4, 3, 7);
        assert!(a.indices.len() / 3 > 1000);
        assert_eq!(a.positions.len(), a.heights.len());
        // Reproductible, et indépendant du chemin par lequel on y arrive.
        let b = region(&grid, &tris, &relief, 100, 4, 3, 7);
        assert_eq!(a, b);
        let c = region(&grid, &tris, &relief, 100, 4, 3, 8);
        assert_ne!(a.heights, c.heights);
        // Moyenne du détail nulle sur chaque cellule : la moyenne des
        // altitudes reste celle de la base interpolée.
        let mut detail: HashMap<u32, (f64, f64)> = HashMap::new();
        let base = region(&grid, &tris, &relief.iter().map(|r| CellRelief { roughness_m: 0.0, ..*r }).collect::<Vec<_>>(), 100, 4, 3, 7);
        for i in 0..a.heights.len() {
            let e = detail.entry(a.cells[i]).or_insert((0.0, 0.0));
            e.0 += (a.heights[i] - base.heights[i]) as f64;
            e.1 += 1.0;
        }
        for (_, (s, n)) in detail {
            assert!((s / n).abs() < 1.0, "{}", s / n);
        }
        // Le détail existe bien.
        let spread = a.heights.iter().zip(&base.heights).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(spread > 50.0, "{spread}");
    }

    #[test]
    fn shared_edges_have_no_cracks_and_faces_point_out() {
        let (grid, relief) = world(3);
        let tris = Triangles::new(&grid);
        let r = region(&grid, &tris, &relief, 5, 3, 2, 1);
        // Chaque arête intérieure est partagée par exactement deux triangles.
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in r.indices.chunks(3) {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|&n| n <= 2));
        let inner = edges.values().filter(|&&n| n == 2).count();
        assert!(inner > edges.len() / 2);
        assert!(r.covered_radius > 0.0);
        assert_eq!(r.gradients.len(), r.positions.len());
        assert!(r.gradients.iter().flatten().all(|g| g.is_finite()));
        assert!(r.gradients.iter().any(|g| g[0] * g[0] + g[1] * g[1] + g[2] * g[2] > 0.0));
    }
}
