//! Palier 1 : apparence des microbes et vue microscope.
//!
//! [Simplification] Les microbes sont des guildes sans individus dans le
//! moteur ; la vue microscope figure des cellules tirées des écotypes
//! présents, dans les proportions de leur biomasse (architecture, « Vue
//! microscope »). La forme vient des traits publiés par le moteur, la
//! variation de détail d'un hachage de la lignée : même lignée, même dessin.
//! Le plan de construction partagé (Génétique et Organismes) remplacera ces
//! traits quand les corps pluricellulaires arriveront (palier 2).

use crate::canvas::{fbm, mix64, Canvas, DrawRng, INK, OCHRE, PAPER, WATER};

/// Ce que le moteur dit d'un microbe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MicrobeTraits {
    pub lineage: u32,
    /// Voies métaboliques (bits du catalogue d'Organismes).
    pub signature: u32,
    /// Couleur de réflectance du pigment, s'il y en a.
    pub pigment_rgb: Option<[u8; 3]>,
    pub gene_count: u32,
    /// Capte la lumière.
    pub phototroph: bool,
    /// Photosynthèse oxygénique (membranes internes empilées).
    pub oxygenic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Sphère (coque).
    Coccus,
    /// Bâtonnet.
    Bacillus,
    /// Hélice.
    Spirillum,
    /// Chaîne de cellules.
    Filament,
}

/// Forme dessinée d'une lignée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MicrobeForm {
    pub shape: Shape,
    /// Longueur et largeur d'une cellule, µm.
    pub length_um: f32,
    pub width_um: f32,
    /// Cellules d'un filament.
    pub segments: u8,
    pub flagella: u8,
    pub granules: u8,
    pub thylakoids: bool,
    pub colour: [f32; 3],
    pub seed: u64,
}

/// Forme d'une lignée à partir de ses traits.
pub fn form(t: &MicrobeTraits, seed: u64) -> MicrobeForm {
    let h = mix64(seed ^ mix64(t.lineage as u64 ^ 0x6D6F_7270_6800));
    let mut r = DrawRng::new(h);
    // Méthanogènes en coques, fermentatrices en bâtonnets, phototrophes à
    // oxygène en filaments (équivalents fonctionnels, jamais des copies).
    let shape = if t.oxygenic {
        if r.unit() < 0.7 {
            Shape::Filament
        } else {
            Shape::Coccus
        }
    } else if t.phototroph {
        [Shape::Bacillus, Shape::Spirillum, Shape::Coccus][(r.next_u64() % 3) as usize]
    } else if t.signature & 1 != 0 {
        if r.unit() < 0.75 {
            Shape::Coccus
        } else {
            Shape::Bacillus
        }
    } else {
        [Shape::Bacillus, Shape::Bacillus, Shape::Spirillum, Shape::Coccus][(r.next_u64() % 4) as usize]
    };
    // Taille : un génome plus long, une cellule un peu plus grande.
    let size = (0.6 + 0.08 * t.gene_count as f32).min(2.5) * r.range(0.8, 1.25);
    let (length_um, width_um) = match shape {
        Shape::Coccus => (size, size),
        Shape::Bacillus => (size * r.range(2.0, 3.5), size),
        Shape::Spirillum => (size * r.range(4.0, 7.0), size * 0.6),
        Shape::Filament => (size * 1.2, size),
    };
    let colour = match t.pigment_rgb {
        Some(c) => [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0],
        // Sans pigment marqué : l'ocre pâle des corps incolores.
        None => {
            let k = r.range(0.35, 0.6);
            [PAPER[0] + (OCHRE[0] - PAPER[0]) * k, PAPER[1] + (OCHRE[1] - PAPER[1]) * k, PAPER[2] + (OCHRE[2] - PAPER[2]) * k]
        }
    };
    MicrobeForm {
        shape,
        length_um,
        width_um,
        segments: if shape == Shape::Filament { 4 + (r.next_u64() % 9) as u8 } else { 1 },
        flagella: if t.phototroph && shape == Shape::Filament { 0 } else { (r.next_u64() % 3) as u8 },
        granules: (r.next_u64() % 5) as u8,
        thylakoids: t.oxygenic,
        colour,
        seed: h,
    }
}

impl MicrobeForm {
    /// Étendue du dessin, µm (pour le placement).
    pub fn extent_um(&self) -> f32 {
        match self.shape {
            Shape::Filament => self.length_um * self.segments as f32,
            _ => self.length_um,
        }
    }
}

/// Distance d'un point à un segment.
fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> (f32, f32) {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = (dx * dx + dy * dy).max(1e-9);
    let t = (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0);
    let (qx, qy) = (ax + t * dx - px, ay + t * dy - py);
    ((qx * qx + qy * qy).sqrt(), t)
}

