//! Tests historiques de l'étape 7, sur des peuples d'essai : ils forment la
//! porte de l'étape (doc d'architecture, « Porte de l'étape 7 »).
//!
//! Les essais longs (cinq Terres de −15 000 à aujourd'hui, monde désertique,
//! vitesse) sont marqués `#[ignore]` : `cargo test --release -p evo-society
//! --test histoire -- --ignored --nocapture`. Les autres tournent à chaque
//! PR.

use evo_society::bench::{self, Bench, HUMANS, OTHERS};
use evo_society::catalog::{catalog, Resource};
use evo_society::chronicle::EventKind;
use evo_society::society::Label;
use evo_society::state::SocietyState;
use evo_society::world::RelationKind;
use std::time::Instant;

/// Ce qu'on retient d'une histoire de Terre.
struct Earth {
    seed: u64,
    hearths: Vec<(i64, u32)>,
    independent: usize,
    first_state: Option<i64>,
    collapses: u64,
    takeoff: Option<i64>,
    /// (année, population, revenu par tête) tous les cent ans.
    samples: Vec<(i64, f64, f64)>,
    max_societies: usize,
    /// Forçage humain à la fin, première année au pas climatique de 10 ans,
    /// bilan carbone rapporté au stock initial.
    forcing: f64,
    step10: Option<i64>,
    carbon: f64,
    /// Première année où le CO₂ fossile est le premier forçage humain.
    fossil_lead: Option<i64>,
    v1: Option<i64>,
}

fn income(b: &Bench) -> (f64, f64) {
    let mut pop = 0.0;
    let mut gdp = 0.0;
    for s in b.world.cells.iter().filter_map(|c| c.settlement.as_ref()) {
        pop += s.pop;
        gdp += s.pop * s.gdp_pc;
    }
    (pop, gdp / pop.max(1.0))
}

fn run_earth(seed: u64, until: i64) -> Earth {
    let mut b = bench::earth(seed);
    let mut samples = Vec::new();
    let mut max_societies = 0;
    let t0 = Instant::now();
    let mut step10 = None;
    let mut fossil_lead = None;
    while b.world.year < until {
        b.step();
        if step10.is_none() && b.world.climate_step_years() == 10 {
            step10 = Some(b.world.year);
        }
        if fossil_lead.is_none() && b.world.forcing_parts().fossil_leads() {
            fossil_lead = Some(b.world.year);
        }
        if b.world.year % 100 == 0 {
            let (pop, inc) = income(&b);
            samples.push((b.world.year, pop, inc));
            max_societies = max_societies.max(b.world.alive_societies().count());
        }
        if b.world.year % 2000 == 0 {
            eprintln!("  graine {seed} : an {} ({:.0} s)", b.world.year, t0.elapsed().as_secs_f64());
        }
    }
    let hearths: Vec<(i64, u32)> = b.world.hearths().iter().map(|e| (e.year, e.cell.unwrap_or(0))).collect();
    // Foyers indépendants : à plus de 2 500 km de tout foyer antérieur.
    let mut kept: Vec<u32> = Vec::new();
    for &(_, c) in &hearths {
        if kept.iter().all(|&k| b.km(k, c) > 2_500.0) {
            kept.push(c);
        }
    }
    let first_state =
        b.world.milestones.iter().find(|e| e.kind == EventKind::Label && e.text.contains(Label::State.label())).map(|e| e.year);
    Earth {
        seed,
        independent: kept.len(),
        hearths,
        first_state,
        collapses: b.world.counters.collapses,
        takeoff: b.world.takeoff.get(&HUMANS).copied(),
        samples,
        max_societies,
        forcing: b.world.forcing_w_m2(),
        step10,
        carbon: b.world.carbon_balance() / b.world.carbon_initial,
        fossil_lead,
        v1: b.world.v1.get(&HUMANS).copied(),
    }
}

