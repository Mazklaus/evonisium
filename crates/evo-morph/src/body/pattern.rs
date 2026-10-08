//! Motifs de la robe (document Rendu du vivant, « Matériaux ») : un modèle
//! de réaction-diffusion de Turing, celui qui explique les rayures et les
//! taches des poissons et des félins (Kondo et Miura, 2010), ici dans sa
//! forme de Gray-Scott (Pearson, 1993), réglé par deux paramètres
//! génétiques (alimentation F, disparition k) et calculé une fois en
//! texture enroulée sur le corps.

use super::plan::Pattern;
use crate::canvas::DrawRng;

/// Taille de la texture : le long du corps, puis autour.
pub const PATTERN_W: usize = 64;
pub const PATTERN_H: usize = 32;
/// Pas de temps : assez pour que les motifs se fixent.
const STEPS: usize = 1800;

/// Texture du motif, valeurs 0 à 1, enroulée dans les deux sens.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternTexture {
    pub values: Vec<f32>,
    pub scale: f32,
}

impl PatternTexture {
    /// Fait pousser le motif. Sans contraste, rien n'est calculé.
    pub fn grow(p: &Pattern, seed: u64) -> PatternTexture {
        let n = PATTERN_W * PATTERN_H;
        if p.contrast <= 0.0 {
            return PatternTexture { values: vec![0.0; n], scale: 1.0 };
        }
        let (du, dv) = (0.2097f32, 0.105f32);
        let (f, k) = (p.feed.clamp(0.01, 0.08), p.kill.clamp(0.04, 0.07));
        let mut u = vec![1.0f32; n];
        let mut v = vec![0.0f32; n];
        let mut rng = DrawRng::new(seed ^ 0x7475_7269_6E67);
        // Germes : de petites taches de l'inhibiteur, au hasard de la lignée.
        for _ in 0..10 {
            let cx = (rng.unit() * PATTERN_W as f32) as usize;
            let cy = (rng.unit() * PATTERN_H as f32) as usize;
            for dy in 0..6 {
                for dx in 0..6 {
                    let i = ((cy + dy) % PATTERN_H) * PATTERN_W + (cx + dx) % PATTERN_W;
                    u[i] = 0.5 + 0.02 * (rng.unit() - 0.5);
                    v[i] = 0.25 + 0.02 * (rng.unit() - 0.5);
                }
            }
        }
        let (mut u2, mut v2) = (u.clone(), v.clone());
        let at = |x: isize, y: isize| -> usize {
            let x = x.rem_euclid(PATTERN_W as isize) as usize;
            let y = y.rem_euclid(PATTERN_H as isize) as usize;
            y * PATTERN_W + x
        };
        for _ in 0..STEPS {
            for y in 0..PATTERN_H as isize {
                for x in 0..PATTERN_W as isize {
                    let i = at(x, y);
                    let lap = |a: &[f32]| a[at(x - 1, y)] + a[at(x + 1, y)] + a[at(x, y - 1)] + a[at(x, y + 1)] - 4.0 * a[i];
                    let uvv = u[i] * v[i] * v[i];
                    u2[i] = (u[i] + du * lap(&u) - uvv + f * (1.0 - u[i])).clamp(0.0, 1.0);
                    v2[i] = (v[i] + dv * lap(&v) + uvv - (f + k) * v[i]).clamp(0.0, 1.0);
                }
            }
            std::mem::swap(&mut u, &mut u2);
            std::mem::swap(&mut v, &mut v2);
        }
        let max = v.iter().cloned().fold(0.0f32, f32::max);
        let values = if max > 1e-6 {
            v.iter()
                .map(|x| {
                    let t = (x / max - 0.35) / 0.3;
                    t.clamp(0.0, 1.0)
                })
                .collect()
        } else {
            vec![0.0; n]
        };
        PatternTexture { values, scale: p.scale.max(0.25) }
    }

    /// Valeur en (u, v), u le long du corps (0 à 1), v autour (0 à 1).
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let x = u * self.scale * PATTERN_W as f32;
        let y = v * PATTERN_H as f32;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let g = |dx: i64, dy: i64| {
            let xi = (x0 as i64 + dx).rem_euclid(PATTERN_W as i64) as usize;
            let yi = (y0 as i64 + dy).rem_euclid(PATTERN_H as i64) as usize;
            self.values[yi * PATTERN_W + xi]
        };
        let a = g(0, 0) + (g(1, 0) - g(0, 0)) * fx;
        let b = g(0, 1) + (g(1, 1) - g(0, 1)) * fx;
        a + (b - a) * fy
    }

    /// Part de la surface couverte par le motif.
    pub fn coverage(&self) -> f32 {
        self.values.iter().sum::<f32>() / self.values.len() as f32
    }
}
