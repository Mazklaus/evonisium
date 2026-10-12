//! Voix des espèces (livraison 2 du doc « Design sonore ») : chaque espèce
//! animale qui porte un organe sonore (`evo_life::SoundOrgan`, lu par
//! `evo-godot`) a un appel tiré de son corps. Le type d'organe donne la
//! manière (trille d'insecte, cri du larynx, croassement d'un sac vocal,
//! grognement d'une vessie natatoire, clics…), la taille de l'organe la
//! hauteur, la puissance le volume et la fréquence des appels, la
//! complexité le nombre de syllabes ; le chœur fait répondre plusieurs
//! individus. La graine héritée fixe le motif : une espèce garde le même
//! appel tant que les gènes de l'appel ne changent pas.
//!
//! [Simplification] Les hauteurs suivent des lois d'échelle simples
//! (fréquence inverse de la taille de l'organe), bornées à l'audible ; la
//! scène au sol n'a pas encore de cycle du jour, donc les espèces du
//! crépuscule et de la nuit appellent seulement plus rarement.

use crate::peuples::{Gosier, Locuteur, Ph, Seg};
use crate::synth::{Bus, Kind, Make, Rng, Voice};

/// Type d'organe sonore, comme `evo_life::SoundKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    /// Frottement d'une râpe sur un grattoir (grillons, sauterelles).
    Stridulation,
    /// Membrane qui claque (cigales).
    Tymbale,
    /// Bourdonnement des ailes.
    Ailes,
    /// Organe des oiseaux : sifflets modulés.
    Syrinx,
    /// Cordes vocales : cris, hurlements, mugissements.
    Larynx,
    /// Sac vocal gonflé (grenouilles) : croassements.
    Sac,
    /// Coups frappés (tambourinage, poitrine, sol).
    Percussion,
    /// Vessie natatoire (poissons) : grognements sourds.
    Vessie,
    /// Clics et sifflets dans l'eau.
    Clics,
}

impl Genre {
    pub const TOUS: [Genre; 9] = [
        Genre::Stridulation,
        Genre::Tymbale,
        Genre::Ailes,
        Genre::Syrinx,
        Genre::Larynx,
        Genre::Sac,
        Genre::Percussion,
        Genre::Vessie,
        Genre::Clics,
    ];
    pub fn nom(self) -> &'static str {
        match self {
            Genre::Stridulation => "stridulation",
            Genre::Tymbale => "tymbale",
            Genre::Ailes => "ailes",
            Genre::Syrinx => "syrinx",
            Genre::Larynx => "larynx",
            Genre::Sac => "sac vocal",
            Genre::Percussion => "percussion",
            Genre::Vessie => "vessie natatoire",
            Genre::Clics => "clics",
        }
    }
    /// Taille typique de l'organe, m (pour les bancs d'essai).
    pub fn taille_type(self) -> f32 {
        match self {
            Genre::Stridulation => 0.003,
            Genre::Tymbale => 0.006,
            Genre::Ailes => 0.01,
            Genre::Syrinx => 0.01,
            Genre::Larynx => 0.08,
            Genre::Sac => 0.01,
            Genre::Percussion => 0.1,
            Genre::Vessie => 0.04,
            Genre::Clics => 0.1,
        }
    }
}

/// Moment de la journée où l'espèce appelle, comme `evo_life::CallTime`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heure {
    Jour,
    Crepuscule,
    Nuit,
}

/// Organe sonore d'une espèce, tel que le son le lit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Organe {
    pub genre: Genre,
    /// Taille de l'organe, m.
    pub taille_m: f32,
    /// Puissance et complexité de 0 à 1.
    pub puissance: f32,
    pub complexite: f32,
    pub choeur: bool,
    pub heure: Heure,
    pub graine: u64,
}

/// Une syllabe du motif d'une espèce : départ, durée, hauteur (rapport à la
/// hauteur de base) au début et à la fin.
#[derive(Clone, Copy, Debug)]
struct Syllabe {
    t: f32,
    dur: f32,
    h0: f32,
    h1: f32,
}

