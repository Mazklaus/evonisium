//! Ambiances des vues : souffle du globe (vent, grondement, orages
//! lointains réglés par le climat publié) et monde liquide de la loupe
//! (bulles, cliquetis des flagelles, battement lent des divisions).

use crate::musique::Lecture;
use crate::synth::{Bed, Bus, FilterKind, Make, Rng, Voice};
use std::f32::consts::TAU;

pub struct Globe {
    pub wind: Bed,
    pub hiss: Bed,
    rumble_phase: f32,
    pub level: f32,
    thunder_acc: f32,
}

impl Globe {
    pub fn new(rate: f32) -> Self {
        Self {
            wind: Bed::new(true, FilterKind::BandPass, 350.0, 0.8, rate).breathe_freq(0.07, 160.0).breathe_gain(0.11, 0.4),
            hiss: Bed::new(false, FilterKind::BandPass, 1800.0, 0.6, rate).breathe_gain(0.09, 0.6),
            rumble_phase: 0.0,
            level: 0.0,
            thunder_acc: 0.0,
        }
    }
    /// Agitation du climat, de 0 à 1 : chaleur et océan libre de glace.
    pub fn storminess(l: &Lecture) -> f32 {
        (0.2 + 0.6 * l.temp * (1.0 - l.ice)).clamp(0.0, 1.0)
    }
    pub fn set(&mut self, l: &Lecture, level: f32) {
        let c = Self::storminess(l);
        self.level = level;
        self.wind.gain.target = (0.12 + 0.3 * c) * level;
        self.wind.freq.target = 240.0 + 520.0 * c;
        self.hiss.gain.target = (0.004 + 0.02 * c) * level;
    }
    #[inline]
    pub fn run(&mut self, rng: &mut Rng, dt: f32, rate: f32) -> f32 {
        self.rumble_phase = (self.rumble_phase + 38.0 * dt).fract();
        let rumble = (TAU * self.rumble_phase).sin() * 0.04 * self.wind.gain.value * 3.0;
        self.wind.run(rng, dt, rate) + self.hiss.run(rng, dt, rate) + rumble
    }
    /// Orages lointains, tirés une fois par seconde.
    pub fn tick(&mut self, dt: f32, l: &Lecture, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        self.thunder_acc += dt;
        while self.thunder_acc >= 1.0 {
            self.thunder_acc -= 1.0;
            if self.level > 0.05 && rng.chance(0.004 + 0.05 * Self::storminess(l)) {
                let v = rng.range(0.3, 0.6) * self.level;
                let pan = rng.bipolar() * 0.7;
                out.push(
                    Voice::new(mk.burst(FilterKind::HighPass, 2500.0, 2500.0, 0.7, 0.002, 0.25, false), Bus::Ambiance, v * 0.12).pan(pan),
                );
                let k = mk.burst(FilterKind::LowPass, rng.range(120.0, 220.0), 90.0, 0.8, 0.15, rng.range(2.5, 4.5), true);
                out.push(Voice::new(k, Bus::Ambiance, v).at(0.08, mk.rate).pan(pan).send(0.4));
            }
        }
    }
}

pub struct Micro {
    pub liquid: Bed,
    pub level: f32,
    acc: f32,
    beat: f32,
}

impl Micro {
    pub fn new(rate: f32) -> Self {
        Self { liquid: Bed::new(true, FilterKind::LowPass, 260.0, 0.7, rate).breathe_freq(0.08, 80.0), level: 0.0, acc: 0.0, beat: 0.0 }
    }
    pub fn set(&mut self, level: f32) {
        self.level = level;
        self.liquid.gain.target = 0.22 * level;
    }
    #[inline]
    pub fn run(&mut self, rng: &mut Rng, dt: f32, rate: f32) -> f32 {
        self.liquid.run(rng, dt, rate)
    }
    /// Bulles, flagelles et divisions ; leur nombre suit la vie publiée.
    pub fn tick(&mut self, dt: f32, l: &Lecture, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        if self.level < 0.05 {
            return;
        }
        let r = mk.rate;
        let pop = (0.2 + 0.8 * l.bio).min(1.0);
        self.acc += dt;
        while self.acc >= 0.06 {
            self.acc -= 0.06;
            if rng.chance(0.03 + 0.25 * pop) {
                let f0 = rng.range(350.0, 1100.0);
                let k = mk.chirp(f0, f0 * rng.range(2.0, 3.0), rng.range(0.03, 0.07));
                out.push(Voice::new(k, Bus::Ambiance, rng.range(0.03, 0.09) * self.level).pan(rng.bipolar() * 0.8).send(0.2));
            }
            if rng.chance(0.03 * pop) {
                let pan = rng.bipolar() * 0.8;
                let f = rng.range(2500.0, 5000.0);
                let mut t = 0.0;
                for _ in 0..(3 + (rng.u() * 7.0) as usize) {
                    let k = mk.burst(FilterKind::BandPass, f, f, 3.0, 0.0005, 0.006, false);
                    out.push(Voice::new(k, Bus::Ambiance, 0.35 * self.level).at(t, r).pan(pan).send(0.1));
                    t += rng.range(0.01, 0.02);
                }
            }
        }
        self.beat += dt;
        if self.beat >= 3.6 {
            self.beat = 0.0;
            out.push(Voice::new(mk.thump(58.0, 0.5), Bus::Ambiance, 0.22 * self.level));
            out.push(Voice::new(mk.thump(52.0, 0.6), Bus::Ambiance, 0.16 * self.level).at(0.32, r));
        }
    }
}