/// Squelette de la forme, en µm, dans le repère de la cellule (axe x) :
/// liste de segments (a, b) et rayon.
fn skeleton(f: &MicrobeForm) -> (Vec<[f32; 4]>, f32) {
    let r = f.width_um / 2.0;
    match f.shape {
        Shape::Coccus => (vec![[0.0, 0.0, 0.0, 0.0]], r),
        Shape::Bacillus => {
            let h = (f.length_um / 2.0 - r).max(0.0);
            (vec![[-h, 0.0, h, 0.0]], r)
        }
        Shape::Spirillum => {
            let n = 24;
            let half = f.length_um / 2.0;
            let amp = f.width_um * 1.2;
            let pts: Vec<(f32, f32)> = (0..=n)
                .map(|i| {
                    let x = -half + f.length_um * i as f32 / n as f32;
                    (x, amp * (x / f.length_um * std::f32::consts::TAU * 1.5).sin())
                })
                .collect();
            (pts.windows(2).map(|w| [w[0].0, w[0].1, w[1].0, w[1].1]).collect(), r)
        }
        Shape::Filament => {
            // Les cellules du filament sont dessinées une à une.
            let total = f.length_um * f.segments as f32;
            let h = (f.length_um / 2.0 - r).max(0.0);
            (
                (0..f.segments)
                    .map(|i| {
                        let cx = -total / 2.0 + f.length_um * (i as f32 + 0.5);
                        [cx - h, 0.0, cx + h, 0.0]
                    })
                    .collect(),
                r * 0.95,
            )
        }
    }
}

/// Dessine un microbe centré en (cx, cy), à `px_per_um` pixels par µm,
/// tourné de `angle` radians : lavis du pigment en plusieurs passes, bord
/// assombri, hachures du côté de l'ombre, contour à la plume.
pub fn draw(canvas: &mut Canvas, f: &MicrobeForm, cx: f32, cy: f32, px_per_um: f32, angle: f32) {
    let (segs, radius) = skeleton(f);
    let (s, c) = angle.sin_cos();
    let ext = (f.extent_um() / 2.0 + f.width_um * 2.5 + 3.0) * px_per_um;
    let ink_w = (0.06 * px_per_um).clamp(0.8, 2.2);
    let (x0, x1) = ((cx - ext).max(0.0) as usize, ((cx + ext) as usize).min(canvas.width.saturating_sub(1)));
    let (y0, y1) = ((cy - ext).max(0.0) as usize, ((cy + ext) as usize).min(canvas.height.saturating_sub(1)));
    let light = (-0.6f32, -0.8f32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            // Repère de la cellule, en µm.
            let (dx, dy) = ((x as f32 + 0.5 - cx) / px_per_um, (y as f32 + 0.5 - cy) / px_per_um);
            let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
            let mut best = f32::INFINITY;
            let mut nearest = (0.0, 0.0);
            for sg in &segs {
                let (d, t) = seg_dist(u, v, sg[0], sg[1], sg[2], sg[3]);
                if d < best {
                    best = d;
                    nearest = (sg[0] + t * (sg[2] - sg[0]), sg[1] + t * (sg[3] - sg[1]));
                }
            }
            let sdf = best - radius;
            let edge_px = sdf * px_per_um;
            if edge_px > ink_w + 1.0 {
                continue;
            }
            if sdf < 0.0 {
                // Normale apparente : du squelette vers le point.
                let (nx, ny) = ((u - nearest.0) / radius.max(1e-3), (v - nearest.1) / radius.max(1e-3));
                let (wnx, wny) = (nx * c - ny * s, nx * s + ny * c);
                let shade = (wnx * light.0 + wny * light.1).clamp(-1.0, 1.0);
                let grain = fbm(f.seed, x as f32 / 6.0, y as f32 / 6.0);
                // Lavis : plus dense au bord (le pigment s'accumule où l'eau sèche).
                let rim = (1.0 + sdf / radius).clamp(0.0, 1.0);
                let alpha = 0.45 + 0.35 * rim.powf(3.0) + 0.15 * (grain - 0.5);
                canvas.wash(x, y, f.colour, alpha);
                if shade < -0.2 {
                    // Hachures à 45° dans l'ombre.
                    let hatch = ((x as f32 + y as f32) / (0.18 * px_per_um).max(3.0)).fract();
                    if hatch < 0.22 {
                        canvas.set(x, y, INK, 0.35 * (-shade - 0.2));
                    }
                }
                if f.thylakoids {
                    // Membranes internes concentriques.
                    let ring = ((-sdf) / (radius * 0.22)).fract();
                    if ring < 0.12 && -sdf > radius * 0.25 {
                        canvas.set(x, y, [f.colour[0] * 0.55, f.colour[1] * 0.55, f.colour[2] * 0.55], 0.5);
                    }
                }
            }
            let cover = (ink_w * 0.5 + 0.5 - edge_px.abs()).clamp(0.0, 1.0);
            if cover > 0.0 {
                canvas.set(x, y, INK, 0.9 * cover);
            }
        }
    }
    // Granules de réserve.
    let mut r = DrawRng::new(f.seed ^ 0x6772);
    for _ in 0..f.granules {
        let sg = segs[(r.next_u64() as usize) % segs.len()];
        let t = r.unit();
        let (u, v) = (sg[0] + t * (sg[2] - sg[0]), sg[1] + t * (sg[3] - sg[1]) + r.range(-0.4, 0.4) * radius);
        let (gx, gy) = (cx + (u * c - v * s) * px_per_um, cy + (u * s + v * c) * px_per_um);
        let gr = radius * 0.18 * px_per_um;
        for k in 0..12 {
            let a0 = k as f32 / 12.0 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / 12.0 * std::f32::consts::TAU;
            canvas.line(gx + gr * a0.cos(), gy + gr * a0.sin(), gx + gr * a1.cos(), gy + gr * a1.sin(), ink_w * 0.7, INK, 0.7);
        }
    }
    // Flagelles : traits ondulés depuis un pôle.
    for k in 0..f.flagella {
        let end = segs[if k % 2 == 0 { segs.len() - 1 } else { 0 }];
        let (pu, sign) = if k % 2 == 0 { (end[2] + radius, 1.0) } else { (end[0] - radius, -1.0) };
        let len = f.length_um * 1.2;
        let mut last = (pu, end[3]);
        for i in 1..=30 {
            let t = i as f32 / 30.0;
            let u = pu + sign * len * t;
            let v = end[3] + radius * 0.45 * (t * std::f32::consts::TAU * 2.0 + k as f32).sin() * t.sqrt();
            let (ax, ay) = (cx + (last.0 * c - last.1 * s) * px_per_um, cy + (last.0 * s + last.1 * c) * px_per_um);
            let (bx, by) = (cx + (u * c - v * s) * px_per_um, cy + (u * s + v * c) * px_per_um);
            canvas.line(ax, ay, bx, by, ink_w * 0.6, INK, 0.8);
            last = (u, v);
        }
    }
}

