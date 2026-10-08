//! `EvoSession` : la partie vue depuis Godot.
//!
//! Le client GDScript ne parle qu'à cette classe. Elle lit l'image figée du
//! dernier pas (jamais l'état pendant qu'il change), prépare les textures de
//! données du globe, met en forme les textes, et transmet au moteur les deux
//! seules choses que le client écrit : des ordres et la zone d'intérêt.

use crate::jobs::Jobs;
use crate::runner::{step_years_for, Command, Observation, Runner};
use evo_core::events::{Event, EventKind, Origin};
use evo_planet::grid::GeodesicGrid;
use evo_planet::Gas;
use evo_sim::{OrderKind, Seeding};
use evo_view::chronicle::{self, Action, Family, StopRules};
use evo_view::format::{self, Lang};
use evo_view::frame::{Frame, LineageFrame};
use evo_view::layers::{self, Focus, Transform};
use evo_view::mesh::GlobeMesh;
use evo_view::naming::{guild_common, main_reaction, metabolism_list, species_names};
use evo_view::palette::{self, PaletteKind, VisionMode};
use evo_view::pick::CellLocator;
use evo_view::save::{PlanetSpec, SaveFile};
use evo_view::species;
use godot::classes::image::Format as ImageFormat;
use godot::classes::Image;
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

fn f32_bytes(v: &[f32]) -> PackedByteArray {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    PackedByteArray::from(&bytes[..])
}

fn image_rgbaf(w: usize, h: usize, data: &[f32]) -> Option<Gd<Image>> {
    Image::create_from_data(w as i32, h as i32, false, ImageFormat::RGBAF, &f32_bytes(data))
}

pub fn image_rgba8(w: usize, h: usize, data: &[u8]) -> Option<Gd<Image>> {
    Image::create_from_data(w as i32, h as i32, false, ImageFormat::RGBA8, &PackedByteArray::from(data))
}

/// Le moteur place le pôle nord sur z ; Godot met le haut sur y. La
/// conversion (une rotation d'un quart de tour autour de x) se fait ici, à
/// la frontière, et nulle part ailleurs.
fn to_godot(p: [f64; 3]) -> Vector3 {
    Vector3::new(p[0] as f32, p[2] as f32, -p[1] as f32)
}

fn from_godot(v: Vector3) -> [f64; 3] {
    [v.x as f64, -v.z as f64, v.y as f64]
}

fn colour(c: [f32; 3]) -> Color {
    Color::from_rgb(c[0], c[1], c[2])
}

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct EvoSession {
    base: Base<RefCounted>,
    runner: Option<Runner>,
    grid: Option<Arc<GeodesicGrid>>,
    locator: Option<CellLocator>,
    lang: Lang,
    celsius: bool,
    focus: Focus,
    /// Pas de la dernière image dont les textures ont été données.
    textures_step: Option<u64>,
    events_polled: usize,
    jobs: Jobs,
    /// Biomasse des espèces regardées, pas après pas (statut des fiches).
    watch: HashMap<u32, Vec<f64>>,
    watch_step: u64,
    bridge_ms: f64,
    pace: f64,
}

#[godot_api]
impl IRefCounted for EvoSession {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            runner: None,
            grid: None,
            locator: None,
            lang: Lang::Fr,
            celsius: true,
            focus: Focus::default(),
            textures_step: None,
            events_polled: 0,
            jobs: Jobs::default(),
            watch: HashMap::new(),
            watch_step: 0,
            bridge_ms: 0.0,
            pace: crate::runner::DEFAULT_SPEED,
        }
    }
}

impl EvoSession {
    fn frame(&self) -> Option<Arc<Frame>> {
        self.runner.as_ref().and_then(|r| r.frame())
    }

    fn lineages(&self) -> Arc<Vec<LineageFrame>> {
        self.runner.as_ref().map(|r| r.shared.lineages.lock().unwrap().clone()).unwrap_or_default()
    }

    fn set_grid(&mut self, level: u32) {
        if self.grid.as_ref().map(|g| g.level) != Some(level) {
            let g = GeodesicGrid::new(level);
            self.locator = Some(CellLocator::new(&g));
            self.grid = Some(Arc::new(g));
        }
    }

    fn order(&self, kind: OrderKind) {
        if let Some(r) = &self.runner {
            r.send(Command::Order(kind));
        }
    }

    fn name_of(&self, lineages: &[LineageFrame], id: u32) -> String {
        let (Some(grid), Some(r)) = (&self.grid, &self.runner) else { return format!("n° {id}") };
        match lineages.get(id as usize).filter(|l| l.id == id) {
            Some(l) => species_names(r.seed, grid, id, l.signature, l.origin_cell, self.lang).common,
            None => format!("n° {id}"),
        }
    }

