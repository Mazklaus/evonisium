//! Son du client Evonisium, indépendant de Godot (le crate `evo-godot` n'en
//! est que le pont). Voir le doc « Evonisium : design sonore » et le
//! paragraphe « Son » du doc d'architecture.
//!
//! Le son lit seulement l'état publié par le moteur et la chronique ; il
//! n'écrit jamais dans le moteur et n'utilise pas son hasard, donc il ne
//! change jamais l'histoire. Tout est synthétisé : aucun fichier audio.
//!
//! Livraison 1 : musique des ères microbiennes, souffle du globe, monde
//! liquide de la loupe, bruits de papier et d'encre de l'interface.

// Affichage seulement : rien de ce crate n'entre dans l'état simulé, les
// mathématiques de la plateforme y suffisent (voir clippy.toml).
#![allow(clippy::disallowed_methods)]

pub mod ambiance;
pub mod musique;
pub mod synth;

use ambiance::{Globe, Micro};
use musique::{Lecture, Musique};
use synth::{Biquad, Bus, FilterKind, Make, Reverb, Rng, Smooth, Voice};

/// Ce que le son lit du monde publié (une fois par image suffit).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Monde {
    /// Fraction molaire d'O₂ (0,21 sur la Terre actuelle).
    pub o2: f64,
    pub temperature_k: f64,
    pub ice: f64,
    pub ocean: f64,
    pub lineages: u32,
    pub paused: bool,
}

/// Vue où se trouve le joueur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vue {
    /// Écrans d'accueil, de création, de chargement.
    Menu,
    Globe,
    /// Loupe de l'inspecteur ouverte.
    Microscope,
    /// Descente au sol.
    Sol,
}

impl Vue {
    pub fn from_name(s: &str) -> Self {
        match s {
            "globe" | "jeu" => Vue::Globe,
            "microscope" | "loupe" => Vue::Microscope,
            "sol" => Vue::Sol,
            _ => Vue::Menu,
        }
    }
}

/// Bruits de papier et d'encre de l'interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bruit {
    /// Ouverture d'une fiche : plume qui gratte.
    Plume,
    /// Changement d'écran : page tournée.
    Page,
    /// Tampon (point de sauvegarde, ordre confirmé).
    Tampon,
    /// Événement notable de la chronique : clochette.
    Cloche,
    /// Étape franchie : tampon puis accord.
    Etape,
}

impl Bruit {
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "plume" => Bruit::Plume,
            "page" => Bruit::Page,
            "tampon" => Bruit::Tampon,
            "cloche" => Bruit::Cloche,
            "etape" => Bruit::Etape,
            _ => return None,
        })
    }
}

/// Moment fort de la chronique, que la musique souligne d'un motif.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    Jalon,
    Froid,
    Extinction,
}

impl Moment {
    /// Motif pour un événement de la chronique (famille du chantier
    /// Interface, score d'intérêt de 0 à 1), ou `None` s'il passe sans bruit.
    pub fn of_event(family: &str, interest: f64) -> Option<Self> {
        if interest < 0.5 {
            return None;
        }
        match family {
            "jalon" | "innovation" => Some(Moment::Jalon),
            "catastrophe" => Some(Moment::Froid),
            "extinction" => Some(Moment::Extinction),
            _ => None,
        }
    }
}

/// Ordre donné au son depuis le fil principal.
#[derive(Clone, Debug)]
pub enum Ordre {
    Monde(Monde),
    Vue(Vue),
    Evenement {
        family: String,
        interest: f64,
    },
    Bruit(Bruit),
    /// Volumes de 0 à 1 : général, musique, ambiances, interface.
    Volumes([f32; 4]),
    Muet(bool),
}

const MAX_VOICES: usize = 160;
/// Gain de sortie : environ -24 dB efficaces sur le globe au volume par défaut.
const GAIN: f32 = 2.5;

