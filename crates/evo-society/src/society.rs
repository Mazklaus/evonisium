//! Sociétés : six dimensions continues, une dizaine de grandeurs agrégées,
//! des institutions paramétrées, et une étiquette calculée pour
//! l'affichage.
//!
//! Les dimensions bougent sous des pressions mesurables : la taille et la
//! densité poussent vers la hiérarchie, le surplus stockable rend possibles
//! les élites et l'impôt, la menace extérieure centralise (Turchin), la
//! circonscription (une vallée fertile cernée de désert, Carneiro, 1970)
//! empêche la fuite et favorise la contrainte, l'excès d'inégalité et la
//! perte de légitimité mènent aux révoltes. Les cycles séculaires suivent la
//! théorie structurelle-démographique (Goldstone, 1991 ; Turchin et
//! Nefedov, 2009) : misère populaire, surproduction des élites, crise
//! fiscale ; leur produit, l'indicateur de tension politique, déclenche les
//! crises.

use crate::catalog::Known;
use crate::science::{Model, DOMAINS};
use evo_culture::language::Phonology;
use serde::{Deserialize, Serialize};

/// Les six dimensions, de 0 à 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dimensions {
    pub centralisation: f64,
    pub inequality: f64,
    pub coercion: f64,
    pub participation: f64,
    pub admin: f64,
    pub cohesion: f64,
}

/// Les grandeurs agrégées.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Aggregates {
    pub population: f64,
    /// Part des élites dans la population.
    pub elites: f64,
    /// Dette publique, en rations.
    pub debt: f64,
    pub legitimacy: f64,
    /// Surplus de l'année, en rations.
    pub surplus: f64,
    /// Techniques connues.
    pub techniques: u32,
    pub revenue: f64,
    pub expenses: f64,
    pub gdp: f64,
    /// Rations par personne (moyenne des cellules).
    pub food_pc: f64,
    pub military: f64,
    /// Indicateur de tension politique (Goldstone).
    pub stress: f64,
    pub cells: u32,
    pub urban: f64,
    pub literacy: f64,
    pub modern_energy: f64,
}

/// Institutions, modules paramétrés.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Institution {
    Council,
    Ruler,
    Court,
    Temple,
    Army,
    Market,
    School,
}

pub const INSTITUTIONS: usize = 7;

impl Institution {
    pub const ALL: [Institution; INSTITUTIONS] = [
        Institution::Council,
        Institution::Ruler,
        Institution::Court,
        Institution::Temple,
        Institution::Army,
        Institution::Market,
        Institution::School,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Institution::Council => "conseil",
            Institution::Ruler => "souverain",
            Institution::Court => "tribunal",
            Institution::Temple => "temple",
            Institution::Army => "armée",
            Institution::Market => "marché",
            Institution::School => "école",
        }
    }
}

/// Étiquette d'affichage (typologie de Service, repère et non échelle).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Label {
    Bands,
    Tribe,
    Chiefdom,
    State,
    Empire,
    ModernState,
}

impl Label {
    pub fn label(self) -> &'static str {
        match self {
            Label::Bands => "bandes",
            Label::Tribe => "tribu",
            Label::Chiefdom => "chefferie",
            Label::State => "État",
            Label::Empire => "empire",
            Label::ModernState => "État moderne",
        }
    }
    /// Rang de complexité (une perte de rang est un effondrement).
    pub fn rank(self) -> u8 {
        match self {
            Label::Bands => 0,
            Label::Tribe => 1,
            Label::Chiefdom => 2,
            Label::State | Label::Empire => 3,
            Label::ModernState => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Society {
    pub id: u32,
    pub parent: Option<u32>,
    pub species: u32,
    /// Peuple dominant (étiquette d'evo-culture).
    pub people: u32,
    pub phonology: Phonology,
    /// Forme du nom dans la langue (voir `Phonology::name`).
    pub name_form: u32,
    pub founded: i64,
    pub ended: Option<i64>,
    pub capital: u32,
    pub cells: Vec<u32>,
    pub dims: Dimensions,
    pub agg: Aggregates,
    pub institutions: [f64; INSTITUTIONS],
    pub label: Label,
    pub sciences: [Model; DOMAINS],
    /// Signe de l'erreur de chaque modèle (pour l'estimation affichée).
    pub bias: [f64; DOMAINS],
    pub religion: u32,
    /// Années de crise en cours (0 : pas de crise).
    pub crisis: u16,
    /// État d'avant la crise : étiquette et population.
    pub before_crisis: Option<(Label, f64, u32)>,
    pub known: Known,
    pub chief: Option<u32>,
    /// Années de guerre récentes, victoires et défaites.
    pub victories: u16,
    pub defeats: u16,
    pub peak: Label,
}

impl Society {
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }

    pub fn name(&self) -> String {
        self.phonology.name(self.name_form)
    }

    pub fn institution(&self, i: Institution) -> f64 {
        self.institutions[i as usize]
    }
}

/// Étiquette à partir des dimensions et de la taille.
pub fn label(dims: &Dimensions, agg: &Aggregates, sedentary: bool, peoples: usize, modern: bool) -> Label {
    let c = dims.centralisation;
    if !sedentary && c < 0.25 {
        return Label::Bands;
    }
    if c < 0.32 || agg.population < 3_000.0 {
        return Label::Tribe;
    }
    // L'État naît avec l'écriture de ses comptes (Uruk, vers −3300) : une
    // administration de 0,15 suffit, ce que donne l'écriture seule.
    if c < 0.5 || dims.admin < 0.15 || agg.population < 30_000.0 || dims.inequality < 0.25 {
        return Label::Chiefdom;
    }
    if modern && dims.admin >= 0.75 && dims.participation >= 0.45 {
        return Label::ModernState;
    }
    if (peoples >= 3 || agg.cells >= 8) && agg.population >= 1_000_000.0 {
        return Label::Empire;
    }
    Label::State
}
