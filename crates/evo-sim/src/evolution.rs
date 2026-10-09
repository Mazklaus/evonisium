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
use evo_genetics::genome::MARKER_LEN;
use evo_genetics::popgen::fixation_probability;
use evo_genetics::{
    mutate, mutate_with_kind, poisson, transfer_gene, tunnel_probability, ChangedElement, Gene, Genome, GenomeChange, GenomeChangeCause,
    MutationKind, OriginFixation, GENOME_CHANGE_CAUSE_COUNT, MUTATION_KINDS, MUTATION_KIND_COUNT,
};
use evo_life::community::{CellContext, Population};
use evo_life::growth::growth_rates_with;
use evo_life::metabolism::photosynthesis_stage;
use evo_life::metabolism::{AEROBIC_RESPIRATION, ANOXYGENIC_CENTRES, FERMENTATION, REACTIONS, REACTION_COUNT};
use evo_life::phenotype::{thermal_factor, Basis, Capacities, Enzyme, PATHWAY_MASK};
use evo_life::{selection_coefficient, GrowthRates, Phenotype, Physiology};
use evo_planet::CellEnvironment;
use evo_planet::WaterChemistry;
use rand::Rng;
use std::sync::Arc;

/// Réglages de l'évolution.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvolutionParams {
    /// Candidats évalués par tour, par classe de mutation (ordre de
    /// [`MUTATION_KINDS`]). Pour les classes courantes, l'offre de mutants
    /// est immense (de l'ordre de 10¹² par génotype et par tour) : chaque
    /// candidat en représente une part égale. Pour les classes innovantes
    /// (de novo, duplication suivie de divergence), c'est le nombre au plus
    /// de mutants innovants évalués quand le tirage en donne davantage.
    pub candidates_per_kind: [usize; MUTATION_KIND_COUNT],
    /// Probabilité qu'une naissance de gène (de novo) ou qu'une copie
    /// divergente porte une fonction nouvelle utilisable. C'est la
    /// difficulté des voies nouvelles : une spécificité nouvelle demande
    /// plusieurs changements précis, que le génome à un domaine par gène
    /// résume en une mutation. Calibrée sur la chronologie terrestre
    /// (docs/etape-4-chronologie.md). [Simplification signalée]
    pub innovation_probability: f64,
    /// Probabilité qu'une duplication suivie de divergence donne un gène de
    /// structure ou de régulation fonctionnel (cytosquelette, adhésion,
    /// signal, régulateur, méiose). [Simplification] calée sur la
    /// chronologie terrestre des premières colonies.
    pub structural_probability: f64,
    /// Tunnel stochastique (Weissman et coll., 2009) : tenté pour les
    /// mutants innovants qui ne se fixent pas seuls, quand leur coefficient
    /// de sélection dépasse ce seuil. Le taux de franchissement est calculé
    /// analytiquement et le nombre de franchissements tiré selon une loi de
    /// Poisson.
    pub tunnel: bool,
    pub tunnel_min_selection: f64,
    /// Transferts horizontaux reçus par génome et par génération, et
    /// candidats évalués par population et par pas.
    pub hgt_rate: f64,
    pub hgt_candidates: usize,
    /// Effectif efficace au plus par m² d'eau habitée : l'offre de mutants
    /// d'un génotype suit l'aire qu'il occupe, quelle que soit la finesse de
    /// la grille (2·10⁻³ : 10⁸ par cellule du vivant de 50 000 km²).
    pub ne_per_m2: f64,
    pub accelerator: AcceleratorParams,
    /// Durée au plus d'un tour « apparition puis fixation », années : un pas
    /// plus long enchaîne plusieurs tours, pour que le nombre de
    /// substitutions par génotype ne dépende pas de la durée du pas
    /// (`None` : un tour par pas, quelle que soit sa durée). Le test
    /// d'équivalence des pas (docs/etape-3-equivalence.md) fixe 100 ka.
    pub round_years: Option<f64>,
    /// Fixations multiples : tous les candidats qui se fixent au même tour
    /// (chacun selon sa probabilité, ce qui fait une loi de Poisson quand
    /// l'offre n'est pas saturée) sont réunis en un génome, s'ils touchent
    /// des gènes distincts et si leur réunion vaut au moins le meilleur seul.
    /// Sans cela, seul le meilleur se fixe à chaque tour.
    #[serde(default)]
    pub multiple_fixations: bool,
}

