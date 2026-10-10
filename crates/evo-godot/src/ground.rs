//! `EvoGround` : la scène au sol vue depuis Godot (descente au sol, étape 5).
//!
//! La scène se construit en tâche de fond (relief, corps, atlas des
//! imposteurs) ; `is_ready()` dit quand elle est prête. Ensuite, à chaque
//! image, `step()` avance la foule et les tampons des `MultiMesh` se lisent
//! espèce par espèce : transformations d'instance, et une texture des poses
//! (trois texels RGBAF par os, une ligne par instance) que le shader des
//! animaux lit avec `INSTANCE_ID`.
//!
//! Les individus sont des figurants (voir `evo_view::ground`) : rien de ce
//! qu'ils font ne remonte au moteur.

use crate::session::{image_rgba8, image_rgbaf};
use evo_morph::body::{impostor_views, Action, Locomotion, Mesh};
use evo_view::ground::{bench_species, real_species, Crowd, Patch, SceneSpecies, Site, PATCH_SIZE_M};
use godot::classes::Image;
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Vues par espèce et côté d'une vue dans l'atlas des imposteurs.
pub const IMPOSTOR_VIEWS: usize = 8;
pub const IMPOSTOR_PX: usize = 64;

/// Ce que la tâche de fond prépare.
pub struct Built {
    pub crowd: Crowd,
    pub atlas: Vec<u8>,
    pub ms: f64,
}

type Slot = Arc<Mutex<Option<Built>>>;

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct EvoGround {
    base: Base<RefCounted>,
    slot: Slot,
    built: Option<Built>,
    fr: bool,
    /// Durée du dernier `step`, ms.
    step_ms: f64,
}

#[godot_api]
impl IRefCounted for EvoGround {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base, slot: Arc::new(Mutex::new(None)), built: None, fr: true, step_ms: 0.0 }
    }
}

/// Construit la scène : figurants du milieu, puis les espèces vraies.
pub fn build(site: Site, real: Vec<SceneSpecies>, total: usize, seed: u64) -> Built {
    let t0 = Instant::now();
    let patch = Patch::new(site.clone(), seed);
    let mut species = bench_species(&site, seed, 6);
    species.extend(real);
    let mut atlas = vec![0u8; IMPOSTOR_VIEWS * IMPOSTOR_PX * IMPOSTOR_PX * species.len() * 4];
    let row = IMPOSTOR_VIEWS * IMPOSTOR_PX * IMPOSTOR_PX * 4;
    for (k, s) in species.iter().enumerate() {
        atlas[k * row..(k + 1) * row].copy_from_slice(&impostor_views(&s.plan, IMPOSTOR_VIEWS, IMPOSTOR_PX));
    }
    let crowd = Crowd::new(patch, species, total, seed);
    Built { crowd, atlas, ms: t0.elapsed().as_secs_f64() * 1000.0 }
}

impl EvoGround {
    /// Lance la construction en tâche de fond.
    pub fn start(&mut self, fr: bool, work: impl FnOnce() -> Built + Send + 'static) {
        self.fr = fr;
        let slot = self.slot.clone();
        std::thread::spawn(move || {
            let b = work();
            *slot.lock().unwrap() = Some(b);
        });
    }

    fn crowd(&self) -> Option<&Crowd> {
        self.built.as_ref().map(|b| &b.crowd)
    }
}

/// Maillage normalisé d'une espèce, avec les os : UV = (os a, os b),
/// UV2 = (poids a, poids b).
fn species_mesh(s: &SceneSpecies, m: &Mesh) -> VarDictionary {
    let k = if s.body.extent_m > 0.0 { 1.0 / s.body.extent_m } else { 1.0 };
    let c = s.centre;
    let verts: Vec<Vector3> = m.positions.iter().map(|p| Vector3::new((p[0] - c[0]) * k, (p[1] - c[1]) * k, (p[2] - c[2]) * k)).collect();
    let normals: Vec<Vector3> = m.normals.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
    let colours: Vec<Color> = m.colours.iter().map(|c| Color::from_rgba(c[0], c[1], c[2], c[3])).collect();
    let uv: Vec<Vector2> = m.bones.iter().map(|b| Vector2::new(b[0] as f32, b[1] as f32)).collect();
    let uv2: Vec<Vector2> = m.weights.iter().map(|w| Vector2::new(w[0], w[1])).collect();
    // Faces avant dans le sens horaire pour Godot.
    let mut idx = Vec::with_capacity(m.indices.len());
    for t in m.indices.chunks(3) {
        idx.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
    }
    let mut d = VarDictionary::new();
    d.set("vertices", &PackedVector3Array::from(&verts[..]));
    d.set("normals", &PackedVector3Array::from(&normals[..]));
    d.set("colors", &PackedColorArray::from(&colours[..]));
    d.set("uv", &PackedVector2Array::from(&uv[..]));
    d.set("uv2", &PackedVector2Array::from(&uv2[..]));
    d.set("indices", &PackedInt32Array::from(&idx[..]));
    d
}

