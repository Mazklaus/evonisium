//! Anatomie et comparateur (document Fonctionnalités, « Comparateur
//! d'espèces » ; Rendu du vivant, palier 2) : le plan de construction d'une
//! espèce tel que le générateur d'apparence le lit, ses traits en clair, et
//! l'ancêtre commun de deux espèces.
//!
//! Le plan simulé (`evo_life::BodyPlan`, fil « cellules complexes ») arrive
//! avec chaque espèce ; [`from_simulated`] le traduit. Les procaryotes gardent
//! leurs formes du palier 1 (`evo_morph::body::from_microbe`), plus variées que
//! la cellule ronde du plan simulé.

use crate::format::{self, Lang};
use crate::frame::LineageFrame;
use crate::species::traits_of;
use evo_life::body as sim;
use evo_morph::body::{self, BodyPlan, Covering, Joint, Module, ModuleKind, Pattern, Symmetry, System, PLAN_VERSION};
use evo_morph::canvas::{mix64, DrawRng};
use evo_sim::history::{Organisation, SpeciesView};
use evo_sim::observation::PopulationView;
use std::collections::HashSet;

/// Plan d'une population (la plus abondante d'une espèce, en général),
/// d'après sa seule forme de microbe.
pub fn plan_for_population(p: &PopulationView, game_seed: u64) -> BodyPlan {
    body::from_microbe(&evo_morph::form(&traits_of(p), game_seed))
}

