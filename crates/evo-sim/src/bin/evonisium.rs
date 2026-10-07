//! Outil en ligne de commande de l'étape 1.
//!
//!   evonisium run   [--seed N] [--level L] [--steps S] [--step-years Y] [--every K]
//!   evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]

use evo_sim::bench::{run_benchmarks, BenchOptions};
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

fn usage() -> ! {
    eprintln!(
        "Usage :\n  evonisium run   [--seed N] [--level L] [--steps S] [--step-years Y] [--every K]\n  evonisium bench [--levels 6,7] [--steps S] [--out FICHIER]"
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") => run(&args),
        Some("bench") => {
            let levels: String = arg(&args, "--levels", "6,7".to_string());
            let opts = BenchOptions {
                levels: levels.split(',').map(|l| l.trim().parse().expect("niveau invalide")).collect(),
                steps: arg(&args, "--steps", 20),
                warmup: arg(&args, "--warmup", 20),
                seed: arg(&args, "--seed", 2026),
            };
            let report = run_benchmarks(&opts);
            match args.iter().position(|a| a == "--out").and_then(|i| args.get(i + 1)) {
                Some(path) => {
                    std::fs::write(path, &report).expect("écriture du rapport");
                    eprintln!("Rapport écrit dans {path}");
                }
                None => print!("{report}"),
            }
        }
        _ => usage(),
    }
}

fn run(args: &[String]) {
    let mut cfg = WorldConfig::new(arg(args, "--seed", 1), arg(args, "--level", 5));
    cfg.step_years = arg(args, "--step-years", cfg.step_years);
    let steps: u64 = arg(args, "--steps", 200);
    let every: u64 = arg(args, "--every", 50);
    let start = Instant::now();
    let mut world = World::new(cfg);
    world.seed_life();
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
}
