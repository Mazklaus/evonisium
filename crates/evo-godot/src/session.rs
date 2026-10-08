//! `EvoSession` : la partie vue depuis Godot.
//!
//! Le client GDScript ne parle qu'à cette classe. Elle lit l'image figée du
//! dernier pas publié par le moteur (jamais l'état pendant qu'il change),
//! prépare les textures de données du globe, met en forme les textes, et
//! transmet au moteur les deux seules choses que le client écrit : des ordres
//! et la zone d'intérêt.

use crate::jobs::Jobs;
use crate::runner::{Game, Loading};
use evo_core::events::{Event, EventKind, Origin};
use evo_engine::{Answer, Gas, InterestZone, Intervention, OrderKind, PublishedState, Query};
use evo_planet::grid::GeodesicGrid;
use evo_view::chronicle::{self, Action, Family, StopRules};
use evo_view::format::{self, Lang};
use evo_view::frame::{Frame, LineageFrame};
use evo_view::layers::{self, Focus, Transform};
use evo_view::mesh::GlobeMesh;
use evo_view::naming::{guild_common, main_reaction, species_names};
use evo_view::palette::{self, PaletteKind, VisionMode};
use evo_view::pick::CellLocator;
use evo_view::save::PlanetSpec;
use evo_view::species;
use godot::classes::image::Format as ImageFormat;
use godot::classes::Image;
use godot::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
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

fn pigment_colour(rgb: Option<[u8; 3]>) -> Color {
    rgb.map_or(Color::from_rgb(0.78, 0.6, 0.33), |c| Color::from_rgba8(c[0], c[1], c[2], 255))
}

/// Vecteur tangent (repère du moteur) d'une vitesse donnée vers l'est et
/// vers le nord en un point de la sphère.
fn tangent(p: [f64; 3], east: f64, north: f64) -> [f64; 3] {
    let lon = p[1].atan2(p[0]);
    let lat = p[2].clamp(-1.0, 1.0).asin();
    let e = [-lon.sin(), lon.cos(), 0.0];
    let n = [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()];
    [east * e[0] + north * n[0], east * e[1] + north * n[1], east * e[2] + north * n[2]]
}

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct EvoSession {
    base: Base<RefCounted>,
    game: Option<Game>,
    loading: Option<Loading>,
    loading_error: String,
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
}

#[godot_api]
impl IRefCounted for EvoSession {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            game: None,
            loading: None,
            loading_error: String::new(),
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
        }
    }
}

impl EvoSession {
    fn wrap(&self, state: Arc<PublishedState>) -> Option<Arc<Frame>> {
        let g = self.game.as_ref()?;
        Some(Arc::new(Frame::new(state, g.planet.clone())))
    }

    fn frame(&self) -> Option<Arc<Frame>> {
        let state = self.game.as_ref()?.engine.frame().current;
        self.wrap(state)
    }

    fn previous(&self) -> Option<Arc<Frame>> {
        let state = self.game.as_ref()?.engine.frame().previous?;
        self.wrap(state)
    }

    fn lineages(&self) -> Arc<Vec<LineageFrame>> {
        self.game.as_ref().map(|g| g.lineages.clone()).unwrap_or_default()
    }

    fn lineage(lineages: &[LineageFrame], id: u32) -> Option<&LineageFrame> {
        match lineages.get(id as usize).filter(|l| l.id == id) {
            Some(l) => Some(l),
            None => lineages.iter().find(|l| l.id == id),
        }
    }

    fn set_grid(&mut self, level: u32) {
        if self.grid.as_ref().map(|g| g.level) != Some(level) {
            let g = GeodesicGrid::new(level);
            self.locator = Some(CellLocator::new(&g));
            self.grid = Some(Arc::new(g));
        }
    }

    fn order(&self, kind: OrderKind) {
        if let Some(g) = &self.game {
            g.order(kind);
        }
    }

    fn name_of(&self, lineages: &[LineageFrame], id: u32) -> String {
        let (Some(grid), Some(g)) = (&self.grid, &self.game) else { return format!("n° {id}") };
        match Self::lineage(lineages, id) {
            Some(l) => species_names(g.seed, grid, id, l.signature, l.origin_cell, self.lang).common,
            None => format!("n° {id}"),
        }
    }

    /// Nom courant d'une espèce (guilde) : celui du moteur en français.
    fn species_name(&self, f: &Frame, species: u32) -> String {
        match (self.lang, f.species(species)) {
            (Lang::Fr, Some(s)) => s.name.clone(),
            _ => {
                let mut c = guild_common(main_reaction(species), self.lang).to_string();
                if let Some(first) = c.get(0..1) {
                    c = first.to_uppercase() + &c[1..];
                }
                c
            }
        }
    }

