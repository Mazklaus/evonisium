//! Lectures cognitives du corps et ce qu'elles permettent d'apprendre.
//!
//! Le plan de construction gagne à l'étape 6 quatre lectures, calculées par
//! le constructeur de phénotypes (fil « Animaux et écosystèmes ») : huit
//! capacités cognitives et le budget qui les borne, l'organe de
//! manipulation, les canaux de communication et la période juvénile.
//! [`Cognition`] les reçoit telles quelles ; [`Learning`] en tire les
//! grandeurs de la transmission culturelle, les mêmes pour les agents
//! détaillés (evo-agents) et pour le modèle agrégé des groupes : c'est ce
//! partage des primitives qui permet de caler l'un sur l'autre.

use serde::{Deserialize, Serialize};

/// Les huit capacités cognitives du doc Intelligence et sociétés.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capacity {
    WorkingMemory,
    LongTermMemory,
    Categorisation,
    PhysicalCausality,
    Planning,
    TheoryOfMind,
    SelfAwareness,
    Symbolic,
}

impl Capacity {
    pub const ALL: [Capacity; 8] = [
        Capacity::WorkingMemory,
        Capacity::LongTermMemory,
        Capacity::Categorisation,
        Capacity::PhysicalCausality,
        Capacity::Planning,
        Capacity::TheoryOfMind,
        Capacity::SelfAwareness,
        Capacity::Symbolic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Capacity::WorkingMemory => "mémoire de travail",
            Capacity::LongTermMemory => "mémoire à long terme",
            Capacity::Categorisation => "catégorisation",
            Capacity::PhysicalCausality => "causalité physique",
            Capacity::Planning => "planification",
            Capacity::TheoryOfMind => "théorie de l'esprit",
            Capacity::SelfAwareness => "conscience de soi",
            Capacity::Symbolic => "pensée symbolique",
        }
    }
}

/// Organe qui sert à manipuler les objets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Organ {
    None,
    Hand,
    Mouth,
    Beak,
    Trunk,
    Tentacle,
}

impl Organ {
    /// Organe autre qu'une main : les techniques passent par leur voie
    /// « sans mains » quand elle existe.
    pub fn handless(self) -> bool {
        !matches!(self, Organ::Hand)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manipulation {
    pub organ: Organ,
    /// Qualité entre 0 et 1 : précision et force de la prise.
    pub quality: f64,
}

/// Canaux de communication : débit de chacun entre 0 et 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Channels {
    pub sound: f64,
    pub visual: f64,
    pub chemical: f64,
    pub tactile: f64,
    pub electric: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Sound,
    Visual,
    Chemical,
    Tactile,
    Electric,
}

impl Channels {
    /// Canal au plus fort débit et ce débit.
    pub fn best(&self) -> (Channel, f64) {
        let all = [
            (Channel::Sound, self.sound),
            (Channel::Visual, self.visual),
            (Channel::Chemical, self.chemical),
            (Channel::Tactile, self.tactile),
            (Channel::Electric, self.electric),
        ];
        let mut best = all[0];
        for c in all {
            if c.1 > best.1 {
                best = c;
            }
        }
        best
    }
}

/// Les quatre lectures cognitives d'un génotype, plus deux grandeurs de
/// démographie et de socialité déjà connues du moteur.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cognition {
    /// Capacités entre 0 et 1, dans l'ordre de [`Capacity::ALL`].
    pub capacities: [f64; 8],
    /// Budget cérébral : somme des capacités que le tissu nerveux permet.
    pub brain_budget: f64,
    pub manipulation: Manipulation,
    pub channels: Channels,
    /// Durée de la période juvénile, années.
    pub juvenile_years: f64,
    /// Temps de génération, années.
    pub generation_years: f64,
    /// Socialité entre 0 (solitaire) et 1 (groupes stables et soudés).
    pub sociality: f64,
}

impl Cognition {
    pub fn get(&self, c: Capacity) -> f64 {
        self.capacities[c as usize]
    }

