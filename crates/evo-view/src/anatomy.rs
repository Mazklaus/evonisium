//! Anatomie et comparateur (document Fonctionnalités, « Comparateur
//! d'espèces » ; Rendu du vivant, palier 2) : le plan de construction d'une
//! espèce tel que le générateur d'apparence le lit, ses traits en clair, et
//! l'ancêtre commun de deux espèces.
//!
//! [Simplification] Tant que le moteur ne publie pas le plan de construction
//! des corps pluricellulaires (fil « cellules complexes »), le plan vient de
//! la forme du microbe (palier 1), traduite par `evo_morph::body::from_microbe`.

use crate::format::{self, Lang};
use crate::frame::LineageFrame;
use crate::species::traits_of;
use evo_morph::body::{self, BodyPlan, ModuleKind, Symmetry, System};
use evo_sim::observation::PopulationView;
use std::collections::HashSet;

/// Plan d'une population (la plus abondante d'une espèce, en général).
pub fn plan_for_population(p: &PopulationView, game_seed: u64) -> BodyPlan {
    body::from_microbe(&evo_morph::form(&traits_of(p), game_seed))
}

/// Ancêtre commun le plus récent de deux espèces : la première lignée
/// commune en remontant depuis la plus ancienne lignée de chacune.
pub fn common_ancestor(lineages: &[LineageFrame], a: u32, b: u32) -> Option<LineageFrame> {
    let founder = |s: u32| {
        lineages.iter().filter(|l| l.signature == s).min_by(|x, y| {
            x.extinct_years.is_some().cmp(&y.extinct_years.is_some()).then(x.born_years.total_cmp(&y.born_years)).then(x.id.cmp(&y.id))
        })
    };
    let find = |id: u32| lineages.get(id as usize).filter(|l| l.id == id).or_else(|| lineages.iter().find(|l| l.id == id));
    let chain = |start: &LineageFrame| {
        let mut out = vec![*start];
        let mut cur = *start;
        while cur.parent != cur.id && out.len() < 100_000 {
            match find(cur.parent) {
                Some(p) => {
                    out.push(*p);
                    cur = *p;
                }
                None => break,
            }
        }
        out
    };
    let ca = chain(founder(a)?);
    let cb = chain(founder(b)?);
    let ids: HashSet<u32> = ca.iter().map(|l| l.id).collect();
    cb.into_iter().find(|l| ids.contains(&l.id))
}

/// Traits du plan en clair, pour le comparateur et l'anatomie.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanTraits {
    /// Plus grande dimension, en mètres.
    pub size_m: f32,
    pub symmetry: &'static str,
    pub modules: usize,
    /// Appendices placés (membres, nageoires, flagelles, antennes), copies
    /// comprises.
    pub appendages: usize,
    pub eyes: usize,
    pub covering: &'static str,
    pub cell_types: u8,
    pub systems: Vec<System>,
}

