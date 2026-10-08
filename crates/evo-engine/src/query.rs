//! Requêtes du client et mémoire longue du moteur.
//!
//! Le journal des événements et les historiques régionaux et par espèce sont
//! rangés dans une base SQLite tenue par le fil de simulation. Les requêtes
//! passent par la même file que les ordres : le fil y répond entre deux pas,
//! sur l'état exact de la fin du dernier pas. Une réponse arrive donc au plus
//! un pas après la demande.

use evo_core::events::Event;
use evo_planet::{BioGrid, WaterPool};
use evo_sim::history::{EventView, Sample};
use evo_sim::World;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::Path;

/// Niveau de la grille des régions de l'historique régional (642 régions
/// de quelque 800 000 km² sur une planète de la taille de la Terre).
pub const REGION_LEVEL: u32 = 3;

/// Ce que le client peut demander.
#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    /// Événements d'identifiant supérieur à `since_id` (0 : depuis le début),
    /// d'intérêt au moins `min_interest`, dans l'ordre, au plus `limit`.
    Events { since_id: u64, min_interest: f32, limit: usize },
    /// Historique des grandeurs globales depuis une date.
    GlobalHistory { from_years: f64 },
    /// Historique de la région qui contient une cellule physique.
    RegionalHistory { cell: u32 },
    /// Historique d'une espèce (biomasse, aire, cellule d'abondance).
    SpeciesHistory { species: u32 },
    /// Milieu type d'une espèce (décor de sa planche, document DA).
    SpeciesHabitat { species: u32 },
    /// Fiche d'une espèce vivante.
    Species { species: u32 },
}

/// Un échantillon de l'historique d'une région.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RegionSample {
    pub years: f64,
    pub region: u32,
    /// Température moyenne, K.
    pub temperature_k: f64,
    /// Part de l'aire couverte par la mer.
    pub ocean_fraction: f64,
    /// O₂ dissous moyen des eaux de la région, mol·m⁻³.
    pub oxygen: f64,
    /// Biomasse, mol de carbone.
    pub biomass: f64,
    /// Espèce dominante (0 sans vie) et nombre d'espèces présentes.
    pub dominant_species: u32,
    pub species_count: u32,
}

/// Un échantillon de l'historique d'une espèce.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpeciesSample {
    pub years: f64,
    pub biomass: f64,
    /// Cellules du vivant occupées.
    pub cells: u32,
    pub ecotypes: u32,
    /// Cellule du vivant de plus grande abondance.
    pub peak_bio_cell: u32,
}

/// Milieu type d'une espèce : de quoi peindre le décor de sa planche.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Habitat {
    pub species: u32,
    pub name: String,
    /// Vrai si l'espèce vit encore ; sinon le milieu est celui de son apogée.
    pub living: bool,
    /// Date à laquelle le milieu est décrit, années.
    pub years: f64,
    /// Cellule du vivant de plus grande abondance.
    pub peak_bio_cell: u32,
    /// Conditions moyennes sur l'aire de l'espèce, pondérées par sa biomasse.
    pub elevation_m: f64,
    pub temperature_k: f64,
    pub seasonal_amplitude_k: f64,
    pub light_w_m2: f64,
    pub ph: f64,
    pub salinity: f64,
    pub oxygen: f64,
    /// Part de la biomasse en mer, en eaux douces, près des sources.
    pub sea_share: f64,
    pub fresh_share: f64,
    pub vent_share: f64,
    /// Latitude moyenne (valeur absolue), radians.
    pub abs_latitude_rad: f64,
    /// Température de l'étoile, K (couleur de la lumière du décor).
    pub star_temperature_k: f64,
    /// Température moyenne et pression de la planète.
    pub planet_temperature_k: f64,
    pub pressure_pa: f64,
    /// Les trois espèces qui partagent le plus son milieu : (espèce, nom,
    /// part de la biomasse voisine).
    pub companions: Vec<(u32, String, f64)>,
    /// Pour une espèce éteinte : la région de son apogée à cette date.
    pub region_at_peak: Option<RegionSample>,
}

/// Réponse à une requête.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Events(Vec<EventView>),
    GlobalHistory(Vec<Sample>),
    RegionalHistory(Vec<RegionSample>),
    SpeciesHistory(Vec<SpeciesSample>),
    Habitat(Option<Habitat>),
    Species(Option<evo_sim::SpeciesView>),
    /// La requête n'a pas pu aboutir (base illisible, …).
    Failed(String),
}

