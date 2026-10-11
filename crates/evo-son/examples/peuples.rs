//! Rend les sons de quelques peuples d'essai (banc d'`evo-culture`) dans des
//! fichiers WAV : la rumeur de chacun au sol, son nom prononcé, ses chants
//! rituels.
//!
//! `cargo run --release -p evo-son --example peuples -- [dossier] [peuples]`

use evo_culture::bench::{grid, seed_technique};
use evo_culture::catalog::{Context, Family};
use evo_culture::cognition::presets;
use evo_culture::traits::TraitKind;
use evo_culture::{CulturalState, CultureWorld, Organ};
use evo_son::peuples::{Langue, Peuple, Rite};
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

/// Peuples vivants de l'état culturel, comme le son les lit.
fn peuples(st: &CulturalState) -> Vec<Peuple> {
    let mut out = Vec::new();
    for p in st.peoples.iter().filter(|p| p.ended_year.is_none()) {
        let Some(l) = p.language.and_then(|l| st.languages.iter().find(|x| x.id == l)) else { continue };
        // Les rites les plus portés du peuple (un rite reste souvent
        // propre à quelques groupes).
        let mut compte: Vec<(u32, usize)> = Vec::new();
        for g in st.groups.iter().filter(|g| g.people == p.id) {
            for t in &g.traits {
                if st.traits.iter().any(|v| v.id == t.id && v.kind == TraitKind::Rite) {
                    match compte.iter_mut().find(|c| c.0 == t.id) {
                        Some(c) => c.1 += 1,
                        None => compte.push((t.id, 1)),
                    }
                }
            }
        }
        compte.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let rites: Vec<Rite> =
            compte.iter().take(4).filter_map(|c| st.traits.iter().find(|v| v.id == c.0)).map(|v| Rite::depuis_nom(&v.name)).collect();
        out.push(Peuple {
            nom: p.name.clone(),
            langue: Langue { mots: l.words.iter().map(|w| w.1.clone()).collect(), cle: l.id as u64 },
            rites,
            population: p.population,
            taille: 1.0,
            cle: p.id as u64,
        });
    }
    out.sort_by(|a, b| b.rites.len().cmp(&a.rites.len()).then(b.population.total_cmp(&a.population)));
    out
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(args.get(1).map_or("peuples", |s| s.as_str()));
    let combien: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    std::fs::create_dir_all(&dir)?;

    let mut w = CultureWorld::new(2026);
    let land = Context::land(Organ::Hand);
    // Six régions sans contact : six peuples, six langues.
    for r in 0..6u32 {
        let cells = grid(&mut w, 1, presets::human(), r * 1000, 6, 5, 900.0, 0.3, land);
        for f in [Family::Percussion, Family::Flake, Family::Fire] {
            seed_technique(&mut w, 1, &cells, f, 0.8, 1.0);
        }
    }
    for _ in 0..6 {
        w.step(10_000.0);
    }
    for _ in 0..200 {
        w.step(1.0);
    }
    let st = CulturalState::of(&w);
    let liste = peuples(&st);
    println!("{} peuples vivants", liste.len());

    let rate = 44_100u32;
    let monde = Monde { o2: 0.21, temperature_k: 288.0, ice: 0.05, ocean: 0.7, lineages: 400, paused: false };
    for (i, p) in liste.iter().take(combien).enumerate() {
        let tr = evo_son::peuples::Tradition::de(p.cle);
        println!(
            "{} : mots {:?}, instruments {:?}, manières {:?}",
            p.nom,
            &p.langue.mots[..5.min(p.langue.mots.len())],
            tr.instruments.iter().map(|i| i.nom()).collect::<Vec<_>>(),
            tr.formes.iter().map(|f| f.nom()).collect::<Vec<_>>()
        );
        let rendre = |son: &mut Son, secondes: f32| {
            let mut buf = vec![[0.0f32; 2]; (rate as f32 * secondes) as usize];
            for c in buf.chunks_mut(512) {
                son.rendre(c);
            }
            buf
        };
        // La rumeur au sol, puis le nom du peuple.
        let mut son = Son::new(rate as f32, 40 + i as u64);
        son.ordre(Ordre::Monde(monde));
        son.ordre(Ordre::Vue(Vue::Sol));
        son.ordre(Ordre::Peuple(Some(p.clone())));
        let mut buf = rendre(&mut son, 20.0);
        son.ordre(Ordre::Prononcer(p.nom.clone()));
        buf.extend(rendre(&mut son, 3.0));
        wav(&dir.join(format!("{}-rumeur.wav", i + 1)), rate, &buf)?;
        // Les voix seules, sans musique, pour juger la prononciation.
        let mut son = Son::new(rate as f32, 50 + i as u64);
        son.ordre(Ordre::Volumes([0.8, 0.0, 0.8, 0.8, 0.0]));
        son.ordre(Ordre::Vue(Vue::Sol));
        son.ordre(Ordre::Peuple(Some(p.clone())));
        son.ordre(Ordre::Prononcer(p.nom.clone()));
        let buf = rendre(&mut son, 10.0);
        wav(&dir.join(format!("{}-voix.wav", i + 1)), rate, &buf)?;
        // Ses chants.
        // Deux rites d'objets différents, pour entendre deux caractères.
        let mut objets: Vec<&str> = Vec::new();
        let choisis: Vec<usize> = (0..p.rites.len())
            .filter(|&r| {
                let neuf = !objets.contains(&p.rites[r].objet.as_str());
                objets.push(&p.rites[r].objet);
                neuf
            })
            .take(2)
            .collect();
        for (k, &r) in choisis.iter().enumerate() {
            let mut son = Son::new(rate as f32, 70 + i as u64);
            son.ordre(Ordre::Monde(monde));
            son.ordre(Ordre::Vue(Vue::Sol));
            son.ordre(Ordre::Peuple(Some(p.clone())));
            son.ordre(Ordre::Rite(r));
            let forme = son.chant().map_or("?", |f| f.nom());
            let t = std::time::Instant::now();
            let buf = rendre(&mut son, 60.0);
            println!("  {} ({forme}) : {:.1} % d'un cœur", p.rites[r].nom, 100.0 * t.elapsed().as_secs_f64() / 60.0);
            wav(&dir.join(format!("{}-chant-{}.wav", i + 1, k + 1)), rate, &buf)?;
        }
    }
    Ok(())
}