    fn event_dict(&self, e: &Event, lineages: &[LineageFrame], rules: Option<&StopRules>) -> VarDictionary {
        let name = |id: u32| self.name_of(lineages, id);
        let level = chronicle::level(e, self.pace);
        let action = rules.map_or(Action::Note, |r| r.decide(e, self.pace));
        let lineage = match &e.kind {
            EventKind::LifeSeeded { lineage }
            | EventKind::NewLineage { lineage, .. }
            | EventKind::LineageExtinct { lineage }
            | EventKind::Innovation { lineage, .. } => *lineage as i64,
            _ => -1,
        };
        let mut d = VarDictionary::new();
        d.set("id", e.id as i64);
        d.set("years", e.years);
        d.set("date", format::duration(e.years, self.lang));
        d.set("text", chronicle::body(e, self.lang, &name));
        d.set("family", Family::of(e).key());
        d.set("family_label", Family::of(e).label(self.lang));
        d.set("level", level as i64);
        d.set("action", action.index());
        d.set("cell", e.cell.map_or(-1, |c| c as i64));
        d.set("lineage", lineage);
        d.set("player", e.origin == Origin::Player);
        d.set("accelerator", e.origin == Origin::Accelerator);
        d.set("interest", e.interest);
        d.set("cause", e.cause.map_or(-1, |c| c as i64));
        d
    }

    fn temperature(&self, k: f64) -> String {
        format::temperature(k, self.celsius, self.lang)
    }
}

#[godot_api]
impl EvoSession {
    // ------------------------------------------------------------------
    // Partie

    /// Nouvelle partie en pause, sans vie. `star_k` ≤ 0 garde l'étoile du
    /// préréglage.
    #[func]
    fn start(&mut self, preset: GString, seed: i64, level: i64, orbit: f64, water: f64, star_k: f64) -> bool {
        let spec = PlanetSpec {
            preset: preset.to_string(),
            orbit_factor: orbit,
            water_factor: water,
            star_temperature_k: (star_k > 0.0).then_some(star_k),
        };
        if evo_planet::PlanetParams::by_key(&spec.preset).is_none() {
            return false;
        }
        let level = level.clamp(2, 7) as u32;
        self.runner = None;
        self.set_grid(level);
        self.reset_view();
        self.runner = Some(Runner::start(spec, seed as u64, level, Seeding::Vents));
        true
    }

    /// Reprend un point de sauvegarde ; renvoie un message d'erreur, ou une
    /// chaîne vide.
    #[func]
    fn load_save(&mut self, path: GString) -> GString {
        let text = match std::fs::read_to_string(path.to_string()) {
            Ok(t) => t,
            Err(e) => return GString::from(&format!("lecture impossible : {e}")),
        };
        match SaveFile::from_text(&text) {
            Ok(save) => {
                self.runner = None;
                self.set_grid(save.level);
                self.reset_view();
                self.runner = Some(Runner::load(save));
                GString::new()
            }
            Err(e) => GString::from(&e),
        }
    }

    /// Nom, date et planète d'un point de sauvegarde, sans le charger.
    #[func]
    fn describe_save(&self, path: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Ok(save) = std::fs::read_to_string(path.to_string()).map_err(|e| e.to_string()).and_then(|t| SaveFile::from_text(&t)) {
            d.set("name", save.name.as_str());
            d.set("date", format::game_date(save.years, self.lang));
            d.set("planet", save.spec.to_params().name.as_str());
            d.set("code", save.spec.code(save.seed, save.level).as_str());
            d.set("steps", save.steps as i64);
        }
        d
    }

    fn reset_view(&mut self) {
        self.textures_step = None;
        self.events_polled = 0;
        self.watch.clear();
        self.focus = Focus::default();
        self.jobs = Jobs::default();
    }

    #[func]
    fn stop(&mut self) {
        self.runner = None;
    }

    #[func]
    fn is_running(&self) -> bool {
        self.runner.is_some()
    }

    /// Rejeu en cours : (pas faits, pas visés), ou (−1, −1).
    #[func]
    fn loading_progress(&self) -> Vector2 {
        match self.runner.as_ref().and_then(|r| *r.shared.loading.lock().unwrap()) {
            Some((a, b)) => Vector2::new(a as f32, b as f32),
            None => Vector2::new(-1.0, -1.0),
        }
    }

    #[func]
    fn save(&self, path: GString, name: GString) {
        if let Some(r) = &self.runner {
            r.send(Command::Save { path: path.to_string(), name: name.to_string() });
        }
    }

    /// Message du moteur (sauvegarde écrite…), ou chaîne vide.
    #[func]
    fn take_notice(&self) -> GString {
        self.runner.as_ref().and_then(|r| r.shared.notices.lock().unwrap().pop_front()).map_or(GString::new(), |s| GString::from(&s))
    }

    #[func]
    fn planet_code(&self) -> GString {
        match &self.runner {
            Some(r) => GString::from(&r.spec.code(r.seed, r.level)),
            None => GString::new(),
        }
    }

    // ------------------------------------------------------------------
    // File d'ordres et canal d'observation

    /// « sources » (près des sources hydrothermales) ou « mers » (toutes
    /// les mers).
    #[func]
    fn set_seeding(&self, kind: GString) {
        if let Some(r) = &self.runner {
            r.send(Command::SetSeeding(if kind == "mers" { Seeding::AllOcean } else { Seeding::Vents }));
        }
    }

    #[func]
    fn seed_life(&self) {
        self.order(OrderKind::SeedLife);
    }

