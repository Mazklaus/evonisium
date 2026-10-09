//! Outil en ligne de commande.
//!
//!   evonisium run   [--world CLÉ] [--seed N] [--level L] [--steps S] [--step-years Y] [--every K] [--out DOSSIER]
//!   evonisium porte [--worlds terre,ocean,...] [--seeds 2026,7,42] [--save-results DOSSIER] [--assemble DOSSIER] [--level L] [--step-years Y] [--max-years Y] [--out FICHIER] [--data DOSSIER]
//!   evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]
//!   evonisium empreinte [--world CLÉ] [--seed N] [--level L] [--steps S]
//!   evonisium equivalence [--world CLÉ] [--seeds 2026,7,42] [--level L] [--steps-years 100000,200000] [--years Y] [--save-results DOSSIER] [--assemble DOSSIER] [--out FICHIER]
//!
//! Mondes : terre, ocean, desert, super-terre, petite, sans-lune.

use evo_planet::PlanetParams;
use evo_sim::bench::{run_benchmarks, BenchOptions};
use evo_sim::equivalence::{format_equivalence, run_one, EquivalenceOptions, EquivalenceRun};
use evo_sim::gate::{format_gate, run_gate, GateOptions, WorldResult};
use evo_sim::orders::{Intervention, OrderKind};
use evo_sim::report::{format_summary, format_years};
use evo_sim::{World, WorldConfig};
use std::time::Instant;

// Allocateur rapide en parallèle : chaque mutant évalué alloue un génome et un
// phénotype, et l'allocateur du système se bloque entre fils.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|v| v.parse().unwrap_or_else(|_| panic!("valeur invalide pour {name} : {v}")))
        .unwrap_or(default)
}

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn usage() -> ! {
    eprintln!(
        "Usage :\n  evonisium run   [--world CLÉ] [--seed N] [--level L] [--steps S] [--step-years Y] [--every K] [--out DOSSIER]\n  evonisium porte [--worlds terre,ocean,...] [--seeds 2026,7,42] [--save-results DOSSIER] [--assemble DOSSIER] [--level L] [--step-years Y] [--max-years Y] [--out FICHIER] [--data DOSSIER]\n  evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]\n  evonisium empreinte [--world CLÉ] [--seed N] [--level L] [--steps S]\nMondes : {}",
        PlanetParams::KEYS.join(", ")
    );
    std::process::exit(2)
}

