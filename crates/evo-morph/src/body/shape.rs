//! Squelette de forme : chaque module du plan, placé dans le repère du corps
//! avec sa symétrie appliquée, devient une primitive paramétrée (tube à
//! section variable, ellipsoïde, lame, filament), puis toutes sont fusionnées
//! en un champ de distance signée par union douce.
//!
//! Repère du corps : x vers l'avant, y vers le dos, z vers la droite, en
//! mètres.

use super::plan::{BodyPlan, Module, ModuleKind, Symmetry};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V3(pub f32, pub f32, pub f32);

// Opérations nommées plutôt que surchargées : le code des formes se lit
// comme des formules.
#[allow(clippy::should_implement_trait)]
impl V3 {
    pub const X: V3 = V3(1.0, 0.0, 0.0);
    pub const Y: V3 = V3(0.0, 1.0, 0.0);
    pub fn add(self, o: V3) -> V3 {
        V3(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    pub fn sub(self, o: V3) -> V3 {
        V3(self.0 - o.0, self.1 - o.1, self.2 - o.2)
    }
    pub fn mul(self, k: f32) -> V3 {
        V3(self.0 * k, self.1 * k, self.2 * k)
    }
    pub fn dot(self, o: V3) -> f32 {
        self.0 * o.0 + self.1 * o.1 + self.2 * o.2
    }
    pub fn cross(self, o: V3) -> V3 {
        V3(self.1 * o.2 - self.2 * o.1, self.2 * o.0 - self.0 * o.2, self.0 * o.1 - self.1 * o.0)
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-12 {
            self.mul(1.0 / l)
        } else {
            V3::X
        }
    }
    pub fn min(self, o: V3) -> V3 {
        V3(self.0.min(o.0), self.1.min(o.1), self.2.min(o.2))
    }
    pub fn max(self, o: V3) -> V3 {
        V3(self.0.max(o.0), self.1.max(o.1), self.2.max(o.2))
    }
}

/// Forme d'une primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Form {
    /// Tube (cône arrondi) de `a` à `b`, rayons `ra` et `rb`.
    Tube { a: V3, b: V3, ra: f32, rb: f32 },
    /// Ellipsoïde de centre `c`, demi-axes le long de `u`, `v`, `w`.
    Ellipsoid { c: V3, u: V3, v: V3, w: V3, r: V3 },
}

/// Une primitive placée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Primitive {
    pub form: Form,
    /// Module du plan d'où elle vient, et copie (symétrie) dans le corps.
    pub module: u16,
    pub instance: u16,
    /// Os de l'animation : une instance placée de module.
    pub bone: u16,
    pub internal: bool,
    /// Pièce rigide : fusion franche plutôt que douce.
    pub rigid: bool,
    pub colour: [f32; 3],
    /// Échelle typique, pour l'union douce.
    pub size: f32,
}

impl Primitive {
    pub fn distance(&self, p: V3) -> f32 {
        match self.form {
            Form::Tube { a, b, ra, rb } => round_cone(p, a, b, ra, rb),
            Form::Ellipsoid { c, u, v, w, r } => {
                let d = p.sub(c);
                ellipsoid(V3(d.dot(u), d.dot(v), d.dot(w)), r)
            }
        }
    }

    /// La même primitive, mise à l'échelle `k` (les calculs de distance se
    /// font sur un corps de taille unité : au micromètre, les carrés de
    /// longueurs sortiraient de la précision des flottants).
    pub fn scaled(&self, k: f32) -> Primitive {
        let form = match self.form {
            Form::Tube { a, b, ra, rb } => Form::Tube { a: a.mul(k), b: b.mul(k), ra: ra * k, rb: rb * k },
            Form::Ellipsoid { c, u, v, w, r } => Form::Ellipsoid { c: c.mul(k), u, v, w, r: r.mul(k) },
        };
        Primitive { form, size: self.size * k, ..*self }
    }

    pub fn bounds(&self) -> (V3, V3) {
        match self.form {
            Form::Tube { a, b, ra, rb } => {
                let r = ra.max(rb);
                let e = V3(r, r, r);
                (a.min(b).sub(e), a.max(b).add(e))
            }
            Form::Ellipsoid { c, r, .. } => {
                let m = r.0.max(r.1).max(r.2);
                let e = V3(m, m, m);
                (c.sub(e), c.add(e))
            }
        }
    }
}

