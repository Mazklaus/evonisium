//! Sociétés et histoire à l'écran (étape 7) : l'état des sociétés publié
//! (`evo_society::state::SocietyState`) mis en forme pour le globe, les
//! fiches, la comparaison de civilisations et la chronique.
//!
//! Le moteur ne fait pas encore vivre de sociétés : l'intégration
//! d'evo-society vient après celle de l'étape 6. En attendant,
//! [`bench_world`] pose des sociétés d'essai sur les terres de la planète en
//! cours (milieu tiré des cellules, espèces sauvages de la Terre réparties
//! selon le climat) et [`History`] garde ce que le client montre de leur
//! passé. Rien de ce banc ne remonte vers le monde ; l'écran le dit au
//! joueur.

use crate::format::{number_in, percent, Lang};
use evo_core::math::Det;
use evo_culture::catalog::Milieu;
use evo_culture::cognition::presets;
use evo_culture::CulturalState;
use evo_planet::grid::GeodesicGrid;
use evo_sim::history::CellView;
use evo_society::bench::upper_palaeolithic;
use evo_society::catalog::catalog;
use evo_society::chronicle::{Event, EventKind};
use evo_society::land::{AnimalTraits, Deposits, Land, PlantOrgan, PlantTraits, WildKind, WildSpecies};
use evo_society::science::{Domain, Truths};
use evo_society::society::{Dimensions, Institution, Label};
use evo_society::state::{RelationView, SocietyState};
use evo_society::world::{RelationKind, SocietyWorld};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Espèce intelligente des sociétés d'essai : celle des peuples d'essai de
/// l'étape 6.
pub const BENCH_HUMAN: u32 = crate::culture::BENCH_HUMAN;

/// Réglages du banc.
#[derive(Clone, Copy, Debug)]
pub struct BenchSpec {
    pub seed: u64,
    /// Terres prises, au plus (le coût d'un an suit leur nombre).
    pub max_cells: usize,
    pub start_year: i64,
    pub end_year: i64,
}

impl Default for BenchSpec {
    fn default() -> Self {
        BenchSpec { seed: 2026, max_cells: 700, start_year: -12_000, end_year: 2_100 }
    }
}

/// Un monde de sociétés d'essai et la cellule du globe de chacune de ses
/// cellules.
pub struct Bench {
    pub world: SocietyWorld,
    /// Cellule du globe de chaque cellule du monde des sociétés.
    pub globe_cell: Vec<u32>,
    pub spec: BenchSpec,
}

impl Bench {
    /// Avance d'un an.
    pub fn step(&mut self) {
        self.world.step();
    }

    pub fn done(&self) -> bool {
        self.world.year >= self.spec.end_year
    }

    /// L'état publié, ses cellules rendues à celles du globe.
    pub fn state(&self) -> SocietyState {
        let mut s = SocietyState::of(&self.world);
        let g = |c: u32| self.globe_cell.get(c as usize).copied().unwrap_or(c);
        for x in &mut s.societies {
            x.capital = g(x.capital);
            for c in &mut x.cells {
                *c = g(*c);
            }
        }
        for c in &mut s.cells {
            c.cell = g(c.cell);
        }
        for c in &mut s.cities {
            c.cell = g(c.cell);
        }
        for r in &mut s.routes {
            r.a = g(r.a);
            r.b = g(r.b);
        }
        s
    }
}

/// Terre où une espèce terrestre peut vivre.
fn habitable(c: &CellView) -> bool {
    !c.is_ocean && c.ice_cover < 0.5 && (255.0..318.0).contains(&c.temperature_k)
}