/// Plan d'une espèce : le plan simulé pour les eucaryotes et les colonies,
/// la forme du palier 1 pour les procaryotes. `None` sans plan ni
/// population.
pub fn plan_for_species(s: &SpeciesView, population: Option<&PopulationView>, game_seed: u64) -> Option<BodyPlan> {
    let seed = mix64(game_seed ^ (s.signature as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let simulated = s.body_plan.as_deref();
    let single_cell = simulated.is_none_or(|p| p.modules.first().is_none_or(|m| m.kind == sim::ModuleKind::Cell));
    match (simulated, population) {
        (Some(p), _) if !single_cell || s.organisation.eukaryote => Some(from_simulated(p, &s.organisation, seed)),
        (_, Some(pop)) => Some(plan_for_population(pop, game_seed)),
        (Some(p), None) => Some(from_simulated(p, &s.organisation, seed)),
        (None, None) => None,
    }
}

/// Couleur d'un pigment, pâlie selon sa concentration.
fn pigment_rgb(p: &sim::PigmentPatch) -> [u8; 3] {
    let c = evo_life::pigment_colour(p.absorption_nm);
    let k = (0.55 + 0.45 * p.concentration.clamp(0.0, 1.0)) as f32;
    let pale = [222.0f32, 212.0, 186.0];
    [0, 1, 2].map(|i| (pale[i] + (c[i] as f32 - pale[i]) * k).round() as u8)
}

fn covering(c: sim::Covering) -> Covering {
    match c {
        sim::Covering::Membrane => Covering::Membrane,
        sim::Covering::Mucus => Covering::Mucus,
        sim::Covering::Cuticle => Covering::Cuticle,
        sim::Covering::Scales => Covering::Scales,
        sim::Covering::Hair => Covering::Hair,
        sim::Covering::Feathers => Covering::Feathers,
        sim::Covering::Bark => Covering::Bark,
        sim::Covering::Wax => Covering::Wax,
    }
}

/// Appareil d'un type cellulaire, d'après ses voies et son rôle.
fn system_of(t: &sim::CellType) -> System {
    let light = evo_life::metabolism::REACTIONS.iter().filter(|r| r.is_light()).fold(0u32, |a, r| a | (1 << r.id));
    if t.pathways & light != 0 {
        System::Photosynthetic
    } else if t.role == "nourricière" {
        System::Digestive
    } else {
        System::Storage
    }
}

fn degrees(v: f64) -> f32 {
    v.to_degrees() as f32
}

/// Traduit un module simulé quelconque (segments, appendices, organes : le
/// format les prévoit, les étapes suivantes les produiront).
fn generic_module(m: &sim::Module, plan: &sim::BodyPlan) -> Module {
    let d = &m.dimensions;
    let kind = match m.kind {
        sim::ModuleKind::Cell => ModuleKind::Head,
        sim::ModuleKind::Body => ModuleKind::Trunk,
        sim::ModuleKind::Segment => ModuleKind::Segment,
        sim::ModuleKind::Appendage => ModuleKind::Limb,
        sim::ModuleKind::Leaf => ModuleKind::Leaf,
        sim::ModuleKind::Organ => {
            let t = m.cell_types.first().and_then(|&i| plan.cell_types.get(i as usize));
            ModuleKind::Organ(t.map_or(System::Storage, system_of))
        }
    };
    let flat = matches!(kind, ModuleKind::Leaf);
    let radius = if flat { d.thickness_m / 2.0 } else { d.width_m / 2.0 }.max(1e-9) as f32;
    let mut out = Module::new(kind, d.length_m.max(0.0) as f32, radius);
    out.width_m = if flat { d.width_m as f32 } else { 0.0 };
    // Attache : le long de l'axe du parent (x), puis autour de lui (y vers
    // le dos, z vers la droite).
    out.attach = m.attach[0].clamp(0.0, 1.0) as f32;
    out.around_deg = degrees(m.attach[2].atan2(m.attach[1]));
    let len = (m.axis[0] * m.axis[0] + m.axis[1] * m.axis[1] + m.axis[2] * m.axis[2]).sqrt().max(1e-12);
    out.elevation_deg = degrees((m.axis[0] / len).clamp(-1.0, 1.0).acos());
    out.symmetry = match m.symmetry.kind {
        sim::SymmetryKind::Radial(n) => Symmetry::Radial(n),
        sim::SymmetryKind::Bilateral => Symmetry::Bilateral,
        _ if m.symmetry.copies > 1 => Symmetry::Segmental(m.symmetry.copies.min(64) as u8),
        _ => Symmetry::None,
    };
    out.joint = m.joints.first().and_then(|j| {
        let dof = match j.kind {
            sim::JointKind::Rigid => return None,
            sim::JointKind::Hinge => 1,
            sim::JointKind::Flexible => 2,
            sim::JointKind::Ball => 3,
        };
        Some(Joint { dof, range_deg: degrees(j.amplitude_rad) })
    });
    out.covering = covering(m.covering);
    out.pigments = sorted_pigments(m);
    out.iridescence = m.structural_colour.map_or(0.0, |c| c.regularity.clamp(0.0, 1.0) as f32);
    out
}

/// Pigments d'un module, de la surface vers le centre, puis par
/// concentration : le premier colore la peau.
fn sorted_pigments(m: &sim::Module) -> Vec<[u8; 3]> {
    let mut p = m.pigments.clone();
    p.sort_by(|a, b| a.depth.total_cmp(&b.depth).then(b.concentration.total_cmp(&a.concentration)));
    let mut out: Vec<[u8; 3]> = Vec::new();
    for x in &p {
        let c = pigment_rgb(x);
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Organe interne centré dans le module `parent`.
fn organ(system: System, parent: u16, length_m: f32, radius_m: f32, attach: f32, around_deg: f32, elevation_deg: f32) -> Module {
    let mut o = Module::new(ModuleKind::Organ(system), length_m.max(1e-9), radius_m.max(1e-9));
    o.parent = Some(parent);
    o.attach = attach;
    o.around_deg = around_deg;
    o.elevation_deg = elevation_deg;
    o
}

/// Traduit le plan simulé pour le générateur d'apparence.
///
/// [Simplification] Les filaments très longs sont épaissis jusqu'à rester
/// visibles (au moins 1,2 % de leur longueur), comme les flagelles des
/// planches de microscopie ; la taille vraie reste celle de la barre
/// d'échelle. Les cellules d'une colonie se lisent dans le motif de la peau.
pub fn from_simulated(plan: &sim::BodyPlan, org: &Organisation, seed: u64) -> BodyPlan {
    let mut rng = DrawRng::new(seed ^ 0x636F_6C6F);
    let mut modules: Vec<Module> = Vec::new();
    let Some(root) = plan.modules.first() else {
        return BodyPlan {
            version: PLAN_VERSION,
            modules: vec![Module::new(ModuleKind::Trunk, 0.0, 1e-6)],
            cell_types: 1,
            pattern: Pattern::NONE,
            seed,
        };
    };
    let d = &root.dimensions;
    let mut pigments = sorted_pigments(root);
    if pigments.is_empty() {
        // Sans pigment : la teinte du type cellulaire de surface, sinon
        // une gelée pâle.
        let surface = root.layers.first().and_then(|l| plan.cell_types.get(l.cell_type as usize));
        pigments.push(
            surface
                .and_then(|t| t.pigment_nm)
                .map_or([214, 204, 172], |nm| pigment_rgb(&sim::PigmentPatch { absorption_nm: nm, concentration: 0.6, depth: 0.0 })),
        );
    }
    let cells = org.body_cells.max(1.0);
    let mut pattern = Pattern::NONE;
    match (root.kind, d.profile) {
        (sim::ModuleKind::Cell, _) => {
            let r = (d.width_m / 2.0).max(1e-9) as f32;
            let mut m = Module::new(ModuleKind::Trunk, r * 0.25, r);
            m.covering = covering(root.covering);
            m.pigments = pigments;
            modules.push(m);
            if org.eukaryote {
                // Noyau, plastes (membranes photosynthétiques),
                // mitochondries et vacuole digestive d'un phagotrophe.
                modules.push(organ(System::Nucleus, 0, r * 0.7, r * 0.36, 0.2, 0.0, 30.0));
                for k in 0..org.plastids.min(6) {
                    modules.push(organ(System::Photosynthetic, 0, r * 0.5, r * 0.2, 0.5, 60.0 * k as f32 + 20.0, 90.0));
                }
                if org.organelles > org.plastids {
                    for k in 0..3 {
                        modules.push(organ(System::Respiratory, 0, r * 0.3, r * 0.12, 0.3, 120.0 * k as f32 + 75.0, 90.0));
                    }
                }
                if org.phagotroph {
                    modules.push(organ(System::Digestive, 0, r * 0.45, r * 0.22, 0.8, 200.0, 70.0));
                }
            }
        }
        (_, sim::Profile::Elongated) => {
            // Filament : une chaîne de cellules, enroulée en boucle lâche.
            let n = (cells.round() as usize).clamp(1, 24);
            let seg = (d.length_m / n as f64) as f32;
            let r = ((d.width_m / 2.0) as f32).max(d.length_m as f32 * 0.012);
            let turn = rng.range(8.0, 15.0) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
            // Chaque maillon est une cellule ovale : les étranglements
            // entre cellules restent lisibles après l'union douce.
            let mut m = Module::new(ModuleKind::Head, seg * 1.1, r);
            m.covering = covering(root.covering);
            m.pigments = pigments.clone();
            modules.push(m);
            // Une file d'une cellule d'épaisseur n'a pas de couches
            // profondes : les autres types cellulaires s'intercalent dans la
            // chaîne selon leur part, un peu renflés (comme les hétérocystes).
            let main_type = root.layers.first().map_or(0, |l| l.cell_type);
            let others: Vec<(usize, [u8; 3])> = root
                .layers
                .iter()
                .filter(|l| l.cell_type != main_type && l.weight > 0.0)
                .filter_map(|l| {
                    let t = plan.cell_types.get(l.cell_type as usize)?;
                    let every = (1.0 / l.weight).round().clamp(2.0, n as f64) as usize;
                    let colour = t.pigment_nm.map_or([200, 188, 150], |nm| {
                        pigment_rgb(&sim::PigmentPatch { absorption_nm: nm, concentration: 0.5, depth: 0.0 })
                    });
                    Some((every, colour))
                })
                .collect();
            for k in 1..n {
                let mut s = Module::new(ModuleKind::Head, seg * 1.1, r * rng.range(0.92, 1.06));
                s.parent = Some(k as u16 - 1);
                s.attach = 1.0;
                s.elevation_deg = turn + rng.range(-4.0, 4.0);
                s.covering = covering(root.covering);
                s.pigments = pigments.clone();
                if let Some((_, c)) = others.iter().find(|(every, _)| k % every == every / 2) {
                    s.pigments = vec![*c];
                    s.radius_m = [r * 1.25, r * 1.25];
                }
                modules.push(s);
            }
            // Dans chaque cellule photosynthétique, ses membranes (thylakoïdes
            // ou plastes), comme pour un microbe seul.
            let photo = plan.cell_types.get(main_type as usize).is_some_and(|t| system_of(t) == System::Photosynthetic);
            if photo {
                for k in 0..n {
                    let rk = modules[k].radius_m[0];
                    modules.push(organ(System::Photosynthetic, k as u16, seg * 0.6, rk * 0.45, 0.2, 0.0, 0.0));
                }
            }
        }
        (_, sim::Profile::Flattened) => {
            // Feuillet : une lame ondulée.
            let mut m = Module::new(ModuleKind::Leaf, d.length_m as f32, (d.thickness_m / 2.0).max(1e-9) as f32);
            m.width_m = d.width_m as f32;
            m.curvature = rng.range(-0.12, 0.12);
            m.covering = covering(root.covering);
            m.pigments = pigments;
            modules.push(m);
        }
        (_, sim::Profile::Round) => {
            // Boule : sphère creuse ou pleine selon les couches.
            let r = (d.width_m.max(d.length_m) / 2.0).max(1e-9) as f32;
            let mut m = Module::new(ModuleKind::Trunk, r * 0.1, r);
            m.covering = covering(root.covering);
            m.pigments = pigments;
            modules.push(m);
        }
    }
    if root.kind != sim::ModuleKind::Cell {
        let filament = d.profile == sim::Profile::Elongated;
        // Couches internes : un organe centré par type cellulaire profond,
        // d'autant plus gros que la couche commence près de la surface.
        let main = &modules[0];
        let (len, r) = (main.length_m.max(main.radius_m[0] * 2.0), main.radius_m[0].max(main.width_m / 2.0));
        let mut seen = Vec::new();
        for l in root.layers.iter().filter(|l| !filament && l.depth >= 0.3) {
            let Some(t) = plan.cell_types.get(l.cell_type as usize) else { continue };
            if seen.contains(&l.cell_type) {
                continue;
            }
            seen.push(l.cell_type);
            let k = (1.0 - l.depth as f32).clamp(0.15, 0.8);
            let along = if modules[0].kind == ModuleKind::Leaf { 0.5 } else { 0.0 };
            modules.push(organ(system_of(t), 0, len * k * 0.9, r * k, along, 0.0, 0.0));
        }
        // Le motif de la peau : celui du plan s'il y en a un (signalisation
        // entre cellules), sinon le grain des cellules de la colonie.
        let across = cells.sqrt().clamp(2.0, 14.0);
        pattern = match root.pattern {
            _ if filament && cells < 8.0 => Pattern::NONE,
            Some(t) => {
                let spots = t.inhibitor_diffusion > 2.0 * t.activator_diffusion;
                Pattern {
                    feed: if spots { 0.035 } else { 0.029 },
                    kill: if spots { 0.065 } else { 0.057 },
                    contrast: (t.activation as f32).clamp(0.2, 1.0) * if filament { 0.5 } else { 1.0 },
                    scale: across,
                }
            }
            None if filament => Pattern::NONE,
            None => Pattern { feed: 0.035, kill: 0.065, contrast: 0.25, scale: across },
        };
    }
    // Modules suivants du plan simulé (aucun à l'étape 4).
    let mut index: Vec<(u16, u16)> = vec![(root.id, 0)];
    for m in plan.modules.iter().skip(1) {
        let mut g = generic_module(m, plan);
        let parent = m.parent.and_then(|p| index.iter().find(|(id, _)| *id == p)).map_or(0, |(_, i)| *i);
        g.parent = Some(parent);
        index.push((m.id, modules.len() as u16));
        modules.push(g);
    }
    let cell_types = plan.cell_types.len().clamp(1, 255) as u8;
    let iridescence = root.structural_colour.map_or(0.0, |c| c.regularity.clamp(0.0, 1.0) as f32);
    modules[0].iridescence = iridescence;
    BodyPlan { version: PLAN_VERSION, modules, cell_types, pattern, seed }
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
        System::Nucleus => {
            if fr {
                "noyau"
            } else {
                "nucleus"
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

    fn sim_plan(kind: sim::ModuleKind, profile: sim::Profile, dims: [f64; 3], layers: &[(f64, f64, u8)]) -> sim::BodyPlan {
        let ty = |pathways, role: &str, nm| sim::CellType { pathways, role: role.into(), expressed_genes: 5, pigment_nm: nm };
        sim::BodyPlan {
            version: sim::BODY_PLAN_VERSION,
            cell_types: vec![
                ty(1 << evo_life::metabolism::OXYGENIC_PHOTOSYNTHESIS, "photosynthétique", Some(680.0)),
                ty(0, "réserve", None),
            ],
            modules: vec![sim::Module {
                id: 0,
                kind,
                cell_types: vec![0, 1],
                layers: layers.iter().map(|&(depth, weight, cell_type)| sim::Layer { depth, weight, cell_type }).collect(),
                master_regulator: None,
                parent: None,
                attach: [0.0; 3],
                axis: [1.0, 0.0, 0.0],
                offset: [0.0; 3],
                dimensions: sim::Dimensions { length_m: dims[0], width_m: dims[1], thickness_m: dims[2], profile, carbon_mol: 1e-12 },
                symmetry: sim::Symmetry { kind: sim::SymmetryKind::Spherical, copies: 1, spacing: 0.0 },
                joints: Vec::new(),
                material: sim::Material::Hydrostatic,
                covering: sim::Covering::Mucus,
                pigments: vec![sim::PigmentPatch { absorption_nm: 680.0, concentration: 0.8, depth: 0.0 }],
                structural_colour: None,
                pattern: None,
                function: sim::Function { pathways: 0, maintenance_kj: 1.0, build_kj: 1.0 },
            }],
        }
    }

    #[test]
    fn simulated_colonies_become_bodies_of_their_true_size() {
        let org =
            Organisation { cell_size: 4.0, body_cells: 200.0, cell_types: 2, eukaryote: true, multicellular: true, ..Default::default() };
        // Boule de 40 µm à cœur de réserve, filament de 200 µm avec un type
        // intercalé, feuillet de 60 µm.
        let ball =
            from_simulated(&sim_plan(sim::ModuleKind::Body, sim::Profile::Round, [40e-6; 3], &[(0.0, 0.7, 0), (0.6, 0.3, 1)]), &org, 1);
        let fil = from_simulated(
            &sim_plan(sim::ModuleKind::Body, sim::Profile::Elongated, [200e-6, 4e-6, 4e-6], &[(0.0, 0.9, 0), (1.0, 0.1, 1)]),
            &org,
            2,
        );
        let sheet =
            from_simulated(&sim_plan(sim::ModuleKind::Body, sim::Profile::Flattened, [60e-6, 60e-6, 8e-6], &[(0.0, 1.0, 0)]), &org, 3);
        for (plan, size) in [(&ball, 40e-6), (&fil, 200e-6 * 0.55), (&sheet, 60e-6)] {
            assert!(plan.is_valid());
            assert_eq!(plan.cell_types, 2);
            let e = body::extent_m(plan);
            assert!(e > size * 0.5 && e < size * 1.6, "{e} {size}");
            let b = body::build(plan, 1);
            assert!(b.lods[0].triangles() > 100);
        }
        // Le cœur de réserve est un organe interne ; le filament intercale
        // ses cellules de l'autre type.
        assert_eq!(plan_traits(&ball, 40e-6, Lang::Fr).systems, vec![System::Storage]);
        assert!(fil.modules.iter().any(|m| m.radius_m[0] > fil.modules[0].radius_m[0] * 1.1));
        assert_eq!(fil.modules.len(), 48);
        assert_eq!(plan_traits(&fil, 1e-4, Lang::Fr).systems, vec![System::Photosynthetic]);
        // Planche de la fiche : barre d'échelle ronde, corps dessiné.
        let (cv, bar) = body::plate(&ball, 240, 240);
        assert!([5.0, 10.0, 20.0].contains(&bar), "{bar}");
        let (blank, _) = (evo_morph::canvas::Canvas::paper(240, 240, ball.seed), 0);
        assert_ne!(cv.checksum(), blank.checksum());
    }

    #[test]
    fn a_eukaryote_cell_shows_its_organelles() {
        let org = Organisation {
            cell_size: 10.0,
            body_cells: 1.0,
            cell_types: 1,
            organelles: 2,
            plastids: 1,
            phagotroph: true,
            eukaryote: true,
            ..Default::default()
        };
        let plan = from_simulated(&sim_plan(sim::ModuleKind::Cell, sim::Profile::Round, [10e-6; 3], &[(0.0, 1.0, 0)]), &org, 4);
        let t = plan_traits(&plan, 10e-6, Lang::Fr);
        for s in [System::Nucleus, System::Photosynthetic, System::Respiratory, System::Digestive] {
            assert!(t.systems.contains(&s), "{s:?}");
        }
        assert_eq!(system_label(System::Nucleus, Lang::Fr), "noyau");
    }
}
