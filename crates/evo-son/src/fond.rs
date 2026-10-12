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
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

/// Le son d'une boucle : déjà décodé, ou encore compressé (Ogg Vorbis). Une
/// boucle compressée n'est décodée, sur un fil à part, qu'au moment de la
/// jouer : décodées toutes ensemble, les boucles d'une partie prendraient
/// plus d'un gigaoctet.
#[derive(Clone, Debug)]
pub enum Contenu {
    /// Échantillons stéréo au taux du son ; la fin se raccorde au début.
    Pcm(Arc<Vec<[f32; 2]>>),
    Ogg(Arc<Vec<u8>>),
}

/// Une boucle du fond.
#[derive(Clone, Debug)]
pub struct Boucle {
    /// Ère (0 : archéen … 6 : premiers peuples).
    pub ere: u8,
    pub son: Contenu,
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

/// Échantillons stéréo décodés.
type Pcm = Vec<[f32; 2]>;

/// Boucle qui joue : indice, échantillons, position, reprises restantes.
struct Courante {
    i: usize,
    pcm: Arc<Vec<[f32; 2]>>,
    pos: usize,
    reprises: u32,
}

pub struct Fond {
    rate: f32,
    boucles: Vec<Boucle>,
    pub ere: u8,
    courante: Option<Courante>,
    /// Prochaine boucle, choisie pendant le silence et décodée à l'avance.
    prochaine: Option<(usize, Receiver<Option<Pcm>>)>,
    pret: Option<(usize, Arc<Vec<[f32; 2]>>)>,
    derniere: Option<usize>,
    /// Silence restant avant la prochaine boucle.
    silence: f32,
    gain: Smooth,
    /// La boucle en cours s'efface (fin de ses reprises ou changement d'ère).
    sortie: bool,
}

impl Fond {
    pub fn new(rate: f32) -> Self {
        Self {
            rate,
            boucles: Vec::new(),
            ere: 0,
            courante: None,
            prochaine: None,
            pret: None,
            derniere: None,
            silence: 4.0,
            gain: Smooth::new(0.0),
            sortie: false,
        }
    }

