//! Les individus reproduisent les taux de leur population, et la scène ne
//! tue que lorsque le niveau 4 le demande.

use evo_agents::agents::{Act, Fate, Scene};
use evo_agents::bench::with_body;
use evo_agents::sample::{PopRates, Sample};
use evo_agents::Diet;

fn sample(rates: PopRates, size: usize, seed: u64, heterotroph: bool) -> Sample {
    evo_agents::bench::sample(rates, size, seed, heterotroph)
}

/// Écart toléré entre un compte de Poisson et son attente : 4 écarts types.
fn close(count: u64, expected: f64) -> bool {
    (count as f64 - expected).abs() <= 4.0 * expected.sqrt().max(1.0)
}

#[test]
fn individuals_reproduce_the_population_rates() {
    let rates = PopRates { birth: 2.0, death: 1.8, predation: 0.25, emigration: 0.5 };
    let mut s = sample(rates, 300, 7, false);
    let p = &s.genotypes[0].phenotype;
    assert!(p.is_multicellular() && p.is_eukaryote() && p.sexual, "la colonie d'essai doit être eucaryote, sexuée et multicellulaire");
    s.advance(40.0);
    let c = s.counters;
    let e = c.exposure;
    assert!(e > 1000.0, "exposition trop courte : {e}");
    assert!(close(c.births, rates.birth * e), "naissances {} pour {:.0} attendues", c.births, rates.birth * e);
    assert!(close(c.deaths, rates.death * e), "morts {} pour {:.0} attendues", c.deaths, rates.death * e);
    let pred = rates.death * rates.predation * e;
    assert!(close(c.predation_deaths, pred), "prédation {} pour {:.0}", c.predation_deaths, pred);
    assert!(close(c.emigrations, rates.emigration * e), "départs {} pour {:.0}", c.emigrations, rates.emigration * e);
    assert!(close(c.immigrations, rates.emigration * e), "arrivées {} pour {:.0}", c.immigrations, rates.emigration * e);
    // L'échantillon reste dans ses bornes.
    assert!(s.members.len() >= 75 && s.members.len() <= 600, "taille {}", s.members.len());
}

#[test]
fn reproduction_is_real_genomes_recombine_and_mutate() {
    let rates = PopRates { birth: 3.0, death: 3.0, predation: 0.0, emigration: 0.0 };
    let mut s = sample(rates, 200, 11, false);
    s.advance(20.0);
    // Des enfants nés de deux parents de l'échantillon.
    let born: Vec<_> = s.members.iter().filter(|m| m.parents[0] != u32::MAX).collect();
    assert!(!born.is_empty());
    assert!(born.iter().any(|m| m.parents[1] != u32::MAX), "aucun enfant à deux parents chez une espèce sexuée");
    // Plusieurs génotypes, chacun développé une fois (cache).
    assert!(s.genotypes.len() > 1, "aucune mutation en {} naissances", s.counters.births);
    let keys: std::collections::HashSet<u64> = s.genotypes.iter().map(|g| evo_agents::sample::genome_key(&g.genome)).collect();
    assert_eq!(keys.len(), s.genotypes.len(), "un génome développé deux fois");
    // La valeur sélective moyenne reste près de 1 : la variation tirée
    // est faible devant celle qui se fixe.
    assert!(s.mean_fitness() > 0.5);
}

#[test]
fn same_seed_same_individuals() {
    let rates = PopRates { birth: 2.0, death: 2.0, predation: 0.1, emigration: 0.3 };
    let run = |seed| {
        let mut s = sample(rates, 150, seed, false);
        s.advance(10.0);
        (s.counters, s.members.iter().map(|m| (m.id, m.genotype, m.position_km[0].to_bits())).collect::<Vec<_>>())
    };
    assert_eq!(run(3), run(3));
    assert_ne!(run(3).0, run(4).0);
}

/// Scène d'un brouteur et de son prédateur : les morts suivent le niveau 4,
/// les chasses spontanées échouent, et le lexique entier peut servir.
#[test]
fn scene_kills_only_when_level_four_says_so() {
    let prey_rates = PopRates { birth: 1.0, death: 1.0, predation: 0.8, emigration: 0.2 };
    // La proie est un petit nageur, le prédateur un grand.
    let prey = with_body(sample(prey_rates, 120, 21, true), 0.05, 0.6, 1.5, Diet::Grazer);
    let pred = with_body(
        sample(PopRates { birth: 0.2, death: 0.2, predation: 0.0, emigration: 0.05 }, 8, 22, true),
        0.4,
        1.8,
        4.0,
        Diet::Predator,
    );
    let mut scene = Scene::new(vec![prey, pred], 200.0, 5, 2.0);
    let start = scene.agents.len();
    let mut seen = [0.0f32; 12];
    for _ in 0..(120.0 / 0.05) as usize {
        scene.step(0.05);
    }
    for a in &scene.agents {
        for (k, t) in seen.iter_mut().enumerate() {
            *t += a.time_in[k];
        }
    }
    let deaths: usize = scene.applied.iter().filter(|e| matches!(e.kind, evo_agents::LifeEventKind::Death { .. })).count();
    let births = scene.applied.iter().filter(|e| matches!(e.kind, evo_agents::LifeEventKind::Birth { .. })).count();
    assert!(deaths > 0 && births > 0, "aucun événement de vie (morts {deaths}, naissances {births})");
    // Les agents encore présents suivent l'échantillon : chaque individu de
    // l'échantillon a un agent (aux morts en cours près).
    let pending = scene.agents.iter().filter(|a| matches!(a.fate, Fate::Doomed { .. } | Fate::Dead { .. })).count();
    let members: usize = scene.species.iter().map(|s| s.sample.members.len()).sum();
    let living = scene.agents.iter().filter(|a| !matches!(a.fate, Fate::Doomed { .. } | Fate::Dead { .. } | Fate::Leaving)).count();
    assert!(
        living <= members + 2 && living + pending + 40 >= members,
        "agents {living} (+{pending}) pour {members} individus, départ {start}"
    );
    let predation = scene.applied.iter().filter(|e| matches!(e.kind, evo_agents::LifeEventKind::Death { predation: true, .. })).count();
    let feed: f32 = scene.agents.iter().filter(|a| a.species == 0).map(|a| a.time_in[Act::Feed.index()]).sum();
    let all: f32 = scene.agents.iter().filter(|a| a.species == 0).map(|a| a.time_in.iter().sum::<f32>()).sum();
    eprintln!(
        "morts {deaths} dont prédation {predation}, prises {}, échappées {}, naissances {births}, part du temps à manger {:.2} (visée {:.2}), actions {seen:?}",
        scene.kills,
        scene.escapes,
        feed / all,
        scene.species[0].feed_share
    );
    assert!(scene.kills as usize <= predation, "un prédateur a tué sans mort tirée au niveau 4");
    assert!(scene.kills > 0, "aucune mort par prédation montrée");
    for act in [Act::Feed, Act::Hunt, Act::Flee, Act::Rest, Act::Migrate] {
        assert!(seen[act.index()] > 0.0, "action jamais vue : {act:?} ({seen:?})");
    }
}
