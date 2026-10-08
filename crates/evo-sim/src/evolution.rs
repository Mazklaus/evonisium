//! Évolution par dème, pendant un pas planétaire (étape 3).
//!
//! Le niveau génétique prévu par le document Vision est le dème : une région
//! de nombreuses cellules du vivant. Les mutations, la fixation et le
//! transfert horizontal sont calculés une fois par dème et par génotype
//! (écotype d'une guilde partagé par plusieurs cellules), dans la cellule où
//! ce génotype est le plus abondant, puis appliqués à toutes les cellules du
//! dème qui le portent. L'écologie reste calculée par cellule. L'offre de
//! mutants est celle de toutes ces cellules réunies : un dème ne découvre ni
//! plus ni moins d'innovations que ses cellules à l'étape 2, pour un coût
//! divisé par le nombre de cellules qui partagent un génotype.
//!
//! Un dème est une région sans barrière : les cellules d'une maille d'une
//! grille géodésique plus grossière (niveau du vivant moins un : environ
//! 4 cellules du vivant, 450 km) qui ont le même milieu (mer ou eaux
//! douces) et une température voisine (classes de 4 K). Un fort gradient de
//! température est une barrière pour la sélection : de part et d'autre, le
//! même génotype s'adapte différemment.
//!
//! [Simplification] Les barrières physiques (détroits, reliefs) à
//! l'intérieur d'une maille sont ignorées ; la migration entre cellules
//! fait le reste.
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

use crate::transitions;
use crate::world::WorldConfig;
use evo_core::rng::{rng_for, Stream};
use evo_genetics::popgen::fixation_probability;
use evo_genetics::{
    mutate, mutate_with_kind, transfer_gene, tunnel_probability, ChangedElement, Genome, GenomeChange, GenomeChangeCause, MutationKind,
    OriginFixation, GENOME_CHANGE_CAUSE_COUNT, MUTATION_KINDS, MUTATION_KIND_COUNT,
};
use evo_life::community::{CellContext, Population};
use evo_life::metabolism::photosynthesis_stage;
use evo_life::phenotype::PATHWAY_MASK;
use evo_life::{growth_rates, selection_coefficient, GrowthRates, Phenotype};
use evo_planet::CellEnvironment;
use evo_planet::WaterChemistry;
use rand::Rng;
use std::sync::Arc;

/// Réglages de l'évolution.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvolutionParams {
    /// Candidats évalués par classe de mutation (ordre de [`MUTATION_KINDS`]).
    pub candidates_per_kind: [usize; MUTATION_KIND_COUNT],
    /// Tunnel stochastique : tenté pour les premiers mutants qui ne se fixent
    /// pas et dont le coefficient de sélection dépasse ce seuil (un mutant
    /// très délétère disparaît avant de porter quoi que ce soit).
    pub tunnel: bool,
    pub tunnel_min_selection: f64,
    /// Tentatives de tunnel au plus par génotype et par pas (les premiers
    /// candidats) ; au-delà, chaque essai est pondéré par
    /// (candidats / essais) pour ne pas biaiser le taux de franchissement.
    pub tunnel_attempts_per_genotype: usize,
    /// Transferts horizontaux reçus par génome et par génération, et
    /// candidats évalués par population et par pas.
    pub hgt_rate: f64,
    pub hgt_candidates: usize,
    pub accelerator: AcceleratorParams,
    /// Durée au plus d'un tour « apparition puis fixation », années : un pas
    /// plus long enchaîne plusieurs tours, pour que le nombre de
    /// substitutions par génotype ne dépende pas de la durée du pas
    /// (`None` : un tour par pas, quelle que soit sa durée). Le test
    /// d'équivalence des pas (docs/etape-3-equivalence.md) fixe 100 ka.
    pub round_years: Option<f64>,
}

impl Default for EvolutionParams {
    fn default() -> Self {
        Self {
            candidates_per_kind: [4, 1, 1, 1, 1, 1, 2, 1],
            tunnel: true,
            tunnel_min_selection: -0.05,
            tunnel_attempts_per_genotype: 2,
            hgt_rate: 1e-7,
            hgt_candidates: 1,
            accelerator: AcceleratorParams::default(),
            round_years: Some(100_000.0),
        }
    }
}

