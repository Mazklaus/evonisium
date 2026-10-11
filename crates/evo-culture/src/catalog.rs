//! Catalogue des techniques de l'étape 6, écrit à la main.
//!
//! Chaque technique est une famille paramétrée : une voie ordinaire (sur
//! terre, avec des mains) et ses voies de remplacement pour les espèces
//! aquatiques, souterraines, sans mains, ou dans un air trop pauvre en
//! oxygène pour le feu. Une voie absente veut dire que la technique n'existe
//! pas dans ce contexte (« des mains ou rien » pour la taille de la pierre).
//!
//! La difficulté est en unités de savoir-faire du modèle de Henrich (2004) :
//! une technique se maintient dans un groupe de n individus connectés si
//! γ + ln n dépasse difficulté × (1 − fidélité). Les valeurs sont calées
//! pour que le cas tasmanien sorte (outils d'os, vêtements cousus, pêche,
//! propulseurs perdus à 4 000 individus, gardés à 100 000), et qu'un
//! chimpanzé garde ses bâtons et ses marteaux sans tailler la pierre.

use crate::cognition::{Cognition, Learning, Organ};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Family {
    Probe,
    Percussion,
    Flake,
    Handaxe,
    Cord,
    Shelter,
    Container,
    Fire,
    Cooking,
    Ornament,
    Spear,
    Raft,
    Thrower,
    BoneTool,
    Clothing,
    Fishing,
}

impl Family {
    pub const ALL: [Family; 16] = [
        Family::Probe,
        Family::Percussion,
        Family::Flake,
        Family::Handaxe,
        Family::Cord,
        Family::Shelter,
        Family::Container,
        Family::Fire,
        Family::Cooking,
        Family::Ornament,
        Family::Spear,
        Family::Raft,
        Family::Thrower,
        Family::BoneTool,
        Family::Clothing,
        Family::Fishing,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Option<Family> {
        Family::ALL.get(i).copied()
    }

    pub fn technique(self) -> &'static Technique {
        &CATALOG[self.index()]
    }
}

/// Milieu de vie d'un groupe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Milieu {
    Land,
    Water,
    Underground,
}

/// Voie d'une technique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    Standard,
    Aquatic,
    Underground,
    Handless,
    LowOxygen,
}

/// Ce qui décide de la voie : milieu, oxygène, organe de manipulation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub milieu: Milieu,
    /// Oxygène de l'air, en pour cent du volume.
    pub o2_percent: f64,
    pub organ: Organ,
}

impl Context {
    pub fn land(organ: Organ) -> Context {
        Context { milieu: Milieu::Land, o2_percent: 21.0, organ }
    }
}

/// En dessous de ce taux d'oxygène, rien ne brûle : les premiers charbons
/// fossiles datent du moment où l'oxygène a passé environ 15 à 17 %.
pub const FIRE_O2_PERCENT: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    pub route: Route,
    pub name: &'static str,
    pub difficulty: f64,
    /// Qualité minimale de l'organe de manipulation.
    pub manipulation: f64,
    /// Causalité physique et planification minimales.
    pub causality: f64,
    pub planning: f64,
    pub symbolic: f64,
    /// Demande une main (sinon la voie sans mains s'applique, si elle existe).
    pub hands: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Technique {
    pub family: Family,
    pub prerequisites: &'static [Family],
    /// Brûle : impossible sous l'eau et sous [`FIRE_O2_PERCENT`].
    pub combustion: bool,
    /// Gain de ressources relatif (biais de succès).
    pub benefit: f64,
    pub variants: &'static [Variant],
    /// Référence terrestre.
    pub reference: &'static str,
}

const fn v(route: Route, name: &'static str, difficulty: f64, manipulation: f64, causality: f64, planning: f64, hands: bool) -> Variant {
    Variant { route, name, difficulty, manipulation, causality, planning, symbolic: 0.0, hands }
}

use Family as F;
use Route as R;

