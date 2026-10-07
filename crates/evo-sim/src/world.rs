//! Le monde de l'étape 1 : une planète fixe peuplée de microbes.
//!
//! Un pas planétaire enchaîne quatre phases :
//! 1. écologie : dynamique rapide des populations couplée à la chimie de l'eau
//!    et aux échanges avec l'extérieur (en parallèle, cellule par cellule) ;
//! 2. évolution : régime « apparition puis fixation » dans chaque population,
//!    avec un coefficient de sélection mesuré par la physiologie ;
//! 3. migration : les écotypes tentent de s'installer dans les cellules
//!    océaniques voisines ;
//! 4. tenue des registres : lignées, événements, conservation du carbone.
//!
//! Les phases parallèles ne lisent et n'écrivent que leur cellule ; tout ce
//! qui crée des identifiants est appliqué ensuite, dans l'ordre des cellules.
//! La même graine donne donc la même histoire quel que soit le nombre de coeurs.

use evo_core::events::{EventKind, EventLog};
use evo_core::flux::{Element, FluxRegistry};
use evo_core::rng::{rng_for, Stream};
use evo_core::Scheduler;
use evo_genetics::genome::MARKER_LEN;
use evo_genetics::{mutate_with_kind, Domain, DomainFamily, Gene, Genome, LineageRegistry, MutationParams, OriginFixation, MUTATION_KINDS};
use evo_life::community::{evaluate, substep, CellContext, Population};
use evo_life::metabolism::{FERMENTATION, METHANOGENESIS, REACTION_COUNT};
use evo_life::{growth_rates, selection_coefficient, Phenotype, Physiology};
use evo_planet::generate::generate;
use evo_planet::{Planet, PlanetParams, WaterChemistry, WaterPool};
use rand::Rng;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Où déposer les cellules minimales au départ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seeding {
    /// Près des sources hydrothermales (choix par défaut du moteur).
    Vents,
    /// Dans toutes les cellules océaniques (mesures de charge).
    AllOcean,
}

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    /// Niveau de subdivision de la grille (6 : 40 962 cellules).
    pub level: u32,
    pub planet: PlanetParams,
    /// Durée d'un pas planétaire, années.
    pub step_years: f64,
    /// Sous-pas écologiques par pas planétaire et leur durée, années.
    pub eco_substeps: usize,
    pub eco_dt_years: f64,
    pub physiology: Physiology,
    pub mutation: MutationParams,
    pub regime: OriginFixation,
    /// Candidats évalués par classe de mutation (même ordre que les classes).
    pub candidates_per_kind: [usize; 6],
    /// Part de la biomasse qui passe chaque année dans une cellule voisine.
    pub migration_rate: f64,
    /// Sous ce seuil de biomasse (mol de carbone), une population disparaît.
    pub extinction_biomass: f64,
    /// Biomasse de départ d'une population fondée par mutation ou migration.
    pub founder_biomass: f64,
    /// Biomasse phototrophe qui absorbe 63 % de la lumière, molC·m⁻².
    pub light_biomass_per_m2: f64,
    pub seeding: Seeding,
    /// Biomasse déposée par cellule au départ, mol de carbone.
    pub seed_biomass: f64,
}

impl WorldConfig {
    pub fn new(seed: u64, level: u32) -> Self {
        Self {
            seed,
            level,
            planet: PlanetParams::earth_archean(),
            step_years: 10_000.0,
            eco_substeps: 30,
            eco_dt_years: 1.0 / 3650.0,
            physiology: Physiology::default(),
            mutation: MutationParams { reaction_count: REACTION_COUNT as u8, ..Default::default() },
            regime: OriginFixation::default(),
            candidates_per_kind: [4, 1, 1, 1, 1, 1],
            migration_rate: 1.0,
            extinction_biomass: 1.0,
            founder_biomass: 100.0,
            light_biomass_per_m2: 0.1,
            seeding: Seeding::Vents,
            seed_biomass: 1e4,
        }
    }
}

/// Compteurs cumulés depuis le début de la partie.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldStats {
    pub steps: u64,
    pub substitutions: u64,
    pub new_lineages: u64,
    pub colonisations: u64,
    pub migrant_replacements: u64,
    pub local_extinctions: u64,
    /// Génomes mutants construits et évalués (mutation, phénotype, r, s).
    pub genetic_evaluations: u64,
}