fn hash(seed: u64, a: u64, b: u64) -> f64 {
    let mut x = seed ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// Milieu d'une cellule du monde des sociétés, lu dans la cellule du globe.
/// [Simplification] Mêmes règles que les bancs de la Terre (terres
/// labourables par la chaleur, la pluie et le relief, forêts par la pluie),
/// gisements tirés au hasard de la cellule.
fn land_of(c: &CellView, lat: f64, lon: f64, area_km2: f64, coast: bool, seed: u64, i: u64) -> Land {
    let temp = c.temperature_k as f64 - 273.15;
    let rain = c.rain_mm_yr as f64;
    let relief = (c.elevation_m as f64 / 3500.0).clamp(0.05, 1.0);
    let river = ((c.river_flow_m3s as f64).max(1.0).dln() / 9.0).clamp(0.0, 0.8) + 0.3 * c.lake_fraction as f64;
    let warm = ((temp - 2.0) / 8.0).clamp(0.0, 1.0);
    let wet = ((rain - 250.0) / 400.0).clamp(0.0, 1.0);
    let arable = (0.35 * (1.0 - relief) * warm * wet).max(0.25 * river.min(1.0));
    let pasture = (0.45 * warm.max(0.3) * ((rain - 120.0) / 300.0).clamp(0.0, 1.0) * (1.0 - 0.5 * wet)).max(0.05);
    let forest =
        if rain > 500.0 && temp > -2.0 { 16_000.0 * (rain / 2000.0).min(1.0) * ((temp + 5.0) / 30.0).clamp(0.2, 1.0) } else { 0.0 };
    let n = |k: u64| hash(seed, i, k);
    let mut deposits = Deposits { iron: 2e8 * n(1), ..Default::default() };
    if n(2) > 0.7 {
        deposits.salt = 1e4;
    }
    if n(3) > 0.85 {
        deposits.coal_c = 4e10 * n(4);
    }
    if n(5) > 0.92 {
        deposits.oil_c = 3e10 * n(6);
    }
    if relief > 0.3 && n(7) > 0.75 {
        deposits.copper = 5e6 * n(8);
    }
    if relief > 0.3 && n(9) > 0.9 {
        deposits.tin = 1e6 * n(10);
    }
    if n(11) > 0.97 {
        deposits.uranium = 1e5;
    }
    Land {
        area_km2,
        lat_deg: lat,
        lon_deg: lon,
        milieu: Milieu::Land,
        ocean: false,
        coast,
        temp_c: temp,
        rain_mm: rain,
        variability: 0.1,
        relief,
        river: river.min(1.0),
        arable,
        pasture,
        forest_c_km2: forest,
        deposits,
        wind: if lat.abs() > 40.0 || relief > 0.4 { 0.6 } else { 0.35 },
        sun: if rain < 400.0 { 0.85 } else { 0.5 },
        geothermal: if relief > 0.6 { 0.4 } else { 0.05 },
        o2_percent: 21.0,
        sky: c.cloud_cover < 0.9,
        wild: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn plant(
    id: u32,
    name: &str,
    organ: PlantOrgan,
    seed_mg: f64,
    annual: bool,
    selfing: bool,
    yield_t_ha: f64,
    temp_c: f64,
    rain_mm: f64,
) -> WildSpecies {
    WildSpecies {
        id,
        name: name.to_string(),
        kind: WildKind::Plant(PlantTraits { organ, seed_mg, annual, selfing, wild_yield_t_ha: yield_t_ha, temp_c, rain_mm }),
    }
}

#[allow(clippy::too_many_arguments)]
fn animal(
    id: u32,
    name: &str,
    mass_kg: f64,
    herbivory: f64,
    maturity_years: f64,
    breeds: bool,
    aggression: f64,
    panic: f64,
    herd: f64,
    milk: bool,
) -> WildSpecies {
    WildSpecies {
        id,
        name: name.to_string(),
        kind: WildKind::Animal(AnimalTraits {
            mass_kg,
            herbivory,
            maturity_years,
            captive_breeding: breeds,
            aggression,
            panic,
            herd,
            milk,
        }),
    }
}

/// Espèces sauvages d'essai, prises aux bancs de la Terre : des herbes à
/// grosses graines, des racines, des bêtes de troupeau dociles ou non.
/// [Simplification] Le moteur les tirera des vraies populations.
fn wild_species() -> Vec<WildSpecies> {
    use PlantOrgan::*;
    vec![
        plant(100, "une herbe à grosses graines", Seed, 30.0, true, true, 0.8, 16.0, 450.0),
        plant(101, "une herbe des steppes", Seed, 35.0, true, true, 0.7, 15.0, 400.0),
        plant(102, "une herbe des marais", Seed, 25.0, true, true, 0.6, 20.0, 1300.0),
        plant(103, "une herbe à petites graines", Seed, 8.0, true, true, 0.6, 12.0, 500.0),
        plant(104, "une herbe des collines chaudes", Seed, 20.0, true, false, 0.5, 21.0, 900.0),
        plant(105, "un tubercule des hauteurs", Root, 0.0, false, false, 0.6, 10.0, 800.0),
        plant(106, "un tubercule des forêts humides", Root, 0.0, false, false, 0.6, 25.0, 2500.0),
        plant(107, "une herbe des savanes", Seed, 25.0, true, true, 0.5, 26.0, 650.0),
        animal(200, "un petit ruminant des rochers", 50.0, 1.0, 1.0, true, 0.2, 0.3, 0.9, true),
        animal(201, "un grand bovin sauvage", 700.0, 1.0, 2.0, true, 0.5, 0.3, 0.8, true),
        animal(202, "un porc des forêts", 100.0, 0.6, 1.0, true, 0.4, 0.3, 0.5, false),
        animal(203, "un coureur des steppes", 400.0, 1.0, 2.0, true, 0.3, 0.5, 0.8, true),
        animal(206, "un équidé rétif", 350.0, 1.0, 2.0, true, 0.9, 0.8, 0.7, false),
        animal(209, "un cervidé farouche", 150.0, 1.0, 2.0, true, 0.3, 0.9, 0.4, false),
    ]
}

/// Climat du banc. [Simplification] Les sociétés d'essai vivent sur la
/// géographie de la planète (côtes, latitudes, montagnes relatives, régions
/// plus ou moins arrosées) sous un climat ramené à celui de la Terre de
/// l'holocène : la planète de la partie peut être gelée, sèche ou faite de
/// hauts plateaux, et l'histoire d'essai doit pouvoir montrer l'affichage.
fn mild_climate(grid: &GeodesicGrid, cells: &[CellView]) -> Vec<CellView> {
    let land: Vec<usize> = (0..cells.len()).filter(|&i| !cells[i].is_ocean).collect();
    if land.is_empty() {
        return cells.to_vec();
    }
    // Rang de l'altitude parmi les terres, de 0 à 1.
    let mut by_height = land.clone();
    by_height.sort_by(|&a, &b| cells[a].elevation_m.total_cmp(&cells[b].elevation_m).then(a.cmp(&b)));
    let mut rank = vec![0.0f32; cells.len()];
    let n = (by_height.len().max(2) - 1) as f32;
    for (k, &i) in by_height.iter().enumerate() {
        rank[i] = k as f32 / n;
    }
    let mean_rain = (land.iter().map(|&i| cells[i].rain_mm_yr.max(1.0) as f64).sum::<f64>() / land.len() as f64) as f32;
    cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut v = *c;
            if !c.is_ocean {
                // Le dixième le plus haut des terres fait les montagnes.
                let height_km = 3.5 * rank[i].powi(3);
                let lat = grid.latitude(i).to_degrees().abs() as f32;
                v.elevation_m = 1000.0 * height_km;
                v.temperature_k = 273.15 + 28.0 - 0.6 * lat - 6.5 * height_km;
                v.rain_mm_yr = (1000.0 * (c.rain_mm_yr.max(1.0) / mean_rain).powf(0.7)).clamp(60.0, 3000.0);
                v.ice_cover = if v.temperature_k < 263.0 { 1.0 } else { 0.0 };
            }
            v
        })
        .collect()
}

/// Région connexe de terres habitables autour de `start`, en largeur
/// d'abord.
fn region(grid: &GeodesicGrid, cells: &[CellView], start: usize, n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(c) = queue.pop_front() {
        out.push(c);
        if out.len() >= n {
            break;
        }
        for nb in grid.neighbours_of(c) {
            if nb < cells.len() && habitable(&cells[nb]) && seen.insert(nb) {
                queue.push_back(nb);
            }
        }
    }
    out
}

/// Cellule de départ : le cœur des peuples d'essai s'il y en a, sinon la
/// terre la plus douce dont la région est assez grande.
fn start_cell(grid: &GeodesicGrid, cells: &[CellView], culture: Option<&CulturalState>, n: usize) -> Option<usize> {
    if let Some(cs) = culture {
        let mut groups: Vec<_> = cs.groups.iter().filter(|g| g.species == BENCH_HUMAN && (g.cell as usize) < cells.len()).collect();
        groups.sort_by(|a, b| b.population.total_cmp(&a.population).then(a.cell.cmp(&b.cell)));
        if let Some(g) = groups.into_iter().find(|g| habitable(&cells[g.cell as usize])) {
            return Some(g.cell as usize);
        }
    }
    let mut candidates: Vec<(f32, usize)> = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| habitable(c))
        .map(|(i, c)| ((c.temperature_k - 288.0).abs() - 0.002 * c.rain_mm_yr.min(2000.0), i))
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    candidates.into_iter().take(200).map(|(_, i)| i).find(|&i| region(grid, cells, i, n).len() >= n / 2)
}

