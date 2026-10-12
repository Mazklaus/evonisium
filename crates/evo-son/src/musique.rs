//! Musique générative des ères (choix C + B + A du doc « Design sonore ») :
//! l'ère donne la palette d'instruments (C), l'état publié de la planète
//! décide de ce qui est joué (B), et tout est joué par la même famille de
//! timbres acoustiques (A). Livraison 1 : les ères microbiennes.
//!
//! - plus de lignées vivantes, plus de petites phrases ;
//! - plus chaud, mode plus lumineux et registre plus haut ;
//! - plus d'oxygène, timbre plus clair (filtre du bus) ;
//! - de longs repos où ne reste que la nappe, pour ne jamais saturer une
//!   partie de 20 à 60 heures.

use crate::synth::{degree, Bus, Make, Rng, Smooth, Voice};
use crate::{Moment, Monde};

/// Modes, du plus sombre au plus lumineux ; l'ionien (majeur), en dernier,
/// ne sert qu'aux boucles du fond qui sont en majeur.
pub const MODES: [(&str, [i32; 7]); 6] = [
    ("phrygien", [0, 1, 3, 5, 7, 8, 10]),
    ("éolien", [0, 2, 3, 5, 7, 8, 10]),
    ("dorien", [0, 2, 3, 5, 7, 9, 10]),
    ("mixolydien", [0, 2, 4, 5, 7, 9, 10]),
    ("lydien", [0, 2, 4, 6, 7, 9, 11]),
    ("ionien", [0, 2, 4, 5, 7, 9, 11]),
];

/// Pas de la musique, en secondes.
pub const STEP: f32 = 0.2;

/// Timbre des petites phrases mélodiques (la kalimba du premier jet était
/// trop vive : l'utilisateur l'a refusée le 2026-10-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phrase {
    /// Flûte douce, sans souffle rauque, une octave sous la kalimba.
    Flute,
    /// Harpe feutrée : corde pincée très sombre, dans le grave.
    Harpe,
    /// Verre doux : notes qui enflent et s'éteignent lentement.
    Verre,
}

impl Phrase {
    pub const TOUTES: [Phrase; 3] = [Phrase::Flute, Phrase::Harpe, Phrase::Verre];
    pub fn nom(self) -> &'static str {
        match self {
            Phrase::Flute => "flûte douce",
            Phrase::Harpe => "harpe feutrée",
            Phrase::Verre => "verre doux",
        }
    }
}

pub struct Musique {
    step: u64,
    acc: f32,
    pub mode: usize,
    pub base: i32,
    chord: i32,
    /// Phrase en cours (true) ou repos (false), et temps restant.
    pub playing: bool,
    phase_left: f32,
    /// Présence des voix mélodiques : 1 pendant une phrase, ~0 au repos.
    pub presence: Smooth,
    /// Délai minimal entre deux motifs d'événement.
    cooldown: f32,
    pending: Vec<Moment>,
    /// Tonalité imposée par la boucle du fond qui joue (tonique MIDI, mode) :
    /// les couches générées s'y accordent et la nappe se tait.
    pub cle: Option<(i32, usize)>,
    /// Timbre des petites phrases.
    pub phrase: Phrase,
}

/// Valeurs de 0 à 1 que la musique lit dans le monde publié.
#[derive(Clone, Copy, Debug)]
pub struct Lecture {
    pub temp: f32,
    pub o2: f32,
    pub bio: f32,
    pub ice: f32,
}

impl Lecture {
    pub fn of(m: Option<&Monde>) -> Self {
        let Some(m) = m else { return Self { temp: 0.5, o2: 0.3, bio: 0.3, ice: 0.0 } };
        let temp = ((m.temperature_k - 255.0) / 65.0).clamp(0.0, 1.0) as f32;
        let o2 = ((m.o2.max(1e-7).log10() + 6.0) / (0.21f64.log10() + 6.0)).clamp(0.0, 1.0) as f32;
        let bio = ((1.0 + m.lineages as f64).log10() / 400f64.log10()).clamp(0.0, 1.0) as f32;
        Self { temp, o2, bio, ice: m.ice.clamp(0.0, 1.0) as f32 }
    }
    /// Fréquence de coupure du bus musique : l'oxygène éclaircit le timbre.
    pub fn cutoff(&self) -> f32 {
        350.0 * 2f32.powf(4.6 * self.o2)
    }
}

impl Musique {
    pub fn new(rng: &mut Rng) -> Self {
        Self {
            step: 0,
            acc: 0.0,
            mode: 2,
            base: 55,
            chord: 0,
            playing: true,
            phase_left: rng.range(50.0, 90.0),
            presence: Smooth::new(0.0),
            cooldown: 0.0,
            pending: Vec::new(),
            cle: None,
            phrase: Phrase::Verre,
        }
    }

    pub fn moment(&mut self, m: Moment) {
        if self.pending.len() < 8 {
            self.pending.push(m);
        }
    }

