//! Animation procédurale (document Rendu du vivant, « Mouvement et
//! comportements visibles ») : aucune animation n'est dessinée à la main.
//! Le squelette vient des os placés du corps (un par instance de module) ;
//! chaque os reçoit un rôle (colonne, tête, patte, nageoire, fouet, tige,
//! lame) et le corps un mode de locomotion déduit de ses organes : marche
//! si au moins deux pattes, vol si des lames sans eau, nage ondulatoire
//! sinon, flagelles pour une cellule, balancement pour un organisme fixé.
//!
//! Une pose est une liste de transformations affines, une par os, qui
//! envoient la position de repos d'un sommet vers sa position animée ; le
//! client les mélange par les poids de peau (deux os par sommet). Chaque
//! action du lexique commun (doc d'architecture, étape 5) a son style :
//! allure, posture de la tête et du corps, oscillations.
//!
//! [Simplification] Pas de cinématique inverse au sol : les pattes balancent
//! autour de leur attache selon l'allure (tripode pour six, diagonale pour
//! quatre, alternance pour deux), le corps suit le relief en bloc.

use super::plan::{BodyPlan, ModuleKind, Symmetry};
use super::shape::{Bone, V3};
use std::f32::consts::{PI, TAU};

/// Actions du lexique commun (doc d'architecture, « Périmètre consolidé de
/// l'étape 5 »). Le lexique reste ouvert aux actions culturelles de
/// l'étape 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Feed,
    Hunt,
    Flee,
    Hide,
    Mate,
    Lay,
    Care,
    Migrate,
    Fight,
    Communicate,
    Build,
    Rest,
}

impl Action {
    pub const ALL: [Action; 12] = [
        Action::Feed,
        Action::Hunt,
        Action::Flee,
        Action::Hide,
        Action::Mate,
        Action::Lay,
        Action::Care,
        Action::Migrate,
        Action::Fight,
        Action::Communicate,
        Action::Build,
        Action::Rest,
    ];

    pub fn index(self) -> usize {
        Action::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Action {
        Action::ALL[i % Action::ALL.len()]
    }

    pub fn key(self) -> &'static str {
        match self {
            Action::Feed => "se_nourrir",
            Action::Hunt => "chasser",
            Action::Flee => "fuir",
            Action::Hide => "se_cacher",
            Action::Mate => "s_accoupler",
            Action::Lay => "pondre",
            Action::Care => "soigner",
            Action::Migrate => "migrer",
            Action::Fight => "se_battre",
            Action::Communicate => "communiquer",
            Action::Build => "construire",
            Action::Rest => "se_reposer",
        }
    }

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Action::Feed, true) => "se nourrit",
            (Action::Feed, false) => "feeding",
            (Action::Hunt, true) => "chasse",
            (Action::Hunt, false) => "hunting",
            (Action::Flee, true) => "fuit",
            (Action::Flee, false) => "fleeing",
            (Action::Hide, true) => "se cache",
            (Action::Hide, false) => "hiding",
            (Action::Mate, true) => "s'accouple",
            (Action::Mate, false) => "mating",
            (Action::Lay, true) => "pond ou met bas",
            (Action::Lay, false) => "laying or giving birth",
            (Action::Care, true) => "soigne les petits",
            (Action::Care, false) => "caring for young",
            (Action::Migrate, true) => "migre",
            (Action::Migrate, false) => "migrating",
            (Action::Fight, true) => "se bat",
            (Action::Fight, false) => "fighting",
            (Action::Communicate, true) => "communique",
            (Action::Communicate, false) => "communicating",
            (Action::Build, true) => "construit",
            (Action::Build, false) => "building",
            (Action::Rest, true) => "se repose",
            (Action::Rest, false) => "resting",
        }
    }

    /// Allure : vitesse de croisière en longueurs de corps par seconde
    /// (0 : sur place).
    pub fn pace(self) -> f32 {
        match self {
            Action::Flee => 3.0,
            Action::Hunt => 1.6,
            Action::Migrate => 1.0,
            Action::Fight => 0.25,
            Action::Mate | Action::Communicate => 0.15,
            Action::Feed | Action::Build => 0.1,
            Action::Care | Action::Lay | Action::Hide | Action::Rest => 0.0,
        }
    }
}

