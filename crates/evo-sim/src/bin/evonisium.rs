//! Outil en ligne de commande.
//!
//!   evonisium run   [--world CLÉ] [--seed N] [--level L] [--steps S] [--step-years Y] [--every K] [--out DOSSIER]
//!   evonisium porte [--worlds terre,ocean,...] [--seed N] [--level L] [--step-years Y] [--max-years Y] [--out FICHIER] [--data DOSSIER]
//!   evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]
//!
//! Mondes : terre, ocean, desert, super-terre, petite, sans-lune.

use evo_planet::PlanetParams;
use evo_sim::bench::{run_benchmarks, BenchOptions};
use evo_sim::gate::{run_gate, GateOptions};
use evo_sim::orders::OrderKind;
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
        "Usage :\n  evonisium run   [--world CLÉ] [--seed N] [--level L] [--steps S] [--step-years Y] [--every K] [--out DOSSIER]\n  evonisium porte [--worlds terre,ocean,...] [--seed N] [--level L] [--step-years Y] [--max-years Y] [--out FICHIER] [--data DOSSIER]\n  evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]\nMondes : {}",
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
                seed: arg(&args, "--seed", defaults.seed),
                level: arg(&args, "--level", defaults.level),
                step_years: arg(&args, "--step-years", defaults.step_years),
                max_years: arg(&args, "--max-years", defaults.max_years),
                oxygen_threshold: arg(&args, "--threshold", defaults.oxygen_threshold),
                hold_years: arg(&args, "--hold-years", defaults.hold_years),
                worlds: opt(&args, "--worlds").map(|w| w.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
                out_dir: opt(&args, "--data").map(Into::into),
            };
            let (report, _) = run_gate(&opts, |r| {
                eprintln!(
                    "{} : {} en {:.0} s, O₂ final {:.1e}, étape {} — {}",
                    r.name,
                    format_years(r.years),
                    r.seconds,
                    r.final_sample.o2_mixing,
                    r.final_sample.photosynthesis_stage,
                    if r.passed { "franchie" } else { "non franchie" }
                )
            });
            write_or_print(opt(&args, "--out"), &report);
        }
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