/// Des sociétés d'essai sur les terres de la planète : une région d'au plus
/// `max_cells` terres habitables autour des peuples d'essai, peuplée de
/// chasseurs-cueilleurs du Paléolithique supérieur. `None` si la planète
/// n'a pas assez de terres habitables.
pub fn bench_world(grid: &GeodesicGrid, cells: &[CellView], culture: Option<&CulturalState>, spec: BenchSpec) -> Option<Bench> {
    let mild = mild_climate(grid, cells);
    let cells = &mild[..];
    let start = start_cell(grid, cells, culture, spec.max_cells)?;
    let picked = region(grid, cells, start, spec.max_cells);
    if picked.len() < 20 {
        return None;
    }
    let mut world = SocietyWorld::new(spec.seed, Truths::earth());
    world.year = spec.start_year;
    world.add_species(BENCH_HUMAN, "hominidés d'essai", presets::human(), Milieu::Land);
    // [Simplification] Rayon de la Terre : la grille ne connaît que des
    // directions.
    let area = 4.0 * std::f64::consts::PI * 6371.0 * 6371.0 / grid.len() as f64;
    let index: BTreeMap<usize, u32> = picked.iter().enumerate().map(|(k, &c)| (c, k as u32)).collect();
    let wild = wild_species();
    // Chaque espèce sauvage vit autour d'un foyer, là où le climat lui va le
    // mieux, avec un peu de hasard : d'où des foyers agricoles distincts.
    let mut ranges: Vec<BTreeSet<usize>> = Vec::new();
    for (k, w) in wild.iter().enumerate() {
        let best = picked
            .iter()
            .map(|&c| {
                let v = &cells[c];
                (w.suits(v.temperature_k as f64 - 273.15, v.rain_mm_yr as f64) * (0.6 + 0.4 * hash(spec.seed, c as u64, 100 + k as u64)), c)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        let mut range = BTreeSet::new();
        if let Some((s, home)) = best {
            if s > 0.3 {
                let radius = if matches!(w.kind, WildKind::Animal(_)) { 4 } else { 2 };
                let mut frontier = vec![home];
                range.insert(home);
                for _ in 0..radius {
                    let mut next = Vec::new();
                    for c in frontier {
                        for nb in grid.neighbours_of(c) {
                            if index.contains_key(&nb) && range.insert(nb) {
                                next.push(nb);
                            }
                        }
                    }
                    frontier = next;
                }
            }
        }
        ranges.push(range);
    }
    let mut people_at: BTreeMap<u32, u32> = BTreeMap::new();
    if let Some(cs) = culture {
        for g in cs.groups.iter().filter(|g| g.species == BENCH_HUMAN) {
            people_at.insert(g.cell, g.people);
        }
    }
    for &c in &picked {
        let v = &cells[c];
        let lat = grid.latitude(c).to_degrees();
        let p = grid.centers[c];
        let lon = p[1].atan2(p[0]).to_degrees();
        let coast = grid.neighbours_of(c).any(|nb| nb < cells.len() && cells[nb].is_ocean);
        let mut land = land_of(v, lat, lon, area, coast, spec.seed, c as u64);
        for (k, w) in wild.iter().enumerate() {
            if ranges[k].contains(&c) {
                land.wild.push(w.id);
            }
        }
        world.add_cell(land);
    }
    for w in wild {
        world.add_wild(w);
    }
    for &c in &picked {
        let a = index[&c];
        for nb in grid.neighbours_of(c) {
            if let Some(&b) = index.get(&nb) {
                if a < b {
                    let (la, lb) = (&world.cells[a as usize].land, &world.cells[b as usize].land);
                    let rough = 1.0 + 1.5 * la.relief.max(lb.relief) + if la.rain_mm.min(lb.rain_mm) < 150.0 { 1.0 } else { 0.0 };
                    world.link(a, b, rough, false);
                }
            }
        }
    }
    // Détroits : deux côtes de la région séparées par une cellule de mer.
    for &c in &picked {
        let a = index[&c];
        if !world.cells[a as usize].land.coast {
            continue;
        }
        for sea in grid.neighbours_of(c).filter(|&nb| nb < cells.len() && cells[nb].is_ocean) {
            for nb in grid.neighbours_of(sea) {
                if let Some(&b) = index.get(&nb) {
                    if a < b && !world.cells[a as usize].links.iter().any(|l| l.to == b) {
                        world.link(a, b, 1.5, true);
                    }
                }
            }
        }
    }
    // Des chasseurs-cueilleurs partout où la cueillette nourrit assez de
    // monde, du peuple d'essai le plus proche s'il y en a un.
    let known = upper_palaeolithic();
    let mut last_people = 0u32;
    for (k, &c) in picked.iter().enumerate() {
        let land = &world.cells[k].land;
        let cap = land.area_km2 * land.npp() * evo_society::economy::FORAGE_PER_NPP;
        if cap < 300.0 {
            continue;
        }
        let people = people_at.get(&(c as u32)).copied().unwrap_or(last_people);
        last_people = people;
        world.settle(k as u32, BENCH_HUMAN, cap * 0.5, known, people);
    }
    world.alive_societies().next()?;
    let globe_cell = picked.iter().map(|&c| c as u32).collect();
    Some(Bench { world, globe_cell, spec })
}

/// Ce que le client garde d'une société à chaque relevé, pour comparer les
/// civilisations dans le temps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    pub year: i64,
    pub pop: f64,
    pub cells: u32,
    pub gdp_pc: f64,
    pub food_pc: f64,
    pub urban: f64,
    pub literacy: f64,
    pub modern_energy: f64,
    pub techniques: u32,
    pub label: u8,
}

/// Le passé tel que le client le garde : relevés par société, chronique
/// complète et ce qu'on sait des sociétés disparues.
#[derive(Clone, Debug, Default)]
pub struct History {
    pub samples: BTreeMap<u32, Vec<Sample>>,
    pub names: BTreeMap<u32, String>,
    pub events: Vec<Event>,
    /// Années des relevés.
    pub years: Vec<i64>,
}

impl History {
    /// Ajoute un relevé et les événements écoulés depuis le précédent.
    pub fn record(&mut self, s: &SocietyState, events: &[Event]) {
        self.years.push(s.year);
        for x in &s.societies {
            self.names.insert(x.id, x.name.clone());
            self.samples.entry(x.id).or_default().push(Sample {
                year: s.year,
                pop: x.agg.population,
                cells: x.cells.len() as u32,
                gdp_pc: if x.agg.population > 0.0 { x.agg.gdp / x.agg.population } else { 0.0 },
                food_pc: x.agg.food_pc,
                urban: x.agg.urban,
                literacy: x.agg.literacy,
                modern_energy: x.agg.modern_energy,
                techniques: x.known.count(),
                label: x.label as u8,
            });
        }
        self.events.extend_from_slice(events);
    }
}

// ----------------------------------------------------------------------
// Mise en forme pour l'écran

/// Couleurs des sociétés : celles des peuples (neuf lavis de l'Atlas).
pub const SOCIETY_COLOURS: [[f32; 3]; 9] = crate::culture::PEOPLE_COLOURS;

/// Rang de couleur d'une société (1 à 9) : une société garde sa couleur.
pub fn colour_index(society: u32) -> u32 {
    society % SOCIETY_COLOURS.len() as u32 + 1
}

pub fn colour(society: u32) -> [f32; 3] {
    SOCIETY_COLOURS[(colour_index(society) - 1) as usize]
}

/// Année d'histoire : « an −8 000 », « an 1750 ».
pub fn year_text(year: i64, lang: Lang) -> String {
    let n = if year.abs() >= 10_000 {
        number_in(lang, year as f64, 0)
    } else if year < 0 {
        format!("−{}", -year)
    } else {
        year.to_string()
    };
    match lang {
        Lang::Fr => format!("an {n}"),
        Lang::En => format!("year {n}"),
    }
}

/// Texture des sociétés pour le globe (même forme que celle des peuples) :
/// rang de couleur, log₁₀ de l'effectif, part des terres en champs, 1.
pub fn territory_texture(state: &SocietyState, cells: usize, cell_area_km2: f64) -> Vec<f32> {
    let (w, h) = crate::layers::texture_size(cells);
    let mut t = vec![0.0f32; w * h * 4];
    for c in &state.cells {
        let i = c.cell as usize;
        if i >= cells {
            continue;
        }
        t[i * 4] = colour_index(c.society) as f32;
        t[i * 4 + 1] = c.pop.max(1.0).log10() as f32;
        t[i * 4 + 2] = (c.cropland_km2 / cell_area_km2.max(1.0)).clamp(0.0, 1.0).sqrt() as f32;
        t[i * 4 + 3] = 1.0;
    }
    t
}

/// Nom d'une société sur le globe, posé sur sa capitale.
#[derive(Clone, Debug, PartialEq)]
pub struct SocietyLabel {
    pub id: u32,
    pub name: String,
    pub label: String,
    pub cell: u32,
    pub pop: f64,
    pub colour: [f32; 3],
}

/// Les sociétés assez grandes pour être nommées sur le globe (trois
/// cellules au moins ou une ville), les plus peuplées d'abord.
pub fn labels(state: &SocietyState, lang: Lang) -> Vec<SocietyLabel> {
    let mut out: Vec<SocietyLabel> = state
        .societies
        .iter()
        .filter(|s| s.cells.len() >= 3 || state.cities.iter().any(|c| c.society == s.id))
        .map(|s| SocietyLabel {
            id: s.id,
            name: s.name.clone(),
            label: label_name(s.label, lang).into(),
            cell: s.capital,
            pop: s.agg.population,
            colour: colour(s.id),
        })
        .collect();
    out.sort_by(|a, b| b.pop.total_cmp(&a.pop).then(a.id.cmp(&b.id)));
    out.truncate(40);
    out
}

pub fn label_name(l: Label, lang: Lang) -> &'static str {
    match (lang, l) {
        (Lang::Fr, _) => l.label(),
        (Lang::En, Label::Bands) => "bands",
        (Lang::En, Label::Tribe) => "tribe",
        (Lang::En, Label::Chiefdom) => "chiefdom",
        (Lang::En, Label::State) => "state",
        (Lang::En, Label::Empire) => "empire",
        (Lang::En, Label::ModernState) => "modern state",
    }
}

