//! Rend, pour chaque ère, le mélange du jeu sur le globe avec les boucles du
//! fond musical livrées (`client/musique/fond`) dans des fichiers WAV, pour
//! régler à l'oreille le niveau des boucles sous les couches générées.
//!
//! `cargo run --release -p evo-son --example fond -- [dossier] [secondes]`

// Affichage seulement, comme le crate : les mathématiques de la plateforme suffisent.
#![allow(clippy::disallowed_methods)]

use evo_son::fond::{liste, Boucle, Contenu, ERES};
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
    let args: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(args.get(1).cloned().unwrap_or_else(|| "fond".into()));
    let secondes: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100.0);
    std::fs::create_dir_all(&out)?;
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../client/musique/fond");
    let entrees = liste(&std::fs::read_to_string(dossier.join("boucles.txt"))?);
    let rate = 48_000.0;
    for (e, nom) in ERES.iter().enumerate() {
        let mut son = Son::new(rate, 7 + e as u64);
        // Une planète plausible pour l'ère : plus d'oxygène, plus de vie.
        let o2 = [1e-6, 0.01, 0.03, 0.1, 0.18, 0.21, 0.21][e];
        let lineages = [6, 20, 60, 150, 300, 400, 400][e];
        son.ordre(Ordre::Monde(Monde { o2, temperature_k: 288.0, ocean: 0.7, lineages, ..Default::default() }));
        son.ordre(Ordre::Vue(Vue::Globe));
        son.ordre(Ordre::Ere(e as u8));
        for x in entrees.iter().filter(|x| x.ere as usize == e) {
            let ogg = std::fs::read(dossier.join(&x.fichier))?;
            son.ordre(Ordre::Boucle(Boucle {
                ere: x.ere,
                son: Contenu::Ogg(Arc::new(ogg)),
                tonique: x.tonique,
                mode: x.mode,
                gain: x.gain,
            }));
        }
        let mut buf = vec![[0.0f32; 2]; (rate * secondes) as usize];
        let t = std::time::Instant::now();
        for (k, bloc) in buf.chunks_mut(4800).enumerate() {
            son.rendre(bloc);
            // Le premier décodage se fait pendant les premières secondes.
            if k < 4 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let cpu = (t.elapsed().as_secs_f32() - 0.4) / secondes * 100.0;
        let rms = (buf.iter().map(|f| (f[0] * f[0] + f[1] * f[1]) as f64).sum::<f64>() / (buf.len() * 2) as f64).sqrt();
        let crete = buf.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
        println!("{e} {nom} : {:.1} dB efficaces, crête {crete:.2}, {cpu:.1} % d'un cœur", 20.0 * rms.log10());
        wav(&out.join(format!("{e}-{}.wav", nom.replace(' ', "-"))), rate as u32, &buf)?;
    }
    Ok(())
}
