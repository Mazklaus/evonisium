//! Rend quelques minutes de son dans des fichiers WAV et mesure le coût de
//! la synthèse (part d'un cœur), sans Godot :
//!
//! `cargo run --release -p evo-son --example ecoute -- [dossier] [secondes]`
//!
//! Scènes : ère microbienne pauvre en vie, monde riche et oxygéné, Terre
//! boule de neige, loupe, menu, et un défilé des bruits d'interface.

use evo_son::{Bruit, Monde, Ordre, Son, Vue};
use std::io::Write;
use std::time::Instant;

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
    let dir = std::path::PathBuf::from(args.get(1).map_or("ecoute", |s| s.as_str()));
    let seconds: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60.0);
    std::fs::create_dir_all(&dir)?;
    let rate = 44_100u32;
    let base = Monde { o2: 1e-5, temperature_k: 300.0, ice: 0.0, ocean: 0.8, lineages: 3, paused: false };
    let scenes: [(&str, Vue, Monde); 5] = [
        ("1-globe-archeen", Vue::Globe, base),
        ("2-globe-oxygene", Vue::Globe, Monde { o2: 0.05, temperature_k: 293.0, lineages: 250, ..base }),
        ("3-globe-boule-de-neige", Vue::Globe, Monde { o2: 0.01, temperature_k: 245.0, ice: 0.9, lineages: 20, ..base }),
        ("4-loupe", Vue::Microscope, Monde { o2: 1e-3, lineages: 60, ..base }),
        ("5-menu", Vue::Menu, base),
    ];
    let mut total = 0.0f64;
    let mut cost = 0.0f64;
    for (i, (name, vue, monde)) in scenes.into_iter().enumerate() {
        let mut son = Son::new(rate as f32, 100 + i as u64);
        son.ordre(Ordre::Monde(monde));
        son.ordre(Ordre::Vue(vue));
        if name.contains("boule") {
            son.ordre(Ordre::Evenement { family: "catastrophe".into(), interest: 0.9 });
        }
        if name.contains("oxygene") {
            son.ordre(Ordre::Evenement { family: "jalon".into(), interest: 0.9 });
        }
        let mut buf = vec![[0.0f32; 2]; (rate as f32 * seconds) as usize];
        let t = Instant::now();
        for chunk in buf.chunks_mut(512) {
            son.rendre(chunk);
        }
        let s = t.elapsed().as_secs_f64();
        total += seconds as f64;
        cost += s;
        println!("{name} : {:.1} % d'un cœur", 100.0 * s / seconds as f64);
        wav(&dir.join(format!("{name}.wav")), rate, &buf)?;
    }
    let mut son = Son::new(rate as f32, 9);
    son.ordre(Ordre::Volumes([0.8, 0.0, 0.0, 1.0]));
    let mut buf = Vec::new();
    for b in [Bruit::Clic, Bruit::Clic, Bruit::Page, Bruit::Plume, Bruit::Fermer, Bruit::Tampon, Bruit::Cloche, Bruit::Etape] {
        son.ordre(Ordre::Bruit(b));
        let secs = if b == Bruit::Etape { 3.0 } else { 1.3 };
        let mut part = vec![[0.0f32; 2]; (rate as f32 * secs) as usize];
        son.rendre(&mut part);
        buf.extend(part);
    }
    wav(&dir.join("6-interface.wav"), rate, &buf)?;
    println!("moyenne : {:.1} % d'un cœur", 100.0 * cost / total);
    Ok(())
}