    /// Vitesse en années de jeu par seconde : la cadence du fil, et la durée
    /// du pas par un ordre (elle change l'histoire, donc le rejeu la garde).
    #[func]
    fn set_speed(&mut self, years_per_second: f64) {
        // La durée du pas en vigueur est celle du monde, pas celle de la
        // dernière demande (nouvelle partie, partie rechargée).
        let old = self.frame().map(|f| f.step_years);
        self.pace = years_per_second.max(1.0);
        if let Some(r) = &self.runner {
            r.send(Command::Pace(self.pace));
            let new = step_years_for(self.pace);
            if old.is_none_or(|o| (new - o).abs() > 1e-9) {
                r.send(Command::Order(OrderKind::SetStepYears(new)));
            }
        }
    }

    #[func]
    fn speed(&self) -> f64 {
        self.pace
    }

    #[func]
    fn pause(&self) {
        self.order(OrderKind::Pause);
    }

    #[func]
    fn resume(&self) {
        self.order(OrderKind::Resume);
    }

    /// Intervention sur l'environnement : « phosphate » (mol), « eruption »
    /// (mol de CO₂), « methane » (mol de CH₄).
    #[func]
    fn intervene(&self, kind: GString, amount: f64) -> bool {
        let order = match kind.to_string().as_str() {
            "phosphate" => OrderKind::AddPhosphate { moles: amount },
            "eruption" => OrderKind::InjectGas { gas: Gas::Co2, moles: amount },
            "methane" => OrderKind::InjectGas { gas: Gas::Ch4, moles: amount },
            _ => return false,
        };
        self.order(order);
        true
    }

    /// Réserve d'influence : publiée par le volet moteur de l'étape 3. En
    /// attendant, `available` vaut faux et les interventions sont libres.
    #[func]
    fn influence(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        d.set("available", false);
        d
    }

    #[func]
    fn mark_lineage(&mut self, lineage: i64) {
        if lineage >= 0 {
            self.order(OrderKind::MarkLineage { lineage: lineage as u32 });
            if !self.focus.marked.contains(&(lineage as u32)) {
                self.focus.marked.push(lineage as u32);
                self.textures_step = None;
            }
        }
    }

    /// Zone d'intérêt de la caméra : sans effet sur l'histoire.
    #[func]
    fn observe(&self, centre: Vector3, radius_rad: f64, band: i64) {
        if let Some(r) = &self.runner {
            r.send(Command::Observe(Observation { centre: from_godot(centre), radius_rad, band: band.clamp(1, 6) as u8 }));
        }
    }

    // ------------------------------------------------------------------
    // Règles d'arrêt, langue, unités

    #[func]
    fn set_rules_profile(&self, name: GString) {
        if let Some(r) = &self.runner {
            *r.shared.rules.lock().unwrap() = Some(StopRules::profile(&name.to_string()));
        }
    }

    #[func]
    fn set_rule(&self, family: GString, action: i64) {
        if let (Some(r), Some(f)) = (&self.runner, Family::from_key(&family.to_string())) {
            let mut rules = r.shared.rules.lock().unwrap();
            rules.get_or_insert_with(|| StopRules::profile("naturaliste")).set(f, Action::from_index(action));
        }
    }

    /// Règles en cours : famille → [libellé, action].
    #[func]
    fn rules(&self) -> VarArray {
        let mut out = VarArray::new();
        let rules =
            self.runner.as_ref().and_then(|r| r.shared.rules.lock().unwrap().clone()).unwrap_or_else(|| StopRules::profile("naturaliste"));
        for f in Family::ALL {
            let mut d = VarDictionary::new();
            d.set("key", f.key());
            d.set("label", f.label(self.lang));
            d.set("action", rules.action_for(f).index());
            out.push(&d.to_variant());
        }
        out
    }

    /// Événement qui a mis le temps en pause, une seule fois (−1 sinon).
    #[func]
    fn take_auto_pause(&self) -> i64 {
        self.runner.as_ref().and_then(|r| r.shared.auto_paused_by.lock().unwrap().take()).map_or(-1, |id| id as i64)
    }

    #[func]
    fn set_language(&mut self, code: GString) {
        self.lang = Lang::from_code(&code.to_string());
    }

    #[func]
    fn set_celsius(&mut self, celsius: bool) {
        self.celsius = celsius;
    }

    // ------------------------------------------------------------------
    // Image du pas

    #[func]
    fn has_frame(&self) -> bool {
        self.frame().is_some()
    }

    /// Vrai si un pas nouveau a été publié depuis les dernières textures.
    #[func]
    fn has_new_frame(&self) -> bool {
        match self.frame() {
            Some(f) => self.textures_step != Some(f.step) || self.textures_step.is_none(),
            None => false,
        }
    }