    pub fn ajouter(&mut self, b: Boucle) {
        let vide = match &b.son {
            Contenu::Pcm(p) => p.is_empty(),
            Contenu::Ogg(o) => o.is_empty(),
        };
        if !vide {
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
        self.courante.as_ref().map(|c| (self.boucles[c.i].tonique, self.boucles[c.i].mode))
    }

    /// Une boucle joue (ou s'efface).
    pub fn joue(&self) -> bool {
        self.courante.is_some()
    }

    /// Ère de la boucle qui joue.
    pub fn ere_jouee(&self) -> Option<u8> {
        self.courante.as_ref().map(|c| self.boucles[c.i].ere)
    }

    /// Pas de contrôle : choisit et prépare la prochaine boucle, la lance à
    /// la fin du silence.
    pub fn tick(&mut self, dt: f32, rng: &mut Rng) {
        if self.courante.is_some() {
            return;
        }
        self.silence -= dt;
        // Une boucle préparée pour une autre ère ne sert plus.
        if self.pret.as_ref().is_some_and(|(i, _)| self.boucles[*i].ere != self.ere) {
            self.pret = None;
        }
        if self.prochaine.as_ref().is_some_and(|(i, _)| self.boucles[*i].ere != self.ere) {
            self.prochaine = None;
        }
        if let Some((i, rx)) = &self.prochaine {
            match rx.try_recv() {
                Ok(Some(pcm)) => {
                    self.pret = Some((*i, Arc::new(pcm)));
                    self.prochaine = None;
                }
                Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Illisible : on la retire et on en choisit une autre.
                    let i = *i;
                    self.prochaine = None;
                    self.boucles.remove(i);
                    self.derniere = None;
                    self.pret = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pret.is_none() && self.prochaine.is_none() {
            self.choisir(rng);
        }
        if self.silence > 0.0 {
            return;
        }
        let Some((i, pcm)) = self.pret.take() else { return };
        // Assez de reprises pour une minute et demie à trois minutes
        // environ : mieux vaut changer souvent de boucle que la répéter.
        let secondes = pcm.len() as f32 / self.rate;
        let reprises = ((rng.range(90.0, 180.0) / secondes.max(1.0)).round() as u32).clamp(1, 8);
        self.courante = Some(Courante { i, pcm, pos: 0, reprises });
        self.derniere = Some(i);
        self.sortie = false;
        self.gain.value = 0.0;
    }

    /// Choisit une boucle de l'ère, autre que la précédente si possible, et
    /// la décode sur un fil à part si elle est compressée.
    fn choisir(&mut self, rng: &mut Rng) {
        let choix: Vec<usize> = (0..self.boucles.len()).filter(|&i| self.boucles[i].ere == self.ere).collect();
        if choix.is_empty() {
            return;
        }
        let autres: Vec<usize> = choix.iter().copied().filter(|&i| Some(i) != self.derniere).collect();
        let pool = if autres.is_empty() { &choix } else { &autres };
        let i = pool[(rng.u() * pool.len() as f32) as usize % pool.len()];
        match &self.boucles[i].son {
            Contenu::Pcm(p) => self.pret = Some((i, p.clone())),
            Contenu::Ogg(o) => {
                let (tx, rx) = channel();
                let (o, rate) = (o.clone(), self.rate);
                let lance = std::thread::Builder::new().name("evo-son-fond".into()).spawn(move || {
                    let _ = tx.send(decoder(&o, rate).ok());
                });
                if lance.is_ok() {
                    self.prochaine = Some((i, rx));
                }
            }
        }
    }

    /// Échantillon suivant (stéréo) ; `dt` = 1 / taux du son.
    #[inline]
    pub fn run(&mut self, dt: f32, rng: &mut Rng) -> [f32; 2] {
        let Some(c) = &mut self.courante else { return [0.0; 2] };
        let b = &self.boucles[c.i];
        // Fondu d'entrée à la première reprise, de sortie à la fin.
        let reste = (c.pcm.len() - c.pos) as f32 * dt;
        if c.reprises == 1 && reste < FONDU {
            self.sortie = true;
        }
        self.gain.target = if self.sortie { 0.0 } else { 1.0 };
        let g = self.gain.step(dt, FONDU / 3.0);
        let x = c.pcm[c.pos];
        let gb = b.gain * g;
        c.pos += 1;
        if c.pos >= c.pcm.len() {
            c.pos = 0;
            c.reprises = c.reprises.saturating_sub(1);
        }
        if (self.sortie && g < 1e-3) || c.reprises == 0 {
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
        Boucle { ere, son: Contenu::Pcm(Arc::new(pcm)), tonique: 50, mode: 1, gain: 1.0 }
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
            vu |= f.ere_jouee() == Some(3);
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
    fn compressed_loops_are_decoded_only_when_chosen() {
        let ogg = Arc::new(include_bytes!("../tests/donnees/essai.ogg").to_vec());
        let mut f = Fond::new(48_000.0);
        let mut rng = Rng::new(3);
        f.ajouter(Boucle { ere: 0, son: Contenu::Ogg(Arc::new(b"pas un ogg".to_vec())), tonique: 50, mode: 1, gain: 1.0 });
        f.ajouter(Boucle { ere: 0, son: Contenu::Ogg(ogg), tonique: 57, mode: 2, gain: 1.0 });
        let dt = 1.0 / 48_000.0;
        let mut joue = false;
        for s in 0..(48_000 * 60) {
            if s % 256 == 0 {
                f.tick(256.0 * dt, &mut rng);
                if f.joue() && !joue {
                    joue = true;
                    assert_eq!(f.cle(), Some((57, 2)));
                }
                // Laisse au fil de décodage le temps de finir.
                if !f.joue() && s % (256 * 64) == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            }
            f.run(dt, &mut rng);
        }
        assert!(joue, "la boucle lisible a joué");
        assert_eq!(f.boucles(), 1, "la boucle illisible est retirée");
    }

    #[test]
    fn the_shipped_loops_are_all_listed_and_decode() {
        let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../client/musique/fond");
        let l = liste(&std::fs::read_to_string(dossier.join("boucles.txt")).unwrap());
        let mut fichiers: Vec<String> = std::fs::read_dir(&dossier)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|f| f.ends_with(".ogg"))
            .collect();
        fichiers.sort();
        let mut listes: Vec<String> = l.iter().map(|e| e.fichier.clone()).collect();
        listes.sort();
        assert_eq!(fichiers, listes, "chaque boucle livrée est listée, et seulement elles");
        for e in 0..ERES.len() as u8 {
            assert!(l.iter().filter(|x| x.ere == e).count() >= 4, "{} : trop peu de boucles", ERES[e as usize]);
        }
        assert!(l.iter().all(|e| e.gain > 0.05 && e.gain < 1.0));
        let t = std::time::Instant::now();
        let pcm = decoder(&std::fs::read(dossier.join(&l[0].fichier)).unwrap(), 48_000.0).unwrap();
        assert!(pcm.len() > 48_000 * 30);
        eprintln!("{} décodé en {:?}", l[0].fichier, t.elapsed());
    }

    #[test]
    fn the_list_reads_era_key_and_gain() {
        let l = liste("# ère 0\narcheen-a.ogg 0 50 éolien 0.8\nmauvaise ligne\nx.ogg 9 50 éolien 1\nforet-b.ogg 4 50 mixolydien 1.0 # D\n");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0], Entree { fichier: "archeen-a.ogg".into(), ere: 0, tonique: 50, mode: 1, gain: 0.8 });
        assert_eq!((l[1].ere, l[1].mode), (4, 3));
    }
}