    /// Capacités ramenées sous le budget : si leur somme le dépasse, elles
    /// sont réduites en proportion.
    pub fn bounded(&self) -> [f64; 8] {
        let sum: f64 = self.capacities.iter().sum();
        let k = if sum > self.brain_budget && sum > 0.0 { self.brain_budget / sum } else { 1.0 };
        let mut out = self.capacities;
        for v in out.iter_mut() {
            *v = (*v * k).clamp(0.0, 1.0);
        }
        out
    }
}

/// Modes d'apprentissage social, du moins au plus fidèle. L'écriture
/// arrive à l'étape 7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Mode {
    /// Aucun apprentissage social : l'espèce n'a pas de groupes culturels.
    None,
    /// Observation et émulation (rehaussement du stimulus).
    Observation,
    Imitation,
    /// Enseignement actif d'un adulte à un jeune.
    Teaching,
    Language,
}

impl Mode {
    /// Fidélité de copie de base (Boyd et Richerson ; Tomasello). Valeurs
    /// d'ordre de grandeur, à caler avec les agents.
    pub fn base_fidelity(self) -> f64 {
        match self {
            Mode::None => 0.0,
            Mode::Observation => 0.6,
            Mode::Imitation => 0.8,
            Mode::Teaching => 0.9,
            Mode::Language => 0.97,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::None => "aucun",
            Mode::Observation => "observation",
            Mode::Imitation => "imitation",
            Mode::Teaching => "enseignement",
            Mode::Language => "langage",
        }
    }
}

/// Bruit des erreurs de copie (paramètre β du modèle de Henrich, 2004), en
/// unités de savoir-faire. Le même pour toutes les techniques : leur
/// difficulté porte la différence.
pub const COPY_NOISE: f64 = 1.0;

/// Constante d'Euler-Mascheroni, qui apparaît dans le maximum de tirages de
/// Gumbel.
pub const EULER_GAMMA: f64 = 0.577_215_664_901_532_9;

/// Difficulté de référence de la culture cumulative : une espèce franchit
/// le seuil quand ses groupes améliorent une technique de cette difficulté
/// au lieu de la perdre. Calée pour qu'un langage dans une bande d'une
/// cinquantaine d'individus connectés y suffise, et que l'enseignement
/// sans langage demande des dizaines de milliers d'individus.
pub const CUMULATIVE_DIFFICULTY: f64 = 150.0;

/// Ce que les lectures cognitives permettent d'apprendre.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Learning {
    pub mode: Mode,
    /// Fidélité de copie effective, entre 0 et 1.
    pub fidelity: f64,
    /// Occasions d'apprendre par individu naïf et par an : chaque
    /// occasion observe un membre du groupe pris au hasard.
    pub contacts_per_year: f64,
    /// Difficulté qu'un individu atteint seul, par essais et erreurs.
    pub individual_skill: f64,
    /// Propension à inventer, entre 0 et 1.
    pub inventiveness: f64,
    /// Pensée symbolique (croyances, mythes, rites, arts).
    pub symbolic: f64,
    /// Biais de conformisme (D de Boyd et Richerson), par génération.
    pub conformism: f64,
    /// Nombre de sens que la langue sait porter, entre 1 et 64.
    pub expressivity: u32,
    pub generation_years: f64,
}

