//! Sons des peuples (livraison 3, volet de l'étape 6) : les langues parlées
//! et les chants rituels.
//!
//! Les mots viennent du générateur de noms d'`evo-culture` : chaque langue a
//! son inventaire de sons, et ses mots sont écrits pour un lecteur français
//! (« kaimou », « tsélang »). Le son les prononce avec une voix de synthèse
//! à formants : une source glottale, trois résonances du conduit vocal qui
//! glissent d'un son à l'autre, des bruits pour les consonnes. Deux langues
//! qui n'ont pas les mêmes sons ne sonnent donc pas pareil, et un changement
//! de son dans la langue s'entend dans tous ses mots.
//!
//! Les chants rituels sont composés à partir du peuple et du rite : le
//! peuple fixe une tradition (gamme, mesure, façon de chanter à plusieurs,
//! percussions), l'objet du rite fixe le caractère (lent et grave pour les
//! morts, vif et frappé pour la chasse). Un meneur lance, le chœur répond,
//! sur des syllabes de la langue.
//!
//! [Simplification] Une voix humaine de synthèse, quelle que soit l'espèce :
//! seule la taille du corps change la hauteur et le timbre. Les espèces qui
//! ne parlent pas par le son entendront leurs voix propres avec la
//! livraison 2 (organes sonores). Pas de grammaire ni d'intonation propre à
//! chaque langue au-delà de l'accent de mot.

use crate::synth::{mtof, Biquad, Bus, FilterKind, Make, Rng, Smooth, Voice};
use std::f32::consts::{PI, TAU};

/// Un son de la langue, avec ce qu'il faut pour le prononcer. Les
/// formants sont ceux d'un conduit d'homme adulte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ph {
    Voyelle([f32; 3]),
    /// Voyelle qui glisse vers une autre (« ai »).
    Diphtongue([f32; 3], [f32; 3]),
    /// Occlusive : silence puis explosion de bruit autour de `burst` Hz.
    Occlusive {
        burst: f32,
        sonore: bool,
    },
    Fricative {
        f: f32,
        q: f32,
        g: f32,
        sonore: bool,
    },
    Nasale([f32; 3]),
    Liquide {
        f: [f32; 3],
        roulee: bool,
    },
    Glissante([f32; 3]),
    Aspiree,
    Pause,
}

impl Ph {
    pub fn voyelle(self) -> bool {
        matches!(self, Ph::Voyelle(_) | Ph::Diphtongue(..))
    }
}

const A: [f32; 3] = [760.0, 1300.0, 2550.0];
const I: [f32; 3] = [280.0, 2250.0, 2950.0];

/// Les sons écrits par `evo_culture::language` (CONSONANTS et VOWELS).
fn son(s: &str) -> Option<Ph> {
    Some(match s {
        "a" => Ph::Voyelle(A),
        "i" => Ph::Voyelle(I),
        "ou" => Ph::Voyelle([310.0, 760.0, 2300.0]),
        "e" => Ph::Voyelle([500.0, 1450.0, 2450.0]),
        "o" => Ph::Voyelle([420.0, 820.0, 2550.0]),
        "é" => Ph::Voyelle([380.0, 2050.0, 2650.0]),
        "u" => Ph::Voyelle([280.0, 1800.0, 2200.0]),
        "è" => Ph::Voyelle([560.0, 1800.0, 2500.0]),
        "eu" => Ph::Voyelle([380.0, 1500.0, 2300.0]),
        "ai" => Ph::Diphtongue(A, I),
        "m" => Ph::Nasale([260.0, 1100.0, 2200.0]),
        "n" => Ph::Nasale([260.0, 1600.0, 2600.0]),
        "ng" => Ph::Nasale([260.0, 2000.0, 2700.0]),
        "p" => Ph::Occlusive { burst: 900.0, sonore: false },
        "b" => Ph::Occlusive { burst: 900.0, sonore: true },
        "t" => Ph::Occlusive { burst: 4200.0, sonore: false },
        "d" => Ph::Occlusive { burst: 4200.0, sonore: true },
        "k" => Ph::Occlusive { burst: 2100.0, sonore: false },
        "g" => Ph::Occlusive { burst: 2100.0, sonore: true },
        "q" => Ph::Occlusive { burst: 1200.0, sonore: false },
        "s" => Ph::Fricative { f: 6500.0, q: 2.0, g: 0.7, sonore: false },
        "z" => Ph::Fricative { f: 6500.0, q: 2.0, g: 0.5, sonore: true },
        "ch" => Ph::Fricative { f: 3200.0, q: 1.6, g: 0.8, sonore: false },
        "j" => Ph::Fricative { f: 3200.0, q: 1.6, g: 0.5, sonore: true },
        "f" => Ph::Fricative { f: 5000.0, q: 0.5, g: 0.25, sonore: false },
        "v" => Ph::Fricative { f: 5000.0, q: 0.5, g: 0.18, sonore: true },
        "kh" => Ph::Fricative { f: 1600.0, q: 1.0, g: 0.5, sonore: false },
        "x" => Ph::Fricative { f: 1900.0, q: 1.2, g: 0.45, sonore: false },
        "h" => Ph::Aspiree,
        "l" => Ph::Liquide { f: [360.0, 1250.0, 2800.0], roulee: false },
        "r" => Ph::Liquide { f: [450.0, 1300.0, 1700.0], roulee: true },
        "w" => Ph::Glissante([300.0, 700.0, 2200.0]),
        "y" => Ph::Glissante([280.0, 2200.0, 3000.0]),
        _ => return None,
    })
}