/// Lumière nocturne d'une cellule habitée : les villes et les campagnes
/// denses brillent, d'autant plus avec l'énergie moderne. 0 à 1.
pub fn lights(state: &SocietyState) -> Vec<(u32, f32)> {
    let mut out = Vec::new();
    for c in &state.cells {
        let city = state.cities.iter().find(|x| Some(x.id) == c.city).map_or(0.0, |x| x.pop);
        let urban = (c.pop * c.urban).max(city);
        // Une ville de 10 000 âmes se voit à peine à la lampe à huile ; une
        // métropole électrique éclaire tout le ciel.
        let lamp = 0.25 + 0.75 * c.modern_energy.clamp(0.0, 1.0);
        let v = ((urban.max(1.0).log10() - 3.0) / 4.0).clamp(0.0, 1.0) * lamp;
        if v > 0.02 {
            out.push((c.cell, v as f32));
        }
    }
    out
}

fn dims_rows(d: &Dimensions, lang: Lang) -> Vec<(String, f64)> {
    let names: [(&str, &str); 6] = [
        ("centralisation", "centralisation"),
        ("inégalités", "inequality"),
        ("coercition", "coercion"),
        ("participation", "participation"),
        ("administration", "administration"),
        ("cohésion", "cohesion"),
    ];
    let v = [d.centralisation, d.inequality, d.coercion, d.participation, d.admin, d.cohesion];
    names.iter().zip(v).map(|(n, x)| ((if lang == Lang::Fr { n.0 } else { n.1 }).to_string(), x)).collect()
}