impl Learning {
    pub fn of(c: &Cognition) -> Learning {
        let cap = c.bounded();
        let g = |k: Capacity| cap[k as usize];
        let (_, bandwidth) = c.channels.best();
        let social = c.sociality.clamp(0.0, 1.0);
        let mode = if social < 0.15 || g(Capacity::Categorisation) < 0.2 || g(Capacity::LongTermMemory) < 0.2 {
            Mode::None
        } else if g(Capacity::Symbolic) >= 0.5 && bandwidth >= 0.5 && g(Capacity::TheoryOfMind) >= 0.5 {
            Mode::Language
        } else if g(Capacity::TheoryOfMind) >= 0.5 && c.juvenile_years >= 1.0 && bandwidth >= 0.3 {
            Mode::Teaching
        } else if g(Capacity::TheoryOfMind) >= 0.3 && g(Capacity::WorkingMemory) >= 0.3 {
            Mode::Imitation
        } else {
            Mode::Observation
        };
        // La fidélité de base est atteinte avec des capacités moyennes ;
        // elle gagne ou perd un peu selon la mémoire et la théorie de
        // l'esprit.
        let tune = (g(Capacity::WorkingMemory) + g(Capacity::TheoryOfMind)) * 0.5;
        let fidelity = if mode == Mode::None {
            0.0
        } else {
            let base = mode.base_fidelity();
            (base + (1.0 - base) * 0.4 * (tune - 0.5)).clamp(0.0, 0.995)
        };
        let generation = c.generation_years.max(0.1);
        // Une longue enfance multiplie les occasions d'apprendre (Kaplan).
        let juvenile = (0.5 + c.juvenile_years / generation).clamp(0.5, 1.5);
        // Calé sur Koshima (banc d'essai `koshima`) : λ = contacts × T ×
        // fidélité ≈ 7 par génération pour un macaque.
        let lambda = 16.0 * social * juvenile * fidelity;
        let contacts_per_year = if fidelity > 0.0 { lambda / (generation * fidelity) } else { 0.0 };
        let individual_skill = 3.0 * (g(Capacity::PhysicalCausality) + g(Capacity::Planning) + g(Capacity::WorkingMemory)) / 3.0;
        let inventiveness = (g(Capacity::PhysicalCausality) * (0.5 + g(Capacity::Planning))).clamp(0.0, 1.0);
        let symbolic = g(Capacity::Symbolic);
        let expressivity = (1.0 + 63.0 * symbolic * bandwidth).round().clamp(1.0, 64.0) as u32;
        Learning {
            mode,
            fidelity,
            contacts_per_year,
            individual_skill,
            inventiveness,
            symbolic,
            conformism: 0.3 + 0.7 * g(Capacity::TheoryOfMind) * social,
            expressivity,
            generation_years: generation,
        }
    }

    pub fn social(&self) -> bool {
        self.mode != Mode::None
    }

    /// Taux de transmission λ par génération : un naïf qui rencontre une
    /// part p de porteurs acquiert le trait au taux λ·p.
    pub fn lambda(&self) -> f64 {
        self.contacts_per_year * self.generation_years * self.fidelity
    }

    /// Gain de savoir-faire par génération sur une technique de difficulté
    /// `d`, dans un groupe de `n` individus connectés (Henrich, 2004) :
    /// chacun copie le plus habile avec une perte α = d·(1 − fidélité) et un
    /// bruit de Gumbel β, d'où Δz̄ = β(γ + ln n) − α.
    pub fn skill_gain(&self, difficulty: f64, connected: f64) -> f64 {
        if !self.social() || connected < 1.0 {
            return -difficulty;
        }
        use evo_core::math::Det;
        COPY_NOISE * (EULER_GAMMA + connected.dln()) - difficulty * (1.0 - self.fidelity)
    }

    /// Même gain, à partir de ln n déjà calculé.
    pub fn skill_gain_ln(&self, difficulty: f64, ln_connected: f64) -> f64 {
        if !self.social() || ln_connected < 0.0 {
            return -difficulty;
        }
        COPY_NOISE * (EULER_GAMMA + ln_connected) - difficulty * (1.0 - self.fidelity)
    }

    /// Gain culturel : le gain de savoir-faire sur la difficulté de
    /// référence. Positif, la culture est cumulative.
    pub fn cultural_gain(&self, connected: f64) -> f64 {
        self.skill_gain(CUMULATIVE_DIFFICULTY, connected)
    }

    /// Maîtrise qu'un individu atteint seul sur une technique de difficulté
    /// `d` (entre 0 et 1).
    pub fn individual_mastery(&self, difficulty: f64) -> f64 {
        (self.individual_skill / difficulty.max(1e-9)).clamp(0.0, 1.0)
    }
}

/// Lectures de banc d'essai pour quelques espèces de référence.
pub mod presets {
    use super::*;

