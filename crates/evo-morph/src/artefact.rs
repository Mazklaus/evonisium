//! Palier 3 du rendu, première part (doc d'architecture, étape 6) : les
//! objets des premières cultures, outils, abris, campements et parures,
//! construits en facettes à partir de quelques primitives (tronc de cône,
//! galet, anneau). Les facettes à normales plates et les couleurs de
//! matière (bois, pierre, peau, os, fibre, ocre) donnent la gravure de
//! l'Atlas ; le client y pose les hachures du terrain.
//!
//! Repère : y vers le haut, x vers l'avant de l'objet, en mètres ; la base
//! de l'objet est à y = 0, au centre. Chaque objet est tiré de sa graine
//! (nombre de perches, galets, perles), pur et déterministe.
//!
//! [Simplification] Les objets ne portent ni os ni poids : un objet tenu
//! suit l'individu en bloc, il n'est pas déformé par la pose.

use crate::body::Mesh;
use crate::canvas::DrawRng;
use crate::facet::{add, mul, norm, sub, Builder, P};
use std::f32::consts::{PI, TAU};

/// Les objets du palier 3 (étape 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Artefact {
    /// Hutte de perches couverte de peaux.
    Hut,
    /// Pare-vent de branchages en arc.
    Windbreak,
    /// Foyer : cercle de pierres, bûches et flamme.
    Hearth,
    /// Séchoir : deux chevalets, une perche, des peaux pendues.
    Rack,
    /// Percuteur : un galet.
    Hammerstone,
    /// Éclat de pierre taillé.
    Flake,
    /// Biface en amande.
    Handaxe,
    /// Épieu : hampe et pointe de pierre.
    Spear,
    /// Bâton à sonder.
    Stick,
    /// Pelote de corde de fibres.
    Cord,
    /// Contenant d'écorce.
    Bowl,
    /// Collier de perches d'os et d'ocre.
    Necklace,
    /// Radeau de rondins liés.
    Raft,
    /// Poinçon d'os.
    Awl,
    /// Nasse de vannerie.
    Trap,
    /// Propulseur et sa sagaie.
    Thrower,
}

impl Artefact {
    pub const ALL: [Artefact; 16] = [
        Artefact::Hut,
        Artefact::Windbreak,
        Artefact::Hearth,
        Artefact::Rack,
        Artefact::Hammerstone,
        Artefact::Flake,
        Artefact::Handaxe,
        Artefact::Spear,
        Artefact::Stick,
        Artefact::Cord,
        Artefact::Bowl,
        Artefact::Necklace,
        Artefact::Raft,
        Artefact::Awl,
        Artefact::Trap,
        Artefact::Thrower,
    ];

    pub fn index(self) -> usize {
        Artefact::ALL.iter().position(|&a| a == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Artefact {
        Artefact::ALL[i.min(Artefact::ALL.len() - 1)]
    }

    pub fn key(self) -> &'static str {
        match self {
            Artefact::Hut => "hutte",
            Artefact::Windbreak => "pare_vent",
            Artefact::Hearth => "foyer",
            Artefact::Rack => "sechoir",
            Artefact::Hammerstone => "percuteur",
            Artefact::Flake => "eclat",
            Artefact::Handaxe => "biface",
            Artefact::Spear => "epieu",
            Artefact::Stick => "baton",
            Artefact::Cord => "corde",
            Artefact::Bowl => "contenant",
            Artefact::Necklace => "collier",
            Artefact::Raft => "radeau",
            Artefact::Awl => "poincon",
            Artefact::Trap => "nasse",
            Artefact::Thrower => "propulseur",
        }
    }

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Artefact::Hut, true) => "hutte",
            (Artefact::Hut, false) => "hut",
            (Artefact::Windbreak, true) => "pare-vent",
            (Artefact::Windbreak, false) => "windbreak",
            (Artefact::Hearth, true) => "foyer",
            (Artefact::Hearth, false) => "hearth",
            (Artefact::Rack, true) => "séchoir",
            (Artefact::Rack, false) => "drying rack",
            (Artefact::Hammerstone, true) => "percuteur",
            (Artefact::Hammerstone, false) => "hammerstone",
            (Artefact::Flake, true) => "éclat",
            (Artefact::Flake, false) => "flake",
            (Artefact::Handaxe, true) => "biface",
            (Artefact::Handaxe, false) => "hand axe",
            (Artefact::Spear, true) => "épieu",
            (Artefact::Spear, false) => "spear",
            (Artefact::Stick, true) => "bâton",
            (Artefact::Stick, false) => "stick",
            (Artefact::Cord, true) => "corde",
            (Artefact::Cord, false) => "cord",
            (Artefact::Bowl, true) => "contenant",
            (Artefact::Bowl, false) => "container",
            (Artefact::Necklace, true) => "collier",
            (Artefact::Necklace, false) => "necklace",
            (Artefact::Raft, true) => "radeau",
            (Artefact::Raft, false) => "raft",
            (Artefact::Awl, true) => "poinçon",
            (Artefact::Awl, false) => "awl",
            (Artefact::Trap, true) => "nasse",
            (Artefact::Trap, false) => "fish trap",
            (Artefact::Thrower, true) => "propulseur",
            (Artefact::Thrower, false) => "spear-thrower",
        }
    }

    /// Objet du campement (posé une fois) plutôt qu'outil tenu ou porté.
    pub fn fixed(self) -> bool {
        matches!(self, Artefact::Hut | Artefact::Windbreak | Artefact::Hearth | Artefact::Rack | Artefact::Raft)
    }
}

