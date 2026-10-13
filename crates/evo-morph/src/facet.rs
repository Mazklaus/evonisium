//! Facettes à normales plates, communes aux objets (étape 6) et aux
//! bâtiments (étape 7) du palier 3 : triangles, quadrilatères, troncs de
//! cône, galets, lames.

use crate::body::Mesh;
use crate::canvas::DrawRng;
use std::f32::consts::{PI, TAU};

/// Constructeur de facettes : chaque triangle a ses trois sommets et sa
/// normale plate.
#[derive(Default)]
pub(crate) struct Builder {
    pub(crate) mesh: Mesh,
}

pub(crate) type P = [f32; 3];

pub(crate) fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn mul(a: P, k: f32) -> P {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub(crate) fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub(crate) fn norm(a: P) -> P {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 1e-9 {
        mul(a, 1.0 / l)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Deux axes perpendiculaires à `d`.
pub(crate) fn frame(d: P) -> (P, P) {
    let up = if d[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let u = norm(cross(d, up));
    let v = norm(cross(u, d));
    (u, v)
}

impl Builder {
    pub(crate) fn tri(&mut self, a: P, b: P, c: P, col: [f32; 3]) {
        let n = norm(cross(sub(b, a), sub(c, a)));
        let base = self.mesh.positions.len() as u32;
        for p in [a, b, c] {
            self.mesh.positions.push(p);
            self.mesh.normals.push(n);
            self.mesh.colours.push([col[0], col[1], col[2], 0.0]);
            self.mesh.bones.push([0, 0]);
            self.mesh.weights.push([1.0, 0.0]);
        }
        self.mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    pub(crate) fn quad(&mut self, a: P, b: P, c: P, d: P, col: [f32; 3]) {
        self.tri(a, b, c, col);
        self.tri(a, c, d, col);
    }

    /// Tronc de cône de `a` (rayon `ra`) à `b` (rayon `rb`), fermé.
    pub(crate) fn tube(&mut self, a: P, b: P, ra: f32, rb: f32, sides: usize, col: [f32; 3]) {
        let d = norm(sub(b, a));
        let (u, v) = frame(d);
        let ring = |c: P, r: f32, k: usize| {
            let t = TAU * k as f32 / sides as f32;
            add(c, add(mul(u, r * t.cos()), mul(v, r * t.sin())))
        };
        for k in 0..sides {
            let (a0, a1) = (ring(a, ra, k), ring(a, ra, k + 1));
            let (b0, b1) = (ring(b, rb, k), ring(b, rb, k + 1));
            self.quad(a0, a1, b1, b0, col);
            if ra > 0.0 {
                self.tri(a, a1, a0, col);
            }
            if rb > 0.0 {
                self.tri(b, b0, b1, col);
            }
        }
    }

    /// Galet : ellipsoïde à facettes, un peu bosselé.
    pub(crate) fn pebble(&mut self, c: P, r: P, rng: &mut DrawRng, col: [f32; 3]) {
        let (rings, segs) = (4, 7);
        let mut pts = vec![[0.0f32; 3]; (rings + 1) * segs];
        for i in 0..=rings {
            let phi = PI * i as f32 / rings as f32;
            for j in 0..segs {
                let th = TAU * j as f32 / segs as f32;
                let k = if i == 0 || i == rings { 1.0 } else { rng.range(0.85, 1.12) };
                pts[i * segs + j] = add(c, [r[0] * k * phi.sin() * th.cos(), r[1] * k * phi.cos(), r[2] * k * phi.sin() * th.sin()]);
            }
        }
        for i in 0..rings {
            for j in 0..segs {
                let (a, b) = (pts[i * segs + j], pts[i * segs + (j + 1) % segs]);
                let (c2, d) = (pts[(i + 1) * segs + (j + 1) % segs], pts[(i + 1) * segs + j]);
                self.quad(a, d, c2, b, col);
            }
        }
    }

    /// Lame plate : losange épais (éclats, bifaces, pointes).
    pub(crate) fn blade(&mut self, base: P, tip: P, width: f32, thick: f32, col: [f32; 3]) {
        let d = sub(tip, base);
        let (u, v) = frame(norm(d));
        let mid = add(base, mul(d, 0.4));
        let l = add(mid, mul(u, width / 2.0));
        let r = add(mid, mul(u, -width / 2.0));
        let up = add(mid, mul(v, thick / 2.0));
        let dn = add(mid, mul(v, -thick / 2.0));
        for (a, b) in [(l, up), (up, r), (r, dn), (dn, l)] {
            self.tri(base, a, b, col);
            self.tri(tip, b, a, col);
        }
    }
}
