//! Outil en ligne de commande.
//!
//!   evonisium run   [--world CLÉ] [--seed N] [--level L] [--steps S] [--step-years Y] [--every K] [--out DOSSIER]
//!   evonisium porte [--worlds terre,ocean,...] [--seeds 2026,7,42] [--save-results DOSSIER] [--assemble DOSSIER] [--level L] [--step-years Y] [--max-years Y] [--out FICHIER] [--data DOSSIER]
//!   evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]
//!   evonisium empreinte [--world CLÉ] [--seed N] [--level L] [--steps S]
//!
//! Mondes : terre, ocean, desert, super-terre, petite, sans-lune.

use evo_planet::PlanetParams;
use evo_sim::bench::{run_benchmarks, BenchOptions};
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
