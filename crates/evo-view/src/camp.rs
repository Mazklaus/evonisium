//! Campement d'un peuple dans la scène au sol (étape 6, palier 3 du rendu,
//! première part) : foyer, huttes ou pare-vent, séchoirs, radeau, outils
//! posés, et les gens du peuple qui fabriquent leurs outils, enseignent,
//! échangent et célèbrent leurs rites autour du feu, outils en main et
//! parures au cou.
//!
//! Tout vient du groupe culturel de la cellule ([`crate::culture::CampView`]) :
//! une technique portée par plus d'un quart du groupe a son objet, des rites
//! mettent un cercle autour du foyer. Affichage seul : rien n'en remonte.
//!
//! [Simplification] Tant que les espèces intelligentes n'ont pas leurs vrais
//! corps (étape 5, fil Animaux), les gens du campement sont des figurants
//! primates à quatre membres ([`crate::ground::BenchKind::Primate`]) ; un
//! outil tenu est posé devant la tête, au sol, pendant le geste.

use crate::anatomy::from_simulated;
use crate::culture::CampView;
use crate::ground::{bench_plan, BenchKind, Patch, SceneSpecies};
use evo_culture::catalog::Family;
use evo_morph::artefact::Artefact;
use evo_morph::body::Action;
use evo_morph::canvas::{mix64, DrawRng};
use evo_sim::history::Organisation;
use std::f32::consts::TAU;

/// L'objet qui montre une technique.
pub fn artefact_of(f: Family) -> Artefact {
    match f {
        Family::Probe => Artefact::Stick,
        Family::Percussion => Artefact::Hammerstone,
        Family::Flake => Artefact::Flake,
        Family::Handaxe => Artefact::Handaxe,
        Family::Cord => Artefact::Cord,
        Family::Shelter => Artefact::Hut,
        Family::Container => Artefact::Bowl,
        Family::Fire => Artefact::Hearth,
        Family::Cooking | Family::Clothing => Artefact::Rack,
        Family::Ornament => Artefact::Necklace,
        Family::Spear => Artefact::Spear,
        Family::Raft => Artefact::Raft,
        Family::Thrower => Artefact::Thrower,
        Family::BoneTool => Artefact::Awl,
        Family::Fishing => Artefact::Trap,
    }
}

/// Un objet posé dans la scène : position (pied au sol), cap, échelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub artefact: Artefact,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    /// Inclinaison autour de l'axe avant de l'objet (radians), pour un
    /// outil couché ou porté de biais.
    pub roll: f32,
    pub scale: f32,
}

impl Placed {
    /// Transformation d'instance (3 lignes de 4), comme celle des
    /// individus : l'avant de l'objet (+x) tourné vers `heading`.
    pub fn rows(&self) -> [f32; 12] {
        let k = self.scale;
        let (sn, cs) = (-self.heading).sin_cos();
        let (sr, cr) = self.roll.sin_cos();
        // Lacet ∘ roulis (autour de x).
        let r = [[cs, sn * sr, sn * cr], [0.0, cr, -sr], [-sn, cs * sr, cs * cr]];
        [
            r[0][0] * k,
            r[0][1] * k,
            r[0][2] * k,
            self.x,
            r[1][0] * k,
            r[1][1] * k,
            r[1][2] * k,
            self.y,
            r[2][0] * k,
            r[2][1] * k,
            r[2][2] * k,
            self.z,
        ]
    }
}

/// Le campement d'une scène.
#[derive(Clone, Debug, PartialEq)]
pub struct Camp {
    pub people: String,
    pub centre: (f32, f32),
    /// Les gens restent à moins de ce rayon du foyer.
    pub radius: f32,
    pub fixed: Vec<Placed>,
    /// Outils de la main, du plus simple au plus élaboré.
    pub tools: Vec<Artefact>,
    pub ornament: bool,
    pub spear: Option<Artefact>,
    /// Part du temps libre donnée à fabriquer, enseigner, échanger, et aux
    /// rites (forcie pour se voir dans une scène de quelques minutes).
    pub shares: [f32; 4],
    /// Nombre de gens montrés.
    pub folk: usize,
    pub seed: u64,
}

/// Le replat le plus doux près du centre de la scène, hors de l'eau.
fn flat_spot(patch: &Patch) -> (f32, f32) {
    let mut best = (0.0, 0.0);
    let mut score = f32::INFINITY;
    for j in -6..=6 {
        for i in -6..=6 {
            let (x, z) = (i as f32 * 12.0, j as f32 * 12.0);
            let h = patch.height_at(x, z);
            if patch.water && h < 0.5 {
                continue;
            }
            let slope = (patch.height_at(x + 6.0, z) - patch.height_at(x - 6.0, z)).abs()
                + (patch.height_at(x, z + 6.0) - patch.height_at(x, z - 6.0)).abs();
            let s = slope + 0.002 * (x * x + z * z).sqrt();
            if s < score {
                score = s;
                best = (x, z);
            }
        }
    }
    best
}