    #[func]
    fn frame_info(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        let g = &f.globals;
        d.set("step", f.step as i64);
        d.set("years", f.years);
        d.set("date", format::game_date(f.years, self.lang));
        d.set("step_years", f.step_years);
        d.set("paused", f.paused);
        d.set("climate", f.climate_mode.label());
        d.set("o2", g.o2_mixing);
        d.set("o2_text", format::power_of_ten_in(self.lang, g.o2_mixing));
        d.set("co2_pa", g.co2_pa);
        d.set("ch4_ppb", g.ch4_ppb);
        d.set("pressure_pa", g.pressure_pa);
        d.set("temperature_k", g.mean_temperature_k);
        d.set("temperature_text", self.temperature(g.mean_temperature_k));
        d.set("ice", g.ice_fraction);
        d.set("ocean", g.ocean_fraction);
        d.set("biomass", g.biomass);
        d.set("biomass_text", format::power_of_ten_in(self.lang, g.biomass));
        d.set("lineages", f.living_lineages as i64);
        d.set("guilds", g.guilds as i64);
        d.set("photosynthesis_stage", g.photosynthesis_stage as i64);
        d.set("accelerator", g.accelerator_on);
        d.set("planet", f.planet.name.as_str());
        d.set("radius_m", f.planet.radius_m);
        d.set("star_k", f.planet.star_temperature_k);
        d.set("star_colour", colour(palette::blackbody(f.planet.star_temperature_k)));
        d.set("cells", f.cells.len() as i64);
        d.set("level", f.planet.level as i64);
        d.set("seed", f.planet.seed as i64);
        let real = self.runner.as_ref().map_or(0.0, |r| r.shared.real_speed());
        d.set("real_speed", real);
        d.set("real_speed_text", if real > 0.0 { format::speed(real, self.lang) } else { "—".into() });
        d.set("bridge_ms", self.bridge_ms);
        d
    }

    /// Couleur du limbe de l'atmosphère : lumière de l'étoile, brume de
    /// méthane orangée, bleu de diffusion d'un air oxygéné.
    #[func]
    fn atmosphere_colour(&self) -> Color {
        let Some(f) = self.frame() else { return Color::from_rgb(0.8, 0.8, 0.8) };
        let star = palette::blackbody(f.planet.star_temperature_k);
        let haze = ((f.globals.ch4_ppb / 5000.0).sqrt() as f32).clamp(0.0, 1.0);
        let blue = ((f.globals.o2_mixing / 0.02).sqrt() as f32).clamp(0.0, 1.0);
        let mut c = [star[0] * 0.8, star[1] * 0.78, star[2] * 0.75];
        let orange = [0.85, 0.55, 0.3];
        let azure = [0.45, 0.62, 0.85];
        for k in 0..3 {
            c[k] += (orange[k] - c[k]) * haze * 0.7;
            c[k] += (azure[k] - c[k]) * blue * 0.8;
        }
        colour(c)
    }

    /// Les trois textures de données du dernier pas (et marque ce pas comme
    /// lu). `previous` donne celles du pas d'avant, pour l'interpolation.
    #[func]
    fn frame_textures(&mut self, previous: bool) -> VarArray {
        let t0 = Instant::now();
        let mut out = VarArray::new();
        let frame = if previous { self.runner.as_ref().and_then(|r| r.previous()) } else { self.frame() };
        let Some(f) = frame else { return out };
        let packed = layers::pack(&f, &self.focus);
        let (w, h) = layers::texture_size(f.cells.len());
        for t in packed.iter() {
            match image_rgbaf(w, h, t) {
                Some(img) => out.push(&img.to_variant()),
                None => out.push(&Variant::nil()),
            }
        }
        if !previous {
            self.textures_step = Some(f.step);
            self.track_watched(&f);
        }
        self.bridge_ms = t0.elapsed().as_secs_f64() * 1000.0;
        out
    }

    /// Pôles et vitesses des plaques : texture 64 × 1 (x, y, z, ω en rad/Ma).
    #[func]
    fn plates_image(&self) -> Option<Gd<Image>> {
        let f = self.frame()?;
        let mut data = vec![0.0f32; 64 * 4];
        for (i, p) in f.plates.iter().take(64).enumerate() {
            // Pôle dans le repère de Godot, comme les sommets du globe.
            data[i * 4..i * 4 + 4].copy_from_slice(&[p.pole[0], p.pole[2], -p.pole[1], p.omega_rad_per_myr]);
        }
        image_rgbaf(64, 1, &data)
    }

    /// Espèce dont l'aire est surlignée (−1 : aucune).
    #[func]
    fn set_focus(&mut self, lineage: i64) {
        let l = (lineage >= 0).then_some(lineage as u32);
        if self.focus.lineage != l {
            self.focus.lineage = l;
            self.textures_step = None;
        }
    }

    #[func]
    fn focus(&self) -> i64 {
        self.focus.lineage.map_or(-1, |l| l as i64)
    }

    // ------------------------------------------------------------------
    // Globe