fn institution_name(i: Institution, lang: Lang) -> &'static str {
    if lang == Lang::Fr {
        return i.label();
    }
    match i {
        Institution::Council => "council",
        Institution::Ruler => "ruler",
        Institution::Court => "court",
        Institution::Temple => "temple",
        Institution::Army => "army",
        Institution::Market => "market",
        Institution::School => "school",
    }
}

fn domain_name(d: Domain, lang: Lang) -> &'static str {
    if lang == Lang::Fr {
        return d.label();
    }
    match d {
        Domain::Mechanics => "mechanics",
        Domain::Chemistry => "chemistry",
        Domain::Climate => "climate",
        Domain::Geology => "geology",
        Domain::Biology => "biology",
        Domain::Heredity => "heredity",
        Domain::Disease => "disease",
        Domain::Astronomy => "astronomy",
    }
}

fn relation_name(r: &RelationView, me: u32, lang: Lang) -> &'static str {
    let fr = lang == Lang::Fr;
    match r.kind {
        RelationKind::Peace => {
            if fr {
                "paix"
            } else {
                "peace"
            }
        }
        RelationKind::Trade => {
            if fr {
                "commerce"
            } else {
                "trade"
            }
        }
        RelationKind::Alliance => "alliance",
        RelationKind::Tribute { payer } => match (payer == me, fr) {
            (true, true) => "lui paie tribut",
            (true, false) => "pays it tribute",
            (false, true) => "reçoit son tribut",
            (false, false) => "receives its tribute",
        },
        RelationKind::War { .. } => {
            if fr {
                "guerre"
            } else {
                "war"
            }
        }
    }
}

/// Une ligne nommée de valeur entre 0 et 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Gauge {
    pub name: String,
    pub value: f64,
}

/// Ce qu'une société croit d'un domaine.
#[derive(Clone, Debug, PartialEq)]
pub struct ScienceLine {
    pub domain: String,
    pub quantity: String,
    pub estimate: String,
    pub truth: String,
    pub precision: f64,
    pub paradigm: u16,
}

/// Fiche d'une société.
#[derive(Clone, Debug, PartialEq)]
pub struct SocietySheet {
    pub id: u32,
    pub name: String,
    pub label: String,
    pub peak: String,
    pub summary: String,
    pub colour: [f32; 3],
    pub parent: Option<(u32, String)>,
    pub founded: String,
    pub capital_cell: u32,
    pub capital: String,
    pub people: u32,
    pub facts: Vec<(String, String)>,
    pub dims: Vec<Gauge>,
    pub institutions: Vec<Gauge>,
    pub sciences: Vec<ScienceLine>,
    /// Techniques connues, les plus avancées d'abord : nom, équivalent
    /// terrestre.
    pub techniques: Vec<(String, String)>,
    pub technique_count: u32,
    pub relations: Vec<(u32, String, String)>,
    pub cities: Vec<(u32, String, String)>,
    pub notables: Vec<(u32, String, String)>,
}

fn money(x: f64, lang: Lang) -> String {
    if x.abs() >= 1e6 {
        crate::format::power_of_ten_in(lang, x)
    } else {
        number_in(lang, x, if x.abs() >= 10.0 { 0 } else { 1 })
    }
}

/// Profondeur de chaque technique dans la chaîne des prérequis : le repère
/// d'« avancée » des listes.
pub fn technique_depths() -> Vec<u32> {
    let cat = catalog();
    let mut depth = vec![0u32; cat.len()];
    // Les prérequis précèdent toujours dans le catalogue.
    for t in &cat.techniques {
        let mut d = 0;
        for &p in t.pre.iter().chain(t.any.iter().flatten()) {
            d = d.max(depth[p as usize] + 1);
        }
        depth[t.id as usize] = d;
    }
    depth
}