pub static CATALOG: [Technique; 16] = [
    Technique {
        family: F::Probe,
        prerequisites: &[],
        combustion: false,
        benefit: 0.15,
        variants: &[
            v(R::Standard, "bâton à sonder", 8.0, 0.3, 0.25, 0.15, true),
            v(R::Handless, "brindille tenue au bec", 8.0, 0.3, 0.3, 0.2, false),
            v(R::Aquatic, "éponge protectrice", 6.0, 0.15, 0.25, 0.2, false),
            v(R::Underground, "sonde de racine", 8.0, 0.3, 0.25, 0.15, false),
        ],
        reference: "pêche aux termites des chimpanzés ; outils des corbeaux de Nouvelle-Calédonie ; éponges des dauphins de Shark Bay",
    },
    Technique {
        family: F::Percussion,
        prerequisites: &[],
        combustion: false,
        benefit: 0.2,
        variants: &[
            v(R::Standard, "marteau et enclume", 9.0, 0.4, 0.3, 0.2, true),
            v(R::Handless, "noix lâchées sur la pierre", 9.0, 0.2, 0.35, 0.25, false),
            v(R::Aquatic, "coquillage brisé sur pierre", 8.0, 0.3, 0.3, 0.2, false),
            v(R::Underground, "marteau de galerie", 9.0, 0.4, 0.3, 0.2, false),
        ],
        reference: "casse-noix des chimpanzés de Taï ; loutres de mer",
    },
    Technique {
        family: F::Flake,
        prerequisites: &[F::Percussion],
        combustion: false,
        benefit: 0.3,
        variants: &[
            v(R::Standard, "éclat de pierre taillé", 40.0, 0.6, 0.45, 0.3, true),
            v(R::Handless, "éclat taillé à la trompe", 50.0, 0.7, 0.5, 0.35, false),
            v(R::Aquatic, "coquille aiguisée", 35.0, 0.5, 0.45, 0.3, false),
            v(R::Underground, "éclat de pierre taillé", 40.0, 0.6, 0.45, 0.3, false),
        ],
        reference: "Lomekwi (3,3 Ma), Oldowayen (2,6 Ma)",
    },
    Technique {
        family: F::Handaxe,
        prerequisites: &[F::Flake],
        combustion: false,
        benefit: 0.25,
        variants: &[
            v(R::Standard, "biface", 120.0, 0.7, 0.55, 0.5, true),
            v(R::Handless, "biface façonné à la trompe", 140.0, 0.75, 0.55, 0.5, false),
            v(R::Underground, "biface", 120.0, 0.7, 0.55, 0.5, false),
        ],
        reference: "Acheuléen (1,76 Ma)",
    },
    Technique {
        family: F::Cord,
        prerequisites: &[],
        combustion: false,
        benefit: 0.15,
        variants: &[
            v(R::Standard, "corde de fibres", 90.0, 0.6, 0.5, 0.45, true),
            v(R::Handless, "fibres nouées", 110.0, 0.7, 0.5, 0.45, false),
            v(R::Aquatic, "corde d'algues tressées", 90.0, 0.6, 0.5, 0.45, false),
            v(R::Underground, "corde de racines", 90.0, 0.6, 0.5, 0.45, false),
        ],
        reference: "fibre torsadée d'Abri du Maras (environ 50 ka)",
    },
    Technique {
        family: F::Shelter,
        prerequisites: &[],
        combustion: false,
        benefit: 0.15,
        variants: &[
            v(R::Standard, "abri de branchages", 25.0, 0.4, 0.35, 0.35, false),
            v(R::Aquatic, "abri de pierres empilées", 25.0, 0.35, 0.35, 0.35, false),
            v(R::Underground, "galerie aménagée", 20.0, 0.25, 0.3, 0.35, false),
        ],
        reference: "nids des grands singes ; huttes de Terra Amata (discutées)",
    },
    Technique {
        family: F::Container,
        prerequisites: &[],
        combustion: false,
        benefit: 0.2,
        variants: &[
            v(R::Standard, "contenant de peau ou d'écorce", 50.0, 0.5, 0.45, 0.45, true),
            v(R::Handless, "coque portée", 50.0, 0.4, 0.45, 0.45, false),
            v(R::Aquatic, "coquille porte-vivres", 45.0, 0.45, 0.45, 0.45, false),
            v(R::Underground, "réserve creusée", 35.0, 0.3, 0.4, 0.5, false),
        ],
        reference: "œufs d'autruche gravés de Diepkloof (60 ka)",
    },
    Technique {
        family: F::Fire,
        prerequisites: &[],
        combustion: true,
        benefit: 0.4,
        variants: &[
            v(R::Standard, "feu entretenu", 80.0, 0.4, 0.5, 0.45, false),
            v(R::Underground, "foyer de grotte", 80.0, 0.4, 0.5, 0.45, false),
            v(R::Aquatic, "cuisson aux sources chaudes", 60.0, 0.3, 0.5, 0.45, false),
            v(R::LowOxygen, "fermentation en fosse", 60.0, 0.3, 0.45, 0.5, false),
        ],
        reference: "Wonderwerk (1 Ma), Gesher Benot Ya'aqov (780 ka)",
    },
    Technique {
        family: F::Cooking,
        prerequisites: &[F::Fire, F::Container],
        combustion: false,
        benefit: 0.3,
        variants: &[
            v(R::Standard, "cuisson des aliments", 60.0, 0.4, 0.5, 0.5, false),
            v(R::Aquatic, "aliments attendris à la source", 55.0, 0.3, 0.5, 0.5, false),
            v(R::Underground, "cuisson des aliments", 60.0, 0.4, 0.5, 0.5, false),
            v(R::LowOxygen, "aliments prédigérés", 55.0, 0.3, 0.45, 0.5, false),
        ],
        reference: "hypothèse de la cuisson (Wrangham)",
    },
    Technique {
        family: F::Ornament,
        prerequisites: &[],
        combustion: false,
        benefit: 0.0,
        variants: &[
            Variant { symbolic: 0.5, ..v(R::Standard, "parure de coquillages percés", 70.0, 0.5, 0.3, 0.3, true) },
            Variant { symbolic: 0.5, ..v(R::Handless, "parure de pigments", 70.0, 0.3, 0.3, 0.3, false) },
            Variant { symbolic: 0.5, ..v(R::Aquatic, "parure de nacre", 70.0, 0.4, 0.3, 0.3, false) },
            Variant { symbolic: 0.5, ..v(R::Underground, "parure de pigments", 70.0, 0.3, 0.3, 0.3, false) },
        ],
        reference: "perles de Blombos et de Grotte des Pigeons (environ 80 ka)",
    },
    Technique {
        family: F::Spear,
        prerequisites: &[F::Flake],
        combustion: false,
        benefit: 0.35,
        variants: &[
            v(R::Standard, "épieu", 150.0, 0.6, 0.55, 0.55, true),
            v(R::Aquatic, "harpon", 160.0, 0.6, 0.55, 0.55, true),
            v(R::Underground, "épieu", 150.0, 0.6, 0.55, 0.55, true),
        ],
        reference: "épieux de Schöningen (300 ka)",
    },
    Technique {
        family: F::Raft,
        prerequisites: &[F::Cord],
        combustion: false,
        benefit: 0.2,
        variants: &[v(R::Standard, "radeau", 250.0, 0.6, 0.6, 0.65, true)],
        reference: "peuplement de l'Australie (environ 65 ka)",
    },
    Technique {
        family: F::Thrower,
        prerequisites: &[F::Spear],
        combustion: false,
        benefit: 0.3,
        variants: &[
            v(R::Standard, "propulseur", 370.0, 0.75, 0.65, 0.65, true),
            v(R::Underground, "propulseur", 370.0, 0.75, 0.65, 0.65, true),
        ],
        reference: "propulseurs du Paléolithique supérieur ; perdus en Tasmanie",
    },
    Technique {
        family: F::BoneTool,
        prerequisites: &[F::Flake],
        combustion: false,
        benefit: 0.2,
        variants: &[
            v(R::Standard, "poinçon d'os", 360.0, 0.7, 0.6, 0.6, true),
            v(R::Aquatic, "poinçon d'arête", 360.0, 0.7, 0.6, 0.6, true),
            v(R::Underground, "poinçon d'os", 360.0, 0.7, 0.6, 0.6, true),
        ],
        reference: "outils d'os perdus en Tasmanie vers 3,5 ka (Henrich, 2004)",
    },
    Technique {
        family: F::Clothing,
        prerequisites: &[F::BoneTool, F::Container],
        combustion: false,
        benefit: 0.25,
        variants: &[
            v(R::Standard, "vêtement cousu", 390.0, 0.75, 0.6, 0.65, true),
            v(R::Underground, "vêtement cousu", 390.0, 0.75, 0.6, 0.65, true),
        ],
        reference: "aiguilles à chas (environ 40 ka) ; vêtements contre le froid perdus en Tasmanie",
    },
    Technique {
        family: F::Fishing,
        prerequisites: &[F::Cord, F::BoneTool],
        combustion: false,
        benefit: 0.3,
        variants: &[
            v(R::Standard, "filet et hameçon", 400.0, 0.7, 0.6, 0.7, true),
            v(R::Aquatic, "filet", 390.0, 0.65, 0.6, 0.7, true),
        ],
        reference: "pêche des poissons à écailles abandonnée en Tasmanie",
    },
];

