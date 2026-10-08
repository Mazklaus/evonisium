//! Aperçu hors Godot : décor de milieu, vue microscope et figure de la
//! lignée la plus abondante après quelques millions d'années, en fichiers
//! RGBA bruts (`cargo run --release -p evo-view --example apercu -- DOSSIER`).

use evo_sim::{World, WorldConfig};
use evo_view::decor::{paint, Habitat};
use evo_view::frame::Frame;
use evo_view::species::microscope_members;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut w = World::new(WorldConfig::new(42, 4));
    w.seed_life();
    let steps: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    for _ in 0..steps {
        w.step();
    }
    let f = Frame::from_world(&w);
    let best = f.populations.iter().max_by(|a, b| a.biomass.total_cmp(&b.biomass)).unwrap();
    let h = Habitat::of_species(&f, best.lineage).unwrap();
    let t = std::time::Instant::now();
    let decor = paint(&h, h.decor_seed(42), 1024, 512);
    println!("décor en {} ms ; milieu {:?}", t.elapsed().as_millis(), h);
    std::fs::write(format!("{out}/decor.rgba"), &decor.pixels).unwrap();
    let cell = h.cell;
    let t = std::time::Instant::now();
    let (m, placed) = evo_morph::microscope_field(&microscope_members(&f, cell), 7, 800, 600);
    println!("microscope en {} ms, {} cellules", t.elapsed().as_millis(), placed.len());
    std::fs::write(format!("{out}/microscope.rgba"), &m.pixels).unwrap();
    let (fig, _) = evo_morph::figure(&evo_morph::form(&evo_view::species::traits_of(best), 42), 512, 384);
    std::fs::write(format!("{out}/figure.rgba"), &fig.pixels).unwrap();
}