/// Base SQLite du moteur.
pub(crate) struct Store {
    db: Connection,
    /// Événements du journal du monde déjà rangés.
    stored_events: usize,
    /// Échantillons de l'historique global déjà traités.
    stored_samples: usize,
    /// Région de chaque cellule du vivant.
    region_of: Vec<u32>,
    regions: usize,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, years REAL NOT NULL, interest REAL NOT NULL, data BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS regional (region INTEGER NOT NULL, years REAL NOT NULL, temperature_k REAL, ocean REAL, oxygen REAL, biomass REAL, dominant INTEGER, species INTEGER, PRIMARY KEY (region, years));
CREATE TABLE IF NOT EXISTS species (species INTEGER NOT NULL, years REAL NOT NULL, biomass REAL, cells INTEGER, ecotypes INTEGER, peak INTEGER, PRIMARY KEY (species, years));
";

fn sql_err(e: rusqlite::Error) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

impl Store {
    /// Ouvre (ou crée) la base au chemin donné et la met à jour d'après le
    /// monde : un monde repris d'une sauvegarde sans base retrouve au moins
    /// son journal d'événements.
    pub(crate) fn open(path: &Path, world: &World) -> std::io::Result<Store> {
        let db = Connection::open(path).map_err(sql_err)?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = OFF;").map_err(sql_err)?;
        db.execute_batch(SCHEMA).map_err(sql_err)?;
        let regions = BioGrid::new(&world.bio.grid, REGION_LEVEL.min(world.bio.grid.level));
        let stored_events: i64 = db.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).map_err(sql_err)?;
        let last: Option<f64> = db.query_row("SELECT MAX(years) FROM species", [], |r| r.get(0)).map_err(sql_err)?;
        let stored_samples = match last {
            Some(y) => world.history.samples.iter().filter(|s| s.years <= y).count(),
            None => 0,
        };
        let mut store = Store {
            db,
            stored_events: (stored_events as usize).min(world.events.events.len()),
            stored_samples,
            regions: regions.len(),
            region_of: regions.parent,
        };
        store.record(world).map_err(sql_err)?;
        Ok(store)
    }

    /// Range ce qui est nouveau depuis le dernier appel : événements, et
    /// historiques régional et par espèce quand un échantillon global est
    /// tombé.
    pub(crate) fn record(&mut self, world: &World) -> rusqlite::Result<()> {
        let tx = self.db.transaction()?;
        {
            let mut ins = tx.prepare_cached("INSERT OR REPLACE INTO events (id, years, interest, data) VALUES (?1, ?2, ?3, ?4)")?;
            for e in &world.events.events[self.stored_events..] {
                let blob = bincode::serialize(e).expect("événement sérialisable");
                ins.execute(params![e.id as i64, e.years, e.interest, blob])?;
            }
        }
        self.stored_events = world.events.events.len();
        let samples = world.history.samples.len();
        if samples > self.stored_samples {
            let years = world.history.samples[samples - 1].years;
            let mut reg = tx.prepare_cached(
                "INSERT OR REPLACE INTO regional (region, years, temperature_k, ocean, oxygen, biomass, dominant, species) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for r in regional_samples(world, &self.region_of, self.regions, years) {
                reg.execute(params![
                    r.region,
                    r.years,
                    r.temperature_k,
                    r.ocean_fraction,
                    r.oxygen,
                    r.biomass,
                    r.dominant_species,
                    r.species_count
                ])?;
            }
            let mut sp = tx.prepare_cached(
                "INSERT OR REPLACE INTO species (species, years, biomass, cells, ecotypes, peak) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for s in world.species() {
                sp.execute(params![s.id, years, s.biomass, s.cells, s.ecotypes, s.peak_bio_cell])?;
            }
            self.stored_samples = samples;
        }
        drop(tx.commit());
        Ok(())
    }

    /// Copie cohérente de la base (point de sauvegarde).
    pub(crate) fn backup_to(&self, path: &Path) -> std::io::Result<()> {
        let _ = std::fs::remove_file(path);
        self.db.execute("VACUUM INTO ?1", params![path.to_string_lossy()]).map_err(sql_err)?;
        Ok(())
    }

    pub(crate) fn answer(&self, world: &World, q: &Query) -> Answer {
        match self.try_answer(world, q) {
            Ok(a) => a,
            Err(e) => Answer::Failed(e.to_string()),
        }
    }

