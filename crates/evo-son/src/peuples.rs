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

use crate::synth::{mtof, Biquad, Bus, FilterKind, Make, Rng, Smooth, Timbre, Voice};
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
    /// Clés des techniques que connaît sa société (catalogue d'`evo-society`,
    /// étape 7) : elles lui donnent ses instruments de métal, de machine et
    /// d'électricité. Vide avant l'étape 7.
    pub techniques: Vec<String>,
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

/// Manière de célébrer : comment le chant se partage entre les voix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forme {
    /// Un meneur lance, le chœur répond.
    AppelReponse,
    /// La même mélodie, reprise en décalé par trois groupes.
    Canon,
    /// Une voix seule, lente et ornée, sur un bourdon fredonné.
    Lamentation,
    /// Des syllabes martelées sur deux notes, au rythme des pieds.
    Scande,
    /// Chaque chanteur ne tient que quelques notes d'un motif qui tourne ;
    /// le motif n'existe qu'à plusieurs.
    Hoquet,
    /// Le même motif repris en boucle, de plus en plus vite.
    Transe,
}

impl Forme {
    pub const TOUTES: [Forme; 6] = [Forme::AppelReponse, Forme::Canon, Forme::Lamentation, Forme::Scande, Forme::Hoquet, Forme::Transe];
    pub fn nom(self) -> &'static str {
        match self {
            Forme::AppelReponse => "appel et réponse",
            Forme::Canon => "canon",
            Forme::Lamentation => "lamentation",
            Forme::Scande => "chant scandé",
            Forme::Hoquet => "hoquet",
            Forme::Transe => "transe",
        }
    }
}

/// Instruments des traditions, tous faits de ce que l'on trouve sans métal :
/// bois, os, peau, pierre, coquille, graines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instrument {
    Tambour,
    TambourEau,
    /// Tambour de bois fendu, à deux notes.
    Fendu,
    Hochet,
    Mains,
    Batons,
    Racleur,
    BatonPluie,
    Rhombe,
    Flute,
    Conque,
    /// Arc musical : une corde pincée.
    Arc,
    /// Pierres qui sonnent.
    Lithophone,
    Guimbarde,
    // Instruments des techniques (étape 7), du bronze à l'électronique.
    /// Lyre : cordes pincées tendues sur un cadre.
    Lyre,
    /// Trompe de métal.
    Trompe,
    /// Gong de bronze.
    Gong,
    /// Orgue : des tuyaux soufflés par une machine.
    Orgue,
    /// Vièle à archet.
    Vielle,
    /// Piano : cordes d'acier frappées.
    Piano,
    /// Guitare électrique.
    Guitare,
    /// Synthétiseur.
    Synthe,
}

/// Instruments des techniques, du plus ancien au plus récent : chacun vient
/// avec l'une des techniques qui le rendent possible (clés du catalogue
/// d'`evo-society`). [Simplification] Une date par instrument, sans chemin
/// propre à chaque peuple ; les voies sans mains n'en ont pas.
pub const INSTRUMENTS_DES_TECHNIQUES: [(Instrument, &[&str]); 8] = [
    (Instrument::Lyre, &["bronze", "fonte_cuivre"]),
    (Instrument::Trompe, &["fonte_cuivre", "bronze"]),
    (Instrument::Gong, &["bronze"]),
    (Instrument::Orgue, &["moulin_eau", "pompe_vapeur", "aq_acoustique"]),
    (Instrument::Vielle, &["universite", "imprimerie"]),
    (Instrument::Piano, &["machine_outil", "acier_bessemer"]),
    (Instrument::Guitare, &["reseau_electrique", "radio"]),
    (Instrument::Synthe, &["transistor", "ordinateur"]),
];

impl Instrument {
    pub const MELODIQUES: [Instrument; 5] =
        [Instrument::Flute, Instrument::Conque, Instrument::Arc, Instrument::Lithophone, Instrument::Guimbarde];
    pub const FRAPPES: [Instrument; 9] = [
        Instrument::Tambour,
        Instrument::TambourEau,
        Instrument::Fendu,
        Instrument::Hochet,
        Instrument::Mains,
        Instrument::Batons,
        Instrument::Racleur,
        Instrument::BatonPluie,
        Instrument::Rhombe,
    ];
    pub fn nom(self) -> &'static str {
        match self {
            Instrument::Tambour => "tambour",
            Instrument::TambourEau => "tambour d'eau",
            Instrument::Fendu => "tambour de bois fendu",
            Instrument::Hochet => "hochet",
            Instrument::Mains => "mains",
            Instrument::Batons => "bâtons entrechoqués",
            Instrument::Racleur => "racleur",
            Instrument::BatonPluie => "bâton de pluie",
            Instrument::Rhombe => "rhombe",
            Instrument::Flute => "flûte d'os",
            Instrument::Conque => "conque",
            Instrument::Arc => "arc musical",
            Instrument::Lithophone => "pierres sonnantes",
            Instrument::Guimbarde => "guimbarde",
            Instrument::Lyre => "lyre",
            Instrument::Trompe => "trompe de métal",
            Instrument::Gong => "gong",
            Instrument::Orgue => "orgue",
            Instrument::Vielle => "vièle à archet",
            Instrument::Piano => "piano",
            Instrument::Guitare => "guitare électrique",
            Instrument::Synthe => "synthétiseur",
        }
    }
    pub fn melodique(self) -> bool {
        Self::MELODIQUES.contains(&self) || (self.de_technique() && self != Instrument::Gong)
    }
    pub fn de_technique(self) -> bool {
        INSTRUMENTS_DES_TECHNIQUES.iter().any(|&(i, _)| i == self)
    }
}