/// Cône arrondi (Inigo Quilez) : distance exacte.
fn round_cone(p: V3, a: V3, b: V3, r1: f32, r2: f32) -> f32 {
    let ba = b.sub(a);
    let l2 = ba.dot(ba);
    if l2 < 1e-14 {
        return p.sub(a).len() - r1.max(r2);
    }
    let rr = r1 - r2;
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;
    let pa = p.sub(a);
    let y = pa.dot(ba);
    let z = y - l2;
    let xv = pa.mul(l2).sub(ba.mul(y));
    let x2 = xv.dot(xv);
    let y2 = y * y * l2;
    let z2 = z * z * l2;
    let k = rr.signum() * rr * rr * x2;
    if z.signum() * a2 * z2 > k {
        return (x2 + z2).sqrt() * il2 - r2;
    }
    if y.signum() * a2 * y2 < k {
        return (x2 + y2).sqrt() * il2 - r1;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - r1
}

/// Ellipsoïde (borne d'Inigo Quilez, assez juste près de la surface).
fn ellipsoid(p: V3, r: V3) -> f32 {
    let k0 = V3(p.0 / r.0, p.1 / r.1, p.2 / r.2).len();
    let k1 = V3(p.0 / (r.0 * r.0), p.1 / (r.1 * r.1), p.2 / (r.2 * r.2)).len();
    if k1 < 1e-12 {
        return -r.0.min(r.1).min(r.2);
    }
    k0 * (k0 - 1.0) / k1
}

/// Union douce polynomiale.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Repère d'un module placé : origine, axe, direction du dos.
#[derive(Clone, Copy, Debug)]
struct Frame {
    origin: V3,
    axis: V3,
    up: V3,
    length: f32,
    radius: [f32; 2],
}

impl Frame {
    fn radius_at(&self, t: f32) -> f32 {
        self.radius[0] + (self.radius[1] - self.radius[0]) * t
    }
}

/// Une instance placée d'un module (un os).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    pub module: u16,
    pub parent: Option<u16>,
    pub head: V3,
    pub tail: V3,
}

/// Le corps placé : primitives et os.
#[derive(Clone, Debug, Default)]
pub struct Shape {
    pub primitives: Vec<Primitive>,
    pub bones: Vec<Bone>,
}

/// Couleur d'un module : son pigment dominant, sinon un ocre pâle.
fn module_colour(m: &Module) -> [f32; 3] {
    match m.pigments.first() {
        Some(c) => [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0],
        None => [0.85, 0.74, 0.55],
    }
}

/// Place le plan : chaque module, ses copies par symétrie et leurs
/// descendants.
pub fn place(plan: &BodyPlan) -> Shape {
    let mut shape = Shape::default();
    if !plan.is_valid() {
        return shape;
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); plan.modules.len()];
    for (i, m) in plan.modules.iter().enumerate().skip(1) {
        children[m.parent.unwrap() as usize].push(i);
    }
    let root = &plan.modules[0];
    let frame = Frame { origin: V3::X.mul(-root.length_m / 2.0), axis: V3::X, up: V3::Y, length: root.length_m, radius: root.radius_m };
    emit(plan, &children, 0, frame, None, 0, &mut shape);
    shape
}

fn emit(plan: &BodyPlan, children: &[Vec<usize>], i: usize, f: Frame, parent_bone: Option<u16>, instance: u16, shape: &mut Shape) {
    let m = &plan.modules[i];
    let bone = shape.bones.len() as u16;
    let tip = curved_tip(&f, m.curvature);
    shape.bones.push(Bone { module: i as u16, parent: parent_bone, head: f.origin, tail: tip });
    push_primitives(m, i as u16, instance, bone, &f, shape);
    for &c in &children[i] {
        let cm = &plan.modules[c];
        for (k, (attach, around)) in copies(cm).into_iter().enumerate() {
            let t = attach.clamp(0.0, 1.0);
            // Point d'attache sur l'axe (courbé) du parent, déporté à sa
            // surface dans la direction de départ.
            let on_axis = point_on(&f, m.curvature, t);
            let side = f.axis.cross(f.up).norm();
            let radial = f.up.mul(around.cos()).add(side.mul(around.sin())).norm();
            let elev = cm.elevation_deg.to_radians();
            let axis = f.axis.mul(elev.cos()).add(radial.mul(elev.sin())).norm();
            let depth = if cm.internal() { 0.0 } else { 0.8 };
            let origin = on_axis.add(radial.mul(f.radius_at(t) * depth));
            // Le dos de l'enfant : la direction radiale tournée avec lui.
            let up = {
                let u = radial.sub(axis.mul(radial.dot(axis)));
                if u.len() < 1e-4 {
                    f.up
                } else {
                    u.norm()
                }
            };
            let up = if elev.abs() < 1e-3 { f.up } else { up };
            let cf = Frame { origin, axis, up, length: cm.length_m, radius: cm.radius_m };
            emit(plan, children, c, cf, Some(bone), k as u16, shape);
        }
    }
}

