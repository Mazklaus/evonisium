//! Apprentissage des agents (étape 6) : les modèles détaillés sur lesquels
//! le modèle agrégé des groupes culturels (evo-culture) est calé, hors
//! partie.
//!
//! Trois bancs, chacun avec les primitives que le modèle agrégé partage
//! (`evo_culture::Learning`) :
//!
//! - [`Band`] : une troupe où chaque individu naît naïf et apprend un trait
//!   en observant des membres pris au hasard (`contacts_per_year`
//!   observations par an, réussies avec la fidélité de copie et l'attrait du
//!   trait). C'est la diffusion de Koshima ; le modèle agrégé en est la
//!   limite de champ moyen, λ·p·(1 − p) − μ·p ;
//! - [`SkillBand`] : le modèle de Henrich (2004) individu par individu.
//!   Chaque génération copie le plus habile avec une perte α et un bruit de
//!   Gumbel β ; le gain moyen par génération est celui du modèle agrégé,
//!   β(γ + ln n) − α ;
//! - [`Signalling`] : un jeu de signalisation de Lewis entre agents qui
//!   apprennent par renforcement (Roth et Erev, avec un léger découragement
//!   des échecs) ; il converge vers une
//!   convention partagée, bornée par l'expressivité de l'espèce. Le modèle
//!   agrégé garde la convention (une forme par sens) et sa dérive.
//!
//! Aucun de ces agents ne touche le monde ni l'état culturel : seuls les
//! tests les comparent.
//!
//! Enfin [`CultureHint`] traduit l'état culturel publié d'un groupe en
//! actions culturelles pour les agents de la scène (fabriquer un outil,
//! enseigner, échanger, célébrer un rite) : de l'affichage, sans effet.

use evo_core::math::Det;
use evo_core::rng::{rng_for, SimRng, Stream};
use evo_culture::cognition::{Learning, Mode};
use evo_culture::state::GroupView;
use evo_culture::traits::TraitKind;
use rand::Rng;

const TAG_BAND: u64 = 61;
const TAG_SKILL: u64 = 62;
const TAG_SIGNAL: u64 = 63;

/// Une troupe qui apprend un trait par observation.
pub struct Band {
    pub learning: Learning,
    /// Attrait du trait (1 + bénéfice pour une technique).
    pub appeal: f64,
    /// Porteurs du trait.
    pub carriers: Vec<bool>,
    rng: SimRng,
    pub year: f64,
}

impl Band {
    /// Troupe de `n` individus dont `initial` portent le trait.
    pub fn new(learning: Learning, appeal: f64, n: usize, initial: usize, seed: u64) -> Band {
        let mut carriers = vec![false; n];
        for c in carriers.iter_mut().take(initial) {
            *c = true;
        }
        Band { learning, appeal, carriers, rng: rng_for(seed, Stream::Individuals, &[TAG_BAND]), year: 0.0 }
    }

    pub fn freq(&self) -> f64 {
        self.carriers.iter().filter(|c| **c).count() as f64 / self.carriers.len() as f64
    }

    /// Avance de `dt` années, en sous-pas de `sub` années. Chaque individu
    /// meurt au taux 1/T et laisse place à un naïf ; chaque naïf observe
    /// `contacts_per_year` membres par an et copie le trait d'un porteur
    /// avec la probabilité fidélité × attrait.
    pub fn advance(&mut self, dt: f64, sub: f64) {
        let n = self.carriers.len();
        let steps = (dt / sub).round().max(1.0) as usize;
        let death = 1.0 - (-sub / self.learning.generation_years).dexp();
        let looks = self.learning.contacts_per_year * sub;
        let success = (self.learning.fidelity * self.appeal).min(1.0);
        for _ in 0..steps {
            let before = self.carriers.clone();
            for i in 0..n {
                if self.rng.random_bool(death) {
                    self.carriers[i] = false;
                    continue;
                }
                if before[i] {
                    continue;
                }
                // Nombre d'observations du sous-pas (Poisson par
                // inversion, la moyenne est petite).
                let mut k = 0;
                let mut acc = (-looks).dexp();
                let mut cum = acc;
                let u: f64 = self.rng.random();
                while u > cum && k < 50 {
                    k += 1;
                    acc *= looks / k as f64;
                    cum += acc;
                }
                for _ in 0..k {
                    let j = self.rng.random_range(0..n);
                    if j != i && before[j] && self.rng.random_bool(success) {
                        self.carriers[i] = true;
                        break;
                    }
                }
            }
            self.year += sub;
        }
    }
}

/// Le modèle de Henrich individu par individu, pour une technique.
pub struct SkillBand {
    pub learning: Learning,
    pub difficulty: f64,
    /// Savoir-faire de chacun, en unités de difficulté.
    pub skill: Vec<f64>,
    rng: SimRng,
}

impl SkillBand {
    pub fn new(learning: Learning, difficulty: f64, n: usize, start: f64, seed: u64) -> SkillBand {
        SkillBand { learning, difficulty, skill: vec![start; n], rng: rng_for(seed, Stream::Individuals, &[TAG_SKILL, n as u64]) }
    }

    pub fn mean(&self) -> f64 {
        self.skill.iter().sum::<f64>() / self.skill.len() as f64
    }

