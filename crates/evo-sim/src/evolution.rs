//! Évolution dans les populations d'une cellule, pendant un pas planétaire.
//!
//! Régime « apparition puis fixation » de l'étape 1, complété à l'étape 2 par
//! les mécanismes qui franchissent les innovations à plusieurs pièces
//! (document Génétique, « Franchir les innovations à plusieurs composants ») :
//!
//! - duplication suivie de divergence (classe de mutation, table de parenté
//!   des domaines tenue par Organismes) ;
//! - tunnel stochastique (Weissman et coll., 2009) : un premier mutant qui ne
//!   se fixe pas peut porter un second mutant avantageux ;
//! - transfert horizontal : un gène d'une autre population de la cellule ;
//! - accélérateur, en dernier recours, quand le chemin vers la photosynthèse
//!   stagne : plus de mutations innovantes et de transferts. Une fixation
//!   qui n'aurait pas eu lieu sans lui porte la cause « Accélérateur ».

use crate::world::WorldConfig;
use evo_core::rng::{rng_for, Stream};
use evo_genetics::popgen::fixation_probability;
use evo_genetics::{
    mutate, mutate_with_kind, transfer_gene, tunnel_probability, ChangedElement, Genome, GenomeChange, GenomeChangeCause, MutationKind,
    OriginFixation, GENOME_CHANGE_CAUSE_COUNT, MUTATION_KINDS, MUTATION_KIND_COUNT,
};
use evo_life::community::{CellContext, Population};
use evo_life::{growth_rates, selection_coefficient, GrowthRates, Phenotype};
use evo_planet::WaterChemistry;
use rand::Rng;
use std::sync::Arc;

/// Réglages de l'évolution.
#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionParams {
    /// Candidats évalués par classe de mutation (ordre de [`MUTATION_KINDS`]).
    pub candidates_per_kind: [usize; MUTATION_KIND_COUNT],
    /// Tunnel stochastique : tenté pour les premiers mutants qui ne se fixent
    /// pas et dont le coefficient de sélection dépasse ce seuil (un mutant
    /// très délétère disparaît avant de porter quoi que ce soit).
    pub tunnel: bool,
    pub tunnel_min_selection: f64,
    /// Transferts horizontaux reçus par génome et par génération, et
    /// candidats évalués par population et par pas.
    pub hgt_rate: f64,
    pub hgt_candidates: usize,
    pub accelerator: AcceleratorParams,
}

impl Default for EvolutionParams {
    fn default() -> Self {
        Self {
            candidates_per_kind: [4, 1, 1, 1, 1, 1, 2],
            tunnel: true,
            tunnel_min_selection: -0.05,
            hgt_rate: 1e-7,
            hgt_candidates: 1,
            accelerator: AcceleratorParams::default(),
        }
    }
}

/// Accélérateur de l'émergence assistée (décision du 7 octobre 2026 : actif
/// seulement quand l'évolution stagne).
#[derive(Clone, Debug, PartialEq)]
pub struct AcceleratorParams {
    pub enabled: bool,
    /// Durée sans progrès sur le chemin de la photosynthèse, depuis l'arrivée
    /// de la vie ou la dernière étape franchie, avant d'agir, années.
    pub patience_years: f64,
    /// Multiplicateur des mutations innovantes (de novo, duplication suivie
    /// de divergence) et des transferts horizontaux quand il agit.
    pub boost: f64,
}

impl Default for AcceleratorParams {
    fn default() -> Self {
        Self { enabled: true, patience_years: 300e6, boost: 100.0 }
    }
}

/// Population fondée par mutation, en attente d'un identifiant de lignée.
pub struct Founder {
    pub parent: u32,
    pub population: Population,
}

/// Où une modification fixée a pris place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// Population existante de la cellule (indice).
    Population(usize),
    /// Population fondatrice (indice dans la liste des fondateurs).
    Founder(usize),
}

/// Modification de génome fixée pendant le pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedChange {
    pub target: Target,
    pub cause: GenomeChangeCause,
    pub element: ChangedElement,
}

/// Compteurs de l'évolution d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvolutionStats {
    pub substitutions: u64,
    pub genetic_evaluations: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    pub fixed_by_cause: [u64; GENOME_CHANGE_CAUSE_COUNT],
}

impl EvolutionStats {
    pub fn add(&mut self, o: &EvolutionStats) {
        self.substitutions += o.substitutions;
        self.genetic_evaluations += o.genetic_evaluations;
        self.tunnel_attempts += o.tunnel_attempts;
        self.tunnel_successes += o.tunnel_successes;
        for (a, b) in self.fixed_by_cause.iter_mut().zip(o.fixed_by_cause) {
            *a += b;
        }
    }
}