fn write_or_print(path: Option<String>, text: &str) {
    match path {
        Some(path) => {
            std::fs::write(&path, text).expect("écriture du rapport");
            eprintln!("Rapport écrit dans {path}");
        }
        None => print!("{text}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") => run(&args),
        Some("porte") => {
            let defaults = GateOptions::default();
            let opts = GateOptions {
                seeds: opt(&args, "--seeds")
                    .map(|v| v.split(',').map(|x| x.trim().parse().expect("graine invalide")).collect())
                    .unwrap_or(defaults.seeds),
                level: arg(&args, "--level", defaults.level),
                step_years: arg(&args, "--step-years", defaults.step_years),
                max_years: arg(&args, "--max-years", defaults.max_years),
                oxygen_threshold: arg(&args, "--threshold", defaults.oxygen_threshold),
                hold_years: arg(&args, "--hold-years", defaults.hold_years),
                worlds: opt(&args, "--worlds").map(|w| w.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
                out_dir: opt(&args, "--data").map(Into::into),
                round_years: opt(&args, "--round-years").map(|v| v.parse().expect("durée invalide")),
            };
            // Rapport assemblé à partir des résultats déjà enregistrés, monde
            // par monde (les parties longues tournent séparément).
            if let Some(dir) = opt(&args, "--assemble") {
                let mut results: Vec<WorldResult> = Vec::new();
                for key in PlanetParams::KEYS {
                    let mut files: Vec<_> = std::fs::read_dir(&dir)
                        .expect("dossier des résultats")
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.starts_with(&format!("{key}-")) && n.ends_with(".resultat"))
                        })
                        .collect();
                    files.sort();
                    for f in files {
                        let bytes = std::fs::read(&f).expect("lecture du résultat");
                        results.push(bincode::deserialize(&bytes).expect("résultat illisible"));
                    }
                }
                results
                    .sort_by_key(|r| (PlanetParams::KEYS.iter().position(|k| *k == r.key), opts.seeds.iter().position(|s| *s == r.seed)));
                write_or_print(opt(&args, "--out"), &format_gate(&opts, &results));
                return;
            }
            let save = opt(&args, "--save-results");
            let (report, _) = run_gate(&opts, |r| {
                eprintln!(
                    "{} (graine {}) : {} en {:.0} s, O₂ final {:.1e}, étape {} — {}",
                    r.name,
                    r.seed,
                    format_years(r.years),
                    r.seconds,
                    r.final_sample.o2_mixing,
                    r.final_sample.photosynthesis_stage,
                    if r.passed { "franchie" } else { "non franchie" }
                );
                if let Some(dir) = &save {
                    let _ = std::fs::create_dir_all(dir);
                    let path = std::path::Path::new(dir).join(format!("{}-{}.resultat", r.key, r.seed));
                    std::fs::write(path, bincode::serialize(r).expect("résultat sérialisable")).expect("écriture du résultat");
                }
            });
            write_or_print(opt(&args, "--out"), &report);
        }
        Some("empreinte") => fingerprint(&args),
        Some("chrono") => chrono(&args),
        Some("complexite") => complexity(&args),
        Some("equivalence") => equivalence(&args),
        Some("bench") => {
            let levels: String = arg(&args, "--levels", "6,7".to_string());
            let opts = BenchOptions {
                levels: levels.split(',').map(|l| l.trim().parse().expect("niveau invalide")).collect(),
                steps: arg(&args, "--steps", 20),
                warmup: arg(&args, "--warmup", 20),
                seed: arg(&args, "--seed", 2026),
            };
            write_or_print(opt(&args, "--out"), &run_benchmarks(&opts));
        }
        _ => usage(),
    }
}