/// Sons d'un mot, lu de gauche à droite en prenant d'abord les graphies de
/// deux lettres (« ou », « ch », « ng »). « ts » donne deux sons.
pub fn phonemes(mot: &str) -> Vec<Ph> {
    let c: Vec<char> = mot.to_lowercase().chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if i + 1 < c.len() {
            let deux: String = c[i..i + 2].iter().collect();
            if deux == "ts" {
                out.extend([son("t"), son("s")].into_iter().flatten());
                i += 2;
                continue;
            }
            if let Some(p) = son(&deux) {
                out.push(p);
                i += 2;
                continue;
            }
        }
        if let Some(p) = son(&c[i].to_string()) {
            out.push(p);
        }
        i += 1;
    }
    out
}

/// Syllabes d'un mot : une voyelle chacune ; entre deux voyelles, une
/// consonne seule ouvre la syllabe suivante, deux se partagent.
pub fn syllabes(mot: &str) -> Vec<Vec<Ph>> {
    let p = phonemes(mot);
    let noyaux: Vec<usize> = (0..p.len()).filter(|&i| p[i].voyelle()).collect();
    if noyaux.is_empty() {
        return if p.is_empty() { Vec::new() } else { vec![p] };
    }
    let mut out = Vec::new();
    let mut debut = 0;
    for (k, &n) in noyaux.iter().enumerate() {
        let fin = match noyaux.get(k + 1) {
            None => p.len(),
            Some(&m) => {
                let entre = m - n - 1;
                n + 1 + entre / 2
            }
        };
        out.push(p[debut..fin].to_vec());
        debut = fin;
    }
    out
}

/// Une voix : hauteur moyenne, taille du conduit (1 pour un homme, plus
/// pour un conduit plus court), part de souffle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Locuteur {
    pub f0: f32,
    pub conduit: f32,
    pub souffle: f32,
}

impl Locuteur {
    /// Une personne au hasard (homme, femme ou enfant) d'une espèce dont le
    /// corps fait `taille` fois celui d'un humain.
    pub fn tire(rng: &mut Rng, taille: f32) -> Self {
        let (f0, conduit) = match (rng.u() * 10.0) as u32 {
            0..=3 => (rng.range(95.0, 135.0), rng.range(0.95, 1.05)),
            4..=7 => (rng.range(170.0, 230.0), rng.range(1.12, 1.2)),
            _ => (rng.range(250.0, 320.0), rng.range(1.28, 1.36)),
        };
        let k = taille.clamp(0.2, 5.0);
        Self { f0: f0 / k.sqrt(), conduit: conduit / k.powf(0.33), souffle: rng.range(0.02, 0.08) }
    }
    pub fn homme(rng: &mut Rng) -> Self {
        Self { f0: rng.range(100.0, 130.0), conduit: rng.range(0.96, 1.04), souffle: 0.04 }
    }
    pub fn femme(rng: &mut Rng) -> Self {
        Self { f0: rng.range(185.0, 225.0), conduit: rng.range(1.13, 1.19), souffle: 0.05 }
    }
}

/// Un son tenu : quoi, combien de temps, à quelle hauteur, à quel volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub ph: Ph,
    pub dur: f32,
    pub f0: f32,
    pub amp: f32,
}

/// Voix de synthèse à formants qui dit ou chante une suite de sons.
pub struct Gosier {
    segs: Vec<Seg>,
    i: usize,
    t: f32,
    rate: f32,
    conduit: f32,
    souffle: f32,
    /// Vibrato des voyelles chantées, en demi-tons (0 en parlant).
    vibrato: f32,
    phase: f32,
    vph: f32,
    vib: f32,
    prev: f32,
    tilt: f32,
    f: [Smooth; 3],
    f0: Smooth,
    voix: Smooth,
    bruit: Smooth,
    formants: [Biquad; 3],
    friction: Biquad,
    tick: u32,
    reste: f32,
}

impl Gosier {
    pub fn new(segs: Vec<Seg>, loc: Locuteur, vibrato: f32, rate: f32) -> Self {
        let f0 = segs.first().map_or(loc.f0, |s| s.f0);
        let mut g = Self {
            segs,
            i: 0,
            t: 0.0,
            rate,
            conduit: loc.conduit,
            souffle: loc.souffle,
            vibrato,
            phase: 0.0,
            vph: 0.0,
            vib: 1.0,
            prev: 0.0,
            tilt: 0.0,
            f: [Smooth::new(500.0), Smooth::new(1500.0), Smooth::new(2500.0)],
            f0: Smooth::new(f0),
            voix: Smooth::new(0.0),
            bruit: Smooth::new(0.0),
            formants: [Biquad::new(FilterKind::BandPass, 500.0, 6.0, rate); 3],
            friction: Biquad::new(FilterKind::BandPass, 4000.0, 1.0, rate),
            tick: 0,
            reste: 0.08,
        };
        g.entrer();
        let c = g.cibles();
        for (s, f) in g.f.iter_mut().zip(c) {
            *s = Smooth::new(f);
        }
        g
    }