/// Style d'une action : ce qui change dans la posture et les oscillations.
#[derive(Clone, Copy, Debug)]
struct Style {
    /// Tête vers le bas (positif) ou le haut, radians.
    head_pitch: f32,
    /// Hochements de la tête : fréquence (Hz) et amplitude (radians).
    head_bob: (f32, f32),
    /// Corps cabré (positif) ou piqué, radians.
    body_pitch: f32,
    /// Corps abaissé, en part de sa hauteur.
    crouch: f32,
    /// Lames écartées (parade, menace), radians.
    spread: f32,
    /// Oscillation de tout le corps (balancement de parade).
    sway: (f32, f32),
}

fn style(a: Action) -> Style {
    let base = Style { head_pitch: 0.0, head_bob: (0.0, 0.0), body_pitch: 0.0, crouch: 0.0, spread: 0.0, sway: (0.0, 0.0) };
    match a {
        Action::Feed => Style { head_pitch: 0.55, head_bob: (1.2, 0.12), ..base },
        Action::Hunt => Style { head_pitch: 0.15, crouch: 0.12, ..base },
        Action::Flee => Style { head_pitch: -0.1, body_pitch: 0.05, ..base },
        Action::Hide => Style { head_pitch: 0.2, crouch: 0.35, ..base },
        Action::Mate => Style { spread: 0.5, sway: (0.8, 0.12), ..base },
        Action::Lay => Style { body_pitch: 0.18, crouch: 0.15, head_pitch: -0.1, ..base },
        Action::Care => Style { head_pitch: 0.4, head_bob: (0.5, 0.15), crouch: 0.05, ..base },
        Action::Migrate => base,
        Action::Fight => Style { body_pitch: 0.3, head_pitch: -0.2, head_bob: (3.0, 0.3), spread: 0.6, ..base },
        Action::Communicate => Style { head_pitch: -0.35, head_bob: (2.0, 0.18), spread: 0.25, ..base },
        Action::Build => Style { head_pitch: 0.6, head_bob: (2.2, 0.25), ..base },
        Action::Rest => Style { crouch: 0.45, head_pitch: 0.25, ..base },
    }
}

/// Rôle d'un os dans l'animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Spine,
    Head,
    Leg,
    Fin,
    Whip,
    Stalk,
    Leaf,
    Rigid,
}

/// Mode de locomotion du corps (doc Organismes, modes de locomotion).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locomotion {
    Walk { legs: u8 },
    Swim,
    Fly,
    Flagellar,
    Sessile,
}

#[derive(Clone, Copy, Debug)]
pub struct RigBone {
    pub parent: Option<u16>,
    pub head: V3,
    pub tail: V3,
    pub role: Role,
    /// -1 à gauche, +1 à droite, 0 dans le plan sagittal.
    pub side: f32,
    /// Position le long du corps, 0 au bout avant, 1 au bout arrière.
    pub order: f32,
    /// Phase de l'allure (patte) ou de l'onde (colonne, fouet), en tours.
    pub phase: f32,
}

/// Squelette animable d'un corps.
#[derive(Clone, Debug)]
pub struct Rig {
    pub bones: Vec<RigBone>,
    pub locomotion: Locomotion,
    /// Plus grande dimension, en mètres.
    pub length: f32,
    /// Hauteur du corps, en mètres (accroupissement, repos).
    pub height: f32,
}

/// Transformation affine : rotation (lignes) puis translation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub r: [[f32; 3]; 3],
    pub t: [f32; 3],
}

impl Affine {
    pub const IDENTITY: Affine = Affine { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] };

    pub fn apply(&self, p: V3) -> V3 {
        let r = &self.r;
        V3(
            r[0][0] * p.0 + r[0][1] * p.1 + r[0][2] * p.2 + self.t[0],
            r[1][0] * p.0 + r[1][1] * p.1 + r[1][2] * p.2 + self.t[1],
            r[2][0] * p.0 + r[2][1] * p.1 + r[2][2] * p.2 + self.t[2],
        )
    }

    /// self ∘ o : applique `o` puis `self`.
    pub fn then_after(&self, o: &Affine) -> Affine {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| self.r[i][k] * o.r[k][j]).sum();
            }
        }
        let t = self.apply(V3(o.t[0], o.t[1], o.t[2]));
        Affine { r, t: [t.0, t.1, t.2] }
    }

    /// Rotation d'angle `a` autour de l'axe `axis` passant par `pivot`.
    pub fn rotation_about(pivot: V3, axis: V3, a: f32) -> Affine {
        let k = axis.norm();
        let (s, c) = a.sin_cos();
        let v = 1.0 - c;
        let r = [
            [c + k.0 * k.0 * v, k.0 * k.1 * v - k.2 * s, k.0 * k.2 * v + k.1 * s],
            [k.1 * k.0 * v + k.2 * s, c + k.1 * k.1 * v, k.1 * k.2 * v - k.0 * s],
            [k.2 * k.0 * v - k.1 * s, k.2 * k.1 * v + k.0 * s, c + k.2 * k.2 * v],
        ];
        let rot = Affine { r, t: [0.0; 3] };
        let p = rot.apply(pivot);
        Affine { r, t: [pivot.0 - p.0, pivot.1 - p.1, pivot.2 - p.2] }
    }

    pub fn translation(d: V3) -> Affine {
        Affine { t: [d.0, d.1, d.2], ..Affine::IDENTITY }
    }

    /// Les 12 nombres en lignes (3 lignes de 4), pour une texture d'os.
    pub fn rows(&self) -> [f32; 12] {
        let (r, t) = (&self.r, &self.t);
        [r[0][0], r[0][1], r[0][2], t[0], r[1][0], r[1][1], r[1][2], t[1], r[2][0], r[2][1], r[2][2], t[2]]
    }
}

