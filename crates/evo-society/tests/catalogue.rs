//! Le catalogue des techniques : il se lit, chaque nœud est atteignable, et
//! les voies de remplacement existent pour les corps et les milieux
//! exotiques.

use evo_society::catalog::{catalog, Known};

#[test]
fn le_catalogue_se_lit_et_tout_est_atteignable() {
    let cat = catalog();
    assert!(cat.len() >= 200, "{} techniques seulement", cat.len());
    // En ignorant le milieu, tout nœud doit être atteignable depuis rien.
    let mut known = Known::default();
    loop {
        let before = known.count();
        for t in 0..cat.len() as u16 {
            if !known.has(t) && cat.ready(t, &known) {
                known.set(t);
            }
        }
        if known.count() == before {
            break;
        }
    }
    let missing: Vec<&str> = (0..cat.len() as u16).filter(|&t| !known.has(t)).map(|t| cat.get(t).key.as_str()).collect();
    assert!(missing.is_empty(), "inatteignables : {missing:?}");
}

#[test]
fn des_voies_de_remplacement_existent() {
    let cat = catalog();
    let route = |p: &str| (0..cat.len() as u16).filter(|&t| cat.get(t).key.starts_with(p)).count();
    // Aquatique, pauvre en oxygène, souterrain, sans mains.
    for (p, min) in [("aq_", 8), ("po_", 5), ("st_", 4), ("sm_", 3)] {
        assert!(route(p) >= min, "voie {p} : {} nœuds", route(p));
    }
    // Sans feu, l'aquatique atteint l'électricité et un réseau.
    let net = (0..cat.len() as u16).find(|&t| cat.get(t).key == "aq_reseau").expect("aq_reseau");
    assert!(cat.get(net).flags.network);
}

#[test]
fn l_ecriture_et_la_fin_de_la_v1_sont_marquees() {
    let cat = catalog();
    let has = |f: fn(&evo_society::catalog::Flags) -> bool| (0..cat.len() as u16).filter(|&t| f(&cat.get(t).flags)).count();
    assert!(has(|f| f.writing) >= 3);
    assert!(has(|f| f.fossil) >= 2);
    assert!(has(|f| f.network) >= 2);
    assert!(has(|f| f.agriculture) >= 4);
}