    pub fn duree(&self) -> f32 {
        self.segs.iter().map(|s| s.dur).sum()
    }

    fn cibles(&self) -> [f32; 3] {
        let Some(s) = self.segs.get(self.i) else { return [500.0, 1500.0, 2500.0] };
        let k = self.conduit;
        let f = match s.ph {
            Ph::Voyelle(f) | Ph::Nasale(f) | Ph::Glissante(f) | Ph::Liquide { f, .. } => f,
            Ph::Diphtongue(a, b) => {
                if self.t < s.dur * 0.45 {
                    a
                } else {
                    b
                }
            }
            // Les consonnes gardent les formants de la voyelle qui suit.
            _ => match self.segs[self.i..].iter().find(|s| s.ph.voyelle()).map(|s| s.ph) {
                Some(Ph::Voyelle(f)) | Some(Ph::Diphtongue(f, _)) => f,
                _ => [500.0, 1500.0, 2500.0],
            },
        };
        [f[0] * k, f[1] * k, f[2] * k]
    }

    /// Règle les cibles en entrant dans un nouveau son.
    fn entrer(&mut self) {
        let Some(s) = self.segs.get(self.i).copied() else {
            self.voix.target = 0.0;
            self.bruit.target = 0.0;
            return;
        };
        self.f0.target = s.f0;
        let (v, n) = match s.ph {
            Ph::Voyelle(_) | Ph::Diphtongue(..) => (1.0, 0.0),
            Ph::Glissante(_) | Ph::Liquide { .. } => (0.7, 0.0),
            Ph::Nasale(_) => (0.45, 0.0),
            Ph::Occlusive { sonore, .. } => (if sonore { 0.15 } else { 0.0 }, 0.0),
            Ph::Fricative { f, q, g, sonore } => {
                self.friction.set(FilterKind::BandPass, f, q, self.rate);
                (if sonore { 0.35 } else { 0.0 }, g)
            }
            Ph::Aspiree => (0.0, 0.0),
            Ph::Pause => (0.0, 0.0),
        };
        self.voix.target = v * s.amp;
        self.bruit.target = n * s.amp;
        if let Ph::Occlusive { burst, .. } = s.ph {
            self.friction.set(FilterKind::BandPass, burst * self.conduit, 1.2, self.rate);
        }
    }

    #[inline]
    pub fn next(&mut self, rng: &mut Rng, dt: f32) -> Option<f32> {
        if self.i >= self.segs.len() {
            self.reste -= dt;
            if self.reste < 0.0 {
                return None;
            }
        } else {
            self.t += dt;
            if self.t >= self.segs[self.i].dur {
                self.t -= self.segs[self.i].dur;
                self.i += 1;
                self.entrer();
            }
        }
        let seg = self.segs.get(self.i).copied();
        // Silence (entre deux phrases, ou pendant que l'autre partie
        // chante) : rien à calculer.
        if seg.is_none_or(|s| s.ph == Ph::Pause) && self.voix.value < 1e-4 && self.bruit.value < 1e-4 {
            self.voix.value = 0.0;
            self.bruit.value = 0.0;
            return Some(0.0);
        }
        self.tick = self.tick.wrapping_add(1);
        if self.tick.is_multiple_of(32) {
            let c = self.cibles();
            let tau = if matches!(seg.map(|s| s.ph), Some(Ph::Occlusive { .. })) { 0.012 } else { 0.022 };
            for k in 0..3 {
                self.f[k].target = c[k];
                let f = self.f[k].step(dt * 32.0, tau);
                // Largeurs de bande de 60, 90 et 140 Hz environ.
                let bw = [60.0, 90.0, 140.0][k] * self.conduit;
                self.formants[k].set(FilterKind::BandPass, f, f / bw, self.rate);
            }
            self.vib = 1.0;
            if self.vibrato > 0.0 && seg.is_some_and(|s| s.ph.voyelle()) && self.t > 0.18 {
                self.vph = (self.vph + 5.4 * dt * 32.0).fract();
                let k = ((self.t - 0.18) / 0.3).min(1.0);
                self.vib = 2f32.powf(self.vibrato * k * (TAU * self.vph).sin() / 12.0);
            }
        }
        let mut f0 = self.f0.step(dt, 0.035) * self.vib;
        f0 *= 1.0 + 0.006 * rng.bipolar();
        self.phase += f0 * dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        // Impulsion glottale de Rosenberg, dérivée (débit d'air rayonné).
        let p = self.phase;
        let g = if p < 0.4 {
            0.5 * (1.0 - (PI * p / 0.4).cos())
        } else if p < 0.56 {
            (PI * (p - 0.4) / 0.32).cos()
        } else {
            0.0
        };
        let d = (g - self.prev) / (f0 * dt).max(1e-4) * 0.08;
        self.prev = g;
        self.tilt += (d - self.tilt) * 0.5;
        let mut v = self.voix.step(dt, 0.015);
        if let Some(Seg { ph: Ph::Liquide { roulee: true, .. }, .. }) = seg {
            v *= 0.55 + 0.45 * (TAU * 28.0 * self.t).sin();
        }
        let white = rng.bipolar();
        let mut src = self.tilt * v + white * self.souffle * v;
        let mut friction = self.bruit.step(dt, 0.01);
        match seg.map(|s| (s.ph, s.dur, s.amp)) {
            Some((Ph::Occlusive { .. }, dur, amp)) => {
                let x = self.t - dur * 0.7;
                if x > 0.0 {
                    let e = (-x / 0.012).exp() * amp;
                    friction = 0.9 * e;
                    src += white * 0.25 * e;
                }
            }
            Some((Ph::Aspiree, _, amp)) => src += white * 0.35 * amp,
            _ => {}
        }
        let voc = self.formants[0].run(src) + 0.55 * self.formants[1].run(src) + 0.28 * self.formants[2].run(src);
        let s = voc + self.friction.run(white) * friction * 0.5;
        Some(s * 0.6)
    }
}

