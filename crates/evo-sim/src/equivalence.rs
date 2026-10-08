//! Test d'équivalence des pas : la même partie, au pas de 100 ka et au pas
//! de 200 ka, doit donner la même évolution aux fluctuations près (date de
//! montée de l'O₂, substitutions par million d'années, nombre de guildes).
//!
//! Exigé par le document d'architecture (« Correction sur monde mûr ») pour
//! accepter la porte de l'étape 3 au pas de 200 ka.

use crate::report::format_years;
use crate::world::{World, WorldConfig};
use evo_planet::{Gas, PlanetParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct EquivalenceOptions {
    pub world: String,
    pub seeds: Vec<u64>,
    pub level: u32,
    pub steps_years: Vec<f64>,
    pub years: f64,
    pub oxygen_threshold: f64,
    /// Durée d'un tour d'évolution, si elle diffère de celle par défaut
    /// (voir `EvolutionParams::round_years` ; 0 : un tour par pas).
    pub round_years: Option<f64>,
}

impl Default for EquivalenceOptions {
    fn default() -> Self {
        Self {
            world: "terre".into(),
            seeds: vec![2026, 7, 42],
            level: 5,
            steps_years: vec![100_000.0, 200_000.0],
            years: 150e6,
            oxygen_threshold: 1e-4,
            round_years: None,
        }
    }
}

/// Une partie de la comparaison.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquivalenceRun {
    pub world: String,
    pub seed: u64,
    pub level: u32,
    pub step_years: f64,
    pub years: f64,
    pub seconds: f64,
    /// Premières dates où l'O₂ dépasse 10⁻⁶ et le seuil.
    pub oxygen_trace_years: Option<f64>,
    pub oxygen_threshold_years: Option<f64>,
    /// Date de l'étape oxygénique de la photosynthèse.
    pub oxygenic_years: Option<f64>,
    pub substitutions: u64,
    /// Substitutions avantageuses (coefficient de sélection positif).
    pub adaptive: u64,
    /// Guildes (voies principales) distinctes sur la planète en fin de
    /// partie, guildes par cellule peuplée en moyenne, et combinaisons de
    /// voies distinctes (signatures).
    pub guilds: usize,
    pub signatures: usize,
    pub guilds_per_cell: f64,
    pub populations: usize,
    pub final_o2: f64,
}

impl EquivalenceRun {
    pub fn substitutions_per_ma(&self) -> f64 {
        self.substitutions as f64 / (self.years / 1e6).max(1e-9)
    }

    pub fn adaptive_per_ma(&self) -> f64 {
        self.adaptive as f64 / (self.years / 1e6).max(1e-9)
    }
}

pub fn run_one(opts: &EquivalenceOptions, seed: u64, step_years: f64) -> EquivalenceRun {
    let params = PlanetParams::by_key(&opts.world).unwrap_or_else(|| panic!("monde inconnu : {}", opts.world));
    let start = Instant::now();
    let mut cfg = WorldConfig::with_planet(params, seed, opts.level);
    cfg.step_years = step_years;
    if let Some(r) = opts.round_years {
        cfg.evolution.round_years = (r > 0.0).then_some(r);
    }
    let mut world = World::new(cfg);
    world.seed_life();
    let (mut trace, mut reached) = (None, None);
    while world.years < opts.years {
        world.step();
        let o2 = world.planet.reservoirs.mixing_ratio(Gas::O2);
        if o2 >= 1e-6 {
            trace.get_or_insert(world.years);
        }
        if o2 >= opts.oxygen_threshold {
            reached.get_or_insert(world.years);
        }
        if world.communities.iter().all(Vec::is_empty) {
            break;
        }
    }
    let guilds: BTreeSet<Option<u8>> = world.communities.iter().flatten().map(|p| p.phenotype.main_pathway()).collect();
    let signatures: BTreeSet<u32> = world.communities.iter().flatten().map(|p| p.signature()).collect();
    let populated: Vec<usize> = world
        .communities
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| c.iter().map(|p| p.phenotype.main_pathway()).collect::<BTreeSet<_>>().len())
        .collect();
    EquivalenceRun {
        world: opts.world.clone(),
        seed,
        level: opts.level,
        step_years,
        years: world.years,
        seconds: start.elapsed().as_secs_f64(),
        oxygen_trace_years: trace,
        oxygen_threshold_years: reached,
        oxygenic_years: world.progress.stage_years[4],
        substitutions: world.stats.substitutions,
        adaptive: world.stats.adaptive_substitutions,
        guilds: guilds.len(),
        signatures: signatures.len(),
        guilds_per_cell: populated.iter().sum::<usize>() as f64 / populated.len().max(1) as f64,
        populations: world.communities.iter().map(Vec::len).sum(),
        final_o2: world.planet.reservoirs.mixing_ratio(Gas::O2),
    }
}

/// Moyenne et écart type (échantillon) ; `None` si une valeur manque.
fn stats(values: &[Option<f64>]) -> Option<(f64, f64)> {
    let v: Option<Vec<f64>> = values.iter().copied().collect();
    let v = v?;
    if v.is_empty() {
        return None;
    }
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = if v.len() > 1 { v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0) } else { 0.0 };
    Some((mean, var.sqrt()))
}