/// Couleurs des matières, sRGB, dans la palette de l'Atlas.
pub const WOOD: [f32; 3] = [0.47, 0.35, 0.24];
pub const BARK: [f32; 3] = [0.40, 0.31, 0.22];
pub const STONE: [f32; 3] = [0.58, 0.56, 0.52];
pub const FLINT: [f32; 3] = [0.50, 0.50, 0.53];
pub const HIDE: [f32; 3] = [0.72, 0.57, 0.40];
pub const BONE: [f32; 3] = [0.88, 0.84, 0.74];
pub const FIBRE: [f32; 3] = [0.76, 0.68, 0.46];
pub const OCHRE_RED: [f32; 3] = [0.62, 0.27, 0.17];
pub const EMBER: [f32; 3] = [0.86, 0.45, 0.18];
pub const ASH: [f32; 3] = [0.36, 0.33, 0.30];

/// Construit l'objet `a` tiré de `seed`.
pub fn mesh(a: Artefact, seed: u64) -> Mesh {
    let mut rng = DrawRng::new(seed ^ (a.index() as u64).wrapping_mul(0x9E37_79B9));
    let mut b = Builder::default();
    match a {
        Artefact::Hut => {
            // Perches en cercle penchées vers le faîte, peaux entre elles
            // sauf à l'entrée (vers +x).
            let n = 9 + (rng.next_u64() % 4) as usize;
            let (r, h) = (rng.range(1.4, 1.8), rng.range(2.0, 2.5));
            let apex = [0.0, h, 0.0];
            let feet: Vec<P> = (0..n)
                .map(|k| {
                    let t = TAU * (k as f32 + 0.5) / n as f32 + rng.range(-0.08, 0.08);
                    [r * t.cos(), 0.0, r * t.sin()]
                })
                .collect();
            for f in &feet {
                let over = add(apex, mul(norm(sub(apex, *f)), 0.35));
                b.tube(*f, over, 0.04, 0.025, 5, WOOD);
            }
            for k in 0..n {
                if k == 0 || k == n - 1 {
                    continue;
                }
                let (f0, f1) = (feet[k], feet[(k + 1) % n]);
                let low0 = add(f0, mul(sub(apex, f0), 0.05));
                let low1 = add(f1, mul(sub(apex, f1), 0.05));
                let hi0 = add(f0, mul(sub(apex, f0), 0.88));
                let hi1 = add(f1, mul(sub(apex, f1), 0.88));
                let shade = if k % 2 == 0 { HIDE } else { [HIDE[0] * 0.9, HIDE[1] * 0.88, HIDE[2] * 0.85] };
                b.quad(low0, low1, hi1, hi0, shade);
                b.quad(low0, hi0, hi1, low1, shade);
            }
        }
        Artefact::Windbreak => {
            let n = 7 + (rng.next_u64() % 4) as usize;
            let r = rng.range(1.6, 2.2);
            for k in 0..n {
                let t = -0.9 + 1.8 * k as f32 / (n - 1) as f32;
                let foot = [-r * t.cos() + r, 0.0, r * t.sin()];
                let top = add(foot, [0.25, rng.range(1.0, 1.4), 0.0]);
                b.tube(foot, top, 0.035, 0.015, 4, WOOD);
                if k + 1 < n {
                    let t2 = -0.9 + 1.8 * (k + 1) as f32 / (n - 1) as f32;
                    let foot2 = [-r * t2.cos() + r, 0.0, r * t2.sin()];
                    let top2 = add(foot2, [0.25, 1.1, 0.0]);
                    b.quad(foot, foot2, top2, add(foot, [0.25, 1.1, 0.0]), FIBRE);
                    b.quad(foot, add(foot, [0.25, 1.1, 0.0]), top2, foot2, FIBRE);
                }
            }
        }
        Artefact::Hearth => {
            let n = 8 + (rng.next_u64() % 3) as usize;
            let r = rng.range(0.45, 0.6);
            for k in 0..n {
                let t = TAU * k as f32 / n as f32;
                let s = rng.range(0.09, 0.14);
                b.pebble([r * t.cos(), s * 0.5, r * t.sin()], [s, s * 0.7, s], &mut rng, STONE);
            }
            // Cendres, bûches en étoile, flamme.
            b.tube([0.0, 0.0, 0.0], [0.0, 0.03, 0.0], r * 0.85, r * 0.8, 9, ASH);
            for k in 0..3 {
                let t = TAU * k as f32 / 3.0 + rng.range(-0.2, 0.2);
                b.tube([0.5 * t.cos(), 0.04, 0.5 * t.sin()], [0.05 * t.cos(), 0.16, 0.05 * t.sin()], 0.05, 0.045, 5, BARK);
            }
            b.tube([0.0, 0.1, 0.0], [0.0, 0.62, 0.0], 0.2, 0.0, 6, EMBER);
            b.tube([0.06, 0.1, 0.03], [0.1, 0.45, 0.02], 0.1, 0.0, 5, OCHRE_RED);
        }
        Artefact::Rack => {
            let w = rng.range(1.8, 2.4);
            for x in [-w / 2.0, w / 2.0] {
                b.tube([x, 0.0, -0.4], [x, 1.5, 0.0], 0.035, 0.03, 5, WOOD);
                b.tube([x, 0.0, 0.4], [x, 1.5, 0.0], 0.035, 0.03, 5, WOOD);
            }
            b.tube([-w / 2.0 - 0.1, 1.5, 0.0], [w / 2.0 + 0.1, 1.5, 0.0], 0.03, 0.03, 5, WOOD);
            let hides = 2 + (rng.next_u64() % 2) as usize;
            for k in 0..hides {
                let x = -w / 2.0 + w * (k as f32 + 0.5) / hides as f32;
                let hw = rng.range(0.25, 0.35);
                let len = rng.range(0.6, 0.9);
                let (a0, a1) = ([x - hw, 1.48, 0.0], [x + hw, 1.48, 0.0]);
                let (b0, b1) = ([x - hw * 0.8, 1.48 - len, 0.05], [x + hw * 0.8, 1.48 - len, 0.05]);
                b.quad(a0, a1, b1, b0, HIDE);
                b.quad(a0, b0, b1, a1, HIDE);
            }
        }
        Artefact::Hammerstone => b.pebble([0.0, 0.035, 0.0], [0.055, 0.035, 0.045], &mut rng, STONE),
        Artefact::Flake => b.blade([-0.03, 0.01, 0.0], [0.04, 0.01, 0.01], 0.045, 0.012, FLINT),
        Artefact::Handaxe => b.blade([-0.07, 0.02, 0.0], [0.09, 0.02, 0.0], 0.09, 0.035, FLINT),
        Artefact::Spear => {
            let len = rng.range(1.8, 2.2);
            b.tube([-len / 2.0, 0.0, 0.0], [len / 2.0 - 0.12, 0.0, 0.0], 0.018, 0.016, 5, WOOD);
            b.blade([len / 2.0 - 0.14, 0.0, 0.0], [len / 2.0 + 0.06, 0.0, 0.0], 0.05, 0.015, FLINT);
            b.tube([len / 2.0 - 0.18, 0.0, 0.0], [len / 2.0 - 0.1, 0.0, 0.0], 0.024, 0.022, 5, FIBRE);
        }
        Artefact::Stick => {
            let len = rng.range(0.5, 0.7);
            b.tube([-len / 2.0, 0.0, 0.0], [len / 2.0, 0.0, 0.01], 0.012, 0.006, 4, WOOD);
        }
        Artefact::Cord => {
            // Pelote : anneaux de corde empilés.
            for k in 0..4 {
                let r = 0.08 - 0.01 * k as f32;
                let y = 0.012 + 0.02 * k as f32;
                let segs = 10;
                for j in 0..segs {
                    let (t0, t1) = (TAU * j as f32 / segs as f32, TAU * (j + 1) as f32 / segs as f32);
                    b.tube([r * t0.cos(), y, r * t0.sin()], [r * t1.cos(), y, r * t1.sin()], 0.011, 0.011, 4, FIBRE);
                }
            }
        }
        Artefact::Bowl => {
            // Coque d'écorce : demi-sphère ouverte, deux faces.
            let (r, rings, segs) = (rng.range(0.12, 0.16), 3, 9);
            let pt = |i: usize, j: usize| {
                let phi = PI / 2.0 * i as f32 / rings as f32;
                let th = TAU * j as f32 / segs as f32;
                [r * phi.sin() * th.cos(), r * (1.0 - phi.cos()), r * phi.sin() * th.sin()]
            };
            for i in 0..rings {
                for j in 0..segs {
                    let (a, b2, c, d) = (pt(i, j), pt(i, j + 1), pt(i + 1, j + 1), pt(i + 1, j));
                    b.quad(a, b2, c, d, BARK);
                    b.quad(a, d, c, b2, [BARK[0] * 1.2, BARK[1] * 1.15, BARK[2] * 1.1]);
                }
            }
        }
        Artefact::Necklace => {
            let n = 9 + (rng.next_u64() % 5) as usize;
            let r = 0.11;
            for k in 0..n {
                let t = PI * 0.15 + PI * 0.7 * k as f32 / (n - 1) as f32;
                let col = if k % 3 == 1 { OCHRE_RED } else { BONE };
                let s = if k == n / 2 { 0.022 } else { 0.012 };
                b.pebble([r * t.cos() - r * 0.3, -r * t.sin() * 0.9, 0.0], [s, s, s], &mut rng, col);
            }
        }
        Artefact::Raft => {
            let logs = 5 + (rng.next_u64() % 2) as usize;
            let len = rng.range(2.3, 2.8);
            for k in 0..logs {
                let z = (k as f32 - (logs - 1) as f32 / 2.0) * 0.17;
                b.tube([-len / 2.0, 0.08, z], [len / 2.0, 0.08, z + rng.range(-0.03, 0.03)], 0.08, 0.07, 6, WOOD);
            }
            for x in [-len / 3.0, len / 3.0] {
                b.tube([x, 0.17, -0.5], [x, 0.17, 0.5], 0.018, 0.018, 4, FIBRE);
            }
        }
        Artefact::Awl => {
            b.tube([-0.06, 0.012, 0.0], [0.03, 0.012, 0.0], 0.012, 0.007, 5, BONE);
            b.tube([0.03, 0.012, 0.0], [0.07, 0.012, 0.0], 0.007, 0.0, 5, BONE);
        }
        Artefact::Trap => {
            // Nasse : cône de vannerie couché, ouvert vers +x.
            let (len, r) = (rng.range(0.7, 0.9), rng.range(0.18, 0.24));
            let ribs = 8;
            for k in 0..ribs {
                let t = TAU * k as f32 / ribs as f32;
                b.tube([len / 2.0, r + r * t.sin(), r * t.cos()], [-len / 2.0, r, 0.0], 0.008, 0.006, 3, FIBRE);
            }
            for f in [0.25, 0.6] {
                let segs = 10;
                let rr = r * (1.0 - f) + 0.02;
                let x = len / 2.0 - len * f;
                for j in 0..segs {
                    let (t0, t1) = (TAU * j as f32 / segs as f32, TAU * (j + 1) as f32 / segs as f32);
                    b.tube([x, r + rr * t0.sin(), rr * t0.cos()], [x, r + rr * t1.sin(), rr * t1.cos()], 0.008, 0.008, 3, BARK);
                }
            }
        }
        Artefact::Thrower => {
            b.tube([-0.3, 0.0, 0.0], [0.3, 0.0, 0.0], 0.016, 0.012, 5, WOOD);
            b.pebble([0.3, 0.02, 0.0], [0.02, 0.025, 0.015], &mut rng, BONE);
            b.tube([-0.2, 0.03, 0.0], [1.3, 0.03, 0.0], 0.009, 0.008, 4, WOOD);
            b.blade([1.28, 0.03, 0.0], [1.38, 0.03, 0.0], 0.025, 0.008, FLINT);
        }
    }
    b.mesh
}