/// Plateau malthusien : avant la sortie industrielle, le revenu par tête
/// reste borné pendant que la population croît.
fn plateau(e: &Earth) -> Option<(f64, f64)> {
    let start = e.first_state?;
    let end = e.takeoff.unwrap_or(i64::MAX);
    let pre: Vec<&(i64, f64, f64)> = e.samples.iter().filter(|s| s.0 >= start && s.0 < end).collect();
    if pre.len() < 20 {
        return None;
    }
    let tail = &pre[pre.len().saturating_sub(30)..];
    let (lo, hi) = tail.iter().fold((f64::MAX, 0.0f64), |(lo, hi), s| (lo.min(s.2), hi.max(s.2)));
    let growth = pre.last()?.1 / pre.first()?.1.max(1.0);
    Some((hi / lo.max(1e-9), growth))
}

#[test]
#[ignore]
fn porte_cinq_terres() {
    let mut report = Vec::new();
    for seed in 1..=5 {
        let e = run_earth(seed, 2_100);
        let agri = e.hearths.first().map(|h| h.0);
        let delay = match (agri, e.first_state) {
            (Some(a), Some(s)) => Some(s - a),
            _ => None,
        };
        let p = plateau(&e);
        eprintln!(
            "graine {} : foyers {:?} ({} indépendants), premier État {:?} ({:?} ans après), effondrements {}, sortie {:?}, plateau {:?}, sociétés au plus {}, forçage {:.2} W/m², pas de 10 ans dès {:?}, CO₂ fossile en tête dès {:?}, fin de la v1 {:?}, bilan carbone {:.1e}",
            e.seed, e.hearths, e.independent, e.first_state, delay, e.collapses, e.takeoff, p, e.max_societies, e.forcing, e.step10, e.fossil_lead, e.v1, e.carbon
        );
        report.push((e, delay, p));
    }
    for (e, delay, p) in &report {
        assert!(e.independent >= 2, "graine {} : {} foyer(s) indépendant(s)", e.seed, e.independent);
        let d = delay.unwrap_or_else(|| panic!("graine {} : pas d'État", e.seed));
        assert!((3_000..=10_000).contains(&d), "graine {} : État {d} ans après l'agriculture", e.seed);
        assert!(e.collapses >= 1, "graine {} : aucun effondrement", e.seed);
        let (ratio, growth) = p.unwrap_or_else(|| panic!("graine {} : pas assez d'histoire avant la sortie", e.seed));
        assert!(ratio < 1.8 && growth > 1.5, "graine {} : pas de plateau malthusien (revenu ×{ratio:.2}, population ×{growth:.2})", e.seed);
        // Les flux humains changent le climat ; le carbone ne se perd pas.
        assert!(e.forcing > 0.1 && e.step10.is_some(), "graine {} : forçage {:.3}", e.seed, e.forcing);
        assert!(e.carbon.abs() < 1e-9, "graine {} : bilan carbone {:.1e}", e.seed, e.carbon);
        // Là où le charbon existe, le CO₂ fossile devient le premier forçage
        // moins de 300 ans après le début de l'industrie (point ajouté par
        // Vision ; vérification finale avec les vrais gisements).
        let t = e.takeoff.unwrap_or_else(|| panic!("graine {} : pas de sortie industrielle", e.seed));
        let lead = e.fossil_lead.unwrap_or_else(|| panic!("graine {} : le CO₂ fossile n'est jamais en tête", e.seed));
        assert!(lead - t <= 300, "graine {} : CO₂ fossile en tête {} ans après la sortie", e.seed, lead - t);
        // La fin de la v1 vient 150 à 350 ans après le début de l'industrie
        // (point ajouté par Vision).
        let v1 = e.v1.unwrap_or_else(|| panic!("graine {} : pas de fin de la v1", e.seed));
        assert!((150..=350).contains(&(v1 - t)), "graine {} : fin de la v1 {} ans après la sortie", e.seed, v1 - t);
    }
}

