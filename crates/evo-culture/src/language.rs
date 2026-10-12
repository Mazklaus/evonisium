//! Langues : un lexique par groupe, une phonologie par langue.
//!
//! [Simplification] Une langue est réduite à un lexique de 64 sens (une
//! liste de Swadesh d'Evonisium) : chaque sens porte une forme, un nombre
//! tiré au hasard qui joue le rôle de convention dans un jeu de
//! signalisation (Lewis, Skyrms). Le jeu complet, avec ses apprenants, tourne
//! chez les agents détaillés (evo-agents, `learning`) ; le modèle agrégé
//! n'en garde que l'issue : chaque groupe partage une convention par sens,
//! elle se renouvelle au rythme de la glottochronologie (86 % des sens
//! gardés en mille ans, Swadesh) et s'aligne sur celle des voisins par
//! alignement sur la majorité de ses voisins. Deux groupes qui ne se parlent plus divergent : une nouvelle
//! langue est déclarée quand leur distance lexicale dépasse un seuil.
//!
//! La phonologie (inventaire de sons, forme des syllabes) est celle de la
//! langue. Elle sert à prononcer les formes : les noms des peuples, des
//! personnes et des dieux en sortent. Un changement de son touche tous les
//! mots d'un coup, comme les lois phonétiques réelles. Pour une espèce qui
//! ne parle pas par le son, les mots affichés sont une transcription.

use evo_core::math::Det;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Nombre de sens du lexique.
pub const MEANINGS: usize = 64;

/// Les sens, dans l'ordre du lexique. Le premier est le nom que le peuple
/// se donne (« nous », « les gens »), comme beaucoup d'ethnonymes réels ;
/// le deuxième nomme la langue.
pub const MEANING_LABELS: [&str; MEANINGS] = [
    "nous",
    "parole",
    "eau",
    "feu",
    "pierre",
    "soleil",
    "lune",
    "ciel",
    "terre",
    "mère",
    "père",
    "enfant",
    "manger",
    "boire",
    "voir",
    "mourir",
    "vivre",
    "grand",
    "petit",
    "un",
    "deux",
    "beaucoup",
    "main",
    "œil",
    "tête",
    "chemin",
    "nuit",
    "jour",
    "pluie",
    "vent",
    "arbre",
    "poisson",
    "viande",
    "os",
    "sang",
    "chant",
    "danger",
    "ami",
    "étranger",
    "esprit",
    "rêve",
    "ancêtre",
    "montagne",
    "mer",
    "étoile",
    "froid",
    "chaud",
    "donner",
    "prendre",
    "faire",
    "savoir",
    "dire",
    "venir",
    "aller",
    "nom",
    "outil",
    "abri",
    "proie",
    "fruit",
    "racine",
    "ventre",
    "cœur",
    "loin",
    "près",
];

/// Sons des inventaires, écrits pour un lecteur français, du plus répandu
/// au plus rare dans les langues du monde.
pub const CONSONANTS: [&str; 24] =
    ["m", "k", "n", "p", "t", "s", "l", "b", "d", "g", "r", "w", "y", "h", "f", "ng", "ch", "v", "z", "ts", "kh", "j", "q", "x"];
pub const VOWELS: [&str; 10] = ["a", "i", "ou", "e", "o", "é", "u", "è", "eu", "ai"];

/// Renouvellement d'un sens par an : 14 % en mille ans.
pub const REPLACEMENT_PER_YEAR: f64 = 1.508e-4;
/// Alignements d'un sens sur la majorité des voisins, par an et par unité
/// de lien.
pub const BORROW_PER_YEAR: f64 = 0.02;
/// Poids de sa propre forme dans le vote, face aux voisins (au poids de
/// leur lien).
pub const SELF_WEIGHT: f64 = 0.5;
/// Part des alignements qui copient un voisin pris au hasard plutôt que la
/// majorité.
pub const VOTER_SHARE: f64 = 0.5;
/// Changement de son par langue et par an.
pub const SOUND_CHANGE_PER_YEAR: f64 = 5e-4;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Phonology {
    /// Indices dans [`CONSONANTS`].
    pub consonants: Vec<u8>,
    /// Indices dans [`VOWELS`].
    pub vowels: Vec<u8>,
    /// Syllabes fermées permises.
    pub codas: bool,
    /// Syllabes sans consonne d'attaque permises.
    pub onsetless: bool,
}

/// Tirage biaisé vers les premiers indices (sons les plus répandus).
fn skewed<R: Rng>(rng: &mut R, n: usize) -> usize {
    let u: f64 = rng.random();
    ((u * u) * n as f64) as usize % n
}

