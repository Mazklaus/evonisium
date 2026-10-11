//! Fond musical par ère : des boucles enregistrées (morceaux générés avec
//! Suno par l'utilisateur, découpés en boucles qui se raccordent), jouées
//! sous les couches générées par le code.
//!
//! Le fond joue les boucles de l'ère en cours. Chaque boucle est reprise
//! quelques fois, puis s'efface ; un silence suit, où seules restent les
//! couches générées, avant une autre boucle de la même ère. Au changement
//! d'ère, le fond passe en fondu à la nouvelle. Les couches générées jouent
//! dans la tonalité de la boucle (`cle`), pour rester accordées.
//!
//! Sans boucle pour l'ère en cours, rien ne change : la musique reste
//! entièrement générée, comme avant.

use crate::synth::{Rng, Smooth};

/// Une boucle décodée, au taux du son.
#[derive(Clone, Debug)]
pub struct Boucle {
    /// Ère (0 : archéen … 6 : premiers peuples).
    pub ere: u8,
    /// Échantillons stéréo ; la fin se raccorde au début.
    pub pcm: Vec<[f32; 2]>,
    /// Tonique (note MIDI) et mode (indice de `musique::MODES`).
    pub tonique: i32,
    pub mode: usize,
    /// Gain propre de la boucle (les morceaux n'ont pas tous le même niveau).
    pub gain: f32,
}

/// Les ères du fond, dans l'ordre du jeu.
pub const ERES: [&str; 7] =
    ["archéen", "grande oxydation", "cellules complexes", "explosion animale", "terres et forêts", "âge des géants", "premiers peuples"];

/// Durée des fondus, en secondes.
const FONDU: f32 = 5.0;

pub struct Fond {
    rate: f32,
    boucles: Vec<Boucle>,
    pub ere: u8,
    /// Boucle en cours, position, reprises restantes.
    courante: Option<(usize, usize, u32)>,
    derniere: Option<usize>,
    /// Silence restant avant la prochaine boucle.
    silence: f32,
    gain: Smooth,
    /// La boucle en cours s'efface (fin de ses reprises ou changement d'ère).
    sortie: bool,
}

impl Fond {
    pub fn new(rate: f32) -> Self {
        Self { rate, boucles: Vec::new(), ere: 0, courante: None, derniere: None, silence: 4.0, gain: Smooth::new(0.0), sortie: false }
    }

    pub fn ajouter(&mut self, b: Boucle) {
        if !b.pcm.is_empty() {
            self.boucles.push(b);
        }
    }

    pub fn boucles(&self) -> usize {
        self.boucles.len()
    }

    pub fn changer_ere(&mut self, ere: u8) {
        if ere != self.ere {
            self.ere = ere;
            if self.courante.is_some() {
                self.sortie = true;
            } else {
                self.silence = self.silence.min(3.0);
            }
        }
    }

    /// Tonalité de la boucle qui joue (tonique, mode), pour accorder les
    /// couches générées.
    pub fn cle(&self) -> Option<(i32, usize)> {
        self.courante.map(|(i, _, _)| (self.boucles[i].tonique, self.boucles[i].mode))
    }

    /// Une boucle joue (ou s'efface).
    pub fn joue(&self) -> bool {
        self.courante.is_some()
    }

    /// Pas de contrôle : choisit la prochaine boucle, gère les fondus.
    pub fn tick(&mut self, dt: f32, rng: &mut Rng) {
        if self.courante.is_none() {
            self.silence -= dt;
            if self.silence > 0.0 {
                return;
            }
            let choix: Vec<usize> = (0..self.boucles.len()).filter(|&i| self.boucles[i].ere == self.ere).collect();
            if choix.is_empty() {
                self.silence = 5.0;
                return;
            }
            let autres: Vec<usize> = choix.iter().copied().filter(|&i| Some(i) != self.derniere).collect();
            let pool = if autres.is_empty() { &choix } else { &autres };
            let i = pool[(rng.u() * pool.len() as f32) as usize % pool.len()];
            // Assez de reprises pour deux à quatre minutes environ.
            let secondes = self.boucles[i].pcm.len() as f32 / self.rate;
            let reprises = ((rng.range(120.0, 240.0) / secondes.max(1.0)).round() as u32).clamp(1, 8);
            self.courante = Some((i, 0, reprises));
            self.derniere = Some(i);
            self.sortie = false;
            self.gain.value = 0.0;
        }
    }