impl Technique {
    /// Voie de la technique dans ce contexte, si elle existe.
    pub fn variant(&self, ctx: &Context) -> Option<&'static Variant> {
        let all: &'static [Variant] = CATALOG[self.family.index()].variants;
        let find = |r: Route| all.iter().find(|v| v.route == r);
        if self.combustion && ctx.milieu != Milieu::Water && ctx.o2_percent < FIRE_O2_PERCENT {
            return find(Route::LowOxygen);
        }
        let base = match ctx.milieu {
            Milieu::Water => find(Route::Aquatic)?,
            Milieu::Underground => find(Route::Underground).or_else(|| find(Route::Standard))?,
            Milieu::Land => find(Route::Standard)?,
        };
        if base.hands && ctx.organ.handless() {
            return find(Route::Handless).filter(|_| ctx.milieu == Milieu::Land);
        }
        Some(base)
    }

    /// La voie existe et le corps la permet.
    pub fn feasible(&self, ctx: &Context, cog: &Cognition, learning: &Learning) -> Option<&'static Variant> {
        let v = self.variant(ctx)?;
        let cap = cog.bounded();
        let ok = cog.manipulation.organ != Organ::None
            && cog.manipulation.quality >= v.manipulation
            && cap[crate::cognition::Capacity::PhysicalCausality as usize] >= v.causality
            && cap[crate::cognition::Capacity::Planning as usize] >= v.planning
            && learning.symbolic >= v.symbolic;
        ok.then_some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::presets;

    #[test]
    fn catalog_is_in_family_order() {
        for (i, t) in CATALOG.iter().enumerate() {
            assert_eq!(t.family.index(), i);
            assert!(!t.variants.is_empty());
            for p in t.prerequisites {
                assert!(p.index() < i, "{:?} demande {:?}, plus loin dans le catalogue", t.family, p);
            }
        }
    }

    #[test]
    fn replacement_routes() {
        let water = Context { milieu: Milieu::Water, o2_percent: 21.0, organ: Organ::Tentacle };
        assert_eq!(F::Fire.technique().variant(&water).unwrap().route, Route::Aquatic);
        assert!(F::Raft.technique().variant(&water).is_none());
        let thin = Context { o2_percent: 12.0, ..Context::land(Organ::Hand) };
        assert_eq!(F::Fire.technique().variant(&thin).unwrap().route, Route::LowOxygen);
        let beak = Context::land(Organ::Beak);
        assert_eq!(F::Probe.technique().variant(&beak).unwrap().route, Route::Handless);
        assert!(F::Spear.technique().variant(&beak).is_none(), "des mains ou rien");
        let crow = presets::crow();
        let l = Learning::of(&crow);
        assert!(F::Probe.technique().feasible(&beak, &crow, &l).is_some());
        let chimp = presets::chimpanzee();
        let l = Learning::of(&chimp);
        let land = Context::land(Organ::Hand);
        assert!(F::Percussion.technique().feasible(&land, &chimp, &l).is_some());
        assert!(F::Handaxe.technique().feasible(&land, &chimp, &l).is_none());
    }
}