pub struct Son {
    pub rate: f32,
    dt: f32,
    rng: Rng,
    mk: Make,
    voices: Vec<Voice>,
    fresh: Vec<Voice>,
    pub musique: Musique,
    globe: Globe,
    micro: Micro,
    verb: Reverb,
    music_lp: [Biquad; 2],
    pub monde: Option<Monde>,
    pub vue: Vue,
    /// Part de chaque bus selon la vue (musique, ambiance), lissée.
    mix_music: Smooth,
    volumes: [f32; 4],
    mute: Smooth,
    ctrl: usize,
}

/// Pas de contrôle, en échantillons.
const BLOCK: usize = 256;

impl Son {
    pub fn new(rate: f32, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let musique = Musique::new(&mut rng);
        Self {
            rate,
            dt: 1.0 / rate,
            rng,
            mk: Make { rate },
            voices: Vec::with_capacity(MAX_VOICES),
            fresh: Vec::with_capacity(64),
            musique,
            globe: Globe::new(rate),
            micro: Micro::new(rate),
            verb: Reverb::new(rate),
            music_lp: [Biquad::new(FilterKind::LowPass, 2000.0, 0.6, rate); 2],
            monde: None,
            vue: Vue::Menu,
            mix_music: Smooth::new(0.6),
            volumes: [0.8, 0.7, 0.7, 0.6],
            mute: Smooth::new(1.0),
            ctrl: 0,
        }
    }

    pub fn ordre(&mut self, o: Ordre) {
        match o {
            Ordre::Monde(m) => self.monde = Some(m),
            Ordre::Vue(v) => self.vue = v,
            Ordre::Evenement { family, interest } => {
                if let Some(m) = Moment::of_event(&family, interest) {
                    self.musique.moment(m);
                }
            }
            Ordre::Bruit(b) => self.bruit(b),
            Ordre::Volumes(v) => self.volumes = v.map(|x| x.clamp(0.0, 1.0)),
            Ordre::Muet(m) => self.mute.target = if m { 0.0 } else { 1.0 },
        }
    }

    fn push(&mut self, v: Voice) {
        if self.voices.len() < MAX_VOICES {
            self.voices.push(v);
        }
    }

    /// Joue un bruit d'interface.
    pub fn bruit(&mut self, b: Bruit) {
        let mk = Make { rate: self.rate };
        let r = self.rate;
        let rng = &mut self.rng;
        let mut v: Vec<Voice> = Vec::new();
        match b {
            Bruit::Plume => {
                let mut t = 0.0;
                for _ in 0..(3 + (rng.u() * 3.0) as usize) {
                    let len = rng.range(0.07, 0.17);
                    let k =
                        mk.burst(FilterKind::BandPass, rng.range(2600.0, 4200.0), rng.range(2000.0, 5200.0), 1.6, len * 0.3, len, false);
                    v.push(Voice::new(k, Bus::Interface, 0.3).at(t, r).send(0.1));
                    t += len + rng.range(0.03, 0.12);
                }
            }
            Bruit::Page => {
                v.push(Voice::new(mk.burst(FilterKind::BandPass, 700.0, 3800.0, 0.9, 0.2, 0.45, false), Bus::Interface, 0.4).send(0.1));
                v.push(Voice::new(mk.thump(180.0, 0.08), Bus::Interface, 0.12).at(0.4, r));
            }
            Bruit::Tampon => {
                v.push(Voice::new(mk.thump(120.0, 0.14), Bus::Interface, 0.7));
                v.push(Voice::new(mk.burst(FilterKind::LowPass, 900.0, 900.0, 1.0, 0.002, 0.06, true), Bus::Interface, 0.35));
                v.push(Voice::new(mk.burst(FilterKind::BandPass, 2200.0, 2200.0, 1.0, 0.001, 0.03, false), Bus::Interface, 0.12));
            }
            Bruit::Cloche => {
                v.push(Voice::new(mk.bell(91.0, 3.5, 1.6, 2.6), Bus::Interface, 0.12).send(0.3));
                v.push(Voice::new(mk.bell(98.0, 3.5, 1.6, 2.2), Bus::Interface, 0.07).at(0.13, r).send(0.3));
            }
            Bruit::Etape => {
                v.push(Voice::new(mk.thump(120.0, 0.14), Bus::Interface, 0.7));
                for (i, m) in [74.0, 78.0, 81.0, 86.0, 90.0].into_iter().enumerate() {
                    v.push(Voice::new(mk.bell(m, 2.0, 1.4, 3.2), Bus::Interface, 0.07).at(0.35 + i as f32 * 0.11, r).send(0.4));
                }
                v.push(
                    Voice::new(mk.pad(rng, &[50.0, 57.0, 62.0, 66.0], 4.0, 0.4, false, 1200.0), Bus::Interface, 0.05).at(0.3, r).send(0.4),
                );
            }
        }
        for x in v {
            self.push(x);
        }
    }

