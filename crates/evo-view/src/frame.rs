//! Ce que le client lit du moteur à chaque pas : une image figée du monde,
//! prise sur le fil de la simulation après chaque pas, puis partagée en
//! lecture seule (`Arc`) avec le fil de rendu.
//!
//! Elle reprend l'état publié de l'étape 2 (`PublishedState`) et y ajoute ce
//! qu'un affichage doit savoir et que l'état publié ne porte pas encore :
//! hauteur au-dessus de la mer, pôles des plaques, populations de chaque
//! cellule, sources hydrothermales. Quand le service moteur de l'étape 3
//! publiera ces données, `Frame::from_world` se réduira à une copie.

use evo_planet::grid::GeodesicGrid;
use evo_sim::{ClimateMode, Sample, World};
use std::sync::Arc;

/// Populations gardées par cellule pour l'inspecteur et les aires.
pub const POPULATIONS_PER_CELL: usize = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellFrame {
    /// Hauteur par rapport au niveau de la mer, m (négative sous la mer).
    pub height_m: f32,
    pub temperature_k: f32,
    pub is_ocean: bool,
    /// Couverture de glace, 0 à 1.
    pub ice_cover: f32,
    /// Biomasse vivante, mol de carbone.
    pub biomass: f32,
    /// O₂ dissous, mol·m⁻³.
    pub oxygen: f32,
    pub plate: u16,
    /// Guilde dominante (signature métabolique), 0 sans vie.
    pub dominant_guild: u32,
    /// Pic d'absorption du pigment de la population phototrophe dominante.
    pub pigment_nm: Option<f32>,
    pub pigment_rgb: Option<[u8; 3]>,
    /// Source hydrothermale active.
    pub vent: bool,
    /// Lumière utile à la photosynthèse en surface, W·m⁻².
    pub light_w_m2: f32,
    pub ph: f32,
    pub salinity: f32,
}

/// Une population d'une cellule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopulationFrame {
    pub lineage: u32,
    pub signature: u32,
    pub biomass: f32,
    pub pigment_nm: Option<f32>,
    pub gene_count: u32,
    pub phototroph: bool,
}

/// Une plaque : pôle de rotation (unitaire) et vitesse angulaire, rad·Ma⁻¹.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlateFrame {
    pub pole: [f32; 3],
    pub omega_rad_per_myr: f32,
}

/// Une lignée, telle que l'arbre du vivant et les fiches la lisent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineageFrame {
    pub id: u32,
    pub parent: u32,
    pub born_years: f64,
    pub extinct_years: Option<f64>,
    pub origin_cell: u32,
    pub signature: u32,
}

/// Paramètres fixes de la planète utiles à l'affichage.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetInfo {
    pub name: String,
    pub seed: u64,
    pub level: u32,
    pub radius_m: f64,
    pub star_temperature_k: f64,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub step: u64,
    pub years: f64,
    pub step_years: f64,
    pub paused: bool,
    pub climate_mode: ClimateMode,
    pub globals: Sample,
    pub cells: Vec<CellFrame>,
    /// Populations de la cellule c : `populations[pop_offsets[c]..pop_offsets[c + 1]]`,
    /// triées par biomasse décroissante.
    pub pop_offsets: Vec<u32>,
    pub populations: Vec<PopulationFrame>,
    pub plates: Vec<PlateFrame>,
    pub planet: Arc<PlanetInfo>,
    /// Identifiant du dernier événement inscrit au journal (exclu), pour lire
    /// les nouveaux.
    pub events_end: u64,
    pub living_lineages: usize,
}