fn run(args: &[String]) {
    let key: String = arg(args, "--world", "terre".to_string());
    let params = PlanetParams::by_key(&key).unwrap_or_else(|| usage());
    let mut cfg = WorldConfig::with_planet(params, arg(args, "--seed", 1), arg(args, "--level", 5));
    cfg.step_years = arg(args, "--step-years", cfg.step_years);
    let steps: u64 = arg(args, "--steps", 200);
    let every: u64 = arg(args, "--every", 50);
    let start = Instant::now();
    let mut world = World::new(cfg);
    // La vie arrive par la file d'ordres, comme toute action extérieure : la
    // partie se rejoue depuis sa graine et ce registre.
    world.orders.submit(0.0, OrderKind::SeedLife);
    println!(
        "Planète {} : {} cellules, {} sources hydrothermales, graine {}",
        world.planet.params.name,
        world.planet.cells.len(),
        world.planet.vent_cells().len(),
        world.config.seed
    );
    for i in 1..=steps {
        world.step();
        if i % every == 0 || i == steps {
            println!("\n— pas {i} —\n{}", format_summary(&world.summary()));
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("{} simulés en {:.1} s ({} par seconde)", format_years(world.years()), elapsed, format_years(world.years() / elapsed));
    if let Some(dir) = opt(args, "--out") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("création du dossier");
        std::fs::write(dir.join("historique.tsv"), world.history.to_tsv()).expect("écriture");
        std::fs::write(dir.join("evenements.tsv"), world.events.to_tsv()).expect("écriture");
        std::fs::write(dir.join("ordres.tsv"), world.orders.to_tsv()).expect("écriture");
        std::fs::write(dir.join("journal-genomes.tsv"), world.journal.to_tsv()).expect("écriture");
        eprintln!("Historique, événements, ordres et journal écrits dans {}", dir.display());
    }
}

/// Partie longue qui suit les étapes de la complexité (étape 4) :
///   evonisium complexite [--world CLÉ] [--seed N] [--level L] [--step-years Y] [--max-years Y] [--every-years Y] [--retention P]
fn complexity(args: &[String]) {
    use evo_sim::world::{COMPLEXITY_STAGES, COMPLEXITY_STAGE_COUNT};
    let key: String = arg(args, "--world", "terre".to_string());
    let params = PlanetParams::by_key(&key).unwrap_or_else(|| usage());
    let mut cfg = WorldConfig::with_planet(params, arg(args, "--seed", 2026), arg(args, "--level", 4));
    cfg.step_years = arg(args, "--step-years", 200_000.0);
    if let Some(p) = opt(args, "--retention") {
        cfg.transitions.retention_probability = p.parse().expect("probabilité invalide");
    }
    if args.iter().any(|a| a == "--sans-accelerateur") {
        cfg.evolution.accelerator.enabled = false;
    }
    let max_years: f64 = arg(args, "--max-years", 3e9);
    let every: f64 = arg(args, "--every-years", 50e6);
    let start = Instant::now();
    let mut world = World::new(cfg);
    world.orders.submit(0.0, OrderKind::SeedLife);
    let mut next = every;
    let mut seen = [false; COMPLEXITY_STAGE_COUNT];
    while world.years() < max_years {
        world.step();
        for (k, done) in seen.iter_mut().enumerate() {
            if !*done {
                if let Some(y) = world.progress.complexity_years[k] {
                    *done = true;
                    println!("  ★ {} : {}", COMPLEXITY_STAGES[k], format_years(y));
                }
            }
        }
        if world.years() >= next {
            next += every;
            let s = world.summary();
            let mut euk = 0.0;
            let mut phago = 0.0;
            let mut multi = 0.0;
            let mut total = 0.0;
            let (mut max_cells, mut max_types, mut max_size) = (1.0f64, 1usize, 1.0f64);
            let (mut genes, mut max_genes, mut pops, mut dead) = (0usize, 0usize, 0usize, 0usize);
            for p in world.communities.iter().flatten() {
                total += p.biomass;
                genes += p.genome.genes.len();
                dead += p.genome.genes.iter().filter(|g| !g.functional).count();
                max_genes = max_genes.max(p.genome.genes.len());
                pops += 1;
                if p.phenotype.is_eukaryote() {
                    euk += p.biomass;
                }
                if p.phenotype.is_phagotroph() {
                    phago += p.biomass;
                }
                if p.phenotype.is_multicellular() {
                    multi += p.biomass;
                }
                max_cells = max_cells.max(p.phenotype.cells());
                max_types = max_types.max(p.phenotype.cell_types());
                max_size = max_size.max(p.phenotype.cell_size);
            }
            let total = total.max(1e-300);
            println!(
                "{:>9} O₂ {:.1e} | phagotrophes {:.1} % eucaryotes {:.1} % multicellulaires {:.1} % | taille max {:.1} cellules max {:.0} types max {} | gènes {:.0} (max {}, {:.0} % inactifs) | vivant niveau {} | {:.0} s",
                format_years(world.years()),
                s.globals.o2_mixing,
                100.0 * phago / total,
                100.0 * euk / total,
                100.0 * multi / total,
                max_size,
                max_cells,
                max_types,
                genes as f64 / pops.max(1) as f64,
                max_genes,
                100.0 * dead as f64 / genes.max(1) as f64,
                world.config.bio_level,
                start.elapsed().as_secs_f64()
            );
        }
        if world.progress.complexity_years[6].is_some() && args.iter().any(|a| a == "--stop") {
            break;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("{} simulés en {:.0} s ({} par seconde)", format_years(world.years()), elapsed, format_years(world.years() / elapsed));
    if let Some(path) = opt(args, "--save") {
        world.save_file(std::path::Path::new(&path)).expect("écriture de la sauvegarde");
    }
}

/// Empreintes de l'état d'une partie scénarisée (ordres, interventions,
/// changement de pas, zone d'intérêt), une par pas : l'intégration continue
/// les compare entre Linux, Windows et macOS.
fn fingerprint(args: &[String]) {
    let key: String = arg(args, "--world", "terre".to_string());
    let params = PlanetParams::by_key(&key).unwrap_or_else(|| usage());
    let mut cfg = WorldConfig::with_planet(params, arg(args, "--seed", 3), arg(args, "--level", 4));
    cfg.step_years = 50_000.0;
    let steps: u64 = arg(args, "--steps", 40);
    let mut world = World::new(cfg);
    world.orders.submit(0.0, OrderKind::SeedLife);
    world.orders.submit(300_000.0, OrderKind::Intervene(Intervention::Fertilize { cell: 11, radius_km: 2500.0, moles_p: 1e13 }));
    world.orders.submit(600_000.0, OrderKind::Intervene(Intervention::Eruption { cell: 40, gas: evo_planet::Gas::Co2, moles: 1e16 }));
    world.orders.submit(900_000.0, OrderKind::SetStepYears(80_000.0));
    for i in 0..steps {
        world.set_interest(Some(evo_sim::InterestZone { center_cell: (i * 97 % 2562) as u32, radius_km: 3000.0, zoom_band: 1 }));
        world.step();
        println!("{i}\t{}\t{:016x}", world.years(), world.state_hash());
    }
    let s = world.summary();
    eprintln!("Bilans : carbone {:.1e}, phosphore {:.1e}, électrons {:.1e}", s.carbon_error, s.phosphorus_error, s.electron_error);
}

/// Chronométrage par poste depuis un point de sauvegarde (un monde mûr se
/// prépare une fois avec `--prepare`, puis chaque essai repart du même état).
///   evonisium chrono --save FICHIER [--prepare PAS] [--level L] [--steps S]
fn chrono(args: &[String]) {
    let path = std::path::PathBuf::from(opt(args, "--save").unwrap_or_else(|| usage()));
    if let Some(warmup) = opt(args, "--prepare") {
        let mut cfg = WorldConfig::new(arg(args, "--seed", 2026), arg(args, "--level", 6));
        cfg.seeding = evo_sim::Seeding::AllOcean;
        let mut world = World::new(cfg);
        world.seed_life();
        let n: u64 = warmup.parse().expect("nombre de pas");
        for i in 0..n {
            world.step();
            if (i + 1) % 50 == 0 {
                eprintln!("préparation : pas {} / {n}", i + 1);
            }
        }
        world.save_file(&path).expect("écriture de la sauvegarde");
        return;
    }
    let t0 = Instant::now();
    let mut world = World::load_file(&path).expect("lecture de la sauvegarde");
    if let Some(v) = opt(args, "--step-years") {
        world.config.step_years = v.parse().expect("pas invalide");
    }
    if let Some(v) = opt(args, "--cap") {
        world.config.max_populations_per_cell = v.parse().expect("plafond invalide");
    }
    if let Some(v) = opt(args, "--round-years") {
        let r: f64 = v.parse().expect("durée invalide");
        world.config.evolution.round_years = (r > 0.0).then_some(r);
    }
    eprintln!("lecture : {:.2} s", t0.elapsed().as_secs_f64());
    if args.iter().any(|a| a == "--sizes") {
        let mb = |n: u64| n as f64 / 1e6;
        eprintln!(
            "tailles (Mo, avant compression) : lignées {:.1}, événements {:.1}, journal {:.1}, historique {:.1}, ordres {:.1}, chimie {:.1}, planète {:.1}",
            mb(bincode::serialized_size(&world.lineages).unwrap_or(0)),
            mb(bincode::serialized_size(&world.events).unwrap_or(0)),
            mb(bincode::serialized_size(&world.journal).unwrap_or(0)),
            mb(bincode::serialized_size(&world.history).unwrap_or(0)),
            mb(bincode::serialized_size(&world.orders).unwrap_or(0)),
            mb(bincode::serialized_size(&world.chemistry).unwrap_or(0)),
            mb(bincode::serialized_size(&world.planet.cells).unwrap_or(0)),
        );
        let recs = &world.lineages.records;
        let founders: Vec<usize> = recs.iter().filter_map(|r| r.founder.as_ref().map(|g| std::sync::Arc::as_ptr(g) as usize)).collect();
        let distinct: std::collections::BTreeSet<_> = founders.iter().collect();
        eprintln!("lignées : {} fiches, {} génomes fondateurs, {} distincts", recs.len(), founders.len(), distinct.len());
        let t = Instant::now();
        let mut out = Vec::new();
        world.save_to(&mut out).expect("écriture");
        eprintln!("écriture : {:.2} s, {:.1} Mo", t.elapsed().as_secs_f64(), mb(out.len() as u64));
    }
    let steps: u32 = arg(args, "--steps", 5);
    let mut total = evo_sim::world::PhaseTimings::default();
    let evals = world.stats.genetic_evaluations;
    let (occupied, saturated, established) =
        (world.stats.occupied_cell_steps, world.stats.saturated_cell_steps, world.stats.established_eviction_cell_steps);
    for _ in 0..steps {
        total.add(&world.step());
    }
    let occ = (world.stats.occupied_cell_steps - occupied).max(1) as f64;
    eprintln!(
        "cellules saturées pendant ces pas : {:.1} %, dont avec éviction d'une population établie : {:.1} %",
        100.0 * (world.stats.saturated_cell_steps - saturated) as f64 / occ,
        100.0 * (world.stats.established_eviction_cell_steps - established) as f64 / occ
    );
    let t = total.divided(steps);
    let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
    let s = world.summary();
    println!(
        "{} populations ; pas {:.0} ms : planète {:.0}, écologie {:.0}, évolution {:.0}, migration {:.0}, registres {:.0} ; {:.0} évaluations par pas ; empreinte {:016x}",
        s.populations,
        ms(t.total()),
        ms(t.planet),
        ms(t.ecology),
        ms(t.evolution),
        ms(t.migration),
        ms(t.bookkeeping),
        (world.stats.genetic_evaluations - evals) as f64 / steps as f64,
        world.state_hash()
    );
}

/// Même partie aux deux pas, graine par graine ; chaque partie est
/// enregistrée à part pour reprendre une série interrompue.
fn equivalence(args: &[String]) {
    let d = EquivalenceOptions::default();
    let list = |name: &str| opt(args, name).map(|v| v.split(',').map(|x| x.trim().to_string()).collect::<Vec<_>>());
    let opts = EquivalenceOptions {
        world: arg(args, "--world", d.world),
        seeds: list("--seeds").map(|v| v.iter().map(|x| x.parse().expect("graine invalide")).collect()).unwrap_or(d.seeds),
        level: arg(args, "--level", d.level),
        steps_years: list("--steps-years").map(|v| v.iter().map(|x| x.parse().expect("pas invalide")).collect()).unwrap_or(d.steps_years),
        years: arg(args, "--years", d.years),
        oxygen_threshold: arg(args, "--threshold", d.oxygen_threshold),
        round_years: opt(args, "--round-years").map(|v| v.parse().expect("durée invalide")),
    };
    let dir = opt(args, "--save-results").or_else(|| opt(args, "--assemble"));
    let path =
        |seed: u64, step: f64| dir.as_ref().map(|d| std::path::Path::new(d).join(format!("{}-{seed}-{step:.0}.equivalence", opts.world)));
    let mut runs: Vec<EquivalenceRun> = Vec::new();
    for &step in &opts.steps_years {
        for &seed in &opts.seeds {
            if let Some(Ok(bytes)) = path(seed, step).map(std::fs::read) {
                runs.push(bincode::deserialize(&bytes).expect("résultat illisible"));
                continue;
            }
            if opt(args, "--assemble").is_some() {
                continue;
            }
            let r = run_one(&opts, seed, step);
            eprintln!(
                "{} graine {seed}, pas de {} : O₂ > seuil à {}, {:.0} substitutions/Ma dont {:.0} avantageuses, {} guildes, {:.0} s",
                opts.world,
                format_years(step),
                r.oxygen_threshold_years.map_or("—".into(), format_years),
                r.substitutions_per_ma(),
                r.adaptive_per_ma(),
                r.guilds,
                r.seconds
            );
            if let Some(p) = path(seed, step) {
                let _ = std::fs::create_dir_all(p.parent().expect("dossier"));
                std::fs::write(p, bincode::serialize(&r).expect("résultat sérialisable")).expect("écriture du résultat");
            }
            runs.push(r);
        }
    }
    write_or_print(opt(args, "--out"), &format_equivalence(&opts, &runs));
}