pub fn sheet(state: &SocietyState, id: u32, lang: Lang) -> Option<SocietySheet> {
    let s = state.societies.iter().find(|s| s.id == id)?;
    let fr = lang == Lang::Fr;
    let cat = catalog();
    let a = &s.agg;
    let name_of = |id: u32| state.societies.iter().find(|x| x.id == id).map(|x| x.name.clone());
    let capital = state.cities.iter().filter(|c| c.society == s.id).max_by(|x, y| x.pop.total_cmp(&y.pop)).map(|c| c.name.clone());
    let label = label_name(s.label, lang).to_string();
    let pop = number_in(lang, a.population.round(), 0);
    let summary = if fr {
        format!(
            "{} : {} de {} âmes sur {} cellule{}, fondée en {}.{}{}",
            s.name,
            label,
            pop,
            s.cells.len(),
            if s.cells.len() > 1 { "s" } else { "" },
            year_text(s.founded, lang).trim_start_matches("an "),
            if s.in_crisis { " Elle traverse une crise." } else { "" },
            if s.at_war { " Elle est en guerre." } else { "" }
        )
    } else {
        format!(
            "{}: a {} of {} souls over {} cell{}, founded in {}.{}{}",
            s.name,
            label,
            pop,
            s.cells.len(),
            if s.cells.len() > 1 { "s" } else { "" },
            year_text(s.founded, lang).trim_start_matches("year "),
            if s.in_crisis { " It is in crisis." } else { "" },
            if s.at_war { " It is at war." } else { "" }
        )
    };
    let gdp_pc = if a.population > 0.0 { a.gdp / a.population } else { 0.0 };
    let mut facts = vec![
        ("Population".to_string(), pop.clone()),
        ((if fr { "Cellules" } else { "Cells" }).to_string(), s.cells.len().to_string()),
        ((if fr { "Rations par tête" } else { "Rations per head" }).to_string(), number_in(lang, a.food_pc, 2)),
        ((if fr { "Produit par tête (rations)" } else { "Output per head (rations)" }).to_string(), money(gdp_pc, lang)),
        ((if fr { "Part urbaine" } else { "Urban share" }).to_string(), percent(a.urban, lang)),
        ((if fr { "Alphabétisation" } else { "Literacy" }).to_string(), percent(a.literacy, lang)),
        ((if fr { "Énergie moderne" } else { "Modern energy" }).to_string(), percent(a.modern_energy, lang)),
        ((if fr { "Élites" } else { "Elites" }).to_string(), percent(a.elites, lang)),
        ((if fr { "Légitimité" } else { "Legitimacy" }).to_string(), percent(a.legitimacy.clamp(0.0, 1.0), lang)),
        ((if fr { "Tension" } else { "Stress" }).to_string(), number_in(lang, a.stress, 2)),
    ];
    // Pas de dette sans trésor (evo-society) : on ne la montre que si elle existe.
    if a.debt > 0.0 {
        facts.push(((if fr { "Dette (rations)" } else { "Debt (rations)" }).to_string(), money(a.debt, lang)));
    }
    let dims = dims_rows(&s.dims, lang).into_iter().map(|(name, value)| Gauge { name, value }).collect();
    let institutions =
        Institution::ALL.iter().zip(s.institutions).map(|(i, v)| Gauge { name: institution_name(*i, lang).into(), value: v }).collect();
    let sciences = s
        .sciences
        .iter()
        .map(|sc| {
            let digits = if sc.truth.abs() >= 100.0 {
                0
            } else if sc.truth.abs() >= 1.0 {
                1
            } else {
                3
            };
            ScienceLine {
                domain: domain_name(sc.domain, lang).into(),
                quantity: sc.domain.quantity().into(),
                estimate: number_in(lang, sc.estimate, digits),
                truth: number_in(lang, sc.truth, digits),
                precision: sc.precision,
                paradigm: sc.paradigm,
            }
        })
        .collect();
    let depth = technique_depths();
    let mut known: Vec<u16> = s.known.iter().collect();
    known.sort_by(|x, y| depth[*y as usize].cmp(&depth[*x as usize]).then(x.cmp(y)));
    let techniques = known.iter().map(|&t| (cat.get(t).name.clone(), cat.get(t).reference.clone())).collect();
    let relations = state
        .relations
        .iter()
        .filter(|r| r.a == s.id || r.b == s.id)
        .filter_map(|r| {
            let other = if r.a == s.id { r.b } else { r.a };
            Some((other, name_of(other)?, relation_name(r, s.id, lang).to_string()))
        })
        .collect();
    let mut cities: Vec<_> = state.cities.iter().filter(|c| c.society == s.id).collect();
    cities.sort_by(|x, y| y.pop.total_cmp(&x.pop).then(x.id.cmp(&y.id)));
    let cities = cities.iter().take(8).map(|c| (c.cell, c.name.clone(), number_in(lang, c.pop.round(), 0))).collect();
    let notables =
        state.notables.iter().filter(|n| n.society == s.id).take(12).map(|n| (n.id, n.name.clone(), n.kind.label().to_string())).collect();
    Some(SocietySheet {
        id: s.id,
        name: s.name.clone(),
        label,
        peak: label_name(s.peak, lang).into(),
        summary,
        colour: colour(s.id),
        parent: s.parent.and_then(|p| Some((p, name_of(p)?))),
        founded: year_text(s.founded, lang),
        capital_cell: s.capital,
        capital: capital.unwrap_or_default(),
        people: s.people,
        facts,
        dims,
        institutions,
        sciences,
        techniques,
        technique_count: s.known.count(),
        relations,
        cities,
        notables,
    })
}

/// Une ligne de la comparaison de deux sociétés.
#[derive(Clone, Debug, PartialEq)]
pub struct CompareRow {
    pub name: String,
    pub a: String,
    pub b: String,
    /// La différence vaut d'être surlignée.
    pub differs: bool,
}

/// Comparaison de deux sociétés, trait par trait.
pub fn compare(state: &SocietyState, a: u32, b: u32, lang: Lang) -> Option<Vec<CompareRow>> {
    let (sa, sb) = (sheet(state, a, lang)?, sheet(state, b, lang)?);
    let (xa, xb) = (state.societies.iter().find(|s| s.id == a)?, state.societies.iter().find(|s| s.id == b)?);
    let mut rows = vec![CompareRow {
        name: if lang == Lang::Fr { "Forme".into() } else { "Form".into() },
        a: sa.label.clone(),
        b: sb.label.clone(),
        differs: xa.label != xb.label,
    }];
    for ((n, va), (_, vb)) in sa.facts.iter().zip(&sb.facts) {
        rows.push(CompareRow { name: n.clone(), a: va.clone(), b: vb.clone(), differs: va != vb });
    }
    let gauge = |rows: &mut Vec<CompareRow>, ga: &[Gauge], gb: &[Gauge]| {
        for (x, y) in ga.iter().zip(gb) {
            rows.push(CompareRow {
                name: x.name.clone(),
                a: percent(x.value, lang),
                b: percent(y.value, lang),
                differs: (x.value - y.value).abs() > 0.15,
            });
        }
    };
    gauge(&mut rows, &sa.dims, &sb.dims);
    gauge(&mut rows, &sa.institutions, &sb.institutions);
    for (x, y) in sa.sciences.iter().zip(&sb.sciences) {
        rows.push(CompareRow {
            name: format!("{} ({})", x.domain, x.quantity),
            a: x.estimate.clone(),
            b: y.estimate.clone(),
            differs: (xa.sciences.iter().find(|s| domain_name(s.domain, lang) == x.domain).map_or(0.0, |s| s.precision)
                - xb.sciences.iter().find(|s| domain_name(s.domain, lang) == y.domain).map_or(0.0, |s| s.precision))
            .abs()
                > 0.1,
        });
    }
    let only = |k: &evo_society::catalog::Known, other: &evo_society::catalog::Known| k.and_not(other).count() as usize;
    rows.push(CompareRow {
        name: if lang == Lang::Fr { "Techniques connues".into() } else { "Known techniques".into() },
        a: format!(
            "{} ({} {})",
            sa.technique_count,
            only(&xa.known, &xb.known),
            if lang == Lang::Fr { "à elle seule" } else { "of its own" }
        ),
        b: format!(
            "{} ({} {})",
            sb.technique_count,
            only(&xb.known, &xa.known),
            if lang == Lang::Fr { "à elle seule" } else { "of its own" }
        ),
        differs: xa.known != xb.known,
    });
    Some(rows)
}

/// Une ligne de la chronique des sociétés.
pub fn event_line(e: &Event, lang: Lang) -> String {
    format!("{} — {}", year_text(e.year, lang), e.text)
}

