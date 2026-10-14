//! Sociétés et histoire vues depuis Godot (étape 7) : territoires, villes,
//! routes, champs et lumières nocturnes sur le globe, liste et fiches des
//! sociétés, comparaison de civilisations, chronique et récit, encyclopédie.
//!
//! Le moteur ne fait pas encore vivre de sociétés. En attendant, le client
//! pose des sociétés d'essai sur les terres de la partie
//! (`evo_view::society::bench_world`), autour des peuples d'essai de
//! l'étape 6, et déroule leur histoire année par année dans une tâche de
//! fond ; rien n'en remonte vers le monde.

use super::*;
use evo_society::state::SocietyState;
use evo_view::encyclopedia::SIMPLIFICATIONS;
use evo_view::society::{self as sv, BenchSpec, History};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Années par seconde au plus : l'histoire d'essai se laisse lire.
const MAX_YEARS_PER_SECOND: f64 = 250.0;
/// Un relevé pour la comparaison tous les tant d'années.
const SAMPLE_YEARS: i64 = 20;

/// Ce que la tâche de fond partage avec l'affichage.
#[derive(Default)]
struct Shared {
    /// 1 : en préparation ; 2 : l'histoire avance ; 3 : finie ; −1 : pas
    /// assez de terres habitables.
    status: i64,
    state: Option<Arc<SocietyState>>,
    history: History,
    /// Compteur des publications, pour que l'affichage sache quand relire.
    serial: i64,
}

#[derive(Default)]
pub(crate) struct SocietySlot {
    shared: Option<Arc<Mutex<Shared>>>,
    stop: Arc<AtomicBool>,
    /// Partie pour laquelle l'histoire tourne.
    key: String,
}

impl Drop for SocietySlot {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn run(
    grid: Arc<GeodesicGrid>,
    frame: Arc<Frame>,
    culture: Option<Arc<evo_culture::CulturalState>>,
    spec: BenchSpec,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
) {
    let Some(mut b) = sv::bench_world(&grid, frame.cells(), culture.as_deref(), spec) else {
        shared.lock().unwrap().status = -1;
        return;
    };
    let mut seen = b.world.events_total;
    let started = Instant::now();
    let first_year = b.world.year;
    let publish = |b: &sv::Bench, events: Vec<evo_society::chronicle::Event>, sample: bool, done: bool| {
        let st = Arc::new(b.state());
        let mut s = shared.lock().unwrap();
        if sample {
            s.history.record(&st, &events);
        } else {
            s.history.events.extend(events);
        }
        s.state = Some(st);
        s.status = if done { 3 } else { 2 };
        s.serial += 1;
    };
    publish(&b, Vec::new(), true, false);
    let mut pending = Vec::new();
    let mut last_publish = Instant::now();
    while !b.done() && !stop.load(Ordering::Relaxed) {
        b.step();
        // Les événements de l'année, cellules rendues à celles du globe.
        let new = (b.world.events_total - seen) as usize;
        seen = b.world.events_total;
        let n = b.world.events.len();
        for e in &b.world.events[n.saturating_sub(new)..] {
            let mut e = e.clone();
            e.cell = e.cell.and_then(|c| b.globe_cell.get(c as usize).copied());
            pending.push(e);
        }
        let sample = b.world.year % SAMPLE_YEARS == 0;
        if sample || last_publish.elapsed().as_secs_f64() > 0.5 {
            publish(&b, std::mem::take(&mut pending), sample, false);
            last_publish = Instant::now();
        }
        // Frein : au plus MAX_YEARS_PER_SECOND.
        let ahead = (b.world.year - first_year) as f64 / MAX_YEARS_PER_SECOND - started.elapsed().as_secs_f64();
        if ahead > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(ahead.min(0.1)));
        }
    }
    publish(&b, pending, true, b.done());
}

impl EvoSession {
    fn society_key(&self) -> String {
        self.game.as_ref().map(|g| format!("{}-{}", g.planet.name, g.planet.seed)).unwrap_or_default()
    }

    fn society_shared(&self) -> Option<Arc<Mutex<Shared>>> {
        (self.society.key == self.society_key()).then(|| self.society.shared.clone()).flatten()
    }

    pub(super) fn society_state(&self) -> Option<Arc<SocietyState>> {
        self.society_shared()?.lock().unwrap().state.clone()
    }

    fn cell_area_km2(&self) -> f64 {
        self.grid.as_ref().map_or(1.0, |g| 4.0 * std::f64::consts::PI * 6371.0 * 6371.0 / g.len() as f64)
    }

