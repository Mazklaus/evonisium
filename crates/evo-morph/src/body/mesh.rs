//! Surface : le champ de distance du corps converti en maillage par surface
//! nets (Gibson, 1998) à trois résolutions, avec normales lissées (gradient
//! du champ, pour des contours d'encre propres), couleurs des pigments, masque
//! du motif de Turing et poids de peau par la primitive la plus proche.

use super::pattern::PatternTexture;
use super::plan::BodyPlan;
use super::shape::{field, place, Bone, Primitive, Shape, V3};

/// Résolutions des trois niveaux de détail : cellules le long du plus grand
/// côté du corps.
pub const LOD_RESOLUTION: [usize; 3] = [64, 36, 18];

/// Un maillage indexé. Les triangles tournent dans le sens direct vu de
/// l'extérieur (normale sortante par la règle de la main droite).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Couleur, et masque du motif dans la composante alpha.
    pub colours: Vec<[f32; 4]>,
    /// Deux os les plus proches et leurs poids (somme 1).
    pub bones: Vec<[u16; 2]>,
    pub weights: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }
}

/// Le corps prêt à afficher.
#[derive(Clone, Debug, Default)]
pub struct Body {
    /// Peau, du plus fin au plus grossier.
    pub lods: Vec<Mesh>,
    /// Organes internes (vue anatomie), colorés par appareil.
    pub organs: Mesh,
    pub bones: Vec<Bone>,
    /// Plus grande dimension, en mètres.
    pub extent_m: f32,
    /// Brillance du revêtement dominant, 0 à 1.
    pub gloss: f32,
    pub iridescence: f32,
}

/// Construit le corps d'un plan. `lods` limite le nombre de niveaux.
pub fn build(plan: &BodyPlan, lods: usize) -> Body {
    let shape = place(plan);
    let (scale, unit) = unit_scale(&shape.primitives);
    let skin: Vec<Primitive> = unit.iter().filter(|p| !p.internal).copied().collect();
    let organs: Vec<Primitive> = unit.iter().filter(|p| p.internal).copied().collect();
    let pattern = PatternTexture::grow(&plan.pattern, plan.seed);
    let (lo, hi) = bounds(&skin);
    let extent = (hi.0 - lo.0).max(hi.1 - lo.1).max(hi.2 - lo.2) / scale;
    let frame = Mapping { lo, hi };
    let mut out = Body {
        lods: Vec::new(),
        organs: Mesh::default(),
        bones: shape.bones.clone(),
        extent_m: extent,
        gloss: plan.modules.first().map(|m| m.covering.gloss()).unwrap_or(0.3),
        iridescence: plan.modules.iter().map(|m| m.iridescence).fold(0.0, f32::max),
    };
    for &res in LOD_RESOLUTION.iter().take(lods.clamp(1, 3)) {
        out.lods.push(surface_nets(&skin, res, Some((&pattern, &frame, plan.pattern.contrast))));
    }
    if !organs.is_empty() {
        out.organs = surface_nets_each(&organs, LOD_RESOLUTION[1]);
    }
    // Retour aux mètres.
    for m in out.lods.iter_mut().chain(std::iter::once(&mut out.organs)) {
        for p in m.positions.iter_mut() {
            for v in p.iter_mut() {
                *v /= scale;
            }
        }
    }
    out
}

/// Primitives ramenées à un corps de taille unité, et le facteur appliqué.
fn unit_scale(prims: &[Primitive]) -> (f32, Vec<Primitive>) {
    let (lo, hi) = bounds(prims);
    let extent = (hi.0 - lo.0).max(hi.1 - lo.1).max(hi.2 - lo.2);
    let k = if extent > 0.0 && extent.is_finite() { 1.0 / extent } else { 1.0 };
    (k, prims.iter().map(|p| p.scaled(k)).collect())
}

/// Repère du motif : coordonnées cylindriques autour de l'axe du corps.
struct Mapping {
    lo: V3,
    hi: V3,
}

impl Mapping {
    fn uv(&self, p: V3) -> (f32, f32) {
        let u = (p.0 - self.lo.0) / (self.hi.0 - self.lo.0).max(1e-9);
        let cy = (self.lo.1 + self.hi.1) / 2.0;
        let cz = (self.lo.2 + self.hi.2) / 2.0;
        let v = (p.2 - cz).atan2(p.1 - cy) / std::f32::consts::TAU + 0.5;
        (u, v)
    }
}

fn bounds(prims: &[Primitive]) -> (V3, V3) {
    let mut lo = V3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut hi = V3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in prims {
        let (a, b) = p.bounds();
        lo = lo.min(a);
        hi = hi.max(b);
    }
    if !lo.0.is_finite() {
        return (V3::default(), V3::default());
    }
    (lo, hi)
}