/// Tradition musicale d'un peuple, tirée de sa clé : la même pour tous ses
/// rites.
#[derive(Clone, Debug, PartialEq)]
pub struct Tradition {
    pub gamme: &'static [i32],
    pub temps: u32,
    pub polyphonie: Polyphonie,
    pub tonique: i32,
    /// Ses instruments : aucun à deux mélodiques, deux à quatre frappés.
    pub instruments: Vec<Instrument>,
    /// Ses deux manières de célébrer préférées.
    pub formes: [Forme; 2],
}

impl Tradition {
    pub fn de(cle: u64) -> Self {
        let mut r = Rng::new(cle ^ 0x7472_6164);
        let gamme = GAMMES[(r.u() * GAMMES.len() as f32) as usize % GAMMES.len()];
        let temps = r.pick(&[2, 3, 4, 4, 3, 5]);
        let polyphonie = r.pick(&[Polyphonie::Unisson, Polyphonie::Heterophonie, Polyphonie::Bourdon, Polyphonie::Quartes]);
        let tonique = 45 + (r.u() * 7.0) as i32;
        let mut instruments = Vec::new();
        let melodiques = match (r.u() * 4.0) as u32 {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        };
        while instruments.len() < melodiques {
            let i = r.pick(&Instrument::MELODIQUES);
            if !instruments.contains(&i) {
                instruments.push(i);
            }
        }
        if r.chance(0.7) {
            instruments.push(Instrument::Tambour);
        }
        let frappes = melodiques + 2 + (r.u() * 3.0) as usize;
        while instruments.len() < frappes {
            let i = r.pick(&Instrument::FRAPPES);
            if !instruments.contains(&i) {
                instruments.push(i);
            }
        }
        let a = r.pick(&Forme::TOUTES);
        let mut b = r.pick(&Forme::TOUTES);
        while b == a {
            b = r.pick(&Forme::TOUTES);
        }
        Self { gamme, temps, polyphonie, tonique, instruments, formes: [a, b] }
    }

    pub fn a(&self, i: Instrument) -> bool {
        self.instruments.contains(&i)
    }

    /// Les instruments que permettent les techniques du peuple : le plus
    /// récent, parfois un plus ancien, le gong de temps en temps. Ils
    /// prennent la place des instruments mélodiques d'avant, dont un seul
    /// reste (la flûte d'os d'un peuple se joue encore à côté du piano).
    pub fn avec_techniques(mut self, techniques: &[String], cle: u64) -> Self {
        let possibles: Vec<Instrument> = INSTRUMENTS_DES_TECHNIQUES
            .iter()
            .filter(|(_, cles)| cles.iter().any(|c| techniques.iter().any(|t| t == c)))
            .map(|&(i, _)| i)
            .collect();
        if possibles.is_empty() {
            return self;
        }
        let mut r = Rng::new(cle ^ 0x7465_6368);
        let melodiques: Vec<Instrument> = possibles.iter().copied().filter(|i| i.melodique()).collect();
        let mut neufs = Vec::new();
        if let Some(&dernier) = melodiques.last() {
            neufs.push(dernier);
            if melodiques.len() > 1 && r.chance(0.6) {
                let autre = melodiques[(r.u() * (melodiques.len() - 1) as f32) as usize % (melodiques.len() - 1)];
                neufs.push(autre);
            }
        }
        if possibles.contains(&Instrument::Gong) && r.chance(0.5) {
            neufs.push(Instrument::Gong);
        }
        let mut garde_ancien = true;
        self.instruments.retain(|i| {
            if !i.melodique() {
                return true;
            }
            std::mem::replace(&mut garde_ancien, false)
        });
        self.instruments.extend(neufs);
        self
    }

    /// Manière de célébrer un rite : celles du peuple, sauf que les morts
    /// appellent souvent la lamentation et la chasse le chant scandé.
    pub fn forme(&self, car: &Caractere, rng: &mut Rng) -> Forme {
        if car.tempo < 66.0 && rng.chance(0.5) {
            Forme::Lamentation
        } else if car.frappe >= 1.0 && rng.chance(0.4) {
            Forme::Scande
        } else {
            self.formes[(rng.u() * 2.0) as usize % 2]
        }
    }
}

/// Une frappe ou une note d'instrument.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Coup {
    Tambour(f32),
    TambourEau(f32),
    Fendu(f32),
    Hochet,
    Mains,
    Baton,
    /// Grain de bruit aigu (bâton de pluie, racleur) : fréquence, volume.
    Grain(f32, f32),
    Souffle {
        midi: f32,
        dur: f32,
        rauque: f32,
    },
    Arc(f32),
    Pierre(f32),
    /// Note d'un instrument des techniques : instrument, note MIDI, durée.
    Note(Instrument, f32, f32),
    Gong(f32),
}

