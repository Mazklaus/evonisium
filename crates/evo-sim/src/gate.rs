//! Porte de l'étape 3 (document Vision, périmètre consolidé) : l'oxygène
//! s'accumule par la photosynthèse, sans script, sur les six mondes de la
//! vague 1, au moins pour certaines graines (le monde désertique peut échouer
//! pour certaines) ; carbone, phosphore et électrons sont conservés ; la
//! vitesse est mesurée à la résolution normale ; une partie se rejoue à
//! l'identique depuis sa graine et son registre d'ordres, quel que soit le
//! parcours de la caméra.
//!
//! Pour chaque monde, la partie démarre avec la cellule minimale près des
//! sources hydrothermales et tourne jusqu'à ce que l'oxygène de l'air reste
//! au-dessus du seuil pendant la durée demandée, ou jusqu'à la durée maximale.
//! Rien dans le moteur ne vise l'oxygène : il n'a qu'une source, la
//! photosynthèse oxygénique apparue par évolution dans les cellules, et ses
//! puits sont tenus processus par processus. Le rapport dit, pour chaque
//! monde, si l'accélérateur a dû agir.

use crate::history::Sample;
use crate::orders::{Intervention, OrderKind};
use crate::report::{causes, format_years};
use crate::world::{World, WorldConfig};
use evo_core::events::{EventKind, Origin};
use evo_genetics::GenomeChangeCause;
use evo_life::metabolism::PHOTOSYNTHESIS_STAGES;
use evo_planet::{Gas, PlanetParams};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct GateOptions {
    /// Graines essayées dans l'ordre, jusqu'à la première qui franchit la
    /// porte.
    pub seeds: Vec<u64>,
    pub level: u32,
    pub step_years: f64,
    pub max_years: f64,
    /// Seuil d'oxygène (fraction molaire) et durée pendant laquelle il doit
    /// tenir.
    pub oxygen_threshold: f64,
    pub hold_years: f64,
    /// Mondes à passer (clés de [`PlanetParams::by_key`]) ; vide : les six.
    pub worlds: Vec<String>,
    /// Dossier où écrire l'historique et les événements de chaque monde.
    pub out_dir: Option<PathBuf>,
    /// Durée d'un tour d'évolution, si elle diffère de celle par défaut
    /// (voir `EvolutionParams::round_years` ; 0 : un tour par pas).
    pub round_years: Option<f64>,
    /// Probabilité d'innovation, si elle diffère de celle par défaut (voir
    /// `EvolutionParams::innovation_probability`).
    pub innovation_probability: Option<f64>,
    /// Ordre d'éviction sous le plafond, s'il diffère de celui par défaut.
    pub eviction: Option<crate::world::Eviction>,
    /// Pas fixe : sans allongement aux périodes calmes.
    pub fixed_step: bool,
}

impl Default for GateOptions {
    fn default() -> Self {
        Self {
            seeds: vec![2026, 7, 42],
            level: 6,
            step_years: 200_000.0,
            max_years: 5.0e8,
            oxygen_threshold: 1e-4,
            hold_years: 50e6,
            worlds: Vec::new(),
            out_dir: None,
            round_years: None,
            innovation_probability: None,
            eviction: None,
            fixed_step: false,
        }
    }
}

