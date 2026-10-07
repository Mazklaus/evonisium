//! Mesure des budgets de calcul (étapes 1 et 2).
//!
//! Le rapport produit compare les mesures aux budgets de la section
//! « Faisabilité et performances » du document Vision.

use crate::report::format_years;
use crate::world::{PhaseTimings, Seeding, World, WorldConfig};
use evo_core::rng::{rng_for, Stream};
use evo_genetics::popgen::fixation_probability;
use evo_genetics::mutate;
use evo_life::{growth_rates, Conditions, Phenotype, Physiology};
use evo_planet::generate::generate;
use evo_planet::{PlanetParams, WaterPool, WATER_POOL_COUNT};
use std::fmt::Write;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct BenchOptions {
    pub levels: Vec<u32>,
    pub steps: u32,
    pub warmup: u32,
    pub seed: u64,
}

const MB: f64 = 1024.0 * 1024.0;

fn read_proc(path: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find(|l| l.starts_with(key)).map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
}

/// Pic de mémoire résidente du processus, Mo (Linux seulement).
pub fn peak_rss_mb() -> Option<f64> {
    read_proc("/proc/self/status", "VmHWM").and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok()).map(|kb| kb / 1024.0)
}

/// Évaluations génétiques complètes par seconde sur un fil : mutation,
/// phénotype, taux de croissance dans un milieu.
fn genetic_throughput(seed: u64) -> (f64, f64) {
    let cfg = WorldConfig::new(seed, 0);
    let world = World::new(cfg);
    let genome = world.minimal_cell();
    let physio = Physiology::default();
    let params = world.config.mutation.clone();
    let cond = Conditions { temperature_k: 295.0, uv_w_m2: 5.0, light_kj: 1e5 };
    let mut chem = [0.0; WATER_POOL_COUNT];
    chem[WaterPool::Dic as usize] = 8.0;
    chem[WaterPool::H2 as usize] = 8e-4;
    chem[WaterPool::Doc as usize] = 1e-3;
    let mut rng = rng_for(seed, Stream::Benchmark, &[]);
    let n = 400_000;
    let t = Instant::now();
    let mut acc = 0.0;
    for _ in 0..n {
        let g = mutate(&genome, &params, &mut rng).genome;
        let p = Phenotype::from_genome(&g, &physio);
        acc += growth_rates(&p, &cond, &chem, &physio).r;
    }
    let evals = n as f64 / t.elapsed().as_secs_f64();
    std::hint::black_box(acc);

    let m = 4_000_000;
    let t = Instant::now();
    let mut acc = 0.0;
    for i in 0..m {
        acc += fixation_probability(1e-4 * (i % 200) as f64 - 0.01, 1e8, 1e-8);
    }
    std::hint::black_box(acc);
    (evals, m as f64 / t.elapsed().as_secs_f64())
}

struct LevelResult {
    level: u32,
    cells: usize,
    ocean_cells: usize,
    grid_seconds: f64,
    world_seconds: f64,
    populations: usize,
    distinct_genomes: usize,
    per_step: PhaseTimings,
    evals_per_step: f64,
    memory_planet: usize,
    memory_populations: usize,
    memory_genomes: usize,
    step_years: f64,
}

fn bench_level(level: u32, opts: &BenchOptions) -> LevelResult {
    let t = Instant::now();
    let grid_planet = generate(PlanetParams::earth_archean(), level, opts.seed);
    let grid_seconds = t.elapsed().as_secs_f64();
    drop(grid_planet);

    let mut cfg = WorldConfig::new(opts.seed, level);
    cfg.seeding = Seeding::AllOcean;
    let t = Instant::now();
    let mut world = World::new(cfg);
    world.seed_life();
    let world_seconds = t.elapsed().as_secs_f64();
    for _ in 0..opts.warmup {
        world.step();
    }
    let evals_before = world.stats.genetic_evaluations;
    let mut total = PhaseTimings::default();
    for _ in 0..opts.steps {
        total.add(&world.step());
    }
    let n = opts.steps.max(1);
    let per_step = total.divided(n);
    let mem = world.memory_bytes();
    let summary = world.summary();
    LevelResult {
        level,
        cells: world.planet.cells.len(),
        ocean_cells: summary.ocean_cells,
        grid_seconds,
        world_seconds,
        populations: summary.populations,
        distinct_genomes: mem.distinct_genomes,
        per_step,
        evals_per_step: (world.stats.genetic_evaluations - evals_before) as f64 / n as f64,
        memory_planet: mem.planet,
        memory_populations: mem.populations,
        memory_genomes: mem.genomes,
        step_years: world.config.step_years,
    }
}

