//! Rend les sons des villes (livraison 3, volet villes) dans des fichiers
//! WAV : une ville à cinq âges des techniques, vue de près sur le globe avec
//! la musique du jeu, puis le même rite d'un peuple joué avec les instruments
//! de quatre âges.
//!
//! `cargo run --release -p evo-son --example villes -- [dossier]`

// Affichage seulement, comme le crate : les mathématiques de la plateforme suffisent.
#![allow(clippy::disallowed_methods)]

use evo_son::peuples::{Langue, Peuple, Rite};
use evo_son::villes::Ville;
use evo_son::{Monde, Ordre, Son, Vue};
use std::io::Write;

fn wav(path: &std::path::Path, rate: u32, frames: &[[f32; 2]]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data = (frames.len() * 4) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    for v in [16u32, 1 | (2 << 16), rate, rate * 4, 4 | (16 << 16)] {
        f.write_all(&v.to_le_bytes())?;
    }
    f.write_all(b"data")?;
    f.write_all(&data.to_le_bytes())?;
    for fr in frames {
        for &x in fr {
            f.write_all(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
        }
    }
    Ok(())
}

const AGES: [(&str, &[&str]); 5] = [
    ("village", &["poterie", "elevage", "culture_cereales", "brique_crue"]),
    ("bronze", &["roue", "chariot", "cheval", "fonte_cuivre", "bronze", "biens_monnaie", "urbanisme"]),
    ("medievale", &["fonte_fer", "pieces", "moulin_eau", "moulin_vent", "equitation", "universite", "imprimerie"]),
    ("industrielle", &["machine_outil", "pompe_vapeur", "machine_vapeur", "filature_mecanique", "locomotive", "ciment_portland"]),
    ("metropole", &["reseau_electrique", "moteur_combustion", "automobile", "aviation", "radio", "transistor"]),
];

fn peuple(techniques: Vec<String>, population: f32) -> Peuple {
    Peuple {
        nom: "Kalo".into(),
        langue: Langue { mots: ["kalomi", "tsékou", "mira", "ouna", "pelak", "sanoé"].map(String::from).to_vec(), cle: 11 },
        rites: vec![Rite::depuis_nom("rite du Mira, pour les ancêtres")],
        population,
        taille: 1.0,
        cle: 11,
        techniques,
    }
}

fn main() -> std::io::Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "villes".into()));
    std::fs::create_dir_all(&out)?;
    let rate = 48_000.0;
    let monde = Monde { o2: 0.21, temperature_k: 288.0, ocean: 0.7, lineages: 400, ..Default::default() };
    let tailles = [3_000.0, 40_000.0, 150_000.0, 800_000.0, 5_000_000.0];
    let mut techniques: Vec<String> = Vec::new();
    for (k, (nom, cles)) in AGES.iter().enumerate() {
        techniques.extend(cles.iter().map(|c| c.to_string()));
        let population = tailles[k];
        let ville = Ville {
            nom: "Kalo".into(),
            cle: 1,
            population,
            urbain: 0.5,
            techniques: techniques.clone(),
            betail: 0.3,
            temple: 0.6,
            marche: 0.6,
            guerre: false,
            peuple: peuple(techniques.clone(), population),
        };
        let mut son = Son::new(rate, 4 + k as u64);
        son.ordre(Ordre::Monde(monde));
        son.ordre(Ordre::Vue(Vue::Globe));
        son.ordre(Ordre::Ville(Some((ville, 1.0))));
        let secondes = 75.0;
        let mut buf = vec![[0.0f32; 2]; (rate * secondes) as usize];
        let t = std::time::Instant::now();
        son.rendre(&mut buf);
        println!("ville {nom} : {:.1} % d'un cœur", t.elapsed().as_secs_f32() / secondes * 100.0);
        wav(&out.join(format!("ville-{k}-{nom}.wav")), rate as u32, &buf)?;
    }
    // Le même rite, joué avec les instruments de quatre âges.
    let ages: [(&str, &[&str]); 4] = [
        ("pierre", &[]),
        ("bronze", &["bronze", "fonte_cuivre"]),
        ("machines", &["bronze", "moulin_eau", "universite", "machine_outil"]),
        ("electronique", &["bronze", "machine_outil", "reseau_electrique", "transistor"]),
    ];
    for (k, (nom, cles)) in ages.iter().enumerate() {
        let p = peuple(cles.iter().map(|c| c.to_string()).collect(), 5000.0);
        let mut son = Son::new(rate, 9);
        son.ordre(Ordre::Monde(monde));
        son.ordre(Ordre::Vue(Vue::Sol));
        son.ordre(Ordre::Peuple(Some(p)));
        son.ordre(Ordre::Rite(0));
        let mut buf = vec![[0.0f32; 2]; (rate * 50.0) as usize];
        son.rendre(&mut buf);
        let fin = buf.iter().rposition(|f| f[0].abs() > 1e-3).unwrap_or(buf.len());
        buf.truncate((fin + rate as usize).min(buf.len()));
        println!("rite, âge {nom} : {:.0} s", buf.len() as f32 / rate);
        wav(&out.join(format!("rite-{k}-{nom}.wav")), rate as u32, &buf)?;
    }
    Ok(())
}
