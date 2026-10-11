//! Imprime des textes du narrateur, une phrase par ligne, pour les écouter :
//! `cargo run -p evo-son --example recit`.

use evo_son::recit::*;

fn main() {
    let large = Lieu {
        region: "Torvax".into(),
        mer: true,
        lac: false,
        glace: false,
        temperature_c: 21.0,
        hauteur_m: -40.0,
        source: false,
        lumiere: 90.0,
    };
    let source = Lieu { region: "Brakor".into(), source: true, hauteur_m: -2400.0, temperature_c: 9.0, lumiere: 0.0, ..large.clone() };
    println!("# moment");
    for s in moment(&Fait::Innovation { chemin: Chemin::Photosynthese, etape: 4, region: "Torvax".into() }, 1.42e9, 7) {
        println!("{s}");
    }
    println!("# fiche");
    let e = Espece {
        nom: "Methanobius brakorensis".into(),
        metabolismes: vec![Metab::Methanogene],
        pigmentee: false,
        age_ans: Some(3.2e8),
        region_origine: "Brakor".into(),
        parent: Some("Protobius kaensis".into()),
        aire: 0.08,
        tendance: Tendance::Expansion,
        ecotypes: 3,
    };
    let voisins = [
        Presence { nom: e.nom.clone(), metab: Some(Metab::Methanogene), part: 0.6 },
        Presence { nom: "Methylophagus brakorensis".into(), metab: Some(Metab::Methanotrophe), part: 0.3 },
    ];
    for s in fiche(&e, &source, &voisins, 0) {
        println!("{s}");
    }
    println!("# scene");
    let p = [
        Presence { nom: "Oxyphycus torvaxensis".into(), metab: Some(Metab::PhotoOxygene), part: 0.7 },
        Presence { nom: "Zymobius torvaxensis".into(), metab: Some(Metab::Fermentation), part: 0.2 },
        Presence { nom: "Desulfobius kaensis".into(), metab: Some(Metab::Sulfato), part: 0.1 },
    ];
    for s in scene(&large, &p, 0) {
        println!("{s}");
    }
}