    /// Échantillon suivant (stéréo) ; `dt` = 1 / taux du son.
    #[inline]
    pub fn run(&mut self, dt: f32, rng: &mut Rng) -> [f32; 2] {
        let Some((i, pos, reprises)) = &mut self.courante else { return [0.0; 2] };
        let b = &self.boucles[*i];
        // Fondu d'entrée à la première reprise, de sortie à la fin.
        let reste = (b.pcm.len() - *pos) as f32 * dt;
        if *reprises == 1 && reste < FONDU {
            self.sortie = true;
        }
        self.gain.target = if self.sortie { 0.0 } else { 1.0 };
        let g = self.gain.step(dt, FONDU / 3.0);
        let x = b.pcm[*pos];
        let gb = b.gain * g;
        *pos += 1;
        if *pos >= b.pcm.len() {
            *pos = 0;
            *reprises = reprises.saturating_sub(1);
        }
        if (self.sortie && g < 1e-3) || *reprises == 0 {
            self.courante = None;
            self.sortie = false;
            self.silence = rng.range(20.0, 60.0);
        }
        [x[0] * gb, x[1] * gb]
    }
}

/// Décode un fichier Ogg Vorbis en stéréo au taux `rate` (rééchantillonné
/// linéairement si besoin ; un fichier mono est mis aux deux canaux).
pub fn decoder(ogg: &[u8], rate: f32) -> Result<Vec<[f32; 2]>, String> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(ogg)).map_err(|e| e.to_string())?;
    let canaux = r.ident_hdr.audio_channels as usize;
    let taux = r.ident_hdr.audio_sample_rate as f32;
    let mut pcm: Vec<[f32; 2]> = Vec::new();
    while let Some(p) = r.read_dec_packet_itl().map_err(|e| e.to_string())? {
        for f in p.chunks_exact(canaux.max(1)) {
            let g = f[0] as f32 / 32768.0;
            let d = if canaux > 1 { f[1] as f32 / 32768.0 } else { g };
            pcm.push([g, d]);
        }
    }
    if pcm.is_empty() {
        return Err("fichier vide".into());
    }
    if (taux - rate).abs() < 0.5 {
        return Ok(pcm);
    }
    let pas = taux / rate;
    let n = ((pcm.len() as f32) / pas) as usize;
    Ok((0..n)
        .map(|k| {
            let x = k as f32 * pas;
            let i = x as usize;
            let a = x - i as f32;
            let (u, v) = (pcm[i.min(pcm.len() - 1)], pcm[(i + 1).min(pcm.len() - 1)]);
            [u[0] + (v[0] - u[0]) * a, u[1] + (v[1] - u[1]) * a]
        })
        .collect())
}

/// Une ligne de la liste des boucles (`boucles.txt`).
#[derive(Clone, Debug, PartialEq)]
pub struct Entree {
    pub fichier: String,
    pub ere: u8,
    pub tonique: i32,
    pub mode: usize,
    pub gain: f32,
}

