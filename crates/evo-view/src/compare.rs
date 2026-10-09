//! Comparaison de deux histoires de la même planète : la partie et sa
//! branche « sans » une intervention (document Fonctionnalités, « Avec et
//! sans » : nombre d'espèces, jalons, carte), ou deux points de sauvegarde.

use crate::format::{self, Lang};
use crate::layers::texture_size;
use evo_sim::history::{PublishedState, Sample};

/// Une ligne du tableau : la grandeur avec et sans, et l'écart lisible.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: &'static str,
    pub label: String,
    pub with: String,
    pub without: String,
    /// Écart relatif (avec / sans − 1), ou absolu pour les températures.
    pub change: f64,
    pub change_text: String,
}

/// Un jalon : sa date dans chaque histoire (`None` : pas encore atteint).
#[derive(Clone, Debug, PartialEq)]
pub struct Milestone {
    pub key: &'static str,
    pub label: String,
    pub with: Option<f64>,
    pub without: Option<f64>,
}

/// Jalons lus dans un historique global : (clé, libellé fr, libellé en,
/// critère).
type Criterion = fn(&Sample) -> bool;
const MILESTONES: [(&str, &str, &str, Criterion); 6] = [
    ("vie", "Vie présente", "Life present", |s| s.biomass > 0.0),
    ("photo-anox", "Photosynthèse anoxygénique", "Anoxygenic photosynthesis", |s| s.photosynthesis_stage >= 2),
    ("photo-ox", "Photosynthèse oxygénique", "Oxygenic photosynthesis", |s| s.photosynthesis_stage >= 4),
    ("o2-trace", "Oxygène dans l'air (10⁻⁵)", "Oxygen in the air (10⁻⁵)", |s| s.o2_mixing >= 1e-5),
    ("o2-grand", "Grande oxydation (10⁻³)", "Great oxidation (10⁻³)", |s| s.o2_mixing >= 1e-3),
    ("o2-moderne", "Oxygène à 10 % du niveau actuel", "Oxygen at 10% of today", |s| s.o2_mixing >= 0.021),
];

/// Date du premier échantillon qui remplit le critère.
fn first(history: &[Sample], c: Criterion) -> Option<f64> {
    history.iter().find(|s| c(s)).map(|s| s.years)
}

pub fn milestones(with: &[Sample], without: &[Sample], lang: Lang) -> Vec<Milestone> {
    MILESTONES
        .iter()
        .map(|(key, fr, en, c)| Milestone {
            key,
            label: if lang == Lang::Fr { (*fr).into() } else { (*en).into() },
            with: first(with, *c),
            without: first(without, *c),
        })
        .collect()
}

fn ratio_text(with: f64, without: f64, lang: Lang) -> (f64, String) {
    if without <= 0.0 && with <= 0.0 {
        return (0.0, "=".into());
    }
    if without <= 0.0 {
        return (f64::INFINITY, "+∞".into());
    }
    let r = with / without - 1.0;
    if r.abs() < 0.005 {
        return (r, "=".into());
    }
    if with / without >= 3.0 || without / with.max(1e-300) >= 3.0 {
        return (r, format!("×{}", format::number_in(lang, with / without, 2)));
    }
    (r, format!("{}{} %", if r > 0.0 { "+" } else { "−" }, format::number_in(lang, r.abs() * 100.0, 0)))
}