/// Techniques connues d'une espèce à la fin d'une histoire.
fn known_by(b: &Bench, species: u32) -> Vec<u16> {
    let cat = catalog();
    (0..cat.len() as u16)
        .filter(|&t| b.world.cells.iter().filter_map(|c| c.settlement.as_ref()).any(|s| s.species == species && s.known.has(t)))
        .collect()
}

#[test]
#[ignore]
fn un_monde_desertique_prend_un_autre_chemin() {
    let mut d = bench::desert(11);
    for _ in 0..12_000 {
        d.step();
    }
    let cat = catalog();
    let known = known_by(&d, HUMANS);
    let names: Vec<&str> = known.iter().map(|&t| cat.get(t).key.as_str()).collect();
    eprintln!("désert : {} techniques : {}", known.len(), names.join(" "));
    // Pas de charbon ni de pétrole : aucune technique qui en demande.
    assert!(known.iter().all(|&t| !cat.get(t).resources.iter().any(|r| matches!(r, Resource::Coal | Resource::Oil))));
    // Et une voie propre : le soleil et le vent.
    assert!(
        known.iter().any(|&t| cat.get(t).resources.contains(&Resource::Sun) || cat.get(t).resources.contains(&Resource::Wind)),
        "aucune technique du soleil ou du vent"
    );
}

#[test]
fn un_monde_ocean_suit_la_voie_aquatique() {
    let mut o = bench::ocean(5);
    for _ in 0..3_000 {
        o.step();
    }
    let cat = catalog();
    let known = known_by(&o, OTHERS);
    let names: Vec<&str> = known.iter().map(|&t| cat.get(t).key.as_str()).collect();
    eprintln!("océan : {}", names.join(" "));
    // Au-delà des coquillages et des coraux du départ.
    assert!(known.iter().filter(|&&t| cat.get(t).key.starts_with("aq_")).count() >= 3, "voie aquatique absente : {names:?}");
    assert!(known.iter().all(|&t| !cat.get(t).body.combustion), "du feu sous l'eau : {names:?}");
}

#[test]
fn deux_especes_se_rencontrent() {
    let mut b = bench::two_species(3);
    let mut met = None;
    let mut dealt = false;
    for _ in 0..4_000 {
        b.step();
        if met.is_none() && !b.world.contacts.is_empty() {
            met = Some(b.world.year);
        }
        let w = &b.world;
        dealt |= w.relations.iter().any(|(k, r)| {
            w.societies[k.0 as usize].species != w.societies[k.1 as usize].species
                && matches!(r.kind, RelationKind::Trade | RelationKind::War { .. } | RelationKind::Alliance | RelationKind::Tribute { .. })
        });
        if met.is_some() && dealt {
            break;
        }
    }
    let year = met.expect("les deux espèces ne se sont jamais rencontrées");
    assert!(dealt, "rencontre en {year}, mais ni commerce ni guerre");
    let first = b.world.milestones.iter().find(|e| e.kind == EventKind::FirstContact).expect("premier contact raconté");
    eprintln!("{} {}", first.year, first.text);
}

#[test]
fn le_rejeu_est_identique_au_bit_pres() {
    // Une histoire lue à chaque pas sur un cœur, l'autre jamais lue sur
    // quatre : même empreinte.
    let run = |threads: usize, read: bool| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let mut b = bench::modern(42);
            let mut seen = 0usize;
            for _ in 0..60 {
                b.step();
                if read {
                    let s = SocietyState::of(&b.world);
                    seen += s.cells.len() + s.notables.len();
                }
            }
            (b.world.fingerprint(), b.world.notables.len(), seen)
        })
    };
    let (a, na, seen) = run(1, true);
    let (b, nb, _) = run(4, false);
    assert!(seen > 0);
    assert!(na > 0, "pas de notables");
    assert_eq!((a, na), (b, nb));
}