pub fn plan_traits(plan: &BodyPlan, size_m: f32, lang: Lang) -> PlanTraits {
    let fr = lang == Lang::Fr;
    let shape = body::shape::place(plan);
    let placed = |f: &dyn Fn(&ModuleKind) -> bool| shape.bones.iter().filter(|b| f(&plan.modules[b.module as usize].kind)).count();
    let symmetry = if plan.modules.iter().any(|m| matches!(m.symmetry, Symmetry::Radial(_))) {
        if fr {
            "radiaire"
        } else {
            "radial"
        }
    } else if plan.modules.iter().any(|m| m.symmetry == Symmetry::Bilateral) {
        if fr {
            "bilatérale"
        } else {
            "bilateral"
        }
    } else if fr {
        "axiale"
    } else {
        "axial"
    };
    let covering = match plan.modules.first().map(|m| m.covering) {
        Some(body::Covering::Membrane) => ("membrane", "membrane"),
        Some(body::Covering::Wall) => ("paroi cellulaire", "cell wall"),
        Some(body::Covering::Naked) => ("peau nue", "bare skin"),
        Some(body::Covering::Mucus) => ("mucus", "mucus"),
        Some(body::Covering::Scales) => ("écailles", "scales"),
        Some(body::Covering::Hair) => ("poils", "hair"),
        Some(body::Covering::Feathers) => ("plumes", "feathers"),
        Some(body::Covering::Cuticle) => ("cuticule", "cuticle"),
        Some(body::Covering::Bark) => ("écorce", "bark"),
        Some(body::Covering::Wax) => ("cire", "wax"),
        None => ("—", "—"),
    };
    let mut systems: Vec<System> = Vec::new();
    for m in &plan.modules {
        if let ModuleKind::Organ(s) = m.kind {
            if !systems.contains(&s) {
                systems.push(s);
            }
        }
    }
    PlanTraits {
        size_m,
        symmetry,
        modules: plan.modules.len(),
        appendages: placed(&|k| matches!(k, ModuleKind::Limb | ModuleKind::Fin | ModuleKind::Flagellum | ModuleKind::Antenna)),
        eyes: placed(&|k| matches!(k, ModuleKind::Eye(_))),
        covering: if fr { covering.0 } else { covering.1 },
        cell_types: plan.cell_types,
        systems,
    }
}

/// Nom d'un appareil.
pub fn system_label(s: System, lang: Lang) -> &'static str {
    let fr = lang == Lang::Fr;
    match s {
        System::Digestive => {
            if fr {
                "digestif"
            } else {
                "digestive"
            }
        }
        System::Circulatory => {
            if fr {
                "circulatoire"
            } else {
                "circulatory"
            }
        }
        System::Nervous => {
            if fr {
                "nerveux"
            } else {
                "nervous"
            }
        }
        System::Respiratory => {
            if fr {
                "respiratoire"
            } else {
                "respiratory"
            }
        }
        System::Reproductive => {
            if fr {
                "reproducteur"
            } else {
                "reproductive"
            }
        }
        System::Skeletal => {
            if fr {
                "squelette"
            } else {
                "skeleton"
            }
        }
        System::Photosynthetic => {
            if fr {
                "membranes photosynthétiques"
            } else {
                "photosynthetic membranes"
            }
        }
        System::Storage => {
            if fr {
                "granules de réserve"
            } else {
                "storage granules"
            }
        }
    }
}

/// Longueur lisible, du micromètre au mètre.
pub fn length_text(m: f32, lang: Lang) -> String {
    let m = m as f64;
    let (v, unit) = if m < 1e-3 {
        (m * 1e6, "µm")
    } else if m < 1e-2 {
        (m * 1e3, "mm")
    } else if m < 1.0 {
        (m * 1e2, "cm")
    } else {
        (m, "m")
    };
    format!("{} {unit}", format::number_in(lang, v, if v < 10.0 { 1 } else { 0 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(id: u32, parent: u32, born: f64, sig: u32) -> LineageFrame {
        LineageFrame { id, parent, born_years: born, extinct_years: None, origin_cell: 0, signature: sig }
    }

    #[test]
    fn common_ancestor_climbs_both_trees() {
        // 0 (sig 1) → 1 (sig 2) → 3 (sig 4) ; 0 → 2 (sig 8).
        let ls = vec![l(0, 0, 0.0, 1), l(1, 0, 10.0, 2), l(2, 0, 20.0, 8), l(3, 1, 30.0, 4)];
        assert_eq!(common_ancestor(&ls, 4, 8).unwrap().id, 0);
        assert_eq!(common_ancestor(&ls, 4, 2).unwrap().id, 1);
        assert!(common_ancestor(&ls, 4, 16).is_none());
    }

    #[test]
    fn traits_of_a_random_body() {
        let plan = body::random_plan(3);
        let t = plan_traits(&plan, 0.3, Lang::Fr);
        assert_eq!(t.modules, plan.modules.len());
        assert!(t.systems.contains(&System::Digestive));
        assert_eq!(length_text(2.5e-6, Lang::Fr), "2,5 µm");
        assert_eq!(length_text(0.35, Lang::En), "35 cm");
    }
}