/// Résultat d'un monde pour une graine.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldResult {
    pub key: String,
    pub seed: u64,
    pub name: String,
    /// Fils de calcul utilisés.
    pub threads: usize,
    pub cells: usize,
    pub years: f64,
    pub seconds: f64,
    /// Dates de chaque étape du chemin vers la photosynthèse.
    pub stage_years: [Option<f64>; 5],
    /// Origine de l'étape oxygénique (moteur ou accélérateur).
    pub oxygenic_origin: Option<Origin>,
    pub rhodopsin_years: Option<f64>,
    /// Premières dates où l'O₂ dépasse 10⁻⁶ et le seuil de la porte.
    pub oxygen_trace_years: Option<f64>,
    pub oxygen_threshold_years: Option<f64>,
    pub final_sample: Sample,
    pub max_o2: f64,
    pub accelerator_steps: u64,
    pub accelerator_fixed: u64,
    pub causes: String,
    pub snowballs: usize,
    pub plate_reorganisations: usize,
    pub carbon_error: f64,
    pub phosphorus_error: f64,
    pub electron_error: f64,
    /// Biomasse des eaux douces en fin de partie, mol de carbone, et cellules
    /// d'eaux douces colonisées.
    pub lake_biomass: f64,
    pub lake_cells: usize,
    /// Correction des électrons sur les flux extrapolés, relative à la
    /// production photosynthétique d'O₂ (voir `close_electrons`).
    pub redox_correction: f64,
    /// Pouvoir oxydant déplacé par les prélèvements freinés, relatif à la
    /// production photosynthétique d'O₂.
    pub redox_throttled: f64,
    /// Part de ce pouvoir oxydant qu'aucun flux n'a pu reprendre, même unité.
    pub redox_unpaired: f64,
    /// Part des cellules du vivant peuplées qui dépassaient le plafond de
    /// populations avant éviction : sur toute la partie, et sur ses 100
    /// derniers pas (monde mûr).
    pub saturated_share: f64,
    pub saturated_share_late: f64,
    /// Part des cellules peuplées qui ont perdu une population établie (plus
    /// grande qu'un fondateur), sur la partie et sur ses 100 derniers pas.
    pub established_share: f64,
    pub established_share_late: f64,
    /// Même part sur les 100 derniers pas, en ne comptant que les
    /// populations établies qui croissaient encore.
    pub growing_share_late: f64,
    /// Mutants innovants apparus et ajoutés par l'accélérateur, essais et
    /// réussites du tunnel.
    pub innovations_drawn: u64,
    pub innovations_accelerated: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    pub oxygen_budget: evo_planet::geochem::OxygenBudget,
    pub replay_ok: bool,
    pub passed: bool,
}

/// Vérifie le rejeu sur un monde : même graine, mêmes ordres, même état.
pub fn replay_check(params: &PlanetParams, seed: u64, level: u32, step_years: f64, steps: usize) -> bool {
    let mut cfg = WorldConfig::with_planet(params.clone(), seed, level);
    cfg.step_years = step_years;
    let mut a = World::new(cfg.clone());
    a.orders.submit(0.0, OrderKind::SeedLife);
    a.orders.submit(step_years * 3.0, OrderKind::SetStepYears(step_years / 2.0));
    a.orders.submit(step_years * 5.0, OrderKind::Intervene(Intervention::Fertilize { cell: 0, radius_km: 2000.0, moles_p: 1e13 }));
    a.orders.submit(step_years * 6.0, OrderKind::Intervene(Intervention::Eruption { cell: 0, gas: Gas::Co2, moles: 1e16 }));
    let n = a.planet.grid.len() as u32;
    for i in 0..steps {
        // Seule la première partie a une caméra qui bouge.
        a.set_interest(Some(crate::observation::InterestZone { center_cell: (i as u32 * 131) % n, radius_km: 3000.0, zoom_band: 2 }));
        a.step();
    }
    let mut b = World::replay(cfg, &a.orders.log());
    for _ in 0..steps {
        b.step();
    }
    a.state_hash() == b.state_hash() && a.years == b.years
}