    /// Gains cibles (musique, globe, loupe) de la vue courante.
    fn targets(&self) -> (f32, f32, f32) {
        let paused = self.monde.is_some_and(|m| m.paused);
        let (m, g, u) = match self.vue {
            Vue::Menu => (0.6, 0.0, 0.0),
            Vue::Globe => (1.0, 1.0, 0.0),
            Vue::Microscope => (0.45, 0.15, 1.0),
            Vue::Sol => (0.7, 0.6, 0.0),
        };
        // En pause, l'histoire s'arrête : la musique se retire un peu.
        if paused {
            (m * 0.5, g, u)
        } else {
            (m, g, u)
        }
    }

    fn control(&mut self) {
        let dt = BLOCK as f32 * self.dt;
        let l = Lecture::of(self.monde.as_ref());
        let (m, g, u) = self.targets();
        self.mix_music.target = m;
        self.globe.set(&l, g);
        self.micro.set(u);
        let lp = l.cutoff();
        for f in self.music_lp.iter_mut() {
            f.set(FilterKind::LowPass, lp, 0.6, self.rate);
        }
        let mut fresh = std::mem::take(&mut self.fresh);
        self.musique.tick(dt, &l, &self.mk, &mut self.rng, &mut fresh);
        self.globe.tick(dt, &l, &self.mk, &mut self.rng, &mut fresh);
        self.micro.tick(dt, &l, &self.mk, &mut self.rng, &mut fresh);
        for v in fresh.drain(..) {
            self.push(v);
        }
        self.fresh = fresh;
    }

    /// Remplit `out` d'échantillons stéréo entrelacés par paires.
    pub fn rendre(&mut self, out: &mut [[f32; 2]]) {
        let dt = self.dt;
        let rate = self.rate;
        for frame in out.iter_mut() {
            if self.ctrl == 0 {
                self.control();
            }
            self.ctrl = (self.ctrl + 1) % BLOCK;
            let mut bus = [[0.0f32; 2]; 3];
            let mut send = 0.0f32;
            let rng = &mut self.rng;
            self.voices.retain_mut(|v| match v.next(rng, dt, rate) {
                Some(s) => {
                    let b = v.bus as usize;
                    let a = (v.pan + 1.0) * 0.25 * std::f32::consts::PI;
                    bus[b][0] += s * a.cos();
                    bus[b][1] += s * a.sin();
                    send += s * v.send;
                    true
                }
                None => false,
            });
            let amb = self.globe.run(rng, dt, rate) + self.micro.run(rng, dt, rate);
            bus[1][0] += amb;
            bus[1][1] += amb;
            let mm = self.mix_music.step(dt, 2.0);
            let [vg, vm, va, vi] = self.volumes;
            let w = self.verb.run(send);
            let mute = self.mute.step(dt, 0.3);
            for ch in 0..2 {
                let music = self.music_lp[ch].run(bus[0][ch]) * mm * vm;
                let x = (music + bus[1][ch] * va + bus[2][ch] * vi + w[ch] * (vm + va) * 0.5) * vg * mute * GAIN;
                // Limiteur doux : jamais de saturation dure.
                frame[ch] = x.tanh() * 0.95;
            }
        }
    }

