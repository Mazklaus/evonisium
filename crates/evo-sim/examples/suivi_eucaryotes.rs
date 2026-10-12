//! Suit pas à pas les populations d'eucaryotes d'une sauvegarde : cellule,
//! biomasse, taux, pour voir comment une lignée nouvelle disparaît.
use evo_sim::World;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut world = World::load_file(Path::new(&args[1])).expect("lecture");
    let steps: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(20);
    for _ in 0..steps {
        let line: Vec<String> = world
            .communities
            .iter()
            .enumerate()
            .flat_map(|(c, pops)| {
                pops.iter()
                    .filter(|p| p.phenotype.is_eukaryote())
                    .map(move |p| format!("c{c}:{:.2e} r {:.1} n {} lign {}", p.biomass, p.rates.r, pops.len(), p.lineage))
            })
            .collect();
        println!("{:.4} Ga | {}", world.years() / 1e9, line.join(" ; "));
        world.step();
    }
}
