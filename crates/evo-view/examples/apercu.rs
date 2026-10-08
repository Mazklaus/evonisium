//! Aperçu hors Godot : décor de milieu, vue microscope et figure de
//! l'espèce la plus abondante après quelques millions d'années, en fichiers
//! RGBA bruts (`cargo run --release -p evo-view --example apercu -- DOSSIER [ANNÉES]`).

use evo_engine::{Answer, Engine, NewGame, OrderKind, Query, When};
use evo_view::decor::{paint, Habitat};
use evo_view::frame::{Frame, PlanetInfo};
use evo_view::species::{microscope_members, traits_of};
use std::sync::Arc;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let years: f64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(4e6);
    let game = NewGame { seed: 42, level: 4, threads: 4, ..NewGame::default() };
    let planet = game.config().unwrap().planet;
    let engine = Engine::new_game(game).unwrap();
    engine.submit(When::Now, OrderKind::SeedLife);
    engine.submit(When::At(years), OrderKind::Pause);
    engine.submit(When::Now, OrderKind::Resume);
    while engine.status().years < years || !engine.status().paused {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let info = PlanetInfo {
        name: planet.name.clone(),
        seed: 42,
        level: 4,
        radius_m: planet.radius_m,
        star_temperature_k: planet.star_temperature_k,
    };
    let f = Frame::new(engine.frame().current, Arc::new(info));
    let best = f.state.species.first().expect("vie");
    let Ok(Answer::Habitat(Some(eh))) = engine.query(Query::SpeciesHabitat { species: best.id }).recv() else { panic!("milieu") };
    let h = Habitat::from_engine(&eh, &f);
    let t = std::time::Instant::now();
    let decor = paint(&h, h.decor_seed(42), 1024, 512);
    println!("décor en {} ms ; milieu {:?}", t.elapsed().as_millis(), h);
    std::fs::write(format!("{out}/decor.rgba"), &decor.pixels).unwrap();
    let Ok(Answer::Cell(Some(cell))) = engine.query(Query::Cell { cell: h.cell as u32 }).recv() else { panic!("cellule") };
    let t = std::time::Instant::now();
    let (m, placed) = evo_morph::microscope_field(&microscope_members(&cell.populations, 42), 7, 800, 600);
    println!("microscope en {} ms, {} cellules", t.elapsed().as_millis(), placed.len());
    std::fs::write(format!("{out}/microscope.rgba"), &m.pixels).unwrap();
    if let Some(p) = cell.populations.iter().find(|p| p.species == best.signature) {
        let (fig, _) = evo_morph::figure(&evo_morph::form(&traits_of(p), 42), 512, 384);
        std::fs::write(format!("{out}/figure.rgba"), &fig.pixels).unwrap();
    }
}