/// Taille de l'objet (plus grande dimension), en mètres.
pub fn extent_m(m: &Mesh) -> f32 {
    let (lo, hi) = m.bounds();
    (hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_artefact_has_a_closed_set_of_facets_at_its_scale() {
        for a in Artefact::ALL {
            let m = mesh(a, 7);
            assert!(m.triangles() >= 8, "{a:?}");
            assert_eq!(m.positions.len(), m.normals.len());
            assert_eq!(m.indices.len() % 3, 0);
            let e = extent_m(&m);
            let (lo, max) = if a.fixed() { (0.8, 3.5) } else { (0.05, 2.5) };
            assert!(e > lo && e < max, "{a:?} {e}");
            for n in &m.normals {
                let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((l - 1.0).abs() < 1e-3 || l == 0.0, "{a:?}");
            }
            assert!(m.positions.iter().all(|p| p.iter().all(|v| v.is_finite())));
        }
    }

    #[test]
    fn artefacts_are_deterministic_and_vary_with_the_seed() {
        assert_eq!(mesh(Artefact::Hut, 3), mesh(Artefact::Hut, 3));
        assert_ne!(mesh(Artefact::Hut, 3), mesh(Artefact::Hut, 4));
        for (i, a) in Artefact::ALL.iter().enumerate() {
            assert_eq!(a.index(), i);
            assert_eq!(Artefact::from_index(i), *a);
        }
    }
}
