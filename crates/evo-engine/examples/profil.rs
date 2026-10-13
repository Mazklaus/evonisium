//! Profil de vitesse d'une partie, phase par phase (étude de la vitesse de
//! simulation, 12 octobre 2026).
//!
//!   cargo run --release -p evo-engine --example profil -- [options]
//!
//! Partie neuve : `--seed N` (414299), `--level L` (4), `--world CLÉ` (terre).
//! Reprise : `--load FICHIER` (point de sauvegarde).
//! `--until ANS` (2,7e9) : date de fin ; `--every ANS` (1e8) : une ligne par
//! tranche ; `--threads N` (4) ; `--save-at ANS --save FICHIER` : point de
//! sauvegarde pris en route, pour mesurer ensuite chaque levier depuis le même
//! état.
//!
//! Leviers (changent l'histoire, pour mesure seulement) : `--deme-level L`,
//! `--cap N` (populations par cellule), `--substeps N` (sous-pas écologiques),
//! `--max-factor N` (allongement du pas en période calme), `--round-years ANS`
//! (durée d'un tour d'évolution).
//!
//! Sortie (TSV) : date (Ma), pas cumulés, pas moyen (ka), ms par pas (horloge
//! murale) puis par phase, cellules du vivant, dèmes, populations, populations
//! terrestres, mémoire résidente (Mo), vitesse (Ma/s).
use evo_planet::{BioGrid, PlanetParams};
use evo_sim::orders::OrderKind;
use evo_sim::{World, WorldConfig};
use std::path::Path;
use std::time::{Duration, Instant};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn arg<T: std::str::FromStr>(a: &[String], name: &str, default: T) -> T {
    opt(a, name).unwrap_or(default)
}

fn opt<T: std::str::FromStr>(a: &[String], name: &str) -> Option<T> {
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .map(|v| v.parse().unwrap_or_else(|_| panic!("valeur invalide pour {name} : {v}")))
}

/// Mémoire résidente du processus, Mo (Linux seulement ; 0 ailleurs).
fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS")).and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok()))
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let threads: usize = arg(&a, "--threads", 4);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("groupe de fils");
    let until: f64 = arg(&a, "--until", 2.7e9);
    let every: f64 = arg(&a, "--every", 1e8);
    let save_at: f64 = arg(&a, "--save-at", f64::INFINITY);
    let save: Option<String> = opt(&a, "--save");
    let mut w = match opt::<String>(&a, "--load") {
        Some(path) => World::load_file(Path::new(&path)).expect("point de sauvegarde illisible"),
        None => {
            let key: String = arg(&a, "--world", "terre".to_string());
            let params = PlanetParams::by_key(&key).unwrap_or_else(|| panic!("monde inconnu : {key}"));
            let cfg = WorldConfig::with_planet(params, arg(&a, "--seed", 414299), arg(&a, "--level", 4));
            let mut w = pool.install(|| World::new(cfg));
            w.orders.submit(0.0, OrderKind::SeedLife);
            w
        }
    };

    // Leviers.
    if let Some(level) = opt::<u32>(&a, "--deme-level") {
        let g = BioGrid::new(&w.bio.grid, level);
        (w.demes, w.deme_index) = (g.children, g.parent);
        w.config.deme_level = Some(level);
    }
    w.config.max_populations_per_cell = arg(&a, "--cap", w.config.max_populations_per_cell);
    w.config.eco_substeps = arg(&a, "--substeps", w.config.eco_substeps);
    if let Some(step) = w.config.adaptive_step.as_mut() {
        step.max_factor = arg(&a, "--max-factor", step.max_factor);
    }
    if let Some(r) = opt::<f64>(&a, "--round-years") {
        w.config.evolution.round_years = Some(r);
    }

    println!("Ma\tpas\tpas_ka\tms_pas\tplanete\tecologie\tevolution\tmigration\tregistres\tcellules\tdemes\tpopulations\tterrestres\tRSS_Mo\tMa_s");
    let (mut last_t, mut last_steps, mut last_years, mut wall) = (w.timings, w.stats.steps, w.years(), Instant::now());
    let mut saved = false;
    while w.years() < until {
        pool.install(|| w.step());
        if let (false, Some(path)) = (saved, &save) {
            if w.years() >= save_at {
                w.save_file(Path::new(path)).expect("écriture du point de sauvegarde");
                saved = true;
                eprintln!("Point de sauvegarde écrit à {:.0} Ma", w.years() / 1e6);
            }
        }
        if (w.years() / every).floor() > (last_years / every).floor() || w.years() >= until {
            let n = (w.stats.steps - last_steps).max(1) as f64;
            let t = w.timings;
            let ms = |x: Duration, y: Duration| (x - y).as_secs_f64() * 1e3 / n;
            let seconds = wall.elapsed().as_secs_f64();
            let populations: usize = w.communities.iter().map(Vec::len).sum();
            println!(
                "{:.0}\t{}\t{:.0}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{}\t{}\t{}\t{}\t{:.0}\t{:.2}",
                w.years() / 1e6,
                w.stats.steps,
                (w.years() - last_years) / n / 1e3,
                seconds * 1e3 / n,
                ms(t.planet, last_t.planet),
                ms(t.ecology, last_t.ecology),
                ms(t.evolution, last_t.evolution),
                ms(t.migration, last_t.migration),
                ms(t.bookkeeping, last_t.bookkeeping),
                w.communities.len(),
                w.demes.len(),
                populations,
                w.land_populations().0,
                rss_mb(),
                (w.years() - last_years) / 1e6 / seconds
            );
            (last_t, last_steps, last_years, wall) = (t, w.stats.steps, w.years(), Instant::now());
        }
    }
}
