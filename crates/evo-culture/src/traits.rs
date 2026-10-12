//! Traits culturels : les techniques du catalogue et les traits générés
//! librement (croyances, rites, arts, tabous, mythes).
//!
//! Les effets des traits générés sont bornés à la cohésion, la coopération,
//! les tabous de ressources et la légitimité (génération mixte, décision du
//! 7 octobre 2026). Leur contenu (nom, objet) est tiré au hasard de l'état
//! simulé et ne change rien d'autre.

use crate::catalog::Family;
use crate::language::Phonology;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub type TraitId = u32;

/// Les techniques ont pour identifiant l'indice de leur famille ; les traits
/// générés commencent ici.
pub const FIRST_GENERATED: TraitId = 64;

/// Borne des effets d'un trait généré.
pub const EFFECT_BOUND: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    Meat,
    Fish,
    Shellfish,
    Fruit,
    Roots,
    Insects,
    Eggs,
}

impl Resource {
    pub const ALL: [Resource; 7] =
        [Resource::Meat, Resource::Fish, Resource::Shellfish, Resource::Fruit, Resource::Roots, Resource::Insects, Resource::Eggs];

    pub fn label(self) -> &'static str {
        match self {
            Resource::Meat => "la viande",
            Resource::Fish => "le poisson",
            Resource::Shellfish => "les coquillages",
            Resource::Fruit => "les fruits",
            Resource::Roots => "les racines",
            Resource::Insects => "les insectes",
            Resource::Eggs => "les œufs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitKind {
    Technique(Family),
    Belief,
    Rite,
    Art,
    Taboo(Resource),
    /// Mythe, par son identifiant dans le registre des mythes.
    Myth(u32),
}

impl TraitKind {
    /// Niche de concurrence d'un trait généré, avec son objet.
    pub fn niche(self, topic: u8) -> Option<u16> {
        let k: u16 = match self {
            TraitKind::Technique(_) => return None,
            TraitKind::Belief => 1,
            TraitKind::Rite => 2,
            TraitKind::Art => 3,
            TraitKind::Taboo(r) => return Some(4 << 8 | r as u16),
            TraitKind::Myth(_) => 5,
        };
        // Quatre niches par genre : les objets se partagent entre elles.
        Some(k << 8 | (topic % 4) as u16)
    }

    pub fn generated(self) -> bool {
        !matches!(self, TraitKind::Technique(_))
    }

    pub fn label(self) -> &'static str {
        match self {
            TraitKind::Technique(_) => "technique",
            TraitKind::Belief => "croyance",
            TraitKind::Rite => "rite",
            TraitKind::Art => "art",
            TraitKind::Taboo(_) => "tabou",
            TraitKind::Myth(_) => "mythe",
        }
    }
}

/// Effets bornés d'un trait.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Effects {
    pub cohesion: f64,
    pub cooperation: f64,
    /// Légitimité : sans effet avant les institutions de l'étape 7.
    pub legitimacy: f64,
}

impl Effects {
    pub fn bounded(self) -> Effects {
        let b = |x: f64| x.clamp(-EFFECT_BOUND, EFFECT_BOUND);
        Effects { cohesion: b(self.cohesion), cooperation: b(self.cooperation), legitimacy: b(self.legitimacy) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitDef {
    pub id: TraitId,
    pub kind: TraitKind,
    pub name: String,
    pub effects: Effects,
    /// Attrait relatif (biais de contenu) : multiplie le taux de
    /// transmission.
    pub appeal: f64,
    /// Objet du trait : les traits générés d'un même genre et d'un même
    /// objet se font concurrence (chacun n'a qu'un rite des morts).
    pub topic: u8,
    pub origin_year: f64,
    pub origin_cell: u32,
    pub origin_species: u32,
}

/// « du Kalo », « de l'Ama ».
pub fn du(word: &str) -> String {
    let w = crate::language::capitalize(word);
    if w.starts_with(['A', 'E', 'I', 'O', 'U', 'É', 'È', 'H']) {
        format!("de l'{w}")
    } else {
        format!("du {w}")
    }
}

const OBJECTS: [&str; 16] = [
    "l'eau",
    "le feu",
    "les morts",
    "la lune",
    "le soleil",
    "les ancêtres",
    "la pluie",
    "les montagnes",
    "la mer",
    "les étoiles",
    "le vent",
    "les grands animaux",
    "les rêves",
    "la naissance",
    "la chasse",
    "le passage des saisons",
];

/// Tire le contenu d'un trait généré du genre demandé.
pub fn generate<R: Rng>(rng: &mut R, kind: TraitKind, phonology: &Phonology) -> (String, Effects, f64, u8) {
    let topic = rng.random_range(0..OBJECTS.len());
    let object = OBJECTS[topic];
    let word = du(&phonology.word(rng.random()));
    let e = |rng: &mut R, lo: f64, hi: f64| rng.random_range(lo..hi);
    let (name, effects) = match kind {
        TraitKind::Belief => (
            format!("croyance {word} : l'esprit habite {object}"),
            Effects { cohesion: e(rng, 0.0, 0.06), cooperation: e(rng, -0.02, 0.04), legitimacy: e(rng, 0.0, 0.05) },
        ),
        TraitKind::Rite => (
            format!("rite {word}, pour {object}"),
            Effects { cohesion: e(rng, 0.02, 0.1), cooperation: e(rng, 0.0, 0.05), legitimacy: e(rng, 0.0, 0.03) },
        ),
        TraitKind::Art => {
            let form = ["chant", "danse", "motif peint", "récit", "rythme frappé"][rng.random_range(0..5)];
            (format!("{form} {word}, qui célèbre {object}"), Effects { cohesion: e(rng, 0.0, 0.05), ..Effects::default() })
        }
        TraitKind::Taboo(r) => (
            format!("interdit {word} : ne pas toucher {}", r.label()),
            Effects { cohesion: e(rng, 0.01, 0.06), cooperation: e(rng, 0.0, 0.02), legitimacy: 0.0 },
        ),
        TraitKind::Myth(_) | TraitKind::Technique(_) => (phonology.name(rng.random()), Effects::default()),
    };
    (name, effects.bounded(), rng.random_range(0.9..1.3), topic as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::rng::{rng_for, Stream};

    #[test]
    fn generated_effects_stay_bounded() {
        let mut rng = rng_for(3, Stream::Culture, &[]);
        let ph = Phonology::random(&mut rng);
        for _ in 0..200 {
            for k in [TraitKind::Belief, TraitKind::Rite, TraitKind::Art, TraitKind::Taboo(Resource::Fish)] {
                let (name, ef, appeal, _) = generate(&mut rng, k, &ph);
                assert!(!name.is_empty());
                for x in [ef.cohesion, ef.cooperation, ef.legitimacy] {
                    assert!(x.abs() <= EFFECT_BOUND);
                }
                assert!((0.9..1.3).contains(&appeal));
            }
        }
    }
}
