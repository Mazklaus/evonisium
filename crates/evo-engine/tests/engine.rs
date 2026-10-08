//! Le moteur vu du client : partie créée, ensemencée, intervention,
//! requêtes, point de sauvegarde et reprise.

use evo_engine::{Answer, Engine, Intervention, NewGame, OrderKind, Query, When};
use evo_sim::InterestZone;
use std::time::{Duration, Instant};

fn wait_for(engine: &Engine, what: &str, pred: impl Fn(&Engine) -> bool) {
    let t0 = Instant::now();
    while !pred(engine) {
        assert!(t0.elapsed() < Duration::from_secs(120), "attente trop longue : {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn small_game(seed: u64) -> NewGame {
    NewGame { seed, preset: "terre".into(), level: 3, threads: 2, sandbox: false }
}

#[test]
fn a_player_creates_seeds_intervenes_queries_and_saves() {
    let engine = Engine::new_game(small_game(5)).unwrap();
    let grid = engine.grid();
    assert_eq!(grid.len(), engine.frame().current.cells.len());
    assert!(engine.frame().current.paused);

    engine.submit(When::Now, OrderKind::SetStepYears(50_000.0));
    engine.submit(When::Now, OrderKind::SeedLife);
    let fert =
        engine.submit(When::At(200_000.0), OrderKind::Intervene(Intervention::Fertilize { cell: 7, radius_km: 3000.0, moles_p: 1e13 }));
    // Trop cher pour la réserve : refusé, avec un événement.
    let refused =
        engine.submit(When::At(200_000.0), OrderKind::Intervene(Intervention::Fertilize { cell: 9, radius_km: 3000.0, moles_p: 1e17 }));
    engine.set_interest(Some(InterestZone { center_cell: 7, radius_km: 2000.0, zoom_band: 2 }));
    engine.submit(When::Now, OrderKind::Resume);
    wait_for(&engine, "1,5 Ma", |e| e.frame().current.years >= 1.5e6);
    let frame = engine.frame();
    assert!(frame.previous.is_some());
    assert!(frame.current.globals.biomass > 0.0);
    assert!(!frame.current.species.is_empty());
    assert!(!frame.current.focus.cells.is_empty());
    assert!(frame.current.influence.points < 60.0 + 2.0 * 1.5);

    let Answer::Events(events) = engine.query(Query::Events { since_id: 0, min_interest: 0.0, limit: 1000 }).recv().unwrap() else {
        panic!()
    };
    assert!(events.iter().any(|e| e.type_name == "ordre refusé"), "{events:#?}");
    assert!(fert < refused);
    let species = frame.current.species[0].id;
    let Answer::Habitat(Some(h)) = engine.query(Query::SpeciesHabitat { species }).recv().unwrap() else { panic!() };
    assert!(h.living && h.temperature_k > 200.0);
    let Answer::SpeciesHistory(hist) = engine.query(Query::SpeciesHistory { species }).recv().unwrap() else { panic!() };
    assert!(!hist.is_empty());
    let Answer::RegionalHistory(reg) = engine.query(Query::RegionalHistory { cell: 7 }).recv().unwrap() else { panic!() };
    assert!(!reg.is_empty());

    let Answer::Cell(Some(cell)) = engine.query(Query::Cell { cell: 7 }).recv().unwrap() else { panic!() };
    assert_eq!(cell.cell, 7);
    let Answer::Lineages(lineages) = engine.query(Query::Lineages { since_years: 0.0 }).recv().unwrap() else { panic!() };
    assert!(!lineages.is_empty());
    assert_eq!(frame.current.species[0].signature, species);

    engine.submit(When::Now, OrderKind::Pause);
    wait_for(&engine, "pause", |e| e.frame().current.paused);
    let dir = std::env::temp_dir().join(format!("evonisium-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("partie.evo");
    engine.save(path.clone()).recv().unwrap().unwrap();
    let years = engine.frame().current.years;
    wait_for(&engine, "empreinte", |e| e.status().state_hash.is_some());
    let hash = engine.status().state_hash;
    drop(engine);

    let resumed = Engine::load(&path, 2).unwrap();
    assert_eq!(resumed.frame().current.years, years);
    let Answer::StateHash(h) = resumed.query(Query::StateHash).recv().unwrap() else { panic!() };
    assert_eq!(Some(h), hash);
    let Answer::SpeciesHistory(again) = resumed.query(Query::SpeciesHistory { species }).recv().unwrap() else { panic!() };
    assert_eq!(again, hist[..again.len()].to_vec());
    let _ = std::fs::remove_dir_all(dir);
}