/// Dispose le campement de `view` dans la scène.
pub fn layout(view: &CampView, people: &str, patch: &Patch, seed: u64) -> Camp {
    let seed = mix64(seed ^ 0x0000_6361_6D70);
    let mut rng = DrawRng::new(seed);
    let centre = flat_spot(patch);
    let knows = |f: Family| view.techniques.contains(&f);
    let mut fixed = Vec::new();
    let mut put = |rng: &mut DrawRng, a: Artefact, r: f32, angle: f32, face_in: bool, roll: f32| {
        let (x, z) = (centre.0 + r * angle.cos(), centre.1 + r * angle.sin());
        let heading = if face_in { (centre.1 - z).atan2(centre.0 - x) } else { rng.range(0.0, TAU) };
        fixed.push(Placed { artefact: a, x, y: patch.height_at(x, z), z, heading, roll, scale: 1.0 });
    };
    if knows(Family::Fire) {
        put(&mut rng, Artefact::Hearth, 0.0, 0.0, false, 0.0);
    }
    // Abris en couronne, l'entrée vers le feu.
    let (shelter, count) = if knows(Family::Shelter) {
        (Artefact::Hut, (view.population / 30.0).round().clamp(2.0, 6.0) as usize)
    } else {
        (Artefact::Windbreak, 2 + (rng.next_u64() % 2) as usize)
    };
    let start = rng.range(0.0, TAU);
    for k in 0..count {
        let a = start + TAU * k as f32 / count as f32 + rng.range(-0.2, 0.2);
        let r = rng.range(7.0, 9.5);
        put(&mut rng, shelter, r, a, true, 0.0);
    }
    if knows(Family::Cooking) || knows(Family::Clothing) {
        let racks = if knows(Family::Cooking) && knows(Family::Clothing) { 2 } else { 1 };
        for k in 0..racks {
            let r = rng.range(4.0, 5.0);
            put(&mut rng, Artefact::Rack, r, start + TAU * (k as f32 + 0.5) / count as f32, true, 0.0);
        }
    }
    if knows(Family::Raft) {
        put(&mut rng, Artefact::Raft, 13.0, start + 0.4, false, 0.0);
    }
    // Outils posés autour du foyer.
    let mut tools: Vec<Artefact> = Vec::new();
    for f in &view.techniques {
        let a = artefact_of(*f);
        if !a.fixed() && a != Artefact::Necklace && !tools.contains(&a) {
            tools.push(a);
        }
    }
    tools.sort();
    for &a in &tools {
        for _ in 0..2 {
            let (r, t) = (rng.range(1.4, 3.2), rng.range(0.0, TAU));
            put(&mut rng, a, r, t, false, 0.0);
        }
    }
    let n_tech = view.techniques.len() as f32;
    let shares = [
        (0.06 * n_tech).min(0.3),
        if n_tech >= 2.0 { 0.12 } else { 0.0 },
        if n_tech >= 4.0 { 0.1 } else { 0.0 },
        if view.rites { 0.15 + 0.1 * view.cohesion.clamp(0.0, 1.0) } else { 0.0 },
    ];
    let spear = [Artefact::Thrower, Artefact::Spear].into_iter().find(|a| tools.contains(a));
    Camp {
        people: people.to_string(),
        centre,
        radius: 11.0,
        fixed,
        tools: tools.into_iter().filter(|a| !matches!(a, Artefact::Spear | Artefact::Thrower | Artefact::Trap)).collect(),
        ornament: knows(Family::Ornament),
        spear,
        shares,
        folk: (view.population / 8.0).round().clamp(8.0, 24.0) as usize,
        seed,
    }
}

/// Les gens d'un peuple d'essai dans la scène : figurants primates.
pub fn folk_species(name: &str, seed: u64) -> SceneSpecies {
    let size = 1.5;
    let sim_plan = bench_plan(BenchKind::Primate, seed, size as f64);
    let org = Organisation { body_cells: 1e9, cell_types: 3, eukaryote: true, multicellular: true, sexual: true, ..Default::default() };
    let mut plan = from_simulated(&sim_plan, &org, seed);
    // Pelage brun : la teinte d'un pigment tiré au hasard ne sied pas à
    // des gens.
    for m in plan.modules.iter_mut() {
        if !m.pigments.is_empty() {
            m.pigments = vec![[112, 80, 56]];
        }
    }
    let mut s = SceneSpecies::new(name.to_string(), None, plan, size, false, Some(BenchKind::Primate));
    s.social = 0.9;
    s
}

