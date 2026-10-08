//! Points de sauvegarde (document Vision, « Interface dans les étapes »,
//! étape 3) : l'état complet du monde, écrit sur disque, d'où une partie
//! reprend exactement comme si elle n'avait pas été interrompue.
//!
//! Format : un en-tête lisible (`EVONISIUM`, version du format), puis l'état
//! sérialisé en binaire (bincode) et compressé (deflate). Les génomes sont
//! rangés une fois dans une table : les populations qui partageaient un
//! génome le partagent encore après la reprise, ce dont dépend l'évolution
//! par dème. Ce qui se recalcule exactement n'est pas écrit : grilles,
//! phénotypes, conditions des cellules du vivant, état publié.

use crate::history::{History, Publication};
use crate::influence::InfluenceReserve;
use crate::orders::OrderQueue;
use crate::world::{Progress, World, WorldConfig, WorldStats};
use evo_core::events::EventLog;
use evo_core::flux::FluxRegistry;
use evo_genetics::{Genome, GenomeJournal, LineageRegistry};
use evo_life::community::Population;
use evo_life::{GrowthRates, Phenotype};
use evo_planet::climate::ClimateState;
use evo_planet::hydrology::CellDisplay;
use evo_planet::tectonics::Tectonics;
use evo_planet::{BioGrid, CellEnvironment, GeodesicGrid, GlobalReservoirs, Planet, PlanetParams, WaterChemistry, WATER_POOL_COUNT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::Arc;

/// Signature des fichiers de sauvegarde.
pub const MAGIC: &[u8; 9] = b"EVONISIUM";
/// Version du format ; une sauvegarde d'une autre version est refusée.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct SavedPopulation {
    lineage: u32,
    genome: u32,
    biomass: f64,
    rates: GrowthRates,
}

#[derive(Serialize, Deserialize)]
struct SavedPlanet {
    params: PlanetParams,
    cells: Vec<CellEnvironment>,
    tectonics: Tectonics,
    climate: ClimateState,
    reservoirs: GlobalReservoirs,
    sea_level_m: f64,
    deep_volume_m3: f64,
    tectonic_clock: f64,
    hydrothermal_share: f64,
    display: Vec<CellDisplay>,
}

#[derive(Serialize, Deserialize)]
struct SaveState {
    engine_version: String,
    config: WorldConfig,
    planet: SavedPlanet,
    chemistry: Vec<WaterChemistry>,
    genomes: Vec<Genome>,
    communities: Vec<Vec<SavedPopulation>>,
    lineages: LineageRegistry,
    events: EventLog,
    journal: GenomeJournal,
    flux: FluxRegistry,
    orders: OrderQueue,
    history: History,
    progress: Progress,
    years: f64,
    paused: bool,
    marked: Vec<u32>,
    oxygen_production: f64,
    previous_rates: Option<[f64; WATER_POOL_COUNT]>,
    stats: WorldStats,
    influence: InfluenceReserve,
    published_events: usize,
}

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

impl World {
    /// Écrit un point de sauvegarde.
    pub fn save_to(&self, out: &mut impl Write) -> io::Result<()> {
        let mut table: HashMap<usize, u32> = HashMap::new();
        let mut genomes = Vec::new();
        let communities = self
            .communities
            .iter()
            .map(|pops| {
                pops.iter()
                    .map(|p| {
                        let key = Arc::as_ptr(&p.genome) as usize;
                        let genome = *table.entry(key).or_insert_with(|| {
                            genomes.push((*p.genome).clone());
                            (genomes.len() - 1) as u32
                        });
                        SavedPopulation { lineage: p.lineage, genome, biomass: p.biomass, rates: p.rates }
                    })
                    .collect()
            })
            .collect();
        let pl = &self.planet;
        let state = SaveState {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            config: self.config.clone(),
            planet: SavedPlanet {
                params: pl.params.clone(),
                cells: pl.cells.clone(),
                tectonics: pl.tectonics.clone(),
                climate: pl.climate.clone(),
                reservoirs: pl.reservoirs.clone(),
                sea_level_m: pl.sea_level_m,
                deep_volume_m3: pl.deep_volume_m3,
                tectonic_clock: pl.tectonic_clock,
                hydrothermal_share: pl.hydrothermal_share,
                display: pl.display.clone(),
            },
            chemistry: self.chemistry.clone(),
            genomes,
            communities,
            lineages: self.lineages.clone(),
            events: self.events.clone(),
            journal: self.journal.clone(),
            flux: self.flux.clone(),
            orders: self.orders.clone(),
            history: self.history.clone(),
            progress: self.progress.clone(),
            years: self.years,
            paused: self.paused,
            marked: self.marked.clone(),
            oxygen_production: self.oxygen_production,
            previous_rates: self.previous_rates(),
            stats: self.stats,
            influence: self.influence,
            published_events: self.published_events(),
        };
        out.write_all(MAGIC)?;
        out.write_all(&FORMAT_VERSION.to_le_bytes())?;
        let mut z = flate2::write::DeflateEncoder::new(out, flate2::Compression::fast());
        bincode::serialize_into(&mut z, &state).map_err(|e| invalid(e.to_string()))?;
        z.finish()?;
        Ok(())
    }

