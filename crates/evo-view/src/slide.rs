//! Lame de microscope en 3D (document Rendu du vivant, vue microscope
//! séparée sous 1 mm ; demande de l'utilisateur du 12 octobre 2026) : une
//! goutte d'eau entre lame et lamelle où nagent les microbes d'une cellule.
//!
//! [Simplification] Comme la loupe en 2D, la lame figure des cellules tirées
//! des écotypes présents dans les proportions de leur biomasse : le moteur ne
//! suit pas d'individus microbiens. La forme vient des mêmes traits
//! (`evo_morph::microbe::form`) ; la nage est une marche « course et
//! culbute » pour les cellules à flagelles, une dérive brownienne pour les
//! autres, un glissement lent pour les filaments. Rien ne remonte au moteur.
//!
//! Unités : micromètres, repère de Godot (y vers le haut, la lame dans le
//! plan x-z).

use evo_morph::body::Mesh;
use evo_morph::canvas::{mix64, DrawRng};
use evo_morph::microbe::{MicrobeForm, Shape};

/// Champ de la lame (x, z) et épaisseur du film d'eau (y), µm.
pub const FIELD_UM: [f32; 3] = [180.0, 24.0, 120.0];

/// Une espèce de la lame.
#[derive(Clone, Debug)]
pub struct SlideSpecies {
    pub form: MicrobeForm,
    /// Part de la biomasse de la cellule.
    pub share: f32,
    /// Signature d'espèce et lignée (fiche, nom).
    pub species: u32,
    pub lineage: u32,
}

#[derive(Clone, Debug)]
pub struct Microbe {
    pub species: u16,
    pub pos: [f32; 3],
    /// Direction de nage (unitaire).
    pub dir: [f32; 3],
    /// Rotation propre autour de l'axe long, radians.
    pub roll: f32,
    run_left: f32,
}

pub struct Slide {
    pub species: Vec<SlideSpecies>,
    pub cells: Vec<Microbe>,
    rng: DrawRng,
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Direction au hasard, surtout dans le plan de la lame (le film est mince).
fn random_dir(r: &mut DrawRng) -> [f32; 3] {
    let a = r.unit() * std::f32::consts::TAU;
    norm([a.cos(), r.range(-0.25, 0.25), a.sin()])
}

impl SlideSpecies {
    /// Vitesse de nage, µm/s : quelques longueurs de corps par seconde pour
    /// une cellule à flagelles (10 à 40 µm/s chez les bactéries réelles).
    pub fn speed_um_s(&self) -> f32 {
        let f = &self.form;
        match (f.shape, f.flagella) {
            (Shape::Filament, _) => 1.5,
            (_, 0) => 0.0,
            _ => (8.0 + 6.0 * f.flagella as f32 + 2.0 * f.length_um).min(40.0),
        }
    }
}

impl Slide {
    /// Peuple la lame : de 120 à 360 cellules selon la diversité, au moins
    /// trois par espèce pour qu'on puisse les trouver.
    pub fn new(species: Vec<SlideSpecies>, seed: u64) -> Slide {
        let mut rng = DrawRng::new(mix64(seed ^ 0x6C61_6D65));
        let total: f32 = species.iter().map(|s| s.share.max(0.0)).sum::<f32>().max(1e-9);
        let count = (120 + 20 * species.len()).min(360);
        let mut cells = Vec::new();
        for (k, s) in species.iter().enumerate() {
            let n = ((count as f32 * s.share.max(0.0) / total).round() as usize).max(3);
            for _ in 0..n {
                let pos = [
                    rng.range(-FIELD_UM[0] / 2.0, FIELD_UM[0] / 2.0),
                    rng.range(-FIELD_UM[1] / 2.0, FIELD_UM[1] / 2.0),
                    rng.range(-FIELD_UM[2] / 2.0, FIELD_UM[2] / 2.0),
                ];
                cells.push(Microbe {
                    species: k as u16,
                    pos,
                    dir: random_dir(&mut rng),
                    roll: rng.unit() * std::f32::consts::TAU,
                    run_left: rng.range(0.0, 2.0),
                });
            }
        }
        Slide { species, cells, rng }
    }