    fn try_answer(&self, world: &World, q: &Query) -> rusqlite::Result<Answer> {
        Ok(match *q {
            Query::Events { since_id, min_interest, limit } => {
                let mut st = self.db.prepare_cached("SELECT data FROM events WHERE id > ?1 AND interest >= ?2 ORDER BY id LIMIT ?3")?;
                let rows = st.query_map(params![since_id as i64, min_interest as f64, limit.min(i64::MAX as usize) as i64], |r| {
                    r.get::<_, Vec<u8>>(0)
                })?;
                let mut out = Vec::new();
                for blob in rows {
                    if let Ok(e) = bincode::deserialize::<Event>(&blob?) {
                        out.push(EventView::from_event(&e));
                    }
                }
                Answer::Events(out)
            }
            Query::GlobalHistory { from_years } => {
                Answer::GlobalHistory(world.history.samples.iter().filter(|s| s.years >= from_years).copied().collect())
            }
            Query::RegionalHistory { cell } => {
                let Some(region) = self.region_of_physical(world, cell) else { return Ok(Answer::RegionalHistory(Vec::new())) };
                Answer::RegionalHistory(self.regional(region, None)?)
            }
            Query::SpeciesHistory { species } => Answer::SpeciesHistory(self.species_history(species)?),
            Query::SpeciesHabitat { species } => Answer::Habitat(self.habitat(world, species)?),
            Query::Species { species } => Answer::Species(world.species().into_iter().find(|s| s.id == species)),
        })
    }

    fn region_of_physical(&self, world: &World, cell: u32) -> Option<u32> {
        let bio = *world.bio.parent.get(cell as usize)?;
        self.region_of.get(bio as usize).copied()
    }

    fn regional(&self, region: u32, at: Option<f64>) -> rusqlite::Result<Vec<RegionSample>> {
        let map = |r: &rusqlite::Row| -> rusqlite::Result<RegionSample> {
            Ok(RegionSample {
                region: r.get(0)?,
                years: r.get(1)?,
                temperature_k: r.get(2)?,
                ocean_fraction: r.get(3)?,
                oxygen: r.get(4)?,
                biomass: r.get(5)?,
                dominant_species: r.get(6)?,
                species_count: r.get(7)?,
            })
        };
        let sql = "SELECT region, years, temperature_k, ocean, oxygen, biomass, dominant, species FROM regional WHERE region = ?1";
        match at {
            None => {
                let mut st = self.db.prepare_cached(&format!("{sql} ORDER BY years"))?;
                let rows = st.query_map(params![region], map)?;
                rows.collect()
            }
            Some(y) => {
                let mut st = self.db.prepare_cached(&format!("{sql} AND years = ?2"))?;
                Ok(st.query_row(params![region, y], map).optional()?.into_iter().collect())
            }
        }
    }

    fn species_history(&self, species: u32) -> rusqlite::Result<Vec<SpeciesSample>> {
        let mut st =
            self.db.prepare_cached("SELECT years, biomass, cells, ecotypes, peak FROM species WHERE species = ?1 ORDER BY years")?;
        let rows = st.query_map(params![species], |r| {
            Ok(SpeciesSample { years: r.get(0)?, biomass: r.get(1)?, cells: r.get(2)?, ecotypes: r.get(3)?, peak_bio_cell: r.get(4)? })
        })?;
        rows.collect()
    }

    fn habitat(&self, world: &World, species: u32) -> rusqlite::Result<Option<Habitat>> {
        if let Some(h) = living_habitat(world, species) {
            return Ok(Some(h));
        }
        // Espèce éteinte : la région de son apogée, à la date de l'apogée.
        let history = self.species_history(species)?;
        let Some(peak) = history.iter().max_by(|a, b| a.biomass.total_cmp(&b.biomass)) else { return Ok(None) };
        let region = self.region_of.get(peak.peak_bio_cell as usize).copied().unwrap_or(0);
        let region_at_peak = self.regional(region, Some(peak.years))?.into_iter().next();
        let p = &world.planet;
        Ok(Some(Habitat {
            species,
            name: evo_life::metabolism::guild_label(species),
            living: false,
            years: peak.years,
            peak_bio_cell: peak.peak_bio_cell,
            temperature_k: region_at_peak.map_or(0.0, |r| r.temperature_k),
            oxygen: region_at_peak.map_or(0.0, |r| r.oxygen),
            sea_share: region_at_peak.map_or(0.0, |r| r.ocean_fraction),
            star_temperature_k: p.params.star_temperature_k,
            planet_temperature_k: p.climate.mean_temperature_k,
            pressure_pa: p.reservoirs.pressure_pa(p.params.gravity(), p.params.surface_area()),
            region_at_peak,
            ..Default::default()
        }))
    }
}