/// Tableau des grandeurs globales.
pub fn rows(with: &PublishedState, without: &PublishedState, lang: Lang, celsius: bool) -> Vec<Row> {
    let fr = lang == Lang::Fr;
    let (a, b) = (&with.globals, &without.globals);
    let mut out = Vec::new();
    let mut push = |key: &'static str, fr_l: &str, en_l: &str, x: f64, y: f64, show: &dyn Fn(f64) -> String| {
        let (change, change_text) = ratio_text(x, y, lang);
        out.push(Row { key, label: if fr { fr_l.into() } else { en_l.into() }, with: show(x), without: show(y), change, change_text });
    };
    let count = |x: f64| format::number_in(lang, x, 0);
    let power = |x: f64| format::power_of_ten_in(lang, x);
    push("especes", "Espèces vivantes", "Living species", with.species.len() as f64, without.species.len() as f64, &count);
    push("lignees", "Lignées vivantes", "Living lineages", a.living_lineages as f64, b.living_lineages as f64, &count);
    push("biomasse", "Biomasse (mol C)", "Biomass (mol C)", a.biomass, b.biomass, &power);
    push("o2", "O₂ de l'air (fraction)", "Air O₂ (fraction)", a.o2_mixing, b.o2_mixing, &power);
    push("co2", "CO₂ (Pa)", "CO₂ (Pa)", a.co2_pa, b.co2_pa, &power);
    push("glace", "Glace (part de la surface)", "Ice (surface share)", a.ice_fraction, b.ice_fraction, &|x| format::percent(x, lang));
    // Température : écart absolu.
    let (t1, t2) = (a.mean_temperature_k, b.mean_temperature_k);
    let d = t1 - t2;
    out.push(Row {
        key: "temperature",
        label: if fr { "Température moyenne".into() } else { "Mean temperature".into() },
        with: format::temperature(t1, celsius, lang),
        without: format::temperature(t2, celsius, lang),
        change: d,
        change_text: if d.abs() < 0.05 {
            "=".into()
        } else {
            format!("{}{} K", if d > 0.0 { "+" } else { "−" }, format::number_in(lang, d.abs(), 1))
        },
    });
    out
}

/// Texture de comparaison (même disposition que les textures de données) :
/// R = log₁₀ du rapport des biomasses (avec / sans), G = écart de
/// température (K), B = 1 si l'espèce dominante diffère, A = 1.
pub fn texture(with: &PublishedState, without: &PublishedState) -> Vec<f32> {
    let n = with.cells.len().min(without.cells.len());
    let (w, h) = texture_size(with.cells.len());
    let mut t = vec![0.0f32; w * h * 4];
    for c in 0..n {
        let (x, y) = (&with.cells[c], &without.cells[c]);
        let i = c * 4;
        t[i] = ((x.biomass.max(0.0) + 1.0) / (y.biomass.max(0.0) + 1.0)).log10();
        t[i + 1] = x.temperature_k - y.temperature_k;
        t[i + 2] = if x.dominant_guild != y.dominant_guild { 1.0 } else { 0.0 };
        t[i + 3] = 1.0;
    }
    t
}

/// Part des cellules dont l'espèce dominante diffère.
pub fn changed_share(with: &PublishedState, without: &PublishedState) -> f64 {
    let n = with.cells.len().min(without.cells.len());
    if n == 0 {
        return 0.0;
    }
    let k = (0..n).filter(|&c| with.cells[c].dominant_guild != without.cells[c].dominant_guild).count();
    k as f64 / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(years: f64, stage: u8, o2: f64) -> Sample {
        Sample { years, photosynthesis_stage: stage, o2_mixing: o2, biomass: 1.0, ..Default::default() }
    }

    #[test]
    fn milestones_and_ratios() {
        let with = vec![sample(0.0, 0, 0.0), sample(1e6, 4, 1e-4), sample(2e6, 4, 2e-3)];
        let without = vec![sample(0.0, 0, 0.0), sample(1e6, 2, 0.0), sample(2e6, 4, 1e-5)];
        let m = milestones(&with, &without, Lang::Fr);
        let ox = m.iter().find(|x| x.key == "photo-ox").unwrap();
        assert_eq!((ox.with, ox.without), (Some(1e6), Some(2e6)));
        let great = m.iter().find(|x| x.key == "o2-grand").unwrap();
        assert_eq!((great.with, great.without), (Some(2e6), None));
        assert_eq!(ratio_text(1.0, 1.0, Lang::Fr).1, "=");
        assert_eq!(ratio_text(1.5, 1.0, Lang::Fr).1, "+50 %");
        assert!(ratio_text(10.0, 1.0, Lang::Fr).1.starts_with('×'));
    }
}