    /// Avance la lame de `dt` secondes.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        let half = [FIELD_UM[0] / 2.0, FIELD_UM[1] / 2.0, FIELD_UM[2] / 2.0];
        for c in &mut self.cells {
            let s = &self.species[c.species as usize];
            let v = s.speed_um_s();
            let r = &mut self.rng;
            if v > 2.0 {
                // Course, puis culbute vers une direction nouvelle.
                c.run_left -= dt;
                if c.run_left <= 0.0 {
                    c.dir = random_dir(r);
                    c.run_left = r.range(0.6, 2.5);
                }
                c.roll += dt * 9.0;
            } else {
                // Dérive : le mouvement brownien d'une cellule de 1 µm est de
                // l'ordre du micromètre par seconde.
                let jitter = 1.2 * dt.sqrt();
                for i in 0..3 {
                    c.pos[i] += r.range(-jitter, jitter);
                }
                c.dir = norm([c.dir[0] + r.range(-0.05, 0.05), c.dir[1] + r.range(-0.02, 0.02), c.dir[2] + r.range(-0.05, 0.05)]);
                c.roll += dt * 0.3;
            }
            for ((p, d), h) in c.pos.iter_mut().zip(c.dir.iter_mut()).zip(half) {
                *p += *d * v * dt;
                // Les parois (lame, lamelle, bord du champ) renvoient.
                if *p > h {
                    *p = h;
                    *d = -d.abs();
                } else if *p < -h {
                    *p = -h;
                    *d = d.abs();
                }
            }
        }
    }

    /// Cellules d'une espèce.
    pub fn count_of(&self, k: usize) -> usize {
        self.cells.iter().filter(|c| c.species as usize == k).count()
    }

    /// Transformations des cellules d'une espèce, au format des `MultiMesh`
    /// de Godot (3 × 4 par instance, ligne par ligne) : l'axe x du maillage
    /// suit la direction de nage.
    pub fn transforms(&self, k: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for c in self.cells.iter().filter(|c| c.species as usize == k) {
            let x = c.dir;
            let up = if x[1].abs() > 0.95 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
            let y0 = norm(cross(cross(x, up), x));
            let z0 = cross(x, y0);
            let (s, co) = c.roll.sin_cos();
            let y = [y0[0] * co + z0[0] * s, y0[1] * co + z0[1] * s, y0[2] * co + z0[2] * s];
            let z = cross(x, y);
            for i in 0..3 {
                out.extend_from_slice(&[x[i], y[i], z[i], c.pos[i]]);
            }
        }
        out
    }

    /// Espèce de la cellule que vise le rayon (`origin`, `dir` unitaire),
    /// la plus proche le long du rayon.
    pub fn pick(&self, origin: [f32; 3], dir: [f32; 3]) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for c in &self.cells {
            let s = &self.species[c.species as usize];
            let radius = (s.form.extent_um() * 0.5).max(s.form.width_um) + 0.8;
            let d = [c.pos[0] - origin[0], c.pos[1] - origin[1], c.pos[2] - origin[2]];
            let t = d[0] * dir[0] + d[1] * dir[1] + d[2] * dir[2];
            if t < 0.0 {
                continue;
            }
            let q = [d[0] - dir[0] * t, d[1] - dir[1] * t, d[2] - dir[2] * t];
            if (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt() <= radius && best.is_none_or(|b| t < b.0) {
                best = Some((t, c.species as usize));
            }
        }
        best.map(|b| b.1)
    }
}

// ----------------------------------------------------------------------
// Maillages

struct Builder {
    mesh: Mesh,
}

impl Builder {
    fn new() -> Builder {
        Builder { mesh: Mesh::default() }
    }

    fn vertex(&mut self, p: [f32; 3], n: [f32; 3], c: [f32; 4]) -> u32 {
        self.mesh.positions.push(p);
        self.mesh.normals.push(n);
        self.mesh.colours.push(c);
        self.mesh.bones.push([0, 0]);
        self.mesh.weights.push([1.0, 0.0]);
        (self.mesh.positions.len() - 1) as u32
    }