    /// Avance de `dt` secondes et ajoute les notes à jouer.
    pub fn tick(&mut self, dt: f32, l: &Lecture, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        self.phase_left -= dt;
        if self.phase_left <= 0.0 {
            self.playing = !self.playing;
            self.phase_left = if self.playing { rng.range(50.0, 90.0) } else { rng.range(25.0, 50.0) };
        }
        self.presence.target = if self.playing { 1.0 } else { 0.08 };
        self.presence.step(dt, 6.0);
        self.cooldown -= dt;
        self.acc += dt;
        while self.acc >= STEP {
            self.acc -= STEP;
            self.play_step(l, mk, rng, out);
            self.step += 1;
        }
    }

    fn play_step(&mut self, l: &Lecture, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        let r = mk.rate;
        let i = self.step;
        let p = self.presence.value;
        if i.is_multiple_of(32) {
            // Accord et nappe grave (violoncelle feutré) : toujours là, même au repos.
            match self.cle {
                // Même registre que d'habitude, dans la tonalité de la boucle.
                Some((tonique, mode)) => {
                    self.mode = mode.min(MODES.len() - 1);
                    self.base = 52 + (tonique - 52).rem_euclid(12);
                }
                None => {
                    self.mode = ((l.temp * 5.0) as usize).min(4);
                    self.base = 52 + (l.temp * 6.0).round() as i32;
                }
            }
            self.chord = if i == 0 {
                0
            } else if rng.chance(0.6) {
                rng.pick(&[0, 3, 4, 5])
            } else {
                rng.pick(&[0, 1, 2, 3, 4, 5, 6])
            };
            // La boucle du fond tient déjà le grave.
            if self.cle.is_none() {
                let sc = &MODES[self.mode].1;
                let notes = [degree(self.base - 12, sc, self.chord), degree(self.base - 12, sc, self.chord + 4)];
                out.push(Voice::new(mk.pad(rng, &notes, 6.6, 1.8, true, 600.0), Bus::Musique, 0.09).send(0.3));
            }
        }
        let sc = MODES[self.mode].1;
        // Bulles des sources chaudes : moins nombreuses quand l'oxygène monte.
        if rng.chance(0.3 * (1.0 - l.o2) * (0.3 + 0.7 * p)) {
            let f0 = rng.range(140.0, 320.0);
            out.push(
                Voice::new(mk.chirp(f0, f0 * rng.range(2.0, 3.0), rng.range(0.03, 0.07)), Bus::Musique, 0.05).pan(rng.bipolar() * 0.8),
            );
        }
        // Petite phrase douce, rare : quand la vie se diversifie.
        if i % 16 == 8 && rng.chance(0.3 * l.bio * p) {
            let mut d = self.chord + 7 + rng.pick(&[0, 2, 4]);
            let mut t = 0.0;
            for _ in 0..rng.pick(&[3, 4, 5]) {
                d += rng.pick(&[-2, -1, 1, 1, 2]);
                let pas = rng.pick(&[0.6, 0.8, 1.0]);
                let v = match self.phrase {
                    Phrase::Flute => Voice::new(mk.flute(degree(self.base, &sc, d), pas * 1.1, 0.0), Bus::Musique, 0.045).send(0.5),
                    Phrase::Harpe => Voice::new(mk.pluck(rng, degree(self.base - 12, &sc, d), 0.07, 2.6), Bus::Musique, 0.11).send(0.45),
                    Phrase::Verre => {
                        let k = mk.pad(rng, &[degree(self.base + 12, &sc, d)], pas * 1.8, pas * 0.6, true, 1400.0);
                        Voice::new(k, Bus::Musique, 0.035).send(0.6)
                    }
                };
                out.push(v.at(t, r).pan(rng.range(-0.4, 0.1)));
                t += pas;
            }
        }
        // Motifs des moments forts de la chronique.
        if self.cooldown <= 0.0 {
            if let Some(m) = self.pending.pop() {
                self.cooldown = 8.0;
                self.motif(m, mk, rng, out);
            }
        }
    }

    fn motif(&self, m: Moment, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        let r = mk.rate;
        let sc = MODES[self.mode].1;
        match m {
            // Les motifs enflent doucement, sans attaque de cloche : les
            // cloches de verre tintaient trop (refusées le 2026-10-12).
            Moment::Jalon => {
                for (j, d) in [0, 2, 4].into_iter().enumerate() {
                    let k = mk.pad(rng, &[degree(self.base + 12, &sc, self.chord + d)], 2.4, 0.5, true, 1600.0);
                    out.push(Voice::new(k, Bus::Musique, 0.04).at(j as f32 * 0.35, r).pan(rng.range(-0.4, 0.4)).send(0.6));
                }
            }
            Moment::Froid => {
                let notes = [degree(self.base + 12, &sc, self.chord), degree(self.base + 12, &sc, self.chord + 3)];
                out.push(Voice::new(mk.pad(rng, &notes, 4.0, 1.2, true, 900.0), Bus::Musique, 0.035).send(0.7));
            }
            Moment::Extinction => {
                out.push(Voice::new(mk.thump(46.0, 2.6), Bus::Musique, 0.5).send(0.3));
                let k = mk.pad(rng, &[degree(self.base - 12, &sc, self.chord)], 5.0, 0.8, true, 500.0);
                out.push(Voice::new(k, Bus::Musique, 0.07).send(0.4));
            }
        }
    }
}