pub fn run_world(key: &str, seed: u64, opts: &GateOptions) -> WorldResult {
    let params = PlanetParams::by_key(key).unwrap_or_else(|| panic!("monde inconnu : {key}"));
    let start = Instant::now();
    let mut cfg = WorldConfig::with_planet(params.clone(), seed, opts.level);
    cfg.step_years = opts.step_years;
    if let Some(r) = opts.round_years {
        cfg.evolution.round_years = (r > 0.0).then_some(r);
    }
    if let Some(p) = opts.innovation_probability {
        cfg.evolution.innovation_probability = p;
    }
    if let Some(e) = opts.eviction {
        cfg.eviction = e;
    }
    if opts.fixed_step {
        cfg.adaptive_step = None;
    }
    let mut world = World::new(cfg);
    world.seed_life();
    let mut above_since: Option<f64> = None;
    let (mut trace, mut reached, mut max_o2) = (None, None, 0.0f64);
    // Cellules peuplées et saturées des derniers pas.
    let mut late: std::collections::VecDeque<(u64, u64, u64, u64)> = std::collections::VecDeque::new();
    while world.years < opts.max_years {
        let st = &world.stats;
        let before = (st.occupied_cell_steps, st.saturated_cell_steps, st.established_eviction_cell_steps, st.growing_eviction_cell_steps);
        world.step();
        let st = &world.stats;
        late.push_back((
            st.occupied_cell_steps - before.0,
            st.saturated_cell_steps - before.1,
            st.established_eviction_cell_steps - before.2,
            st.growing_eviction_cell_steps - before.3,
        ));
        if late.len() > 100 {
            late.pop_front();
        }
        let o2 = world.planet.reservoirs.mixing_ratio(Gas::O2);
        if world.stats.steps.is_multiple_of(500) {
            eprintln!(
                "  {key} (graine {seed}) : {}, O₂ {o2:.1e}, étape {}, {:.0} s",
                format_years(world.years),
                world.progress.best_stage,
                start.elapsed().as_secs_f64()
            );
        }
        max_o2 = max_o2.max(o2);
        if o2 >= 1e-6 && trace.is_none() {
            trace = Some(world.years);
        }
        if o2 >= opts.oxygen_threshold {
            reached.get_or_insert(world.years);
            let since = *above_since.get_or_insert(world.years);
            if world.years - since >= opts.hold_years {
                break;
            }
        } else {
            above_since = None;
        }
        if world.communities.iter().all(Vec::is_empty) {
            break;
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let events = &world.events.events;
    let oxygenic_origin = events.iter().find_map(|e| match &e.kind {
        EventKind::Innovation { stage: 4, pathway, .. } if *pathway == evo_life::metabolism::PHOTOSYNTHESIS_PATHWAY => Some(e.origin),
        _ => None,
    });
    let rhodopsin_years = events.iter().find_map(|e| match &e.kind {
        EventKind::Innovation { pathway, .. } if *pathway == evo_life::metabolism::RHODOPSIN_PATHWAY => Some(e.years),
        _ => None,
    });
    let held = above_since.is_some_and(|since| world.years - since >= opts.hold_years);
    let carbon_error = world.carbon_balance_error();
    let phosphorus_error = world.phosphorus_balance_error();
    let electron_error = world.electron_balance_error();
    let replay_ok = replay_check(&params, seed, 3, 50_000.0, 12);
    let budget = world.planet.reservoirs.oxygen;
    let photosynthetic = budget.photosynthesis > 0.0 && world.progress.best_stage >= 4;
    // Sur des milliards d'années, les arrondis des sous-pas des boîtes
    // s'accumulent : on tolère un millionième.
    let passed = held && photosynthetic && carbon_error < 1e-6 && phosphorus_error < 1e-6 && electron_error < 1e-6 && replay_ok;

    if let Some(dir) = &opts.out_dir {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join(format!("{key}-{seed}-historique.tsv")), world.history.to_tsv());
        let notable: String = world
            .events
            .to_tsv()
            .lines()
            .enumerate()
            .filter(|(i, l)| *i == 0 || !(l.contains("\tnouvelle lignée\t") || l.contains("\textinction de lignée\t")))
            .map(|(_, l)| format!("{l}\n"))
            .collect();
        let _ = std::fs::write(dir.join(format!("{key}-{seed}-evenements.tsv")), notable);
    }

    let summary = world.summary();
    WorldResult {
        key: key.to_string(),
        seed,
        name: params.name.clone(),
        threads: rayon::current_num_threads(),
        cells: world.planet.cells.len(),
        years: world.years,
        seconds,
        stage_years: world.progress.stage_years,
        oxygenic_origin,
        rhodopsin_years,
        oxygen_trace_years: trace,
        oxygen_threshold_years: reached,
        final_sample: world.sample(),
        max_o2,
        accelerator_steps: world.stats.accelerator_steps,
        accelerator_fixed: world.stats.fixed_changes_by_cause[GenomeChangeCause::Accelerator.index()],
        causes: causes(&world.stats.fixed_changes_by_cause),
        snowballs: world.events.count(|k| matches!(k, EventKind::Snowball { starts: true, .. })),
        plate_reorganisations: world.events.count(|k| matches!(k, EventKind::PlateReorganisation { .. })),
        carbon_error,
        phosphorus_error,
        electron_error,
        lake_biomass: summary.lake_biomass,
        lake_cells: summary.lake_cells,
        redox_correction: world.stats.redox_correction / world.planet.reservoirs.oxygen.photosynthesis.max(1.0),
        redox_throttled: world.stats.redox_throttled / world.planet.reservoirs.oxygen.photosynthesis.max(1.0),
        redox_unpaired: world.stats.redox_unpaired / world.planet.reservoirs.oxygen.photosynthesis.max(1.0),
        saturated_share: world.stats.saturated_cell_steps as f64 / world.stats.occupied_cell_steps.max(1) as f64,
        saturated_share_late: late.iter().map(|l| l.1).sum::<u64>() as f64 / late.iter().map(|l| l.0).sum::<u64>().max(1) as f64,
        established_share: world.stats.established_eviction_cell_steps as f64 / world.stats.occupied_cell_steps.max(1) as f64,
        established_share_late: late.iter().map(|l| l.2).sum::<u64>() as f64 / late.iter().map(|l| l.0).sum::<u64>().max(1) as f64,
        growing_share_late: late.iter().map(|l| l.3).sum::<u64>() as f64 / late.iter().map(|l| l.0).sum::<u64>().max(1) as f64,
        innovations_drawn: world.stats.innovations_drawn,
        innovations_accelerated: world.stats.innovations_accelerated,
        tunnel_attempts: world.stats.tunnel_attempts,
        tunnel_successes: world.stats.tunnel_successes,
        oxygen_budget: budget,
        replay_ok,
        passed,
    }
}

fn date(y: Option<f64>) -> String {
    y.map_or("—".into(), format_years)
}

/// Passe la porte et renvoie le rapport en Markdown.
pub fn run_gate(opts: &GateOptions, mut progress: impl FnMut(&WorldResult)) -> (String, Vec<WorldResult>) {
    let keys: Vec<String> =
        if opts.worlds.is_empty() { PlanetParams::KEYS.iter().map(|k| k.to_string()).collect() } else { opts.worlds.clone() };
    let mut results = Vec::new();
    for k in &keys {
        for &seed in &opts.seeds {
            let r = run_world(k, seed, opts);
            progress(&r);
            let passed = r.passed;
            results.push(r);
            if passed {
                break;
            }
        }
    }
    (format_gate(opts, &results), results)
}

/// Fraction d'O₂ de l'air actuel (niveau actuel de l'atmosphère, PAL).
const PRESENT_O2: f64 = 0.21;

pub fn format_gate(opts: &GateOptions, results: &[WorldResult]) -> String {
    let mut out = String::new();
    let mut keys: Vec<&str> = Vec::new();
    for r in results {
        if !keys.contains(&r.key.as_str()) {
            keys.push(&r.key);
        }
    }
    let world_passed = |k: &str| results.iter().any(|r| r.key == k && r.passed);
    let passed_worlds = keys.iter().filter(|k| world_passed(k)).count();
    let desert_only = keys.iter().all(|k| world_passed(k) || *k == "desert");
    let _ = writeln!(out, "# Porte de l'oxygène (étapes 3 et 4) : l'oxygène s'accumule sur les six mondes\n");
    let _ = writeln!(
        out,
        "Rapport produit par `evonisium porte`. Grille de niveau {} ({} cellules physiques, vie un niveau en dessous), pas demandé de {}{}, au plus {} par partie, {}, graines essayées dans l'ordre {:?} jusqu'à la première qui franchit la porte. Critère : la fraction d'O₂ de l'air dépasse {:.0e} et s'y maintient {} ; l'O₂ vient de la photosynthèse oxygénique apparue par évolution ; carbone, phosphore et électrons conservés à 10⁻⁶ près ; la partie se rejoue à l'identique depuis sa graine et ses ordres, avec une caméra qui bouge dans l'une et pas dans l'autre.\n",
        opts.level,
        results.first().map_or(0, |r| r.cells),
        format_years(opts.step_years),
        if opts.fixed_step { " (fixe)" } else { " (allongé jusqu'à 3 fois aux périodes calmes)" },
        format_years(opts.max_years),
        match opts.round_years.map_or(crate::evolution::EvolutionParams::default().round_years, |r| (r > 0.0).then_some(r)) {
            Some(r) => format!("un tour d'évolution « apparition puis fixation » par tranche de {} du pas", format_years(r)),
            None => "un seul tour d'évolution par pas".into(),
        },
        opts.seeds,
        opts.oxygen_threshold,
        format_years(opts.hold_years)
    );
    let verdict = if passed_worlds == keys.len() {
        format!("porte franchie sur les {} mondes", keys.len())
    } else if desert_only {
        format!("porte franchie sur {passed_worlds} mondes sur {} ; le monde désertique échoue pour les graines essayées, ce que la porte tolère", keys.len())
    } else {
        format!("porte non franchie : {passed_worlds} mondes sur {} (voir le détail)", keys.len())
    };
    let _ = writeln!(out, "**Verdict : {verdict}**\n");

    let _ = writeln!(out, "## Chemin vers la photosynthèse\n");
    let _ = writeln!(out, "Dates de première apparition sur la planète, depuis le dépôt de la cellule minimale.\n");
    let _ = writeln!(
        out,
        "| Monde | Graine | {} | {} | {} | {} | Rhodopsine | Origine de l'étape oxygénique |\n|---|---|---|---|---|---|---|---|---|---|",
        PHOTOSYNTHESIS_STAGES[1], PHOTOSYNTHESIS_STAGES[2], PHOTOSYNTHESIS_STAGES[3], PHOTOSYNTHESIS_STAGES[4]
    );
    for r in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            r.name,
            r.seed,
            date(r.stage_years[1]),
            date(r.stage_years[2]),
            date(r.stage_years[3]),
            date(r.stage_years[4]),
            date(r.rhodopsin_years),
            r.oxygenic_origin.map_or("—", |o| o.label())
        );
    }

    let _ = writeln!(out, "\n## Oxygène et planète\n");
    let _ = writeln!(
        out,
        "« PAL » : niveau actuel de l'atmosphère terrestre (21 %). Pour comparaison, la littérature place l'O₂ du Protérozoïque, après la Grande Oxydation, entre 0,1 % et 10 % du niveau actuel selon les auteurs (Lyons, Reinhard et Planavsky, 2014, *Nature* 506 ; Planavsky et coll., 2014, *Science* 346), et celui de l'Archéen sous 10⁻⁵ PAL. La partie s'arrête dès que la porte est franchie : l'O₂ final est celui de la fin du maintien, pas un plateau à l'équilibre.\n"
    );
    let _ = writeln!(
        out,
        "| Monde | Graine | Durée simulée | O₂ > 10⁻⁶ | O₂ > seuil | O₂ final | O₂ final (PAL) | O₂ maximal | CO₂ final | Température | Glace | Océan | Eaux douces colonisées | Biomasse des eaux douces | Verdict |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for r in results {
        let s = &r.final_sample;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {:.1e} | {:.2} % | {:.1e} | {:.0} Pa | {:.0} K | {:.0} % | {:.0} % | {} cellules | {:.1e} mol C | {} |",
            r.name,
            r.seed,
            format_years(r.years),
            date(r.oxygen_trace_years),
            date(r.oxygen_threshold_years),
            s.o2_mixing,
            100.0 * s.o2_mixing / PRESENT_O2,
            r.max_o2,
            s.co2_pa,
            s.mean_temperature_k,
            (100.0 * s.ice_fraction).max(0.0),
            100.0 * s.ocean_fraction,
            r.lake_cells,
            r.lake_biomass,
            if r.passed { "franchie" } else { "non franchie" }
        );
    }

    let _ = writeln!(out, "\n## Budget de l'oxygène (cumulé sur la partie, mol d'O₂)\n");
    let _ = writeln!(
        out,
        "La photosynthèse oxygénique est la seule source. La couche de surface en reprend une partie (respiration, oxydation du fer et du sulfure sur place) ; le reste gagne l'air, où les puits globaux le consomment.\n"
    );
    let _ = writeln!(
        out,
        "| Monde | Graine | Photosynthèse (brut) | Libéré vers l'air | Repris en surface | Respiration profonde | Gaz réduits (H₂) | Méthane | Fer et manganèse | Plancher océanique | Roches exposées | Sulfure |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for r in results {
        let b = &r.oxygen_budget;
        let _ = writeln!(
            out,
            "| {} | {} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} | {:.2e} |",
            r.name,
            r.seed,
            b.photosynthesis,
            b.surface_release,
            b.surface_uptake,
            b.deep_respiration,
            b.reduced_gases,
            b.methane,
            b.iron_manganese,
            b.seafloor_oxidation,
            b.oxidative_weathering,
            b.sulfide
        );
    }

    let _ = writeln!(out, "\n## Aide de l'accélérateur, bilans, rejeu et vitesse\n");
    let _ = writeln!(
        out,
        "« Écart des électrons » : écart entre le pouvoir oxydant des flux de surface prolongés sur chaque pas et celui de leurs sources hydrothermales, en part de la production photosynthétique d'O₂ de la partie ; il n'est pas corrigé et ne vient que des arrondis. « Électrons freinés » : pouvoir oxydant déplacé quand une boîte vide freine un prélèvement des couches (ce qu'il alimentait est freiné avec lui), même unité ; « non repris » : la part qu'aucun flux de la couche n'a pu reprendre. « Bilan électrons » : écart du registre de flux, qui doit rester nul. La vitesse est celle de la partie entière, sur la machine de mesure.\n"
    );
    let _ = writeln!(
        out,
        "| Monde | Graine | Pas avec accélérateur | Modifications fixées grâce à lui | Modifications fixées par cause | Bilan carbone | Bilan phosphore | Bilan électrons | Écart des électrons | Électrons freinés | Non repris | Rejeu identique | Calcul | Vitesse |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for r in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {:.1e} | {:.1e} | {:.1e} | {:.1e} | {:.1} % | {:.2} % | {} | {:.0} s sur {} fils | {} par seconde |",
            r.name,
            r.seed,
            r.accelerator_steps,
            r.accelerator_fixed,
            r.causes,
            r.carbon_error,
            r.phosphorus_error,
            r.electron_error,
            r.redox_correction,
            100.0 * r.redox_throttled,
            100.0 * r.redox_unpaired,
            if r.replay_ok { "oui" } else { "non" },
            r.seconds,
            r.threads,
            format_years(r.years / r.seconds.max(1e-9))
        );
    }

    let _ = writeln!(out, "\n## Garde-fous du plafond de populations et du tunnel\n");
    let _ = writeln!(
        out,
        "Plafond de populations par cellule du vivant : jamais la dernière d'une guilde (voie principale) ; parmi les autres, on évince d'abord la moins abondante (règle par défaut), ou, avec `--eviction invasion`, la plus basse fitness d'invasion (taux de croissance dans la communauté résidente). « Dépassent le plafond » : part des cellules peuplées qui dépassaient le plafond avant éviction, sur toute la partie et sur ses 100 derniers pas. « Saturées » : part des cellules peuplées qui ont perdu une population établie (plus que la biomasse d'un fondateur), même découpage (au-delà de 2 % sur monde mûr, la règle est à revoir) ; « dont en croissance » : celles dont la population évincée croissait encore. Innovations : mutants innovants (de novo, duplication suivie de divergence) apparus sur la partie, tirés selon une loi de Poisson, dont ceux que l'accélérateur a ajoutés. Tunnel : essais (un par mutant innovant qui ne se fixe pas seul) et réussites, au taux de Weissman et coll. (2009) tiré selon une loi de Poisson.\n"
    );
    let _ = writeln!(
        out,
        "| Monde | Graine | Dépassent le plafond (partie) | Dépassent le plafond (100 derniers pas) | Saturées (partie) | Saturées (100 derniers pas) | dont en croissance | Innovations apparues | dont accélérateur | Essais du tunnel | Réussites |\n|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for r in results {
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} % | {:.1} % | {:.2} % | {:.2} % | {:.2} % | {} | {} | {} | {} |",
            r.name,
            r.seed,
            100.0 * r.saturated_share,
            100.0 * r.saturated_share_late,
            100.0 * r.established_share,
            100.0 * r.established_share_late,
            100.0 * r.growing_share_late,
            r.innovations_drawn,
            r.innovations_accelerated,
            r.tunnel_attempts,
            r.tunnel_successes
        );
    }
    out
}