fn locomotion_label(l: Locomotion, aquatic: bool, fr: bool) -> String {
    match (l, fr) {
        // Sans pattes hors de l'eau, l'ondulation est une reptation.
        (Locomotion::Swim, true) if !aquatic => "rampe".into(),
        (Locomotion::Swim, false) if !aquatic => "crawls".into(),
        (Locomotion::Walk { legs }, true) => format!("marche ({legs} pattes)"),
        (Locomotion::Walk { legs }, false) => format!("walks ({legs} legs)"),
        (Locomotion::Swim, true) => "nage".into(),
        (Locomotion::Swim, false) => "swims".into(),
        (Locomotion::Fly, true) => "vole".into(),
        (Locomotion::Fly, false) => "flies".into(),
        (Locomotion::Flagellar, true) => "nage à flagelle".into(),
        (Locomotion::Flagellar, false) => "flagellar".into(),
        (Locomotion::Sessile, true) => "fixé".into(),
        (Locomotion::Sessile, false) => "sessile".into(),
    }
}

#[godot_api]
impl EvoGround {
    /// Scène de banc d'essai sans partie : un milieu type (mer ou terre),
    /// pour les essais et la porte.
    #[func]
    fn bench(ocean: bool, seed: i64, total: i64) -> Gd<EvoGround> {
        let site = Site {
            cell: 0,
            elevation_m: if ocean { -25.0 } else { 350.0 },
            is_ocean: ocean,
            temperature_k: 291.0,
            ice_cover: 0.0,
            roughness_m: 140.0,
            life_rgb: Some([96, 128, 70]),
            biomass_per_m2: 0.4,
            sky: [0.82, 0.86, 0.88],
        };
        let mut g = Gd::<EvoGround>::default();
        let (seed, total) = (seed as u64, total.clamp(1, 1_000_000) as usize);
        g.bind_mut().start(true, move || build(site, Vec::new(), total, seed));
        g
    }

    #[func]
    fn is_ready(&mut self) -> bool {
        if self.built.is_none() {
            self.built = self.slot.lock().unwrap().take();
        }
        self.built.is_some()
    }

    /// Durée de construction (ms), du dernier pas (ms), effectifs.
    #[func]
    fn stats(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(b) = &self.built {
            d.set("build_ms", b.ms);
            d.set("individuals", b.crowd.individuals.len() as i64);
            d.set("animated", b.crowd.near.len() as i64);
            d.set("species", b.crowd.species.len() as i64);
        }
        d.set("step_ms", self.step_ms);
        d
    }

    /// Le terrain : sommets, normales, couleurs, indices ; taille du carré,
    /// eau (surface à y = 0), teinte du ciel.
    #[func]
    fn terrain(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(c) = self.crowd() else { return d };
        let (pos, nor, col, idx) = c.patch.mesh();
        let verts: Vec<Vector3> = pos.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
        let normals: Vec<Vector3> = nor.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
        let colours: Vec<Color> = col.iter().map(|c| Color::from_rgb(c[0], c[1], c[2])).collect();
        let mut tri = Vec::with_capacity(idx.len());
        for t in idx.chunks(3) {
            tri.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
        }
        d.set("vertices", &PackedVector3Array::from(&verts[..]));
        d.set("normals", &PackedVector3Array::from(&normals[..]));
        d.set("colors", &PackedColorArray::from(&colours[..]));
        d.set("indices", &PackedInt32Array::from(&tri[..]));
        d.set("size", PATCH_SIZE_M as f64);
        d.set("water", c.patch.water);
        let s = c.patch.site.sky;
        d.set("sky", Color::from_rgb(s[0], s[1], s[2]));
        d
    }

