//! Bancs d'essai de la porte de l'étape 6, côté modèle agrégé.

use evo_culture::bench::{grid, seed_technique};
use evo_culture::catalog::{Context, Family, Milieu};
use evo_culture::cognition::presets;
use evo_culture::world::CultureWorld;
use evo_culture::{CulturalState, Organ, Phenomenon};

const HUMAN: u32 = 1;

/// Cas tasmanien (Henrich, 2004) : une île de 4 000 personnes coupée du
/// continent perd ses techniques les plus difficiles (outils d'os,
/// vêtements cousus, pêche, propulseurs) et garde les simples ; le continent
/// garde tout.
#[test]
fn tasmanian_loss() {
    let mut w = CultureWorld::new(41);
    let land = Context::land(Organ::Hand);
    let mainland = grid(&mut w, HUMAN, presets::human(), 0, 6, 6, 4000.0, 1.0, land);
    w.add_group(100, HUMAN, 4000.0, land, Some(0));
    w.link(100, 0, 1.0);
    let all = [
        Family::Percussion,
        Family::Flake,
        Family::Fire,
        Family::Container,
        Family::Cord,
        Family::Spear,
        Family::BoneTool,
        Family::Clothing,
        Family::Fishing,
        Family::Thrower,
    ];
    let mut cells = mainland.clone();
    cells.push(100);
    for f in all {
        seed_technique(&mut w, HUMAN, &cells, f, 0.9, 1.0);
    }
    // Montée des eaux : l'île est coupée.
    w.link(100, 0, 0.0);
    for _ in 0..20_000 {
        w.step(1.0);
    }
    let island = w.group(100, HUMAN).unwrap();
    let centre = w.group(mainland[14], HUMAN).unwrap();
    let hard = [Family::BoneTool, Family::Clothing, Family::Fishing, Family::Thrower];
    let easy = [Family::Percussion, Family::Flake, Family::Fire, Family::Spear];
    for f in hard {
        assert!(!island.knows(f), "l'île garde {f:?}");
        assert!(centre.knows(f), "le continent perd {f:?}");
    }
    for f in easy {
        assert!(island.knows(f), "l'île perd {f:?}");
    }
    // L'île est devenue un autre peuple, avec une autre langue.
    assert_ne!(island.language, centre.language, "langue de l'île");
    assert_ne!(island.people, centre.people, "peuple de l'île");
    let st = CulturalState::of(&w);
    let p = st.peoples.iter().find(|p| p.id == island.people).unwrap();
    assert!(p.parent.is_some(), "le peuple de l'île descend de celui du continent");
}

/// Cas de Koshima : le lavage des patates douces, inventé en 1953 par une
/// jeune femelle, gagne la moitié de la troupe en quelques années et se
/// stabilise vers les trois quarts.
#[test]
fn koshima_diffusion() {
    let mut halves = Vec::new();
    let mut plateau = Vec::new();
    for seed in 0..200u64 {
        let mut w = CultureWorld::new(seed);
        w.add_species(2, presets::macaque());
        w.add_group(0, 2, 60.0, Context::land(Organ::Hand), None);
        w.seed_trait(0, 2, Family::Percussion as u32, 1.0 / 60.0, 1.0);
        let mut half = None;
        for y in 1..=20 {
            w.step(1.0);
            let f = w.group(0, 2).unwrap().slot(Family::Percussion as u32).map(|s| s.freq).unwrap_or(0.0);
            if half.is_none() && f >= 0.5 {
                half = Some(y);
            }
            if y == 20 && f > 0.0 {
                plateau.push(f);
            }
        }
        if let Some(h) = half {
            halves.push(h);
        }
    }
    halves.sort();
    let median = halves[halves.len() / 2];
    let mean_plateau = plateau.iter().sum::<f64>() / plateau.len() as f64;
    eprintln!("Koshima : {} diffusions sur 200, médiane à mi-troupe {median} ans, palier {mean_plateau:.2}", halves.len());
    assert!(halves.len() > 100, "l'invention se perd trop souvent : {}", halves.len());
    assert!((3..=10).contains(&median), "moitié de la troupe en {median} ans");
    assert!((0.7..0.95).contains(&mean_plateau), "palier {mean_plateau}");
}

