//! Palettes des calques de données : perceptuellement uniformes et lisibles
//! par les daltoniens (document Direction artistique : viridis pour les
//! quantités, bleu-rouge divergent pour les écarts, posées en lavis sous
//! l'encre), et couleurs de l'Atlas naturaliste.

/// Couleurs de l'Atlas naturaliste (section « Palette » de la DA retenue).
pub mod atlas {
    pub const PAPER: [u8; 3] = [0xEC, 0xE2, 0xC9];
    pub const WATER_WASH: [u8; 3] = [0xB9, 0xC6, 0xBF];
    pub const OCHRE: [u8; 3] = [0xC7, 0x9A, 0x55];
    pub const PLANT_GREEN: [u8; 3] = [0x8A, 0x9A, 0x5B];
    pub const VERMILION: [u8; 3] = [0x9A, 0x3B, 0x22];
    pub const SEPIA_INK: [u8; 3] = [0x4A, 0x33, 0x22];
}

/// Mode de vision choisi dans les réglages d'accessibilité.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisionMode {
    #[default]
    Standard,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    HighContrast,
}

impl VisionMode {
    pub fn from_index(i: i64) -> Self {
        match i {
            1 => Self::Protanopia,
            2 => Self::Deuteranopia,
            3 => Self::Tritanopia,
            4 => Self::HighContrast,
            _ => Self::Standard,
        }
    }
}

/// Familles de palettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKind {
    /// Quantité : viridis (cividis en contraste élevé).
    Sequential,
    /// Écart à une référence : bleu-rouge (orange-violet pour les tritans).
    Diverging,
    /// Catégories (plaques, guildes) : Okabe-Ito.
    Categorical,
}

const VIRIDIS: [u32; 10] = [0x440154, 0x482878, 0x3E4989, 0x31688E, 0x26828E, 0x1F9E89, 0x35B779, 0x6DCD59, 0xB4DE2C, 0xFDE725];
const CIVIDIS: [u32; 10] = [0x00204D, 0x00336F, 0x39486B, 0x575C6D, 0x707173, 0x8A8779, 0xA69D75, 0xC4B56C, 0xE4CF5B, 0xFFEA46];
const RDBU: [u32; 11] = [0x053061, 0x2166AC, 0x4393C3, 0x92C5DE, 0xD1E5F0, 0xF7F7F7, 0xFDDBC7, 0xF4A582, 0xD6604D, 0xB2182B, 0x67001F];
const PUOR: [u32; 11] = [0x2D004B, 0x542788, 0x8073AC, 0xB2ABD2, 0xD8DAEB, 0xF7F7F7, 0xFEE0B6, 0xFDB863, 0xE08214, 0xB35806, 0x7F3B08];
/// Okabe et Ito (2008), sans le noir qui se confond avec l'encre.
const OKABE_ITO: [u32; 8] = [0xE69F00, 0x56B4E9, 0x009E73, 0xF0E442, 0x0072B2, 0xD55E00, 0xCC79A7, 0x999999];
/// Variante pour les tritans : on écarte les paires bleu/vert et jaune/rose.
const TRITAN_SAFE: [u32; 8] = [0xD55E00, 0x0072B2, 0x999999, 0xCC79A7, 0x332288, 0xE69F00, 0x117733, 0x882255];

fn rgb(c: u32) -> [f32; 3] {
    [((c >> 16) & 0xFF) as f32 / 255.0, ((c >> 8) & 0xFF) as f32 / 255.0, (c & 0xFF) as f32 / 255.0]
}

fn ramp(stops: &[u32], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len() - 2);
    let f = t - i as f32;
    let (a, b) = (rgb(stops[i]), rgb(stops[i + 1]));
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

/// Couleur d'une valeur normalisée (0 à 1) ; pour les catégories, `t` est
/// l'indice de la catégorie divisé par 255.
pub fn colour(kind: PaletteKind, mode: VisionMode, t: f32) -> [f32; 3] {
    match kind {
        PaletteKind::Sequential => ramp(if mode == VisionMode::HighContrast { &CIVIDIS } else { &VIRIDIS }, t),
        PaletteKind::Diverging => ramp(if mode == VisionMode::Tritanopia { &PUOR } else { &RDBU }, t),
        PaletteKind::Categorical => {
            let set = if mode == VisionMode::Tritanopia { &TRITAN_SAFE } else { &OKABE_ITO };
            let i = (t * 255.0).round() as usize;
            rgb(set[i % set.len()])
        }
    }
}

/// Table de 256 couleurs RGBA8, à poser dans une texture 256 × 1.
pub fn lut(kind: PaletteKind, mode: VisionMode) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let c = colour(kind, mode, i as f32 / 255.0);
        out.extend(c.iter().map(|x| (x * 255.0).round() as u8));
        out.push(255);
    }
    out
}

/// Luminance relative (WCAG), pour vérifier les contrastes.
pub fn luminance(c: [f32; 3]) -> f32 {
    let lin = |x: f32| if x <= 0.039_28 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// Couleur d'un corps noir à la température `t` (K), normalisée à 1 sur son
/// canal le plus fort : la teinte de la lumière de l'étoile (approximation de
/// Tanner Helland, ajustée sur les tables CIE 1964).
pub fn blackbody(t_k: f64) -> [f32; 3] {
    let t = (t_k / 100.0).clamp(10.0, 400.0);
    let r = if t <= 66.0 { 255.0 } else { 329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2) };
    let g = if t <= 66.0 { 99.470_802_586_1 * t.ln() - 161.119_568_166_1 } else { 288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    let c = [r.clamp(0.0, 255.0), g.clamp(0.0, 255.0), b.clamp(0.0, 255.0)];
    let m = c[0].max(c[1]).max(c[2]).max(1.0);
    [(c[0] / m) as f32, (c[1] / m) as f32, (c[2] / m) as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_palettes_grow_in_lightness() {
        for mode in [VisionMode::Standard, VisionMode::HighContrast] {
            let mut last = -1.0;
            for i in 0..=20 {
                let l = luminance(colour(PaletteKind::Sequential, mode, i as f32 / 20.0));
                assert!(l > last, "{mode:?} non monotone en {i}");
                last = l;
            }
        }
    }

    #[test]
    fn diverging_is_light_in_the_middle_and_dark_at_both_ends() {
        let mid = luminance(colour(PaletteKind::Diverging, VisionMode::Standard, 0.5));
        assert!(mid > 0.9);
        assert!(luminance(colour(PaletteKind::Diverging, VisionMode::Standard, 0.0)) < 0.1);
        assert!(luminance(colour(PaletteKind::Diverging, VisionMode::Standard, 1.0)) < 0.1);
        assert_eq!(lut(PaletteKind::Diverging, VisionMode::Tritanopia).len(), 1024);
    }

    #[test]
    fn ink_reads_on_paper() {
        let paper = luminance(atlas::PAPER.map(|x| x as f32 / 255.0));
        let ink = luminance(atlas::SEPIA_INK.map(|x| x as f32 / 255.0));
        // Contraste WCAG d'au moins 7:1 pour le texte (niveau AAA).
        assert!((paper + 0.05) / (ink + 0.05) > 7.0);
    }

    #[test]
    fn star_colours() {
        let sun = blackbody(5772.0);
        assert!(sun[0] >= sun[2] && sun[2] > 0.8);
        let red_dwarf = blackbody(3200.0);
        assert!(red_dwarf[2] < 0.6 && red_dwarf[0] == 1.0);
        let hot = blackbody(10000.0);
        assert!(hot[2] == 1.0 && hot[0] < 1.0);
    }
}