impl Default for EvolutionParams {
    fn default() -> Self {
        Self {
            candidates_per_kind: [4, 1, 1, 1, 1, 2, 2, 1],
            innovation_probability: 1e-13,
            structural_probability: 1e-13,
            tunnel: true,
            tunnel_min_selection: -0.05,
            hgt_rate: 1e-7,
            hgt_candidates: 1,
            ne_per_m2: 2e-3,
            accelerator: AcceleratorParams::default(),
            round_years: Some(100_000.0),
            multiple_fixations: true,
        }
    }
}

/// Classe innovante : naissance d'un gène, ou copie qui diverge vers une
/// famille apparentée.
pub fn is_innovative(kind: MutationKind) -> bool {
    matches!(kind, MutationKind::DeNovo | MutationKind::DuplicationDivergence)
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
    /// Quand l'accélérateur de la complexité agit, son effet sur les
    /// rétentions d'endosymbiotes est multiplié par 10 à chaque durée
    /// écoulée sans nouvelle étape, années.
    pub complexity_escalation_years: f64,
    /// Plafond de cet effet sur les rétentions.
    pub complexity_max_boost: f64,
}

impl Default for AcceleratorParams {
    fn default() -> Self {
        Self {
            enabled: true,
            patience_years: 600e6,
            boost: 100.0,
            complexity_patience_years: 1.0e9,
            complexity_escalation_years: 100e6,
            complexity_max_boost: 1e12,
        }
    }
}

/// Compteurs de l'évolution d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvolutionStats {
    pub substitutions: u64,
    pub genetic_evaluations: u64,
    pub tunnel_attempts: u64,
    pub tunnel_successes: u64,
    /// Mutants innovants apparus (tirage de Poisson), dont ceux que
    /// l'accélérateur a ajoutés, et ceux qui ont été évalués.
    pub innovations_drawn: u64,
    pub innovations_accelerated: u64,
    pub innovations_evaluated: u64,
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
        self.innovations_drawn += o.innovations_drawn;
        self.innovations_accelerated += o.innovations_accelerated;
        self.innovations_evaluated += o.innovations_evaluated;
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

/// Sommes des capacités et des affinités des enzymes d'un phénotype à une
/// température, par voie, avant normalisation (mêmes opérations, dans le
/// même ordre, que [`Phenotype::capacities`]). Celles du résident sont
/// calculées une fois par génotype et par cellule jugée ; celles d'un mutant
/// s'en déduisent en retirant les enzymes qu'il a perdues ou changées et en
/// ajoutant les siennes.
#[derive(Clone, Copy, Debug)]
pub struct RawCapacities {
    /// Sommes brutes (sans l'échelle de la cellule) : enzymes qui passent par
    /// la membrane de l'hôte, et voies lumineuses des organites.
    membrane: [f64; REACTION_COUNT],
    membrane_aff: [f64; REACTION_COUNT],
    inner: [f64; REACTION_COUNT],
    inner_aff: [f64; REACTION_COUNT],
    count: [u32; REACTION_COUNT],
    /// Digestion des proies : fermentation (cytoplasme), respiration de
    /// l'hôte et des organites.
    ferment: f64,
    respire_membrane: f64,
    respire_inner: f64,
}

impl RawCapacities {
    pub fn of(enzymes: &[Enzyme], t: f64, physio: &Physiology) -> Self {
        let z = [0.0; REACTION_COUNT];
        let mut raw = Self {
            membrane: z,
            membrane_aff: z,
            inner: z,
            inner_aff: z,
            count: [0; REACTION_COUNT],
            ferment: 0.0,
            respire_membrane: 0.0,
            respire_inner: 0.0,
        };
        for e in enzymes {
            raw.add(e, t, physio, 1.0);
        }
        raw
    }

    fn add(&mut self, e: &Enzyme, t: f64, physio: &Physiology, sign: f64) {
        let r = e.reaction as usize;
        let c = e.efficiency * thermal_factor(t, e.t_opt_k, e.t_width_k, physio);
        // Comme `Phenotype::capacities` : ce qui puise dans l'eau traverse la
        // membrane de l'hôte ; la lumière d'un plaste est captée à
        // l'intérieur.
        if e.internal && REACTIONS[r].is_light() {
            self.inner[r] += sign * c;
            self.inner_aff[r] += sign * c * e.affinity;
        } else {
            self.membrane[r] += sign * c;
            self.membrane_aff[r] += sign * c * e.affinity;
        }
        if e.reaction == FERMENTATION {
            self.ferment += sign * c;
        } else if e.reaction == AEROBIC_RESPIRATION {
            if e.internal {
                self.respire_inner += sign * c;
            } else {
                self.respire_membrane += sign * c;
            }
        }
        if sign > 0.0 {
            self.count[r] += 1;
        } else {
            self.count[r] -= 1;
        }
    }