/// Un chant rituel en cours : les voix chantées sont lancées d'un coup, les
/// instruments au fil du temps.
pub struct Chant {
    pub t: f32,
    pub fin: f32,
    pub forme: Forme,
    coups: Vec<(f32, Coup)>,
    suivant: usize,
}

/// Silence dans une ligne de notes.
const SILENCE: i32 = i32::MIN;

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

/// Notes en temps → notes en secondes.
fn en_secondes(notes: &[(f32, i32)], beat: f32) -> Vec<(f32, i32)> {
    notes.iter().map(|&(d, g)| (d * beat, g)).collect()
}

fn duree(notes: &[(f32, i32)]) -> f32 {
    notes.iter().map(|n| n.0).sum()
}

/// Une ligne à chanter : qui, quelles phrases (début en secondes, notes en
/// secondes), transposée de combien.
struct Ligne {
    loc: Locuteur,
    phrases: Vec<(f32, Vec<(f32, i32)>)>,
    decal: i32,
    octave: i32,
    orne: bool,
    gain: f32,
}

impl Chant {
    /// Compose et lance un chant : renvoie le chant et ses voix chantées.
    pub fn composer(peuple: &Peuple, rite: usize, rng: &mut Rng, mk: &Make) -> (Chant, Vec<Voice>) {
        let rate = mk.rate;
        let tr = Tradition::de(peuple.cle).avec_techniques(&peuple.techniques, peuple.cle);
        let r = peuple.rites.get(rite).cloned().unwrap_or(Rite { nom: String::new(), objet: String::new() });
        let car = caractere(&r.objet);
        let graine = r.nom.bytes().fold(peuple.cle, |h, b| h.wrapping_mul(0x100_0000_01b3) ^ b as u64);
        let mut cr = Rng::new(graine);
        let forme = tr.forme(&car, &mut cr);
        let tempo = car.tempo * cr.range(0.92, 1.08);
        let beat = 60.0 / tempo;
        let mesure = tr.temps as f32 * beat;
        let mesures = if tr.temps >= 4 { 2 } else { 3 };
        let appel = melodie(&mut cr, tr.temps, mesures, car.vivacite, 4);
        let reponse = melodie(&mut cr, tr.temps, mesures, car.vivacite, 2);
        let racine = tr.tonique + car.registre;
        let melodiques: Vec<Instrument> = tr.instruments.iter().copied().filter(|i| i.melodique()).collect();

        // Texte : le mot du rite, puis des mots de la langue.
        let mut texte: Vec<Vec<Ph>> = Vec::new();
        if let Some(m) = r.mot() {
            texte.extend(syllabes(&m));
        }
        let reserve = peuple.langue.syllabes();
        while texte.len() < 12 {
            texte.extend(peuple.langue.babil(&mut cr, &reserve));
        }

        // Les chanteurs : un meneur, un chœur plus nombreux quand le peuple
        // est grand (une femme sur deux, à l'octave).
        let meneur = Locuteur::homme(&mut cr);
        let n_choeur = (3.0 + peuple.population.max(1.0).ln() * 0.6 * (1.0 - car.douceur * 0.5)).clamp(3.0, 9.0) as usize;
        let choeur: Vec<(Locuteur, i32)> =
            (0..n_choeur).map(|c| if c % 2 == 1 { (Locuteur::femme(&mut cr), 12) } else { (Locuteur::homme(&mut cr), 0) }).collect();
        let g_choeur = 0.38 / (n_choeur as f32).sqrt();

        // Une mélodie jouée seule ouvre le chant quand le peuple a une flûte
        // ou des pierres sonnantes (sauf pour le chant scandé, qui part d'un
        // coup).
        let solo = tr.a(Instrument::Flute) || tr.a(Instrument::Lithophone) || melodiques.iter().any(|i| i.de_technique());
        let intro = if solo && forme != Forme::Scande { duree(&appel) * beat + beat } else { 0.0 };

        let mut lignes: Vec<Ligne> = Vec::new();
        // Bourdons fredonnés : (début, fin, degré, octave, locuteur).
        let mut bourdons: Vec<(f32, f32, i32, Locuteur, i32)> = Vec::new();
        // Les temps, pour les percussions : (instant, rang dans la mesure,
        // intensité de 0 à 1).
        let mut grille: Vec<(f32, u32, f32)> = Vec::new();
        let grille_reguliere = |de: f32, a: f32, beat: f32, temps: u32, g: &mut Vec<(f32, u32, f32)>| {
            let mut t = de;
            let mut n = 0;
            while t < a - beat * 0.5 {
                g.push((t, n % temps, 1.0));
                t += beat;
                n += 1;
            }
        };
        // Phrase de l'instrument (début, notes en secondes), doublée à
        // l'octave par la flûte ou les pierres.
        let mut jeu: Vec<(f32, Vec<(f32, i32)>)> = Vec::new();
        if intro > 0.0 {
            jeu.push((0.0, en_secondes(&appel, beat)));
        }

        let fin = match forme {
            Forme::AppelReponse => {
                let tours = 4 + (cr.u() * 3.0) as usize;
                let (a, rp) = (en_secondes(&appel, beat), en_secondes(&reponse, beat));
                let (la, lr) = (duree(&a), duree(&rp));
                let cycle = la + lr;
                lignes.push(Ligne {
                    loc: meneur,
                    phrases: (0..tours).map(|k| (intro + k as f32 * cycle, a.clone())).collect(),
                    decal: 0,
                    octave: 0,
                    orne: true,
                    gain: 0.5,
                });
                for (c, &(loc, oct)) in choeur.iter().enumerate() {
                    let phrases: Vec<_> = (0..tours).map(|k| (intro + k as f32 * cycle + la, rp.clone())).collect();
                    match tr.polyphonie {
                        Polyphonie::Bourdon if c < 2 => {
                            for k in 0..tours {
                                let d = intro + k as f32 * cycle + la;
                                bourdons.push((d, d + lr, 0, loc, oct - 12 * (c == 0) as i32));
                            }
                        }
                        Polyphonie::Quartes if c % 3 == 2 => {
                            lignes.push(Ligne { loc, phrases, decal: 2, octave: oct - 12, orne: false, gain: g_choeur })
                        }
                        p => {
                            lignes.push(Ligne { loc, phrases, decal: 0, octave: oct, orne: p == Polyphonie::Heterophonie, gain: g_choeur })
                        }
                    }
                }
                if solo {
                    for k in (1..tours).step_by(2) {
                        jeu.push((intro + k as f32 * cycle, a.clone()));
                    }
                }
                let fin = intro + tours as f32 * cycle;
                grille_reguliere(intro + la, fin, beat, tr.temps, &mut grille);
                fin
            }
            Forme::Canon => {
                let mut m = en_secondes(&appel, beat);
                m.extend(en_secondes(&reponse, beat));
                let l = duree(&m);
                let reprises = 3 + (cr.u() * 2.0) as usize;
                let groupes = 3;
                let entree = |g: usize| intro + g as f32 * mesure * if mesures == 2 { 1.0 } else { 1.5 };
                let phrases = |g: usize| (0..reprises).map(|k| (entree(g) + k as f32 * l, m.clone())).collect::<Vec<_>>();
                lignes.push(Ligne { loc: meneur, phrases: phrases(0), decal: 0, octave: 0, orne: false, gain: 0.4 });
                for (c, &(loc, oct)) in choeur.iter().enumerate() {
                    lignes.push(Ligne { loc, phrases: phrases(c % groupes), decal: 0, octave: oct, orne: false, gain: g_choeur });
                }
                if solo {
                    jeu.push((entree(0), m.clone()));
                }
                let fin = entree(groupes - 1) + reprises as f32 * l;
                grille_reguliere(entree(1), fin, beat, tr.temps, &mut grille);
                fin
            }
            Forme::Lamentation => {
                // Lente, ornée, chaque phrase suivie d'un silence.
                let a = en_secondes(&appel, beat * 1.6);
                let rp = en_secondes(&reponse, beat * 1.6);
                let tours = 3 + (cr.u() * 2.0) as usize;
                let souffle = 2.0 * beat;
                let mut t = intro;
                let mut phrases = Vec::new();
                for k in 0..tours {
                    let p = if k % 2 == 0 { a.clone() } else { rp.clone() };
                    let d = duree(&p);
                    phrases.push((t, p));
                    t += d + souffle;
                }
                let fin = t;
                lignes.push(Ligne { loc: meneur, phrases: phrases.clone(), decal: 0, octave: 0, orne: true, gain: 0.55 });
                // Le chœur fredonne la tonique et la quinte, bouche fermée puis ouverte.
                let quinte = tr.gamme.iter().position(|&x| x == 7).unwrap_or(2) as i32;
                for (c, &(loc, oct)) in choeur.iter().enumerate() {
                    bourdons.push((intro * 0.5, fin, if c % 3 == 2 { quinte } else { 0 }, loc, oct - 12));
                }
                if solo {
                    // L'instrument répond à chaque phrase, dans le silence.
                    for (k, (d, p)) in phrases.iter().enumerate() {
                        if k % 2 == 1 {
                            jeu.push((d + duree(p) - beat, en_secondes(&reponse[reponse.len().saturating_sub(3)..], beat)));
                        }
                    }
                }
                // Un seul coup sourd par mesure lente.
                let mut tt = intro;
                while tt < fin {
                    grille.push((tt, 0, 0.6));
                    tt += mesure * 1.6;
                }
                fin
            }
            Forme::Scande => {
                // Croches martelées sur deux notes, de plus en plus serrées.
                let tours = 6 + (cr.u() * 3.0) as usize;
                let croches = (tr.temps * 4) as usize;
                let motif: Vec<i32> = (0..croches).map(|i| if i % (tr.temps as usize * 2) == 0 { 1 } else { 0 }).collect();
                let mut t = 0.0;
                let mut b = beat;
                let mut phrases = Vec::new();
                for k in 0..tours {
                    let mut p: Vec<(f32, i32)> = motif.iter().map(|&g| (b * 0.5, g)).collect();
                    // Un cri tenu à la fin de chaque deuxième tour.
                    if k % 2 == 1 {
                        p.truncate(croches - 2);
                        p.push((b, 2));
                    }
                    let n_temps = tr.temps * 2;
                    for i in 0..n_temps {
                        grille.push((t + i as f32 * b, i % tr.temps, 0.7 + 0.3 * k as f32 / tours as f32));
                    }
                    let d = duree(&p);
                    phrases.push((t, p));
                    t += d;
                    b *= 0.975;
                }
                lignes.push(Ligne { loc: meneur, phrases: phrases.clone(), decal: 0, octave: 0, orne: false, gain: 0.4 });
                for &(loc, oct) in &choeur {
                    lignes.push(Ligne { loc, phrases: phrases.clone(), decal: 0, octave: oct, orne: false, gain: g_choeur * 1.1 });
                }
                t
            }
            Forme::Hoquet => {
                // Un motif de huit croches, partagé note à note entre les voix.
                let motif: Vec<i32> = (0..8).map(|_| (cr.u() * 5.0) as i32).collect();
                let reprises = 12 + (cr.u() * 6.0) as usize;
                let l = 8.0 * beat * 0.5;
                for (c, &(loc, oct)) in choeur.iter().enumerate() {
                    let notes: Vec<(f32, i32)> = motif
                        .iter()
                        .enumerate()
                        .map(|(i, &g)| (beat * 0.5, if i % n_choeur.min(4) == c % 4 { g } else { SILENCE }))
                        .collect();
                    let phrases = (0..reprises).map(|k| (intro + k as f32 * l, notes.clone())).collect();
                    lignes.push(Ligne { loc, phrases, decal: 0, octave: oct, orne: false, gain: g_choeur * 1.3 });
                }
                // Le meneur chante l'appel par-dessus, un tour sur deux.
                let a = en_secondes(&appel, beat);
                let la = duree(&a);
                let fin = intro + reprises as f32 * l;
                let mut t = intro + 2.0 * l;
                let mut phrases = Vec::new();
                while t + la < fin {
                    phrases.push((t, a.clone()));
                    t += la * 2.0;
                }
                lignes.push(Ligne { loc: meneur, phrases, decal: 2, octave: 0, orne: true, gain: 0.4 });
                grille_reguliere(intro, fin, beat, tr.temps, &mut grille);
                fin
            }
            Forme::Transe => {
                // Le même motif en boucle, du lent au très vif.
                let reprises = 9 + (cr.u() * 4.0) as usize;
                let mut t = intro;
                let mut phrases = Vec::new();
                for k in 0..reprises {
                    let x = k as f32 / (reprises - 1) as f32;
                    let b = beat * (1.12 - 0.4 * x);
                    let p = en_secondes(&reponse, b);
                    for i in 0..(duree(&reponse) as u32) {
                        grille.push((t + i as f32 * b, i % tr.temps, 0.35 + 0.65 * x));
                    }
                    let d = duree(&p);
                    phrases.push((t, p));
                    t += d;
                }
                lignes.push(Ligne { loc: meneur, phrases: phrases.clone(), decal: 0, octave: 0, orne: false, gain: 0.4 });
                for &(loc, oct) in &choeur {
                    lignes.push(Ligne { loc, phrases: phrases.clone(), decal: 0, octave: oct, orne: false, gain: g_choeur });
                }
                t
            }
        };

        // Les voix chantées.
        let mut voix = Vec::new();
        let unisson = tr.polyphonie == Polyphonie::Unisson || matches!(forme, Forme::Scande | Forme::Hoquet);
        let note = |deg: i32, octave: i32, decal: i32, rng: &mut Rng| {
            mtof(crate::synth::degree(racine + octave, tr.gamme, deg + decal) + rng.range(-0.08, 0.08))
        };
        for ligne in &lignes {
            let retard = if unisson { cr.range(0.0, 0.02) } else { cr.range(0.0, 0.05) };
            let mut segs = vec![Seg { ph: Ph::Pause, dur: retard.max(0.001), f0: ligne.loc.f0, amp: 0.0 }];
            let mut t = retard;
            let mut s = 0;
            for (debut, notes) in &ligne.phrases {
                if *debut + retard > t {
                    segs.push(Seg { ph: Ph::Pause, dur: *debut + retard - t, f0: ligne.loc.f0, amp: 0.0 });
                    t = *debut + retard;
                }
                for &(dur, deg) in notes {
                    t += dur;
                    if deg == SILENCE {
                        segs.push(Seg { ph: Ph::Pause, dur, f0: ligne.loc.f0, amp: 0.0 });
                        continue;
                    }
                    let f0 = note(deg, ligne.octave, ligne.decal, &mut cr);
                    let syl = &texte[s % texte.len()];
                    s += 1;
                    let cons = (syl.iter().filter(|p| !p.voyelle()).count() as f32 * 0.05).min(dur * 0.5);
                    let n_cons = syl.iter().filter(|p| !p.voyelle()).count().max(1) as f32;
                    let ornement = ligne.orne && dur > 0.4 && cr.chance(0.3);
                    for &p in syl {
                        if p.voyelle() {
                            let v = (dur - cons).max(0.04);
                            if ornement {
                                let haut = note(deg + 1, ligne.octave, ligne.decal, &mut cr);
                                segs.push(Seg { ph: p, dur: v * 0.25, f0: haut, amp: 1.0 });
                                segs.push(Seg { ph: p, dur: v * 0.75, f0, amp: 1.0 });
                            } else {
                                segs.push(Seg { ph: p, dur: v, f0, amp: 1.0 });
                            }
                        } else {
                            segs.push(Seg { ph: p, dur: cons / n_cons, f0, amp: 0.9 });
                        }
                    }
                }
            }
            let pan = if ligne.gain >= 0.4 { 0.0 } else { cr.range(-0.6, 0.6) };
            let vib = if ligne.gain >= 0.4 { 0.35 } else { 0.25 };
            voix.push(
                Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, ligne.loc, vib, rate))), Bus::Ambiance, ligne.gain)
                    .pan(pan)
                    .send(0.4),
            );
        }
        for &(de, a, deg, loc, oct) in &bourdons {
            let f0 = note(deg, oct, 0, &mut cr);
            let segs = vec![
                Seg { ph: Ph::Pause, dur: de.max(0.001), f0, amp: 0.0 },
                Seg { ph: son("m").unwrap_or(Ph::Pause), dur: ((a - de) * 0.4).max(0.05), f0, amp: 0.6 },
                Seg { ph: son(if deg == 0 { "o" } else { "ou" }).unwrap_or(Ph::Pause), dur: ((a - de) * 0.6).max(0.05), f0, amp: 0.6 },
            ];
            voix.push(
                Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, loc, 0.1, rate))), Bus::Ambiance, g_choeur * 0.7)
                    .pan(cr.range(-0.5, 0.5))
                    .send(0.5),
            );
        }

        // Instruments au long cours : rhombe et guimbarde.
        if tr.a(Instrument::Rhombe) {
            voix.push(Voice::new(mk.rhombe(cr.range(90.0, 150.0), cr.range(0.9, 1.6), fin + 2.0), Bus::Ambiance, 0.06).pan(-0.4).send(0.5));
        }
        if tr.a(Instrument::Guimbarde) {
            // Une voix sans souffle sur une note grave fixe, dont la bouche
            // change de voyelle à chaque croche : le nasillement de la guimbarde.
            let f0 = mtof((racine - 12) as f32);
            let loc = Locuteur { f0, conduit: 1.0, souffle: 0.0 };
            let voyelles = ["i", "ou", "a", "é", "o"];
            let mut segs = vec![Seg { ph: Ph::Pause, dur: intro.max(0.001), f0, amp: 0.0 }];
            let mut t = intro;
            let mut k = 0;
            while t < fin {
                let v = son(voyelles[(cr.u() * voyelles.len() as f32) as usize % voyelles.len()]).unwrap_or(Ph::Pause);
                segs.push(Seg { ph: v, dur: beat * 0.5 * if k % 4 == 3 { 0.6 } else { 1.0 }, f0, amp: 1.0 });
                if k % 4 == 3 {
                    segs.push(Seg { ph: Ph::Pause, dur: beat * 0.2, f0, amp: 0.0 });
                }
                t += beat * 0.5;
                k += 1;
            }
            voix.push(
                Voice::new(crate::synth::Kind::Gosier(Box::new(Gosier::new(segs, loc, 0.0, rate))), Bus::Ambiance, 0.18).pan(0.4).send(0.2),
            );
        }

        // Percussions et notes des instruments, d'après la grille des temps.
        let mut coups = Vec::new();
        let frappe = car.frappe;
        for (i, &(t, n, x)) in grille.iter().enumerate() {
            let premier = n == 0;
            let f = frappe * x;
            if tr.a(Instrument::Tambour) && (premier || f > 0.7 || (f > 0.4 && n % 2 == 0)) {
                coups.push((t, Coup::Tambour(if premier { 1.0 } else { 0.6 } * (0.4 + 0.6 * f))));
            }
            if tr.a(Instrument::TambourEau) && n % 2 == 0 {
                coups.push((t, Coup::TambourEau(0.5 + 0.5 * f)));
            }
            if tr.a(Instrument::Fendu) && f > 0.2 {
                // Deux notes de bois, grave sur le temps, aiguë en contretemps.
                coups.push((t, Coup::Fendu((racine + 12) as f32)));
                if f > 0.5 || i % 2 == 1 {
                    let dt = grille.get(i + 1).map_or(beat, |g| g.0 - t);
                    coups.push((t + dt * 0.5, Coup::Fendu((racine + 17) as f32)));
                }
            }
            if tr.a(Instrument::Hochet) && f > 0.15 {
                let dt = grille.get(i + 1).map_or(beat, |g| g.0 - t);
                coups.push((t + dt * 0.5, Coup::Hochet));
                if f > 0.5 {
                    coups.push((t + dt * 0.25, Coup::Hochet));
                    coups.push((t + dt * 0.75, Coup::Hochet));
                }
            }
            if tr.a(Instrument::Mains) && f > 0.45 && !premier {
                coups.push((t, Coup::Mains));
            }
            if tr.a(Instrument::Batons) && f > 0.25 && n + 1 == tr.temps {
                let dt = grille.get(i + 1).map_or(beat, |g| g.0 - t);
                coups.push((t + dt * 0.5, Coup::Baton));
            }
            if tr.a(Instrument::Racleur) && f > 0.3 && n % 2 == 1 {
                for g in 0..6 {
                    coups.push((t + g as f32 * 0.035, Coup::Grain(cr.range(2500.0, 3500.0), 0.12)));
                }
            }
            if tr.a(Instrument::Arc) && f > 0.1 {
                let d = if n == 0 {
                    0
                } else if n % 2 == 1 {
                    1
                } else {
                    0
                };
                coups.push((t, Coup::Arc(crate::synth::degree(racine, tr.gamme, d))));
            }
        }
        if tr.a(Instrument::BatonPluie) {
            for (de, a) in [(0.0, 3.0), (fin - 1.0, fin + 2.0)] {
                for _ in 0..50 {
                    coups.push((cr.range(de, a), Coup::Grain(cr.range(5000.0, 9000.0), cr.range(0.02, 0.07))));
                }
            }
        }
        if tr.a(Instrument::Conque) {
            // Longs appels graves au début et à la fin.
            for t in [0.0, fin - 1.0] {
                coups.push((t, Coup::Souffle { midi: (racine - 12) as f32, dur: 2.2, rauque: 1.0 }));
            }
        }
        for &inst in &melodiques {
            if matches!(inst, Instrument::Conque | Instrument::Arc | Instrument::Guimbarde) {
                continue;
            }
            for (debut, notes) in &jeu {
                let mut t = *debut;
                for &(dur, deg) in notes {
                    if deg != SILENCE {
                        let m = crate::synth::degree(racine + 12, tr.gamme, deg);
                        coups.push((
                            t,
                            match inst {
                                Instrument::Flute => Coup::Souffle { midi: m + 12.0, dur: dur * 0.95, rauque: 0.1 },
                                Instrument::Lithophone => Coup::Pierre(m + 12.0),
                                Instrument::Guitare => Coup::Note(inst, m - 12.0, dur),
                                Instrument::Piano | Instrument::Lyre => Coup::Note(inst, m + 12.0, dur),
                                _ => Coup::Note(inst, m, dur),
                            },
                        ));
                    }
                    t += dur;
                }
            }
        }
        if tr.a(Instrument::Gong) {
            // Le gong ouvre chaque phrase jouée, et le chant.
            coups.push((0.0, Coup::Gong((racine - 24) as f32)));
            for (debut, _) in &jeu {
                coups.push((*debut, Coup::Gong((racine - 24) as f32)));
            }
        }
        coups.sort_by(|a, b| a.0.total_cmp(&b.0));
        let _ = rng;
        (Chant { t: 0.0, fin: fin + 2.5, forme, coups, suivant: 0 }, voix)
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
                Coup::TambourEau(g) => Voice::new(mk.thump(rng.range(58.0, 64.0), 0.6), Bus::Ambiance, 0.35 * g).pan(0.2).send(0.45),
                Coup::Fendu(m) => Voice::new(mk.bell(m, 1.5, 0.7, 0.3), Bus::Ambiance, 0.16).pan(-0.2).send(0.3),
                Coup::Hochet => Voice::new(mk.burst(FilterKind::HighPass, 4500.0, 4500.0, 0.8, 0.004, 0.07, false), Bus::Ambiance, 0.09)
                    .pan(0.35)
                    .send(0.2),
                Coup::Mains => Voice::new(mk.burst(FilterKind::BandPass, 1400.0, 1100.0, 1.1, 0.001, 0.05, false), Bus::Ambiance, 0.32)
                    .pan(rng.range(-0.4, 0.4))
                    .send(0.35),
                Coup::Baton => Voice::new(mk.bell(rng.range(82.0, 84.0), 2.76, 0.6, 0.12), Bus::Ambiance, 0.12).pan(-0.3).send(0.2),
                Coup::Grain(f, g) => Voice::new(mk.burst(FilterKind::BandPass, f, f, 2.0, 0.001, 0.018, false), Bus::Ambiance, g)
                    .pan(rng.range(-0.5, 0.5))
                    .send(0.3),
                Coup::Souffle { midi, dur, rauque } => {
                    Voice::new(mk.flute(midi, dur, rauque), Bus::Ambiance, if rauque > 0.5 { 0.14 } else { 0.1 }).pan(0.3).send(0.5)
                }
                Coup::Arc(m) => Voice::new(mk.pluck(rng, m, 0.15, 1.2), Bus::Ambiance, 0.2).pan(0.25).send(0.25),
                Coup::Pierre(m) => Voice::new(mk.bell(m, 2.76, 1.0, 1.4), Bus::Ambiance, 0.08).pan(0.3).send(0.4),
                Coup::Note(i, m, d) => note_de_technique(i, m, d, mk, rng),
                Coup::Gong(m) => Voice::new(mk.bell(m, 1.41, 2.2, 6.0), Bus::Ambiance, 0.09).pan(-0.3).send(0.6),
            };
            out.push(v.at(at, mk.rate));
        }
        self.t = fin;
    }

    pub fn fini(&self) -> bool {
        self.t >= self.fin
    }
}