    fn event_dict(&self, e: &Event, lineages: &[LineageFrame], rules: Option<&StopRules>, pace: f64) -> VarDictionary {
        let name = |id: u32| self.name_of(lineages, id);
        let level = chronicle::level(e, pace);
        let action = rules.map_or(Action::Note, |r| r.decide(e, pace));
        let lineage = match &e.kind {
            EventKind::LifeSeeded { lineage }
            | EventKind::NewLineage { lineage, .. }
            | EventKind::LineageExtinct { lineage }
            | EventKind::Innovation { lineage, .. } => Some(*lineage),
            _ => None,
        };
        let species = lineage.and_then(|l| Self::lineage(lineages, l)).map_or(-1, |l| l.signature as i64);
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
        d.set("lineage", lineage.map_or(-1, |l| l as i64));
        d.set("species", species);
        d.set("player", e.origin == Origin::Player);
        d.set("refused", matches!(e.kind, EventKind::OrderRefused { .. }));
        d.set("accelerator", e.origin == Origin::Accelerator);
        d.set("interest", e.interest);
        d.set("cause", e.cause.map_or(-1, |c| c as i64));
        d
    }

    fn temperature(&self, k: f64) -> String {
        format::temperature(k, self.celsius, self.lang)
    }

    fn reset_view(&mut self) {
        self.textures_step = None;
        self.events_polled = 0;
        self.watch.clear();
        self.focus = Focus::default();
        self.jobs = Jobs::default();
    }

    fn intervention(kind: &str, amount: f64, cell: u32, radius_km: f64) -> Option<Intervention> {
        Some(match kind {
            "phosphate" => Intervention::Fertilize { cell, radius_km, moles_p: amount },
            "eruption" => Intervention::Eruption { cell, gas: Gas::Co2, moles: amount },
            "methane" => Intervention::Eruption { cell, gas: Gas::Ch4, moles: amount },
            "hydrogene" => Intervention::Eruption { cell, gas: Gas::H2, moles: amount },
            _ => return None,
        })
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
        self.game = None;
        self.loading = None;
        self.set_grid(level);
        self.reset_view();
        match Game::start(spec, seed as u64, level, "sources", false) {
            Ok(g) => {
                self.game = Some(g);
                true
            }
            Err(e) => {
                godot_error!("partie impossible : {e}");
                false
            }
        }
    }

    /// Reprend un point de sauvegarde (en tâche de fond) ; renvoie un
    /// message d'erreur, ou une chaîne vide.
    #[func]
    fn load_save(&mut self, path: GString) -> GString {
        let path = PathBuf::from(path.to_string());
        if let Err(e) = crate::runner::read_meta(&path) {
            return GString::from(&e);
        }
        self.game = None;
        self.reset_view();
        self.loading_error.clear();
        self.loading = Some(Loading::start(path));
        GString::new()
    }

    /// Nom, date et planète d'un point de sauvegarde, sans le charger.
    #[func]
    fn describe_save(&self, path: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Ok(save) = crate::runner::read_meta(&PathBuf::from(path.to_string())) {
            d.set("name", save.name.as_str());
            d.set("date", format::game_date(save.years, self.lang));
            d.set("planet", save.spec.to_params().name.as_str());
            d.set("code", save.spec.code(save.seed, save.level).as_str());
            d.set("steps", save.steps as i64);
        }
        d
    }

    #[func]
    fn stop(&mut self) {
        self.game = None;
        self.loading = None;
    }

    #[func]
    fn is_running(&self) -> bool {
        self.game.is_some()
    }

    /// Reprise en cours : (0, 1) tant que l'état se lit, (−1, −1) sinon.
    /// La partie prend la main dès qu'elle est prête.
    #[func]
    fn loading_progress(&mut self) -> Vector2 {
        let Some(l) = &mut self.loading else { return Vector2::new(-1.0, -1.0) };
        match l.take() {
            None => Vector2::new(0.0, 1.0),
            Some(Ok(g)) => {
                self.loading = None;
                self.set_grid(g.level);
                self.game = Some(g);
                Vector2::new(-1.0, -1.0)
            }
            Some(Err(e)) => {
                self.loading = None;
                self.loading_error = e;
                Vector2::new(-1.0, -1.0)
            }
        }
    }

    #[func]
    fn loading_error(&self) -> GString {
        GString::from(&self.loading_error)
    }

    #[func]
    fn save(&mut self, path: GString, name: GString) {
        if let Some(g) = &mut self.game {
            g.save(&path.to_string(), &name.to_string());
        }
    }