/// Squelette d'un corps : rôles des os et mode de locomotion. `aquatic`
/// dit si le corps vit dans l'eau (lames qui nagent ou ailes qui volent).
pub fn rig(plan: &BodyPlan, bones: &[Bone], aquatic: bool) -> Rig {
    let (mut lo, mut hi) = (V3(f32::INFINITY, f32::INFINITY, f32::INFINITY), V3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY));
    for b in bones {
        lo = lo.min(b.head).min(b.tail);
        hi = hi.max(b.head).max(b.tail);
    }
    let span_x = (hi.0 - lo.0).max(1e-12);
    let roles: Vec<Role> = bones
        .iter()
        .map(|b| match plan.modules[b.module as usize].kind {
            ModuleKind::Trunk | ModuleKind::Segment => Role::Spine,
            ModuleKind::Head | ModuleKind::Mouth | ModuleKind::Eye(_) => Role::Head,
            ModuleKind::Limb => Role::Leg,
            ModuleKind::Fin | ModuleKind::Gill => Role::Fin,
            ModuleKind::Flagellum | ModuleKind::Antenna => Role::Whip,
            ModuleKind::Stalk => Role::Stalk,
            ModuleKind::Leaf => Role::Leaf,
            ModuleKind::Shell | ModuleKind::Organ(_) => Role::Rigid,
        })
        .collect();
    // Une patte compte si elle part d'un os de la colonne vers le bas ;
    // les bras d'une couronne (méduse, étoile de mer) n'en sont pas.
    let is_leg = |i: usize| {
        let b = &bones[i];
        roles[i] == Role::Leg
            && !matches!(plan.modules[b.module as usize].symmetry, Symmetry::Radial(_))
            && b.parent.is_some_and(|p| roles[p as usize] == Role::Spine)
            && b.tail.sub(b.head).norm().1 < 0.2
    };
    let legs: Vec<usize> = (0..bones.len()).filter(|&i| is_leg(i)).collect();
    let fins = roles.iter().filter(|r| **r == Role::Fin).count();
    let whips = roles.iter().filter(|r| **r == Role::Whip).count();
    let root_stalk = roles.first() == Some(&Role::Stalk);
    let spine_len = bones.iter().zip(&roles).filter(|(_, r)| **r == Role::Spine).count();
    // Des ailes : deux lames latérales plus longues que la moitié du corps.
    let wings = (0..bones.len()).filter(|&i| roles[i] == Role::Fin && bones[i].tail.sub(bones[i].head).2.abs() > 0.5 * span_x).count();
    let locomotion = if root_stalk || roles.iter().all(|r| matches!(r, Role::Leaf | Role::Stalk | Role::Rigid)) {
        Locomotion::Sessile
    } else if wings >= 2 && !aquatic {
        Locomotion::Fly
    } else if legs.len() >= 2 {
        Locomotion::Walk { legs: legs.len().min(255) as u8 }
    } else if fins >= 2 && !aquatic {
        Locomotion::Fly
    } else if spine_len <= 1 && whips > 0 && fins == 0 {
        Locomotion::Flagellar
    } else {
        Locomotion::Swim
    };
    // Allure : les pattes classées d'avant en arrière, par paires.
    let mut by_x: Vec<usize> = legs.clone();
    by_x.sort_by(|a, b| bones[*b].head.0.total_cmp(&bones[*a].head.0).then(a.cmp(b)));
    let mut out: Vec<RigBone> = bones
        .iter()
        .zip(&roles)
        .map(|(b, &role)| {
            let mid = b.head.add(b.tail).mul(0.5);
            let side = if mid.2.abs() < 1e-6 * span_x { 0.0 } else { mid.2.signum() };
            RigBone { parent: b.parent, head: b.head, tail: b.tail, role, side, order: (hi.0 - mid.0) / span_x, phase: 0.0 }
        })
        .collect();
    let mut left = 0;
    let mut right = 0;
    for &i in &by_x {
        let k = if out[i].side < 0.0 {
            left += 1;
            left - 1
        } else {
            right += 1;
            right - 1
        };
        // Paires alternées, gauche et droite en opposition : diagonale à
        // quatre pattes, tripode à six, alternance à deux.
        out[i].phase = ((k % 2) as f32 * 0.5 + if out[i].side < 0.0 { 0.5 } else { 0.0 }).fract();
    }
    for b in out.iter_mut() {
        if matches!(b.role, Role::Spine | Role::Whip | Role::Stalk | Role::Leaf) {
            b.phase = b.order;
        }
    }
    let length = (hi.0 - lo.0).max(hi.1 - lo.1).max(hi.2 - lo.2).max(1e-9);
    Rig { bones: out, locomotion, length, height: (hi.1 - lo.1).max(1e-9) }
}

