//! Événements de l'histoire des sociétés, pour la chronique et le
//! narrateur, et personnages notables.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// Une technique inventée (et non reçue d'un voisin).
    Invention,
    /// Un foyer agricole indépendant.
    AgricultureHearth,
    /// Une société change d'étiquette (devient chefferie, État…).
    Label,
    SocietyFounded,
    Scission,
    Collapse,
    CrisisStart,
    CrisisEnd,
    WarStart,
    WarEnd,
    Conquest,
    Alliance,
    Tribute,
    Trade,
    Epidemic,
    NewDisease,
    Famine,
    CityFounded,
    Paradigm,
    ScienceConflict,
    NewReligion,
    /// Deux espèces intelligentes se rencontrent.
    FirstContact,
    /// Sortie du piège malthusien.
    IndustrialTakeoff,
    /// Un équivalent du XXIe siècle : fin de la première version.
    EndOfFirstVersion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub year: i64,
    pub kind: EventKind,
    pub society: Option<u32>,
    pub other: Option<u32>,
    pub cell: Option<u32>,
    pub technique: Option<u16>,
    pub notable: Option<u32>,
    pub text: String,
    /// Score d'intérêt, de 0 à 1, comme les événements du moteur.
    pub interest: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotableKind {
    Chief,
    Prophet,
    Conqueror,
    Savant,
    Inventor,
}

impl NotableKind {
    pub fn label(self) -> &'static str {
        match self {
            NotableKind::Chief => "chef",
            NotableKind::Prophet => "prophète",
            NotableKind::Conqueror => "conquérant",
            NotableKind::Savant => "savant",
            NotableKind::Inventor => "inventeur",
        }
    }
}

/// Un personnage notable : causal, tiré par le moteur quelle que soit la
/// caméra. Son effet reste borné : légitimité, cohésion, décision de
/// guerre, diffusion d'une idée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notable {
    pub id: u32,
    pub kind: NotableKind,
    pub society: u32,
    pub species: u32,
    pub name: String,
    pub born: i64,
    pub died: Option<i64>,
    /// Fin prévue de son action (mort ou retrait).
    pub until: i64,
    pub ambition: f64,
    pub skill: f64,
    /// Domaine d'un savant, technique d'un inventeur, religion d'un prophète.
    pub subject: u32,
    pub deeds: Vec<String>,
}

/// Plafond des notables vivants (doc d'architecture, étape 7).
pub const MAX_NOTABLES: usize = 5_000;