    /// Capsule de `a` à `b`, rayon `r` : un cylindre et deux demi-sphères,
    /// faite en anneaux le long de l'axe.
    fn capsule(&mut self, a: [f32; 3], b: [f32; 3], r: f32, colour: [f32; 4], segs: usize) {
        let axis = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        let x = if len > 1e-6 { norm(axis) } else { [1.0, 0.0, 0.0] };
        let up = if x[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let y = norm(cross(cross(x, up), x));
        let z = cross(x, y);
        let cap = (segs / 2).max(3);
        // Anneaux : latitude de −90° à +90°, la moitié arrière en `a`, la
        // moitié avant en `b`.
        let mut rings: Vec<([f32; 3], f32, f32)> = Vec::new(); // (centre, rayon d'anneau, sinus de latitude)
        for i in 0..=cap {
            let t = -std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_2 * i as f32 / cap as f32;
            rings.push((a, r * t.cos(), t.sin()));
        }
        for i in 0..=cap {
            let t = std::f32::consts::FRAC_PI_2 * i as f32 / cap as f32;
            rings.push((b, r * t.cos(), t.sin()));
        }
        let base = self.mesh.positions.len() as u32;
        for &(c, rr, s) in &rings {
            for j in 0..segs {
                let ang = std::f32::consts::TAU * j as f32 / segs as f32;
                let (sa, ca) = ang.sin_cos();
                let radial = [y[0] * ca + z[0] * sa, y[1] * ca + z[1] * sa, y[2] * ca + z[2] * sa];
                let n = norm([
                    radial[0] * (1.0 - s * s).sqrt() + x[0] * s,
                    radial[1] * (1.0 - s * s).sqrt() + x[1] * s,
                    radial[2] * (1.0 - s * s).sqrt() + x[2] * s,
                ]);
                let p = [c[0] + radial[0] * rr + x[0] * r * s, c[1] + radial[1] * rr + x[1] * r * s, c[2] + radial[2] * rr + x[2] * r * s];
                self.vertex(p, n, colour);
            }
        }
        let segs = segs as u32;
        for i in 0..rings.len() as u32 - 1 {
            for j in 0..segs {
                let (a0, a1) = (base + i * segs + j, base + i * segs + (j + 1) % segs);
                let (b0, b1) = (a0 + segs, a1 + segs);
                self.mesh.indices.extend_from_slice(&[a0, b0, a1, a1, b0, b1]);
            }
        }
    }
}

/// Maillage d'une cellule, en µm, axe long selon x, centré : le corps à la
/// couleur de son pigment, ses flagelles en fines ondulations à l'arrière,
/// ses granules en grains sombres.
pub fn cell_mesh(f: &MicrobeForm) -> Mesh {
    let mut b = Builder::new();
    let body = [f.colour[0], f.colour[1], f.colour[2], 1.0];
    let dark = [f.colour[0] * 0.55, f.colour[1] * 0.5, f.colour[2] * 0.45, 1.0];
    let r = f.width_um / 2.0;
    match f.shape {
        Shape::Coccus => b.capsule([0.0; 3], [0.0; 3], r, body, 18),
        Shape::Bacillus => {
            let h = (f.length_um / 2.0 - r).max(0.0);
            b.capsule([-h, 0.0, 0.0], [h, 0.0, 0.0], r, body, 16);
        }
        Shape::Spirillum => {
            // Hélice : des capsules jointives le long de la spire.
            let n = 28;
            let amp = f.width_um * 1.1;
            let pts: Vec<[f32; 3]> = (0..=n)
                .map(|i| {
                    let x = -f.length_um / 2.0 + f.length_um * i as f32 / n as f32;
                    let a = x / f.length_um * std::f32::consts::TAU * 1.5;
                    [x, amp * a.sin(), amp * a.cos()]
                })
                .collect();
            for w in pts.windows(2) {
                b.capsule(w[0], w[1], r * 0.8, body, 10);
            }
        }
        Shape::Filament => {
            let total = f.length_um * f.segments as f32;
            let h = (f.length_um / 2.0 - r).max(0.0) * 0.92;
            for i in 0..f.segments {
                let cx = -total / 2.0 + f.length_um * (i as f32 + 0.5);
                b.capsule([cx - h, 0.0, 0.0], [cx + h, 0.0, 0.0], r * 0.95, body, 12);
            }
        }
    }
    // Flagelles : à l'arrière, ondulés, quatre à cinq longueurs de corps.
    let back = -f.extent_um() / 2.0;
    let mut rng = DrawRng::new(f.seed ^ 0x666C);
    for k in 0..f.flagella {
        let tilt = (k as f32 - (f.flagella as f32 - 1.0) / 2.0) * 0.5;
        let len = f.length_um.max(1.5) * rng.range(2.0, 3.0);
        let n = 16;
        let mut prev = [back, tilt * r, 0.0];
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let x = back - len * t;
            let wave = 0.35 * f.width_um * (t * std::f32::consts::TAU * 2.0).sin();
            let p = [x, tilt * r * (1.0 + 2.0 * t) + wave, wave * 0.5];
            b.capsule(prev, p, 0.07, dark, 5);
            prev = p;
        }
    }
    // Granules : grains sombres affleurant à la surface.
    for _ in 0..f.granules.min(4) {
        let x = rng.range(-0.3, 0.3) * f.length_um.min(f.extent_um());
        let a = rng.unit() * std::f32::consts::TAU;
        let p = [x, r * 0.75 * a.cos(), r * 0.75 * a.sin()];
        b.capsule(p, p, (r * 0.28).max(0.08), dark, 8);
    }
    b.mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_morph::microbe::{form, MicrobeTraits};

