//! Rend les trois timbres des petites phrases de la musique générée (flûte
//! douce, harpe feutrée, verre doux), seuls puis sous une boucle du fond,
//! pour choisir à l'oreille celui qui remplace la kalimba.
//!
//! `cargo run --release -p evo-son --example phrases -- [dossier]`

// Affichage seulement, comme le crate : les mathématiques de la plateforme suffisent.
#![allow(clippy::disallowed_methods)]

use evo_son::fond::{liste, Boucle, Contenu};
use evo_son::musique::Phrase;
use evo_son::{Monde, Ordre, Son, Vue};
use std::io::Write;
use std::sync::Arc;

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

fn main() -> std::io::Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "phrases".into()));
    std::fs::create_dir_all(&out)?;
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../client/musique/fond");
    let entrees = liste(&std::fs::read_to_string(dossier.join("boucles.txt"))?);
    let rate = 48_000.0;
    for (k, phrase) in Phrase::TOUTES.into_iter().enumerate() {
        for avec_boucle in [false, true] {
            let mut son = Son::new(rate, 11);
            son.musique.phrase = phrase;
            son.ordre(Ordre::Monde(Monde { o2: 0.15, temperature_k: 288.0, ocean: 0.7, lineages: 400, ..Default::default() }));
            son.ordre(Ordre::Vue(Vue::Globe));
            son.ordre(Ordre::Ere(4));
            if avec_boucle {
                let x = entrees.iter().find(|x| x.ere == 4).expect("une boucle des terres et forêts");
                let ogg = std::fs::read(dossier.join(&x.fichier))?;
                son.ordre(Ordre::Boucle(Boucle {
                    ere: 4,
                    son: Contenu::Ogg(Arc::new(ogg)),
                    tonique: x.tonique,
                    mode: x.mode,
                    gain: x.gain,
                }));
            }
            let mut buf = vec![[0.0f32; 2]; (rate * 60.0) as usize];
            for (b, bloc) in buf.chunks_mut(4800).enumerate() {
                son.rendre(bloc);
                if b < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            let nom = format!("{k}-{}{}.wav", phrase.nom().replace(' ', "-"), if avec_boucle { "-boucle" } else { "" });
            println!("{nom}");
            wav(&out.join(nom), rate as u32, &buf)?;
        }
    }
    Ok(())
}
