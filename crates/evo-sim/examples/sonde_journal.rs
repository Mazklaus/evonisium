//! Compte les événements d'une sauvegarde par type et taille du journal.
use evo_sim::World;
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = World::load_file(Path::new(&args[1])).expect("lecture");
    let ev = &world.events.events;
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for e in ev {
        *by.entry(e.kind.type_name().to_string()).or_default() += 1;
    }
    println!("{} événements, {} octets chacun en mémoire (hors chaînes)", ev.len(), std::mem::size_of_val(&ev[0]));
    for (k, n) in by {
        println!("  {k} : {n}");
    }
}