/// Hauteur de base d'un organe, Hz : inverse de sa taille, bornée.
fn base(o: &Organe) -> f32 {
    let s = o.taille_m.max(1e-4);
    match o.genre {
        Genre::Stridulation => (250.0 / s.sqrt()).clamp(2500.0, 9000.0),
        Genre::Tymbale => (300.0 / s.sqrt()).clamp(2000.0, 7000.0),
        Genre::Ailes => (2.5 / s).clamp(90.0, 600.0),
        Genre::Syrinx => (300.0 / s.sqrt()).clamp(1200.0, 6000.0),
        Genre::Larynx => (60.0 / s.powf(0.7)).clamp(45.0, 1100.0),
        Genre::Sac => (60.0 / s.sqrt()).clamp(150.0, 1400.0),
        Genre::Percussion => (12.0 / s).clamp(40.0, 400.0),
        Genre::Vessie => (25.0 / s.sqrt()).clamp(70.0, 320.0),
        Genre::Clics => (2500.0 / s.sqrt()).clamp(4000.0, 12000.0),
    }
}

/// Le motif d'une espèce, fixé par sa graine : une à sept syllabes selon la
/// complexité, avec leurs durées, leurs silences et leurs glissés.
fn motif(o: &Organe) -> Vec<Syllabe> {
    let mut r = Rng::new(o.graine | 1);
    let n = 1 + (o.complexite.clamp(0.0, 1.0) * 6.0).round() as usize;
    let (dmin, dmax, gap) = match o.genre {
        Genre::Stridulation => (0.08, 0.35, 0.12),
        Genre::Tymbale => (0.4, 1.4, 0.3),
        Genre::Ailes => (0.5, 1.5, 0.2),
        Genre::Syrinx => (0.05, 0.3, 0.06),
        Genre::Larynx => (0.15, 0.9, 0.15),
        Genre::Sac => (0.12, 0.45, 0.15),
        Genre::Percussion => (0.04, 0.12, 0.08),
        Genre::Vessie => (0.15, 0.6, 0.2),
        Genre::Clics => (0.1, 0.6, 0.1),
    };
    // Glissé typique : les sifflets et les cris glissent, les insectes non.
    let glisse = match o.genre {
        Genre::Syrinx => 0.5,
        Genre::Larynx | Genre::Clics => 0.3,
        Genre::Sac | Genre::Vessie => 0.1,
        _ => 0.03,
    };
    let mut t = 0.0;
    let mut h = 1.0f32;
    // Un motif répète souvent sa première syllabe : seuls les plus complexes
    // varient à chaque note.
    let repete = r.u() > o.complexite;
    let premiere = (r.range(dmin, dmax), r.range(-glisse, glisse));
    (0..n)
        .map(|i| {
            let (dur, g) = if repete || i == 0 { premiere } else { (r.range(dmin, dmax), r.range(-glisse, glisse)) };
            if i > 0 && !repete {
                h *= 2f32.powf(r.range(-0.25, 0.25) * (0.3 + glisse));
            }
            let s = Syllabe { t, dur, h0: h, h1: h * 2f32.powf(g) };
            t += dur + gap * r.range(0.6, 1.4);
            s
        })
        .collect()
}

/// Durée d'un appel de l'organe, s.
pub fn duree(o: &Organe) -> f32 {
    motif(o).last().map_or(0.0, |s| s.t + s.dur)
}

/// Voix d'une syllabe.
fn syllabe(o: &Organe, s: &Syllabe, f: f32, mk: &Make, rng: &mut Rng) -> Kind {
    let (f0, f1, d) = (f * s.h0, f * s.h1, s.dur);
    let c = o.complexite;
    match o.genre {
        // Ton pur haché en impulsions sèches : 20 à 70 coups par seconde.
        Genre::Stridulation => mk.trille(f0, f1, 25.0 + 45.0 * c, 0.6, 0.15, 0.2, d, 0.004),
        // Bruit grésillant très rapide.
        Genre::Tymbale => mk.trille(f0, f1, 120.0 + 200.0 * c, 0.3, 0.85, 0.0, d, 0.08),
        // Bourdon riche en harmoniques, battement des ailes.
        Genre::Ailes => mk.trille(f0, f1 * 1.02, f0 * 0.5, 0.05, 0.1, 0.9, d, 0.1),
        // Sifflet modulé.
        Genre::Syrinx => mk.trille(f0, f1, 0.0, 0.0, 0.02, 0.15, d, 0.01),
        // Voyelle ouverte qui se ferme : cri, aboiement ou mugissement.
        Genre::Larynx => {
            let k = (o.taille_m / 0.05).clamp(0.3, 8.0);
            let conduit = 1.0 / k.powf(0.35);
            let ouvert = [800.0, 1250.0, 2600.0];
            let ferme = [400.0, 900.0, 2400.0];
            let segs = vec![
                Seg { ph: Ph::Voyelle(ouvert), dur: d * 0.6, f0, amp: 1.0 },
                Seg { ph: Ph::Voyelle(ferme), dur: d * 0.4, f0: f1, amp: 0.8 },
            ];
            let loc = Locuteur { f0, conduit, souffle: 0.02 + 0.03 * rng.u() };
            Kind::Gosier(Box::new(Gosier::new(segs, loc, 0.0, mk.rate)))
        }
        // Croassement : son grave haché, riche en harmoniques.
        Genre::Sac => mk.trille(f0, f1, 18.0 + 60.0 * c, 0.45, 0.1, 0.8, d, 0.01),
        // Coup sourd.
        Genre::Percussion => mk.thump(f0 * 1.8, d * 3.0),
        // Grognement sourd et pulsé.
        Genre::Vessie => mk.trille(f0, f1, 15.0 + 50.0 * c, 0.5, 0.2, 0.6, d, 0.02),
        // Train de clics, puis un sifflet chez les plus complexes.
        Genre::Clics => {
            if c > 0.5 && s.h0 != 1.0 {
                mk.trille(f0 * 0.9, f1 * 1.2, 0.0, 0.0, 0.0, 0.05, d, 0.02)
            } else {
                mk.trille(f0, f0, 8.0 + 120.0 * c * c, 1.0, 1.0, 0.0, d, 0.002)
            }
        }
    }
}

