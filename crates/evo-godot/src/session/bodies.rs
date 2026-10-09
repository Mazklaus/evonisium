//! Corps, anatomie et comparateur vus depuis Godot (palier 2 d'evo-morph).
//! Les maillages arrivent en tableaux que GDScript range dans un
//! `ArrayMesh` ; ils sont ramenés à une taille unité, centrés, et la taille
//! vraie voyage à côté pour la barre d'échelle et le comparateur.

use super::*;
use evo_morph::body::{Body, Mesh};
use evo_view::anatomy;

fn mesh_dict(m: &Mesh, centre: [f32; 3], scale: f32) -> VarDictionary {
    let mut d = VarDictionary::new();
    let verts: Vec<Vector3> = m
        .positions
        .iter()
        .map(|p| Vector3::new((p[0] - centre[0]) * scale, (p[1] - centre[1]) * scale, (p[2] - centre[2]) * scale))
        .collect();
    let normals: Vec<Vector3> = m.normals.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
    let colours: Vec<Color> = m.colours.iter().map(|c| Color::from_rgba(c[0], c[1], c[2], c[3])).collect();
    // Godot attend quatre os par sommet.
    let mut bones = Vec::with_capacity(m.bones.len() * 4);
    let mut weights = Vec::with_capacity(m.weights.len() * 4);
    for (b, w) in m.bones.iter().zip(&m.weights) {
        bones.extend_from_slice(&[b[0] as i32, b[1] as i32, 0, 0]);
        weights.extend_from_slice(&[w[0], w[1], 0.0, 0.0]);
    }
    // Godot tient pour faces avant celles qui tournent dans le sens des
    // aiguilles d'une montre : on inverse chaque triangle.
    let mut indices = Vec::with_capacity(m.indices.len());
    for t in m.indices.chunks(3) {
        indices.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
    }
    d.set("vertices", &PackedVector3Array::from(&verts[..]));
    d.set("normals", &PackedVector3Array::from(&normals[..]));
    d.set("colors", &PackedColorArray::from(&colours[..]));
    d.set("bones", &PackedInt32Array::from(&bones[..]));
    d.set("weights", &PackedFloat32Array::from(&weights[..]));
    d.set("indices", &PackedInt32Array::from(&indices[..]));
    d
}

fn centre_of(body: &Body) -> [f32; 3] {
    match body.lods.first() {
        Some(m) if !m.positions.is_empty() => {
            let (lo, hi) = m.bounds();
            [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0]
        }
        _ => [0.0; 3],
    }
}