/// Ce que le son sait d'une langue : quelques mots (ceux que publie l'état
/// culturel) et une clé stable.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Langue {
    pub mots: Vec<String>,
    pub cle: u64,
}

impl Langue {
    /// Réserve de syllabes de la langue, pour le babil.
    pub fn syllabes(&self) -> Vec<Vec<Ph>> {
        let mut out: Vec<Vec<Ph>> = Vec::new();
        for m in &self.mots {
            for s in syllabes(m) {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        if out.is_empty() {
            out.push(phonemes("ma"));
        }
        out
    }

    /// Place de l'accent : première syllabe, avant-dernière ou dernière.
    pub fn accent(&self) -> u8 {
        (self.cle % 3) as u8
    }

    /// Un mot de babil : des syllabes de la langue mises bout à bout.
    pub fn babil(&self, rng: &mut Rng, reserve: &[Vec<Ph>]) -> Vec<Vec<Ph>> {
        let n = match (rng.u() * 10.0) as u32 {
            0..=2 => 1,
            3..=7 => 2,
            _ => 3,
        };
        (0..n).map(|_| reserve[(rng.u() * reserve.len() as f32) as usize % reserve.len()].clone()).collect()
    }
}

/// Durée parlée d'un son.
fn duree_dite(p: Ph, rng: &mut Rng) -> f32 {
    rng.range(0.85, 1.15)
        * match p {
            Ph::Voyelle(_) => 0.1,
            Ph::Diphtongue(..) => 0.15,
            Ph::Occlusive { .. } => 0.075,
            Ph::Fricative { .. } => 0.09,
            Ph::Nasale(_) => 0.065,
            Ph::Liquide { .. } => 0.06,
            Ph::Glissante(_) => 0.05,
            Ph::Aspiree => 0.06,
            Ph::Pause => 0.1,
        }
}

/// Une phrase parlée : des mots (en syllabes), une voix, l'accent de la
/// langue. La voix descend au fil de la phrase et monte sur les syllabes
/// accentuées.
pub fn phrase(mots: &[Vec<Vec<Ph>>], loc: Locuteur, accent: u8, rng: &mut Rng) -> Vec<Seg> {
    let total: usize = mots.iter().map(|m| m.len()).sum::<usize>().max(1);
    let question = rng.chance(0.15);
    let mut segs = Vec::new();
    let mut k = 0;
    for (w, mot) in mots.iter().enumerate() {
        let n = mot.len();
        let tonique = match accent {
            0 => 0,
            1 => n.saturating_sub(2),
            _ => n - 1,
        };
        for (s, syl) in mot.iter().enumerate() {
            let x = k as f32 / total as f32;
            let mut f0 = loc.f0 * (1.12 - 0.27 * x);
            if question && w + 1 == mots.len() && s + 1 == n {
                f0 = loc.f0 * 1.3;
            }
            let fort = s == tonique;
            if fort {
                f0 *= 1.12;
            }
            let fin = w + 1 == mots.len() && s + 1 == n;
            for &p in syl {
                let mut dur = duree_dite(p, rng);
                if p.voyelle() {
                    dur *= if fort { 1.4 } else { 1.0 } * if fin { 1.5 } else { 1.0 };
                }
                segs.push(Seg { ph: p, dur, f0, amp: if fort { 1.0 } else { 0.8 } });
            }
            k += 1;
        }
        if w + 1 < mots.len() && rng.chance(0.35) {
            segs.push(Seg { ph: Ph::Pause, dur: rng.range(0.04, 0.12), f0: loc.f0, amp: 0.0 });
        }
    }
    segs
}

/// Prononce un mot (un nom de peuple, de langue, de personne).
pub fn prononcer(mot: &str, langue: &Langue, loc: Locuteur, rng: &mut Rng, rate: f32) -> Gosier {
    let m: Vec<Vec<Vec<Ph>>> = mot.split_whitespace().map(syllabes).filter(|s| !s.is_empty()).collect();
    Gosier::new(phrase(&m, loc, langue.accent(), rng), loc, 0.0, rate)
}

/// Un rite tel que le son le lit : son nom (« rite du Kalo, pour les
/// morts ») et son objet (« les morts »).
#[derive(Clone, Debug, PartialEq)]
pub struct Rite {
    pub nom: String,
    pub objet: String,
}

impl Rite {
    /// Lit l'objet dans le nom généré par `evo_culture::traits`.
    pub fn depuis_nom(nom: &str) -> Self {
        let objet = nom.split_once(", pour ").map_or("", |(_, o)| o).to_string();
        Self { nom: nom.to_string(), objet }
    }

    /// Le mot propre du rite (« Kalo »).
    pub fn mot(&self) -> Option<String> {
        self.nom
            .split(|c: char| c.is_whitespace() || c == ',' || c == '\'')
            .find(|w| w.chars().next().is_some_and(|c| c.is_uppercase()))
            .map(|w| w.to_string())
    }
}

/// Ce que le son sait d'un peuple.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Peuple {
    pub nom: String,
    pub langue: Langue,
    pub rites: Vec<Rite>,
    pub population: f32,
    /// Taille du corps de l'espèce, par rapport à un humain.
    pub taille: f32,
    pub cle: u64,
}

/// Caractère d'un chant selon l'objet du rite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caractere {
    pub tempo: f32,
    /// Décalage de registre, en demi-tons.
    pub registre: i32,
    /// Part de croches (0 : notes longues).
    pub vivacite: f32,
    /// Force des percussions, de 0 à 1.
    pub frappe: f32,
    /// Douceur : moins de voix, plus de souffle.
    pub douceur: f32,
}