/// Copies d'un module par sa symétrie : (point d'attache, angle autour).
fn copies(m: &Module) -> Vec<(f32, f32)> {
    let a = m.around_deg.to_radians();
    match m.symmetry {
        Symmetry::None => vec![(m.attach, a)],
        Symmetry::Bilateral => {
            if (m.around_deg % 180.0).abs() < 1e-3 {
                // Dans le plan sagittal : une seule pièce.
                vec![(m.attach, a)]
            } else {
                vec![(m.attach, a), (m.attach, -a)]
            }
        }
        Symmetry::Radial(n) => {
            let n = n.max(1);
            (0..n).map(|k| (m.attach, a + std::f32::consts::TAU * k as f32 / n as f32)).collect()
        }
        Symmetry::Segmental(n) => {
            let n = n.max(1);
            let end = 0.95_f32.max(m.attach);
            (0..n).map(|k| (m.attach + (end - m.attach) * if n > 1 { k as f32 / (n - 1) as f32 } else { 0.0 }, a)).collect()
        }
    }
}

fn point_on(f: &Frame, curvature: f32, t: f32) -> V3 {
    // Courbe quadratique : le bout dévie vers le dos de `curvature` × longueur.
    f.origin.add(f.axis.mul(f.length * t)).add(f.up.mul(curvature * f.length * t * t))
}

fn curved_tip(f: &Frame, curvature: f32) -> V3 {
    point_on(f, curvature, 1.0)
}

fn push_primitives(m: &Module, module: u16, instance: u16, bone: u16, f: &Frame, shape: &mut Shape) {
    let colour = module_colour(m);
    let internal = m.internal();
    let rigid = matches!(m.kind, ModuleKind::Shell);
    let size = f.radius[0].max(f.radius[1]);
    let mut push = |form: Form, colour: [f32; 3], size: f32| {
        shape.primitives.push(Primitive { form, module, instance, bone, internal, rigid, colour, size });
    };
    let side = f.axis.cross(f.up).norm();
    match m.kind {
        ModuleKind::Trunk | ModuleKind::Segment | ModuleKind::Limb | ModuleKind::Stalk | ModuleKind::Antenna | ModuleKind::Flagellum => {
            // Tube, en trois tronçons s'il est courbé.
            let n = if m.curvature.abs() > 1e-3 { 3 } else { 1 };
            for k in 0..n {
                let (t0, t1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                let a = point_on(f, m.curvature, t0);
                let b = point_on(f, m.curvature, t1);
                push(Form::Tube { a, b, ra: f.radius_at(t0), rb: f.radius_at(t1) }, colour, size);
            }
        }
        ModuleKind::Head | ModuleKind::Shell | ModuleKind::Organ(_) | ModuleKind::Mouth | ModuleKind::Eye(_) => {
            let c = f.origin.add(f.axis.mul(f.length / 2.0));
            let r = V3((f.length / 2.0).max(1e-9), f.radius[0], f.radius[0].max(f.radius[1]));
            let colour = match m.kind {
                ModuleKind::Organ(s) => s.colour(),
                ModuleKind::Mouth => [0.35, 0.22, 0.18],
                ModuleKind::Eye(_) => [0.12, 0.09, 0.08],
                _ => colour,
            };
            push(Form::Ellipsoid { c, u: f.axis, v: f.up, w: side, r }, colour, size);
        }
        ModuleKind::Fin | ModuleKind::Leaf | ModuleKind::Gill => {
            // Lame : ellipsoïde aplati, son plan contient l'axe et le dos.
            let c = f.origin.add(f.axis.mul(f.length / 2.0));
            let w = m.width_m.max(f.radius[0] * 2.0);
            let r = V3((f.length / 2.0).max(1e-9), w / 2.0, f.radius[0].min(w / 8.0).max(1e-9));
            push(Form::Ellipsoid { c, u: f.axis, v: f.up, w: side, r }, colour, size);
        }
    }
}

/// Champ de distance du corps (peau ou organes internes) en un point, avec
/// la primitive la plus proche et la deuxième.
pub fn field(prims: &[Primitive], p: V3) -> (f32, usize, usize) {
    let mut d = f32::INFINITY;
    let (mut best, mut second) = (usize::MAX, usize::MAX);
    let (mut bd, mut sd) = (f32::INFINITY, f32::INFINITY);
    for (i, pr) in prims.iter().enumerate() {
        let di = pr.distance(p);
        if di < bd {
            second = best;
            sd = bd;
            best = i;
            bd = di;
        } else if di < sd {
            second = i;
            sd = di;
        }
        d = if pr.rigid || !d.is_finite() { d.min(di) } else { smin(d, di, 0.45 * pr.size) };
    }
    (d, best, second)
}