impl Camp {
    /// Action culturelle tirée selon les parts du campement (`r` uniforme
    /// dans [0, 1)), ou `None` pour les gestes de tous les jours.
    pub fn draw_action_at(&self, r: f32) -> Option<Action> {
        let acts = [Action::MakeTool, Action::Teach, Action::Trade, Action::Rite];
        let mut acc = 0.0;
        for (k, a) in acts.iter().enumerate() {
            acc += self.shares[k];
            if r < acc {
                return Some(*a);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Site;

    fn site() -> Site {
        Site {
            cell: 3,
            elevation_m: 300.0,
            is_ocean: false,
            temperature_k: 290.0,
            ice_cover: 0.0,
            roughness_m: 120.0,
            life_rgb: None,
            biomass_per_m2: 0.3,
            sky: [0.8, 0.85, 0.9],
        }
    }

    #[test]
    fn a_camp_shows_what_the_group_knows() {
        let patch = Patch::new(site(), 5);
        let view = CampView {
            people: 1,
            population: 120.0,
            cohesion: 0.6,
            techniques: vec![Family::Fire, Family::Shelter, Family::Flake, Family::Spear, Family::Ornament, Family::Cooking],
            rites: true,
        };
        let c = layout(&view, "Fako", &patch, 9);
        let count = |a: Artefact| c.fixed.iter().filter(|p| p.artefact == a).count();
        assert_eq!(count(Artefact::Hearth), 1);
        assert_eq!(count(Artefact::Hut), 4);
        assert_eq!(count(Artefact::Rack), 1);
        assert_eq!(count(Artefact::Windbreak), 0);
        assert!(c.tools.contains(&Artefact::Flake));
        assert_eq!(c.spear, Some(Artefact::Spear));
        assert!(c.ornament);
        assert!(c.shares[3] > 0.0);
        assert_eq!(c, layout(&view, "Fako", &patch, 9));
        // Sans feu ni abri : pare-vent, pas de foyer, pas de rite.
        let bare = CampView { techniques: vec![Family::Percussion], rites: false, ..view };
        let b = layout(&bare, "Fako", &patch, 9);
        assert!(b.fixed.iter().all(|p| p.artefact != Artefact::Hearth && p.artefact != Artefact::Hut));
        assert!(b.fixed.iter().any(|p| p.artefact == Artefact::Windbreak));
        assert_eq!(b.shares[3], 0.0);
        assert_eq!(b.draw_action_at(0.99), None);
    }

    #[test]
    fn the_folk_make_tools_teach_trade_and_dance_around_the_fire() {
        use crate::ground::Crowd;
        let patch = Patch::new(site(), 5);
        let view = CampView {
            people: 1,
            population: 160.0,
            cohesion: 0.8,
            techniques: vec![
                Family::Fire,
                Family::Shelter,
                Family::Flake,
                Family::Handaxe,
                Family::Spear,
                Family::Ornament,
                Family::Container,
            ],
            rites: true,
        };
        let camp = layout(&view, "Fako", &patch, 9);
        let (cx, cz) = camp.centre;
        let mut crowd = Crowd::new(patch, Vec::new(), 0, 3);
        crowd.add_camp(camp, folk_species("Fako", 4));
        let n = crowd.individuals.len();
        assert_eq!(n, 20);
        let mut seen = std::collections::HashSet::new();
        let mut props = 0;
        for _ in 0..2400 {
            let y = crowd.patch.height_at(cx, cz) + 12.0;
            crowd.step(0.05, [cx, y, cz + 15.0], 64);
            for ind in &crowd.individuals {
                seen.insert(ind.action);
                assert!((ind.x - cx).hypot(ind.z - cz) < 40.0, "{ind:?}");
            }
            props = props.max(crowd.props().len());
        }
        for a in [Action::MakeTool, Action::Teach, Action::Rite] {
            assert!(seen.contains(&a), "{a:?} {seen:?}");
        }
        // Colliers au cou de chacun, et des outils en main.
        assert!(props > n, "{props}");
    }

    #[test]
    fn placed_rows_keep_the_scale_and_turn_the_front() {
        let p = Placed { artefact: Artefact::Spear, x: 1.0, y: 2.0, z: 3.0, heading: TAU / 4.0, roll: 0.0, scale: 2.0 };
        let r = p.rows();
        // L'avant (+x) tourné d'un quart de tour vers +z.
        let front = [r[0], r[4], r[8]];
        assert!(front[0].abs() < 1e-5 && (front[2] - 2.0).abs() < 1e-5, "{front:?}");
        assert_eq!((r[3], r[7], r[11]), (1.0, 2.0, 3.0));
    }
}