    fn cog(capacities: [f64; 8], organ: Organ, quality: f64, sound: f64, juvenile: f64, generation: f64, sociality: f64) -> Cognition {
        Cognition {
            capacities,
            brain_budget: 8.0,
            manipulation: Manipulation { organ, quality },
            channels: Channels { sound, visual: sound * 0.6, ..Channels::default() },
            juvenile_years: juvenile,
            generation_years: generation,
            sociality,
        }
    }

    /// Macaque japonais (Koshima) : observation, génération d'une dizaine
    /// d'années.
    pub fn macaque() -> Cognition {
        cog([0.4, 0.45, 0.5, 0.3, 0.2, 0.25, 0.2, 0.1], Organ::Hand, 0.55, 0.3, 4.0, 10.0, 0.8)
    }

    /// Chimpanzé : imitation, outils simples.
    pub fn chimpanzee() -> Cognition {
        cog([0.5, 0.55, 0.6, 0.45, 0.35, 0.4, 0.4, 0.2], Organ::Hand, 0.65, 0.35, 8.0, 25.0, 0.8)
    }

    /// Humain : langage.
    pub fn human() -> Cognition {
        cog([0.8, 0.85, 0.9, 0.85, 0.8, 0.85, 0.8, 0.85], Organ::Hand, 0.95, 0.9, 14.0, 25.0, 0.9)
    }

    /// Corbeau de Nouvelle-Calédonie : outils au bec.
    pub fn crow() -> Cognition {
        cog([0.45, 0.5, 0.55, 0.5, 0.35, 0.3, 0.3, 0.1], Organ::Beak, 0.5, 0.5, 1.5, 5.0, 0.4)
    }

    /// Dauphin : apprentissage social poussé, sans mains.
    pub fn dolphin() -> Cognition {
        cog([0.6, 0.65, 0.6, 0.35, 0.4, 0.55, 0.6, 0.35], Organ::Mouth, 0.3, 0.8, 4.0, 20.0, 0.85)
    }

    /// Espèce aquatique à tentacules, au langage par signaux lumineux.
    pub fn tentacled() -> Cognition {
        let mut c = cog([0.8, 0.8, 0.85, 0.8, 0.75, 0.8, 0.75, 0.8], Organ::Tentacle, 0.85, 0.2, 10.0, 20.0, 0.85);
        c.channels.visual = 0.9;
        c
    }

    /// Solitaire sans apprentissage social.
    pub fn solitary() -> Cognition {
        cog([0.3, 0.3, 0.3, 0.2, 0.1, 0.05, 0.1, 0.0], Organ::Mouth, 0.2, 0.2, 0.5, 2.0, 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_follow_the_body() {
        assert_eq!(Learning::of(&presets::solitary()).mode, Mode::None);
        assert_eq!(Learning::of(&presets::macaque()).mode, Mode::Observation);
        assert_eq!(Learning::of(&presets::chimpanzee()).mode, Mode::Imitation);
        assert_eq!(Learning::of(&presets::dolphin()).mode, Mode::Teaching);
        assert_eq!(Learning::of(&presets::human()).mode, Mode::Language);
        assert_eq!(Learning::of(&presets::tentacled()).mode, Mode::Language);
    }

    #[test]
    fn cumulative_culture_needs_language_or_crowds() {
        let human = Learning::of(&presets::human());
        let chimp = Learning::of(&presets::chimpanzee());
        let dolphin = Learning::of(&presets::dolphin());
        assert!(human.cultural_gain(150.0) > 0.0);
        assert!(human.cultural_gain(5.0) < 0.0);
        assert!(chimp.cultural_gain(1e6) < 0.0);
        assert!(dolphin.cultural_gain(1000.0) < 0.0);
    }

    #[test]
    fn budget_bounds_the_capacities() {
        let mut c = presets::human();
        c.brain_budget = 3.4;
        let b = c.bounded();
        assert!((b.iter().sum::<f64>() - 3.4).abs() < 1e-9);
        assert!(Learning::of(&c).mode < Mode::Language);
    }
}