/// Organes : chacun maillé à part (ils ne fusionnent pas entre eux).
fn surface_nets_each(prims: &[Primitive], res: usize) -> Mesh {
    let mut out = Mesh::default();
    for p in prims {
        let m = surface_nets(std::slice::from_ref(p), res / 2, None);
        let base = out.positions.len() as u32;
        out.positions.extend(m.positions);
        out.normals.extend(m.normals);
        out.colours.extend(m.colours);
        out.bones.extend(m.bones);
        out.weights.extend(m.weights);
        out.indices.extend(m.indices.iter().map(|i| i + base));
    }
    out
}

fn surface_nets(prims: &[Primitive], res: usize, pattern: Option<(&PatternTexture, &Mapping, f32)>) -> Mesh {
    let mut mesh = Mesh::default();
    if prims.is_empty() {
        return mesh;
    }
    let (lo, hi) = bounds(prims);
    let extent = (hi.0 - lo.0).max(hi.1 - lo.1).max(hi.2 - lo.2).max(1e-12);
    let h = extent / res.max(4) as f32;
    // Deux cellules de marge autour de la boîte.
    let origin = lo.sub(V3(2.0 * h, 2.0 * h, 2.0 * h));
    let dim = |a: f32, b: f32| (((b - a) / h).ceil() as usize + 5).min(res + 8);
    let (nx, ny, nz) = (dim(lo.0, hi.0), dim(lo.1, hi.1), dim(lo.2, hi.2));
    let at = |x: usize, y: usize, z: usize| origin.add(V3(x as f32 * h, y as f32 * h, z as f32 * h));
    let idx = |x: usize, y: usize, z: usize| (z * ny + y) * nx + x;
    let mut vals = vec![0.0f32; nx * ny * nz];
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                vals[idx(x, y, z)] = field(prims, at(x, y, z)).0;
            }
        }
    }
    // Un sommet par cube traversé : moyenne des points de passage sur ses
    // arêtes.
    const CORNERS: [(usize, usize, usize); 8] = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (1, 1, 0), (0, 0, 1), (1, 0, 1), (0, 1, 1), (1, 1, 1)];
    const EDGES: [(usize, usize); 12] = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];
    let cidx = |x: usize, y: usize, z: usize| (z * (ny - 1) + y) * (nx - 1) + x;
    let mut cube_vertex = vec![u32::MAX; (nx - 1) * (ny - 1) * (nz - 1)];
    for z in 0..nz - 1 {
        for y in 0..ny - 1 {
            for x in 0..nx - 1 {
                let v: [f32; 8] = std::array::from_fn(|k| {
                    let (dx, dy, dz) = CORNERS[k];
                    vals[idx(x + dx, y + dy, z + dz)]
                });
                let inside = v.iter().filter(|d| **d < 0.0).count();
                if inside == 0 || inside == 8 {
                    continue;
                }
                let mut sum = V3::default();
                let mut n = 0.0;
                for &(a, b) in &EDGES {
                    if (v[a] < 0.0) != (v[b] < 0.0) {
                        let t = v[a] / (v[a] - v[b]);
                        let (ax, ay, az) = CORNERS[a];
                        let (bx, by, bz) = CORNERS[b];
                        let pa = V3(ax as f32, ay as f32, az as f32);
                        let pb = V3(bx as f32, by as f32, bz as f32);
                        sum = sum.add(pa.add(pb.sub(pa).mul(t)));
                        n += 1.0;
                    }
                }
                let p = at(x, y, z).add(sum.mul(h / n));
                cube_vertex[cidx(x, y, z)] = mesh.positions.len() as u32;
                push_vertex(&mut mesh, prims, p, h, pattern);
            }
        }
    }
    // Un quadrilatère par arête de la grille qui traverse la surface.
    let quad = |c: [u32; 4], mesh: &mut Mesh| {
        if c.contains(&u32::MAX) {
            return;
        }
        let p = |i: u32| {
            let q = mesh.positions[i as usize];
            V3(q[0], q[1], q[2])
        };
        let n = mesh.normals[c[0] as usize];
        let face = p(c[1]).sub(p(c[0])).cross(p(c[2]).sub(p(c[0])));
        let tri =
            if face.dot(V3(n[0], n[1], n[2])) >= 0.0 { [c[0], c[1], c[2], c[0], c[2], c[3]] } else { [c[0], c[2], c[1], c[0], c[3], c[2]] };
        mesh.indices.extend_from_slice(&tri);
    };
    for z in 0..nz - 1 {
        for y in 0..ny - 1 {
            for x in 0..nx - 1 {
                let s = vals[idx(x, y, z)] < 0.0;
                if y > 0 && z > 0 && s != (vals[idx(x + 1, y, z)] < 0.0) {
                    quad(
                        [
                            cube_vertex[cidx(x, y - 1, z - 1)],
                            cube_vertex[cidx(x, y, z - 1)],
                            cube_vertex[cidx(x, y, z)],
                            cube_vertex[cidx(x, y - 1, z)],
                        ],
                        &mut mesh,
                    );
                }
                if x > 0 && z > 0 && s != (vals[idx(x, y + 1, z)] < 0.0) {
                    quad(
                        [
                            cube_vertex[cidx(x - 1, y, z - 1)],
                            cube_vertex[cidx(x, y, z - 1)],
                            cube_vertex[cidx(x, y, z)],
                            cube_vertex[cidx(x - 1, y, z)],
                        ],
                        &mut mesh,
                    );
                }
                if x > 0 && y > 0 && s != (vals[idx(x, y, z + 1)] < 0.0) {
                    quad(
                        [
                            cube_vertex[cidx(x - 1, y - 1, z)],
                            cube_vertex[cidx(x, y - 1, z)],
                            cube_vertex[cidx(x, y, z)],
                            cube_vertex[cidx(x - 1, y, z)],
                        ],
                        &mut mesh,
                    );
                }
            }
        }
    }
    mesh
}