    pub fn voices(&self) -> usize {
        self.voices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terre(lineages: u32, o2: f64) -> Monde {
        Monde { o2, temperature_k: 288.0, ice: 0.05, ocean: 0.7, lineages, paused: false }
    }

    fn render(son: &mut Son, seconds: f32) -> (f32, f32) {
        let mut buf = vec![[0.0f32; 2]; (son.rate * seconds) as usize];
        son.rendre(&mut buf);
        let mut peak = 0.0f32;
        let mut sum = 0.0f64;
        for f in &buf {
            for &x in f {
                assert!(x.is_finite());
                peak = peak.max(x.abs());
                sum += (x * x) as f64;
            }
        }
        (peak, (sum / (buf.len() * 2) as f64).sqrt() as f32)
    }

    #[test]
    fn every_view_sounds_without_clipping() {
        for vue in [Vue::Menu, Vue::Globe, Vue::Microscope, Vue::Sol] {
            let mut son = Son::new(44_100.0, 7);
            son.ordre(Ordre::Monde(terre(40, 1e-3)));
            son.ordre(Ordre::Vue(vue));
            let (peak, rms) = render(&mut son, 20.0);
            assert!(peak < 0.95, "{vue:?} : crête {peak}");
            assert!(rms > 0.002, "{vue:?} : silence ({rms})");
            assert!(son.voices() < MAX_VOICES);
        }
    }

    #[test]
    fn interface_sounds_play_and_end() {
        let mut son = Son::new(44_100.0, 3);
        son.ordre(Ordre::Volumes([1.0, 0.0, 0.0, 1.0]));
        for b in [Bruit::Plume, Bruit::Page, Bruit::Tampon, Bruit::Cloche, Bruit::Etape] {
            son.ordre(Ordre::Bruit(b));
        }
        let (peak, _) = render(&mut son, 1.0);
        assert!(peak > 0.01 && peak < 0.95);
        render(&mut son, 6.0);
        // La musique tourne encore (volume nul) mais tous les bruits sont finis.
        assert!(son.voices.iter().all(|v| v.bus != Bus::Interface));
    }

    #[test]
    fn life_and_oxygen_change_the_music() {
        let poor = Lecture::of(Some(&terre(1, 1e-6)));
        let rich = Lecture::of(Some(&terre(300, 0.2)));
        assert!(rich.bio > poor.bio + 0.5);
        assert!(rich.cutoff() > 4.0 * poor.cutoff());
        let cold = Lecture::of(Some(&Monde { temperature_k: 240.0, ..terre(10, 1e-3) }));
        let hot = Lecture::of(Some(&Monde { temperature_k: 320.0, ..terre(10, 1e-3) }));
        let mode = |l: &Lecture| ((l.temp * 5.0) as usize).min(4);
        assert_eq!(musique::MODES[mode(&cold)].0, "phrygien");
        assert_eq!(musique::MODES[mode(&hot)].0, "lydien");
    }

    #[test]
    fn music_rests_between_phrases() {
        let mut son = Son::new(22_050.0, 11);
        son.ordre(Ordre::Monde(terre(100, 1e-2)));
        son.ordre(Ordre::Vue(Vue::Globe));
        let mut seen_rest = false;
        for _ in 0..300 {
            render(&mut son, 1.0);
            seen_rest |= !son.musique.playing;
        }
        assert!(seen_rest, "cinq minutes sans un seul repos");
    }

    #[test]
    fn mute_silences_everything() {
        let mut son = Son::new(44_100.0, 5);
        son.ordre(Ordre::Monde(terre(40, 1e-3)));
        son.ordre(Ordre::Vue(Vue::Globe));
        son.ordre(Ordre::Muet(true));
        render(&mut son, 3.0);
        let (peak, _) = render(&mut son, 2.0);
        assert!(peak < 1e-3);
    }

    #[test]
    fn notable_events_become_motifs() {
        assert_eq!(Moment::of_event("jalon", 0.9), Some(Moment::Jalon));
        assert_eq!(Moment::of_event("catastrophe", 0.85), Some(Moment::Froid));
        assert_eq!(Moment::of_event("extinction", 0.1), None);
        assert_eq!(Moment::of_event("speciation", 0.9), None);
    }
}