    /// Maillage du globe : sommets sur la sphère unité, coordonnées de
    /// texel de chaque sommet (UV2), indices.
    #[func]
    fn globe_mesh(&self, tiles: bool) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(grid) = &self.grid else { return d };
        let m = if tiles { GlobeMesh::tiles(grid) } else { GlobeMesh::smooth(grid) };
        let verts: Vec<Vector3> = m.positions.iter().map(|p| to_godot([p[0] as f64, p[1] as f64, p[2] as f64])).collect();
        let uvs: Vec<Vector2> = m
            .cells
            .iter()
            .map(|&c| {
                let uv = layers::texel_uv(c as usize, grid.len());
                Vector2::new(uv[0], uv[1])
            })
            .collect();
        // Godot attend des triangles dans le sens horaire vu de face.
        let mut idx: Vec<i32> = Vec::with_capacity(m.indices.len());
        for t in m.indices.chunks(3) {
            idx.extend_from_slice(&[t[0] as i32, t[2] as i32, t[1] as i32]);
        }
        d.set("vertices", &PackedVector3Array::from(&verts[..]));
        d.set("uv2", &PackedVector2Array::from(&uvs[..]));
        d.set("indices", &PackedInt32Array::from(&idx[..]));
        d.set("cells", grid.len() as i64);
        d.set("texture_height", layers::texture_size(grid.len()).1 as i64);
        d
    }

    /// Cellule sous une direction (repère du globe).
    #[func]
    fn cell_at(&self, direction: Vector3) -> i64 {
        match (&self.grid, &self.locator) {
            (Some(g), Some(l)) => l.locate(g, from_godot(direction)) as i64,
            _ => -1,
        }
    }

    #[func]
    fn cell_centre(&self, cell: i64) -> Vector3 {
        match &self.grid {
            Some(g) if (cell as usize) < g.len() => to_godot(g.centers[cell as usize]),
            _ => Vector3::ZERO,
        }
    }

    /// Flèches des plaques : (position, vitesse en cm/an) sur un
    /// échantillon de cellules (grille du niveau 3).
    #[func]
    fn plate_arrows(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(g)) = (self.frame(), &self.grid) else { return d };
        let n = GeodesicGrid::cell_count_for_level(3).min(g.len());
        let mut pos = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);
        for c in 0..n {
            let p = g.centers[c];
            let v = f.plate_velocity(g, c);
            pos.push(to_godot(p));
            // m/an → cm/an
            vel.push(to_godot([v[0] * 100.0, v[1] * 100.0, v[2] * 100.0]));
        }
        d.set("positions", &PackedVector3Array::from(&pos[..]));
        d.set("velocities", &PackedVector3Array::from(&vel[..]));
        d
    }

    /// Inspecteur de cellule : milieu en clair et populations.
    #[func]
    fn cell_info(&self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(grid)) = (self.frame(), &self.grid) else { return d };
        if cell < 0 || cell as usize >= f.cells.len() {
            return d;
        }
        let c = cell as usize;
        let e = &f.cells[c];
        let lat = grid.latitude(c).to_degrees();
        let p = grid.centers[c];
        let lon = p[1].atan2(p[0]).to_degrees();
        let fr = self.lang == Lang::Fr;
        d.set("cell", cell);
        d.set(
            "place",
            format!(
                "{} {}, {} {}",
                format::number_in(self.lang, lat.abs(), 1),
                if lat >= 0.0 { "N" } else { "S" },
                format::number_in(self.lang, lon.abs(), 1),
                if lon >= 0.0 {
                    "E"
                } else if fr {
                    "O"
                } else {
                    "W"
                }
            ),
        );
        d.set("region", evo_view::naming::place_name(f.planet.seed, evo_view::naming::region_of(grid, c)));
        d.set("is_ocean", e.is_ocean);
        d.set(
            "medium",
            match (fr, e.is_ocean, e.ice_cover > 0.5) {
                (true, true, false) => "mer",
                (true, true, true) => "mer gelée",
                (true, false, false) => "terre émergée",
                (true, false, true) => "glacier",
                (false, true, false) => "sea",
                (false, true, true) => "frozen sea",
                (false, false, false) => "land",
                (false, false, true) => "ice sheet",
            },
        );
        let height = e.height_m as f64;
        d.set(
            "height",
            if e.is_ocean {
                format!("{} {} m", if fr { "profondeur" } else { "depth" }, format::number_in(self.lang, -height, 0))
            } else {
                format!("{} {} m", if fr { "altitude" } else { "elevation" }, format::number_in(self.lang, height, 0))
            },
        );
        d.set("temperature", self.temperature(e.temperature_k as f64));
        d.set("light", format!("{} W·m⁻²", format::number_in(self.lang, e.light_w_m2 as f64, 1)));
        d.set("oxygen", format!("{} mol·m⁻³", format::power_of_ten_in(self.lang, e.oxygen as f64)));
        d.set("ph", format::number_in(self.lang, e.ph as f64, 2));
        d.set("salinity", format!("{} g·kg⁻¹", format::number_in(self.lang, e.salinity as f64, 1)));
        d.set("biomass", format!("{} mol C", format::power_of_ten_in(self.lang, e.biomass as f64)));
        d.set("vent", e.vent);
        d.set("plate", e.plate as i64);
        let lineages = self.lineages();
        let mut pops = VarArray::new();
        let total: f32 = f.populations_of(c).iter().map(|p| p.biomass).sum();
        for p in f.populations_of(c) {
            let mut pd = VarDictionary::new();
            pd.set("lineage", p.lineage as i64);
            pd.set("name", self.name_of(&lineages, p.lineage).as_str());
            pd.set("guild", guild_common(main_reaction(p.signature), self.lang));
            pd.set("share", if total > 0.0 { p.biomass / total } else { 0.0 });
            pd.set("biomass", format::power_of_ten_in(self.lang, p.biomass as f64).as_str());
            pd.set(
                "colour",
                p.pigment_nm.map_or(Color::from_rgb(0.78, 0.6, 0.33), |nm| {
                    let c = evo_life::pigment_colour(nm as f64);
                    Color::from_rgba8(c[0], c[1], c[2], 255)
                }),
            );
            pops.push(&pd.to_variant());
        }
        d.set("populations", &pops);
        d
    }

    // ------------------------------------------------------------------
    // Espèces et arbre du vivant

    #[func]
    fn lineage_name(&self, lineage: i64) -> GString {
        GString::from(&self.name_of(&self.lineages(), lineage.max(0) as u32))
    }

    /// Fiche d'espèce simple.
    #[func]
    fn species_info(&mut self, lineage: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return d };
        let lineages = self.lineages();
        let Some(l) = lineages.get(lineage.max(0) as usize).filter(|l| l.id as i64 == lineage) else { return d };
        let s = species::summary(&f, &grid, l, self.lang);
        self.watch.entry(l.id).or_insert_with(|| vec![s.biomass]);
        let status = species::Status::from_series(
            self.watch.get(&l.id).map_or(&[][..], |v| &v[..]),
            s.extinct_years.is_some() || s.range_cells == 0,
        );
        let fr = self.lang == Lang::Fr;
        d.set("lineage", lineage);
        d.set("scientific", s.names.scientific.as_str());
        d.set("common", s.names.common.as_str());
        let mut mets = PackedStringArray::new();
        for m in &s.metabolisms {
            mets.push(*m);
        }
        d.set("metabolisms", &mets);
        d.set(
            "pigment",
            match s.pigment_nm {
                Some(nm) => format!(
                    "{} {} nm",
                    if fr { "pigment absorbant à" } else { "pigment absorbing at" },
                    format::number_in(self.lang, nm as f64, 0)
                ),
                None => (if fr { "sans pigment" } else { "no pigment" }).to_string(),
            },
        );
        d.set("range_cells", s.range_cells as i64);
        d.set("range_share", format::percent(s.range_share as f64, self.lang));
        d.set("biomass", format!("{} mol C", format::power_of_ten_in(self.lang, s.biomass)));
        d.set("born", format::duration(s.born_years, self.lang));
        d.set("age", format::duration(f.years - s.born_years, self.lang));
        d.set("parent", s.parent.map_or(-1, |p| p as i64));
        d.set("parent_name", s.parent.map_or(String::new(), |p| self.name_of(&lineages, p)));
        d.set("status", status.label(self.lang));
        d.set("extinct", s.extinct_years.is_some());
        d.set("marked", self.focus.marked.contains(&l.id));
        d.set("origin_region", evo_view::naming::place_name(f.planet.seed, evo_view::naming::region_of(&grid, l.origin_cell as usize)));
        d
    }

    /// Arbre du vivant élagué : tableaux parallèles, un noeud par indice.
    #[func]
    fn tree(&self, max_leaves: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        let lineages = self.lineages();
        let t = evo_view::tree::build(&lineages, f.years, max_leaves.max(2) as usize);
        let mut ids = PackedInt32Array::new();
        let mut parents = PackedInt32Array::new();
        let mut born = PackedFloat64Array::new();
        let mut end = PackedFloat64Array::new();
        let mut ys = PackedFloat32Array::new();
        let mut living = PackedByteArray::new();
        let mut collapsed = PackedInt32Array::new();
        let mut names = PackedStringArray::new();
        for n in &t.nodes {
            ids.push(n.lineage as i32);
            parents.push(n.parent as i32);
            born.push(n.born_years);
            end.push(n.end_years);
            ys.push(n.y);
            living.push(n.living as u8);
            collapsed.push(n.collapsed as i32);
            names.push(&self.name_of(&lineages, n.lineage));
        }
        d.set("ids", &ids);
        d.set("parents", &parents);
        d.set("born", &born);
        d.set("end", &end);
        d.set("y", &ys);
        d.set("living", &living);
        d.set("collapsed", &collapsed);
        d.set("names", &names);
        d.set("leaves", t.leaves as i64);
        d.set("from_years", if t.from_years.is_finite() { t.from_years } else { 0.0 });
        d.set("to_years", t.to_years);
        d.set("total", lineages.len() as i64);
        d
    }

    /// Lance le calcul du décor de milieu d'une espèce (tâche de fond).
    #[func]
    fn request_decor(&mut self, lineage: i64, width: i64, height: i64) -> GString {
        let key = format!("decor-{lineage}");
        if let Some(f) = self.frame() {
            if let Some(h) = evo_view::decor::Habitat::of_species(&f, lineage.max(0) as u32) {
                let seed = h.decor_seed(f.planet.seed);
                let (w, hh) = (width.clamp(64, 2048) as usize, height.clamp(32, 1024) as usize);
                self.jobs.spawn(key.clone(), seed, move || (evo_view::decor::paint(&h, seed, w, hh), VarDictionaryLite::default()));
            }
        }
        GString::from(&key)
    }

    /// Lance la vue microscope d'une cellule.
    #[func]
    fn request_microscope(&mut self, cell: i64, width: i64, height: i64) -> GString {
        let key = format!("microscope-{cell}");
        if let Some(f) = self.frame() {
            if (cell as usize) < f.cells.len() {
                let members = species::microscope_members(&f, cell as usize);
                let seed = f.planet.seed ^ ((cell as u64) << 24) ^ (f.step / 50);
                let (w, h) = (width.clamp(64, 2048) as usize, height.clamp(64, 2048) as usize);
                self.jobs.spawn(key.clone(), seed, move || {
                    let (c, _) = evo_morph::microscope_field(&members, seed, w, h);
                    (c, VarDictionaryLite { bar_um: evo_morph::microbe::MICROSCOPE_BAR_UM })
                });
            }
        }
        GString::from(&key)
    }

    /// Lance la figure d'une espèce (planche).
    #[func]
    fn request_figure(&mut self, lineage: i64, width: i64, height: i64) -> GString {
        let key = format!("figure-{lineage}");
        if let Some(f) = self.frame() {
            let range = f.range_of(lineage.max(0) as u32);
            if let Some(&(c, _)) = range.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
                if let Some(p) = f.populations_of(c).iter().find(|p| p.lineage as i64 == lineage) {
                    let form = evo_morph::form(&species::traits_of(p), f.planet.seed);
                    let (w, h) = (width.clamp(64, 2048) as usize, height.clamp(64, 2048) as usize);
                    self.jobs.spawn(key.clone(), form.seed, move || {
                        let (c, bar) = evo_morph::figure(&form, w, h);
                        (c, VarDictionaryLite { bar_um: bar })
                    });
                }
            }
        }
        GString::from(&key)
    }

    /// Image prête d'une tâche de fond, ou rien.
    #[func]
    fn take_image(&mut self, key: GString) -> Option<Gd<Image>> {
        let (c, _) = self.jobs.take(&key.to_string())?;
        image_rgba8(c.width, c.height, &c.pixels)
    }

    /// Longueur de la barre d'échelle de la dernière image prise (µm).
    #[func]
    fn last_scale_bar_um(&self) -> f64 {
        self.jobs.last_bar_um as f64
    }

    // ------------------------------------------------------------------
    // Chronique et historique

    /// Événements nouveaux depuis le dernier appel.
    #[func]
    fn poll_events(&mut self) -> VarArray {
        let mut out = VarArray::new();
        let Some(r) = &self.runner else { return out };
        let events = r.shared.events.lock().unwrap();
        if self.events_polled >= events.len() {
            return out;
        }
        let rules = r.shared.rules.lock().unwrap().clone();
        let lineages = self.lineages();
        // Au plus 200 par appel : le reste reste dans la chronique.
        let start = self.events_polled.max(events.len().saturating_sub(200));
        for e in &events[start..] {
            out.push(&self.event_dict(e, &lineages, rules.as_ref()).to_variant());
        }
        let n = events.len();
        drop(events);
        self.events_polled = n;
        out
    }

    #[func]
    fn events_count(&self) -> i64 {
        self.runner.as_ref().map_or(0, |r| r.shared.events.lock().unwrap().len() as i64)
    }

    /// Page de la chronique, du plus récent au plus ancien, filtrée par
    /// famille (vide : toutes) et par texte.
    #[func]
    fn events_page(&self, offset: i64, count: i64, family: GString, search: GString) -> VarArray {
        let mut out = VarArray::new();
        let Some(r) = &self.runner else { return out };
        let events = r.shared.events.lock().unwrap().clone();
        let lineages = self.lineages();
        let family = Family::from_key(&family.to_string());
        let search = search.to_string().to_lowercase();
        let mut skipped = 0;
        for e in events.iter().rev() {
            if family.is_some_and(|f| Family::of(e) != f) {
                continue;
            }
            // Les commandes du temps encombrent la chronique.
            if let EventKind::OrderApplied { .. } = e.kind {
                if e.origin != Origin::Player && family != Some(Family::Intervention) {
                    continue;
                }
            }
            let d = self.event_dict(e, &lineages, None);
            if !search.is_empty() && !d.get("text").map(|t| t.to_string().to_lowercase()).unwrap_or_default().contains(&search) {
                continue;
            }
            if skipped < offset {
                skipped += 1;
                continue;
            }
            out.push(&d.to_variant());
            if out.len() as i64 >= count {
                break;
            }
        }
        out
    }

    /// Historique échantillonné des grandeurs globales.
    #[func]
    fn history(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(r) = &self.runner else { return d };
        let h = r.shared.history.lock().unwrap();
        let col = |f: &dyn Fn(&evo_sim::Sample) -> f64| PackedFloat64Array::from(&h.iter().map(f).collect::<Vec<f64>>()[..]);
        d.set("years", &col(&|s| s.years));
        d.set("o2", &col(&|s| s.o2_mixing));
        d.set("co2_pa", &col(&|s| s.co2_pa));
        d.set("ch4_ppb", &col(&|s| s.ch4_ppb));
        d.set("temperature_k", &col(&|s| s.mean_temperature_k));
        d.set("ice", &col(&|s| s.ice_fraction));
        d.set("biomass", &col(&|s| s.biomass));
        d.set("lineages", &col(&|s| s.living_lineages as f64));
        d.set("photosynthesis_stage", &col(&|s| s.photosynthesis_stage as f64));
        d
    }

    // ------------------------------------------------------------------
    // Calques, palettes, mise en forme

    #[func]
    fn layers(&self) -> VarArray {
        let mut out = VarArray::new();
        for l in layers::catalogue() {
            let mut d = VarDictionary::new();
            d.set("key", l.key);
            d.set("name", if self.lang == Lang::Fr { l.name_fr } else { l.name_en });
            d.set("family", l.family_fr);
            d.set("unit", l.unit);
            d.set("texture", l.texture as i64);
            d.set("channel", l.channel as i64);
            d.set(
                "transform",
                match l.transform {
                    Transform::Linear => 0i64,
                    Transform::Log10 => 1,
                    Transform::Log10Stored => 2,
                    Transform::Category => 3,
                },
            );
            d.set("min", l.min as f64);
            d.set("max", l.max as f64);
            d.set(
                "palette",
                match l.palette {
                    PaletteKind::Sequential => 0i64,
                    PaletteKind::Diverging => 1,
                    PaletteKind::Categorical => 2,
                },
            );
            let mut pos = PackedFloat32Array::new();
            let mut labels = PackedStringArray::new();
            for (p, s) in l.ticks(5) {
                pos.push(p);
                labels.push(&s);
            }
            d.set("tick_positions", &pos);
            d.set("tick_labels", &labels);
            out.push(&d.to_variant());
        }
        out
    }

    /// Palette en texture 256 × 1 : `kind` 0 séquentielle, 1 divergente,
    /// 2 catégorielle ; `mode` 0 standard, 1 protanopie, 2 deutéranopie,
    /// 3 tritanopie, 4 contraste élevé.
    #[func]
    fn palette_image(&self, kind: i64, mode: i64) -> Option<Gd<Image>> {
        let k = match kind {
            1 => PaletteKind::Diverging,
            2 => PaletteKind::Categorical,
            _ => PaletteKind::Sequential,
        };
        image_rgba8(256, 1, &palette::lut(k, VisionMode::from_index(mode)))
    }

    /// Libellés des guildes colorées sur le calque « guilde dominante ».
    #[func]
    fn guild_legend(&self) -> PackedStringArray {
        let mut out = PackedStringArray::new();
        if let Some(f) = self.frame() {
            for (sig, _) in f.top_guilds(layers::GUILD_CATEGORIES) {
                out.push(&metabolism_list(sig, self.lang).join(" + "));
            }
        }
        out
    }

    #[func]
    fn format_duration(&self, years: f64) -> GString {
        GString::from(&format::duration(years, self.lang))
    }

    #[func]
    fn format_speed(&self, years_per_second: f64) -> GString {
        GString::from(&format::speed(years_per_second, self.lang))
    }

    #[func]
    fn format_power(&self, x: f64) -> GString {
        GString::from(&format::power_of_ten_in(self.lang, x))
    }

    /// Pause au pas donné (scénario de la porte).
    #[func]
    fn pause_at_step(&self, step: i64) {
        if let Some(r) = &self.runner {
            r.send(Command::PauseAtStep(step.max(0) as u64));
        }
    }

    /// Empreinte de l'état à la dernière pause : [pas, empreinte en
    /// hexadécimal], ou tableau vide.
    #[func]
    fn state_hash(&self) -> VarArray {
        let mut out = VarArray::new();
        if let Some((step, h)) = self.runner.as_ref().and_then(|r| *r.shared.hash.lock().unwrap()) {
            out.push(&(step as i64).to_variant());
            out.push(&GString::from(&format!("{h:016x}")).to_variant());
        }
        out
    }

    /// Temps passé dans le pont lors du dernier envoi de textures, ms.
    #[func]
    fn bridge_ms(&self) -> f64 {
        self.bridge_ms
    }
}

impl EvoSession {
    /// Suit la biomasse des espèces dont la fiche a été ouverte (statut).
    fn track_watched(&mut self, f: &Frame) {
        if f.step == self.watch_step || self.watch.is_empty() {
            return;
        }
        self.watch_step = f.step;
        let ids: Vec<u32> = self.watch.keys().copied().collect();
        for id in ids {
            let b: f64 = f.range_of(id).iter().map(|r| r.1 as f64).sum();
            let v = self.watch.get_mut(&id).unwrap();
            v.push(b);
            if v.len() > 30 {
                v.remove(0);
            }
        }
    }
}

/// Métadonnées d'une image de tâche de fond.
#[derive(Clone, Copy, Debug, Default)]
pub struct VarDictionaryLite {
    pub bar_um: f32,
}