/// État animé d'un individu.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub action: Action,
    /// Temps propre de l'individu, s.
    pub time: f32,
    /// Vitesse au sol en longueurs de corps par seconde.
    pub speed: f32,
    /// Décalage de phase propre (les individus d'un troupeau ne marchent
    /// pas au pas).
    pub phase: f32,
}

/// Pose d'un individu : une transformation par os, dans le repère du corps
/// (celui des sommets de repos).
pub fn pose(rig: &Rig, m: &Motion) -> Vec<Affine> {
    let s = style(m.action);
    let n = rig.bones.len();
    let mut local = vec![Affine::IDENTITY; n];
    // Cadence : un cycle par longueur de corps parcourue, au moins un léger
    // mouvement sur place.
    let cadence = (0.6 + 1.1 * m.speed).min(6.0);
    let t = m.time + m.phase;
    let w = TAU * cadence * t;
    let gait = (m.speed / 1.5).clamp(0.0, 1.0);
    let side_axis = V3(0.0, 0.0, 1.0);
    let up = V3::Y;
    for (i, b) in rig.bones.iter().enumerate() {
        let dir = b.tail.sub(b.head).norm();
        let a = match (b.role, rig.locomotion) {
            // Onde latérale le long du tronc, plus ample vers l'arrière.
            (Role::Spine, Locomotion::Swim) => Some((up, (0.08 + 0.32 * gait) * (0.2 + b.order) * (w - TAU * b.phase).sin())),
            (Role::Spine, Locomotion::Walk { .. }) => Some((up, 0.05 * gait * (w * 2.0 - TAU * b.phase).sin())),
            (Role::Spine, Locomotion::Fly) => Some((side_axis, 0.03 * (w).sin())),
            (Role::Spine, _) => Some((up, 0.04 * (0.5 * w - TAU * b.phase).sin())),
            // Patte : balancement autour de l'attache, selon l'allure.
            (Role::Leg, Locomotion::Walk { .. }) => Some((side_axis, (0.1 + 0.45 * gait) * (w + TAU * b.phase).sin())),
            (Role::Leg, _) => Some((side_axis, 0.15 * (w + TAU * b.phase).sin())),
            // Lames : battement d'aile, godille, ou écartement de parade.
            (Role::Fin, Locomotion::Fly) => Some((V3::X, b.side * (0.25 + s.spread + 0.6 * (w * 1.5).sin()))),
            (Role::Fin, _) => Some((V3::X, b.side * (s.spread + (0.1 + 0.25 * gait) * (w + TAU * b.order).sin()))),
            // Flagelles, antennes, tentacules : onde le long du fouet.
            (Role::Whip, Locomotion::Flagellar) => Some((up.cross(dir).norm(), 0.5 * (w * 2.0 - TAU * b.phase * 2.0).sin())),
            (Role::Whip, _) => Some((up.cross(dir).norm(), 0.18 * (0.7 * w - TAU * b.phase).sin())),
            // Fixés : balancement dans le courant ou le vent.
            (Role::Stalk | Role::Leaf, _) => Some((side_axis, 0.06 * (0.25 * TAU * t + PI * b.phase).sin())),
            // Tête : posture de l'action et hochements.
            (Role::Head, _) => {
                let bob = s.head_bob.1 * (TAU * s.head_bob.0 * t).sin();
                Some((side_axis, -(s.head_pitch + bob)))
            }
            (Role::Rigid, _) => None,
        };
        if let Some((axis, angle)) = a {
            if angle.is_finite() && angle.abs() > 1e-6 {
                local[i] = Affine::rotation_about(b.head, axis, angle);
            }
        }
    }
    // Composition le long de la hiérarchie : un os suit son parent.
    let mut world = vec![Affine::IDENTITY; n];
    for i in 0..n {
        world[i] = match rig.bones[i].parent {
            Some(p) if (p as usize) < i => world[p as usize].then_after(&local[i]),
            _ => local[i],
        };
    }
    // Corps entier : cabré, abaissé, balancé ; respiration au repos.
    let centre = V3(0.0, 0.0, 0.0);
    let breathe = if m.action == Action::Rest { 0.02 * (TAU * 0.3 * t).sin() } else { 0.0 };
    let sway = s.sway.1 * (TAU * s.sway.0 * t).sin();
    let bounce = if matches!(rig.locomotion, Locomotion::Walk { .. }) { 0.03 * gait * (2.0 * w).sin().abs() } else { 0.0 };
    let body = Affine::translation(V3(0.0, -rig.height * (s.crouch - breathe - bounce), 0.0))
        .then_after(&Affine::rotation_about(centre, V3::X, sway))
        .then_after(&Affine::rotation_about(centre, side_axis, s.body_pitch));
    world.iter().map(|w| body.then_after(w)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{random_plan, shape::place};

    fn walker() -> (BodyPlan, Vec<Bone>) {
        // Un plan d'essai à pattes : on cherche une graine qui en donne.
        for seed in 0..200 {
            let plan = random_plan(seed);
            let bones = place(&plan).bones;
            if let Locomotion::Walk { legs } = rig(&plan, &bones, false).locomotion {
                if legs >= 4 {
                    return (plan, bones);
                }
            }
        }
        panic!("aucun marcheur");
    }

    #[test]
    fn walkers_alternate_their_legs_and_every_action_poses() {
        let (plan, bones) = walker();
        let r = rig(&plan, &bones, false);
        // Diagonale : la première patte gauche et la première droite sont en
        // opposition.
        let legs: Vec<&RigBone> = r.bones.iter().filter(|b| b.role == Role::Leg && b.parent.is_some()).collect();
        let front = |side: f32| legs.iter().filter(|b| b.side == side).max_by(|a, b| a.head.0.total_cmp(&b.head.0)).map(|b| b.phase);
        let (l, rr) = (front(-1.0).unwrap(), front(1.0).unwrap());
        assert!((l - rr).abs() > 0.4, "{l} {rr}");
        for a in Action::ALL {
            let p = pose(&r, &Motion { action: a, time: 0.7, speed: a.pace(), phase: 0.0 });
            assert_eq!(p.len(), bones.len());
            assert!(p.iter().all(|m| m.rows().iter().all(|v| v.is_finite())));
        }
        // Le repos abaisse le corps, la fuite le fait bouger plus qu'au repos.
        let rest = pose(&r, &Motion { action: Action::Rest, time: 0.0, speed: 0.0, phase: 0.0 });
        assert!(rest[0].t[1] < 0.0);
        let moved = |a: Action| {
            let p0 = pose(&r, &Motion { action: a, time: 0.0, speed: a.pace(), phase: 0.0 });
            let p1 = pose(&r, &Motion { action: a, time: 0.05, speed: a.pace(), phase: 0.0 });
            p0.iter().zip(&p1).map(|(x, y)| x.apply(bones[0].tail).sub(y.apply(bones[0].tail)).len()).sum::<f32>()
                + p0.iter().zip(&p1).zip(&bones).map(|((x, y), b)| x.apply(b.tail).sub(y.apply(b.tail)).len()).sum::<f32>()
        };
        assert!(moved(Action::Flee) > moved(Action::Rest) * 2.0);
    }

    #[test]
    fn affine_composition_matches_application() {
        let a = Affine::rotation_about(V3(1.0, 0.0, 0.0), V3::Y, 0.4);
        let b = Affine::rotation_about(V3(0.0, 2.0, 0.0), V3(0.0, 0.0, 1.0), -0.7);
        let p = V3(0.3, -0.2, 0.9);
        let x = a.then_after(&b).apply(p);
        let y = a.apply(b.apply(p));
        assert!(x.sub(y).len() < 1e-5);
        // Le pivot ne bouge pas.
        assert!(a.apply(V3(1.0, 0.0, 0.0)).sub(V3(1.0, 0.0, 0.0)).len() < 1e-6);
    }
}