    fn cell_position(&self, cell: u32) -> Vector3 {
        self.cell_centre(cell as i64)
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Lance l'histoire des sociétés d'essai (après les peuples d'essai,
    /// dont elle reprend le pays).
    #[func]
    fn society_bench(&mut self, seed: i64) {
        let key = self.society_key();
        if self.society.key == key && self.society.shared.is_some() {
            return;
        }
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return };
        let culture = self.culture();
        let shared = Arc::new(Mutex::new(Shared { status: 1, ..Default::default() }));
        let stop = Arc::new(AtomicBool::new(false));
        self.society = SocietySlot { shared: Some(shared.clone()), stop: stop.clone(), key };
        let spec = BenchSpec { seed: seed as u64, ..Default::default() };
        std::thread::spawn(move || run(grid, f, culture, spec, shared, stop));
    }

    /// 0 : pas de sociétés ; 1 : en préparation ; 2 : l'histoire avance ;
    /// 3 : finie ; −1 : pas assez de terres habitables.
    #[func]
    fn society_status(&self) -> i64 {
        self.society_shared().map_or(0, |s| s.lock().unwrap().status)
    }

    /// Compteur des publications (relire quand il change).
    #[func]
    fn society_serial(&self) -> i64 {
        self.society_shared().map_or(0, |s| s.lock().unwrap().serial)
    }

    /// Année de l'histoire d'essai, en clair.
    #[func]
    fn society_year(&self) -> GString {
        self.society_state().map_or(GString::new(), |s| GString::from(&sv::year_text(s.year, self.lang)))
    }

    #[func]
    fn society_year_value(&self) -> i64 {
        self.society_state().map_or(0, |s| s.year)
    }

    /// Texture des sociétés (rang de couleur, log₁₀ de l'effectif, champs).
    #[func]
    fn society_texture(&self) -> Option<Gd<Image>> {
        let st = self.society_state()?;
        let cells = self.frame()?.cells().len();
        let (w, h) = layers::texture_size(cells);
        image_rgbaf(w, h, &sv::territory_texture(&st, cells, self.cell_area_km2()))
    }

    /// Noms des sociétés à poser sur le globe.
    #[func]
    fn society_labels(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.society_state() else { return out };
        for l in sv::labels(&st, self.lang) {
            let mut d = VarDictionary::new();
            d.set("id", l.id as i64);
            d.set("name", l.name.as_str());
            d.set("label", l.label.as_str());
            d.set("position", self.cell_position(l.cell));
            d.set("population", l.pop);
            d.set("colour", colour(l.colour));
            out.push(&d.to_variant());
        }
        out
    }

    /// Villes : position, nom, effectif, société, capitale ou non.
    #[func]
    fn society_cities(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.society_state() else { return out };
        let capitals: std::collections::BTreeSet<u32> = st.societies.iter().map(|s| s.capital).collect();
        let mut cities: Vec<_> = st.cities.iter().collect();
        cities.sort_by(|a, b| b.pop.total_cmp(&a.pop).then(a.id.cmp(&b.id)));
        for c in cities {
            let mut d = VarDictionary::new();
            d.set("id", c.id as i64);
            d.set("cell", c.cell as i64);
            d.set("name", c.name.as_str());
            d.set("population", c.pop);
            d.set("society", c.society as i64);
            d.set("capital", capitals.contains(&c.cell));
            d.set("position", self.cell_position(c.cell));
            out.push(&d.to_variant());
        }
        out
    }

    /// Routes d'échange : paires de points (deux par route), puis la valeur
    /// et la voie (mer) de chacune dans `values` (x : log₁₀ de la valeur ;
    /// y : 1 par mer).
    #[func]
    fn society_routes(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let mut points = Vec::new();
        let mut values = Vec::new();
        if let Some(st) = self.society_state() {
            for r in &st.routes {
                points.push(self.cell_position(r.a));
                points.push(self.cell_position(r.b));
                values.push(Vector2::new(r.value.max(1.0).log10() as f32, if r.sea { 1.0 } else { 0.0 }));
            }
        }
        d.set("points", &PackedVector3Array::from(&points[..]));
        d.set("values", &PackedVector2Array::from(&values[..]));
        d
    }

    /// Lumières nocturnes : position et éclat (0 à 1) par cellule.
    #[func]
    fn society_lights(&self) -> PackedVector4Array {
        let mut out = Vec::new();
        if let Some(st) = self.society_state() {
            for (cell, v) in sv::lights(&st) {
                let p = self.cell_position(cell);
                out.push(Vector4::new(p.x, p.y, p.z, v));
            }
        }
        PackedVector4Array::from(&out[..])
    }

    /// Liste des sociétés vivantes, les plus peuplées d'abord.
    #[func]
    fn society_list(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.society_state() else { return out };
        for r in sv::list(&st, self.lang) {
            let mut d = VarDictionary::new();
            d.set("id", r.id as i64);
            d.set("name", r.name.as_str());
            d.set("label", r.label.as_str());
            d.set("population", format::number_in(self.lang, r.pop.round(), 0));
            d.set("cells", r.cells as i64);
            d.set("techniques", r.techniques as i64);
            d.set("colour", colour(r.colour));
            d.set("crisis", r.crisis);
            d.set("war", r.war);
            out.push(&d.to_variant());
        }
        out
    }

    /// Fiche d'une société, ou {} si elle n'existe plus.
    #[func]
    fn society_sheet(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.society_state() else { return d };
        let Some(s) = sv::sheet(&st, id as u32, self.lang) else { return d };
        d.set("id", s.id as i64);
        d.set("name", s.name.as_str());
        d.set("label", s.label.as_str());
        d.set("peak", s.peak.as_str());
        d.set("summary", s.summary.as_str());
        d.set("colour", colour(s.colour));
        d.set("parent", s.parent.as_ref().map_or(-1, |p| p.0 as i64));
        d.set("parent_name", s.parent.as_ref().map_or("", |p| p.1.as_str()));
        d.set("founded", s.founded.as_str());
        d.set("cell", s.capital_cell as i64);
        d.set("capital", s.capital.as_str());
        d.set("people", s.people as i64);
        let pairs = |v: &[(String, String)]| {
            let mut a = VarArray::new();
            for (k, x) in v {
                let mut e = VarArray::new();
                e.push(&k.to_variant());
                e.push(&x.to_variant());
                a.push(&e.to_variant());
            }
            a
        };
        let gauges = |v: &[sv::Gauge]| {
            let mut a = VarArray::new();
            for g in v {
                let mut e = VarDictionary::new();
                e.set("name", g.name.as_str());
                e.set("value", g.value);
                a.push(&e.to_variant());
            }
            a
        };
        d.set("facts", &pairs(&s.facts));
        d.set("dims", &gauges(&s.dims));
        d.set("institutions", &gauges(&s.institutions));
        let mut sc = VarArray::new();
        for l in &s.sciences {
            let mut e = VarDictionary::new();
            e.set("domain", l.domain.as_str());
            e.set("quantity", l.quantity.as_str());
            e.set("estimate", l.estimate.as_str());
            e.set("truth", l.truth.as_str());
            e.set("precision", l.precision);
            e.set("paradigm", l.paradigm as i64);
            sc.push(&e.to_variant());
        }
        d.set("sciences", &sc);
        d.set("techniques", &pairs(&s.techniques));
        d.set("technique_count", s.technique_count as i64);
        let triples = |v: &[(u32, String, String)]| {
            let mut a = VarArray::new();
            for (id, n, x) in v {
                let mut e = VarDictionary::new();
                e.set("id", *id as i64);
                e.set("name", n.as_str());
                e.set("detail", x.as_str());
                a.push(&e.to_variant());
            }
            a
        };
        d.set("relations", &triples(&s.relations));
        d.set("cities", &triples(&s.cities));
        d.set("notables", &triples(&s.notables));
        d
    }

    /// Comparaison de deux sociétés, trait par trait.
    #[func]
    fn society_compare(&self, a: i64, b: i64) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.society_state() else { return out };
        for r in sv::compare(&st, a as u32, b as u32, self.lang).unwrap_or_default() {
            let mut d = VarDictionary::new();
            d.set("name", r.name.as_str());
            d.set("a", r.a.as_str());
            d.set("b", r.b.as_str());
            d.set("differs", r.differs);
            out.push(&d.to_variant());
        }
        out
    }