    /// Capacités d'un mutant dont les enzymes sont `mutant`, le résident
    /// (dont `self` est la somme) ayant `resident`. Les enzymes communes en
    /// tête et en queue de liste ne sont pas recalculées.
    pub fn mutant(&self, resident: &[Enzyme], mutant: &Phenotype, t: f64, physio: &Physiology) -> Capacities {
        let enzymes = &mutant.enzymes;
        let mut head = 0;
        while head < resident.len() && head < enzymes.len() && resident[head] == enzymes[head] {
            head += 1;
        }
        let mut tail = 0;
        while tail < resident.len() - head
            && tail < enzymes.len() - head
            && resident[resident.len() - 1 - tail] == enzymes[enzymes.len() - 1 - tail]
        {
            tail += 1;
        }
        let mut raw = *self;
        for e in &resident[head..resident.len() - tail] {
            raw.add(e, t, physio, -1.0);
        }
        for e in &enzymes[head..enzymes.len() - tail] {
            raw.add(e, t, physio, 1.0);
        }
        raw.finish(mutant.cell_size, physio)
    }

    /// Capacités normalisées d'une cellule de taille `cell_size`, comme
    /// [`Phenotype::capacities`].
    pub fn finish(&self, cell_size: f64, physio: &Physiology) -> Capacities {
        let membrane = 1.0 / cell_size;
        let inner = physio.organelle_scale;
        let mut cap = [0.0; REACTION_COUNT];
        let mut affinity = [1.0; REACTION_COUNT];
        for r in 0..REACTION_COUNT {
            let c = self.membrane[r] * membrane + self.inner[r] * inner;
            if self.count[r] > 0 && c > 0.0 {
                cap[r] = c;
                affinity[r] = (self.membrane_aff[r] * membrane + self.inner_aff[r] * inner) / c;
            }
        }
        let partner = ANOXYGENIC_CENTRES.iter().map(|&r| cap[r as usize].min(1.0)).fold(0.0, f64::max);
        Capacities {
            cap,
            affinity,
            partner,
            digest_fermentation: self.ferment,
            digest_respiration: self.respire_membrane * membrane + self.respire_inner * inner,
        }
    }
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
    /// Offre de mutants : somme des effectifs efficaces de chaque cellule
    /// (chacun plafonné selon l'aire d'eau de la cellule).
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
            let ne = census.min(cfg.evolution.ne_per_m2 * envs[c].water_area_m2).max(1.0);
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
    /// Changements fixés au même tour et réunis avec le principal : cause,
    /// élément touché (rang dans le génome du résident), et vrai s'il était
    /// avantageux seul.
    pub extra: Vec<(GenomeChangeCause, ChangedElement, bool)>,
}

/// Fixation retenue pour un génotype au cours d'un tour.
pub struct Fixed {
    pub change: GenomeChange,
    pub phenotype: Phenotype,
    pub rates: GrowthRates,
    pub selection: f64,
    pub extra: Vec<(GenomeChangeCause, ChangedElement, bool)>,
}

