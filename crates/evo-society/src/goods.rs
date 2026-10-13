//! Les 28 catégories de biens tenues par chaque cellule habitée.
//!
//! Les biens de consommation se comptent en « besoins d'une personne pour un
//! an » (une ration nourrit une personne un an ; un logement abrite une
//! personne un an), les matières en tonnes, l'électricité en mégawattheures.
//! Les valeurs de référence (en rations) ne servent qu'à mesurer la
//! production : les prix, eux, n'existent qu'avec un marché (voir
//! `economy`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Good {
    WildFood,
    Grain,
    Roots,
    Livestock,
    AnimalFood,
    Fish,
    Wood,
    Charcoal,
    Stone,
    Pottery,
    Textile,
    Leather,
    Salt,
    Copper,
    Tin,
    Bronze,
    Iron,
    Tools,
    Weapons,
    Buildings,
    Coal,
    Oil,
    Electricity,
    Machines,
    Fertilizer,
    Luxury,
    Books,
    Medicine,
}

pub const GOODS: usize = 28;

/// Un bilan : une quantité par bien.
pub type Goods = [f64; GOODS];

pub const ZERO: Goods = [0.0; GOODS];

impl Good {
    pub const ALL: [Good; GOODS] = [
        Good::WildFood,
        Good::Grain,
        Good::Roots,
        Good::Livestock,
        Good::AnimalFood,
        Good::Fish,
        Good::Wood,
        Good::Charcoal,
        Good::Stone,
        Good::Pottery,
        Good::Textile,
        Good::Leather,
        Good::Salt,
        Good::Copper,
        Good::Tin,
        Good::Bronze,
        Good::Iron,
        Good::Tools,
        Good::Weapons,
        Good::Buildings,
        Good::Coal,
        Good::Oil,
        Good::Electricity,
        Good::Machines,
        Good::Fertilizer,
        Good::Luxury,
        Good::Books,
        Good::Medicine,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Clé des fichiers de données.
    pub fn key(self) -> &'static str {
        match self {
            Good::WildFood => "sauvage",
            Good::Grain => "grain",
            Good::Roots => "tubercules",
            Good::Livestock => "betail",
            Good::AnimalFood => "lait_viande",
            Good::Fish => "poisson",
            Good::Wood => "bois",
            Good::Charcoal => "charbon_bois",
            Good::Stone => "pierre",
            Good::Pottery => "poterie",
            Good::Textile => "textile",
            Good::Leather => "cuir",
            Good::Salt => "sel",
            Good::Copper => "cuivre",
            Good::Tin => "etain",
            Good::Bronze => "bronze",
            Good::Iron => "fer",
            Good::Tools => "outils",
            Good::Weapons => "armes",
            Good::Buildings => "batiments",
            Good::Coal => "charbon",
            Good::Oil => "petrole",
            Good::Electricity => "electricite",
            Good::Machines => "machines",
            Good::Fertilizer => "engrais",
            Good::Luxury => "luxe",
            Good::Books => "livres",
            Good::Medicine => "medicaments",
        }
    }

    pub fn from_key(k: &str) -> Option<Good> {
        Good::ALL.iter().copied().find(|g| g.key() == k)
    }

    pub fn label(self) -> &'static str {
        match self {
            Good::WildFood => "nourriture sauvage",
            Good::Grain => "céréales",
            Good::Roots => "tubercules et légumes",
            Good::Livestock => "bétail",
            Good::AnimalFood => "lait et viande",
            Good::Fish => "poisson",
            Good::Wood => "bois",
            Good::Charcoal => "charbon de bois",
            Good::Stone => "pierre",
            Good::Pottery => "poterie",
            Good::Textile => "textiles",
            Good::Leather => "cuir",
            Good::Salt => "sel",
            Good::Copper => "cuivre",
            Good::Tin => "étain",
            Good::Bronze => "bronze",
            Good::Iron => "fer et acier",
            Good::Tools => "outils",
            Good::Weapons => "armes",
            Good::Buildings => "bâtiments",
            Good::Coal => "charbon",
            Good::Oil => "pétrole et gaz",
            Good::Electricity => "électricité",
            Good::Machines => "machines",
            Good::Fertilizer => "engrais",
            Good::Luxury => "biens de luxe",
            Good::Books => "écrits",
            Good::Medicine => "médicaments",
        }
    }

    /// Nourriture, en rations.
    pub fn is_food(self) -> bool {
        matches!(self, Good::WildFood | Good::Grain | Good::Roots | Good::AnimalFood | Good::Fish)
    }

    /// Valeur de référence en rations par unité, pour mesurer la production.
    pub fn value(self) -> f64 {
        match self {
            Good::WildFood | Good::Grain | Good::Roots => 1.0,
            Good::AnimalFood | Good::Fish => 1.3,
            Good::Livestock => 0.0,
            Good::Wood => 0.08,
            Good::Charcoal => 0.3,
            Good::Stone => 0.02,
            Good::Pottery => 0.1,
            Good::Textile => 0.25,
            Good::Leather => 0.2,
            Good::Salt => 0.5,
            Good::Copper => 3.0,
            Good::Tin => 5.0,
            Good::Bronze => 4.0,
            Good::Iron => 1.5,
            Good::Tools => 0.3,
            Good::Weapons => 0.5,
            Good::Buildings => 0.4,
            Good::Coal => 0.06,
            Good::Oil => 0.12,
            Good::Electricity => 0.04,
            Good::Machines => 1.0,
            Good::Fertilizer => 0.4,
            Good::Luxury => 1.0,
            Good::Books => 0.5,
            Good::Medicine => 0.5,
        }
    }

    /// Carbone contenu, en tonnes de carbone par unité : seuls comptent les
    /// stocks lents (forêt, fossiles) ; la nourriture vient de l'air de
    /// l'année et y retourne.
    pub fn carbon(self) -> f64 {
        match self {
            Good::Wood => 0.5,
            Good::Charcoal => 0.85,
            Good::Coal => 0.7,
            Good::Oil => 0.85,
            _ => 0.0,
        }
    }

    /// Part perdue chaque année en stock (pourriture, usure).
    pub fn decay(self) -> f64 {
        match self {
            Good::WildFood | Good::Fish | Good::AnimalFood => 1.0,
            Good::Grain => 0.25,
            Good::Roots => 0.6,
            Good::Livestock => 0.0,
            Good::Wood | Good::Charcoal => 0.15,
            Good::Electricity => 1.0,
            Good::Medicine => 0.5,
            Good::Tools | Good::Weapons | Good::Machines => 0.08,
            Good::Buildings => 0.03,
            Good::Textile | Good::Leather | Good::Pottery | Good::Books => 0.2,
            Good::Fertilizer => 0.5,
            _ => 0.02,
        }
    }
}

/// Valeur d'un bilan.
pub fn value(g: &Goods) -> f64 {
    Good::ALL.iter().map(|&k| g[k.index()] * k.value()).sum()
}

/// Rations d'un bilan.
pub fn food(g: &Goods) -> f64 {
    Good::ALL.iter().filter(|k| k.is_food()).map(|&k| g[k.index()]).sum()
}

/// Carbone d'un bilan, t C.
pub fn carbon(g: &Goods) -> f64 {
    Good::ALL.iter().map(|&k| g[k.index()] * k.carbon()).sum()
}
