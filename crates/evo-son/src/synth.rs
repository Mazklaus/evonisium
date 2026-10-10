//! Briques de synthèse : hasard local, filtres, voix ponctuelles (cordes
//! pincées, cloches, nappes, bulles, souffles, coups sourds), fonds continus
//! et réverbération. Tout est en `f32` et calculé échantillon par
//! échantillon, sans allocation hors de la création d'une voix.

use std::f32::consts::TAU;

/// Hasard propre au son (xorshift64*). Il ne touche jamais au générateur du
/// moteur : le son n'entre pas dans l'histoire.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniforme dans [0, 1).
    pub fn u(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.u()
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.u() < p
    }
    pub fn pick<T: Copy>(&mut self, s: &[T]) -> T {
        s[(self.u() * s.len() as f32) as usize % s.len()]
    }
    /// Uniforme dans [-1, 1).
    pub fn bipolar(&mut self) -> f32 {
        self.u() * 2.0 - 1.0
    }
}

/// Fréquence d'une note MIDI.
pub fn mtof(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// Degré `d` d'une gamme (intervalles en demi-tons) au-dessus de `root`.
pub fn degree(root: i32, scale: &[i32], d: i32) -> f32 {
    let n = scale.len() as i32;
    (root + 12 * d.div_euclid(n) + scale[d.rem_euclid(n) as usize]) as f32
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bus {
    Musique,
    Ambiance,
    Interface,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
}

/// Filtre biquadratique (formules de R. Bristow-Johnson).
#[derive(Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new(kind: FilterKind, f: f32, q: f32, rate: f32) -> Self {
        let mut b = Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 };
        b.set(kind, f, q, rate);
        b
    }
    pub fn set(&mut self, kind: FilterKind, f: f32, q: f32, rate: f32) {
        let f = f.clamp(10.0, rate * 0.45);
        let w = TAU * f / rate;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q.max(0.05));
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = match kind {
            FilterKind::LowPass => ((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0),
            FilterKind::HighPass => ((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0),
            FilterKind::BandPass => (alpha, 0.0, -alpha),
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Bruit brun : bruit blanc intégré avec fuite (spectre en 1/f²).
#[derive(Default, Clone, Copy)]
pub struct Brown(f32);

impl Brown {
    #[inline]
    pub fn run(&mut self, white: f32) -> f32 {
        self.0 = (self.0 + 0.02 * white) / 1.02;
        self.0 * 3.5
    }
}

/// Valeur lissée vers une cible (pente exponentielle).
#[derive(Clone, Copy)]
pub struct Smooth {
    pub value: f32,
    pub target: f32,
}

impl Smooth {
    pub fn new(v: f32) -> Self {
        Self { value: v, target: v }
    }
    /// Avance d'un pas de durée `dt` avec une constante de temps `tau`.
    pub fn step(&mut self, dt: f32, tau: f32) -> f32 {
        self.value += (self.target - self.value) * (dt / tau.max(1e-4)).min(1.0);
        self.value
    }
}

/// Forme d'une voix ponctuelle.
pub enum Kind {
    /// Corde pincée (Karplus-Strong) : harpe, kalimba, luth.
    Pluck { ring: Vec<f32>, idx: usize, loss: f32, dur: f32 },
    /// Cloche en modulation de phase : verre, kalimba, clochette.
    Bell { f: f32, ratio: f32, index: f32, dec: f32, pc: f32, pm: f32 },
    /// Nappe tenue d'oscillateurs désaccordés, filtrée.
    Pad { oscs: Vec<(f32, f32)>, tri: bool, lp: Biquad, att: f32, dur: f32 },
    /// Glissando bref en sinus : bulle, goutte.
    Chirp { f0: f32, f1: f32, dur: f32, phase: f32 },
    /// Souffle de bruit filtré, filtre éventuellement balayé.
    Burst { filt: Biquad, kind: FilterKind, f0: f32, f1: f32, q: f32, att: f32, len: f32, brown: Option<Brown> },
    /// Coup sourd : sinus dont la hauteur tombe.
    Thump { f0: f32, dec: f32, phase: f32 },
}

pub struct Voice {
    pub kind: Kind,
    pub bus: Bus,
    /// Retard avant le départ, en échantillons.
    pub delay: u32,
    pub age: f32,
    pub gain: f32,
    pub pan: f32,
    /// Part envoyée à la réverbération.
    pub send: f32,
}

impl Voice {
    pub fn new(kind: Kind, bus: Bus, gain: f32) -> Self {
        Self { kind, bus, delay: 0, age: 0.0, gain, pan: 0.0, send: 0.25 }
    }
    pub fn at(mut self, seconds: f32, rate: f32) -> Self {
        self.delay = (seconds.max(0.0) * rate) as u32;
        self
    }
    pub fn pan(mut self, p: f32) -> Self {
        self.pan = p.clamp(-1.0, 1.0);
        self
    }
    pub fn send(mut self, s: f32) -> Self {
        self.send = s;
        self
    }

    /// Échantillon suivant, ou `None` quand la voix est finie.
    #[inline]
    pub fn next(&mut self, rng: &mut Rng, dt: f32, rate: f32) -> Option<f32> {
        if self.delay > 0 {
            self.delay -= 1;
            return Some(0.0);
        }
        let t = self.age;
        self.age += dt;
        let s = match &mut self.kind {
            Kind::Pluck { ring, idx, loss, dur } => {
                if t > *dur {
                    return None;
                }
                let n = ring.len();
                let a = ring[*idx];
                let b = ring[(*idx + 1) % n];
                ring[*idx] = (a + b) * 0.5 * *loss;
                *idx = (*idx + 1) % n;
                let fade = ((*dur - t) / 0.05).min(1.0);
                a * fade
            }
            Kind::Bell { f, ratio, index, dec, pc, pm } => {
                if t > *dec {
                    return None;
                }
                *pc = (*pc + *f * dt).fract();
                *pm = (*pm + *f * *ratio * dt).fract();
                let idx = *index * (-t / (*dec * 0.25)).exp() + 0.05;
                let env = if t < 0.004 { t / 0.004 } else { (-(t - 0.004) * 6.9 / *dec).exp() };
                (TAU * *pc + idx * (TAU * *pm).sin()).sin() * env
            }
            Kind::Pad { oscs, tri, lp, att, dur } => {
                if t > *dur {
                    return None;
                }
                let rel = *dur * 0.35;
                let env = if t < *att {
                    t / *att
                } else if t > *dur - rel {
                    (*dur - t) / rel
                } else {
                    1.0
                };
                let mut sum = 0.0;
                for (ph, inc) in oscs.iter_mut() {
                    *ph = (*ph + *inc).fract();
                    sum += if *tri { 4.0 * (*ph - 0.5).abs() - 1.0 } else { (TAU * *ph).sin() };
                }
                lp.run(sum / oscs.len() as f32) * env
            }
            Kind::Chirp { f0, f1, dur, phase } => {
                if t > *dur + 0.04 {
                    return None;
                }
                let k = (t / *dur).min(1.0);
                let f = *f0 * (*f1 / *f0).powf(k);
                *phase = (*phase + f * dt).fract();
                let env = if t < 0.004 { t / 0.004 } else { (-(t - 0.004) / (*dur * 0.6 + 0.01)).exp() };
                (TAU * *phase).sin() * env
            }
            Kind::Burst { filt, kind, f0, f1, q, att, len, brown } => {
                if t > *len {
                    return None;
                }
                if *f1 != *f0 && ((t * rate) as u32).is_multiple_of(64) {
                    filt.set(*kind, *f0 * (*f1 / *f0).powf(t / *len), *q, rate);
                }
                let w = rng.bipolar();
                let x = match brown {
                    Some(b) => b.run(w),
                    None => w,
                };
                let env = if t < *att { t / *att } else { (-(t - *att) * 5.0 / (*len - *att).max(1e-3)).exp() };
                filt.run(x) * env
            }
            Kind::Thump { f0, dec, phase } => {
                if t > *dec {
                    return None;
                }
                let f = *f0 * (0.45f32).powf((t / (*dec * 0.6)).min(1.0));
                *phase = (*phase + f * dt).fract();
                (TAU * *phase).sin() * (-t * 5.0 / *dec).exp()
            }
        };
        Some(s * self.gain)
    }
}

/// Fabrique de voix, au taux d'échantillonnage donné.
pub struct Make {
    pub rate: f32,
}

impl Make {
    pub fn pluck(&self, rng: &mut Rng, midi: f32, bright: f32, dur: f32) -> Kind {
        let f = mtof(midi);
        let n = ((self.rate / f).round() as usize).max(2);
        let mut ring = vec![0.0f32; n];
        let mut prev = 0.0;
        for v in ring.iter_mut() {
            prev += (rng.bipolar() - prev) * bright.clamp(0.05, 1.0);
            *v = prev;
        }
        let loss = 0.001f32.powf(1.0 / (dur * f * 1.4));
        Kind::Pluck { ring, idx: 0, loss, dur }
    }
    pub fn bell(&self, midi: f32, ratio: f32, index: f32, dec: f32) -> Kind {
        Kind::Bell { f: mtof(midi), ratio, index, dec, pc: 0.0, pm: 0.0 }
    }
    pub fn pad(&self, rng: &mut Rng, notes: &[f32], dur: f32, att: f32, tri: bool, cutoff: f32) -> Kind {
        let mut oscs = Vec::with_capacity(notes.len() * 2);
        for &m in notes {
            for cents in [-5.0f32, 5.0] {
                let f = mtof(m + (cents + rng.range(-2.0, 2.0)) / 100.0);
                oscs.push((rng.u(), f / self.rate));
            }
        }
        Kind::Pad { oscs, tri, lp: Biquad::new(FilterKind::LowPass, cutoff, 0.7, self.rate), att, dur }
    }
    pub fn chirp(&self, f0: f32, f1: f32, dur: f32) -> Kind {
        Kind::Chirp { f0, f1, dur, phase: 0.0 }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn burst(&self, kind: FilterKind, f0: f32, f1: f32, q: f32, att: f32, len: f32, brown: bool) -> Kind {
        Kind::Burst { filt: Biquad::new(kind, f0, q, self.rate), kind, f0, f1, q, att, len, brown: brown.then(Brown::default) }
    }
    pub fn thump(&self, f0: f32, dec: f32) -> Kind {
        Kind::Thump { f0, dec, phase: 0.0 }
    }
}

/// Fond continu : bruit filtré dont la fréquence et le volume respirent.
pub struct Bed {
    brown: Option<Brown>,
    filt: Biquad,
    kind: FilterKind,
    pub freq: Smooth,
    q: f32,
    pub gain: Smooth,
    lfo_f: (f32, f32, f32),
    lfo_g: (f32, f32, f32),
    tick: u32,
}

impl Bed {
    pub fn new(brown: bool, kind: FilterKind, freq: f32, q: f32, rate: f32) -> Self {
        Self {
            brown: brown.then(Brown::default),
            filt: Biquad::new(kind, freq, q, rate),
            kind,
            freq: Smooth::new(freq),
            q,
            gain: Smooth::new(0.0),
            lfo_f: (0.0, 0.0, 0.0),
            lfo_g: (0.0, 0.0, 0.0),
            tick: 0,
        }
    }
    /// Respiration de la fréquence : (cadence en Hz, ampleur en Hz).
    pub fn breathe_freq(mut self, rate_hz: f32, depth: f32) -> Self {
        self.lfo_f = (0.0, rate_hz, depth);
        self
    }
    /// Respiration du volume : (cadence en Hz, part du volume).
    pub fn breathe_gain(mut self, rate_hz: f32, depth: f32) -> Self {
        self.lfo_g = (0.0, rate_hz, depth);
        self
    }
    #[inline]
    pub fn run(&mut self, rng: &mut Rng, dt: f32, rate: f32) -> f32 {
        self.lfo_f.0 = (self.lfo_f.0 + self.lfo_f.1 * dt).fract();
        self.lfo_g.0 = (self.lfo_g.0 + self.lfo_g.1 * dt).fract();
        let g = self.gain.step(dt, 0.8);
        self.tick = self.tick.wrapping_add(1);
        if self.tick.is_multiple_of(64) {
            let f = self.freq.step(dt * 64.0, 1.0) + self.lfo_f.2 * (TAU * self.lfo_f.0).sin();
            self.filt.set(self.kind, f.max(20.0), self.q, rate);
        }
        if g < 1e-5 {
            return 0.0;
        }
        let w = rng.bipolar();
        let x = match &mut self.brown {
            Some(b) => b.run(w),
            None => w,
        };
        self.filt.run(x) * g * (1.0 + self.lfo_g.2 * (TAU * self.lfo_g.0).sin())
    }
}

/// Réverbération de type Freeverb, allégée (quatre peignes, deux passe-tout
/// par canal).
pub struct Reverb {
    combs: [[(Vec<f32>, usize, f32); 4]; 2],
    alls: [[(Vec<f32>, usize); 2]; 2],
}

impl Reverb {
    pub fn new(rate: f32) -> Self {
        let k = rate / 44_100.0;
        let comb = |n: usize| (vec![0.0f32; ((n as f32) * k) as usize + 1], 0usize, 0.0f32);
        let all = |n: usize| (vec![0.0f32; ((n as f32) * k) as usize + 1], 0usize);
        let c = [1116, 1188, 1277, 1356];
        let a = [556, 441];
        Self { combs: [c.map(comb), c.map(|n| comb(n + 23))], alls: [a.map(all), a.map(|n| all(n + 23))] }
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> [f32; 2] {
        let mut out = [0.0f32; 2];
        for (ch, o) in out.iter_mut().enumerate() {
            let mut s = 0.0;
            for (buf, i, lp) in self.combs[ch].iter_mut() {
                let y = buf[*i];
                *lp = y * 0.75 + *lp * 0.25;
                buf[*i] = x + *lp * 0.85;
                *i = (*i + 1) % buf.len();
                s += y;
            }
            for (buf, i) in self.alls[ch].iter_mut() {
                let b = buf[*i];
                buf[*i] = s + b * 0.5;
                *i = (*i + 1) % buf.len();
                s = b - s;
            }
            *o = s * 0.12;
        }
        out
    }
}