impl WorldStats {
    fn add(&mut self, o: &WorldStats) {
        self.substitutions += o.substitutions;
        self.new_lineages += o.new_lineages;
        self.colonisations += o.colonisations;
        self.migrant_replacements += o.migrant_replacements;
        self.local_extinctions += o.local_extinctions;
        self.genetic_evaluations += o.genetic_evaluations;
    }
}

/// Temps passé dans chaque phase d'un pas.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhaseTimings {
    pub ecology: Duration,
    pub evolution: Duration,
    pub migration: Duration,
    pub bookkeeping: Duration,
}

impl PhaseTimings {
    pub fn total(&self) -> Duration {
        self.ecology + self.evolution + self.migration + self.bookkeeping
    }

    pub fn add(&mut self, o: &PhaseTimings) {
        self.ecology += o.ecology;
        self.evolution += o.evolution;
        self.migration += o.migration;
        self.bookkeeping += o.bookkeeping;
    }
}

/// Population fondée par mutation, en attente d'un identifiant de lignée.
struct Founder {
    parent: u32,
    population: Population,
}

/// Immigrant retenu pour une guilde d'une cellule.
struct MigrationWinner {
    signature: u32,
    source: (usize, usize),
    selection: f64,
    rates: evo_life::GrowthRates,
    lineage: u32,
    genome: Arc<Genome>,
    phenotype: Arc<Phenotype>,
}

pub struct World {
    pub config: WorldConfig,
    pub planet: Planet,
    pub chemistry: Vec<WaterChemistry>,
    pub communities: Vec<Vec<Population>>,
    pub lineages: LineageRegistry,
    pub events: EventLog,
    pub flux: FluxRegistry,
    pub scheduler: Scheduler,
    pub stats: WorldStats,
    pub timings: PhaseTimings,
}

impl World {
    pub fn new(config: WorldConfig) -> Self {
        let planet = generate(config.planet.clone(), config.level, config.seed);
        let chemistry = planet.initial_chemistry();
        let mut flux = FluxRegistry::default();
        flux.set_initial(Element::Carbon, planet.water_carbon(&chemistry));
        let mut scheduler = Scheduler::new(0.0);
        scheduler.subscribe("monde microbien", config.step_years);
        let n = planet.cells.len();
        Self {
            config,
            planet,
            chemistry,
            communities: vec![Vec::new(); n],
            lineages: LineageRegistry::default(),
            events: EventLog::default(),
            flux,
            scheduler,
            stats: WorldStats::default(),
            timings: PhaseTimings::default(),
        }
    }

    pub fn years(&self) -> f64 {
        self.scheduler.clock.years
    }

