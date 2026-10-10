//! Échantillons tirés d'un vrai monde : ils lisent ses taux, ils ne le
//! touchent pas.

use evo_agents::sample::{individuals_of, pop_rates, Sample};
use evo_agents::zones::{active_cells, Reason};
use evo_sim::{InterestZone, World, WorldConfig};

#[test]
fn a_world_sample_reads_its_population_and_leaves_the_world_untouched() {
    let mut w = World::new(WorldConfig::new(17, 3));
    w.seed_life();
    for _ in 0..8 {
        w.step();
    }
    let (c, i) = w
        .communities
        .iter()
        .enumerate()
        .flat_map(|(c, pops)| pops.iter().enumerate().map(move |(i, p)| (c, i, p.biomass)))
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(c, i, _)| (c, i))
        .expect("du vivant");
    let before = w.state_hash();
    let p = &w.communities[c][i];
    let mut s = Sample::draw(&w, c, i, 200).expect("échantillon");
    assert_eq!(s.lineage, p.lineage);
    assert_eq!(s.rates, pop_rates(p, w.config.migration_rate));
    assert!((s.census - individuals_of(p, &w.config.physiology)).abs() <= 1e-9 * s.census);
    assert_eq!(s.members.len(), 200);
    // Le génome de la population est le génotype 0, de valeur 1.
    assert_eq!(*s.genotypes[0].genome, *p.genome);
    assert_eq!(s.genotypes[0].fitness, 1.0);
    // Les microbes se divisent vite : quelques jours suffisent.
    s.advance(10.0 / s.rates.birth.max(1e-3));
    // Les taux de l'échantillon sont ceux de la population, aux
    // fluctuations près (4 écarts types d'un compte de Poisson).
    let k = s.counters;
    let close = |n: u64, rate: f64| (n as f64 - rate * k.exposure).abs() <= 4.0 * (rate * k.exposure).sqrt().max(1.0);
    assert!(k.births > 100, "naissances : {}", k.births);
    assert!(close(k.births, s.rates.birth), "naissances {} pour {:.0}", k.births, s.rates.birth * k.exposure);
    assert!(close(k.deaths, s.rates.death), "morts {} pour {:.0}", k.deaths, s.rates.death * k.exposure);
    assert!(close(k.emigrations, s.rates.emigration), "départs {} pour {:.0}", k.emigrations, s.rates.emigration * k.exposure);
    // Même tirage à la même date.
    let again = Sample::draw(&w, c, i, 200).unwrap();
    assert_eq!(again.members, Sample::draw(&w, c, i, 200).unwrap().members);
    assert_eq!(w.state_hash(), before, "tirer un échantillon a changé le monde");

    // La zone regardée est active ; sans zone, pas de cellule regardée.
    assert!(active_cells(&w).iter().all(|x| x.1 != Reason::Observed));
    w.set_interest(Some(InterestZone { center_cell: 5, radius_km: 300.0, zoom_band: 6 }));
    let cells = active_cells(&w);
    assert_eq!(cells.first(), Some(&(w.bio.parent[5], Reason::Observed)));
}