fn date(y: Option<f64>) -> String {
    y.map_or("—".into(), format_years)
}

/// Rapport de comparaison. Deux pas sont jugés équivalents sur une grandeur
/// quand l'écart de leurs moyennes ne dépasse pas deux fois l'écart type
/// entre graines (le plus grand des deux pas).
pub fn format_equivalence(opts: &EquivalenceOptions, runs: &[EquivalenceRun]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Équivalence des pas de 100 ka et de 200 ka (étape 3)\n");
    let _ = writeln!(
        out,
        "Rapport produit par `evonisium equivalence`. Monde « {} », grille de niveau {} (vie un niveau en dessous), graines {:?}, {} simulés par partie, chaque partie jouée aux pas de {}, {}. Les deux pas partent de la même graine, mais le tirage des mutations dépend du numéro du pas : les parties divergent dans le détail, et l'équivalence se juge sur la moyenne des graines. Critère : l'écart des moyennes entre pas ne dépasse pas deux fois l'écart type entre graines. « Guildes » : voies principales (la voie utilisable qui porte le plus d'efficacité enzymatique) ; les combinaisons de voies sont données pour information.\n",
        opts.world,
        opts.level,
        opts.seeds,
        format_years(opts.years),
        opts.steps_years.iter().map(|y| format_years(*y)).collect::<Vec<_>>().join(" et "),
        match opts.round_years.map_or(crate::evolution::EvolutionParams::default().round_years, |r| (r > 0.0).then_some(r)) {
            Some(r) => format!("l'évolution enchaînant un tour « apparition puis fixation » par tranche de {} du pas", format_years(r)),
            None => "avec un seul tour « apparition puis fixation » par pas".into(),
        }
    );
    let _ = writeln!(
        out,
        "| Graine | Pas | O₂ > 10⁻⁶ | O₂ > {:.0e} | Photosynthèse oxygénique | Substitutions par Ma | dont avantageuses | Guildes (planète) | Guildes par cellule | Combinaisons de voies | Populations | O₂ final | Calcul |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|",
        opts.oxygen_threshold
    );
    for r in runs {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {:.0} | {:.0} | {} | {:.2} | {} | {} | {:.1e} | {:.0} s |",
            r.seed,
            format_years(r.step_years),
            date(r.oxygen_trace_years),
            date(r.oxygen_threshold_years),
            date(r.oxygenic_years),
            r.substitutions_per_ma(),
            r.adaptive_per_ma(),
            r.guilds,
            r.guilds_per_cell,
            r.signatures,
            r.populations,
            r.final_o2,
            r.seconds
        );
    }

    type Metric = (&'static str, fn(&EquivalenceRun) -> Option<f64>, bool);
    let metrics: [Metric; 6] = [
        ("Montée de l'O₂ au-dessus du seuil", |r| r.oxygen_threshold_years, true),
        ("Photosynthèse oxygénique", |r| r.oxygenic_years, true),
        ("Substitutions par Ma", |r| Some(r.substitutions_per_ma()), false),
        ("Substitutions avantageuses par Ma", |r| Some(r.adaptive_per_ma()), false),
        ("Guildes (planète)", |r| Some(r.guilds as f64), false),
        ("Guildes par cellule", |r| Some(r.guilds_per_cell), false),
    ];
    let _ = writeln!(out, "\n## Comparaison\n");
    let mut steps = opts.steps_years.clone();
    steps.dedup();
    let head: Vec<String> = steps.iter().map(|s| format!("Pas de {} (moyenne ± écart type)", format_years(*s))).collect();
    let _ =
        writeln!(out, "| Grandeur | {} | Écart des moyennes | Verdict |\n|---|{}---|---|", head.join(" | "), "---|".repeat(steps.len()));
    let mut all_ok = true;
    for (label, f, is_date) in metrics {
        let per_step: Vec<Option<(f64, f64)>> = steps
            .iter()
            .map(|&s| {
                let vals: Vec<Option<f64>> = runs.iter().filter(|r| r.step_years == s).map(f).collect();
                stats(&vals)
            })
            .collect();
        let show = |m: f64| if is_date { format_years(m) } else { format!("{m:.2}") };
        let cells: Vec<String> = per_step
            .iter()
            .map(|p| p.map_or("non atteinte par toutes les graines".into(), |(m, sd)| format!("{} ± {}", show(m), show(sd))))
            .collect();
        let (gap, verdict) = match (per_step.first().copied().flatten(), per_step.last().copied().flatten()) {
            (Some((m1, s1)), Some((m2, s2))) => {
                let gap = (m2 - m1).abs();
                let ok = gap <= 2.0 * s1.max(s2) + 1e-12;
                all_ok &= ok;
                (show(gap), if ok { "équivalent" } else { "écart" })
            }
            _ => {
                all_ok = false;
                ("—".into(), "incomplet")
            }
        };
        let _ = writeln!(out, "| {label} | {} | {gap} | {verdict} |", cells.join(" | "));
    }
    let _ = writeln!(
        out,
        "\n**Verdict : {}**",
        if all_ok {
            "les deux pas donnent la même évolution aux fluctuations près"
        } else {
            "écart sur au moins une grandeur (voir le tableau)"
        }
    );
    out
}