/// Grandes étapes d'une période, pour le récit.
fn milestone(k: EventKind) -> bool {
    matches!(
        k,
        EventKind::AgricultureHearth
            | EventKind::Label
            | EventKind::Collapse
            | EventKind::Conquest
            | EventKind::NewReligion
            | EventKind::Paradigm
            | EventKind::FirstContact
            | EventKind::IndustrialTakeoff
            | EventKind::EndOfFirstVersion
            | EventKind::NewDisease
    )
}

/// Le récit d'une période, écrit comme un chapitre d'histoire : une
/// ouverture, les grands moments dans l'ordre (les plus intéressants seuls
/// quand il y en a trop), le bilan de la période.
pub fn story(history: &History, from: i64, to: i64, lang: Lang) -> Vec<String> {
    let fr = lang == Lang::Fr;
    let span: Vec<&Event> = history.events.iter().filter(|e| e.year >= from && e.year <= to).collect();
    let mut out = Vec::new();
    if span.is_empty() {
        out.push(if fr {
            "Rien de marquant n'a laissé de trace pendant cette période.".into()
        } else {
            "Nothing of note left a trace in this period.".into()
        });
        return out;
    }
    let inventions = span.iter().filter(|e| e.kind == EventKind::Invention).count();
    let wars = span.iter().filter(|e| e.kind == EventKind::WarStart).count();
    let cities = span.iter().filter(|e| e.kind == EventKind::CityFounded).count();
    let epidemics = span.iter().filter(|e| e.kind == EventKind::Epidemic).count();
    let (a, b) = (year_text(from, lang), year_text(to, lang));
    let n = |k: usize| number_in(lang, k as f64, 0);
    let (ni, nc, nw, ne) = (n(inventions), n(cities), n(wars), n(epidemics));
    out.push(if fr {
        format!(
            "De l'{a} à l'{b}, {ni} invention{} ont vu le jour, {nc} ville{} ont été fondée{}, {nw} guerre{} ont éclaté et {ne} épidémie{} ont frappé.",
            if inventions > 1 { "s" } else { "" },
            if cities > 1 { "s" } else { "" },
            if cities > 1 { "s" } else { "" },
            if wars > 1 { "s" } else { "" },
            if epidemics > 1 { "s" } else { "" }
        )
    } else {
        format!("From {a} to {b}, {ni} inventions appeared, {nc} cities were founded, {nw} wars broke out and {ne} epidemics struck.")
    });
    let mut big: Vec<&Event> = span.iter().copied().filter(|e| milestone(e.kind)).collect();
    if big.len() > 14 {
        let mut by_interest = big.clone();
        by_interest.sort_by(|x, y| y.interest.total_cmp(&x.interest).then(x.year.cmp(&y.year)));
        let keep: BTreeSet<*const Event> = by_interest.iter().take(14).map(|e| *e as *const Event).collect();
        big.retain(|e| keep.contains(&(*e as *const Event)));
    }
    // Quelques inventions phares si la période n'a pas d'autre grand moment.
    if big.len() < 4 {
        let mut inv: Vec<&Event> = span.iter().copied().filter(|e| e.kind == EventKind::Invention).collect();
        inv.sort_by(|x, y| y.interest.total_cmp(&x.interest).then(x.year.cmp(&y.year)));
        big.extend(inv.into_iter().take(4));
        big.sort_by_key(|e| e.year);
    }
    for e in big {
        let t = e.text.trim_end_matches('.');
        out.push(if fr {
            format!("En {}, {}.", year_text(e.year, lang).trim_start_matches("an "), lower_first(t))
        } else {
            format!("In {}, {}.", year_text(e.year, lang).trim_start_matches("year "), lower_first(t))
        });
    }
    if let (Some(&first), Some(&last)) = (history.years.iter().find(|&&y| y >= from), history.years.iter().rev().find(|&&y| y <= to)) {
        let pop_at = |y: i64| -> f64 { history.samples.values().filter_map(|v| v.iter().find(|s| s.year == y)).map(|s| s.pop).sum() };
        let (p0, p1) = (pop_at(first), pop_at(last));
        if p0 > 0.0 && first != last {
            out.push(if fr {
                format!("La population est passée de {} à {} âmes.", number_in(lang, p0.round(), 0), number_in(lang, p1.round(), 0))
            } else {
                format!("Population went from {} to {} souls.", number_in(lang, p0.round(), 0), number_in(lang, p1.round(), 0))
            });
        }
    }
    out
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        // Un nom propre en tête garde sa majuscule : on ne baisse que les
        // mots courants.
        Some(f)
            if s.split_whitespace().next().is_some_and(|w| {
                [
                    "La",
                    "Le",
                    "Les",
                    "L'",
                    "Une",
                    "Un",
                    "Des",
                    "Premier",
                    "Première",
                    "Paix",
                    "Crise",
                    "Chez",
                    "Alliance",
                    "Effondrement",
                    "The",
                    "A",
                    "An",
                ]
                .iter()
                .any(|x| w == *x || (x.ends_with('\'') && w.starts_with(x)))
            }) =>
        {
            f.to_lowercase().collect::<String>() + c.as_str()
        }
        Some(f) => f.to_string() + c.as_str(),
        None => String::new(),
    }
}

/// Une entrée de l'encyclopédie des techniques.
#[derive(Clone, Debug, PartialEq)]
pub struct TechniqueEntry {
    pub id: u16,
    pub name: String,
    /// L'équivalent sur Terre.
    pub on_earth: String,
    pub requires: Vec<String>,
    pub depth: u32,
    /// Voie de remplacement (aquatique, pauvre en oxygène, souterraine, sans
    /// mains), ou vide.
    pub path: String,
}

