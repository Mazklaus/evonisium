//! Textures de données et calques de lecture.
//!
//! Chaque cellule occupe un texel de textures de 256 × ⌈n/256⌉ (256 × 161 à
//! 40 962 cellules). Trois textures RGBA en flottants suffisent aux calques
//! de l'étape 3 ; un calque n'est qu'un choix de texture, de canal, de
//! transformation, de bornes et de palette, sans reconstruire de géométrie.
//!
//! | Texture | R | G | B | A |
//! |---|---|---|---|---|
//! | 0 | hauteur / mer (m) | température (K) | log₁₀(biomasse + 1) | O₂ dissous (mol·m⁻³) |
//! | 1 | pigment R | pigment G | pigment B | drapeaux |
//! | 2 | plaque | guilde (rang) | glace (0-1) | part de l'espèce choisie |
//!
//! Drapeaux : 1 océan, 2 glace, 4 pigment, 8 source hydrothermale,
//! 16 aire de l'espèce choisie, 32 lignée suivie présente.

use crate::frame::Frame;
use crate::palette::PaletteKind;

pub const TEX_WIDTH: usize = 256;
pub const TEXTURE_COUNT: usize = 3;

pub const FLAG_OCEAN: u32 = 1;
pub const FLAG_ICE: u32 = 2;
pub const FLAG_PIGMENT: u32 = 4;
pub const FLAG_VENT: u32 = 8;
pub const FLAG_FOCUS: u32 = 16;
pub const FLAG_MARKED: u32 = 32;

/// Guildes distinguées par couleur ; les autres partagent la dernière.
pub const GUILD_CATEGORIES: usize = 7;

pub fn texture_size(cells: usize) -> (usize, usize) {
    (TEX_WIDTH, cells.div_ceil(TEX_WIDTH).max(1))
}

/// Coordonnées de texture du centre du texel d'une cellule.
pub fn texel_uv(cell: usize, cells: usize) -> [f32; 2] {
    let (w, h) = texture_size(cells);
    [((cell % w) as f32 + 0.5) / w as f32, ((cell / w) as f32 + 0.5) / h as f32]
}

/// Ce que le client met en avant sur le globe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Focus {
    /// Espèce dont l'aire est surlignée.
    pub lineage: Option<u32>,
    /// Lignées suivies.
    pub marked: Vec<u32>,
}

/// Les trois textures d'un pas, RGBA en flottants, ligne par ligne.
pub fn pack(frame: &Frame, focus: &Focus) -> [Vec<f32>; TEXTURE_COUNT] {
    let n = frame.cells.len();
    let (w, h) = texture_size(n);
    let mut t: [Vec<f32>; TEXTURE_COUNT] = std::array::from_fn(|_| vec![0.0; w * h * 4]);
    let guilds = frame.top_guilds(GUILD_CATEGORIES);
    for (c, cell) in frame.cells.iter().enumerate() {
        let i = c * 4;
        t[0][i] = cell.height_m;
        t[0][i + 1] = cell.temperature_k;
        t[0][i + 2] = (cell.biomass.max(0.0) + 1.0).log10();
        t[0][i + 3] = cell.oxygen;
        let mut flags = 0;
        if cell.is_ocean {
            flags |= FLAG_OCEAN;
        }
        if cell.ice_cover > 0.5 {
            flags |= FLAG_ICE;
        }
        if cell.vent {
            flags |= FLAG_VENT;
        }
        if let Some(rgb) = cell.pigment_rgb {
            flags |= FLAG_PIGMENT;
            t[1][i] = rgb[0] as f32 / 255.0;
            t[1][i + 1] = rgb[1] as f32 / 255.0;
            t[1][i + 2] = rgb[2] as f32 / 255.0;
        }
        let pops = frame.populations_of(c);
        let mut share = 0.0;
        if let Some(l) = focus.lineage {
            if let Some(p) = pops.iter().find(|p| p.lineage == l) {
                flags |= FLAG_FOCUS;
                share = if cell.biomass > 0.0 { p.biomass / cell.biomass } else { 0.0 };
            }
        }
        if pops.iter().any(|p| focus.marked.contains(&p.lineage)) {
            flags |= FLAG_MARKED;
        }
        t[1][i + 3] = flags as f32;
        t[2][i] = cell.plate as f32;
        t[2][i + 1] = if cell.dominant_guild == 0 {
            0.0
        } else {
            (guilds.iter().position(|g| g.0 == cell.dominant_guild).unwrap_or(GUILD_CATEGORIES) + 1) as f32
        };
        t[2][i + 2] = cell.ice_cover;
        t[2][i + 3] = share;
    }
    t
}

/// Transformation appliquée à la valeur brute avant la palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    Linear,
    /// Valeur à passer en log₁₀.
    Log10,
    /// Valeur déjà rangée en log₁₀ dans la texture.
    Log10Stored,
    /// Catégorie (indice entier).
    Category,
}

/// Un calque de lecture.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub key: &'static str,
    pub name_fr: &'static str,
    pub name_en: &'static str,
    pub family_fr: &'static str,
    pub unit: &'static str,
    pub texture: u8,
    pub channel: u8,
    pub transform: Transform,
    /// Bornes de la palette, dans l'unité affichée (après transformation).
    pub min: f32,
    pub max: f32,
    pub palette: PaletteKind,
    /// Décalage d'affichage (ex. kelvins → degrés Celsius).
    pub display_offset: f32,
}