pub fn run_benchmarks(opts: &BenchOptions) -> String {
    let threads = rayon::current_num_threads();
    let cpu = read_proc("/proc/cpuinfo", "model name").unwrap_or_else(|| "inconnu".into());
    let ram = read_proc("/proc/meminfo", "MemTotal")
        .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
        .map(|kb| format!("{:.1} Go", kb / MB))
        .unwrap_or_else(|| "inconnue".into());

    let (evals_per_s, kimura_per_s) = genetic_throughput(opts.seed);
    let results: Vec<LevelResult> = opts.levels.iter().map(|&l| bench_level(l, opts)).collect();

    let mut out = String::new();
    let _ = writeln!(out, "# Mesures des budgets de calcul — étape 2\n");
    let _ = writeln!(out, "Rapport produit par `evonisium bench`. Chaque mesure est une moyenne sur {} pas après {} pas de mise en route, avec des cellules minimales déposées dans toutes les cellules océaniques (charge maximale). Le pas comprend désormais la planète vivante : tectonique (un pas sur dix à 100 000 ans), climat d'équilibre, boîtes chimiques globales, tunnel stochastique et transfert horizontal.\n", opts.steps, opts.warmup);
    let _ = writeln!(
        out,
        "Machine de mesure : {cpu}, {threads} fils, {ram} de mémoire. La machine cible du document Vision a 8 coeurs et 16 Go, dont 2 réservés à l'affichage : la simulation en a 6.\n"
    );

    let _ = writeln!(out, "## Débit génétique (un fil)\n");
    let _ = writeln!(out, "| Opération | Par seconde |\n|---|---|");
    let _ = writeln!(out, "| Évaluation complète d'un mutant (mutation, phénotype, taux de croissance) | {evals_per_s:.2e} |");
    let _ = writeln!(out, "| Probabilité de fixation de Kimura | {kimura_per_s:.2e} |");
    let _ = writeln!(out, "| Estimation sur 6 coeurs (évaluations complètes) | {:.2e} |\n", evals_per_s * 6.0);

    let _ = writeln!(out, "## Monde microbien complet\n");
    let _ = writeln!(
        out,
        "| Grille | Cellules | Cellules océaniques | Populations | Génomes distincts | Génération de la planète | Pas complet | dont planète | dont écologie | dont évolution | dont migration | Évaluations génétiques par pas | Évaluations par seconde |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in &results {
        let step = r.per_step.total().as_secs_f64();
        let _ = writeln!(
            out,
            "| niveau {} | {} | {} | {} | {} | {:.2} s | {:.0} ms | {:.0} ms | {:.0} ms | {:.0} ms | {:.0} ms | {:.0} | {:.2e} |",
            r.level,
            r.cells,
            r.ocean_cells,
            r.populations,
            r.distinct_genomes,
            r.grid_seconds + r.world_seconds,
            step * 1e3,
            r.per_step.planet.as_secs_f64() * 1e3,
            r.per_step.ecology.as_secs_f64() * 1e3,
            r.per_step.evolution.as_secs_f64() * 1e3,
            r.per_step.migration.as_secs_f64() * 1e3,
            r.evals_per_step,
            r.evals_per_step / step
        );
    }

    let _ = writeln!(out, "\n## Vitesse du temps\n");
    let _ = writeln!(out, "Le document Vision vise environ 1 million d'années par seconde dans le monde microbien. La vitesse dépend du pas planétaire choisi : un pas plus long coûte le même calcul mais l'évolution y est plus grossière (une substitution au plus par population et par pas).\n");
    let _ = writeln!(out, "| Grille | Pas mesuré | Vitesse avec ce pas | Pas nécessaire pour 1 Ma/s ici | Même chose estimée sur 6 coeurs |\n|---|---|---|---|---|");
    for r in &results {
        let step = r.per_step.total().as_secs_f64();
        let needed = 1e6 * step;
        let needed_8 = needed * threads as f64 / 6.0;
        let _ = writeln!(
            out,
            "| niveau {} | {} | {} par seconde | {} | {} |",
            r.level,
            format_years(r.step_years),
            format_years(r.step_years / step),
            format_years(needed),
            format_years(needed_8)
        );
    }

    let _ = writeln!(out, "\n## Mémoire\n");
    let _ = writeln!(
        out,
        "| Grille | Monde physique | Populations | Génomes et phénotypes | Octets par génome distinct |\n|---|---|---|---|---|"
    );
    for r in &results {
        let _ = writeln!(
            out,
            "| niveau {} | {:.1} Mo | {:.1} Mo | {:.1} Mo | {:.0} |",
            r.level,
            r.memory_planet as f64 / MB,
            r.memory_populations as f64 / MB,
            r.memory_genomes as f64 / MB,
            r.memory_genomes as f64 / r.distinct_genomes.max(1) as f64
        );
    }
    if let Some(peak) = peak_rss_mb() {
        let _ = writeln!(out, "\nPic de mémoire résidente du processus pendant toutes les mesures : {peak:.0} Mo.");
    }
    if let Some(r) = results.last() {
        let per_genome = r.memory_genomes as f64 / r.distinct_genomes.max(1) as f64;
        let _ = writeln!(
            out,
            "\nExtrapolation : 20 000 espèces × 50 génotypes de ce format occuperaient {:.2} Go. Les génomes de l'étape 1 sont ceux de cellules minimales (quelques gènes) ; un animal complexe en aura des milliers, d'où le stockage en différences par rapport au génome de référence prévu par le document Vision.",
            20_000.0 * 50.0 * per_genome / (MB * 1024.0)
        );
    }
    out
}