impl Frame {
    pub fn from_world(world: &World) -> Self {
        let planet = &world.planet;
        let sea = planet.sea_level_m;
        let n = planet.cells.len();
        let mut cells = Vec::with_capacity(n);
        let mut pop_offsets = Vec::with_capacity(n + 1);
        let mut populations = Vec::with_capacity(n * 2);
        pop_offsets.push(0);
        for c in 0..n {
            let env = &planet.cells[c];
            let pops = &world.communities[c];
            let dominant = pops.iter().max_by(|a, b| a.biomass.total_cmp(&b.biomass));
            let photo = pops.iter().filter(|p| p.phenotype.pigment_nm.is_some()).max_by(|a, b| a.biomass.total_cmp(&b.biomass));
            let pigment_nm = photo.and_then(|p| p.phenotype.pigment_nm);
            cells.push(CellFrame {
                height_m: (env.elevation_m - sea) as f32,
                temperature_k: env.temperature_k as f32,
                is_ocean: env.is_ocean,
                ice_cover: env.ice_cover as f32,
                biomass: pops.iter().map(|p| p.biomass).sum::<f64>() as f32,
                oxygen: world.chemistry[c][evo_planet::WaterPool::O2 as usize] as f32,
                plate: planet.tectonics.parcel_of(c).plate,
                dominant_guild: dominant.map_or(0, |p| p.signature()),
                pigment_nm: pigment_nm.map(|x| x as f32),
                pigment_rgb: pigment_nm.map(evo_life::pigment_colour),
                vent: env.vent_h2_supply > 0.0,
                light_w_m2: env.light_par_w_m2 as f32,
                ph: env.ph as f32,
                salinity: env.salinity as f32,
            });
            // Une lignée peut compter plusieurs génotypes dans la cellule : le
            // client montre la lignée, avec la biomasse de tous ses génotypes
            // et les traits du plus abondant.
            let mut order: Vec<usize> = (0..pops.len()).collect();
            order.sort_by(|&a, &b| pops[b].biomass.total_cmp(&pops[a].biomass).then(a.cmp(&b)));
            let mut ps: Vec<PopulationFrame> = Vec::new();
            for i in order {
                let p = &pops[i];
                match ps.iter_mut().find(|q| q.lineage == p.lineage) {
                    Some(q) => q.biomass += p.biomass as f32,
                    None => ps.push(PopulationFrame {
                        lineage: p.lineage,
                        signature: p.signature(),
                        biomass: p.biomass as f32,
                        pigment_nm: p.phenotype.pigment_nm.map(|x| x as f32),
                        gene_count: p.phenotype.gene_count,
                        phototroph: p.phenotype.phototroph,
                    }),
                }
            }
            ps.sort_by(|a, b| b.biomass.total_cmp(&a.biomass).then(a.lineage.cmp(&b.lineage)));
            ps.truncate(POPULATIONS_PER_CELL);
            populations.extend(ps);
            pop_offsets.push(populations.len() as u32);
        }
        let plates = planet
            .tectonics
            .plates
            .iter()
            .map(|p| PlateFrame { pole: p.pole.map(|x| x as f32), omega_rad_per_myr: p.omega as f32 })
            .collect();
        Self {
            step: world.stats.steps,
            years: world.years,
            step_years: world.config.step_years,
            paused: world.paused,
            climate_mode: ClimateMode::for_step(world.config.step_years),
            globals: world.history.last().filter(|s| s.years == world.years).copied().unwrap_or_else(|| world.sample()),
            cells,
            pop_offsets,
            populations,
            plates,
            planet: Arc::new(PlanetInfo {
                name: planet.params.name.clone(),
                seed: world.config.seed,
                level: world.config.level,
                radius_m: planet.params.radius_m,
                star_temperature_k: planet.params.star_temperature_k,
            }),
            events_end: world.events.events.len() as u64,
            living_lineages: world.lineages.living_count(),
        }
    }

    pub fn populations_of(&self, cell: usize) -> &[PopulationFrame] {
        let (a, b) = (self.pop_offsets[cell] as usize, self.pop_offsets[cell + 1] as usize);
        &self.populations[a..b]
    }

    /// Cellules où une lignée est présente, avec sa biomasse.
    pub fn range_of(&self, lineage: u32) -> Vec<(usize, f32)> {
        (0..self.cells.len()).filter_map(|c| self.populations_of(c).iter().find(|p| p.lineage == lineage).map(|p| (c, p.biomass))).collect()
    }

    /// Guildes les plus répandues (nombre de cellules dominées), au plus `k`.
    pub fn top_guilds(&self, k: usize) -> Vec<(u32, usize)> {
        let mut counts = std::collections::BTreeMap::new();
        for c in &self.cells {
            if c.dominant_guild != 0 {
                *counts.entry(c.dominant_guild).or_insert(0usize) += 1;
            }
        }
        let mut v: Vec<(u32, usize)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(k);
        v
    }

    /// Vitesse d'une cellule portée par sa plaque, vecteur tangent en m·an⁻¹
    /// (ω × r).
    pub fn plate_velocity(&self, grid: &GeodesicGrid, cell: usize) -> [f64; 3] {
        let Some(p) = self.plates.get(self.cells[cell].plate as usize) else { return [0.0; 3] };
        let r = grid.centers[cell];
        let w = p.pole.map(|x| x as f64);
        let k = p.omega_rad_per_myr as f64 / 1e6 * self.planet.radius_m;
        [k * (w[1] * r[2] - w[2] * r[1]), k * (w[2] * r[0] - w[0] * r[2]), k * (w[0] * r[1] - w[1] * r[0])]
    }
}

/// Lignées connues du moteur.
pub fn lineages_of(world: &World) -> Vec<LineageFrame> {
    world
        .lineages
        .records
        .iter()
        .map(|r| LineageFrame {
            id: r.id,
            parent: r.parent,
            born_years: r.born_years,
            extinct_years: r.extinct_years,
            origin_cell: r.origin_cell,
            signature: r.signature,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_sim::WorldConfig;

    #[test]
    fn frame_mirrors_the_world() {
        let mut w = World::new(WorldConfig::new(7, 3));
        w.seed_life();
        w.step();
        let f = Frame::from_world(&w);
        assert_eq!(f.cells.len(), w.planet.cells.len());
        assert_eq!(f.pop_offsets.len(), f.cells.len() + 1);
        let total: f32 = f.cells.iter().map(|c| c.biomass).sum();
        assert!((total as f64 - w.biomass()).abs() / w.biomass() < 1e-3);
        // Les mers sont sous le niveau de la mer et les terres au-dessus.
        for c in &f.cells {
            if c.is_ocean {
                assert!(c.height_m <= 1.0, "{}", c.height_m);
            }
        }
        let seeded = f.populations.iter().map(|p| p.lineage).min().unwrap();
        assert!(!f.range_of(seeded).is_empty());
        assert!(!f.plates.is_empty());
    }
}