/// Le catalogue des techniques, dans l'ordre de la chaîne des prérequis.
pub fn techniques(lang: Lang) -> Vec<TechniqueEntry> {
    let cat = catalog();
    let depth = technique_depths();
    let fr = lang == Lang::Fr;
    let mut out: Vec<TechniqueEntry> = cat
        .techniques
        .iter()
        .map(|t| {
            let path = match t.key.split('_').next() {
                Some("aq") if t.key.starts_with("aq_") => {
                    if fr {
                        "aquatique"
                    } else {
                        "aquatic"
                    }
                }
                Some("po") if t.key.starts_with("po_") => {
                    if fr {
                        "pauvre en oxygène"
                    } else {
                        "low oxygen"
                    }
                }
                Some("st") if t.key.starts_with("st_") => {
                    if fr {
                        "souterraine"
                    } else {
                        "underground"
                    }
                }
                Some("sm") if t.key.starts_with("sm_") => {
                    if fr {
                        "sans mains"
                    } else {
                        "handless"
                    }
                }
                _ => "",
            };
            TechniqueEntry {
                id: t.id,
                name: t.name.clone(),
                on_earth: t.reference.clone(),
                requires: t
                    .pre
                    .iter()
                    .map(|&p| cat.get(p).name.clone())
                    .chain(
                        t.any
                            .iter()
                            .map(|g| g.iter().map(|&p| cat.get(p).name.clone()).collect::<Vec<_>>().join(if fr { " ou " } else { " or " })),
                    )
                    .collect(),
                depth: depth[t.id as usize],
                path: path.into(),
            }
        })
        .collect();
    out.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.id.cmp(&b.id)));
    out
}

/// Bilan de la planète des sociétés : flux de l'année et cumulés.
pub fn planet_lines(state: &SocietyState, lang: Lang) -> Vec<(String, String)> {
    let p = &state.planet;
    let fr = lang == Lang::Fr;
    let t = |x: f64| format!("{} t C", crate::format::power_of_ten_in(lang, x));
    vec![
        ((if fr { "Forçage des sociétés" } else { "Forcing by societies" }).into(), format!("{} W/m²", number_in(lang, p.forcing_w_m2, 3))),
        ((if fr { "Pas du climat" } else { "Climate step" }).into(), format!("{} {}", p.climate_step_years, if fr { "ans" } else { "yr" })),
        ((if fr { "CO₂ émis (cumul)" } else { "CO₂ emitted (total)" }).into(), t(p.cum_co2_c)),
        ((if fr { "Méthane émis (cumul)" } else { "Methane emitted (total)" }).into(), t(p.cum_ch4_c)),
        ((if fr { "Déforestation (cumul)" } else { "Deforestation (total)" }).into(), t(p.cum_deforestation_c)),
        (
            (if fr { "Érosion (cumul)" } else { "Erosion (total)" }).into(),
            format!("{} t", crate::format::power_of_ten_in(lang, p.cum_erosion_t)),
        ),
    ]
}

/// Ligne de la liste des sociétés.
#[derive(Clone, Debug, PartialEq)]
pub struct ListRow {
    pub id: u32,
    pub name: String,
    pub label: String,
    pub pop: f64,
    pub cells: u32,
    pub techniques: u32,
    pub colour: [f32; 3],
    pub crisis: bool,
    pub war: bool,
}

pub fn list(state: &SocietyState, lang: Lang) -> Vec<ListRow> {
    let mut v: Vec<ListRow> = state
        .societies
        .iter()
        .map(|s| ListRow {
            id: s.id,
            name: s.name.clone(),
            label: label_name(s.label, lang).into(),
            pop: s.agg.population,
            cells: s.cells.len() as u32,
            techniques: s.known.count(),
            colour: colour(s.id),
            crisis: s.in_crisis,
            war: s.at_war,
        })
        .collect();
    v.sort_by(|a, b| b.pop.total_cmp(&a.pop).then(a.id.cmp(&b.id)));
    v
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    pub fn earthlike(grid: &GeodesicGrid) -> Vec<CellView> {
        (0..grid.len())
            .map(|i| {
                let lat = grid.latitude(i).to_degrees() as f32;
                let p = grid.centers[i];
                // Un continent sur un tiers du globe.
                let land = p[0] > 0.3;
                CellView {
                    is_ocean: !land,
                    temperature_k: 300.0 - 0.45 * lat.abs(),
                    rain_mm_yr: 300.0 + 900.0 * (1.0 - lat.abs() / 90.0),
                    elevation_m: if land { 400.0 + 2000.0 * (p[2].abs() as f32) } else { -3000.0 },
                    river_flow_m3s: if land { 300.0 } else { 0.0 },
                    ..Default::default()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::earthlike;
    use super::*;

    #[test]
    fn des_societes_d_essai_naissent_et_avancent() {
        let grid = GeodesicGrid::new(4);
        let cells = earthlike(&grid);
        let mut b = bench_world(&grid, &cells, None, BenchSpec { max_cells: 200, ..Default::default() }).expect("banc");
        assert!(b.world.cells.len() >= 100);
        let first = b.state();
        assert!(!first.societies.is_empty());
        assert!(first.cells.iter().all(|c| !cells[c.cell as usize].is_ocean));
        let mut h = History::default();
        for k in 0..300 {
            b.step();
            if k % 50 == 0 {
                let s = b.state();
                let ev = s.events.clone();
                h.record(&s, &ev);
            }
        }
        assert_eq!(b.world.year, BenchSpec::default().start_year + 300);
        assert!(!h.samples.is_empty());
    }

    #[test]
    fn le_banc_est_rejouable() {
        let grid = GeodesicGrid::new(4);
        let cells = earthlike(&grid);
        let run = || {
            let mut b = bench_world(&grid, &cells, None, BenchSpec { max_cells: 120, ..Default::default() }).unwrap();
            for _ in 0..100 {
                b.step();
            }
            b.state().fingerprint
        };
        assert_eq!(run(), run());
    }
}

#[cfg(test)]
mod speed {
    use super::*;

    /// Vitesse du banc sur 700 terres (`cargo test --release -p evo-view society_speed -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn society_speed() {
        let grid = GeodesicGrid::new(5);
        let cells = tests_support::earthlike(&grid);
        let mut b = bench_world(&grid, &cells, None, BenchSpec::default()).unwrap();
        let t = std::time::Instant::now();
        for k in 0..14_000 {
            b.step();
            if k % 1000 == 999 {
                let s = b.state();
                println!(
                    "{} : {} sociétés, {} villes, {} hab., {:.0} a/s",
                    s.year,
                    s.societies.len(),
                    s.cities.len(),
                    s.societies.iter().map(|x| x.agg.population).sum::<f64>().round(),
                    (k + 1) as f64 / t.elapsed().as_secs_f64()
                );
            }
        }
    }
}