/// Accélérateur de l'émergence assistée (décision du 7 octobre 2026 : actif
/// seulement quand l'évolution stagne).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AcceleratorParams {
    pub enabled: bool,
    /// Durée sans progrès sur le chemin de la photosynthèse, depuis l'arrivée
    /// de la vie ou la dernière étape franchie, avant d'agir, années.
    pub patience_years: f64,
    /// Multiplicateur des mutations innovantes (de novo, duplication suivie
    /// de divergence) et des transferts horizontaux quand il agit.
    pub boost: f64,
    /// Durée sans nouvelle étape de la complexité (après la photosynthèse
    /// oxygénique) avant d'agir sur les transitions de l'étape 4 : rétentions
    /// d'endosymbiotes et mutations innovantes plus fréquentes.
    pub complexity_patience_years: f64,
}

impl Default for AcceleratorParams {
    fn default() -> Self {
        Self { enabled: true, patience_years: 300e6, boost: 100.0, complexity_patience_years: 1.5e9 }
    }
}

/// Compteurs de l'évolution d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvolutionStats {
    pub substitutions: u64,
    pub genetic_evaluations: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    /// Génotypes avec au moins un candidat au tunnel, et ceux qui en avaient
    /// plus que la borne d'essais.
    pub tunnel_genotypes: u64,
    pub tunnel_capped: u64,
    /// Mutations réunies par recombinaison chez les sexués.
    pub recombinations: u64,
    pub fixed_by_cause: [u64; GENOME_CHANGE_CAUSE_COUNT],
}