    /// Courbe d'une société dans le temps : (année, valeur). `key` :
    /// population, cells, gdp_pc, food_pc, urban, literacy, modern_energy,
    /// techniques.
    #[func]
    fn society_series(&self, id: i64, key: GString) -> PackedVector2Array {
        let Some(sh) = self.society_shared() else { return PackedVector2Array::new() };
        let s = sh.lock().unwrap();
        let key = key.to_string();
        let v: Vec<Vector2> = s
            .history
            .samples
            .get(&(id as u32))
            .map(|v| {
                v.iter()
                    .map(|x| {
                        let y = match key.as_str() {
                            "population" => x.pop,
                            "cells" => x.cells as f64,
                            "gdp_pc" => x.gdp_pc,
                            "food_pc" => x.food_pc,
                            "urban" => x.urban,
                            "literacy" => x.literacy,
                            "modern_energy" => x.modern_energy,
                            _ => x.techniques as f64,
                        };
                        Vector2::new(x.year as f32, y as f32)
                    })
                    .collect()
            })
            .unwrap_or_default();
        PackedVector2Array::from(&v[..])
    }

    /// Chronique des sociétés, les plus récents d'abord : au plus `limit`
    /// entrées dont l'intérêt dépasse `min_interest`, filtrées par `query`.
    #[func]
    fn society_events(&self, limit: i64, min_interest: f64, query: GString) -> VarArray {
        let mut out = VarArray::new();
        let Some(sh) = self.society_shared() else { return out };
        let s = sh.lock().unwrap();
        let q = query.to_string().to_lowercase();
        for e in s
            .history
            .events
            .iter()
            .rev()
            .filter(|e| e.interest >= min_interest && (q.is_empty() || e.text.to_lowercase().contains(&q)))
            .take(limit.max(0) as usize)
        {
            let mut d = VarDictionary::new();
            d.set("year", sv::year_text(e.year, self.lang).as_str());
            d.set("text", e.text.as_str());
            d.set("cell", e.cell.map_or(-1, |c| c as i64));
            d.set("society", e.society.map_or(-1, |c| c as i64));
            d.set("interest", e.interest);
            out.push(&d.to_variant());
        }
        out
    }