    #[func]
    fn species_count(&self) -> i64 {
        self.crowd().map_or(0, |c| c.species.len() as i64)
    }

    /// Nom, taille, figurant ou espèce vraie, locomotion, nombre d'os.
    #[func]
    fn species_info(&self, k: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.crowd().and_then(|c| c.species.get(k as usize)) else { return d };
        d.set("name", s.name.as_str());
        d.set("size_m", s.size_m as f64);
        d.set("bench", s.signature.is_none());
        d.set("predator", s.predator);
        d.set("aquatic", s.aquatic);
        d.set("locomotion", locomotion_label(s.rig.locomotion, s.aquatic, self.fr).as_str());
        d.set("bones", s.bone_count() as i64);
        d.set("individuals", self.crowd().map_or(0, |c| c.individuals.iter().filter(|i| i.species as i64 == k).count() as i64));
        d
    }

    /// Maillage d'une espèce au niveau de détail `lod` (0 = le plus fin).
    #[func]
    fn species_mesh(&self, k: i64, lod: i64) -> VarDictionary {
        let Some(s) = self.crowd().and_then(|c| c.species.get(k as usize)) else { return VarDictionary::new() };
        let lod = (lod.max(0) as usize).min(s.body.lods.len().saturating_sub(1));
        match s.body.lods.get(lod) {
            Some(m) => species_mesh(s, m),
            None => VarDictionary::new(),
        }
    }

    /// Atlas des imposteurs : une ligne par espèce, `views` vues de côté `px`.
    #[func]
    fn impostor_atlas(&self) -> Option<Gd<Image>> {
        let b = self.built.as_ref()?;
        let n = b.crowd.species.len().max(1);
        image_rgba8(IMPOSTOR_VIEWS * IMPOSTOR_PX, IMPOSTOR_PX * n, &b.atlas)
    }

    #[func]
    fn impostor_layout(&self) -> Vector2i {
        Vector2i::new(IMPOSTOR_VIEWS as i32, IMPOSTOR_PX as i32)
    }

    /// Avance la scène de `dt` secondes ; `animated` individus, les plus
    /// proches de `camera`, sont animés.
    #[func]
    fn step(&mut self, dt: f64, camera: Vector3, animated: i64) {
        let t0 = Instant::now();
        if let Some(b) = self.built.as_mut() {
            b.crowd.step(dt as f32, [camera.x, camera.y, camera.z], animated.max(0) as usize);
        }
        self.step_ms = t0.elapsed().as_secs_f64() * 1000.0;
    }

    /// Individus animés d'une espèce (indices dans la foule), dans l'ordre
    /// des lignes de ses tampons.
    #[func]
    fn animated_of(&self, k: i64) -> PackedInt32Array {
        let Some(c) = self.crowd() else { return PackedInt32Array::new() };
        let v: Vec<i32> = c.near.iter().filter(|&&i| c.individuals[i as usize].species as i64 == k).map(|&i| i as i32).collect();
        PackedInt32Array::from(&v[..])
    }

    /// Tampon `MultiMesh` (12 nombres par instance) des individus donnés,
    /// complété par des instances nulles jusqu'à `rows`.
    #[func]
    fn transforms(&self, ids: PackedInt32Array, rows: i64) -> PackedFloat32Array {
        let Some(c) = self.crowd() else { return PackedFloat32Array::new() };
        let rows = (rows.max(ids.len() as i64)) as usize;
        let mut out = vec![0.0f32; rows * 12];
        for (r, &i) in ids.as_slice().iter().enumerate() {
            out[r * 12..r * 12 + 12].copy_from_slice(&c.instance_rows(i as usize));
        }
        PackedFloat32Array::from(&out[..])
    }

    /// Texture des poses : une ligne par individu, trois texels par os
    /// (les trois lignes de la transformation de l'os). `rows` est la
    /// hauteur voulue (au moins le nombre d'individus).
    #[func]
    fn poses(&self, k: i64, ids: PackedInt32Array, rows: i64) -> Option<Gd<Image>> {
        let c = self.crowd()?;
        let s = c.species.get(k as usize)?;
        let bones = s.bone_count().max(1);
        let rows = (rows.max(ids.len() as i64).max(1)) as usize;
        let mut data = vec![0.0f32; bones * 12 * rows];
        for (r, &i) in ids.as_slice().iter().enumerate() {
            let pose = c.pose_of(i as usize);
            let line = &mut data[r * bones * 12..(r + 1) * bones * 12];
            for (b, m) in pose.iter().enumerate().take(bones) {
                line[b * 12..b * 12 + 12].copy_from_slice(&m.rows());
            }
        }
        image_rgbaf(bones * 3, rows, &data)
    }

