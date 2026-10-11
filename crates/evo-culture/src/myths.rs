//! Mythes : ce qu'un peuple raconte d'un phénomène qu'il ne s'explique pas.
//!
//! Une intervention du joueur vue par un peuple est marquée inexpliquée et
//! peut faire naître un mythe, tiré avec le hasard de l'état simulé (doc
//! d'architecture, étape 6). Les catastrophes naturelles le peuvent aussi.
//! Le mythe a un nom, une figure (un être nommé dans la langue du peuple),
//! un thème et un court récit par gabarit ; il devient un trait culturel du
//! groupe, transmis comme les autres, aux effets bornés (cohésion, tabou).

use crate::language::Phonology;
use crate::traits::{Effects, Resource};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Ce qu'un peuple a vu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phenomenon {
    Rain,
    Drought,
    Cold,
    Heat,
    Eruption,
    Earthquake,
    Impact,
    Flood,
    Abundance,
    Light,
}

impl Phenomenon {
    pub const ALL: [Phenomenon; 10] = [
        Phenomenon::Rain,
        Phenomenon::Drought,
        Phenomenon::Cold,
        Phenomenon::Heat,
        Phenomenon::Eruption,
        Phenomenon::Earthquake,
        Phenomenon::Impact,
        Phenomenon::Flood,
        Phenomenon::Abundance,
        Phenomenon::Light,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Phenomenon::Rain => "une pluie venue de nulle part",
            Phenomenon::Drought => "une sécheresse soudaine",
            Phenomenon::Cold => "un froid sans saison",
            Phenomenon::Heat => "une chaleur sans saison",
            Phenomenon::Eruption => "une montagne en feu",
            Phenomenon::Earthquake => "un tremblement de terre",
            Phenomenon::Impact => "une pierre tombée du ciel",
            Phenomenon::Flood => "une montée des eaux",
            Phenomenon::Abundance => "une abondance inattendue",
            Phenomenon::Light => "une lumière dans le ciel",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Theme {
    Anger,
    Gift,
    Punishment,
    Origin,
    Guardian,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Anger => "colère",
            Theme::Gift => "don",
            Theme::Punishment => "châtiment",
            Theme::Origin => "origine",
            Theme::Guardian => "gardien",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Myth {
    pub id: u32,
    /// Peuple qui l'a vu naître.
    pub people: u32,
    pub species: u32,
    pub cell: u32,
    pub year: f64,
    pub phenomenon: Phenomenon,
    /// Né d'une intervention du joueur.
    pub intervention: bool,
    pub theme: Theme,
    /// L'être du mythe, nommé dans la langue du peuple.
    pub figure: String,
    pub title: String,
    pub text: String,
    pub effects: Effects,
    pub taboo: Option<Resource>,
}

/// Probabilité qu'un phénomène inexpliqué fasse naître un mythe, selon la
/// pensée symbolique de l'espèce. Nulle en dessous de 0,25 : le phénomène
/// est seulement noté inexpliqué.
pub fn myth_chance(symbolic: f64) -> f64 {
    if symbolic < 0.25 {
        0.0
    } else {
        (0.3 + 0.7 * symbolic).min(1.0)
    }
}

/// Tire un mythe.
#[allow(clippy::too_many_arguments)]
pub fn generate<R: Rng>(
    rng: &mut R,
    id: u32,
    people: u32,
    people_name: &str,
    species: u32,
    cell: u32,
    year: f64,
    phenomenon: Phenomenon,
    intervention: bool,
    phonology: &Phonology,
) -> Myth {
    let figure = phonology.name(rng.random());
    let theme = match phenomenon {
        Phenomenon::Rain | Phenomenon::Abundance => [Theme::Gift, Theme::Guardian, Theme::Origin][rng.random_range(0..3)],
        Phenomenon::Light | Phenomenon::Impact => [Theme::Origin, Theme::Anger, Theme::Gift][rng.random_range(0..3)],
        _ => [Theme::Anger, Theme::Punishment, Theme::Guardian][rng.random_range(0..3)],
    };
    let what = phenomenon.label();
    let text = match theme {
        Theme::Anger => format!("Quand {figure} s'irrita, il y eut {what}, et les {people_name} apprirent à baisser la voix."),
        Theme::Gift => format!("{figure} eut pitié des {people_name} et leur envoya {what} ; depuis, on lui laisse une part."),
        Theme::Punishment => {
            format!("Les {people_name} avaient oublié {figure}. Alors vint {what}, et ceux qui se souvinrent survécurent.")
        }
        Theme::Origin => format!("Au commencement il y eut {what} : c'est {figure} qui ouvrit le monde aux {people_name}."),
        Theme::Guardian => format!("{figure} veille sur les {people_name} ; {what} est le signe de son passage."),
    };
    let title = match theme {
        Theme::Anger => format!("La colère de {figure}"),
        Theme::Gift => format!("Le don de {figure}"),
        Theme::Punishment => format!("L'oubli de {figure}"),
        Theme::Origin => format!("Le commencement selon {figure}"),
        Theme::Guardian => format!("{figure} le gardien"),
    };
    let taboo = if matches!(theme, Theme::Punishment | Theme::Anger) && rng.random_bool(0.4) {
        Some(Resource::ALL[rng.random_range(0..Resource::ALL.len())])
    } else {
        None
    };
    let text = match taboo {
        Some(r) => format!("{text} Il ne faut plus toucher {}.", r.label()),
        None => text,
    };
    let effects = Effects {
        cohesion: rng.random_range(0.02..0.08),
        cooperation: rng.random_range(0.0..0.03),
        legitimacy: rng.random_range(0.0..0.05),
    }
    .bounded();
    Myth { id, people, species, cell, year, phenomenon, intervention, theme, figure, title, text, effects, taboo }
}