pub struct CellEvolution {
    pub founders: Vec<Founder>,
    pub changes: Vec<FixedChange>,
    pub stats: EvolutionStats,
}

/// Meilleur candidat fixé pour une population.
struct Best {
    s: f64,
    change: GenomeChange,
    phenotype: Phenotype,
    rates: GrowthRates,
}

/// Tire le sort d'un candidat : fixation ordinaire, ou, si l'accélérateur
/// agit sur sa classe, fixation qu'il rend possible (cause « Accélérateur »).
fn fixes(regime: &OriginFixation, s: f64, ne: f64, copies: f64, boost: f64, rng: &mut impl Rng) -> Option<bool> {
    if regime.candidate_fixes(s, ne, copies, rng) {
        return Some(false);
    }
    if boost > 1.0 && regime.candidate_fixes(s, ne, copies * (boost - 1.0), rng) {
        return Some(true);
    }
    None
}

/// Régime « apparition puis fixation » dans les populations d'une cellule.
#[allow(clippy::too_many_arguments)]
pub fn evolve_cell(
    cell: usize,
    pops: &mut [Population],
    chem: &WaterChemistry,
    ctx: &CellContext,
    cfg: &WorldConfig,
    dt: f64,
    step_index: u64,
    accelerator_on: bool,
) -> CellEvolution {
    let mut out = CellEvolution { founders: Vec::new(), changes: Vec::new(), stats: EvolutionStats::default() };
    if pops.is_empty() {
        return out;
    }
    let physio = &cfg.physiology;
    let evo = &cfg.evolution;
    let cond = ctx.conditions(CellContext::photo_biomass(pops));
    let weight_total: f64 = cfg.mutation.weights.iter().sum();
    let cell_biomass: f64 = pops.iter().map(|p| p.biomass).sum();
    let boost = if accelerator_on { evo.accelerator.boost } else { 1.0 };
    let n = pops.len();
    for i in 0..n {
        let resident = &pops[i];
        if resident.rates.birth <= 0.0 {
            continue;
        }
        let mut rng = rng_for(cfg.seed, Stream::Mutation, &[step_index, cell as u64, i as u64]);
        let generations = dt / resident.rates.generation_time(physio);
        let ne = cfg.regime.effective_size(resident.census(physio));
        let u = cfg.mutation.genomic_rate(&resident.genome);

        // Évalue un génome candidat contre la population qu'il affronterait.
        let evaluate = |genome: &Genome, stats: &mut EvolutionStats| -> Option<(f64, Phenotype, GrowthRates)> {
            let phenotype = Phenotype::from_genome(genome, physio);
            stats.genetic_evaluations += 1;
            if phenotype.signature == 0 {
                return None;
            }
            let rates = growth_rates(&phenotype, &cond, chem, physio);
            // Un mutant de guilde nouvelle est jugé contre la population de
            // cette guilde si elle existe déjà dans la cellule.
            let competitor = if phenotype.signature == resident.signature() {
                resident
            } else {
                pops.iter().find(|q| q.signature() == phenotype.signature).unwrap_or(resident)
            };
            Some((selection_coefficient(&rates, &competitor.rates, physio), phenotype, rates))
        };

        let mut best: Option<Best> = None;
        let consider = |best: &mut Option<Best>, s: f64, change: GenomeChange, phenotype: Phenotype, rates: GrowthRates| {
            if best.as_ref().is_none_or(|b| s > b.s) {
                *best = Some(Best { s, change, phenotype, rates });
            }
        };

        for (k, &kind) in MUTATION_KINDS.iter().enumerate() {
            let count = evo.candidates_per_kind[k];
            if count == 0 {
                continue;
            }
            let innovative = matches!(kind, MutationKind::DeNovo | MutationKind::DuplicationDivergence);
            let kind_boost = if innovative { boost } else { 1.0 };
            let copies = ne * u * generations * cfg.mutation.weights[k] / weight_total / count as f64;
            for _ in 0..count {
                let mut change = mutate_with_kind(&resident.genome, kind, &cfg.mutation, &mut rng);
                let Some((s, phenotype, rates)) = evaluate(&change.genome, &mut out.stats) else { continue };
                if best.as_ref().is_some_and(|b| b.s >= s) && s > 0.0 {
                    continue;
                }
                match fixes(&cfg.regime, s, ne, copies, kind_boost, &mut rng) {
                    Some(accelerated) => {
                        if accelerated {
                            change.cause = GenomeChangeCause::Accelerator;
                        }
                        consider(&mut best, s, change, phenotype, rates);
                    }
                    None if evo.tunnel && s > evo.tunnel_min_selection => {
                        // Tunnel stochastique : la lignée du premier mutant,
                        // tant qu'elle survit, produit des doubles mutants.
                        let second = mutate(&change.genome, &cfg.mutation, &mut rng);
                        let GenomeChangeCause::SpontaneousMutation(kind2) = second.cause else { continue };
                        out.stats.tunnel_attempts += 1;
                        let Some((s2, phenotype2, rates2)) = evaluate(&second.genome, &mut out.stats) else { continue };
                        if s2 <= 0.0 || best.as_ref().is_some_and(|b| b.s >= s2) {
                            continue;
                        }
                        let k2 = MUTATION_KINDS.iter().position(|&x| x == kind2).unwrap_or(0);
                        let mu2 = u * cfg.mutation.weights[k2] / weight_total;
                        let p2 = fixation_probability(s2, ne, 1.0 / ne);
                        let p1 = tunnel_probability((-s).max(0.0), mu2, p2);
                        if rng.random::<f64>() < OriginFixation::any_fixes(p1, copies) {
                            out.stats.tunnel_successes += 1;
                            let double = GenomeChange { genome: second.genome, cause: change.cause, element: ChangedElement::Several };
                            consider(&mut best, s2, double, phenotype2, rates2);
                        }
                    }
                    None => {}
                }
            }
        }

        // Transfert horizontal depuis une autre population de la cellule.
        let others = cell_biomass - resident.biomass;
        if evo.hgt_candidates > 0 && evo.hgt_rate > 0.0 && others > 0.0 {
            for _ in 0..evo.hgt_candidates {
                let mut x = rng.random::<f64>() * others;
                let Some(donor) = pops.iter().enumerate().filter(|&(j, _)| j != i).find_map(|(_, q)| {
                    x -= q.biomass;
                    (x < 0.0).then_some(q)
                }) else {
                    continue;
                };
                let genes: Vec<_> = donor.genome.functional_genes().copied().collect();
                if genes.is_empty() {
                    continue;
                }
                let gene = genes[rng.random_range(0..genes.len())];
                let mut change = transfer_gene(&resident.genome, gene, GenomeChangeCause::HorizontalTransfer);
                let Some((s, phenotype, rates)) = evaluate(&change.genome, &mut out.stats) else { continue };
                if best.as_ref().is_some_and(|b| b.s >= s) {
                    continue;
                }
                let copies = ne * evo.hgt_rate * generations * donor.biomass / cell_biomass / evo.hgt_candidates as f64;
                if let Some(accelerated) = fixes(&cfg.regime, s, ne, copies, boost, &mut rng) {
                    if accelerated {
                        change.cause = GenomeChangeCause::Accelerator;
                    }
                    consider(&mut best, s, change, phenotype, rates);
                }
            }
        }

        let Some(Best { change, phenotype, rates, .. }) = best else { continue };
        let GenomeChange { genome, cause, element } = change;
        out.stats.fixed_by_cause[cause.index()] += 1;
        let parent_lineage = pops[i].lineage;
        let (genome, phenotype) = (Arc::new(genome), Arc::new(phenotype));
        let target = if phenotype.signature == pops[i].signature() {
            let p = &mut pops[i];
            p.genome = genome;
            p.phenotype = phenotype;
            p.rates = rates;
            Target::Population(i)
        } else if let Some(j) = pops.iter().position(|q| q.signature() == phenotype.signature) {
            let q = &mut pops[j];
            q.genome = genome;
            q.phenotype = phenotype;
            q.rates = rates;
            q.lineage = parent_lineage;
            Target::Population(j)
        } else if !out.founders.iter().any(|f| f.population.signature() == phenotype.signature) {
            // Nouvelle guilde : la biomasse fondatrice est prise au parent.
            let give = cfg.founder_biomass.min(0.5 * pops[i].biomass);
            if give < cfg.extinction_biomass {
                out.stats.fixed_by_cause[cause.index()] -= 1;
                continue;
            }
            pops[i].biomass -= give;
            out.founders.push(Founder {
                parent: parent_lineage,
                population: Population { lineage: parent_lineage, genome, phenotype, biomass: give, rates },
            });
            Target::Founder(out.founders.len() - 1)
        } else {
            out.stats.fixed_by_cause[cause.index()] -= 1;
            continue;
        };
        out.stats.substitutions += 1;
        out.changes.push(FixedChange { target, cause, element });
    }
    out
}