/// Gain d'un appel selon le type et la puissance de l'organe.
fn gain(o: &Organe) -> f32 {
    let g = match o.genre {
        Genre::Stridulation => 0.12,
        Genre::Syrinx => 0.035,
        Genre::Tymbale => 0.12,
        Genre::Ailes => 0.06,
        Genre::Larynx => 0.35,
        Genre::Sac => 0.1,
        Genre::Percussion => 0.09,
        Genre::Vessie => 0.12,
        Genre::Clics => 0.06,
    };
    g * (0.35 + 0.65 * o.puissance.clamp(0.0, 1.0))
}

/// Un appel de l'espèce : son motif joué par un individu à `distance`
/// mètres, plus ses voisins s'il appelle en chœur. `t0` : départ, s.
#[allow(clippy::too_many_arguments)]
pub fn appel(o: &Organe, distance: f32, pan: f32, t0: f32, choeur: bool, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
    let m = motif(o);
    let individus = if choeur && o.choeur { 2 + (rng.u() * 4.0) as usize } else { 1 };
    for k in 0..individus {
        // Chaque individu a sa hauteur, un peu décalé des autres.
        let f = base(o) * 2f32.powf(rng.range(-0.06, 0.06) + if k > 0 { rng.range(-0.08, 0.08) } else { 0.0 });
        let dt = if k == 0 { 0.0 } else { rng.range(0.1, 1.6) };
        let d = if k == 0 { distance } else { distance * rng.range(0.7, 1.6) };
        let att = 1.0 / (1.0 + d / 8.0);
        let p = if k == 0 { pan } else { (pan + rng.range(-0.5, 0.5)).clamp(-1.0, 1.0) };
        let g = gain(o) * att;
        for s in &m {
            let v =
                Voice::new(syllabe(o, s, f, mk, rng), Bus::Ambiance, g).at(t0 + dt + s.t, mk.rate).pan(p).send((0.1 + d / 60.0).min(0.7));
            out.push(v);
        }
    }
}

/// Une espèce de la scène : son organe, combien on en voit, et le temps
/// avant son prochain appel.
struct Espece {
    organe: Organe,
    abondance: f32,
    prochain: f32,
}

/// La faune d'une scène au sol : chaque espèce appelle à son rythme.
#[derive(Default)]
pub struct Faune {
    especes: Vec<Espece>,
}

/// Voix d'appels au plus en même temps.
pub const MAX_VOIX: usize = 48;

impl Faune {
    /// Les espèces de la scène (organe, individus visibles).
    pub fn new(especes: &[(Organe, u32)], rng: &mut Rng) -> Self {
        Self { especes: especes.iter().map(|&(organe, n)| Espece { organe, abondance: n as f32, prochain: rng.range(1.0, 8.0) }).collect() }
    }

    pub fn len(&self) -> usize {
        self.especes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.especes.is_empty()
    }

