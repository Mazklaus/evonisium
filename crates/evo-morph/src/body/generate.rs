//! Plans d'essai et adaptateur du vivant microbien.
//!
//! - [`random_plan`] tire un plan quelconque pour le banc d'essai : le
//!   générateur ne doit rien supposer de terrestre (six membres, trois yeux
//!   ou une symétrie d'ordre 5 se dessinent comme le reste).
//! - [`from_microbe`] traduit la forme d'un microbe (palier 1) en plan, pour
//!   que l'anatomie, le comparateur et les silhouettes du décor marchent dès
//!   l'ère microbienne.

use super::plan::{BodyPlan, Covering, EyeStage, Joint, Module, ModuleKind, Pattern, Symmetry, System, PLAN_VERSION};
use crate::canvas::{mix64, DrawRng};
use crate::microbe::{MicrobeForm, Shape};

fn rgb(c: [f32; 3]) -> [u8; 3] {
    [(c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8]
}

/// Plan d'un microbe : corps cellulaire, flagelles, et à l'intérieur ses
/// membranes photosynthétiques et ses granules de réserve.
pub fn from_microbe(f: &MicrobeForm) -> BodyPlan {
    let um = 1.0e-6;
    let r = f.width_um / 2.0 * um;
    let colour = rgb(f.colour);
    let mut modules = Vec::new();
    let mut body = Module::new(ModuleKind::Trunk, 0.0, r);
    body.covering = Covering::Wall;
    body.pigments = vec![colour];
    let mut rng = DrawRng::new(f.seed);
    match f.shape {
        Shape::Coccus => {
            body.length_m = r * 0.2;
        }
        Shape::Bacillus => {
            body.length_m = (f.length_um * um - 2.0 * r).max(r * 0.2);
        }
        Shape::Spirillum => {
            body.length_m = f.length_um * um;
            body.curvature = 0.25;
            body.radius_m = [r * 0.8, r * 0.8];
        }
        Shape::Filament => {
            body.length_m = f.length_um * um;
        }
    }
    modules.push(body);
    if f.shape == Shape::Filament && f.segments > 1 {
        // Les cellules suivantes de la chaîne, alignées derrière la première.
        let mut prev = 0u16;
        for _ in 1..f.segments {
            let mut s = Module::new(ModuleKind::Segment, f.length_um * um, r * rng.range(0.92, 1.05));
            s.parent = Some(prev);
            s.attach = 1.0;
            s.elevation_deg = rng.range(-8.0, 8.0);
            s.around_deg = 0.0;
            s.covering = Covering::Wall;
            s.pigments = vec![colour];
            modules.push(s);
            prev = modules.len() as u16 - 1;
        }
    }
    for k in 0..f.flagella {
        // Flagelle épaissi, comme sur les planches de microscopie : à
        // l'échelle vraie (20 nm), aucun maillage ne le montrerait.
        let mut fl = Module::new(ModuleKind::Flagellum, f.length_um.max(2.0) * um * rng.range(1.0, 1.6), r * 0.25);
        fl.parent = Some(0);
        fl.attach = 0.0;
        fl.around_deg = 180.0 + 40.0 * k as f32;
        fl.elevation_deg = 165.0;
        fl.curvature = rng.range(-0.15, 0.15);
        fl.covering = Covering::Membrane;
        fl.pigments = vec![colour];
        modules.push(fl);
    }
    if f.thylakoids {
        let mut t = Module::new(ModuleKind::Organ(System::Photosynthetic), modules[0].length_m.max(r) * 0.8, r * 0.45);
        t.parent = Some(0);
        t.attach = 0.1;
        t.radius_m = [r * 0.45, r * 0.55];
        modules.push(t);
    }
    for k in 0..f.granules {
        let mut g = Module::new(ModuleKind::Organ(System::Storage), r * 0.35, r * 0.18);
        g.parent = Some(0);
        g.attach = (k as f32 + 0.5) / f.granules as f32;
        g.around_deg = 70.0 * k as f32;
        g.elevation_deg = 90.0;
        modules.push(g);
    }
    BodyPlan { version: PLAN_VERSION, modules, cell_types: 1, pattern: Pattern::NONE, seed: f.seed }
}

/// Couleur de pigment tirée au hasard, plutôt terne comme le vivant.
fn pigment(rng: &mut DrawRng) -> [u8; 3] {
    let h = rng.unit() * 6.0;
    let s = rng.range(0.2, 0.65);
    let v = rng.range(0.45, 0.85);
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    rgb([r + m, g + m, b + m])
}

/// Plan tiré au hasard pour le banc d'essai : de la méduse au vertébré
/// marcheur, en passant par des formes sans équivalent terrestre.
pub fn random_plan(seed: u64) -> BodyPlan {
    let mut rng = DrawRng::new(mix64(seed ^ 0x706C_616E));
    let size = 10f32.powf(rng.range(-2.5, 0.8));
    let colour = pigment(&mut rng);
    let second = pigment(&mut rng);
    let covering = [Covering::Naked, Covering::Mucus, Covering::Scales, Covering::Hair, Covering::Cuticle, Covering::Feathers]
        [(rng.next_u64() % 6) as usize];
    let radial = rng.unit() < 0.2;
    let mut modules = Vec::new();
    let add = |mut m: Module, parent: Option<u16>, modules: &mut Vec<Module>| -> u16 {
        m.parent = parent;
        m.covering = covering;
        if m.pigments.is_empty() {
            m.pigments = vec![colour, second];
        }
        modules.push(m);
        modules.len() as u16 - 1
    };
    let girth = size * rng.range(0.08, 0.25);
    let mut trunk = Module::new(ModuleKind::Trunk, size * 0.6, girth);
    trunk.radius_m = [girth * rng.range(0.5, 1.0), girth * rng.range(0.4, 1.0)];
    trunk.curvature = rng.range(-0.1, 0.1);
    let root = add(trunk, None, &mut modules);
    if radial {
        // Corps radiaire : bras ou tentacules d'ordre 3 à 8 autour de l'axe.
        let n = 3 + (rng.next_u64() % 6) as u8;
        let mut arm = Module::new(ModuleKind::Antenna, size * rng.range(0.4, 0.9), girth * 0.3);
        arm.radius_m = [girth * 0.35, girth * 0.06];
        arm.attach = 0.15;
        arm.elevation_deg = rng.range(110.0, 160.0);
        arm.curvature = rng.range(-0.3, 0.3);
        arm.symmetry = Symmetry::Radial(n);
        arm.joint = Some(Joint { dof: 2, range_deg: 60.0 });
        add(arm, Some(root), &mut modules);
        let mut eye = Module::new(ModuleKind::Eye(EyeStage::Spot), girth * 0.2, girth * 0.12);
        eye.attach = 0.95;
        eye.elevation_deg = 70.0;
        eye.symmetry = Symmetry::Radial(n);
        add(eye, Some(root), &mut modules);
    } else {
        let mut head = Module::new(ModuleKind::Head, girth * rng.range(1.2, 2.2), girth * rng.range(0.6, 1.0));
        head.attach = 1.0;
        head.elevation_deg = rng.range(-15.0, 15.0);
        head.joint = Some(Joint { dof: 2, range_deg: 40.0 });
        let h = add(head, Some(root), &mut modules);
        let eyes = 1 + (rng.next_u64() % 3) as u8;
        let stage = [EyeStage::Cup, EyeStage::Pinhole, EyeStage::Lens, EyeStage::Compound][(rng.next_u64() % 4) as usize];
        let mut eye = Module::new(ModuleKind::Eye(stage), girth * 0.3, girth * 0.18);
        eye.attach = 0.6;
        eye.around_deg = if eyes == 1 { 0.0 } else { 60.0 };
        eye.elevation_deg = 80.0;
        eye.symmetry = if eyes == 2 {
            Symmetry::Bilateral
        } else if eyes >= 3 {
            Symmetry::Radial(eyes)
        } else {
            Symmetry::None
        };
        add(eye, Some(h), &mut modules);
        let mut mouth = Module::new(ModuleKind::Mouth, girth * 0.3, girth * 0.2);
        mouth.attach = 0.95;
        mouth.around_deg = 180.0;
        mouth.elevation_deg = 50.0;
        add(mouth, Some(h), &mut modules);
        if rng.unit() < 0.5 {
            let mut ant = Module::new(ModuleKind::Antenna, size * rng.range(0.2, 0.5), girth * 0.06);
            ant.attach = 0.8;
            ant.around_deg = 35.0;
            ant.elevation_deg = 40.0;
            ant.curvature = 0.2;
            ant.symmetry = Symmetry::Bilateral;
            add(ant, Some(h), &mut modules);
        }
        // Queue.
        let mut tail = Module::new(ModuleKind::Segment, size * rng.range(0.2, 0.6), girth * 0.6);
        tail.radius_m = [girth * 0.6, girth * 0.1];
        tail.attach = 0.0;
        tail.elevation_deg = 180.0 + rng.range(-20.0, 20.0);
        tail.curvature = rng.range(-0.2, 0.2);
        tail.joint = Some(Joint { dof: 2, range_deg: 50.0 });
        let t = add(tail, Some(root), &mut modules);
        // Paires de membres ou de nageoires (0 à 4 paires : six membres
        // passent comme quatre).
        let pairs = (rng.next_u64() % 5) as u8;
        let fins = rng.unit() < 0.4;
        if pairs > 0 {
            let mut limb = if fins {
                let mut m = Module::new(ModuleKind::Fin, size * rng.range(0.15, 0.3), girth * 0.05);
                m.width_m = size * rng.range(0.08, 0.2);
                m
            } else {
                let mut m = Module::new(ModuleKind::Limb, size * rng.range(0.2, 0.45), girth * 0.25);
                m.radius_m = [girth * 0.28, girth * 0.14];
                m
            };
            limb.attach = 0.15;
            limb.around_deg = rng.range(100.0, 140.0);
            limb.elevation_deg = rng.range(60.0, 100.0);
            limb.symmetry = if pairs == 1 { Symmetry::Bilateral } else { Symmetry::Segmental(pairs) };
            limb.joint = Some(Joint { dof: 3, range_deg: 90.0 });
            if limb.symmetry != Symmetry::Bilateral {
                // Paires segmentaires : la paire est un enfant bilatéral de
                // chaque copie.
                let mut carrier = Module::new(ModuleKind::Segment, girth * 0.1, girth * 0.4);
                carrier.attach = 0.15;
                carrier.around_deg = 0.0;
                carrier.elevation_deg = 90.0;
                carrier.symmetry = Symmetry::Segmental(pairs);
                let c = add(carrier, Some(root), &mut modules);
                limb.attach = 0.0;
                limb.symmetry = Symmetry::Bilateral;
                limb.elevation_deg = 170.0 - limb.elevation_deg;
                add(limb.clone(), Some(c), &mut modules);
            } else {
                let l = add(limb.clone(), Some(root), &mut modules);
                if !fins && rng.unit() < 0.7 {
                    let mut lower = Module::new(ModuleKind::Limb, limb.length_m * 0.9, girth * 0.12);
                    lower.radius_m = [girth * 0.14, girth * 0.08];
                    lower.attach = 1.0;
                    lower.elevation_deg = rng.range(40.0, 80.0);
                    lower.joint = Some(Joint { dof: 1, range_deg: 120.0 });
                    add(lower, Some(l), &mut modules);
                }
            }
        }
        if rng.unit() < 0.5 {
            let mut fin = Module::new(ModuleKind::Fin, size * 0.2, girth * 0.04);
            fin.width_m = size * rng.range(0.06, 0.15);
            fin.attach = 0.4;
            fin.elevation_deg = 150.0;
            fin.around_deg = 0.0;
            add(fin, Some(t), &mut modules);
        }
        if rng.unit() < 0.3 {
            let mut shell = Module::new(ModuleKind::Shell, size * 0.45, girth * 1.25);
            shell.attach = 0.25;
            shell.elevation_deg = 0.0;
            shell.pigments = vec![second];
            add(shell, Some(root), &mut modules);
        }
    }
    // Organes internes de la vue anatomie.
    let mut gut = Module::new(ModuleKind::Organ(System::Digestive), size * 0.5, girth * 0.3);
    gut.attach = 0.05;
    gut.curvature = 0.05;
    add(gut, Some(root), &mut modules);
    let mut heart = Module::new(ModuleKind::Organ(System::Circulatory), girth * 0.4, girth * 0.2);
    heart.attach = 0.75;
    add(heart, Some(root), &mut modules);
    let mut nerve = Module::new(ModuleKind::Organ(System::Nervous), size * 0.55, girth * 0.06);
    nerve.attach = 0.02;
    add(nerve, Some(root), &mut modules);
    let mut gonad = Module::new(ModuleKind::Organ(System::Reproductive), girth * 0.5, girth * 0.18);
    gonad.attach = 0.2;
    add(gonad, Some(root), &mut modules);
    let pattern = Pattern {
        feed: rng.range(0.022, 0.05),
        kill: rng.range(0.055, 0.065),
        contrast: if rng.unit() < 0.7 { rng.range(0.4, 1.0) } else { 0.0 },
        scale: rng.range(1.0, 3.0),
    };
    let cell_types = 3 + (rng.next_u64() % 60) as u8;
    BodyPlan { version: PLAN_VERSION, modules, cell_types, pattern, seed }
}