    /// Reprend une partie depuis un point de sauvegarde.
    pub fn load_from(input: &mut impl Read) -> io::Result<World> {
        let mut magic = [0u8; 9];
        input.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid("ce fichier n'est pas une sauvegarde d'Evonisium"));
        }
        let mut v = [0u8; 4];
        input.read_exact(&mut v)?;
        let version = u32::from_le_bytes(v);
        if version != FORMAT_VERSION {
            return Err(invalid(format!("format de sauvegarde {version}, ce moteur lit le format {FORMAT_VERSION}")));
        }
        let z = flate2::read::DeflateDecoder::new(input);
        let s: SaveState = bincode::deserialize_from(z).map_err(|e| invalid(e.to_string()))?;
        let cfg = s.config;
        let grid = GeodesicGrid::new(cfg.level);
        let sp = s.planet;
        let planet = Planet {
            params: sp.params,
            grid,
            cells: sp.cells,
            tectonics: sp.tectonics,
            climate: sp.climate,
            reservoirs: sp.reservoirs,
            sea_level_m: sp.sea_level_m,
            deep_volume_m3: sp.deep_volume_m3,
            tectonic_clock: sp.tectonic_clock,
            hydrothermal_share: sp.hydrothermal_share,
            display: sp.display,
        };
        let mut bio = BioGrid::new(&planet.grid, cfg.bio_level);
        bio.aggregate(&planet.cells);
        let shared: Vec<(Arc<Genome>, Arc<Phenotype>)> = s
            .genomes
            .into_iter()
            .map(|g| {
                let p = Phenotype::from_genome(&g, &cfg.physiology);
                (Arc::new(g), Arc::new(p))
            })
            .collect();
        let mut communities = Vec::with_capacity(s.communities.len());
        for pops in s.communities {
            let mut v = Vec::with_capacity(pops.len());
            for p in pops {
                let (g, ph) = shared.get(p.genome as usize).ok_or_else(|| invalid("génome absent de la table"))?;
                v.push(Population { lineage: p.lineage, genome: g.clone(), phenotype: ph.clone(), biomass: p.biomass, rates: p.rates });
            }
            communities.push(v);
        }
        if communities.len() != bio.len() || s.chemistry.len() != bio.len() {
            return Err(invalid("taille de la grille du vivant incohérente"));
        }
        let mut world = World::assemble(cfg, planet, bio, s.chemistry, communities);
        world.lineages = s.lineages;
        world.events = s.events;
        world.journal = s.journal;
        world.flux = s.flux;
        world.orders = s.orders;
        world.history = s.history;
        world.progress = s.progress;
        world.years = s.years;
        world.paused = s.paused;
        world.marked = s.marked;
        world.oxygen_production = s.oxygen_production;
        world.set_previous_rates(s.previous_rates);
        world.stats = s.stats;
        world.influence = s.influence;
        world.set_published_events(s.published_events);
        world.publication = Publication::default();
        world.republish();
        Ok(world)
    }

    /// Écrit un point de sauvegarde dans un fichier.
    pub fn save_file(&self, path: &std::path::Path) -> io::Result<()> {
        // Écriture dans un fichier voisin puis renommage : un point de
        // sauvegarde n'est jamais à moitié écrit.
        let tmp = path.with_extension("tmp");
        {
            let mut f = io::BufWriter::new(std::fs::File::create(&tmp)?);
            self.save_to(&mut f)?;
            f.flush()?;
        }
        std::fs::rename(tmp, path)
    }

    pub fn load_file(path: &std::path::Path) -> io::Result<World> {
        let mut f = io::BufReader::new(std::fs::File::open(path)?);
        World::load_from(&mut f)
    }
}

/// Configuration minimale lue sans charger la partie (écran des sauvegardes).
pub fn peek_config(input: &mut impl Read) -> io::Result<(WorldConfig, f64)> {
    let w = World::load_from(input)?;
    Ok((w.config.clone(), w.years))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::{Intervention, OrderKind};

    #[test]
    fn resumed_game_continues_bit_for_bit() {
        let mut cfg = WorldConfig::new(21, 3);
        cfg.step_years = 40_000.0;
        let mut a = World::new(cfg);
        a.orders.submit(0.0, OrderKind::SeedLife);
        a.orders.submit(200_000.0, OrderKind::Intervene(Intervention::Fertilize { cell: 10, radius_km: 2000.0, moles_p: 1e13 }));
        for _ in 0..8 {
            a.step();
        }
        let mut bytes = Vec::new();
        a.save_to(&mut bytes).unwrap();
        let mut b = World::load_from(&mut bytes.as_slice()).unwrap();
        assert_eq!(a.state_hash(), b.state_hash());
        for _ in 0..8 {
            a.step();
            b.step();
        }
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.events.events, b.events.events);
        assert!(World::load_from(&mut &b"pas une sauvegarde"[..]).is_err());
    }
}
