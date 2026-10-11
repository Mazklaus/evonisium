//! Vitesse du modèle culturel en mode historique : une espèce à langage sur
//! 2 000 cellules, au pas d'un an.
//!
//! `cargo run --release -p evo-culture --example vitesse`

use evo_culture::bench::{grid, seed_technique};
use evo_culture::catalog::{Context, Family};
use evo_culture::cognition::presets;
use evo_culture::world::CultureWorld;
use evo_culture::{CulturalState, Organ};
use std::time::Instant;

fn main() {
    let mut w = CultureWorld::new(1);
    let land = Context::land(Organ::Hand);
    let cells = grid(&mut w, 1, presets::human(), 0, 50, 40, 1200.0, 0.3, land);
    for f in [Family::Percussion, Family::Flake, Family::Fire, Family::Shelter] {
        seed_technique(&mut w, 1, &cells, f, 0.8, 1.0);
    }
    // Deux pas à l'équilibre : la bascule, des peuples et des langues.
    w.step(10_000.0);
    w.step(10_000.0);
    assert!(w.historical());
    let years: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(2000);
    let t = Instant::now();
    for _ in 0..years {
        w.step(1.0);
    }
    let dt = t.elapsed().as_secs_f64();
    let t2 = Instant::now();
    let st = CulturalState::of(&w);
    let publish = t2.elapsed().as_secs_f64();
    let traits: usize = w.groups.iter().map(|g| g.traits.len()).sum();
    println!(
        "{} groupes, {:.1} traits par groupe, {} peuples vivants, {} langues vivantes, {} notables vivants, {} mythes",
        w.groups.len(),
        traits as f64 / w.groups.len() as f64,
        st.peoples.iter().filter(|p| p.ended_year.is_none()).count(),
        st.languages.iter().filter(|l| l.ended_year.is_none()).count(),
        st.notables.iter().filter(|n| n.alive).count(),
        st.myths.len()
    );
    println!("{years} ans en {dt:.2} s : {:.0} années par seconde ({} fils) ; publication {:.1} ms", years as f64 / dt, rayon::current_num_threads(), publish * 1e3);
}