impl Phonology {
    pub fn random<R: Rng>(rng: &mut R) -> Phonology {
        let nc = rng.random_range(6..=14);
        let nv = rng.random_range(3..=7);
        let mut consonants = Vec::new();
        while consonants.len() < nc {
            let c = skewed(rng, CONSONANTS.len()) as u8;
            if !consonants.contains(&c) {
                consonants.push(c);
            }
        }
        let mut vowels = vec![0u8, 1];
        while vowels.len() < nv {
            let v = skewed(rng, VOWELS.len()) as u8;
            if !vowels.contains(&v) {
                vowels.push(v);
            }
        }
        Phonology { consonants, vowels, codas: rng.random_bool(0.5), onsetless: rng.random_bool(0.6) }
    }

    /// Changement de son : un son de l'inventaire en remplace un autre,
    /// dans tous les mots.
    pub fn shift<R: Rng>(&mut self, rng: &mut R) {
        if rng.random_bool(0.7) {
            let i = rng.random_range(0..self.consonants.len());
            for _ in 0..8 {
                let c = skewed(rng, CONSONANTS.len()) as u8;
                if !self.consonants.contains(&c) {
                    self.consonants[i] = c;
                    break;
                }
            }
        } else {
            let i = rng.random_range(0..self.vowels.len());
            for _ in 0..8 {
                let v = skewed(rng, VOWELS.len()) as u8;
                if !self.vowels.contains(&v) {
                    self.vowels[i] = v;
                    break;
                }
            }
        }
    }

    /// Mot d'une forme : la forme fixe la suite des positions dans
    /// l'inventaire, la phonologie fixe les sons.
    pub fn word(&self, form: u32) -> String {
        let mut h = evo_core::rng::splitmix64(form as u64 ^ 0x5157_4F52_4D45);
        let mut take = |n: u64| {
            let r = h % n;
            h = evo_core::rng::splitmix64(h);
            r as usize
        };
        let syllables = match take(10) {
            0..=2 => 1,
            3..=7 => 2,
            _ => 3,
        };
        let mut out = String::new();
        for s in 0..syllables {
            let onset = !(self.onsetless && s == 0 && take(4) == 0);
            if onset {
                let i = take(64) * take(64) / 64 % self.consonants.len();
                out.push_str(CONSONANTS[self.consonants[i] as usize]);
            }
            let v = take(64) * take(64) / 64 % self.vowels.len();
            out.push_str(VOWELS[self.vowels[v] as usize]);
            if self.codas && take(3) == 0 {
                let c = take(self.consonants.len() as u64);
                out.push_str(CONSONANTS[self.consonants[c] as usize]);
            }
        }
        out
    }

    /// Nom propre : mot capitalisé.
    pub fn name(&self, form: u32) -> String {
        capitalize(&self.word(form))
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Lexique d'un groupe : une forme par sens.
pub type Lexicon = [u32; MEANINGS];

/// (Dé)sérialisation d'un lexique en suite de nombres.
pub mod lexicon_serde {
    use super::{Lexicon, MEANINGS};
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::borrowed_box)]
    pub fn serialize<S: Serializer>(l: &Box<Lexicon>, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(l.iter())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Box<Lexicon>, D::Error> {
        let v = Vec::<u32>::deserialize(d)?;
        let mut l = Box::new([0u32; MEANINGS]);
        if v.len() != MEANINGS {
            return Err(serde::de::Error::invalid_length(v.len(), &"64 formes"));
        }
        l.copy_from_slice(&v);
        Ok(l)
    }
}

pub fn random_lexicon<R: Rng>(rng: &mut R) -> Box<Lexicon> {
    let mut l = Box::new([0u32; MEANINGS]);
    for f in l.iter_mut() {
        *f = rng.random();
    }
    l
}

/// Distance lexicale : part des sens dont la forme diffère, sur les
/// `meanings` premiers sens (l'expressivité de l'espèce).
pub fn distance(a: &Lexicon, b: &Lexicon, meanings: usize) -> f64 {
    let k = meanings.clamp(1, MEANINGS);
    let diff = a[..k].iter().zip(&b[..k]).filter(|(x, y)| x != y).count();
    diff as f64 / k as f64
}

/// Part des sens gardés après `years` ans sans contact.
pub fn retention(years: f64) -> f64 {
    (-REPLACEMENT_PER_YEAR * years).dexp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::rng::{rng_for, Stream};

    #[test]
    fn swadesh_rate() {
        assert!((retention(1000.0) - 0.86).abs() < 0.001);
    }

    #[test]
    fn words_are_pronounceable_and_follow_sound_changes() {
        let mut rng = rng_for(1, Stream::Culture, &[]);
        let mut p = Phonology::random(&mut rng);
        let w: Vec<String> = (0..20).map(|f| p.word(f)).collect();
        assert!(w.iter().all(|s| !s.is_empty() && s.len() < 20), "{w:?}");
        let before = p.clone();
        for _ in 0..6 {
            p.shift(&mut rng);
        }
        assert_ne!(before, p);
        let after: Vec<String> = (0..20).map(|f| p.word(f)).collect();
        assert!(after.iter().zip(&w).any(|(a, b)| a != b));
        // Même forme, même phonologie : même mot.
        assert_eq!(p.word(7), p.word(7));
    }
}