    /// Le récit d'une période de l'histoire d'essai, en paragraphes.
    #[func]
    fn society_story(&self, from_year: i64, to_year: i64) -> PackedStringArray {
        let Some(sh) = self.society_shared() else { return PackedStringArray::new() };
        let s = sh.lock().unwrap();
        let v: Vec<GString> = sv::story(&s.history, from_year, to_year, self.lang).iter().map(GString::from).collect();
        PackedStringArray::from(&v[..])
    }

    /// Première et dernière année de l'histoire d'essai gardée.
    #[func]
    fn society_span(&self) -> Vector2i {
        let Some(sh) = self.society_shared() else { return Vector2i::ZERO };
        let s = sh.lock().unwrap();
        Vector2i::new(s.history.years.first().copied().unwrap_or(0) as i32, s.state.as_ref().map_or(0, |x| x.year) as i32)
    }

    /// Bilan des sociétés pour la planète : [nom, valeur] par ligne.
    #[func]
    fn society_planet(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.society_state() else { return out };
        for (k, v) in sv::planet_lines(&st, self.lang) {
            let mut e = VarArray::new();
            e.push(&k.to_variant());
            e.push(&v.to_variant());
            out.push(&e.to_variant());
        }
        out
    }

    /// La société d'une cellule, pour l'inspecteur : {} si personne.
    #[func]
    fn society_at(&self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.society_state() else { return d };
        let Some(c) = st.cells.iter().find(|c| c.cell as i64 == cell) else { return d };
        let Some(s) = st.societies.iter().find(|s| s.id == c.society) else { return d };
        let fr = self.lang == Lang::Fr;
        let city = c.city.and_then(|id| st.cities.iter().find(|x| x.id == id)).map(|x| x.name.clone());
        let text = if fr {
            format!(
                "{} ({}) : {} âmes ici{}, {} % en ville, {} rations par tête.",
                s.name,
                sv::label_name(s.label, self.lang),
                format::number_in(self.lang, c.pop.round(), 0),
                city.map_or(String::new(), |n| format!(", ville de {n}")),
                (c.urban * 100.0).round(),
                format::number_in(self.lang, c.food_pc, 2)
            )
        } else {
            format!(
                "{} ({}): {} souls here{}, {}% urban, {} rations per head.",
                s.name,
                sv::label_name(s.label, self.lang),
                format::number_in(self.lang, c.pop.round(), 0),
                city.map_or(String::new(), |n| format!(", city of {n}")),
                (c.urban * 100.0).round(),
                format::number_in(self.lang, c.food_pc, 2)
            )
        };
        d.set("society", s.id as i64);
        d.set("text", text.as_str());
        d
    }

    /// Le tableau des simplifications assumées : source, titre, réalité,
    /// ce qu'on perd, ce qu'on garde.
    #[func]
    fn encyclopedia_simplifications(&self) -> VarArray {
        let mut out = VarArray::new();
        for s in SIMPLIFICATIONS {
            let mut d = VarDictionary::new();
            d.set("source", s.source);
            d.set("title", s.title);
            d.set("reality", s.reality);
            d.set("lost", s.lost);
            d.set("kept", s.kept);
            out.push(&d.to_variant());
        }
        out
    }

    /// Le catalogue des techniques, dans l'ordre de la chaîne des prérequis.
    #[func]
    fn encyclopedia_techniques(&self) -> VarArray {
        let mut out = VarArray::new();
        for t in sv::techniques(self.lang) {
            let mut d = VarDictionary::new();
            d.set("id", t.id as i64);
            d.set("name", t.name.as_str());
            d.set("on_earth", t.on_earth.as_str());
            d.set("requires", t.requires.join(", ").as_str());
            d.set("depth", t.depth as i64);
            d.set("path", t.path.as_str());
            out.push(&d.to_variant());
        }
        out
    }
}
