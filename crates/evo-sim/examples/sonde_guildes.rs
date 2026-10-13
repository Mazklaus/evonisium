//! Populations et guildes par cellule d'un point de sauvegarde.
//!
//!   cargo run --release -p evo-sim --example sonde_guildes -- FICHIER
use evo_sim::World;
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("point de sauvegarde");
    let w = World::load_file(Path::new(&path)).expect("illisible");
    let max = w.config.max_populations_per_cell;
    let (mut cells, mut pops, mut over, mut guilds_total) = (0, 0, 0, 0);
    let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
    let mut keys: BTreeMap<String, usize> = BTreeMap::new();
    for c in &w.communities {
        if c.is_empty() {
            continue;
        }
        cells += 1;
        pops += c.len();
        let mut g: Vec<Option<u32>> = c.iter().map(|p| p.phenotype.guild_key()).collect();
        g.sort();
        g.dedup();
        guilds_total += g.len();
        if c.len() > max {
            over += 1;
        }
        *hist.entry(c.len()).or_default() += 1;
        for p in c {
            let s = p.phenotype.signature;
            let k = format!(
                "{}{}{}{}{}",
                if p.phenotype.is_multicellular() { "M" } else { "-" },
                if p.phenotype.is_eukaryote() { "E" } else { "-" },
                if p.phenotype.is_terrestrial() { "T" } else { "-" },
                if p.phenotype.is_phagotroph() { "P" } else { "-" },
                if p.phenotype.plastids > 0 { "L" } else { "-" },
            );
            let _ = s;
            *keys.entry(k).or_default() += 1;
        }
    }
    println!("{:.0} Ma : {cells} cellules occupées, {pops} populations ({:.1} par cellule), {:.1} guildes par cellule, {over} au-dessus du plafond {max}", w.years / 1e6, pops as f64 / cells as f64, guilds_total as f64 / cells as f64);
    println!("populations par cellule : {hist:?}");
    println!("organisation (M E T P L) : {keys:?}");
}
