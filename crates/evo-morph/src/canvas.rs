//! Toile de l'Atlas : papier grainé, lavis, encre et hachures, en RGBA8.
//!
//! Tout est déterministe : le grain du papier et les irrégularités du lavis
//! viennent d'un bruit de valeur haché à partir d'une graine.

/// Couleurs de la DA retenue (document Direction artistique, « Palette »).
pub const PAPER: [f32; 3] = [0xEC as f32 / 255.0, 0xE2 as f32 / 255.0, 0xC9 as f32 / 255.0];
pub const INK: [f32; 3] = [0x4A as f32 / 255.0, 0x33 as f32 / 255.0, 0x22 as f32 / 255.0];
pub const WATER: [f32; 3] = [0xB9 as f32 / 255.0, 0xC6 as f32 / 255.0, 0xBF as f32 / 255.0];
pub const OCHRE: [f32; 3] = [0xC7 as f32 / 255.0, 0x9A as f32 / 255.0, 0x55 as f32 / 255.0];
pub const VERMILION: [f32; 3] = [0x9A as f32 / 255.0, 0x3B as f32 / 255.0, 0x22 as f32 / 255.0];

#[inline]
pub fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Générateur minimal et reproductible (affichage seulement : il ne touche
/// jamais aux flux de hasard de la simulation).
#[derive(Clone, Debug)]
pub struct DrawRng(u64);

impl DrawRng {
    pub fn new(seed: u64) -> Self {
        Self(mix64(seed ^ 0xA71A_5000))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = mix64(self.0);
        self.0
    }
    /// Uniforme dans [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
}

fn lattice(seed: u64, x: i32, y: i32) -> f32 {
    let h = mix64(seed ^ ((x as u32 as u64) << 32 | y as u32 as u64));
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Bruit de valeur lissé, dans [0, 1].
pub fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let a = lattice(seed, xi, yi);
    let b = lattice(seed, xi + 1, yi);
    let c = lattice(seed, xi, yi + 1);
    let d = lattice(seed, xi + 1, yi + 1);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

/// Bruit fractal (quatre octaves), dans [0, 1].
pub fn fbm(seed: u64, x: f32, y: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut f = 1.0;
    for o in 0..4 {
        sum += amp * value_noise(seed.wrapping_add(o), x * f, y * f);
        amp *= 0.5;
        f *= 2.0;
    }
    sum / 0.9375
}

#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    /// RGBA8, ligne par ligne, depuis le haut.
    pub pixels: Vec<u8>,
}

impl Canvas {
    /// Feuille de papier grainé.
    pub fn paper(width: usize, height: usize, seed: u64) -> Self {
        let mut c = Self { width, height, pixels: vec![255; width * height * 4] };
        for y in 0..height {
            for x in 0..width {
                let g = fbm(seed, x as f32 / 3.0, y as f32 / 3.0) - 0.5;
                let fibre = value_noise(seed ^ 0xF1B, x as f32 / 40.0, y as f32 / 1.5) - 0.5;
                let k = 1.0 + 0.05 * g + 0.02 * fibre;
                c.set(x, y, [PAPER[0] * k, PAPER[1] * k, PAPER[2] * k], 1.0);
            }
        }
        c
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> [f32; 3] {
        let i = (y * self.width + x) * 4;
        [self.pixels[i] as f32 / 255.0, self.pixels[i + 1] as f32 / 255.0, self.pixels[i + 2] as f32 / 255.0]
    }

    /// Pose une couleur avec une opacité (mélange normal).
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, c: [f32; 3], alpha: f32) {
        if x >= self.width || y >= self.height || alpha <= 0.0 {
            return;
        }
        let i = (y * self.width + x) * 4;
        let a = alpha.min(1.0);
        for (k, &ck) in c.iter().enumerate() {
            let old = self.pixels[i + k] as f32 / 255.0;
            self.pixels[i + k] = ((old + (ck - old) * a).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        self.pixels[i + 3] = 255;
    }

    /// Lavis : mélange multiplicatif, comme un pigment transparent sur le
    /// papier (les passes successives foncent).
    #[inline]
    pub fn wash(&mut self, x: usize, y: usize, c: [f32; 3], alpha: f32) {
        if x >= self.width || y >= self.height || alpha <= 0.0 {
            return;
        }
        let old = self.get(x, y);
        let a = alpha.min(1.0);
        let m = [old[0] * c[0], old[1] * c[1], old[2] * c[2]];
        self.set(x, y, [old[0] + (m[0] - old[0]) * a, old[1] + (m[1] - old[1]) * a, old[2] + (m[2] - old[2]) * a], 1.0);
    }

    /// Trait d'encre antialiasé d'épaisseur `w` pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: [f32; 3], alpha: f32) {
        let (minx, maxx) = (x0.min(x1) - w - 1.0, x0.max(x1) + w + 1.0);
        let (miny, maxy) = (y0.min(y1) - w - 1.0, y0.max(y1) + w + 1.0);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in miny.max(0.0) as usize..=(maxy.max(0.0) as usize).min(self.height.saturating_sub(1)) {
            for x in minx.max(0.0) as usize..=(maxx.max(0.0) as usize).min(self.width.saturating_sub(1)) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (x0 + t * dx - px, y0 + t * dy - py);
                let d = (qx * qx + qy * qy).sqrt();
                let cover = (w * 0.5 + 0.5 - d).clamp(0.0, 1.0);
                if cover > 0.0 {
                    self.set(x, y, c, alpha * cover);
                }
            }
        }
    }

    /// Barre d'échelle de naturaliste : un filet, deux butées et une
    /// subdivision en cinq.
    pub fn scale_bar(&mut self, x: f32, y: f32, length_px: f32) {
        self.line(x, y, x + length_px, y, 1.6, INK, 1.0);
        for k in 0..=5 {
            let xx = x + length_px * k as f32 / 5.0;
            let h = if k == 0 || k == 5 { 6.0 } else { 3.0 };
            self.line(xx, y - h, xx, y + h, 1.2, INK, 1.0);
        }
        for k in 0..5 {
            if k % 2 == 0 {
                let x0 = x + length_px * k as f32 / 5.0;
                self.line(x0, y + 1.5, x0 + length_px / 5.0, y + 1.5, 2.0, INK, 0.9);
            }
        }
    }

    /// Cadre de planche en double filet.
    pub fn double_frame(&mut self, margin: f32) {
        let (w, h) = (self.width as f32, self.height as f32);
        for (m, t) in [(margin, 2.0), (margin + 5.0, 0.8)] {
            self.line(m, m, w - m, m, t, INK, 1.0);
            self.line(w - m, m, w - m, h - m, t, INK, 1.0);
            self.line(w - m, h - m, m, h - m, t, INK, 1.0);
            self.line(m, h - m, m, m, t, INK, 1.0);
        }
    }

    /// Somme de contrôle (tests de déterminisme).
    pub fn checksum(&self) -> u64 {
        self.pixels.chunks(8).fold(0xCBF2_9CE4_8422_2325, |h, c| mix64(h ^ c.iter().fold(0u64, |a, &b| a << 8 | b as u64)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_is_deterministic_and_close_to_the_palette() {
        let a = Canvas::paper(64, 32, 9);
        let b = Canvas::paper(64, 32, 9);
        assert_eq!(a.checksum(), b.checksum());
        assert_ne!(a.checksum(), Canvas::paper(64, 32, 10).checksum());
        let mean: f32 = (0..32).flat_map(|y| (0..64).map(move |x| (x, y))).map(|(x, y)| a.get(x, y)[0]).sum::<f32>() / (64.0 * 32.0);
        assert!((mean - PAPER[0]).abs() < 0.03);
    }

    #[test]
    fn ink_lines_darken() {
        let mut c = Canvas::paper(40, 40, 1);
        c.line(5.0, 20.0, 35.0, 20.0, 2.0, INK, 1.0);
        assert!(c.get(20, 20)[0] < 0.4);
        assert!(c.get(20, 5)[0] > 0.8);
    }
}