/// Traditions : sur deux mondes (une terre de chimpanzés et de corbeaux, une
/// mer de dauphins), des techniques sont inventées, jamais posées, et
/// transmises au moins dix générations ; elles diffèrent selon la lignée.
#[test]
fn traditions_in_several_lineages() {
    for (world, species) in [
        ("terre", vec![(3, presets::chimpanzee(), Organ::Hand, Milieu::Land), (4, presets::crow(), Organ::Beak, Milieu::Land)]),
        ("mer", vec![(5, presets::dolphin(), Organ::Mouth, Milieu::Water)]),
    ] {
        let mut w = CultureWorld::new(7);
        for (i, &(id, cog, organ, milieu)) in species.iter().enumerate() {
            let ctx = Context { milieu, o2_percent: 21.0, organ };
            grid(&mut w, id, cog, 1000 * i as u32, 8, 5, 300.0, 0.3, ctx);
        }
        for _ in 0..100 {
            w.step(10_000.0);
        }
        for &(id, ..) in &species {
            let mut best = 0.0f64;
            let mut known = std::collections::BTreeSet::new();
            for g in w.groups.iter().filter(|g| g.species == id) {
                for s in &g.traits {
                    if s.id < 64 {
                        best = best.max(w.generations(g, s));
                        known.insert(s.id);
                    }
                }
            }
            eprintln!("{world}, espèce {id} : techniques {known:?}, plus vieille tradition {best:.0} générations");
            assert!(best >= 10.0, "{world}, espèce {id} : aucune tradition de dix générations");
        }
        assert!(w.counters.inventions > 0);
    }
}

/// Bascule : une espèce à langage, en bandes reliées, a un gain culturel
/// positif deux pas de suite et passe en mode historique ; des chimpanzés,
/// jamais.
#[test]
fn switch_to_historical_mode() {
    let mut w = CultureWorld::new(9);
    let land = Context::land(Organ::Hand);
    grid(&mut w, 3, presets::chimpanzee(), 0, 6, 6, 500.0, 0.5, land);
    w.step(10_000.0);
    assert!(!w.historical());
    grid(&mut w, HUMAN, presets::human(), 100, 4, 4, 200.0, 0.3, land);
    w.step(10_000.0);
    assert!(!w.historical(), "un seul pas positif ne suffit pas");
    w.step(10_000.0);
    assert!(w.historical());
    assert!(w.species(HUMAN).unwrap().historical());
    assert!(!w.species(3).unwrap().historical());
}

/// Une intervention vue par un peuple fait naître un mythe ; une espèce sans
/// pensée symbolique la note seulement inexpliquée.
#[test]
fn intervention_gives_a_myth() {
    let mut w = CultureWorld::new(13);
    let land = Context::land(Organ::Hand);
    grid(&mut w, HUMAN, presets::human(), 0, 3, 3, 300.0, 0.5, land);
    grid(&mut w, 3, presets::chimpanzee(), 0, 1, 1, 300.0, 0.5, land);
    let mut myths = Vec::new();
    for p in [Phenomenon::Rain, Phenomenon::Impact, Phenomenon::Eruption] {
        myths.extend(w.witness(0, p, true));
        w.step(1.0);
    }
    assert!(!myths.is_empty(), "aucun mythe");
    let st = CulturalState::of(&w);
    let m = &st.myths[0];
    eprintln!("{} : {}", m.title, m.text);
    assert!(m.intervention);
    assert!(st.unexplained.iter().any(|u| u.myth.is_none()), "le chimpanzé ne fait pas de mythe");
    // Le mythe est un trait du groupe.
    assert!(st.traits.iter().any(|t| matches!(t.kind, evo_culture::traits::TraitKind::Myth(_))));
}

/// Rejeu : même graine, même histoire, que l'état soit lu ou non, sur un
/// cœur ou plusieurs.
#[test]
fn replay_is_identical() {
    let run = |observe: bool| {
        let mut w = CultureWorld::new(5);
        let land = Context::land(Organ::Hand);
        let cells = grid(&mut w, HUMAN, presets::human(), 0, 5, 5, 800.0, 0.3, land);
        seed_technique(&mut w, HUMAN, &cells[..3], Family::Fire, 0.5, 0.8);
        w.step(10_000.0);
        w.step(10_000.0);
        // Une île coupée du continent : un autre peuple, dont les techniques
        // reviennent par des passeurs quand on la relie.
        w.add_group(99, HUMAN, 3000.0, land, None);
        for y in 0..1500 {
            if y == 600 {
                w.link(99, 0, 0.3);
            }
            w.step(1.0);
            if y == 150 {
                w.witness(7, Phenomenon::Light, true);
            }
            if observe {
                let _ = CulturalState::of(&w);
            }
        }
        CulturalState::of(&w)
    };
    let a = run(false);
    let b = run(true);
    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(a, b);
    // Le nombre de cœurs ne change rien.
    let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap().install(|| run(false));
    assert_eq!(a, one);
    assert!(!a.notables.is_empty(), "aucun notable en 1 500 ans de mode historique");
}