/// Lit la liste des boucles : une par ligne, `fichier ère tonique mode gain`
/// (ère de 0 à 6, tonique en note MIDI, mode par son nom dans
/// `musique::MODES`) ; `#` commence un commentaire. Les lignes mal formées
/// sont ignorées.
pub fn liste(texte: &str) -> Vec<Entree> {
    texte
        .lines()
        .filter_map(|l| {
            let l = l.split('#').next()?.trim();
            let m: Vec<&str> = l.split_whitespace().collect();
            let [fichier, ere, tonique, mode, gain] = m[..] else { return None };
            let mode = crate::musique::MODES.iter().position(|(n, _)| *n == mode)?;
            let ere: u8 = ere.parse().ok().filter(|&e| (e as usize) < ERES.len())?;
            Some(Entree { fichier: fichier.to_string(), ere, tonique: tonique.parse().ok()?, mode, gain: gain.parse().ok()? })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boucle(ere: u8, secondes: f32) -> Boucle {
        let n = (44_100.0 * secondes) as usize;
        let pcm = (0..n).map(|k| [(k as f32 * 0.01).sin() * 0.3; 2]).collect();
        Boucle { ere, pcm, tonique: 50, mode: 1, gain: 1.0 }
    }

    #[test]
    fn loops_of_the_era_play_with_rests_between() {
        let mut f = Fond::new(44_100.0);
        let mut rng = Rng::new(1);
        f.ajouter(boucle(0, 30.0));
        f.ajouter(boucle(0, 40.0));
        f.ajouter(boucle(3, 30.0));
        let dt = 1.0 / 44_100.0;
        let (mut joue, mut silence) = (0usize, 0usize);
        let mut cles = Vec::new();
        for s in 0..(44_100 * 900) {
            if s % 256 == 0 {
                f.tick(256.0 * dt, &mut rng);
                if let Some(c) = f.cle() {
                    if !cles.contains(&c) {
                        cles.push(c);
                    }
                }
            }
            let x = f.run(dt, &mut rng);
            assert!(x[0].abs() <= 0.31);
            if f.joue() {
                joue += 1;
            } else {
                silence += 1;
            }
        }
        assert!(joue > silence, "le fond joue la plupart du temps : {joue} contre {silence}");
        assert!(silence > 44_100 * 20, "au moins un silence entre deux boucles");
        assert_eq!(cles, vec![(50, 1)]);
        // Changement d'ère : la boucle en cours s'efface, une de la nouvelle ère suit.
        f.changer_ere(3);
        let mut vu = false;
        for s in 0..(44_100 * 120) {
            if s % 256 == 0 {
                f.tick(256.0 * dt, &mut rng);
            }
            f.run(dt, &mut rng);
            vu |= f.courante.is_some_and(|(i, _, _)| f.boucles[i].ere == 3);
        }
        assert!(vu);
    }

    #[test]
    fn without_loops_nothing_plays() {
        let mut f = Fond::new(44_100.0);
        let mut rng = Rng::new(2);
        for _ in 0..1000 {
            f.tick(0.1, &mut rng);
        }
        assert!(!f.joue());
        assert_eq!(f.run(1.0 / 44_100.0, &mut rng), [0.0; 2]);
    }

    #[test]
    fn an_ogg_file_decodes_at_the_sound_rate() {
        // Deux secondes de la à 220 Hz (ffmpeg, crête ≈ 0,09), stéréo, à 22 050 Hz.
        let ogg = include_bytes!("../tests/donnees/essai.ogg");
        let pcm = decoder(ogg, 48_000.0).unwrap();
        assert!((pcm.len() as f32 / 48_000.0 - 2.0).abs() < 0.05, "{} échantillons", pcm.len());
        let crete = pcm.iter().map(|x| x[0].abs()).fold(0.0, f32::max);
        assert!(crete > 0.05 && crete <= 1.0, "crête {crete}");
        // Passages par zéro : la hauteur est gardée au rééchantillonnage.
        let zeros = pcm.windows(2).filter(|w| w[0][0] < 0.0 && w[1][0] >= 0.0).count();
        assert!((430..=450).contains(&zeros), "{zeros} périodes");
        assert!(decoder(b"pas un ogg", 48_000.0).is_err());
    }

    #[test]
    fn the_list_reads_era_key_and_gain() {
        let l = liste("# ère 0\narcheen-a.ogg 0 50 éolien 0.8\nmauvaise ligne\nx.ogg 9 50 éolien 1\nforet-b.ogg 4 50 mixolydien 1.0 # D\n");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0], Entree { fichier: "archeen-a.ogg".into(), ere: 0, tonique: 50, mode: 1, gain: 0.8 });
        assert_eq!((l[1].ere, l[1].mode), (4, 3));
    }
}