pub fn caractere(objet: &str) -> Caractere {
    let c = |tempo, registre, vivacite, frappe, douceur| Caractere { tempo, registre, vivacite, frappe, douceur };
    let o = objet.to_lowercase();
    if o.contains("mort") || o.contains("ancêtre") {
        c(62.0, -3, 0.1, 0.45, 0.4)
    } else if o.contains("chasse") || o.contains("animaux") {
        c(118.0, 0, 0.6, 1.0, 0.0)
    } else if o.contains("pluie") || o.contains("eau") || o.contains("mer") {
        c(84.0, 0, 0.35, 0.5, 0.3)
    } else if o.contains("soleil") || o.contains("feu") {
        c(104.0, 2, 0.45, 0.8, 0.1)
    } else if o.contains("lune") || o.contains("étoile") || o.contains("rêve") {
        c(70.0, 3, 0.2, 0.25, 0.7)
    } else if o.contains("naissance") {
        c(76.0, 2, 0.25, 0.3, 0.6)
    } else {
        c(92.0, 0, 0.35, 0.6, 0.2)
    }
}

/// Gammes des traditions : sans demi-tons pour la plupart, comme beaucoup de
/// musiques de tradition orale.
const GAMMES: [&[i32]; 6] = [&[0, 2, 4, 7, 9], &[0, 3, 5, 7, 10], &[0, 2, 5, 7], &[0, 2, 3, 7, 8], &[0, 2, 5, 7, 9], &[0, 1, 5, 7, 8]];

/// Façon de chanter à plusieurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polyphonie {
    /// Tous à l'unisson (et à l'octave).
    Unisson,
    /// Chacun orne un peu à sa façon.
    Heterophonie,
    /// Un bourdon tenu sous la mélodie.
    Bourdon,
    /// La mélodie doublée à la quarte.
    Quartes,
}

/// Tradition musicale d'un peuple, tirée de sa clé : la même pour tous ses
/// rites.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tradition {
    pub gamme: &'static [i32],
    pub temps: u32,
    pub polyphonie: Polyphonie,
    /// Hochets, battements de mains, bâtons entrechoqués.
    pub hochet: bool,
    pub mains: bool,
    pub batons: bool,
    pub tonique: i32,
}

impl Tradition {
    pub fn de(cle: u64) -> Self {
        let mut r = Rng::new(cle ^ 0x7472_6164);
        Self {
            gamme: GAMMES[(r.u() * GAMMES.len() as f32) as usize % GAMMES.len()],
            temps: r.pick(&[2, 3, 4, 4, 3, 5]),
            polyphonie: r.pick(&[Polyphonie::Unisson, Polyphonie::Heterophonie, Polyphonie::Bourdon, Polyphonie::Quartes]),
            hochet: r.chance(0.6),
            mains: r.chance(0.5),
            batons: r.chance(0.4),
            tonique: 45 + (r.u() * 7.0) as i32,
        }
    }
}

/// Une frappe de percussion.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Coup {
    Tambour(f32),
    Hochet,
    Mains,
    Baton,
}

/// Un chant rituel en cours : les voix chantées sont lancées d'un coup, les
/// percussions au fil du temps.
pub struct Chant {
    pub t: f32,
    pub fin: f32,
    coups: Vec<(f32, Coup)>,
    suivant: usize,
}

