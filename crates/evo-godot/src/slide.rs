//! `EvoSlide` : la lame de microscope en 3D vue depuis Godot (voir
//! `evo_view::slide`). Elle se construit en tâche de fond à partir des
//! populations de la cellule ; `is_ready()` dit quand elle est prête. Ensuite,
//! à chaque image, `step()` fait nager les cellules et les tampons des
//! `MultiMesh` se lisent espèce par espèce.

use evo_view::slide::{cell_mesh, Slide, FIELD_UM};
use godot::prelude::*;
use std::sync::{Arc, Mutex};

pub(crate) type Slot = Arc<Mutex<Option<Slide>>>;

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct EvoSlide {
    base: Base<RefCounted>,
    pub(crate) slot: Slot,
    slide: Option<Slide>,
}

#[godot_api]
impl IRefCounted for EvoSlide {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base, slot: Arc::new(Mutex::new(None)), slide: None }
    }
}

#[godot_api]
impl EvoSlide {
    #[func]
    fn is_ready(&mut self) -> bool {
        if self.slide.is_none() {
            self.slide = self.slot.lock().unwrap().take();
        }
        self.slide.is_some()
    }

    /// Taille du champ (x, z) et épaisseur du film d'eau (y), µm.
    #[func]
    fn field(&self) -> Vector3 {
        Vector3::new(FIELD_UM[0], FIELD_UM[1], FIELD_UM[2])
    }

    #[func]
    fn species_count(&self) -> i64 {
        self.slide.as_ref().map_or(0, |s| s.species.len() as i64)
    }

    /// Signature d'espèce, lignée, part de la biomasse, couleur, nombre de
    /// cellules, taille (µm) et nage d'une espèce de la lame.
    #[func]
    fn species_info(&self, k: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.slide.as_ref() else { return d };
        let Some(sp) = s.species.get(k as usize) else { return d };
        d.set("species", sp.species as i64);
        d.set("lineage", sp.lineage as i64);
        d.set("share", sp.share as f64);
        d.set("colour", Color::from_rgb(sp.form.colour[0], sp.form.colour[1], sp.form.colour[2]));
        d.set("cells", s.count_of(k as usize) as i64);
        d.set("length_um", sp.form.extent_um() as f64);
        d.set("speed_um_s", sp.speed_um_s() as f64);
        d
    }

    /// Maillage d'une cellule de l'espèce `k`, en µm.
    #[func]
    fn mesh(&self, k: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(sp) = self.slide.as_ref().and_then(|s| s.species.get(k as usize)) else { return d };
        let m = cell_mesh(&sp.form);
        let verts: Vec<Vector3> = m.positions.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
        let normals: Vec<Vector3> = m.normals.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
        let colours: Vec<Color> = m.colours.iter().map(|c| Color::from_rgba(c[0], c[1], c[2], c[3])).collect();
        // Faces avant dans le sens horaire pour Godot.
        let mut idx = Vec::with_capacity(m.indices.len());
        for t in m.indices.chunks(3) {
            idx.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
        }
        d.set("vertices", &PackedVector3Array::from(&verts[..]));
        d.set("normals", &PackedVector3Array::from(&normals[..]));
        d.set("colors", &PackedColorArray::from(&colours[..]));
        d.set("indices", &PackedInt32Array::from(&idx[..]));
        d
    }

    #[func]
    fn step(&mut self, dt: f64) {
        if let Some(s) = self.slide.as_mut() {
            s.step(dt as f32);
        }
    }

    /// Transformations des cellules de l'espèce `k` (tampon d'un `MultiMesh`).
    #[func]
    fn transforms(&self, k: i64) -> PackedFloat32Array {
        let v = self.slide.as_ref().map(|s| s.transforms(k.max(0) as usize)).unwrap_or_default();
        PackedFloat32Array::from(&v[..])
    }

    /// Espèce (indice) de la cellule sous le rayon, ou −1.
    #[func]
    fn pick(&self, origin: Vector3, dir: Vector3) -> i64 {
        let d = dir.normalized();
        self.slide.as_ref().and_then(|s| s.pick([origin.x, origin.y, origin.z], [d.x, d.y, d.z])).map_or(-1, |k| k as i64)
    }
}