/// Un membre de la vue microscope : sa forme et sa part de la biomasse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub form: MicrobeForm,
    pub share: f32,
}

/// Champ de microscope : un disque d'eau sur la feuille, des cellules tirées
/// des écotypes présents dans les proportions de leur biomasse, une barre
/// d'échelle de 5 µm. Renvoie aussi la position de chaque cellule dessinée
/// (indice du membre, x, y).
pub fn microscope_field(members: &[Member], seed: u64, width: usize, height: usize) -> (Canvas, Vec<(usize, f32, f32)>) {
    let mut canvas = Canvas::paper(width, height, seed);
    let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
    let radius = (width.min(height) as f32) * 0.46;
    // Lavis d'eau dans l'oculaire.
    for y in 0..height {
        for x in 0..width {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt();
            if d < radius {
                let k = 0.35 + 0.15 * fbm(seed ^ 0x77, x as f32 / 50.0, y as f32 / 50.0);
                canvas.wash(x, y, WATER, k);
            }
        }
    }
    let px_per_um = radius / 30.0;
    let mut placed: Vec<(usize, f32, f32, f32)> = Vec::new();
    let total: f32 = members.iter().map(|m| m.share.max(0.0)).sum();
    if total > 0.0 {
        let mut r = DrawRng::new(seed ^ 0x6D1C);
        // De 25 à 60 cellules selon la diversité.
        let count = (40 + 8 * members.len()).min(90);
        let mut attempts = 0;
        while placed.len() < count && attempts < count * 30 {
            attempts += 1;
            let mut pick = r.unit() * total;
            let mut idx = 0;
            for (i, m) in members.iter().enumerate() {
                pick -= m.share.max(0.0);
                if pick <= 0.0 {
                    idx = i;
                    break;
                }
            }
            let f = &members[idx].form;
            let ext = f.extent_um() * px_per_um * 0.5 + 0.5 * f.width_um * px_per_um + 3.0;
            let a = r.unit() * std::f32::consts::TAU;
            let d = radius * r.unit().sqrt() * 0.9;
            let (x, y) = (cx + d * a.cos(), cy + d * a.sin());
            if (x - cx).hypot(y - cy) + ext > radius * 0.98 {
                continue;
            }
            if placed.iter().any(|&(_, px, py, pe)| (px - x).hypot(py - y) < pe + ext) {
                continue;
            }
            placed.push((idx, x, y, ext));
        }
        for &(idx, x, y, _) in &placed {
            let angle = DrawRng::new(seed ^ (x.to_bits() as u64) << 7 ^ y.to_bits() as u64).unit() * std::f32::consts::TAU;
            draw(&mut canvas, &members[idx].form, x, y, px_per_um, angle);
        }
    }
    // Bord de l'oculaire en double filet.
    for (rr, w) in [(radius, 2.4), (radius + 6.0, 1.0)] {
        let n = 240;
        for k in 0..n {
            let a0 = k as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / n as f32 * std::f32::consts::TAU;
            canvas.line(cx + rr * a0.cos(), cy + rr * a0.sin(), cx + rr * a1.cos(), cy + rr * a1.sin(), w, INK, 1.0);
        }
    }
    canvas.scale_bar(cx + radius * 0.35, cy + radius * 0.82, MICROSCOPE_BAR_UM * px_per_um);
    (canvas, placed.into_iter().map(|(i, x, y, _)| (i, x, y)).collect())
}