/// Une phrase mélodique : (durée en temps, degré de la gamme).
fn melodie(rng: &mut Rng, temps: u32, mesures: u32, vivacite: f32, haut: i32) -> Vec<(f32, i32)> {
    let mut notes = Vec::new();
    let mut d = haut;
    let total = (temps * mesures) as f32;
    let mut t = 0.0;
    while t < total - 2.0 {
        let deux = rng.chance(vivacite);
        for _ in 0..(if deux { 2 } else { 1 }) {
            notes.push((if deux { 0.5 } else { 1.0 }, d));
            // Le chant descend par paliers, avec de petits rebonds.
            d += if rng.chance(0.62) {
                -1
            } else if rng.chance(0.6) {
                0
            } else {
                1
            };
            d = d.clamp(-1, haut + 1);
        }
        t += 1.0;
    }
    // Repos sur la tonique, tenu deux temps.
    notes.push((total - t, 0));
    notes
}

impl Chant {
    /// Compose et lance un chant : renvoie le chant et ses voix chantées.
    pub fn composer(peuple: &Peuple, rite: usize, rng: &mut Rng, mk: &Make) -> (Chant, Vec<Voice>) {
        let rate = mk.rate;
        let tr = Tradition::de(peuple.cle);
        let r = peuple.rites.get(rite).cloned().unwrap_or(Rite { nom: String::new(), objet: String::new() });
        let car = caractere(&r.objet);
        let graine = r.nom.bytes().fold(peuple.cle, |h, b| h.wrapping_mul(0x100_0000_01b3) ^ b as u64);
        let mut cr = Rng::new(graine);
        let tempo = car.tempo * cr.range(0.92, 1.08);
        let beat = 60.0 / tempo;
        let mesures = if tr.temps >= 4 { 2 } else { 3 };
        let appel = melodie(&mut cr, tr.temps, mesures, car.vivacite, 4);
        let reponse = melodie(&mut cr, tr.temps, mesures, car.vivacite, 2);
        let tours = 4 + (cr.u() * 3.0) as usize;
        let racine = tr.tonique + car.registre;

        // Texte : le mot du rite, puis des mots de la langue.
        let mut texte: Vec<Vec<Ph>> = Vec::new();
        if let Some(m) = r.mot() {
            texte.extend(syllabes(&m));
        }
        let reserve = peuple.langue.syllabes();
        while texte.len() < 12 {
            texte.extend(peuple.langue.babil(&mut cr, &reserve));
        }
        let longueur = |n: &[(f32, i32)]| n.iter().map(|x| x.0).sum::<f32>() * beat;
        let (la, lr) = (longueur(&appel), longueur(&reponse));
        let cycle = la + lr;

        // Une ligne chantée : les notes de `phrases`, aux temps donnés, sur
        // le texte. `decal` : degrés ajoutés ; `octave` : demi-tons.
        let chanter =
            |phrases: &[(f32, &Vec<(f32, i32)>)], loc: Locuteur, decal: i32, octave: i32, retard: f32, orne: bool, rng: &mut Rng| {
                let mut segs = vec![Seg { ph: Ph::Pause, dur: retard.max(0.001), f0: loc.f0, amp: 0.0 }];
                let mut t = retard;
                let mut s = 0;
                for &(debut, notes) in phrases {
                    if debut > t {
                        segs.push(Seg { ph: Ph::Pause, dur: debut - t, f0: loc.f0, amp: 0.0 });
                        t = debut;
                    }
                    for &(d, deg) in notes.iter() {
                        let f0 = mtof(crate::synth::degree(racine + octave, tr.gamme, deg + decal) + rng.range(-0.08, 0.08));
                        let dur = d * beat;
                        let syl = &texte[s % texte.len()];
                        s += 1;
                        let cons: f32 = syl.iter().filter(|p| !p.voyelle()).count() as f32 * 0.05;
                        let ornement = orne && dur > 0.4 && rng.chance(0.3);
                        for &p in syl {
                            if p.voyelle() {
                                let v = (dur - cons).max(0.06);
                                if ornement {
                                    let haut = mtof(crate::synth::degree(racine + octave, tr.gamme, deg + decal + 1));
                                    segs.push(Seg { ph: p, dur: v * 0.25, f0: haut, amp: 1.0 });
                                    segs.push(Seg { ph: p, dur: v * 0.75, f0, amp: 1.0 });
                                } else {
                                    segs.push(Seg { ph: p, dur: v, f0, amp: 1.0 });
                                }
                            } else {
                                segs.push(Seg { ph: p, dur: 0.05, f0, amp: 0.9 });
                            }
                        }
                        t += dur;
                    }
                }
                (segs, t)
            };

        let mut voix = Vec::new();
        // Le meneur lance chaque tour.
        let meneur = Locuteur::homme(&mut cr);
        let appels: Vec<(f32, &Vec<(f32, i32)>)> = (0..tours).map(|k| (k as f32 * cycle, &appel)).collect();
        let (segs, fin_meneur) = chanter(&appels, meneur, 0, 0, 0.0, true, &mut cr);
        voix.push(Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, meneur, 0.35, rate))), Bus::Ambiance, 0.5).send(0.35));

        // Le chœur répond, plus nombreux quand le peuple est grand.
        let choristes = (3.0 + peuple.population.max(1.0).ln() * 0.6 * (1.0 - car.douceur * 0.5)).clamp(3.0, 9.0) as usize;
        let reponses: Vec<(f32, &Vec<(f32, i32)>)> = (0..tours).map(|k| (k as f32 * cycle + la, &reponse)).collect();
        let mut fin = fin_meneur;
        for c in 0..choristes {
            let femme = c % 2 == 1;
            let loc = if femme { Locuteur::femme(&mut cr) } else { Locuteur::homme(&mut cr) };
            let octave = if femme { 12 } else { 0 };
            let retard = if tr.polyphonie == Polyphonie::Unisson { cr.range(0.0, 0.02) } else { cr.range(0.0, 0.05) };
            let (segs, t) = match tr.polyphonie {
                Polyphonie::Bourdon if c < 2 => {
                    // Le bourdon tient la tonique sur une voyelle ouverte.
                    let f0 = mtof((racine + octave - 12 * (c == 0) as i32) as f32);
                    let mut s = Vec::new();
                    for k in 0..tours {
                        let debut = k as f32 * cycle + la;
                        s.push(Seg { ph: Ph::Pause, dur: if k == 0 { debut.max(0.001) } else { la }, f0, amp: 0.0 });
                        s.push(Seg { ph: son("m").unwrap_or(Ph::Pause), dur: 0.08, f0, amp: 0.6 });
                        s.push(Seg { ph: son("o").unwrap_or(Ph::Pause), dur: lr - 0.08, f0, amp: 0.7 });
                    }
                    (s, tours as f32 * cycle)
                }
                Polyphonie::Quartes if c % 3 == 2 => chanter(&reponses, loc, 2, octave - 12, retard, false, &mut cr),
                Polyphonie::Heterophonie => chanter(&reponses, loc, 0, octave, retard, true, &mut cr),
                _ => chanter(&reponses, loc, 0, octave, retard, false, &mut cr),
            };
            fin = fin.max(t);
            let g = 0.38 / (choristes as f32).sqrt();
            let pan = cr.range(-0.6, 0.6);
            voix.push(
                Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, loc, 0.25, rate))), Bus::Ambiance, g).pan(pan).send(0.45),
            );
        }

        // Percussions : elles entrent après le premier appel.
        let mut coups = Vec::new();
        let mut t = la;
        let mut n = 0u32;
        while t < fin - beat * 0.5 {
            let premier = n.is_multiple_of(tr.temps);
            if premier || car.frappe > 0.7 || (car.frappe > 0.4 && n.is_multiple_of(2)) {
                coups.push((t, Coup::Tambour(if premier { 1.0 } else { 0.6 } * (0.4 + 0.6 * car.frappe))));
            }
            if tr.hochet && car.frappe > 0.2 {
                coups.push((t + beat * 0.5, Coup::Hochet));
                if car.vivacite > 0.4 {
                    coups.push((t + beat * 0.25, Coup::Hochet));
                    coups.push((t + beat * 0.75, Coup::Hochet));
                }
            }
            if tr.mains && car.frappe > 0.5 && !premier {
                coups.push((t, Coup::Mains));
            }
            if tr.batons && car.frappe > 0.3 && n % tr.temps == tr.temps - 1 {
                coups.push((t + beat * 0.5, Coup::Baton));
            }
            t += beat;
            n += 1;
        }
        coups.sort_by(|a, b| a.0.total_cmp(&b.0));
        let _ = rng;
        (Chant { t: 0.0, fin: fin + 1.5, coups, suivant: 0 }, voix)
    }

    /// Avance de `dt` et lance les frappes qui tombent dans ce pas.
    pub fn tick(&mut self, dt: f32, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        let fin = self.t + dt;
        while let Some(&(t, c)) = self.coups.get(self.suivant) {
            if t >= fin {
                break;
            }
            self.suivant += 1;
            let at = (t - self.t).max(0.0);
            let v = match c {
                Coup::Tambour(g) => Voice::new(mk.thump(rng.range(88.0, 96.0), 0.35), Bus::Ambiance, 0.55 * g).send(0.3),
                Coup::Hochet => Voice::new(mk.burst(FilterKind::HighPass, 4500.0, 4500.0, 0.8, 0.004, 0.07, false), Bus::Ambiance, 0.09)
                    .pan(0.35)
                    .send(0.2),
                Coup::Mains => Voice::new(mk.burst(FilterKind::BandPass, 1400.0, 1100.0, 1.1, 0.001, 0.05, false), Bus::Ambiance, 0.32)
                    .pan(rng.range(-0.4, 0.4))
                    .send(0.35),
                Coup::Baton => Voice::new(mk.bell(rng.range(82.0, 84.0), 2.76, 0.6, 0.12), Bus::Ambiance, 0.12).pan(-0.3).send(0.2),
            };
            out.push(v.at(at, mk.rate));
        }
        self.t = fin;
    }

    pub fn fini(&self) -> bool {
        self.t >= self.fin
    }
}