#[test]
fn l_etat_publie_fait_l_aller_retour() {
    let mut b = bench::two_species(9);
    for _ in 0..200 {
        b.step();
    }
    let s = SocietyState::of(&b.world);
    assert_eq!(s.version, evo_society::state::FORMAT_VERSION);
    let bytes = bincode::serialize(&s).unwrap();
    let back: SocietyState = bincode::deserialize(&bytes).unwrap();
    assert_eq!(s, back);
    assert!(!s.societies.is_empty() && !s.cells.is_empty());
}

#[test]
fn les_flux_humains_changent_le_climat_et_les_sols_sans_perdre_de_carbone() {
    let mut b = bench::modern(7);
    let soil0: f64 = b.world.cells.iter().map(|c| c.eco.soil).sum();
    let mut step10 = None;
    let mut fossil_lead = None;
    for _ in 0..250 {
        b.step();
        if step10.is_none() && b.world.climate_step_years() == 10 {
            step10 = Some(b.world.year);
        }
        if fossil_lead.is_none() && b.world.forcing_parts().fossil_leads() {
            fossil_lead = Some(b.world.year);
        }
    }
    let w = &b.world;
    let soil: f64 = w.cells.iter().map(|c| c.eco.soil).sum();
    eprintln!(
        "an {} : forçage {:.2} W/m² ({:?}), CO₂ fossile en tête dès {:?}, pas du climat à 10 ans dès {:?}, CO₂ cumulé {:.2e} t C, sols {:.1} → {:.1}, érosion {:.2e} t, bilan {:.3e} t C sur {:.3e}",
        w.year,
        w.forcing_w_m2(),
        w.forcing_parts(),
        fossil_lead,
        step10,
        w.totals.cum_co2_c,
        soil0,
        soil,
        w.totals.cum_erosion_t,
        w.carbon_balance(),
        w.carbon_initial
    );
    // Le forçage fort se vérifie sur les cinq Terres (porte) ; ici, en
    // 250 ans d'industrie naissante, il doit au moins apparaître.
    assert!(w.forcing_w_m2() > 0.01, "forçage {}", w.forcing_w_m2());
    let _ = step10;
    // Sur une Terre de 1750 riche en charbon, le CO₂ fossile passe en tête
    // dans les 300 ans.
    assert!(fossil_lead.is_some_and(|y| y <= 2050), "CO₂ fossile en tête : {fossil_lead:?}");
    assert!((soil - soil0).abs() > 1.0 && w.totals.cum_erosion_t > 0.0);
    assert!(w.carbon_balance().abs() < 1e-9 * w.carbon_initial, "bilan {}", w.carbon_balance());
}

#[test]
#[ignore]
fn la_fin_de_la_v1_est_racontee_et_la_simulation_continue() {
    let mut b = bench::modern(3);
    let mut end = None;
    for _ in 0..800 {
        b.step();
        if end.is_none() {
            end = b.world.v1.get(&HUMANS).copied();
            if let Some(y) = end {
                eprintln!("fin de la v1 en {y}");
            }
        }
        if end.is_some_and(|y| b.world.year > y + 50) {
            break;
        }
    }
    let y = end.expect("fin de la v1 jamais atteinte");
    let e = b.world.milestones.iter().find(|e| e.kind == EventKind::EndOfFirstVersion).expect("fin racontée");
    eprintln!("{}", e.text);
    assert!(b.world.year > y + 50);
    assert!(b.world.alive_societies().count() > 0);
}

#[test]
#[ignore]
fn vitesse_du_mode_historique() {
    let mut b = bench::earth(1);
    for _ in 0..200 {
        b.step();
    }
    let n = b.world.alive_societies().count();
    let t0 = Instant::now();
    for _ in 0..500 {
        b.step();
    }
    let rate = 500.0 / t0.elapsed().as_secs_f64();
    let bytes = bincode::serialize(&b.world).unwrap().len();
    eprintln!("{rate:.0} ans/s avec {n} sociétés ; état {:.1} Mo", bytes as f64 / 1e6);
    assert!(n >= 300);
    assert!(rate >= 100.0, "{rate:.0} ans/s");
}