impl Layer {
    /// Valeur normalisée (0 à 1) d'une valeur brute, comme la calcule le
    /// shader.
    pub fn normalise(&self, raw: f32) -> f32 {
        let v = match self.transform {
            Transform::Log10 => raw.max(1e-30).log10(),
            _ => raw,
        };
        ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Graduations de la légende : (position 0-1, libellé).
    pub fn ticks(&self, count: usize) -> Vec<(f32, String)> {
        if self.transform == Transform::Category {
            return Vec::new();
        }
        (0..count)
            .map(|i| {
                let f = i as f32 / (count - 1).max(1) as f32;
                let v = self.min + f * (self.max - self.min);
                let label = match self.transform {
                    Transform::Log10 | Transform::Log10Stored => crate::format::power_of_ten(10f64.powf(v as f64)),
                    _ => crate::format::number(v as f64 + self.display_offset as f64, 0),
                };
                (f, label)
            })
            .collect()
    }
}

/// Calques de l'étape 3 : physique, chimie et vie (document Fonctionnalités,
/// tableau des étapes). Les vents, courants, nuages et précipitations
/// attendent les sorties d'affichage de la planète (volet moteur).
pub fn catalogue() -> Vec<Layer> {
    use PaletteKind::*;
    use Transform::*;
    vec![
        Layer {
            key: "relief",
            name_fr: "Relief et bathymétrie",
            name_en: "Relief and bathymetry",
            family_fr: "Physique",
            unit: "m",
            texture: 0,
            channel: 0,
            transform: Linear,
            min: -6000.0,
            max: 6000.0,
            palette: Diverging,
            display_offset: 0.0,
        },
        Layer {
            key: "temperature",
            name_fr: "Température moyenne",
            name_en: "Mean temperature",
            family_fr: "Physique",
            unit: "°C",
            texture: 0,
            channel: 1,
            transform: Linear,
            min: 233.15,
            max: 333.15,
            palette: Diverging,
            display_offset: -273.15,
        },
        Layer {
            key: "glace",
            name_fr: "Glaces",
            name_en: "Ice",
            family_fr: "Physique",
            unit: "fraction",
            texture: 2,
            channel: 2,
            transform: Linear,
            min: 0.0,
            max: 1.0,
            palette: Sequential,
            display_offset: 0.0,
        },
        Layer {
            key: "plaques",
            name_fr: "Plaques tectoniques",
            name_en: "Tectonic plates",
            family_fr: "Physique",
            unit: "",
            texture: 2,
            channel: 0,
            transform: Category,
            min: 0.0,
            max: 1.0,
            palette: Categorical,
            display_offset: 0.0,
        },
        Layer {
            key: "oxygene",
            name_fr: "Oxygène dissous",
            name_en: "Dissolved oxygen",
            family_fr: "Chimie",
            unit: "mol·m⁻³",
            texture: 0,
            channel: 3,
            transform: Log10,
            min: -9.0,
            max: -1.0,
            palette: Sequential,
            display_offset: 0.0,
        },
        Layer {
            key: "biomasse",
            name_fr: "Biomasse",
            name_en: "Biomass",
            family_fr: "Vie",
            unit: "mol C",
            texture: 0,
            channel: 2,
            transform: Log10Stored,
            min: 0.0,
            max: 12.0,
            palette: Sequential,
            display_offset: 0.0,
        },
        Layer {
            key: "guildes",
            name_fr: "Guilde dominante",
            name_en: "Dominant guild",
            family_fr: "Vie",
            unit: "",
            texture: 2,
            channel: 1,
            transform: Category,
            min: 0.0,
            max: 1.0,
            palette: Categorical,
            display_offset: 0.0,
        },
        Layer {
            key: "espece",
            name_fr: "Aire de l'espèce choisie",
            name_en: "Range of the chosen species",
            family_fr: "Vie",
            unit: "part de la biomasse",
            texture: 2,
            channel: 3,
            transform: Linear,
            min: 0.0,
            max: 1.0,
            palette: Sequential,
            display_offset: 0.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_sim::{World, WorldConfig};

    #[test]
    fn texture_layout() {
        assert_eq!(texture_size(40_962), (256, 161));
        assert_eq!(texture_size(10_242), (256, 41));
        let uv = texel_uv(257, 40_962);
        assert!((uv[0] - 1.5 / 256.0).abs() < 1e-6 && (uv[1] - 1.5 / 161.0).abs() < 1e-6);
    }

    #[test]
    fn packing_sets_flags_and_focus() {
        let mut w = World::new(WorldConfig::new(3, 3));
        w.seed_life();
        w.step();
        let f = Frame::from_world(&w);
        let lineage = f.populations[0].lineage;
        let t = pack(&f, &Focus { lineage: Some(lineage), marked: vec![lineage] });
        let flags = |c: usize| t[1][c * 4 + 3] as u32;
        let ocean = (0..f.cells.len()).filter(|&c| flags(c) & FLAG_OCEAN != 0).count();
        assert_eq!(ocean, f.cells.iter().filter(|c| c.is_ocean).count());
        let focus = (0..f.cells.len()).filter(|&c| flags(c) & FLAG_FOCUS != 0).count();
        assert_eq!(focus, f.range_of(lineage).len());
        assert!(focus > 0);
        assert!((0..f.cells.len()).all(|c| (0.0..=1.0).contains(&t[2][c * 4 + 3])));
    }

    #[test]
    fn layers_have_sane_ranges() {
        for l in catalogue() {
            assert!(l.max > l.min, "{}", l.key);
            assert!((l.texture as usize) < TEXTURE_COUNT && l.channel < 4);
        }
        let t = catalogue().into_iter().find(|l| l.key == "temperature").unwrap();
        assert_eq!(t.normalise(283.15), 0.5);
        assert_eq!(t.ticks(3)[1].1, "10");
    }
}
