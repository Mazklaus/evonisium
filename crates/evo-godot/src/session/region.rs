//! Régions et paysages du globe (incrément G3) vus depuis Godot : le
//! maillage subdivisé autour du point regardé, avec l'altitude fine de chaque
//! sommet (UV.x, en mètres) et la cellule dont il lit les données (UV2).

use super::*;
use evo_view::terrain::{self, Triangles};

#[godot_api(secondary)]
impl EvoSession {
    /// Demande la région de `rings` anneaux autour de `cell`, subdivisée
    /// `depth` fois. La clé change avec la cellule, la finesse et l'état
    /// (le relief suit la tectonique) ; `take_region(clé)` la rend.
    #[func]
    fn request_region(&mut self, cell: i64, rings: i64, depth: i64) -> GString {
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return GString::new() };
        if cell < 0 || cell as usize >= grid.len() {
            return GString::new();
        }
        let (rings, depth) = (rings.clamp(1, 24) as usize, depth.clamp(0, 6) as u32);
        // Le relief ne bouge qu'à l'échelle des millions d'années : une
        // région par tranche de 200 pas suffit.
        let key = format!("{cell}-{rings}-{depth}-{}", f.state.step / 200);
        if self.region.asked == key {
            return GString::from(&key);
        }
        self.region.asked = key.clone();
        let done = self.region.done.clone();
        let cache = self.region.triangles.clone();
        let seed = f.planet.seed;
        let k = key.clone();
        std::thread::spawn(move || {
            let tris = {
                let mut slot = cache.lock().unwrap();
                slot.get_or_insert_with(|| Arc::new(Triangles::new(&grid))).clone()
            };
            let cells = f.cells();
            let elevation: Vec<f32> = cells.iter().map(|c| c.elevation_m).collect();
            let ice: Vec<f32> = cells.iter().map(|c| c.ice_cover).collect();
            let ocean: Vec<bool> = cells.iter().map(|c| c.is_ocean).collect();
            let relief = terrain::reliefs(&grid, &elevation, &ice, &ocean);
            let r = terrain::region(&grid, &tris, &relief, cell as usize, rings, depth, seed ^ 0x007E_44A1);
            *done.lock().unwrap() = Some((k, r));
        });
        GString::from(&key)
    }

    /// Cellule la plus haute de la planète (aller voir un massif), hors
    /// des glaces si `ice_free`.
    #[func]
    fn highest_cell(&self, ice_free: bool) -> i64 {
        let Some(f) = self.frame() else { return -1 };
        f.cells()
            .iter()
            .enumerate()
            .filter(|(_, c)| !ice_free || c.ice_cover < 0.2)
            .max_by(|a, b| a.1.elevation_m.total_cmp(&b.1.elevation_m))
            .map_or(-1, |(i, _)| i as i64)
    }

    /// Région prête pour cette clé : sommets (sphère unité), UV2 (texel de
    /// la cellule), UV (altitude fine en m, 0), indices, rayon angulaire du
    /// disque couvert et direction du centre. Vide tant qu'elle se calcule.
    #[func]
    fn take_region(&mut self, key: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        let mut slot = self.region.done.lock().unwrap();
        let Some((k, r)) = slot.as_ref() else { return d };
        if *k != key.to_string() {
            return d;
        }
        let verts: Vec<Vector3> = r.positions.iter().map(|p| to_godot([p[0] as f64, p[1] as f64, p[2] as f64])).collect();
        let uv2: Vec<Vector2> = r.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
        let uv: Vec<Vector2> = r.heights.iter().map(|h| Vector2::new(*h, 0.0)).collect();
        // Godot attend des triangles dans le sens horaire vu de face.
        let mut idx: Vec<i32> = Vec::with_capacity(r.indices.len());
        for t in r.indices.chunks(3) {
            idx.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
        }
        d.set("vertices", &PackedVector3Array::from(&verts[..]));
        d.set("uv2", &PackedVector2Array::from(&uv2[..]));
        d.set("uv", &PackedVector2Array::from(&uv[..]));
        d.set("indices", &PackedInt32Array::from(&idx[..]));
        let custom0: Vec<f32> = r.uvs_bc.iter().flatten().copied().collect();
        let custom1: Vec<f32> = r.weights.iter().flat_map(|w| [w[0], w[1], w[2], 0.0]).collect();
        d.set("custom0", &PackedFloat32Array::from(&custom0[..]));
        // Gradient de l'altitude, dans le repère de Godot.
        let custom2: Vec<f32> = r
            .gradients
            .iter()
            .flat_map(|g| {
                let v = to_godot([g[0] as f64, g[1] as f64, g[2] as f64]);
                [v.x, v.y, v.z]
            })
            .collect();
        d.set("custom2", &PackedFloat32Array::from(&custom2[..]));
        d.set("custom1", &PackedFloat32Array::from(&custom1[..]));
        d.set("covered_radius", r.covered_radius as f64);
        if let Some(g) = &self.grid {
            d.set("centre", to_godot(g.centers[r.centre]));
        }
        *slot = None;
        d
    }
}