    /// Tampon des imposteurs : tous les individus (16 nombres : transformation
    /// puis données d'instance = ligne de l'atlas, lacet). Les individus
    /// animés y sont réduits à rien.
    #[func]
    fn impostors(&self) -> PackedFloat32Array {
        let Some(c) = self.crowd() else { return PackedFloat32Array::new() };
        let mut animated = vec![false; c.individuals.len()];
        for &i in &c.near {
            animated[i as usize] = true;
        }
        let mut out = Vec::with_capacity(c.individuals.len() * 16);
        for (i, ind) in c.individuals.iter().enumerate() {
            if animated[i] {
                out.extend_from_slice(&[0.0; 16]);
                continue;
            }
            out.extend_from_slice(&c.instance_rows(i));
            out.extend_from_slice(&[ind.species as f32, ind.heading, 0.0, 0.0]);
        }
        PackedFloat32Array::from(&out[..])
    }

    #[func]
    fn individual_count(&self) -> i64 {
        self.crowd().map_or(0, |c| c.individuals.len() as i64)
    }

    /// Individu sous un rayon de la caméra, sinon -1.
    #[func]
    fn pick(&self, origin: Vector3, dir: Vector3) -> i64 {
        self.crowd().and_then(|c| c.pick([origin.x, origin.y, origin.z], [dir.x, dir.y, dir.z])).map_or(-1, |i| i as i64)
    }

    /// Un individu : position, cap, espèce, action en cours, taille.
    #[func]
    fn individual(&self, i: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(c) = self.crowd() else { return d };
        let Some(ind) = c.individuals.get(i.max(0) as usize) else { return d };
        let s = &c.species[ind.species as usize];
        d.set("position", Vector3::new(ind.x, ind.y, ind.z));
        d.set("heading", ind.heading as f64);
        d.set("species", ind.species as i64);
        d.set("action", ind.action.key());
        d.set("action_label", ind.action.label(self.fr));
        d.set("size_m", (s.size_m * ind.scale) as f64);
        d.set("speed", ind.speed as f64);
        d.set("animated", c.near.contains(&(i as u32)));
        d
    }

    /// Un individu animé de l'espèce `k` près du centre (pour la suivre).
    #[func]
    fn first_of(&self, k: i64) -> i64 {
        let Some(c) = self.crowd() else { return -1 };
        c.individuals
            .iter()
            .enumerate()
            .filter(|(_, i)| i.species as i64 == k)
            .min_by(|a, b| (a.1.x.hypot(a.1.z)).total_cmp(&b.1.x.hypot(b.1.z)))
            .map_or(-1, |(i, _)| i as i64)
    }

    /// Les douze actions du lexique : clé et libellé.
    #[func]
    fn actions(&self) -> VarArray {
        let mut out = VarArray::new();
        for a in Action::ALL {
            let mut d = VarDictionary::new();
            d.set("key", a.key());
            d.set("label", a.label(self.fr));
            out.push(&d.to_variant());
        }
        out
    }

    /// Impose une action (clé du lexique) à une espèce (-1 : toutes) ; une
    /// clé vide rend la main aux comportements.
    #[func]
    fn force_action(&mut self, species: i64, key: GString) {
        let key = key.to_string();
        let action = Action::ALL.into_iter().find(|a| a.key() == key);
        if let Some(b) = self.built.as_mut() {
            let s = (species >= 0).then_some(species as u16);
            b.crowd.force(s, action);
        }
    }
}

/// Espèces vraies d'une cellule, prêtes pour la scène.
pub fn real_for(
    list: Vec<(evo_sim::history::SpeciesView, Option<evo_sim::observation::PopulationView>)>,
    seed: u64,
    aquatic: bool,
) -> Vec<SceneSpecies> {
    real_species(&list, seed, aquatic, 4)
}