    /// Avance de `dt` ; `voix` : appels qui sonnent déjà.
    pub fn tick(&mut self, dt: f32, voix: usize, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        for e in &mut self.especes {
            e.prochain -= dt;
            if e.prochain > 0.0 {
                continue;
            }
            let o = e.organe;
            // Une espèce puissante et nombreuse appelle plus souvent ; celles
            // du crépuscule et de la nuit, rarement en plein jour.
            let heure = match o.heure {
                Heure::Jour => 1.0,
                Heure::Crepuscule => 0.45,
                Heure::Nuit => 0.2,
            };
            let rythme = heure * (0.3 + 0.7 * o.puissance) * (1.0 + e.abondance.max(1.0).ln() * 0.5);
            e.prochain = rng.range(6.0, 18.0) / rythme.max(0.05) + duree(&o);
            if voix + out.len() >= MAX_VOIX {
                continue;
            }
            let distance = rng.range(4.0, 40.0);
            appel(&o, distance, rng.bipolar() * 0.85, 0.0, true, mk, rng, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn organe(genre: Genre, complexite: f32, graine: u64) -> Organe {
        Organe { genre, taille_m: genre.taille_type(), puissance: 0.7, complexite, choeur: true, heure: Heure::Jour, graine }
    }

    fn rendre(voix: &mut Vec<Voice>, secondes: f32) -> (f32, f32) {
        let rate = 48_000.0;
        let mut rng = Rng::new(9);
        let (mut crete, mut somme) = (0.0f32, 0.0f64);
        let n = (rate * secondes) as usize;
        for _ in 0..n {
            let mut x = 0.0;
            voix.retain_mut(|v| match v.next(&mut rng, 1.0 / rate, rate) {
                Some(s) => {
                    x += s;
                    true
                }
                None => false,
            });
            crete = crete.max(x.abs());
            somme += (x * x) as f64;
        }
        (crete, (somme / n as f64).sqrt() as f32)
    }

    #[test]
    fn every_organ_calls_audibly_and_the_seed_fixes_the_motif() {
        let mk = Make { rate: 48_000.0 };
        for g in Genre::TOUS {
            let o = organe(g, 0.6, 42);
            let mut rng = Rng::new(1);
            let mut v = Vec::new();
            appel(&o, 5.0, 0.0, 0.0, false, &mk, &mut rng, &mut v);
            assert!(!v.is_empty(), "{}", g.nom());
            let d = duree(&o);
            assert!(d > 0.03 && d < 15.0, "{} : {d} s", g.nom());
            let (crete, rms) = rendre(&mut v, d + 1.0);
            assert!(rms > 1e-3 && crete < 0.8, "{} : crête {crete}, efficace {rms}", g.nom());
            assert!(v.is_empty(), "{} : l'appel se termine", g.nom());
            // Même graine, même motif ; autre graine, autre motif.
            assert_eq!(duree(&o), duree(&organe(g, 0.6, 42)));
        }
        let a: Vec<f32> = (0..20).map(|s| duree(&organe(Genre::Syrinx, 0.8, s))).collect();
        assert!(a.iter().any(|&d| (d - a[0]).abs() > 0.05), "les graines varient les motifs");
    }

    #[test]
    fn complex_organs_have_longer_calls_and_choruses_more_voices() {
        let mk = Make { rate: 48_000.0 };
        let simple = organe(Genre::Sac, 0.0, 7);
        let riche = organe(Genre::Sac, 1.0, 7);
        assert_eq!(motif(&simple).len(), 1);
        assert_eq!(motif(&riche).len(), 7);
        let mut rng = Rng::new(3);
        let (mut seul, mut choeur) = (Vec::new(), Vec::new());
        appel(&riche, 5.0, 0.0, 0.0, false, &mk, &mut rng, &mut seul);
        appel(&riche, 5.0, 0.0, 0.0, true, &mk, &mut rng, &mut choeur);
        assert!(choeur.len() >= 2 * seul.len());
        // Plus gros organe, plus grave.
        let gros = Organe { taille_m: 0.5, ..organe(Genre::Larynx, 0.5, 1) };
        let petit = Organe { taille_m: 0.02, ..organe(Genre::Larynx, 0.5, 1) };
        assert!(base(&gros) < base(&petit) / 4.0);
    }

    #[test]
    fn a_scene_calls_without_flooding() {
        let mk = Make { rate: 48_000.0 };
        let mut rng = Rng::new(5);
        let especes: Vec<(Organe, u32)> = Genre::TOUS.iter().enumerate().map(|(i, &g)| (organe(g, 0.5, i as u64), 30)).collect();
        let mut f = Faune::new(&especes, &mut rng);
        let mut appels = 0;
        let mut out = Vec::new();
        for _ in 0..(120 * 10) {
            out.clear();
            f.tick(0.1, 0, &mk, &mut rng, &mut out);
            appels += usize::from(!out.is_empty());
            assert!(out.len() <= MAX_VOIX + 40);
        }
        // Neuf espèces sur deux minutes : chacune appelle, mais pas sans arrêt.
        assert!((15..=200).contains(&appels), "{appels} appels");
    }
}