impl EvolutionStats {
    pub fn add(&mut self, o: &EvolutionStats) {
        self.substitutions += o.substitutions;
        self.genetic_evaluations += o.genetic_evaluations;
        self.tunnel_attempts += o.tunnel_attempts;
        self.tunnel_successes += o.tunnel_successes;
        self.tunnel_genotypes += o.tunnel_genotypes;
        self.tunnel_capped += o.tunnel_capped;
        self.recombinations += o.recombinations;
        for (a, b) in self.fixed_by_cause.iter_mut().zip(o.fixed_by_cause) {
            *a += b;
        }
    }
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

/// Vrai si `new` a une fonction absente de `old` : une réaction catalysée de
/// plus, une étape plus avancée du chemin vers la photosynthèse, ou une
/// rhodopsine.
fn gains_function(new: &Phenotype, old: &Phenotype) -> bool {
    new.signature & !old.signature != 0
        || photosynthesis_stage(new) > photosynthesis_stage(old)
        || (new.rhodopsin > 0.0 && old.rhodopsin <= 0.0)
}

/// Largeur des classes de température qui séparent les dèmes d'une maille, K.
pub const DEME_TEMPERATURE_BAND_K: f64 = 2.0;

/// Classe de milieu d'une cellule du vivant : eaux douces ou mer, et
/// température.
pub fn habitat_class(env: &CellEnvironment) -> i64 {
    let band = (env.temperature_k / DEME_TEMPERATURE_BAND_K).floor() as i64;
    band * 2 + i64::from(env.is_ocean)
}

/// Groupe des populations d'un dème qui partagent un génotype.
pub struct GenotypeGroup {
    /// Classe de milieu du dème (voir [`habitat_class`]).
    pub habitat: i64,
    /// Cellule du vivant et indice de la population la plus abondante.
    pub rep: (usize, usize),
    /// Offre de mutants : somme des effectifs efficaces de chaque cellule.
    pub supply_ne: f64,
    /// Effectif total du groupe.
    pub census: f64,
    /// Cellules où le génotype est présent.
    pub cells: u32,
    /// Populations du groupe (cellule, indice, biomasse), les plus abondantes
    /// d'abord, au plus [`MAX_JUDGED_CELLS`] : un mutant y est jugé.
    pub members: Vec<(usize, usize, f64)>,
}

/// Cellules où un mutant est jugé, au plus, pour un génotype d'un dème.
pub const MAX_JUDGED_CELLS: usize = 6;

impl GenotypeGroup {
    /// Multiplicateur du nombre de candidats évalués : un génotype répandu
    /// sur plusieurs cellules y explore plus de mutants, comme ses cellules
    /// le faisaient séparément, mais en racine carrée du nombre de cellules
    /// (au plus 4) pour garder le gain de calcul.
    pub fn candidate_factor(&self) -> usize {
        ((self.cells as f64).sqrt().ceil() as usize).clamp(1, 4)
    }
}

/// Rassemble les populations des cellules `cells` par génotype, dans l'ordre
/// de première apparition (cellules croissantes, puis indices).
pub fn genotype_groups(cells: &[u32], communities: &[Vec<Population>], envs: &[CellEnvironment], cfg: &WorldConfig) -> Vec<GenotypeGroup> {
    let physio = &cfg.physiology;
    let mut keys: Vec<(usize, i64)> = Vec::new();
    let mut groups: Vec<GenotypeGroup> = Vec::new();
    for &c in cells {
        let c = c as usize;
        let habitat = habitat_class(&envs[c]);
        for (i, p) in communities[c].iter().enumerate() {
            let key = (Arc::as_ptr(&p.genome) as usize, habitat);
            let census = p.census(physio);
            let ne = cfg.regime.effective_size(census);
            match keys.iter().position(|&k| k == key) {
                Some(g) => {
                    let grp = &mut groups[g];
                    grp.supply_ne += ne;
                    grp.census += census;
                    grp.cells += 1;
                    let (rc, ri) = grp.rep;
                    if p.biomass > communities[rc][ri].biomass {
                        grp.rep = (c, i);
                    }
                    grp.members.push((c, i, p.biomass));
                }
                None => {
                    keys.push(key);
                    groups.push(GenotypeGroup { habitat, rep: (c, i), supply_ne: ne, census, cells: 1, members: vec![(c, i, p.biomass)] });
                }
            }
        }
    }
    for g in groups.iter_mut() {
        // Tri stable : à biomasse égale, l'ordre des cellules décide.
        g.members.sort_by(|a, b| b.2.total_cmp(&a.2));
        g.members.truncate(MAX_JUDGED_CELLS);
    }
    groups
}

/// Modification fixée pour un génotype d'un dème.
pub struct Fixation {
    pub habitat: i64,
    pub rep: (usize, usize),
    pub genome: Arc<Genome>,
    pub phenotype: Arc<Phenotype>,
    pub rates: GrowthRates,
    pub cause: GenomeChangeCause,
    pub element: ChangedElement,
    /// Coefficient de sélection du changement fixé (0 : neutre).
    pub selection: f64,
}

/// Régime « apparition puis fixation » pour un génotype : le résident est la
/// population `i` de la cellule `pops`, l'offre de mutants celle du groupe.
#[allow(clippy::too_many_arguments)]
pub fn evolve_genotype(
    communities: &[Vec<Population>],
    chemistry: &[WaterChemistry],
    envs: &[CellEnvironment],
    cfg: &WorldConfig,
    dt: f64,
    years: f64,
    group: &GenotypeGroup,
    rng: &mut impl Rng,
    accelerator_on: bool,
    complex_accelerator_on: bool,
    stats: &mut EvolutionStats,
) -> Option<(GenomeChange, Phenotype, GrowthRates, f64)> {
    let physio = &cfg.physiology;
    let evo = &cfg.evolution;
    let (rc, i) = group.rep;
    let pops = &communities[rc];
    let resident = &pops[i];
    if resident.rates.birth <= 0.0 {
        return None;
    }
    // Conditions des cellules où le mutant est jugé (la représentative
    // d'abord).
    let judged: Vec<(usize, usize, f64, evo_life::Conditions)> = group
        .members
        .iter()
        .map(|&(c, j, b)| {
            let ctx = CellContext { env: &envs[c], light_biomass_per_m2: cfg.light_biomass_per_m2 };
            (c, j, b, ctx.conditions_of(&communities[c], physio))
        })
        .collect();
    let weight: f64 = judged.iter().map(|j| j.2).sum::<f64>().max(f64::MIN_POSITIVE);
    let chem = &chemistry[rc];
    let cond = judged.iter().find(|j| (j.0, j.1) == (rc, i)).map(|j| j.3).unwrap_or_else(|| {
        let ctx = CellContext { env: &envs[rc], light_biomass_per_m2: cfg.light_biomass_per_m2 };
        ctx.conditions_of(pops, physio)
    });
    let weight_total: f64 = cfg.mutation.weights.iter().sum();
    let cell_biomass: f64 = pops.iter().map(|p| p.biomass).sum();
    let boost = if accelerator_on { evo.accelerator.boost } else { 1.0 };
    let complex_boost = if complex_accelerator_on { evo.accelerator.boost } else { 1.0 };
    let generations = dt / resident.rates.generation_time(physio);
    let ne = cfg.regime.effective_size(group.census);
    let supply = group.supply_ne.max(ne);
    let u = cfg.mutation.genomic_rate(&resident.genome);

    // Évalue un génome candidat contre la population qu'il affronterait.
    // Coefficient de sélection d'un phénotype candidat.
    let judge = |phenotype: Phenotype| -> Option<(f64, Phenotype, GrowthRates)> {
        if phenotype.signature & PATHWAY_MASK == 0 {
            return None;
        }
        // Mutation sans effet sur le phénotype (marqueur, gène inactif) :
        // neutre, sans réévaluer la croissance.
        if phenotype == *resident.phenotype {
            return Some((0.0, phenotype, resident.rates));
        }
        let rates = growth_rates(&phenotype, &cond, chem, physio);
        // Un mutant de guilde nouvelle est jugé contre la population de
        // cette guilde si elle existe déjà dans la cellule.
        let against = |pops: &[Population], own: &Population, rates: &GrowthRates| {
            let competitor = if phenotype.signature == own.signature() {
                own
            } else {
                pops.iter().find(|q| q.signature() == phenotype.signature).unwrap_or(own)
            };
            selection_coefficient(rates, &competitor.rates, physio)
        };
        if phenotype.signature != resident.signature() || judged.len() <= 1 {
            // Une guilde nouvelle naît dans la cellule représentative.
            return Some((against(pops, resident, &rates), phenotype, rates));
        }
        // Même guilde : le mutant remplacera le génotype dans tout le dème ;
        // son coefficient de sélection est la moyenne, pondérée par la
        // biomasse, de ceux des cellules où il est jugé.
        let mut s = 0.0;
        for &(c, j, b, ref cnd) in &judged {
            let r = if (c, j) == (rc, i) { rates } else { growth_rates(&phenotype, cnd, &chemistry[c], physio) };
            s += b * against(&communities[c], &communities[c][j], &r);
        }
        Some((s / weight, phenotype, rates))
    };
    let evaluate = |genome: &Genome, stats: &mut EvolutionStats| -> Option<(f64, Phenotype, GrowthRates)> {
        let phenotype = Phenotype::from_genome(genome, physio);
        stats.genetic_evaluations += 1;
        judge(phenotype)
    };

    let mut best: Option<Best> = None;
    let consider = |best: &mut Option<Best>, s: f64, change: GenomeChange, phenotype: Phenotype, rates: GrowthRates| {
        if best.as_ref().is_none_or(|b| s > b.s) {
            *best = Some(Best { s, change, phenotype, rates });
        }
    };
    // Chez un sexué, les autres mutations avantageuses qui se fixeraient
    // peuvent se réunir à la meilleure (voir `transitions::recombine`).
    let sexual = resident.phenotype.sexual;
    let has_regulator = resident.genome.functional_genes().any(|g| g.domain.family == evo_genetics::DomainFamily::Regulator);
    let mut beneficial: Vec<GenomeChange> = Vec::new();

    // Candidats au tunnel : ceux qui ne se fixent pas seuls mais sont assez
    // proches de la neutralité. Seuls les premiers (borne par génotype et
    // par pas) sont tentés ; chaque essai compte alors pour
    // (candidats / essais), ce qui garde le taux de franchissement sans biais.
    let mut tunnel_pending: Vec<(GenomeChange, f64, f64)> = Vec::new();
    let mut tunnel_eligible = 0usize;
    for (k, &kind) in MUTATION_KINDS.iter().enumerate() {
        let count = evo.candidates_per_kind[k] * group.candidate_factor();
        // Sans régulateur, pas de bloc à dupliquer : la classe se confondrait
        // avec la duplication simple.
        if count == 0 || (kind == MutationKind::ModuleDuplication && !has_regulator) {
            continue;
        }
        let innovative = matches!(kind, MutationKind::DeNovo | MutationKind::DuplicationDivergence | MutationKind::ModuleDuplication);
        let kind_boost = if innovative { boost.max(complex_boost) } else { 1.0 };
        let copies = supply * u * generations * cfg.mutation.weights[k] / weight_total / count as f64;
        for _ in 0..count {
            let mut change = mutate_with_kind(&resident.genome, kind, &cfg.mutation, rng);
            let Some((s, phenotype, rates)) = evaluate(&change.genome, stats) else { continue };
            if best.as_ref().is_some_and(|b| b.s >= s) && s > 0.0 {
                if sexual && kind == MutationKind::Point && fixes(&cfg.regime, s, ne, copies, kind_boost, rng).is_some() {
                    beneficial.push(change);
                }
                continue;
            }
            match fixes(&cfg.regime, s, ne, copies, kind_boost, rng) {
                Some(accelerated) => {
                    if accelerated {
                        change.cause = GenomeChangeCause::Accelerator;
                    }
                    if sexual && s > 0.0 {
                        if let Some(b) = best.as_ref().filter(|b| b.s > 0.0) {
                            beneficial.push(b.change.clone());
                        }
                    }
                    consider(&mut best, s, change, phenotype, rates);
                }
                None if evo.tunnel && s > evo.tunnel_min_selection => {
                    tunnel_eligible += 1;
                    if tunnel_pending.len() < evo.tunnel_attempts_per_genotype {
                        tunnel_pending.push((change, s, copies));
                    }
                }
                None => {}
            }
        }
    }
    if tunnel_eligible > 0 {
        stats.tunnel_genotypes += 1;
        if tunnel_eligible > tunnel_pending.len() {
            stats.tunnel_capped += 1;
        }
    }
    let tunnel_weight = if tunnel_pending.is_empty() { 1.0 } else { tunnel_eligible as f64 / tunnel_pending.len() as f64 };
    for (change, s, copies) in tunnel_pending {
        // Tunnel stochastique : la lignée du premier mutant, tant qu'elle
        // survit, produit des doubles mutants.
        let second = mutate(&change.genome, &cfg.mutation, rng);
        let GenomeChangeCause::SpontaneousMutation(kind2) = second.cause else { continue };
        stats.tunnel_attempts += 1;
        // Perdre ou supprimer un gène, ou changer un marqueur, ne fait gagner
        // aucune fonction : inutile de construire le phénotype.
        if matches!(kind2, MutationKind::LossOfFunction | MutationKind::Deletion | MutationKind::NeutralMarker) {
            continue;
        }
        // Le tunnel sert à franchir une innovation à deux pièces : le double
        // mutant doit gagner une fonction que le résident n'a pas. Sinon, la
        // seconde mutation seule, bien plus fréquente, l'emporte sur ce double
        // qui traîne la première comme un poids mort. Ce critère se lit sur le
        // phénotype, avant tout calcul de croissance.
        let phenotype2 = Phenotype::from_genome(&second.genome, physio);
        stats.genetic_evaluations += 1;
        if !gains_function(&phenotype2, &resident.phenotype) {
            continue;
        }
        let Some((s2, phenotype2, rates2)) = judge(phenotype2) else { continue };
        if s2 <= 0.0 || best.as_ref().is_some_and(|b| b.s >= s2) {
            continue;
        }
        let k2 = MUTATION_KINDS.iter().position(|&x| x == kind2).unwrap_or(0);
        let mu2 = u * cfg.mutation.weights[k2] / weight_total;
        let p2 = fixation_probability(s2, ne, 1.0 / ne);
        let p1 = tunnel_probability((-s).max(0.0), mu2, p2);
        if rng.random::<f64>() < OriginFixation::any_fixes(p1, copies * tunnel_weight) {
            stats.tunnel_successes += 1;
            let double = GenomeChange { genome: second.genome, cause: change.cause, element: ChangedElement::Several };
            consider(&mut best, s2, double, phenotype2, rates2);
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
            let Some((s, phenotype, rates)) = evaluate(&change.genome, stats) else { continue };
            if best.as_ref().is_some_and(|b| b.s >= s) {
                continue;
            }
            let copies = supply * evo.hgt_rate * generations * donor.biomass / cell_biomass / evo.hgt_candidates as f64;
            if let Some(accelerated) = fixes(&cfg.regime, s, ne, copies, boost, rng) {
                if accelerated {
                    change.cause = GenomeChangeCause::Accelerator;
                }
                consider(&mut best, s, change, phenotype, rates);
            }
        }
    }
    // Endosymbiose : un phagotrophe garde une proie englobée.
    if resident.phenotype.engulfment > 0.0 {
        // Les rétentions se comptent sur l'effectif réel (pas sur l'effectif
        // efficace plafonné) : un événement rare dont le nombre ne dépend
        // pas de la résolution de la grille.
        let partners = transitions::engulfed_partners(resident, i, pops, group.census, dt, years, cfg, rng);
        for partner in partners {
            let Some((s, phenotype, rates)) = evaluate(&partner.change.genome, stats) else { continue };
            if best.as_ref().is_some_and(|b| b.s >= s) {
                continue;
            }
            if let Some(accelerated) = fixes(&cfg.regime, s, ne, partner.copies, complex_boost, rng) {
                let mut change = partner.change;
                if accelerated {
                    change.cause = GenomeChangeCause::Accelerator;
                }
                consider(&mut best, s, change, phenotype, rates);
            }
        }
    }
    // Sexe : la meilleure mutation se recombine avec les autres avantageuses.
    if sexual && !beneficial.is_empty() {
        if let Some(b) = best.as_ref().filter(|b| b.s > 0.0) {
            if let Some(change) = transitions::recombine(&resident.genome, &b.change, &beneficial) {
                if let Some((s, phenotype, rates)) = evaluate(&change.genome, stats) {
                    if s > b.s {
                        stats.recombinations += 1;
                        best = Some(Best { s, change, phenotype, rates });
                    }
                }
            }
        }
    }
    best.map(|b| (b.change, b.phenotype, b.rates, b.s))
}

/// Évolution d'un dème : un tirage par génotype, dans sa cellule
/// représentative. Ne modifie rien : les fixations sont appliquées ensuite,
/// dans l'ordre des dèmes.
#[allow(clippy::too_many_arguments)]
pub fn evolve_deme(
    deme: usize,
    cells: &[u32],
    communities: &[Vec<Population>],
    chemistry: &[WaterChemistry],
    envs: &[CellEnvironment],
    cfg: &WorldConfig,
    dt: f64,
    years: f64,
    step_index: u64,
    round: u64,
    accelerator_on: bool,
    complex_accelerator_on: bool,
) -> (Vec<Fixation>, EvolutionStats) {
    let mut stats = EvolutionStats::default();
    let mut out = Vec::new();
    for group in genotype_groups(cells, communities, envs, cfg) {
        let (c, i) = group.rep;
        // Le premier tour garde le tirage d'un pas à un seul tour.
        let mut rng = if round == 0 {
            rng_for(cfg.seed, Stream::Mutation, &[step_index, deme as u64, c as u64, i as u64])
        } else {
            rng_for(cfg.seed, Stream::Mutation, &[step_index, deme as u64, c as u64, i as u64, round])
        };
        if let Some((change, phenotype, rates, selection)) = evolve_genotype(
            communities,
            chemistry,
            envs,
            cfg,
            dt,
            years,
            &group,
            &mut rng,
            accelerator_on,
            complex_accelerator_on,
            &mut stats,
        ) {
            let GenomeChange { genome, cause, element } = change;
            out.push(Fixation {
                habitat: group.habitat,
                rep: group.rep,
                genome: Arc::new(genome),
                phenotype: Arc::new(phenotype),
                rates,
                cause,
                element,
                selection,
            });
        }
    }
    (out, stats)
}