fn push_vertex(mesh: &mut Mesh, prims: &[Primitive], p: V3, h: f32, pattern: Option<(&PatternTexture, &Mapping, f32)>) {
    let e = h * 0.5;
    let f = |q: V3| field(prims, q).0;
    let n = V3(
        f(p.add(V3(e, 0.0, 0.0))) - f(p.sub(V3(e, 0.0, 0.0))),
        f(p.add(V3(0.0, e, 0.0))) - f(p.sub(V3(0.0, e, 0.0))),
        f(p.add(V3(0.0, 0.0, e))) - f(p.sub(V3(0.0, 0.0, e))),
    )
    .norm();
    let (_, a, b) = field(prims, p);
    let da = prims[a].distance(p).abs();
    let (bb, wb) = if b < prims.len() {
        let db = prims[b].distance(p).abs();
        // Poids de peau : la primitive la plus proche domine, la seconde
        // compte d'autant plus que les deux distances sont voisines.
        let w = ((da - db) / (2.0 * h).max(1e-9) + 0.5).clamp(0.0, 0.5);
        (prims[b].bone, w)
    } else {
        (prims[a].bone, 0.0)
    };
    let ca = prims[a].colour;
    let cb = if b < prims.len() { prims[b].colour } else { ca };
    let mut c = [ca[0] + (cb[0] - ca[0]) * wb, ca[1] + (cb[1] - ca[1]) * wb, ca[2] + (cb[2] - ca[2]) * wb, 0.0];
    if let Some((tex, map, contrast)) = pattern {
        let (u, v) = map.uv(p);
        let m = tex.sample(u, v) * contrast;
        // Le motif fonce le pigment vers une teinte d'encre.
        for (ck, ink) in c.iter_mut().zip(crate::canvas::INK) {
            *ck = *ck * (1.0 - 0.6 * m) + ink * 0.25 * m;
        }
        c[3] = m;
    }
    mesh.positions.push([p.0, p.1, p.2]);
    mesh.normals.push([n.0, n.1, n.2]);
    mesh.colours.push(c);
    mesh.bones.push([prims[a].bone, bb]);
    mesh.weights.push([1.0 - wb, wb]);
}

/// Plus grande dimension de la peau d'un plan, en mètres.
pub fn extent_m(plan: &BodyPlan) -> f32 {
    let shape = place(plan);
    let skin: Vec<Primitive> = shape.primitives.into_iter().filter(|p| !p.internal).collect();
    let (lo, hi) = bounds(&skin);
    (hi.0 - lo.0).max(hi.1 - lo.1).max(hi.2 - lo.2)
}

/// Silhouette de profil (vue de droite) : masque `width` × `height`, 1 dans
/// le corps. Renvoie aussi les mètres par pixel.
pub fn silhouette(plan: &BodyPlan, width: usize, height: usize) -> (Vec<f32>, f32) {
    let shape: Shape = place(plan);
    let (scale, unit) = unit_scale(&shape.primitives);
    let skin: Vec<Primitive> = unit.into_iter().filter(|p| !p.internal).collect();
    let mut mask = vec![0.0f32; width * height];
    if skin.is_empty() {
        return (mask, 1.0);
    }
    let (lo, hi) = bounds(&skin);
    let mpp = ((hi.0 - lo.0) / (width as f32 * 0.92)).max((hi.1 - lo.1) / (height as f32 * 0.92)).max(1e-12);
    let cx = (lo.0 + hi.0) / 2.0;
    let cy = (lo.1 + hi.1) / 2.0;
    for py in 0..height {
        for px in 0..width {
            let x = cx + (px as f32 + 0.5 - width as f32 / 2.0) * mpp;
            let y = cy - (py as f32 + 0.5 - height as f32 / 2.0) * mpp;
            // Lancer de rayon par pas de sphère le long de z : le champ
            // borne la distance à la surface.
            let mut best = f32::INFINITY;
            let mut z = lo.2 - mpp;
            while z <= hi.2 + mpp {
                let d = field(&skin, V3(x, y, z)).0;
                best = best.min(d);
                if best < 0.0 {
                    break;
                }
                z += d.max(mpp * 0.5);
            }
            // Bord adouci sur un pixel.
            mask[py * width + px] = (0.5 - best / mpp).clamp(0.0, 1.0);
        }
    }
    (mask, mpp / scale)
}