/// La rumeur d'un peuple : des gens qui se parlent, à quelques pas ou au
/// loin, dans leur langue.
pub struct Rumeur {
    pub peuple: Peuple,
    reserve: Vec<Vec<Ph>>,
    prochain: f32,
    /// Prochain chant spontané (en secondes), si le peuple a des rites.
    pub prochain_chant: f32,
}

impl Rumeur {
    pub fn new(peuple: Peuple, rng: &mut Rng) -> Self {
        let reserve = peuple.langue.syllabes();
        Self { peuple, reserve, prochain: rng.range(0.5, 2.0), prochain_chant: rng.range(25.0, 60.0) }
    }

    /// Lance de temps en temps une phrase ; `level` de 0 à 1 (0 : rien).
    pub fn tick(&mut self, dt: f32, level: f32, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>, parleurs: usize) {
        self.prochain -= dt;
        if self.prochain > 0.0 || level <= 0.01 {
            return;
        }
        let densite = (self.peuple.population.max(1.0).ln() / 6.0).clamp(0.3, 1.5);
        self.prochain = rng.range(0.7, 3.0) / densite;
        if parleurs >= 4 {
            return;
        }
        let loc = Locuteur::tire(rng, if self.peuple.taille > 0.0 { self.peuple.taille } else { 1.0 });
        let n = 1 + (rng.u() * 4.0) as usize;
        let mots: Vec<Vec<Vec<Ph>>> = (0..n).map(|_| self.peuple.langue.babil(rng, &self.reserve)).collect();
        let segs = phrase(&mots, loc, self.peuple.langue.accent(), rng);
        let loin = rng.u();
        let g = level * (0.16 - 0.11 * loin);
        out.push(
            Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, loc, 0.0, mk.rate))), Bus::Ambiance, g)
                .pan(rng.range(-0.85, 0.85))
                .send(0.15 + 0.5 * loin),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_split_into_sounds_and_syllables() {
        assert_eq!(phonemes("tsou").len(), 3);
        assert_eq!(phonemes("ngai"), vec![son("ng").unwrap(), son("ai").unwrap()]);
        let s = syllabes("kaimoulang");
        assert_eq!(s.len(), 3, "{s:?}");
        assert_eq!(s[0], phonemes("kai"));
        assert_eq!(s[2], phonemes("lang"));
        assert_eq!(syllabes("Askè").len(), 2);
    }

    fn rendre(g: &mut Gosier, rng: &mut Rng, rate: f32) -> Vec<f32> {
        let mut v = Vec::new();
        while let Some(x) = g.next(rng, 1.0 / rate) {
            v.push(x);
        }
        v
    }

    #[test]
    fn a_word_is_spoken_with_a_voice() {
        let mut rng = Rng::new(4);
        let langue = Langue { mots: vec!["kalomi".into(), "tsékou".into()], cle: 1 };
        let mut g = prononcer("Kalomi", &langue, Locuteur::homme(&mut rng), &mut rng, 22_050.0);
        let d = g.duree();
        assert!((0.3..1.2).contains(&d), "{d}");
        let v = rendre(&mut g, &mut rng, 22_050.0);
        let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let rms = (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        assert!(v.iter().all(|x| x.is_finite()));
        assert!(peak < 1.5 && rms > 0.01, "crête {peak}, efficace {rms}");
    }

    #[test]
    fn rites_have_a_character_and_peoples_a_tradition() {
        assert!(caractere("les morts").tempo < caractere("la chasse").tempo);
        assert_eq!(Rite::depuis_nom("rite du Kalo, pour les morts").objet, "les morts");
        assert_eq!(Rite::depuis_nom("rite de l'Ama, pour la lune").mot().as_deref(), Some("Ama"));
        // Une tradition par peuple, toujours la même.
        assert_eq!(Tradition::de(7), Tradition::de(7));
        let differentes = (0..20).map(Tradition::de).filter(|t| t.gamme != Tradition::de(0).gamme).count();
        assert!(differentes > 5);
    }

    #[test]
    fn a_chant_has_a_leader_a_chorus_and_drums() {
        let mut rng = Rng::new(9);
        let mk = Make { rate: 22_050.0 };
        let peuple = Peuple {
            nom: "Kalo".into(),
            langue: Langue { mots: vec!["kalo".into(), "mira".into(), "tsoungé".into()], cle: 5 },
            rites: vec![Rite::depuis_nom("rite du Mira, pour la chasse")],
            population: 400.0,
            taille: 1.0,
            cle: 5,
        };
        let (mut chant, voix) = Chant::composer(&peuple, 0, &mut rng, &mk);
        assert!(voix.len() >= 4, "meneur et chœur : {}", voix.len());
        assert!((10.0..90.0).contains(&chant.fin), "{}", chant.fin);
        let mut coups = Vec::new();
        while !chant.fini() {
            chant.tick(0.1, &mk, &mut rng, &mut coups);
        }
        assert!(coups.len() > 10);
    }
}