/// Longueur de la barre d'échelle de la vue microscope, µm.
pub const MICROSCOPE_BAR_UM: f32 = 5.0;

/// Longueur « ronde » (1, 2 ou 5 × 10ⁿ µm) proche de `target_um`.
pub fn nice_length(target_um: f32) -> f32 {
    let e = target_um.max(1e-6).log10().floor();
    let base = 10f32.powf(e);
    let m = target_um / base;
    base * if m >= 5.0 {
        5.0
    } else if m >= 2.0 {
        2.0
    } else {
        1.0
    }
}

/// Figure d'une espèce seule, pour une fiche : une cellule au centre et une
/// barre d'échelle dont la longueur (µm) est renvoyée pour la légende.
pub fn figure(f: &MicrobeForm, width: usize, height: usize) -> (Canvas, f32) {
    let mut canvas = Canvas::paper(width, height, f.seed);
    let px_per_um = (width as f32 * 0.6 / (f.extent_um() + f.width_um * 2.0)).min(height as f32 * 0.5 / (f.width_um * 2.5));
    draw(&mut canvas, f, width as f32 / 2.0, height as f32 * 0.45, px_per_um, -0.25);
    let bar = nice_length(width as f32 * 0.2 / px_per_um);
    canvas.scale_bar(width as f32 * 0.08, height as f32 * 0.88, bar * px_per_um);
    (canvas, bar)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traits(lineage: u32, oxygenic: bool) -> MicrobeTraits {
        MicrobeTraits {
            lineage,
            signature: if oxygenic { 1 << 6 } else { 1 },
            pigment_rgb: oxygenic.then_some([120, 160, 90]),
            gene_count: 6,
            phototroph: oxygenic,
            oxygenic,
        }
    }

    #[test]
    fn forms_are_deterministic_and_follow_traits() {
        let a = form(&traits(3, false), 42);
        assert_eq!(a, form(&traits(3, false), 42));
        let filaments = (0..50).filter(|&l| form(&traits(l, true), 42).shape == Shape::Filament).count();
        assert!(filaments > 25, "{filaments}");
        assert!(form(&traits(3, true), 42).thylakoids);
    }

    #[test]
    fn microscope_draws_in_proportion() {
        let members = [Member { form: form(&traits(1, false), 7), share: 0.8 }, Member { form: form(&traits(2, true), 7), share: 0.2 }];
        let (c1, placed) = microscope_field(&members, 11, 320, 240);
        let (c2, _) = microscope_field(&members, 11, 320, 240);
        assert_eq!(c1.checksum(), c2.checksum());
        assert!(placed.len() >= 30, "{}", placed.len());
        let first = placed.iter().filter(|p| p.0 == 0).count() as f32 / placed.len() as f32;
        assert!(first > 0.5, "{first}");
    }

    #[test]
    fn figure_is_quick() {
        let f = form(&traits(5, true), 1);
        let t = std::time::Instant::now();
        let (c, bar) = figure(&f, 512, 384);
        assert!(t.elapsed().as_millis() < 500);
        assert!([1.0, 2.0, 5.0, 10.0, 20.0, 0.5, 0.2].contains(&bar), "{bar}");
        assert_eq!(nice_length(3.4), 2.0);
        assert_eq!(c.pixels.len(), 512 * 384 * 4);
    }
}