/// Timbre d'un instrument des techniques (ceux qui ne sont pas des cordes
/// pincées).
pub fn timbre(i: Instrument) -> Timbre {
    let scie = [1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.12];
    let base = Timbre { harm: scie, eclat: 0.0, att: 0.02, rel: 0.1, chute: 0.0, vib: (0.0, 0.0), sature: 0.0, raideur: 0.0 };
    match i {
        Instrument::Trompe => {
            Timbre { harm: [1.0, 0.8, 0.6, 0.5, 0.35, 0.25, 0.18, 0.12], eclat: 0.85, att: 0.07, vib: (5.0, 0.003), ..base }
        }
        Instrument::Vielle => Timbre { eclat: 0.2, att: 0.14, rel: 0.15, vib: (5.5, 0.006), ..base },
        Instrument::Orgue => Timbre { harm: [1.0, 0.7, 0.0, 0.45, 0.0, 0.3, 0.0, 0.2], att: 0.03, rel: 0.08, ..base },
        Instrument::Piano => {
            Timbre { harm: [1.0, 0.6, 0.35, 0.25, 0.15, 0.1, 0.07, 0.05], att: 0.003, rel: 0.25, chute: 2.5, raideur: 0.0004, ..base }
        }
        Instrument::Guitare => Timbre {
            harm: [1.0, 0.7, 0.5, 0.4, 0.3, 0.2, 0.15, 0.1],
            att: 0.004,
            rel: 0.15,
            chute: 3.0,
            vib: (5.0, 0.002),
            sature: 0.7,
            ..base
        },
        Instrument::Synthe => Timbre { eclat: 0.5, att: 0.15, rel: 0.4, vib: (0.4, 0.002), ..base },
        _ => base,
    }
}

