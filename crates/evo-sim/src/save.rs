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
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::Arc;

/// Signature des fichiers de sauvegarde.
pub const MAGIC: &[u8; 9] = b"EVONISIUM";
/// Version du format ; une sauvegarde d'une autre version est refusée.
pub const FORMAT_VERSION: u32 = 3;

#[derive(Serialize, Deserialize)]
struct SavedPopulation {
    lineage: u32,
    genome: u32,
    biomass: f64,
    rates: GrowthRates,
}

// Les gros morceaux sont empruntés à l'écriture (pas de copie de l'état) et
// possédés à la lecture : `Cow` s'écrit comme la valeur qu'il porte.
#[derive(Serialize, Deserialize)]
struct SavedPlanet<'a> {
    params: Cow<'a, PlanetParams>,
    cells: Cow<'a, [CellEnvironment]>,
    tectonics: Cow<'a, Tectonics>,
    climate: Cow<'a, ClimateState>,
    reservoirs: Cow<'a, GlobalReservoirs>,
    sea_level_m: f64,
    deep_volume_m3: f64,
    tectonic_clock: f64,
    hydrothermal_share: f64,
    display: Cow<'a, [CellDisplay]>,
}

#[derive(Serialize, Deserialize)]
struct SaveState<'a> {
    // En tête : ce que l'écran des sauvegardes lit sans charger la partie.
    engine_version: String,
    years: f64,
    config: Cow<'a, WorldConfig>,
    planet: SavedPlanet<'a>,
    chemistry: Cow<'a, [WaterChemistry]>,
    genomes: Vec<Genome>,
    communities: Vec<Vec<SavedPopulation>>,
    /// Registre des lignées sans les génomes fondateurs, rangés dans la
    /// table des génomes (`founders`, un indice par lignée).
    lineages: LineageRegistry,
    founders: Vec<Option<u32>>,
    events: Cow<'a, EventLog>,
    journal: Cow<'a, GenomeJournal>,
    flux: Cow<'a, FluxRegistry>,
    orders: Cow<'a, OrderQueue>,
    history: Cow<'a, History>,
    progress: Cow<'a, Progress>,
    paused: bool,
    marked: Cow<'a, [u32]>,
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
        let mut index = |g: &Arc<Genome>, genomes: &mut Vec<Genome>| -> u32 {
            *table.entry(Arc::as_ptr(g) as usize).or_insert_with(|| {
                genomes.push((**g).clone());
                (genomes.len() - 1) as u32
            })
        };
        let communities = self
            .communities
            .iter()
            .map(|pops| {
                pops.iter()
                    .map(|p| {
                        let genome = index(&p.genome, &mut genomes);
                        SavedPopulation { lineage: p.lineage, genome, biomass: p.biomass, rates: p.rates }
                    })
                    .collect()
            })
            .collect();
        let founders = self.lineages.records.iter().map(|r| r.founder.as_ref().map(|g| index(g, &mut genomes))).collect();
        let mut lineages = self.lineages.clone();
        for r in &mut lineages.records {
            r.founder = None;
        }
        let pl = &self.planet;
        let state = SaveState {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            years: self.years,
            config: Cow::Borrowed(&self.config),
            planet: SavedPlanet {
                params: Cow::Borrowed(&pl.params),
                cells: Cow::Borrowed(&pl.cells),
                tectonics: Cow::Borrowed(&pl.tectonics),
                climate: Cow::Borrowed(&pl.climate),
                reservoirs: Cow::Borrowed(&pl.reservoirs),
                sea_level_m: pl.sea_level_m,
                deep_volume_m3: pl.deep_volume_m3,
                tectonic_clock: pl.tectonic_clock,
                hydrothermal_share: pl.hydrothermal_share,
                display: Cow::Borrowed(&pl.display),
            },
            chemistry: Cow::Borrowed(&self.chemistry),
            genomes,
            communities,
            lineages,
            founders,
            events: Cow::Borrowed(&self.events),
            journal: Cow::Borrowed(&self.journal),
            flux: Cow::Borrowed(&self.flux),
            orders: Cow::Borrowed(&self.orders),
            history: Cow::Borrowed(&self.history),
            progress: Cow::Borrowed(&self.progress),
            paused: self.paused,
            marked: Cow::Borrowed(&self.marked),
            oxygen_production: self.oxygen_production,
            previous_rates: self.previous_rates(),
            stats: self.stats,
            influence: self.influence,
            published_events: self.published_events(),
        };
        out.write_all(MAGIC)?;
        out.write_all(&FORMAT_VERSION.to_le_bytes())?;
        // bincode écrit champ par champ : sans tampon, chaque petit morceau
        // traverse le compresseur, vingt fois plus lentement.
        let z = flate2::write::DeflateEncoder::new(out, flate2::Compression::fast());
        let mut buf = io::BufWriter::with_capacity(1 << 20, z);
        bincode::serialize_into(&mut buf, &state).map_err(|e| invalid(e.to_string()))?;
        buf.into_inner().map_err(|e| e.into_error())?.finish()?;
        Ok(())
    }

    /// Reprend une partie depuis un point de sauvegarde.
    pub fn load_from(input: &mut impl Read) -> io::Result<World> {
        read_header(input)?;
        let z = io::BufReader::with_capacity(1 << 20, flate2::read::DeflateDecoder::new(input));
        let s: SaveState = bincode::deserialize_from(z).map_err(|e| invalid(e.to_string()))?;
        let cfg = s.config.into_owned();
        let grid = GeodesicGrid::new(cfg.level);
        let sp = s.planet;
        let planet = Planet {
            params: sp.params.into_owned(),
            grid,
            cells: sp.cells.into_owned(),
            tectonics: sp.tectonics.into_owned(),
            climate: sp.climate.into_owned(),
            reservoirs: sp.reservoirs.into_owned(),
            sea_level_m: sp.sea_level_m,
            deep_volume_m3: sp.deep_volume_m3,
            tectonic_clock: sp.tectonic_clock,
            hydrothermal_share: sp.hydrothermal_share,
            display: sp.display.into_owned(),
        };
        let mut bio = BioGrid::new(&planet.grid, cfg.bio_level);
        bio.aggregate(&planet.cells);
        // Phénotypes construits pour les seuls génomes portés par une
        // population (les fondateurs de lignées n'en ont pas besoin).
        let shared: Vec<Arc<Genome>> = s.genomes.into_iter().map(Arc::new).collect();
        let mut phenotypes: Vec<Option<Arc<Phenotype>>> = vec![None; shared.len()];
        let mut communities = Vec::with_capacity(s.communities.len());
        for pops in s.communities {
            let mut v = Vec::with_capacity(pops.len());
            for p in pops {
                let k = p.genome as usize;
                let g = shared.get(k).ok_or_else(|| invalid("génome absent de la table"))?;
                let ph = phenotypes[k].get_or_insert_with(|| Arc::new(Phenotype::from_genome(g, &cfg.physiology)));
                v.push(Population { lineage: p.lineage, genome: g.clone(), phenotype: ph.clone(), biomass: p.biomass, rates: p.rates });
            }
            communities.push(v);
        }
        if communities.len() != bio.len() || s.chemistry.len() != bio.len() {
            return Err(invalid("taille de la grille du vivant incohérente"));
        }
        let mut world = World::assemble(cfg, planet, bio, s.chemistry.into_owned(), communities);
        world.lineages = s.lineages;
        if s.founders.len() != world.lineages.records.len() {
            return Err(invalid("génomes fondateurs incohérents"));
        }
        for (r, f) in world.lineages.records.iter_mut().zip(s.founders) {
            r.founder = match f {
                Some(i) => Some(shared.get(i as usize).ok_or_else(|| invalid("génome absent de la table"))?.clone()),
                None => None,
            };
        }
        world.events = s.events.into_owned();
        world.journal = s.journal.into_owned();
        world.flux = s.flux.into_owned();
        world.orders = s.orders.into_owned();
        world.history = s.history.into_owned();
        world.progress = s.progress.into_owned();
        world.years = s.years;
        world.paused = s.paused;
        world.marked = s.marked.into_owned();
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
    #[derive(Deserialize)]
    struct Head {
        _engine_version: String,
        years: f64,
        config: WorldConfig,
    }
    read_header(input)?;
    let h: Head =
        bincode::deserialize_from(io::BufReader::new(flate2::read::DeflateDecoder::new(input))).map_err(|e| invalid(e.to_string()))?;
    Ok((h.config, h.years))
}

/// Lit et vérifie la signature et la version du format.
fn read_header(input: &mut impl Read) -> io::Result<()> {
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
    Ok(())
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
        let (peeked, years) = peek_config(&mut bytes.as_slice()).unwrap();
        assert_eq!((peeked.seed, years), (a.config.seed, a.years));
        for _ in 0..8 {
            a.step();
            b.step();
        }
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.events.events, b.events.events);
        assert!(World::load_from(&mut &b"pas une sauvegarde"[..]).is_err());
    }
}
