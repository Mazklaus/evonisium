//! Sciences : pour chaque domaine que le moteur simule vraiment, une société
//! a un modèle, c'est-à-dire une estimation des vrais paramètres de la
//! planète et une précision.
//!
//! La précision monte lentement par l'observation, vite par l'expérience et
//! la mesure ; sans l'instrument du domaine (lunette, microscope…), elle
//! plafonne. Un modèle dominant résiste aux anomalies jusqu'à une bascule
//! (Kuhn, 1962) : la précision avance par paliers. Une société qui sait
//! assez de choses heurte ses croyances : la tension monte quand le temple
//! est fort.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Domain {
    Mechanics,
    Chemistry,
    Climate,
    Geology,
    Biology,
    Heredity,
    Disease,
    Astronomy,
}

pub const DOMAINS: usize = 8;

impl Domain {
    pub const ALL: [Domain; DOMAINS] = [
        Domain::Mechanics,
        Domain::Chemistry,
        Domain::Climate,
        Domain::Geology,
        Domain::Biology,
        Domain::Heredity,
        Domain::Disease,
        Domain::Astronomy,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key(self) -> &'static str {
        match self {
            Domain::Mechanics => "mecanique",
            Domain::Chemistry => "chimie",
            Domain::Climate => "climat",
            Domain::Geology => "geologie",
            Domain::Biology => "biologie",
            Domain::Heredity => "heredite",
            Domain::Disease => "maladies",
            Domain::Astronomy => "astronomie",
        }
    }

    pub fn from_key(k: &str) -> Option<Domain> {
        Domain::ALL.iter().copied().find(|d| d.key() == k)
    }

    pub fn label(self) -> &'static str {
        match self {
            Domain::Mechanics => "mécanique",
            Domain::Chemistry => "chimie",
            Domain::Climate => "climat",
            Domain::Geology => "géologie",
            Domain::Biology => "biologie",
            Domain::Heredity => "hérédité",
            Domain::Disease => "maladies",
            Domain::Astronomy => "astronomie",
        }
    }

    /// Précision atteinte sans l'instrument du domaine : l'œil nu suffit
    /// pour les saisons et le mouvement des astres, pas pour les microbes.
    pub fn naked_eye(self) -> f64 {
        match self {
            Domain::Mechanics => 0.45,
            Domain::Chemistry => 0.3,
            Domain::Climate => 0.35,
            Domain::Geology => 0.3,
            Domain::Biology => 0.4,
            Domain::Heredity => 0.2,
            Domain::Disease => 0.15,
            Domain::Astronomy => 0.45,
        }
    }

    /// Ce que le modèle du domaine estime, pour l'affichage.
    pub fn quantity(self) -> &'static str {
        match self {
            Domain::Mechanics => "pesanteur à la surface, m/s²",
            Domain::Chemistry => "nombre d'éléments stables",
            Domain::Climate => "sensibilité du climat au doublement du CO₂, °C",
            Domain::Geology => "âge de la planète, Ga",
            Domain::Biology => "âge de la vie, Ga",
            Domain::Heredity => "taux de mutation par génome et par génération",
            Domain::Disease => "part des morts dues aux microbes",
            Domain::Astronomy => "durée de l'année, jours",
        }
    }
}

/// Précision qu'une science « moderne » atteint dans tous les domaines.
pub const MODERN: f64 = 0.9;

/// Modèle d'un domaine dans une société.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Précision, de 0 (mythe) à 1.
    pub precision: f64,
    /// Anomalies accumulées que le modèle dominant n'explique pas.
    pub anomalies: f64,
    /// Bascules de paradigme passées.
    pub paradigm: u16,
}

impl Default for Model {
    fn default() -> Model {
        Model { precision: 0.02, anomalies: 0.0, paradigm: 0 }
    }
}

/// Estimation qu'une société fait d'une vraie valeur : l'erreur relative
/// décroît avec la précision (un facteur 3 près du mythe, quelques pour
/// mille à la science moderne). Le signe de l'erreur vient de la société.
pub fn estimate(truth: f64, precision: f64, bias: f64) -> f64 {
    let miss = 1.0 - precision.clamp(0.0, 1.0);
    let err = miss * miss * 2.0;
    truth * (1.0 + err * bias.clamp(-1.0, 1.0))
}

/// Les vraies valeurs de la planète, que les modèles approchent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Truths(pub [f64; DOMAINS]);

impl Truths {
    pub fn earth() -> Truths {
        Truths([9.81, 80.0, 3.0, 4.54, 3.8, 70.0, 0.4, 365.25])
    }
}