/// Un échantillon par région, à la date donnée.
fn regional_samples(world: &World, region_of: &[u32], regions: usize, years: f64) -> Vec<RegionSample> {
    #[derive(Default, Clone)]
    struct Acc {
        area: f64,
        temp: f64,
        ocean: f64,
        water: f64,
        oxygen: f64,
        biomass: f64,
        by_species: BTreeMap<u32, f64>,
    }
    let mut acc = vec![Acc::default(); regions];
    for (b, env) in world.bio.env.iter().enumerate() {
        let a = &mut acc[region_of[b] as usize];
        a.area += env.area_m2;
        a.temp += env.temperature_k * env.area_m2;
        if env.is_ocean {
            a.ocean += env.area_m2;
        }
        a.water += env.water_volume_m3;
        a.oxygen += world.chemistry[b][WaterPool::O2 as usize] * env.water_volume_m3;
        for p in &world.communities[b] {
            a.biomass += p.biomass;
            *a.by_species.entry(p.signature()).or_default() += p.biomass;
        }
    }
    acc.into_iter()
        .enumerate()
        .map(|(r, a)| RegionSample {
            years,
            region: r as u32,
            temperature_k: a.temp / a.area.max(1.0),
            ocean_fraction: a.ocean / a.area.max(1.0),
            oxygen: if a.water > 0.0 { a.oxygen / a.water } else { 0.0 },
            biomass: a.biomass,
            dominant_species: a.by_species.iter().max_by(|x, y| x.1.total_cmp(y.1).then(y.0.cmp(x.0))).map_or(0, |(s, _)| *s),
            species_count: a.by_species.len() as u32,
        })
        .collect()
}

/// Milieu type d'une espèce vivante, ou `None` si elle n'a plus de
/// population.
pub fn living_habitat(world: &World, species: u32) -> Option<Habitat> {
    let mut h =
        Habitat { species, name: evo_life::metabolism::guild_label(species), living: true, years: world.years(), ..Default::default() };
    let mut total = 0.0;
    let mut peak = 0.0;
    let mut neighbours: BTreeMap<u32, f64> = BTreeMap::new();
    for (b, pops) in world.communities.iter().enumerate() {
        let mine: f64 = pops.iter().filter(|p| p.signature() == species).map(|p| p.biomass).sum();
        if mine <= 0.0 {
            continue;
        }
        let env = &world.bio.env[b];
        total += mine;
        if mine > peak {
            peak = mine;
            h.peak_bio_cell = b as u32;
        }
        h.elevation_m += mine * env.elevation_m;
        h.temperature_k += mine * env.temperature_k;
        h.seasonal_amplitude_k += mine * env.seasonal_amplitude_k;
        h.light_w_m2 += mine * env.light_par_w_m2;
        h.ph += mine * env.ph;
        h.salinity += mine * env.salinity;
        h.oxygen += mine * world.chemistry[b][WaterPool::O2 as usize];
        h.abs_latitude_rad += mine * env.latitude_rad.abs();
        if env.is_ocean {
            h.sea_share += mine;
        } else {
            h.fresh_share += mine;
        }
        if env.vent_h2_supply + env.vent_h2s_supply + env.vent_fe_supply > 0.0 {
            h.vent_share += mine;
        }
        for p in pops.iter().filter(|p| p.signature() != species) {
            *neighbours.entry(p.signature()).or_default() += p.biomass;
        }
    }
    if total <= 0.0 {
        return None;
    }
    for v in [
        &mut h.elevation_m,
        &mut h.temperature_k,
        &mut h.seasonal_amplitude_k,
        &mut h.light_w_m2,
        &mut h.ph,
        &mut h.salinity,
        &mut h.oxygen,
        &mut h.abs_latitude_rad,
        &mut h.sea_share,
        &mut h.fresh_share,
        &mut h.vent_share,
    ] {
        *v /= total;
    }
    let others: f64 = neighbours.values().sum();
    let mut list: Vec<(u32, f64)> = neighbours.into_iter().collect();
    list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    h.companions = list
        .into_iter()
        .take(3)
        .map(|(s, m)| (s, evo_life::metabolism::guild_label(s), if others > 0.0 { m / others } else { 0.0 }))
        .collect();
    let p = &world.planet;
    h.star_temperature_k = p.params.star_temperature_k;
    h.planet_temperature_k = p.climate.mean_temperature_k;
    h.pressure_pa = p.reservoirs.pressure_pa(p.params.gravity(), p.params.surface_area());
    Some(h)
}