    /// Message du moteur (sauvegarde écrite…), ou chaîne vide.
    #[func]
    fn take_notice(&mut self) -> GString {
        self.game.as_mut().and_then(|g| g.notices.pop_front()).map_or(GString::new(), |s| GString::from(&s))
    }

    #[func]
    fn planet_code(&self) -> GString {
        match &self.game {
            Some(g) => GString::from(&g.spec.code(g.seed, g.level)),
            None => GString::new(),
        }
    }

    // ------------------------------------------------------------------
    // File d'ordres et canal d'observation

    /// « sources » (près des sources hydrothermales) ou « mers » (toutes
    /// les mers). Le lieu du dépôt est un paramètre de la partie : tant que
    /// la vie n'est pas déposée, la partie est recréée avec ce choix.
    #[func]
    fn set_seeding(&mut self, kind: GString) {
        let kind = if kind == "mers" { "mers" } else { "sources" };
        let Some(g) = &self.game else { return };
        if g.seeding == kind || !g.lineages.is_empty() || g.engine.frame().current.globals.biomass > 0.0 {
            return;
        }
        let (spec, seed, level, sandbox, pace) = (g.spec.clone(), g.seed, g.level, g.sandbox, g.pace);
        self.game = None;
        match Game::start(spec, seed, level, kind, sandbox) {
            Ok(mut g) => {
                g.set_speed(pace);
                self.game = Some(g);
            }
            Err(e) => {
                godot_error!("partie impossible : {e}");
            }
        }
        self.textures_step = None;
    }

    #[func]
    fn seed_life(&self) {
        self.order(OrderKind::SeedLife);
    }

    /// Vitesse en années de jeu par seconde.
    #[func]
    fn set_speed(&mut self, years_per_second: f64) {
        if let Some(g) = &mut self.game {
            g.set_speed(years_per_second);
        }
    }

    #[func]
    fn speed(&self) -> f64 {
        self.game.as_ref().map_or(crate::runner::DEFAULT_SPEED, |g| g.pace)
    }

    #[func]
    fn pause(&self) {
        self.order(OrderKind::Pause);
    }

    #[func]
    fn resume(&mut self) {
        if let Some(g) = &mut self.game {
            g.auto_paused_by = None;
        }
        self.order(OrderKind::Resume);
    }

    /// Coût en influence d'une intervention.
    #[func]
    fn intervention_cost(&self, kind: GString, amount: f64) -> f64 {
        Self::intervention(&kind.to_string(), amount, 0, 1000.0).map_or(0.0, |i| i.cost())
    }