impl EvoSession {
    /// Plan d'une espèce : sa population la plus abondante dans sa cellule
    /// d'apogée. Lancé en tâche de fond ; la clé sert à le reprendre.
    fn spawn_body(&mut self, species: u32) -> String {
        let key = format!("corps-{species}");
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return key };
        let Some(sv) = f.species(species) else { return key };
        let rx = g.engine.query(Query::Cell { cell: sv.peak_bio_cell });
        let (signature, game_seed) = (sv.signature, f.planet.seed);
        // Le corps ne change qu'avec le génotype : on le refait au plus
        // toutes les 500 étapes.
        self.bodies.spawn(key.clone(), f.state.step / 500, move || {
            let Ok(Answer::Cell(Some(detail))) = rx.recv() else { return None };
            let p = detail.populations.iter().filter(|p| p.species == signature).max_by(|a, b| a.biomass.total_cmp(&b.biomass))?;
            Some(anatomy::plan_for_population(p, game_seed))
        });
        key
    }

    fn traits_dict(&self, entry: &crate::bodies::BodyEntry) -> VarDictionary {
        let t = anatomy::plan_traits(&entry.plan, entry.body.extent_m, self.lang);
        let mut d = VarDictionary::new();
        d.set("size", anatomy::length_text(t.size_m, self.lang).as_str());
        d.set("size_m", t.size_m as f64);
        d.set("symmetry", t.symmetry);
        d.set("modules", t.modules as i64);
        d.set("appendages", t.appendages as i64);
        d.set("eyes", t.eyes as i64);
        d.set("covering", t.covering);
        d.set("cell_types", t.cell_types as i64);
        let mut systems = VarArray::new();
        for s in t.systems {
            let mut sd = VarDictionary::new();
            sd.set("key", s.key());
            sd.set("label", anatomy::system_label(s, self.lang));
            let c = s.colour();
            sd.set("colour", Color::from_rgb(c[0], c[1], c[2]));
            systems.push(&sd.to_variant());
        }
        d.set("systems", &systems);
        d
    }

    fn body_dict(&self, entry: &crate::bodies::BodyEntry) -> VarDictionary {
        let body = &entry.body;
        let mut d = VarDictionary::new();
        let centre = centre_of(body);
        let scale = if body.extent_m > 0.0 { 1.0 / body.extent_m } else { 1.0 };
        let mut lods = VarArray::new();
        for m in &body.lods {
            lods.push(&mesh_dict(m, centre, scale).to_variant());
        }
        d.set("ready", true);
        d.set("lods", &lods);
        d.set("organs", &mesh_dict(&body.organs, centre, scale));
        d.set("extent_m", body.extent_m as f64);
        d.set("gloss", body.gloss as f64);
        d.set("iridescence", body.iridescence as f64);
        d.set("bone_count", body.bones.len() as i64);
        d.set("traits", &self.traits_dict(entry));
        d
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Demande le corps d'une espèce ; `body(clé)` le rend une fois prêt.
    #[func]
    fn request_body(&mut self, species: i64) -> GString {
        GString::from(&self.spawn_body(species.max(0) as u32))
    }

    /// Corps d'un plan tiré au hasard (banc d'essai du générateur).
    #[func]
    fn request_test_body(&mut self, seed: i64) -> GString {
        let key = format!("essai-{seed}");
        self.bodies.spawn(key.clone(), 0, move || Some(evo_morph::body::random_plan(seed as u64)));
        GString::from(&key)
    }

    /// Corps prêt : niveaux de détail (`lods`, du plus fin au plus
    /// grossier), organes internes, taille vraie, brillance, traits. Vide
    /// tant qu'il se calcule.
    #[func]
    fn body(&self, key: GString) -> VarDictionary {
        match self.bodies.get(&key.to_string()) {
            Some(e) => self.body_dict(&e),
            None => VarDictionary::new(),
        }
    }

    /// Comparateur : deux espèces côte à côte. Lignes du tableau (avec
    /// leurs différences), ancêtre commun, et les clés des deux corps.
    #[func]
    fn compare_species(&mut self, a: i64, b: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        let (a, b) = (a.max(0) as u32, b.max(0) as u32);
        let key_a = self.spawn_body(a);
        let key_b = self.spawn_body(b);
        d.set("key_a", key_a.as_str());
        d.set("key_b", key_b.as_str());
        let ia = self.species_info(a as i64);
        let ib = self.species_info(b as i64);
        if ia.is_empty() || ib.is_empty() {
            return d;
        }
        let fr = self.lang == Lang::Fr;
        let mut rows = VarArray::new();
        let mut row = |label: &str, va: String, vb: String| {
            let mut r = VarDictionary::new();
            r.set("label", label);
            r.set("differs", va != vb);
            r.set("a", va.as_str());
            r.set("b", vb.as_str());
            rows.push(&r.to_variant());
        };
        let s = |dct: &VarDictionary, k: &str| dct.get(k).map(|v| v.stringify().to_string()).unwrap_or_default();
        let mets = |dct: &VarDictionary| {
            dct.get("metabolisms")
                .and_then(|v| v.try_to::<PackedStringArray>().ok())
                .map(|a| a.as_slice().iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default()
        };
        row(if fr { "Nom" } else { "Name" }, s(&ia, "scientific"), s(&ib, "scientific"));
        row(if fr { "Métabolisme" } else { "Metabolism" }, mets(&ia), mets(&ib));
        row("Pigment", s(&ia, "pigment"), s(&ib, "pigment"));
        row(if fr { "Statut" } else { "Status" }, s(&ia, "status"), s(&ib, "status"));
        row(if fr { "Biomasse" } else { "Biomass" }, s(&ia, "biomass"), s(&ib, "biomass"));
        row(if fr { "Aire" } else { "Range" }, s(&ia, "range_share"), s(&ib, "range_share"));
        row(if fr { "Âge" } else { "Age" }, s(&ia, "age"), s(&ib, "age"));
        row(if fr { "Lignées" } else { "Lineages" }, s(&ia, "lineages"), s(&ib, "lineages"));
        let (ea, eb) = (self.bodies.get(&key_a), self.bodies.get(&key_b));
        d.set("ready", ea.is_some() && eb.is_some());
        if let (Some(ea), Some(eb)) = (&ea, &eb) {
            let ta = anatomy::plan_traits(&ea.plan, ea.body.extent_m, self.lang);
            let tb = anatomy::plan_traits(&eb.plan, eb.body.extent_m, self.lang);
            row(if fr { "Taille" } else { "Size" }, anatomy::length_text(ta.size_m, self.lang), anatomy::length_text(tb.size_m, self.lang));
            row(if fr { "Symétrie" } else { "Symmetry" }, ta.symmetry.into(), tb.symmetry.into());
            row(if fr { "Revêtement" } else { "Covering" }, ta.covering.into(), tb.covering.into());
            row(if fr { "Appendices" } else { "Appendages" }, ta.appendages.to_string(), tb.appendages.to_string());
            row(if fr { "Types cellulaires" } else { "Cell types" }, ta.cell_types.to_string(), tb.cell_types.to_string());
            let sys = |t: &anatomy::PlanTraits| {
                let v: Vec<&str> = t.systems.iter().map(|s| anatomy::system_label(*s, self.lang)).collect();
                if v.is_empty() {
                    "—".to_string()
                } else {
                    v.join(", ")
                }
            };
            row(if fr { "Organes internes" } else { "Internal organs" }, sys(&ta), sys(&tb));
            // Même échelle : la plus grande des deux occupe le cadre.
            let largest = ta.size_m.max(tb.size_m).max(1e-12);
            d.set("relative_a", (ta.size_m / largest) as f64);
            d.set("relative_b", (tb.size_m / largest) as f64);
        }
        d.set("rows", &rows);
        let lineages = self.lineages();
        match anatomy::common_ancestor(&lineages, a, b) {
            Some(l) => {
                d.set("ancestor", l.signature as i64);
                d.set("ancestor_name", self.species_name(&f, l.signature).as_str());
                d.set("divergence", format::duration(f.state.years - l.born_years, self.lang).as_str());
            }
            None => d.set("ancestor", -1i64),
        }
        d
    }

    /// Espèces vivantes, de la plus abondante à la moins abondante (choix
    /// de la seconde espèce du comparateur).
    #[func]
    fn living_species(&self, max: i64) -> VarArray {
        let mut out = VarArray::new();
        let Some(f) = self.frame() else { return out };
        let mut list: Vec<_> = f.state.species.iter().filter(|s| s.biomass > 0.0).collect();
        list.sort_by(|x, y| y.biomass.total_cmp(&x.biomass).then(x.signature.cmp(&y.signature)));
        for s in list.into_iter().take(max.max(1) as usize) {
            let mut d = VarDictionary::new();
            d.set("species", s.signature as i64);
            d.set("name", self.species_name(&f, s.signature).as_str());
            out.push(&d.to_variant());
        }
        out
    }
}
