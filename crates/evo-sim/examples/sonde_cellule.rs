//! Rejoue l'écologie rapide d'une cellule d'une sauvegarde, sous-pas par
//! sous-pas, et affiche la biomasse de chaque population.
use evo_life::community::{evaluate, substep, CellContext};
use evo_sim::World;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = World::load_file(Path::new(&args[1])).expect("lecture");
    let c: usize = args[2].parse().expect("cellule");
    let physio = world.config.physiology.clone();
    let ctx = CellContext { env: &world.bio.env[c], light_biomass_per_m2: world.config.light_biomass_per_m2 };
    let mut pops = world.communities[c].clone();
    let mut chem = world.chemistry[c];
    let show = |pops: &[evo_life::Population], k: usize| {
        let s: Vec<String> = pops
            .iter()
            .map(|p| format!("{}{:.2e}/r{:.0}", if p.phenotype.is_eukaryote() { "E" } else { "" }, p.biomass, p.rates.r))
            .collect();
        println!("{k:>3} {}", s.join(" "));
    };
    evaluate(&mut pops, &ctx, &chem, &physio);
    show(&pops, 0);
    for k in 1..=world.config.eco_substeps {
        substep(&mut pops, &ctx, &mut chem, world.config.eco_dt_years, &physio);
        if k % 5 == 0 {
            show(&pops, k);
        }
    }
    let names = ["DIC", "DOC", "H2", "CH4", "O2", "SO4", "H2S", "Fe2", "Mn2", "FeOx", "MnOx", "PO4"];
    let s: Vec<String> = chem.iter().zip(names).map(|(x, n)| format!("{n} {x:.2e}")).collect();
    println!("{}", s.join(", "));
}