    /// Intervention sur l'environnement autour d'une cellule : « phosphate »
    /// (mol de P, dans `radius_km`), « eruption » (mol de CO₂), « methane »
    /// (mol de CH₄), « hydrogene » (mol d'H₂). Le moteur la refuse si la
    /// réserve d'influence ne suffit pas ; le client vérifie d'abord.
    #[func]
    fn intervene(&self, kind: GString, amount: f64, cell: i64, radius_km: f64) -> bool {
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return false };
        if cell < 0 || cell as usize >= f.cells().len() {
            return false;
        }
        let Some(i) = Self::intervention(&kind.to_string(), amount, cell as u32, radius_km.max(1.0)) else { return false };
        let inf = &f.state.influence;
        if !inf.sandbox && (inf.points as f64) < i.cost() {
            return false;
        }
        g.order(OrderKind::Intervene(i));
        true
    }

    /// Réserve d'influence.
    #[func]
    fn influence(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else {
            d.set("available", false);
            return d;
        };
        let inf = &f.state.influence;
        d.set("available", true);
        d.set("points", inf.points as f64);
        d.set("max", inf.max as f64);
        d.set("recharge_per_myr", inf.recharge_per_myr as f64);
        d.set("sandbox", inf.sandbox);
        d
    }

    #[func]
    fn mark_lineage(&mut self, lineage: i64) {
        if lineage >= 0 {
            self.order(OrderKind::MarkLineage { lineage: lineage as u32 });
            if !self.focus.marked.contains(&(lineage as u32)) {
                self.focus.marked.push(lineage as u32);
            }
        }
    }

    /// Zone d'intérêt de la caméra : sans effet sur l'histoire.
    #[func]
    fn observe(&self, centre: Vector3, radius_rad: f64, band: i64) {
        let (Some(g), Some(grid), Some(loc)) = (&self.game, &self.grid, &self.locator) else { return };
        let cell = loc.locate(grid, from_godot(centre)) as u32;
        g.observe(InterestZone {
            center_cell: cell,
            radius_km: radius_rad * g.planet.radius_m / 1000.0,
            zoom_band: band.clamp(1, 6) as u8,
        });
    }

    // ------------------------------------------------------------------
    // Règles d'arrêt, langue, unités

    #[func]
    fn set_rules_profile(&mut self, name: GString) {
        if let Some(g) = &mut self.game {
            g.rules = StopRules::profile(&name.to_string());
        }
    }

    #[func]
    fn set_rule(&mut self, family: GString, action: i64) {
        if let (Some(g), Some(f)) = (&mut self.game, Family::from_key(&family.to_string())) {
            g.rules.set(f, Action::from_index(action));
        }
    }

    /// Règles en cours : famille → [libellé, action].
    #[func]
    fn rules(&self) -> VarArray {
        let mut out = VarArray::new();
        let rules = self.game.as_ref().map_or_else(|| StopRules::profile("naturaliste"), |g| g.rules.clone());
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
    fn take_auto_pause(&mut self) -> i64 {
        let Some(g) = &mut self.game else { return -1 };
        // La pause demandée n'est pas encore appliquée : on attend.
        if g.auto_paused_by.is_some() && !g.engine.status().paused {
            return -1;
        }
        g.auto_paused_by.take().map_or(-1, |id| id as i64)
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
    // État publié

    #[func]
    fn has_frame(&self) -> bool {
        self.game.is_some()
    }

    /// Vrai si un pas nouveau a été publié depuis les dernières textures.
    #[func]
    fn has_new_frame(&self) -> bool {
        match self.frame() {
            Some(f) => self.textures_step != Some(f.state.step),
            None => false,
        }
    }

    #[func]
    fn frame_info(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return d };
        let st = &f.state;
        let gl = &st.globals;
        d.set("step", st.step as i64);
        d.set("years", st.years);
        d.set("date", format::game_date(st.years, self.lang));
        d.set("step_years", st.step_years);
        d.set("paused", st.paused);
        d.set("climate", st.climate_mode.label());
        d.set("o2", gl.o2_mixing);
        d.set("o2_text", format::power_of_ten_in(self.lang, gl.o2_mixing));
        d.set("co2_pa", gl.co2_pa);
        d.set("ch4_ppb", gl.ch4_ppb);
        d.set("pressure_pa", gl.pressure_pa);
        d.set("temperature_k", gl.mean_temperature_k);
        d.set("temperature_text", self.temperature(gl.mean_temperature_k));
        d.set("ice", gl.ice_fraction);
        d.set("ocean", gl.ocean_fraction);
        d.set("biomass", gl.biomass);
        d.set("biomass_text", format::power_of_ten_in(self.lang, gl.biomass));
        d.set("lineages", gl.living_lineages as i64);
        d.set("guilds", gl.guilds as i64);
        d.set("species", st.species.len() as i64);
        d.set("photosynthesis_stage", gl.photosynthesis_stage as i64);
        d.set("accelerator", gl.accelerator_on);
        d.set("influence", st.influence.points as f64);
        d.set("influence_max", st.influence.max as f64);
        d.set("sandbox", st.influence.sandbox);
        d.set("planet", f.planet.name.as_str());
        d.set("radius_m", f.planet.radius_m);
        d.set("star_k", f.planet.star_temperature_k);
        d.set("star_colour", colour(palette::blackbody(f.planet.star_temperature_k)));
        d.set("cells", st.cells.len() as i64);
        d.set("level", f.planet.level as i64);
        d.set("seed", f.planet.seed as i64);
        let status = g.engine.status();
        let real = if status.paused { 0.0 } else { status.years_per_second };
        d.set("real_speed", real);
        d.set("real_speed_text", if real > 0.0 { format::speed(real, self.lang) } else { "—".into() });
        d.set("step_seconds", status.last_step_seconds);
        d.set("engine_error", status.error.unwrap_or_default().as_str());
        d.set("bridge_ms", self.bridge_ms);
        d
    }

    /// Couleur du limbe de l'atmosphère : lumière de l'étoile, brume de
    /// méthane orangée, bleu de diffusion d'un air oxygéné.
    #[func]
    fn atmosphere_colour(&self) -> Color {
        let Some(f) = self.frame() else { return Color::from_rgb(0.8, 0.8, 0.8) };
        let star = palette::blackbody(f.planet.star_temperature_k);
        let haze = ((f.state.globals.ch4_ppb / 5000.0).sqrt() as f32).clamp(0.0, 1.0);
        let blue = ((f.state.globals.o2_mixing / 0.02).sqrt() as f32).clamp(0.0, 1.0);
        let mut c = [star[0] * 0.8, star[1] * 0.78, star[2] * 0.75];
        let orange = [0.85, 0.55, 0.3];
        let azure = [0.45, 0.62, 0.85];
        for k in 0..3 {
            c[k] += (orange[k] - c[k]) * haze * 0.7;
            c[k] += (azure[k] - c[k]) * blue * 0.8;
        }
        colour(c)
    }

    /// Les textures de données du dernier pas (et marque ce pas comme lu).
    /// `previous` donne celles du pas d'avant, pour l'interpolation.
    #[func]
    fn frame_textures(&mut self, previous: bool) -> VarArray {
        let t0 = Instant::now();
        let mut out = VarArray::new();
        let frame = if previous { self.previous() } else { self.frame() };
        let Some(f) = frame else { return out };
        let packed = layers::pack(&f, &self.focus);
        let (w, h) = layers::texture_size(f.cells().len());
        for t in packed.iter() {
            match image_rgbaf(w, h, t) {
                Some(img) => out.push(&img.to_variant()),
                None => out.push(&Variant::nil()),
            }
        }
        if !previous {
            self.textures_step = Some(f.state.step);
            self.track_watched(&f);
        }
        self.bridge_ms = t0.elapsed().as_secs_f64() * 1000.0;
        out
    }

    /// Espèce dont l'aire est surlignée (−1 : aucune).
    #[func]
    fn set_focus(&mut self, species: i64) {
        let s = (species >= 0).then_some(species as u32);
        if self.focus.species != s {
            self.focus.species = s;
            self.textures_step = None;
        }
    }

    #[func]
    fn focus(&self) -> i64 {
        self.focus.species.map_or(-1, |l| l as i64)
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
            Some(g) if cell >= 0 && (cell as usize) < g.len() => to_godot(g.centers[cell as usize]),
            _ => Vector3::ZERO,
        }
    }

    /// Flèches des plaques : (position, vitesse en cm/an) sur un
    /// échantillon de cellules (grille du niveau 3).
    #[func]
    fn plate_arrows(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(g)) = (self.frame(), &self.grid) else { return d };
        let n = GeodesicGrid::cell_count_for_level(3).min(g.len()).min(f.cells().len());
        let mut pos = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);
        for c in 0..n {
            let p = g.centers[c];
            let v = f.cells()[c].plate_velocity_cm_yr;
            pos.push(to_godot(p));
            vel.push(to_godot(tangent(p, v[0] as f64, v[1] as f64)));
        }
        d.set("positions", &PackedVector3Array::from(&pos[..]));
        d.set("velocities", &PackedVector3Array::from(&vel[..]));
        d
    }

    /// Inspecteur de cellule : milieu en clair et populations. Le détail
    /// (chimie, sources, populations) arrive du moteur un peu après : tant
    /// qu'il manque, `pending` vaut vrai.
    #[func]
    fn cell_info(&mut self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return d };
        if cell < 0 || cell as usize >= f.cells().len() {
            return d;
        }
        let c = cell as usize;
        let e = &f.cells()[c];
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
        d.set("region", evo_view::naming::place_name(f.planet.seed, evo_view::naming::region_of(&grid, c)));
        d.set("is_ocean", e.is_ocean);
        let lake = !e.is_ocean && e.lake_fraction > 0.3;
        d.set(
            "medium",
            match (fr, e.is_ocean, lake, e.ice_cover > 0.5) {
                (true, true, _, false) => "mer",
                (true, true, _, true) => "mer gelée",
                (true, false, true, false) => "lacs et marais",
                (true, false, _, true) => "glacier",
                (true, false, false, false) => "terre émergée",
                (false, true, _, false) => "sea",
                (false, true, _, true) => "frozen sea",
                (false, false, true, false) => "lakes and wetlands",
                (false, false, _, true) => "ice sheet",
                (false, false, false, false) => "land",
            },
        );
        let height = e.elevation_m as f64;
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
        d.set("rain", format!("{} mm·{}", format::number_in(self.lang, e.rain_mm_yr as f64, 0), if fr { "an⁻¹" } else { "yr⁻¹" }));
        d.set("biomass", format!("{} mol C", format::power_of_ten_in(self.lang, e.biomass as f64)));
        d.set("plate", e.plate as i64);
        // Détail du moteur.
        let step = f.state.step;
        let lineages = self.lineages();
        let Some(g) = &mut self.game else { return d };
        g.request_cell(cell as u32, step);
        let detail = g.cell.as_ref().filter(|(x, _)| x.cell == cell as u32).map(|(x, _)| x.clone());
        d.set("pending", detail.is_none());
        let mut pops = VarArray::new();
        if let Some(x) = detail {
            d.set("ph", format::number_in(self.lang, x.ph, 2));
            d.set("salinity", format!("{} g·kg⁻¹", format::number_in(self.lang, x.salinity, 1)));
            d.set("uv", format!("{} W·m⁻²", format::number_in(self.lang, x.uv_w_m2, 1)));
            let vent = x.vent_h2 + x.vent_h2s + x.vent_fe > 0.0;
            d.set("vent", vent);
            if vent {
                d.set(
                    "vent_text",
                    format!(
                        "H₂ {} · H₂S {} mol·{}",
                        format::power_of_ten_in(self.lang, x.vent_h2),
                        format::power_of_ten_in(self.lang, x.vent_h2s),
                        if fr { "an⁻¹" } else { "yr⁻¹" }
                    ),
                );
            }
            let mut chem = VarArray::new();
            for (name, v) in &x.chemistry {
                let mut cd = VarDictionary::new();
                cd.set("name", *name);
                cd.set("value", format!("{} mol·m⁻³", format::power_of_ten_in(self.lang, *v)));
                chem.push(&cd.to_variant());
            }
            d.set("chemistry", &chem);
            // Une lignée peut compter plusieurs écotypes dans la cellule : une
            // ligne par lignée, de la plus abondante à la plus rare.
            let mut by_lineage: Vec<evo_sim::observation::PopulationView> = Vec::new();
            for p in &x.populations {
                match by_lineage.iter_mut().find(|q| q.lineage == p.lineage) {
                    Some(q) => q.biomass += p.biomass,
                    None => by_lineage.push(p.clone()),
                }
            }
            by_lineage.sort_by(|a, b| b.biomass.total_cmp(&a.biomass).then(a.lineage.cmp(&b.lineage)));
            let total: f32 = by_lineage.iter().map(|p| p.biomass).sum();
            for p in &by_lineage {
                let mut pd = VarDictionary::new();
                pd.set("lineage", p.lineage as i64);
                pd.set("species", p.species as i64);
                pd.set("name", self.name_of(&lineages, p.lineage).as_str());
                pd.set("guild", guild_common(main_reaction(p.species), self.lang));
                pd.set("share", if total > 0.0 { p.biomass / total } else { 0.0 });
                pd.set("biomass", format::power_of_ten_in(self.lang, p.biomass as f64).as_str());
                pd.set("colour", pigment_colour(p.pigment_rgb.or_else(|| p.pigment_nm.map(|nm| evo_life::pigment_colour(nm as f64)))));
                pops.push(&pd.to_variant());
            }
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

    /// Espèce d'une lignée (−1 si inconnue).
    #[func]
    fn species_of_lineage(&self, lineage: i64) -> i64 {
        Self::lineage(&self.lineages(), lineage.max(0) as u32).map_or(-1, |l| l.signature as i64)
    }

    /// Fiche d'espèce simple.
    #[func]
    fn species_info(&mut self, species: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return d };
        let Some(sv) = f.species(species.max(0) as u32) else { return d };
        let lineages = self.lineages();
        let s = species::summary(&f, &grid, sv, &lineages, self.lang);
        self.watch.entry(s.species).or_insert_with(|| vec![s.biomass]);
        let status = species::Status::from_series(self.watch.get(&s.species).map_or(&[][..], |v| &v[..]), s.range_cells == 0);
        let fr = self.lang == Lang::Fr;
        d.set("species", species);
        d.set("scientific", s.scientific.as_str());
        d.set("common", self.species_name(&f, s.species).as_str());
        let mut mets = PackedStringArray::new();
        for m in &s.metabolisms {
            mets.push(*m);
        }
        d.set("metabolisms", &mets);
        d.set(
            "pigment",
            match s.pigment_rgb {
                Some(_) => (if fr { "pigmentée" } else { "pigmented" }).to_string(),
                None => (if fr { "sans pigment" } else { "no pigment" }).to_string(),
            },
        );
        d.set("colour", pigment_colour(s.pigment_rgb));
        d.set("range_cells", s.range_cells as i64);
        d.set("range_share", format::percent(s.range_share as f64, self.lang));
        d.set("biomass", format!("{} mol C", format::power_of_ten_in(self.lang, s.biomass)));
        d.set("ecotypes", s.ecotypes as i64);
        d.set("lineages", s.lineages as i64);
        d.set("born", s.born_years.map_or("—".into(), |y| format::duration(y, self.lang)));
        d.set("age", s.born_years.map_or("—".into(), |y| format::duration(f.state.years - y, self.lang)));
        let parent_species = s.parent_lineage.and_then(|p| Self::lineage(&lineages, p)).map(|l| l.signature).filter(|&p| p != s.species);
        d.set("parent", parent_species.map_or(-1, |p| p as i64));
        d.set("parent_name", parent_species.map_or(String::new(), |p| self.species_name(&f, p)));
        d.set("status", status.label(self.lang));
        d.set("extinct", false);
        // Lignée suivie quand le joueur suit l'espèce : sa plus ancienne
        // lignée vivante.
        let founder = lineages
            .iter()
            .filter(|l| l.signature == s.species && l.extinct_years.is_none())
            .min_by(|a, b| a.born_years.total_cmp(&b.born_years).then(a.id.cmp(&b.id)));
        d.set("founder", founder.map_or(-1, |l| l.id as i64));
        d.set("marked", founder.is_some_and(|l| self.focus.marked.contains(&l.id)));
        d.set("peak_cell", s.peak_cell as i64);
        d.set("origin_region", evo_view::naming::place_name(f.planet.seed, evo_view::naming::region_of(&grid, s.origin_cell as usize)));
        d
    }

    /// Arbre du vivant élagué : tableaux parallèles, un noeud par indice.
    #[func]
    fn tree(&self, max_leaves: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        let lineages = self.lineages();
        let t = evo_view::tree::build(&lineages, f.state.years, max_leaves.max(2) as usize);
        let mut ids = PackedInt32Array::new();
        let mut species = PackedInt64Array::new();
        let mut parents = PackedInt32Array::new();
        let mut born = PackedFloat64Array::new();
        let mut end = PackedFloat64Array::new();
        let mut ys = PackedFloat32Array::new();
        let mut living = PackedByteArray::new();
        let mut collapsed = PackedInt32Array::new();
        let mut names = PackedStringArray::new();
        for n in &t.nodes {
            ids.push(n.lineage as i32);
            species.push(n.signature as i64);
            parents.push(n.parent as i32);
            born.push(n.born_years);
            end.push(n.end_years);
            ys.push(n.y);
            living.push(n.living as u8);
            collapsed.push(n.collapsed as i32);
            names.push(&self.name_of(&lineages, n.lineage));
        }
        d.set("ids", &ids);
        d.set("species", &species);
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

    /// Lance le calcul du décor de milieu d'une espèce (tâche de fond : le
    /// milieu type vient du moteur).
    #[func]
    fn request_decor(&mut self, species: i64, width: i64, height: i64) -> GString {
        let key = format!("decor-{species}");
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return GString::from(&key) };
        let rx = g.engine.query(Query::SpeciesHabitat { species: species.max(0) as u32 });
        let (w, hh) = (width.clamp(64, 2048) as usize, height.clamp(32, 1024) as usize);
        let game_seed = f.planet.seed;
        self.jobs.spawn(key.clone(), species as u64 ^ (f.state.step / 50) << 32, move || {
            let Ok(Answer::Habitat(Some(eh))) = rx.recv() else { return None };
            let h = evo_view::decor::Habitat::from_engine(&eh, &f);
            let seed = h.decor_seed(game_seed);
            Some((evo_view::decor::paint(&h, seed, w, hh), VarDictionaryLite::default()))
        });
        GString::from(&key)
    }

    /// Lance la vue microscope d'une cellule.
    #[func]
    fn request_microscope(&mut self, cell: i64, width: i64, height: i64) -> GString {
        let key = format!("microscope-{cell}");
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return GString::from(&key) };
        if cell < 0 || cell as usize >= f.cells().len() {
            return GString::from(&key);
        }
        let rx = g.engine.query(Query::Cell { cell: cell as u32 });
        let game_seed = f.planet.seed;
        let seed = game_seed ^ ((cell as u64) << 24) ^ (f.state.step / 50);
        let (w, h) = (width.clamp(64, 2048) as usize, height.clamp(64, 2048) as usize);
        self.jobs.spawn(key.clone(), seed, move || {
            let Ok(Answer::Cell(Some(detail))) = rx.recv() else { return None };
            let members = species::microscope_members(&detail.populations, game_seed);
            let (c, _) = evo_morph::microscope_field(&members, seed, w, h);
            Some((c, VarDictionaryLite { bar_um: evo_morph::microbe::MICROSCOPE_BAR_UM }))
        });
        GString::from(&key)
    }

    /// Lance la figure d'une espèce (planche) : la population la plus
    /// abondante de l'espèce dans sa cellule d'apogée.
    #[func]
    fn request_figure(&mut self, species: i64, width: i64, height: i64) -> GString {
        let key = format!("figure-{species}");
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return GString::from(&key) };
        let Some(sv) = f.species(species.max(0) as u32) else { return GString::from(&key) };
        let rx = g.engine.query(Query::Cell { cell: sv.peak_bio_cell });
        let (signature, game_seed) = (sv.signature, f.planet.seed);
        let (w, h) = (width.clamp(64, 2048) as usize, height.clamp(64, 2048) as usize);
        self.jobs.spawn(key.clone(), species as u64 ^ (f.state.step / 50) << 32, move || {
            let Ok(Answer::Cell(Some(detail))) = rx.recv() else { return None };
            let p = detail.populations.iter().filter(|p| p.species == signature).max_by(|a, b| a.biomass.total_cmp(&b.biomass))?;
            let form = evo_morph::form(&species::traits_of(p), game_seed);
            let (c, bar) = evo_morph::figure(&form, w, h);
            Some((c, VarDictionaryLite { bar_um: bar }))
        });
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

    /// Relève les réponses du moteur et renvoie les événements nouveaux
    /// depuis le dernier appel (une fois par image).
    #[func]
    fn poll_events(&mut self) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = &mut self.game else { return out };
        g.pump();
        let g = self.game.as_ref().unwrap();
        let events = &g.events;
        if self.events_polled >= events.len() {
            return out;
        }
        let lineages = self.lineages();
        // Au plus 200 par appel : le reste reste dans la chronique.
        let start = self.events_polled.max(events.len().saturating_sub(200));
        for e in &events[start..] {
            out.push(&self.event_dict(e, &lineages, Some(&g.rules), g.pace).to_variant());
        }
        self.events_polled = events.len();
        out
    }

    #[func]
    fn events_count(&self) -> i64 {
        self.game.as_ref().map_or(0, |g| g.events.len() as i64)
    }

    /// Page de la chronique, du plus récent au plus ancien, filtrée par
    /// famille (vide : toutes) et par texte.
    #[func]
    fn events_page(&self, offset: i64, count: i64, family: GString, search: GString) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = &self.game else { return out };
        let lineages = self.lineages();
        let family = Family::from_key(&family.to_string());
        let search = search.to_string().to_lowercase();
        let mut skipped = 0;
        for e in g.events.iter().rev() {
            if family.is_some_and(|f| Family::of(e) != f) {
                continue;
            }
            // Les commandes du temps encombrent la chronique.
            if let EventKind::OrderApplied { .. } = e.kind {
                if e.origin != Origin::Player && family != Some(Family::Intervention) {
                    continue;
                }
            }
            let d = self.event_dict(e, &lineages, None, g.pace);
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
        let Some(g) = &self.game else { return d };
        let h = &g.history;
        let col = |f: &dyn Fn(&evo_engine::Sample) -> f64| PackedFloat64Array::from(&h.iter().map(f).collect::<Vec<f64>>()[..]);
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

    /// Noms des espèces colorées sur le calque « guilde dominante ».
    #[func]
    fn guild_legend(&self) -> PackedStringArray {
        let mut out = PackedStringArray::new();
        if let Some(f) = self.frame() {
            for (id, _) in f.top_guilds(layers::GUILD_CATEGORIES) {
                out.push(&self.species_name(&f, id));
            }
        }
        out
    }

    /// Espèces de la légende, dans le même ordre (pour ouvrir leur fiche).
    #[func]
    fn guild_ids(&self) -> PackedInt64Array {
        let mut out = PackedInt64Array::new();
        if let Some(f) = self.frame() {
            for (id, _) in f.top_guilds(layers::GUILD_CATEGORIES) {
                out.push(id as i64);
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

    /// Pause à une date de jeu, par un ordre (scénario de la porte : deux
    /// parties s'arrêtent au même pas, quelle que soit la caméra).
    #[func]
    fn pause_at(&self, years: f64) {
        if let Some(g) = &self.game {
            g.order_at(years, OrderKind::Pause);
        }
    }

    /// Empreinte de l'état quand la partie est en pause : [pas, empreinte en
    /// hexadécimal], ou tableau vide.
    #[func]
    fn state_hash(&self) -> VarArray {
        let mut out = VarArray::new();
        if let Some(g) = &self.game {
            let st = g.engine.status();
            // Une partie reprise n'a pas encore d'empreinte tant qu'aucun pas
            // n'a tourné : on la demande (en pause, le moteur répond vite).
            let hash = st.state_hash.or_else(|| {
                let a = st.paused.then(|| g.engine.query(Query::StateHash).recv_timeout(std::time::Duration::from_secs(5)).ok());
                match a.flatten() {
                    Some(Answer::StateHash(h)) => Some(h),
                    _ => None,
                }
            });
            if let (true, Some(h)) = (st.paused, hash) {
                out.push(&(st.steps as i64).to_variant());
                out.push(&GString::from(&format!("{h:016x}")).to_variant());
            }
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
        if f.state.step == self.watch_step || self.watch.is_empty() {
            return;
        }
        self.watch_step = f.state.step;
        for (id, v) in self.watch.iter_mut() {
            v.push(f.species(*id).map_or(0.0, |s| s.biomass));
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