    fn sp(lineage: u32, oxygenic: bool, share: f32) -> SlideSpecies {
        let t = MicrobeTraits { lineage, signature: 1, pigment_rgb: None, gene_count: 8, phototroph: oxygenic, oxygenic };
        SlideSpecies { form: form(&t, 7), share, species: 1, lineage }
    }

    #[test]
    fn every_species_has_cells_and_they_stay_on_the_slide() {
        let mut s = Slide::new(vec![sp(1, false, 0.9), sp(2, true, 0.1), sp(3, false, 0.0)], 5);
        for k in 0..3 {
            assert!(s.count_of(k) >= 3);
        }
        for _ in 0..600 {
            s.step(1.0 / 30.0);
        }
        for c in &s.cells {
            for (p, f) in c.pos.iter().zip(FIELD_UM) {
                assert!(p.abs() <= f / 2.0 + 1e-3);
            }
        }
        assert_eq!(s.transforms(0).len(), 12 * s.count_of(0));
    }

    #[test]
    fn swimmers_move_and_the_picked_cell_is_the_nearest() {
        let mut s = Slide::new(vec![sp(1, false, 1.0)], 9);
        let before: Vec<[f32; 3]> = s.cells.iter().map(|c| c.pos).collect();
        s.step(0.1);
        assert!(s.cells.iter().zip(&before).any(|(c, b)| c.pos != *b));
        let c = s.cells[0].pos;
        let hit = s.pick([c[0], c[1] + 50.0, c[2]], [0.0, -1.0, 0.0]);
        assert_eq!(hit, Some(0));
        assert_eq!(s.pick([c[0], 500.0, c[2]], [0.0, 1.0, 0.0]), None);
    }

    #[test]
    fn meshes_are_closed_and_sized_in_micrometres() {
        for (l, ox) in [(1, false), (2, true), (3, false), (4, true), (11, false)] {
            let f = sp(l, ox, 1.0).form;
            let m = cell_mesh(&f);
            assert!(!m.indices.is_empty() && m.indices.len().is_multiple_of(3));
            assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
            let ext = m.positions.iter().map(|p| p[0].abs()).fold(0.0, f32::max);
            assert!(ext > 0.2 && ext < 200.0, "{ext}");
        }
    }
}
