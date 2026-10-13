//! Rend les voix d'espèces (livraison 2) dans des fichiers WAV : chaque type
//! d'organe sonore, simple puis complexe, puis trois scènes au sol avec la
//! musique du jeu.
//!
//! `cargo run --release -p evo-son --example faune -- [dossier]`

// Affichage seulement, comme le crate : les mathématiques de la plateforme suffisent.
#![allow(clippy::disallowed_methods)]

use evo_son::faune::{appel, duree, Genre, Heure, Organe};
use evo_son::synth::{Make, Rng, Voice};
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

fn organe(genre: Genre, complexite: f32, graine: u64) -> Organe {
    Organe { genre, taille_m: genre.taille_type(), puissance: 0.7, complexite, choeur: true, heure: Heure::Jour, graine }
}

/// Rend des voix seules (sans musique) pendant `secondes`.
fn seules(mut voix: Vec<Voice>, secondes: f32, rate: f32) -> Vec<[f32; 2]> {
    let mut rng = Rng::new(1);
    let mut out = vec![[0.0f32; 2]; (rate * secondes) as usize];
    for fr in out.iter_mut() {
        voix.retain_mut(|v| match v.next(&mut rng, 1.0 / rate, rate) {
            Some(s) => {
                let a = (v.pan + 1.0) * 0.25 * std::f32::consts::PI;
                fr[0] += s * a.cos() * 2.5;
                fr[1] += s * a.sin() * 2.5;
                true
            }
            None => false,
        });
        *fr = fr.map(|x| x.tanh() * 0.95);
    }
    out
}

fn main() -> std::io::Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "faune".into()));
    std::fs::create_dir_all(&out)?;
    let rate = 48_000.0;
    let mk = Make { rate };
    let mut rng = Rng::new(17);
    // Chaque organe : deux appels d'une espèce simple, puis deux d'une
    // espèce complexe, la seconde fois en chœur.
    for (i, g) in Genre::TOUS.into_iter().enumerate() {
        let mut voix = Vec::new();
        let mut t = 0.3;
        for (c, graine) in [(0.15, 11 + i as u64), (0.85, 23 + i as u64)] {
            let o = organe(g, c, graine);
            for choeur in [false, true] {
                appel(&o, 6.0, rng.bipolar() * 0.4, t, choeur, &mk, &mut rng, &mut voix);
                t += duree(&o) + if choeur { 2.5 } else { 1.2 };
            }
        }
        let nom = format!("{i}-{}.wav", g.nom().replace(' ', "-"));
        println!("{nom} : {t:.0} s");
        wav(&out.join(nom), rate as u32, &seules(voix, t + 1.0, rate))?;
    }
    // Quatre scènes au sol, avec la musique du jeu ; la jungle, foisonnante,
    // montre le dosage (premier plan qui tourne, tours de parole).
    let jungle: Vec<(Organe, u32)> = (0..14u64)
        .map(|i| {
            let g = [Genre::Stridulation, Genre::Tymbale, Genre::Larynx, Genre::Sac, Genre::Percussion, Genre::Stridulation, Genre::Larynx]
                [i as usize % 7];
            let taille = g.taille_type() * (0.4 + 0.3 * i as f32);
            (Organe { taille_m: taille, ..organe(g, 0.2 + 0.05 * i as f32, 400 + i) }, 10 + 5 * i as u32)
        })
        .collect();
    let scenes: [(&str, Vec<(Organe, u32)>); 4] = [
        (
            "foret",
            vec![
                (Organe { taille_m: 0.004, ..organe(Genre::Stridulation, 0.3, 101) }, 40),
                (Organe { taille_m: 0.002, ..organe(Genre::Stridulation, 0.7, 102) }, 25),
                (organe(Genre::Tymbale, 0.4, 103), 10),
                (Organe { taille_m: 0.15, puissance: 0.9, ..organe(Genre::Larynx, 0.6, 104) }, 4),
                (organe(Genre::Percussion, 0.5, 105), 3),
            ],
        ),
        (
            "mare",
            vec![
                (Organe { taille_m: 0.006, ..organe(Genre::Sac, 0.3, 201) }, 30),
                (Organe { taille_m: 0.02, heure: Heure::Crepuscule, ..organe(Genre::Sac, 0.7, 202) }, 8),
                (organe(Genre::Stridulation, 0.5, 203), 20),
                (Organe { taille_m: 0.03, ..organe(Genre::Larynx, 0.4, 204) }, 5),
            ],
        ),
        (
            "mer",
            vec![
                (organe(Genre::Vessie, 0.4, 301), 30),
                (Organe { taille_m: 0.1, ..organe(Genre::Vessie, 0.8, 302) }, 6),
                (organe(Genre::Clics, 0.8, 303), 8),
                (Organe { taille_m: 0.03, ..organe(Genre::Clics, 0.3, 304) }, 12),
            ],
        ),
        ("jungle", jungle),
    ];
    for (nom, especes) in scenes {
        let mut son = Son::new(rate, 3);
        son.ordre(Ordre::Monde(Monde { o2: 0.2, temperature_k: 290.0, ocean: 0.7, lineages: 300, ..Default::default() }));
        son.ordre(Ordre::Vue(Vue::Sol));
        son.ordre(Ordre::Faune(Some(especes)));
        let mut buf = vec![[0.0f32; 2]; (rate * 90.0) as usize];
        let t = std::time::Instant::now();
        son.rendre(&mut buf);
        println!("scène {nom} : {:.1} % d'un cœur", t.elapsed().as_secs_f32() / 90.0 * 100.0);
        wav(&out.join(format!("scene-{nom}.wav")), rate as u32, &buf)?;
    }
    Ok(())
}