    /// Une génération : chacun copie le plus habile avec une perte
    /// α = difficulté × (1 − fidélité) et un bruit de Gumbel d'échelle β ;
    /// personne ne descend sous ce qu'il trouverait seul.
    pub fn generation(&mut self) {
        let best = self.skill.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let alpha = self.difficulty * (1.0 - self.learning.fidelity);
        let beta = evo_culture::cognition::COPY_NOISE;
        let floor = self.learning.individual_skill;
        for z in self.skill.iter_mut() {
            let u: f64 = self.rng.random::<f64>().max(1e-300);
            let gumbel = -beta * (-(u.dln())).dln();
            *z = (best - alpha + gumbel).max(floor);
        }
    }
}

/// Jeu de signalisation de Lewis : `states` situations, autant de signaux,
/// des agents qui émettent et reçoivent, renforcés quand ils se
/// comprennent.
pub struct Signalling {
    pub states: usize,
    /// Situations que chaque agent sait distinguer (expressivité).
    pub capacity: usize,
    /// Urnes d'émission [agent][état][signal] et de réception
    /// [agent][signal][état].
    send: Vec<Vec<Vec<f64>>>,
    recv: Vec<Vec<Vec<f64>>>,
    rng: SimRng,
    pub successes: u64,
    pub rounds: u64,
}

impl Signalling {
    pub fn new(agents: usize, states: usize, capacity: usize, seed: u64) -> Signalling {
        let urn = vec![vec![vec![1.0; states]; states]; agents];
        Signalling {
            states,
            capacity: capacity.min(states),
            send: urn.clone(),
            recv: urn,
            rng: rng_for(seed, Stream::Individuals, &[TAG_SIGNAL, agents as u64, states as u64]),
            successes: 0,
            rounds: 0,
        }
    }

    fn draw(rng: &mut SimRng, w: &[f64]) -> usize {
        let total: f64 = w.iter().sum();
        let mut u = rng.random::<f64>() * total;
        for (i, x) in w.iter().enumerate() {
            u -= x;
            if u <= 0.0 {
                return i;
            }
        }
        w.len() - 1
    }

    /// Une rencontre : un émetteur voit une situation et émet, un receveur
    /// agit. Les situations au-delà de l'expressivité ne se disent pas.
    pub fn round(&mut self) -> bool {
        let n = self.send.len();
        let a = self.rng.random_range(0..n);
        let mut b = self.rng.random_range(0..n - 1);
        if b >= a {
            b += 1;
        }
        let state = self.rng.random_range(0..self.states);
        self.rounds += 1;
        if state >= self.capacity {
            return false;
        }
        let signal = Self::draw(&mut self.rng, &self.send[a][state]);
        let act = Self::draw(&mut self.rng, &self.recv[b][signal][..self.capacity]);
        let ok = act == state;
        if ok {
            self.send[a][state][signal] += 1.0;
            self.recv[b][signal][state] += 1.0;
            self.successes += 1;
        } else {
            // Un échec décourage un peu (Bush et Mosteller) : sans lui, le
            // renforcement seul s'enlise souvent dans des signaux confondus.
            let s = &mut self.send[a][state][signal];
            *s = (*s - 0.5).max(0.1);
            let r = &mut self.recv[b][signal][act];
            *r = (*r - 0.5).max(0.1);
        }
        ok
    }

    /// Réussite moyenne sur `rounds` rencontres.
    pub fn play(&mut self, rounds: u64) -> f64 {
        let mut ok = 0u64;
        for _ in 0..rounds {
            ok += u64::from(self.round());
        }
        ok as f64 / rounds as f64
    }

    /// Part des situations dites dont le signal le plus fort est partagé par
    /// tous les émetteurs : la convention de la population.
    pub fn convention(&self) -> f64 {
        let mut shared = 0;
        for s in 0..self.capacity {
            let top = |a: usize| {
                let w = &self.send[a][s];
                (0..w.len()).max_by(|&x, &y| w[x].total_cmp(&w[y]).then(y.cmp(&x))).unwrap_or(0)
            };
            let first = top(0);
            if (1..self.send.len()).all(|a| top(a) == first) {
                shared += 1;
            }
        }
        shared as f64 / self.capacity.max(1) as f64
    }
}

/// Part du temps libre qu'un agent donne aux actions culturelles.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CultureHint {
    pub tool: f32,
    pub teach: f32,
    pub trade: f32,
    pub rite: f32,
}

impl CultureHint {
    pub fn total(&self) -> f32 {
        self.tool + self.teach + self.trade + self.rite
    }

    /// Lit le groupe culturel publié de la cellule observée.
    pub fn of(group: &GroupView, learning: &Learning, kinds: impl Fn(u32) -> Option<TraitKind>) -> CultureHint {
        let mut tools = 0.0f32;
        let mut rites = 0.0f32;
        for t in &group.traits {
            match kinds(t.id) {
                Some(TraitKind::Technique(_)) => tools += t.freq * t.mastery,
                Some(TraitKind::Rite | TraitKind::Myth(_) | TraitKind::Art) => rites += t.freq,
                _ => {}
            }
        }
        let teach = if learning.mode >= Mode::Teaching { 0.03 } else { 0.0 };
        CultureHint {
            tool: (0.03 * tools).min(0.12),
            teach,
            trade: if learning.mode >= Mode::Language { 0.02 } else { 0.0 },
            rite: (0.02 * rites).min(0.06),
        }
    }
}