/// Une note d'instrument des techniques.
pub fn note_de_technique(i: Instrument, m: f32, d: f32, mk: &Make, rng: &mut Rng) -> Voice {
    let (kind, g, pan) = match i {
        Instrument::Lyre => (mk.pluck(rng, m, 0.5, 1.6), 0.18, 0.25),
        Instrument::Trompe => (mk.additif(m, d * 0.9, timbre(i)), 0.06, -0.25),
        Instrument::Vielle => (mk.additif(m, d * 0.98, timbre(i)), 0.06, 0.3),
        Instrument::Orgue => (mk.additif(m, d * 0.98, timbre(i)), 0.05, 0.0),
        Instrument::Piano => (mk.additif(m, d.max(0.4), timbre(i)), 0.09, 0.2),
        Instrument::Guitare => (mk.additif(m, d.max(0.3), timbre(i)), 0.05, -0.3),
        Instrument::Synthe => (mk.additif(m, d, timbre(i)), 0.045, 0.1),
        _ => (mk.pluck(rng, m, 0.3, 1.0), 0.15, 0.0),
    };
    Voice::new(kind, Bus::Ambiance, g).pan(pan).send(0.35)
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
        // Sur quelques dizaines de peuples, tous les instruments et toutes
        // les manières de célébrer se rencontrent.
        let traditions: Vec<Tradition> = (0..60).map(Tradition::de).collect();
        for i in Instrument::MELODIQUES.iter().chain(&Instrument::FRAPPES) {
            assert!(traditions.iter().any(|t| t.a(*i)), "{}", i.nom());
        }
        for f in Forme::TOUTES {
            assert!(traditions.iter().any(|t| t.formes.contains(&f)), "{}", f.nom());
        }
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
            techniques: Vec::new(),
        };
        let (mut chant, voix) = Chant::composer(&peuple, 0, &mut rng, &mk);
        assert!(voix.len() >= 4, "meneur et chœur : {}", voix.len());
        assert!((10.0..90.0).contains(&chant.fin), "{}", chant.fin);
        let mut coups = Vec::new();
        while !chant.fini() {
            chant.tick(0.1, &mk, &mut rng, &mut coups);
        }
        assert!(coups.len() > 10);
        // Chaque manière de célébrer donne un chant d'une durée raisonnable.
        let mut vues = Vec::new();
        for cle in 0..40 {
            let p = Peuple { cle, ..peuple.clone() };
            let (c, v) = Chant::composer(&p, 0, &mut rng, &mk);
            assert!((8.0..120.0).contains(&c.fin), "{:?} : {}", c.forme, c.fin);
            assert!(v.len() >= 4);
            if !vues.contains(&c.forme) {
                vues.push(c.forme);
            }
        }
        assert!(vues.len() >= 5, "{vues:?}");
    }
}