    /// Cellule minimale (décision du 7 octobre 2026) : une chimioautotrophie
    /// simple (H₂ + CO₂), une fermentation rudimentaire et un gène de
    /// réparation de l'ADN, adaptés à une eau tiède.
    pub fn minimal_cell(&self) -> Genome {
        let mut rng = rng_for(self.config.seed, Stream::Seeding, &[0]);
        let gene = |family, efficiency, affinity| Gene {
            domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 12.0 },
            functional: true,
        };
        let mut marker = [0u8; MARKER_LEN];
        for b in marker.iter_mut() {
            *b = rng.random_range(0..4);
        }
        Genome {
            genes: vec![
                gene(DomainFamily::Catalytic(METHANOGENESIS), 1.0, 1.0),
                gene(DomainFamily::Catalytic(FERMENTATION), 0.3, 0.5),
                gene(DomainFamily::Repair, 1.0, 0.5),
            ],
            marker,
        }
    }

    /// Dépose des cellules minimales ; le carbone de leur biomasse est pris au
    /// carbone inorganique dissous de la cellule.
    pub fn seed_life(&mut self) {
        let genome = Arc::new(self.minimal_cell());
        let phenotype = Arc::new(Phenotype::from_genome(&genome, &self.config.physiology));
        let cells: Vec<usize> = match self.config.seeding {
            Seeding::Vents => self.planet.vent_cells(),
            Seeding::AllOcean => self.planet.ocean_cells().collect(),
        };
        let first = cells.first().copied().unwrap_or(0) as u32;
        let lineage = self.lineages.found(None, self.years(), first, phenotype.signature, genome.clone());
        self.events.push(self.years(), Some(first), EventKind::LifeSeeded { lineage });
        for c in cells {
            let v = self.planet.cells[c].water_volume_m3;
            let b = self.config.seed_biomass.min(0.5 * self.chemistry[c][WaterPool::Dic as usize] * v);
            self.chemistry[c][WaterPool::Dic as usize] -= b / v;
            self.communities[c].push(Population {
                lineage,
                genome: genome.clone(),
                phenotype: phenotype.clone(),
                biomass: b,
                rates: Default::default(),
            });
        }
    }

    /// Carbone total du système (eau et biomasse), mol.
    pub fn total_carbon(&self) -> f64 {
        self.planet.water_carbon(&self.chemistry) + self.communities.iter().flatten().map(|p| p.biomass).sum::<f64>()
    }

    /// Écart relatif du bilan de carbone.
    pub fn carbon_balance_error(&self) -> f64 {
        self.flux.relative_error(Element::Carbon, self.total_carbon())
    }

    /// Avance d'un pas planétaire.
    pub fn step(&mut self) -> PhaseTimings {
        let mut timings = PhaseTimings::default();
        let dt = self.scheduler.next_tick().first().map(|&(_, s)| s).unwrap_or(self.config.step_years);
        let step_index = self.stats.steps;
        let years = self.years();

        // 1. Écologie.
        let t0 = Instant::now();
        let cfg = &self.config;
        let planet = &self.planet;
        let (flux, eco_stats) = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter_mut())
            .enumerate()
            .map(|(c, (pops, chem))| {
                let mut flux = FluxRegistry::default();
                let mut stats = WorldStats::default();
                let env = &planet.cells[c];
                if !env.is_ocean {
                    return (flux, stats);
                }
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                for _ in 0..cfg.eco_substeps {
                    if !pops.is_empty() {
                        substep(pops, &ctx, chem, cfg.eco_dt_years, &cfg.physiology);
                    }
                    planet.exchange(c, chem, cfg.eco_dt_years, &mut flux);
                }
                // Extinctions locales : la biomasse restante redevient matière organique.
                pops.retain(|p| {
                    if p.biomass < cfg.extinction_biomass {
                        chem[WaterPool::Doc as usize] += p.biomass / env.water_volume_m3;
                        stats.local_extinctions += 1;
                        false
                    } else {
                        true
                    }
                });
                evaluate(pops, &ctx, chem, &cfg.physiology);
                (flux, stats)
            })
            .reduce(
                || (FluxRegistry::default(), WorldStats::default()),
                |(mut fa, mut sa), (fb, sb)| {
                    fa.merge(&fb);
                    sa.add(&sb);
                    (fa, sa)
                },
            );
        self.flux.merge(&flux);
        self.stats.add(&eco_stats);
        timings.ecology = t0.elapsed();

        // 2. Évolution.
        let t1 = Instant::now();
        let seed = cfg.seed;
        let results: Vec<(Vec<Founder>, WorldStats)> = self
            .communities
            .par_iter_mut()
            .zip(self.chemistry.par_iter())
            .enumerate()
            .map(|(c, (pops, chem))| {
                let env = &planet.cells[c];
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                evolve_cell(c, pops, chem, &ctx, cfg, dt, seed, step_index)
            })
            .collect();
        let mut founders_by_cell = Vec::with_capacity(results.len());
        for (founders, stats) in results {
            self.stats.add(&stats);
            founders_by_cell.push(founders);
        }
        for (c, founders) in founders_by_cell.into_iter().enumerate() {
            for f in founders {
                let mut p = f.population;
                p.lineage = self.lineages.found(Some(f.parent), years, c as u32, p.signature(), p.genome.clone());
                self.events.push(
                    years,
                    Some(c as u32),
                    EventKind::NewLineage { lineage: p.lineage, parent: f.parent, signature: p.signature() },
                );
                self.stats.new_lineages += 1;
                self.communities[c].push(p);
            }
        }
        timings.evolution = t1.elapsed();

        // 3. Migration.
        let t2 = Instant::now();
        self.migrate(dt, step_index);
        timings.migration = t2.elapsed();

        // 4. Registres.
        let t3 = Instant::now();
        let mut alive = vec![0u32; self.lineages.records.len()];
        for p in self.communities.iter().flatten() {
            alive[p.lineage as usize] += 1;
        }
        for (id, &count) in alive.iter().enumerate() {
            let rec = &mut self.lineages.records[id];
            if count == 0 && rec.extinct_years.is_none() {
                rec.extinct_years = Some(years);
                self.events.push(years, None, EventKind::LineageExtinct { lineage: id as u32 });
            }
        }
        self.stats.steps += 1;
        timings.bookkeeping = t3.elapsed();
        self.timings.add(&timings);
        timings
    }

    /// Chaque cellule océanique regarde les écotypes de ses voisines et
    /// retient, par guilde, le meilleur immigrant qui réussit à s'installer.
    fn migrate(&mut self, dt: f64, step_index: u64) {
        let cfg = &self.config;
        let planet = &self.planet;
        let communities = &self.communities;
        let chemistry = &self.chemistry;
        let physio = &cfg.physiology;

        let winners: Vec<Vec<MigrationWinner>> = (0..communities.len())
            .into_par_iter()
            .map(|target| {
                let mut best: Vec<MigrationWinner> = Vec::new();
                let env = &planet.cells[target];
                if !env.is_ocean {
                    return best;
                }
                let residents = &communities[target];
                let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
                let light = ctx.light_per_biomass(CellContext::photo_biomass(residents));
                for src in planet.grid.neighbours_of(target) {
                    for (i, p) in communities[src].iter().enumerate() {
                        if p.rates.birth <= 0.0 {
                            continue;
                        }
                        let resident = residents.iter().find(|q| q.signature() == p.signature());
                        if resident.is_some_and(|q| Arc::ptr_eq(&q.genome, &p.genome)) {
                            continue;
                        }
                        let rates = growth_rates(&p.phenotype, env.temperature_k, &chemistry[target], light, physio);
                        let (s, ne) = match resident {
                            Some(q) => (selection_coefficient(&rates, &q.rates, physio), cfg.regime.effective_size(q.census(physio))),
                            None => (rates.r * rates.generation_time(physio), cfg.regime.ne_cap),
                        };
                        if s <= 0.0 {
                            continue;
                        }
                        let slot = best.iter().position(|w| w.signature == p.signature());
                        if slot.is_some_and(|k| best[k].selection >= s) {
                            continue;
                        }
                        let migrants = cfg.regime.effective_size(p.census(physio) * cfg.migration_rate * dt);
                        let mut rng = rng_for(cfg.seed, Stream::Migration, &[step_index, target as u64, src as u64, i as u64]);
                        if !cfg.regime.candidate_fixes(s, ne, migrants, &mut rng) {
                            continue;
                        }
                        let w = MigrationWinner {
                            signature: p.signature(),
                            source: (src, i),
                            selection: s,
                            rates,
                            lineage: p.lineage,
                            genome: p.genome.clone(),
                            phenotype: p.phenotype.clone(),
                        };
                        match slot {
                            Some(k) => best[k] = w,
                            None => best.push(w),
                        }
                    }
                }
                best
            })
            .collect();

        for (target, list) in winners.into_iter().enumerate() {
            for w in list {
                let (sc, si) = w.source;
                let (lineage, genome, phenotype) = (w.lineage, w.genome, w.phenotype);
                if let Some(q) = self.communities[target].iter_mut().find(|q| q.signature() == w.signature) {
                    q.genome = genome;
                    q.phenotype = phenotype;
                    q.lineage = lineage;
                    self.stats.migrant_replacements += 1;
                } else {
                    // La biomasse des colons est prise à la population source.
                    let give = self.config.founder_biomass.min(0.5 * self.communities[sc][si].biomass);
                    if give < self.config.extinction_biomass {
                        continue;
                    }
                    self.communities[sc][si].biomass -= give;
                    self.communities[target].push(Population { lineage, genome, phenotype, biomass: give, rates: w.rates });
                    self.stats.colonisations += 1;
                }
            }
        }
    }

    /// Résumé lisible de l'état du monde.
    pub fn summary(&self) -> Summary {
        let mut guilds: BTreeMap<u32, (usize, f64)> = BTreeMap::new();
        let (mut mismatch, mut weight) = (0.0, 0.0);
        for (c, pops) in self.communities.iter().enumerate() {
            for p in pops {
                let e = guilds.entry(p.signature()).or_default();
                e.0 += 1;
                e.1 += p.biomass;
                if let Some(t) = p.phenotype.mean_t_opt() {
                    mismatch += p.biomass * (t - self.planet.cells[c].temperature_k).abs();
                    weight += p.biomass;
                }
            }
        }
        let ocean: Vec<usize> = self.planet.ocean_cells().collect();
        let colonised = ocean.iter().filter(|&&c| !self.communities[c].is_empty()).count();
        let volume: f64 = ocean.iter().map(|&c| self.planet.cells[c].water_volume_m3).sum();
        let mut mean_chem = [0.0; evo_planet::WATER_POOL_COUNT];
        for &c in &ocean {
            let v = self.planet.cells[c].water_volume_m3;
            for (m, x) in mean_chem.iter_mut().zip(self.chemistry[c]) {
                *m += x * v / volume;
            }
        }
        Summary {
            years: self.years(),
            ocean_cells: ocean.len(),
            colonised_cells: colonised,
            populations: self.communities.iter().map(Vec::len).sum(),
            biomass: self.communities.iter().flatten().map(|p| p.biomass).sum(),
            guilds,
            lineages_total: self.lineages.records.len(),
            lineages_living: self.lineages.living().count(),
            thermal_mismatch_k: if weight > 0.0 { mismatch / weight } else { f64::NAN },
            mean_genes: mean(self.communities.iter().flatten().map(|p| p.genome.genes.len() as f64)),
            mean_functional_genes: mean(self.communities.iter().flatten().map(|p| p.genome.functional_genes().count() as f64)),
            mean_chemistry: mean_chem,
            carbon_error: self.carbon_balance_error(),
            stats: self.stats,
        }
    }

    /// Mémoire occupée par l'état de la simulation, en octets (estimation par
    /// la taille des structures ; les génomes partagés sont comptés une fois).
    pub fn memory_bytes(&self) -> MemoryReport {
        let mut seen = std::collections::HashSet::new();
        let mut genomes = 0usize;
        for p in self.communities.iter().flatten() {
            if seen.insert(Arc::as_ptr(&p.genome) as usize) {
                genomes += p.genome.memory_bytes() + std::mem::size_of::<Phenotype>() + p.phenotype.enzymes.capacity() * 40;
            }
        }
        MemoryReport {
            planet: self.planet.memory_bytes() + self.chemistry.capacity() * std::mem::size_of::<WaterChemistry>(),
            populations: self
                .communities
                .iter()
                .map(|v| std::mem::size_of::<Vec<Population>>() + v.capacity() * std::mem::size_of::<Population>())
                .sum(),
            genomes,
            distinct_genomes: seen.len(),
            lineages: self.lineages.memory_bytes(),
        }
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryReport {
    pub planet: usize,
    pub populations: usize,
    pub genomes: usize,
    pub distinct_genomes: usize,
    pub lineages: usize,
}

#[derive(Clone, Debug)]
pub struct Summary {
    pub years: f64,
    pub ocean_cells: usize,
    pub colonised_cells: usize,
    pub populations: usize,
    pub biomass: f64,
    /// Par guilde (signature) : nombre de populations et biomasse.
    pub guilds: BTreeMap<u32, (usize, f64)>,
    pub lineages_total: usize,
    pub lineages_living: usize,
    /// Écart moyen entre l'optimum thermique des enzymes et la température
    /// locale, pondéré par la biomasse, K.
    pub thermal_mismatch_k: f64,
    /// Nombre moyen de gènes par population, dont fonctionnels.
    pub mean_genes: f64,
    pub mean_functional_genes: f64,
    pub mean_chemistry: WaterChemistry,
    pub carbon_error: f64,
    pub stats: WorldStats,
}

/// Régime « apparition puis fixation » dans les populations d'une cellule.
#[allow(clippy::too_many_arguments)]
fn evolve_cell(
    cell: usize,
    pops: &mut [Population],
    chem: &WaterChemistry,
    ctx: &CellContext,
    cfg: &WorldConfig,
    dt: f64,
    seed: u64,
    step_index: u64,
) -> (Vec<Founder>, WorldStats) {
    let mut stats = WorldStats::default();
    let mut founders = Vec::new();
    if pops.is_empty() {
        return (founders, stats);
    }
    let physio = &cfg.physiology;
    let light = ctx.light_per_biomass(CellContext::photo_biomass(pops));
    let weight_total: f64 = cfg.mutation.weights.iter().sum();
    let n = pops.len();
    for i in 0..n {
        let resident = &pops[i];
        if resident.rates.birth <= 0.0 {
            continue;
        }
        let mut rng = rng_for(seed, Stream::Mutation, &[step_index, cell as u64, i as u64]);
        let generations = dt / resident.rates.generation_time(physio);
        let ne = cfg.regime.effective_size(resident.census(physio));
        let u = cfg.mutation.genomic_rate(&resident.genome);
        let mut best: Option<(f64, Genome, Phenotype, evo_life::GrowthRates)> = None;
        for (k, &kind) in MUTATION_KINDS.iter().enumerate() {
            let count = cfg.candidates_per_kind[k];
            if count == 0 {
                continue;
            }
            let copies = ne * u * generations * cfg.mutation.weights[k] / weight_total / count as f64;
            for _ in 0..count {
                let genome = mutate_with_kind(&resident.genome, kind, &cfg.mutation, &mut rng).genome;
                let phenotype = Phenotype::from_genome(&genome, physio);
                let rates = growth_rates(&phenotype, ctx.env.temperature_k, chem, light, physio);
                stats.genetic_evaluations += 1;
                if phenotype.signature == 0 {
                    continue;
                }
                // Un mutant de guilde nouvelle est jugé contre la population de
                // cette guilde si elle existe déjà dans la cellule.
                let competitor = if phenotype.signature == resident.signature() {
                    resident
                } else {
                    pops.iter().find(|q| q.signature() == phenotype.signature).unwrap_or(resident)
                };
                let s = selection_coefficient(&rates, &competitor.rates, physio);
                if best.as_ref().is_some_and(|b| b.0 >= s) {
                    continue;
                }
                if cfg.regime.candidate_fixes(s, ne, copies, &mut rng) {
                    best = Some((s, genome, phenotype, rates));
                }
            }
        }
        let Some((_, genome, phenotype, rates)) = best else { continue };
        let parent_lineage = pops[i].lineage;
        let (genome, phenotype) = (Arc::new(genome), Arc::new(phenotype));
        if phenotype.signature == pops[i].signature() {
            let p = &mut pops[i];
            p.genome = genome;
            p.phenotype = phenotype;
            p.rates = rates;
            stats.substitutions += 1;
        } else if let Some(j) = pops.iter().position(|q| q.signature() == phenotype.signature) {
            let q = &mut pops[j];
            q.genome = genome;
            q.phenotype = phenotype;
            q.rates = rates;
            q.lineage = parent_lineage;
            stats.substitutions += 1;
        } else if !founders.iter().any(|f: &Founder| f.population.signature() == phenotype.signature) {
            // Nouvelle guilde : la biomasse fondatrice est prise au parent.
            let give = cfg.founder_biomass.min(0.5 * pops[i].biomass);
            if give < cfg.extinction_biomass {
                continue;
            }
            pops[i].biomass -= give;
            founders.push(Founder {
                parent: parent_lineage,
                population: Population { lineage: parent_lineage, genome, phenotype, biomass: give, rates },
            });
        }
    }
    (founders, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_world(seed: u64) -> World {
        let mut cfg = WorldConfig::new(seed, 3);
        cfg.step_years = 1000.0;
        let mut w = World::new(cfg);
        w.seed_life();
        w
    }

    #[test]
    fn same_seed_same_history() {
        let mut a = small_world(9);
        let mut b = small_world(9);
        for _ in 0..15 {
            a.step();
            b.step();
        }
        let (sa, sb) = (a.summary(), b.summary());
        assert_eq!(sa.stats, sb.stats);
        assert_eq!(sa.biomass.to_bits(), sb.biomass.to_bits());
        assert_eq!(a.events.events, b.events.events);
    }

    #[test]
    fn determinism_does_not_depend_on_thread_count() {
        let run = |threads: usize| {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
            pool.install(|| {
                let mut w = small_world(4);
                for _ in 0..10 {
                    w.step();
                }
                (w.summary().biomass.to_bits(), w.stats)
            })
        };
        assert_eq!(run(1), run(4));
    }

    #[test]
    fn carbon_is_conserved() {
        let mut w = small_world(3);
        for _ in 0..20 {
            w.step();
            assert!(w.carbon_balance_error() < 1e-9, "écart {}", w.carbon_balance_error());
        }
    }

    #[test]
    fn life_spreads_and_adapts_to_local_temperature() {
        let mut w = small_world(1);
        let start = w.summary();
        for _ in 0..60 {
            w.step();
        }
        let end = w.summary();
        assert!(end.colonised_cells > 3 * start.colonised_cells, "{} → {}", start.colonised_cells, end.colonised_cells);
        assert!(end.stats.substitutions > 0);
        assert!(end.thermal_mismatch_k < 3.0, "écart thermique {} K", end.thermal_mismatch_k);
    }
}