/// Réunit en un génome le changement principal et ceux, fixés au même tour,
/// qui touchent des gènes distincts (une suppression ou une modification
/// par gène du résident, des insertions en nombre quelconque, un changement
/// par site du marqueur). Tous dérivent du même résident. Renvoie le génome
/// réuni et les rangs des changements retenus, le principal compris.
pub fn combine_changes(resident: &Genome, changes: &[&GenomeChange]) -> Option<(GenomeChange, Vec<usize>)> {
    let first = changes.first()?;
    let n = resident.genes.len();
    // Par gène du résident : remplacé (Some(Some)), supprimé (Some(None)).
    let mut edits: Vec<Option<Option<Gene>>> = vec![None; n];
    // Gènes insérés avant le gène de même rang du résident (n : à la fin).
    let mut inserts: Vec<Vec<Gene>> = vec![Vec::new(); n + 1];
    let mut marker = resident.marker;
    let mut marked = [false; MARKER_LEN];
    let mut taken = Vec::new();
    for (k, ch) in changes.iter().enumerate() {
        let ok = match ch.element {
            ChangedElement::Gene { index, .. } => {
                let i = index as usize;
                let free = i < n && edits[i].is_none() && i < ch.genome.genes.len();
                if free {
                    edits[i] = Some(Some(ch.genome.genes[i]));
                }
                free
            }
            ChangedElement::Inserted { index, .. } => {
                let i = index as usize;
                let free = i <= n && i < ch.genome.genes.len();
                if free {
                    inserts[i].push(ch.genome.genes[i]);
                }
                free
            }
            ChangedElement::Removed { index, .. } => {
                let i = index as usize;
                let free = i < n && edits[i].is_none();
                if free {
                    edits[i] = Some(None);
                }
                free
            }
            ChangedElement::Marker { site } => {
                let i = site as usize;
                let free = i < MARKER_LEN && !marked[i];
                if free {
                    marked[i] = true;
                    marker[i] = ch.genome.marker[i];
                }
                free
            }
            // Plusieurs éléments, organite, bloc : jamais réunis.
            _ => false,
        };
        if ok {
            taken.push(k);
        } else if k == 0 {
            return None;
        }
    }
    let change = resident.derive(first.cause, |g| {
        let mut genes = Vec::with_capacity(n + 1);
        for (i, edit) in edits.iter().enumerate() {
            genes.extend_from_slice(&inserts[i]);
            match edit {
                None => genes.push(resident.genes[i]),
                Some(Some(gene)) => genes.push(*gene),
                Some(None) => {}
            }
        }
        genes.extend_from_slice(&inserts[n]);
        g.genes = genes;
        g.marker = marker;
        first.element
    });
    Some((change, taken))
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
    complex_boost: f64,
    stats: &mut EvolutionStats,
) -> Option<Fixed> {
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
    let judged: Vec<(usize, usize, f64, evo_life::Conditions, RawCapacities)> = group
        .members
        .iter()
        .map(|&(c, j, b)| {
            let ctx = CellContext { env: &envs[c], light_biomass_per_m2: cfg.light_biomass_per_m2 };
            let cnd = ctx.conditions_of(&communities[c], physio);
            (c, j, b, cnd, RawCapacities::of(&resident.phenotype.enzymes, cnd.temperature_k, physio))
        })
        .collect();
    let weight: f64 = judged.iter().map(|j| j.2).sum::<f64>().max(f64::MIN_POSITIVE);
    let chem = &chemistry[rc];
    let (cond, base) = judged.iter().find(|j| (j.0, j.1) == (rc, i)).map(|j| (j.3, j.4)).unwrap_or_else(|| {
        let ctx = CellContext { env: &envs[rc], light_biomass_per_m2: cfg.light_biomass_per_m2 };
        let cnd = ctx.conditions_of(pops, physio);
        (cnd, RawCapacities::of(&resident.phenotype.enzymes, cnd.temperature_k, physio))
    });
    let weight_total: f64 = cfg.mutation.weights.iter().sum();
    let cell_biomass: f64 = pops.iter().map(|p| p.biomass).sum();
    let boost = if accelerator_on { evo.accelerator.boost } else { 1.0 };
    // Les rétentions d'endosymbiotes reçoivent l'effet entier (qui croît
    // tant que la complexité stagne) ; les mutations, celui de l'accélérateur.
    let retention_boost = complex_boost;
    let complex_boost = complex_boost.min(evo.accelerator.boost);
    let regime = &cfg.regime;
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
        // Capacités du mutant à partir de celles du résident, mises en cache
        // par génotype et par cellule : seules les enzymes qui diffèrent sont
        // recalculées.
        let rates_in = |cnd: &evo_life::Conditions, base: &RawCapacities, chem: &WaterChemistry| {
            let caps = base.mutant(&resident.phenotype.enzymes, &phenotype, cnd.temperature_k, physio);
            growth_rates_with(&phenotype, &caps, cnd, chem, physio)
        };
        let rates = rates_in(&cond, &base, chem);
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
        for &(c, j, b, ref cnd, ref raw) in &judged {
            let r = if (c, j) == (rc, i) { rates } else { rates_in(cnd, raw, &chemistry[c]) };
            s += b * against(&communities[c], &communities[c][j], &r);
        }
        Some((s / weight, phenotype, rates))
    };
    // Construction incrémentale : la base du résident sert à tous ses
    // mutants ponctuels.
    let basis = Basis::new(&resident.genome, physio);
    let evaluate = |change: &GenomeChange, stats: &mut EvolutionStats| -> Option<(f64, Phenotype, GrowthRates)> {
        let genome = &change.genome;
        let unchanged = match change.element {
            ChangedElement::Marker { .. } => true,
            ChangedElement::Gene { index, .. } => {
                genome.genes.len() == resident.genome.genes.len()
                    && !genome.genes[index as usize].functional
                    && !resident.genome.genes[index as usize].functional
            }
            _ => false,
        };
        let phenotype = if unchanged {
            (*resident.phenotype).clone()
        } else {
            match basis.derive(&resident.genome, genome, change.element, physio) {
                Some(b) => Phenotype::assemble(genome, &b, physio),
                None => Phenotype::from_genome(genome, physio),
            }
        };
        stats.genetic_evaluations += 1;
        judge(phenotype)
    };

    // Candidats qui se fixent au cours du tour ; sans fixations multiples,
    // seul le meilleur est gardé.
    let multiple = evo.multiple_fixations;
    let mut fixers: Vec<Best> = Vec::new();
    let best_s = |fixers: &[Best]| fixers.iter().map(|b| b.s).fold(f64::NEG_INFINITY, f64::max);
    let consider = |fixers: &mut Vec<Best>, s: f64, change: GenomeChange, phenotype: Phenotype, rates: GrowthRates| {
        if multiple {
            fixers.push(Best { s, change, phenotype, rates });
        } else if fixers.first().is_none_or(|b| s > b.s) {
            fixers.clear();
            fixers.push(Best { s, change, phenotype, rates });
        }
    };
    // Chez un sexué, les autres mutations avantageuses qui se fixeraient
    // peuvent se réunir à la meilleure (voir `transitions::recombine`).
    let sexual = resident.phenotype.sexual;
    let has_regulator = resident.genome.functional_genes().any(|g| g.domain.family == evo_genetics::DomainFamily::Regulator);
    let mut beneficial: Vec<GenomeChange> = Vec::new();

    // Mutants des classes courantes : l'offre est immense, chaque candidat
    // représente une part égale des mutants de sa classe. Mutants innovants :
    // leur nombre est tiré selon une loi de Poisson (apparition × probabilité
    // d'une fonction nouvelle), et chacun est évalué tant qu'ils restent peu
    // nombreux. Ceux que l'accélérateur ajoute sont tirés à part.
    let mut tunnel_pending: Vec<(GenomeChange, f64, f64)> = Vec::new();
    for (k, &kind) in MUTATION_KINDS.iter().enumerate() {
        // Sans régulateur, pas de bloc à dupliquer : la classe se confondrait
        // avec la duplication simple.
        if kind == MutationKind::ModuleDuplication && !has_regulator {
            continue;
        }
        // L'accélérateur des transitions de la complexité agit aussi sur
        // les mutations innovantes.
        let boost = if is_innovative(kind) { boost.max(complex_boost) } else { boost };
        let arising = supply * u * generations * cfg.mutation.weights[k] / weight_total;
        // Une copie qui diverge vers une famille de structure ou de
        // régulation (cytosquelette, adhésion, signal, régulateur, méiose)
        // n'invente pas de chimie : des homologues existent chez les
        // procaryotes (FtsZ et MreB pour l'actine et la tubuline, systèmes à
        // deux composants, recombinases). Elle a sa propre probabilité
        // (`structural_probability`), bien plus forte que celle des voies
        // nouvelles ; ces copies sont tirées selon une loi de Poisson et le
        // génome n'est copié que pour elles.
        if kind == MutationKind::DuplicationDivergence {
            let lambda = arising * evo.structural_probability;
            let natural = poisson(lambda, rng);
            let extra = if complex_boost > 1.0 { poisson(lambda * (complex_boost - 1.0), rng) } else { 0 };
            let n = natural + extra;
            let count = (n as usize).min(evo.candidates_per_kind[k].max(1) * group.candidate_factor());
            for _ in 0..count {
                // La famille d'arrivée suit la table de parenté : on tire
                // jusqu'à une famille de structure (au plus quelques essais).
                let drawn = (0..8).find_map(|_| {
                    evo_genetics::divergent_copy(&resident.genome, &cfg.mutation, rng).filter(|(_, g)| g.domain.family.is_cellular())
                });
                let Some((i, copy)) = drawn else { continue };
                let mut change = evo_genetics::insert_copy(&resident.genome, i, copy);
                let Some((s, phenotype, rates)) = evaluate(&change, stats) else { continue };
                if !multiple && best_s(&fixers) >= s {
                    continue;
                }
                if regime.candidate_fixes(s, ne, n as f64 / count as f64, rng) {
                    if extra > 0 && rng.random::<f64>() < extra as f64 / n as f64 {
                        change.cause = GenomeChangeCause::Accelerator;
                    }
                    consider(&mut fixers, s, change, phenotype, rates);
                }
            }
        }
        let (count, copies, accelerated) = if is_innovative(kind) {
            let lambda = arising * evo.innovation_probability;
            let natural = poisson(lambda, rng);
            let extra = if boost > 1.0 { poisson(lambda * (boost - 1.0), rng) } else { 0 };
            let n = natural + extra;
            stats.innovations_drawn += n;
            stats.innovations_accelerated += extra;
            if n == 0 {
                continue;
            }
            let count = (n as usize).min(evo.candidates_per_kind[k].max(1));
            stats.innovations_evaluated += count as u64;
            (count, n as f64 / count as f64, extra as f64 / n as f64)
        } else {
            let count = evo.candidates_per_kind[k] * group.candidate_factor();
            if count == 0 {
                continue;
            }
            (count, arising / count as f64, 0.0)
        };
        for _ in 0..count {
            let mut change = mutate_with_kind(&resident.genome, kind, &cfg.mutation, rng);
            // Un marqueur neutre ne change pas le phénotype : neutre, sans
            // le reconstruire.
            let judged = if kind == MutationKind::NeutralMarker {
                Some((0.0, (*resident.phenotype).clone(), resident.rates))
            } else {
                evaluate(&change, stats)
            };
            let Some((s, phenotype, rates)) = judged else { continue };
            if !multiple && best_s(&fixers) >= s && s > 0.0 {
                if sexual && kind == MutationKind::Point && regime.candidate_fixes(s, ne, copies, rng) {
                    beneficial.push(change);
                }
                continue;
            }
            if regime.candidate_fixes(s, ne, copies, rng) {
                if accelerated > 0.0 && rng.random::<f64>() < accelerated {
                    change.cause = GenomeChangeCause::Accelerator;
                }
                if !multiple && sexual && s > 0.0 {
                    if let Some(b) = fixers.first().filter(|b| b.s > 0.0) {
                        beneficial.push(b.change.clone());
                    }
                }
                consider(&mut fixers, s, change, phenotype, rates);
            } else if is_innovative(kind) && evo.tunnel && s > evo.tunnel_min_selection {
                tunnel_pending.push((change, s, copies));
            }
        }
    }
    for (change, s, copies) in tunnel_pending {
        // Tunnel stochastique : la lignée du mutant innovant, tant qu'elle
        // survit, produit des doubles mutants ; la seconde mutation est tirée
        // parmi toutes les classes, et représente sa classe.
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
        if s2 <= 0.0 || best_s(&fixers) >= s2 {
            continue;
        }
        // Taux de la seconde mutation : celui de sa classe, une classe
        // innovante ne donnant une fonction nouvelle qu'avec sa probabilité.
        let k2 = MUTATION_KINDS.iter().position(|&x| x == kind2).unwrap_or(0);
        let innovation = if is_innovative(kind2) { evo.innovation_probability } else { 1.0 };
        let mu2 = u * cfg.mutation.weights[k2] / weight_total * innovation;
        let p2 = fixation_probability(s2, ne, 1.0 / ne);
        // Probabilité qu'une lignée intermédiaire franchisse la vallée
        // (Weissman et coll., 2009), puis nombre de franchissements parmi les
        // `copies` lignées, selon une loi de Poisson : au moins un suffit.
        let p1 = tunnel_probability((-s).max(0.0), mu2, p2);
        if poisson(copies * p1, rng) > 0 {
            stats.tunnel_successes += 1;
            let double = GenomeChange { genome: second.genome, cause: change.cause, element: ChangedElement::Several };
            consider(&mut fixers, s2, double, phenotype2, rates2);
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
            let Some((s, phenotype, rates)) = evaluate(&change, stats) else { continue };
            if !multiple && best_s(&fixers) >= s {
                continue;
            }
            let copies = supply * evo.hgt_rate * generations * donor.biomass / cell_biomass / evo.hgt_candidates as f64;
            if let Some(accelerated) = fixes(&cfg.regime, s, ne, copies, boost, rng) {
                if accelerated {
                    change.cause = GenomeChangeCause::Accelerator;
                }
                consider(&mut fixers, s, change, phenotype, rates);
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
            let Some((s, phenotype, rates)) = evaluate(&partner.change, stats) else { continue };
            if !multiple && best_s(&fixers) >= s {
                continue;
            }
            if let Some(accelerated) = fixes(&cfg.regime, s, ne, partner.copies, retention_boost, rng) {
                let mut change = partner.change;
                if accelerated {
                    change.cause = GenomeChangeCause::Accelerator;
                }
                consider(&mut fixers, s, change, phenotype, rates);
            }
        }
    }
    // Sexe : la meilleure mutation se recombine avec les autres avantageuses
    // (avec les fixations multiples, leur réunion ci-dessous en tient lieu).
    if !multiple && sexual && !beneficial.is_empty() {
        let mut recombined = None;
        if let Some(b) = fixers.first().filter(|b| b.s > 0.0) {
            if let Some(change) = transitions::recombine(&resident.genome, &b.change, &beneficial) {
                if let Some((s, phenotype, rates)) = evaluate(&change, stats) {
                    if s > b.s {
                        stats.recombinations += 1;
                        recombined = Some(Best { s, change, phenotype, rates });
                    }
                }
            }
        }
        if let Some(r) = recombined {
            fixers = vec![r];
        }
    }
    // Le meilleur d'abord ; à égalité, l'ordre des tirages.
    fixers.sort_by(|a, b| b.s.total_cmp(&a.s));
    let mut fixers = fixers.into_iter();
    let first = fixers.next()?;
    // Les autres candidats fixés, sauf les délétères (presque neutres), se
    // fixent à la suite du meilleur : on les réunit en un génome, jugé une
    // fois, retenu s'il vaut au moins le meilleur seul et garde sa guilde.
    let others: Vec<Best> = fixers.filter(|b| b.s >= 0.0).collect();
    if !others.is_empty() {
        let changes: Vec<&GenomeChange> = std::iter::once(&first.change).chain(others.iter().map(|b| &b.change)).collect();
        if let Some((change, taken)) = combine_changes(&resident.genome, &changes) {
            if taken.len() > 1 {
                // Plusieurs changements : le phénotype se construit en entier
                // (la construction incrémentale ne suit qu'un élément).
                stats.genetic_evaluations += 1;
                if let Some((s, phenotype, rates)) = judge(Phenotype::from_genome(&change.genome, physio)) {
                    if s >= first.s && phenotype.signature == first.phenotype.signature {
                        let extra = taken[1..]
                            .iter()
                            .map(|&k| {
                                let o = &others[k - 1];
                                (o.change.cause, o.change.element, o.s > 0.0)
                            })
                            .collect();
                        return Some(Fixed { change, phenotype, rates, selection: s, extra });
                    }
                }
            }
        }
    }
    Some(Fixed { change: first.change, phenotype: first.phenotype, rates: first.rates, selection: first.s, extra: Vec::new() })
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
    complex_boost: f64,
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
        if let Some(Fixed { change, phenotype, rates, selection, extra }) =
            evolve_genotype(communities, chemistry, envs, cfg, dt, years, &group, &mut rng, accelerator_on, complex_boost, &mut stats)
        {
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
                extra,
            });
        }
    }
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::rng::{rng_for, Stream};
    use evo_genetics::{Domain, DomainFamily, Gene, MutationParams};
    use evo_life::metabolism::{domain_relations, ANOXYGENIC_PHOTOSYNTHESIS, METHANOGENESIS, PHOTOFERROTROPHY};
    use evo_life::phenotype::Capacities;

    /// Les capacités d'un mutant déduites de celles du résident sont celles
    /// d'un calcul complet, aux arrondis près, pour toutes les classes de
    /// mutations ; celles du résident le sont exactement.
    #[test]
    fn cached_capacities_match_a_full_computation() {
        let physio = Physiology::default();
        let gene = |family, efficiency, t_opt_k| Gene {
            domain: Domain { family, efficiency, affinity: 1.3, t_opt_k, t_width_k: 9.0, absorption_nm: 450.0 },
            functional: true,
        };
        let mut genome = Genome::new(
            vec![
                gene(DomainFamily::Catalytic(METHANOGENESIS), 1.0, 300.0),
                gene(DomainFamily::Cytochrome, 0.4, 300.0),
                gene(DomainFamily::Catalytic(PHOTOFERROTROPHY), 0.3, 296.0),
                gene(DomainFamily::Pigment, 0.5, 300.0),
                gene(DomainFamily::Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.2, 305.0),
                gene(DomainFamily::Catalytic(METHANOGENESIS), 0.6, 290.0),
                gene(DomainFamily::Catalytic(FERMENTATION), 0.5, 299.0),
                // Une grande cellule : la membrane compte.
                gene(DomainFamily::Cytoskeleton, 0.3, 300.0),
            ],
            [0; evo_genetics::genome::MARKER_LEN],
        );
        // Un plaste et une mitochondrie : capacités internes.
        genome.organelles.push(evo_genetics::Organelle {
            genes: vec![
                gene(DomainFamily::Pigment, 0.6, 300.0),
                gene(DomainFamily::Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.5, 300.0),
                gene(DomainFamily::Catalytic(AEROBIC_RESPIRATION), 0.7, 301.0),
            ],
            origin_lineage: 1,
            acquired_years: 0.0,
        });
        let params = MutationParams { weights: [1.0; MUTATION_KIND_COUNT], relations: domain_relations(), ..Default::default() };
        let resident = Phenotype::from_genome(&genome, &physio);
        let t = 297.0;
        let base = RawCapacities::of(&resident.enzymes, t, &physio);
        let close = |a: &Capacities, b: &Capacities| {
            for r in 0..REACTION_COUNT {
                assert!((a.cap[r] - b.cap[r]).abs() < 1e-12, "capacité {r} : {} contre {}", a.cap[r], b.cap[r]);
                assert!((a.affinity[r] - b.affinity[r]).abs() < 1e-9, "affinité {r}");
            }
            assert!((a.partner - b.partner).abs() < 1e-12);
            assert!((a.digest_fermentation - b.digest_fermentation).abs() < 1e-12);
            assert!((a.digest_respiration - b.digest_respiration).abs() < 1e-12);
        };
        assert!(resident.cell_size > 1.5);
        close(&base.finish(resident.cell_size, &physio), &resident.capacities(t, &physio));
        let mut rng = rng_for(9, Stream::Validation, &[]);
        for _ in 0..2000 {
            let m = mutate(&genome, &params, &mut rng);
            let mutant = Phenotype::from_genome(&m.genome, &physio);
            let fast = base.mutant(&resident.enzymes, &mutant, t, &physio);
            close(&fast, &mutant.capacities(t, &physio));
        }
    }

    /// Les changements fixés au même tour se réunissent en un génome, rang
    /// par rang du résident ; deux changements d'un même gène ne se cumulent
    /// pas (le premier, le meilleur, l'emporte).
    #[test]
    fn changes_of_one_round_combine_gene_by_gene() {
        let gene = |family, efficiency| Gene {
            domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: 300.0, t_width_k: 9.0, absorption_nm: 450.0 },
            functional: true,
        };
        let (a, b, c) =
            (gene(DomainFamily::Catalytic(METHANOGENESIS), 1.0), gene(DomainFamily::Cytochrome, 0.4), gene(DomainFamily::Pigment, 0.5));
        let d = gene(DomainFamily::Rhodopsin, 0.05);
        let resident = Genome { genes: vec![a, b, c], marker: [0; evo_genetics::genome::MARKER_LEN], organelles: Vec::new() };
        let cause = GenomeChangeCause::SpontaneousMutation(MutationKind::Point);
        let b2 = gene(DomainFamily::Cytochrome, 0.6);
        let point = resident.derive(cause, |g| {
            g.genes[1] = b2;
            ChangedElement::Gene { index: 1, family: DomainFamily::Cytochrome }
        });
        let dup = resident.derive(cause, |g| {
            g.genes.insert(1, a);
            ChangedElement::Inserted { index: 1, family: a.domain.family }
        });
        let del = resident.derive(cause, |g| {
            g.genes.remove(2);
            ChangedElement::Removed { index: 2, family: c.domain.family }
        });
        let lof = resident.derive(cause, |g| {
            g.genes[1].functional = false;
            ChangedElement::Gene { index: 1, family: DomainFamily::Cytochrome }
        });
        let new = resident.derive(cause, |g| {
            g.genes.push(d);
            ChangedElement::Inserted { index: 3, family: d.domain.family }
        });
        let marker = resident.derive(cause, |g| {
            g.marker[5] = 2;
            ChangedElement::Marker { site: 5 }
        });
        let (combined, taken) = combine_changes(&resident, &[&point, &dup, &del, &lof, &new, &marker]).unwrap();
        assert_eq!(taken, vec![0, 1, 2, 4, 5]);
        assert_eq!(combined.genome.genes, vec![a, a, b2, d]);
        assert_eq!(combined.genome.marker[5], 2);
        assert_eq!(combined.element, point.element);
        let several = GenomeChange { element: ChangedElement::Several, ..point.clone() };
        assert!(combine_changes(&resident, &[&several, &dup]).is_none());
    }
}
