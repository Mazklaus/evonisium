//! Mutations. Aucune n'a d'effet prédéfini : elles modifient le génome, et
//! leur conséquence découle du phénotype que le chantier Organismes en tire.

use crate::genome::{
    ChangedElement, Domain, DomainFamily, DomainRelation, Gene, Genome, GenomeChange, GenomeChangeCause, ReactionId, MARKER_LEN,
};
use evo_core::math::Det;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MutationKind {
    /// Décale un paramètre d'un domaine (petit pas le plus souvent).
    Point,
    /// Le gène devient pseudogène.
    LossOfFunction,
    /// Copie d'un gène, insérée juste après l'original.
    Duplication,
    /// Retrait d'un gène.
    Deletion,
    /// Substitution dans le marqueur neutre.
    NeutralMarker,
    /// Naissance rare d'un gène catalytique pour une réaction nouvelle.
    DeNovo,
    /// Duplication suivie de divergence : la copie passe à une famille
    /// apparentée (table de parenté), avec une qualité d'abord médiocre.
    /// C'est la voie principale d'apparition des nouveaux domaines.
    DuplicationDivergence,
}

/// Nombre de classes de mutations.
pub const MUTATION_KIND_COUNT: usize = 7;

/// Taux et poids relatifs des classes de mutations.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MutationParams {
    /// Mutations par gène et par génération en l'absence de réparation.
    pub rate_per_gene: f64,
    /// Réduction maximale du taux par la réparation de l'ADN.
    pub max_repair_factor: f64,
    /// Poids relatifs : ponctuelle, perte, duplication, délétion, neutre,
    /// de novo, duplication suivie de divergence.
    pub weights: [f64; MUTATION_KIND_COUNT],
    /// Écart type des petits pas (relatif pour efficacité, affinité, largeur).
    pub small_step: f64,
    /// Écart type des grands sauts, et leur part parmi les mutations ponctuelles.
    pub large_step: f64,
    pub large_step_share: f64,
    /// Écart type des pas de température optimale, K.
    pub t_step_k: f64,
    /// Écart type des pas du pic d'absorption, nm.
    pub absorption_step_nm: f64,
    /// Nombre de réactions du catalogue (pour les gènes de novo).
    pub reaction_count: u8,
    /// Parentés entre familles, fournies par le catalogue d'Organismes.
    pub relations: Vec<DomainRelation>,
    /// Qualité d'une copie divergente, relative à l'original.
    pub divergence_quality: f64,
}

impl Default for MutationParams {
    fn default() -> Self {
        Self {
            // Avec une réparation efficace (facteur 10), un génome minimal de
            // 4 à 10 gènes mute au rythme de la règle de Drake (≈ 0,003).
            rate_per_gene: 3e-3,
            max_repair_factor: 10.0,
            // La divergence vers une famille apparentée est une partie des
            // duplications ; le de novo reste dix fois plus rare. Biais de
            // délétion des bactéries (Kuo et Ochman, 2009 ; Mira et coll.,
            // 2001) : les délétions sont plus fréquentes que les
            // duplications, ce qui borne la taille des génomes.
            weights: [0.70, 0.08, 0.03, 0.12, 0.089, 0.001, 0.01],
            small_step: 0.08,
            large_step: 0.5,
            large_step_share: 0.05,
            t_step_k: 1.5,
            absorption_step_nm: 12.0,
            reaction_count: 7,
            relations: Vec::new(),
            divergence_quality: 0.2,
        }
    }
}

impl MutationParams {
    /// Mutations attendues par génome et par génération : le taux est un trait
    /// du génome (gènes de réparation), pas une constante globale.
    pub fn genomic_rate(&self, genome: &Genome) -> f64 {
        let repair = genome.family_efficiency(DomainFamily::Repair).min(1.0);
        let factor = 1.0 + (self.max_repair_factor - 1.0) * repair;
        self.rate_per_gene * genome.genes.len().max(1) as f64 / factor
    }
}

/// Toutes les classes, dans l'ordre des poids de [`MutationParams::weights`].
pub const MUTATION_KINDS: [MutationKind; MUTATION_KIND_COUNT] = [
    MutationKind::Point,
    MutationKind::LossOfFunction,
    MutationKind::Duplication,
    MutationKind::Deletion,
    MutationKind::NeutralMarker,
    MutationKind::DeNovo,
    MutationKind::DuplicationDivergence,
];

impl MutationParams {
    /// Familles vers lesquelles une copie de `family` peut diverger.
    pub fn relatives(&self, family: DomainFamily) -> impl Iterator<Item = &DomainRelation> {
        self.relations.iter().filter(move |r| r.from == family)
    }
}

fn pick_kind(params: &MutationParams, rng: &mut impl Rng) -> MutationKind {
    const KINDS: [MutationKind; MUTATION_KIND_COUNT] = MUTATION_KINDS;
    let total: f64 = params.weights.iter().sum();
    let mut x = rng.random::<f64>() * total;
    for (k, w) in KINDS.iter().zip(params.weights) {
        if x < w {
            return *k;
        }
        x -= w;
    }
    MutationKind::Point
}

fn point_mutation(domain: &mut Domain, params: &MutationParams, rng: &mut impl Rng) {
    let large = rng.random::<f64>() < params.large_step_share;
    let sd = if large { params.large_step } else { params.small_step };
    let z = evo_core::math::standard_normal(rng);
    let scale = if large { 6.0 } else { 1.0 };
    match rng.random_range(0..5) {
        0 => domain.efficiency = (domain.efficiency * (sd * z).dexp()).max(1e-6),
        1 => domain.affinity = (domain.affinity * (sd * z).dexp()).max(1e-6),
        2 => domain.t_opt_k += z * params.t_step_k * scale,
        3 => domain.t_width_k = (domain.t_width_k * (sd * z).dexp()).clamp(0.5, 80.0),
        _ => domain.absorption_nm = (domain.absorption_nm + z * params.absorption_step_nm * scale).clamp(300.0, 1100.0),
    }
}

/// Applique une mutation tirée au hasard et renvoie le génome mutant.
pub fn mutate(genome: &Genome, params: &MutationParams, rng: &mut impl Rng) -> GenomeChange {
    let kind = pick_kind(params, rng);
    mutate_with_kind(genome, kind, params, rng)
}

/// Applique une mutation d'une classe donnée (échantillonnage stratifié par
/// classe dans le régime « apparition puis fixation »).
pub fn mutate_with_kind(genome: &Genome, kind: MutationKind, params: &MutationParams, rng: &mut impl Rng) -> GenomeChange {
    let mut kind = kind;
    if genome.genes.is_empty() && kind != MutationKind::NeutralMarker {
        kind = MutationKind::DeNovo;
    }
    genome.derive(GenomeChangeCause::SpontaneousMutation(kind), |g| apply(g, kind, params, rng))
}

/// Applique une seconde mutation à un génome déjà muté (double mutant du
/// tunnel stochastique) ; la cause reste celle de la première.
pub fn mutate_again(first: &GenomeChange, params: &MutationParams, rng: &mut impl Rng) -> GenomeChange {
    let second = mutate(&first.genome, params, rng);
    GenomeChange { genome: second.genome, cause: first.cause, element: ChangedElement::Several }
}

fn apply(g: &mut Genome, kind: MutationKind, params: &MutationParams, rng: &mut impl Rng) -> ChangedElement {
    let gene_at = |g: &Genome, i: usize| ChangedElement::Gene { index: i as u16, family: g.genes[i].domain.family };
    match kind {
        MutationKind::Point => {
            let i = rng.random_range(0..g.genes.len());
            point_mutation(&mut g.genes[i].domain, params, rng);
            gene_at(g, i)
        }
        MutationKind::LossOfFunction => {
            let i = rng.random_range(0..g.genes.len());
            g.genes[i].functional = false;
            gene_at(g, i)
        }
        MutationKind::Duplication => {
            let i = rng.random_range(0..g.genes.len());
            let copy = g.genes[i];
            g.genes.insert(i + 1, copy);
            ChangedElement::Inserted { index: (i + 1) as u16, family: copy.domain.family }
        }
        MutationKind::Deletion => {
            let i = rng.random_range(0..g.genes.len());
            let family = g.genes.remove(i).domain.family;
            ChangedElement::Removed { index: i as u16, family }
        }
        MutationKind::NeutralMarker => {
            let i = rng.random_range(0..MARKER_LEN);
            g.marker[i] = (g.marker[i] + rng.random_range(1..4)) % 4;
            ChangedElement::Marker { site: i as u16 }
        }
        MutationKind::DuplicationDivergence => {
            let i = rng.random_range(0..g.genes.len());
            let mut copy = g.genes[i];
            let total: f64 = params.relatives(copy.domain.family).map(|r| r.weight).sum();
            if total > 0.0 {
                let mut x = rng.random::<f64>() * total;
                for r in params.relatives(copy.domain.family) {
                    if x < r.weight {
                        copy.domain.family = r.to;
                        break;
                    }
                    x -= r.weight;
                }
                copy.domain.efficiency *= params.divergence_quality;
            }
            // Sans parent déclaré, c'est une simple duplication.
            g.genes.insert(i + 1, copy);
            ChangedElement::Inserted { index: (i + 1) as u16, family: copy.domain.family }
        }
        MutationKind::DeNovo => {
            let n = g.genes.len().max(1) as f64;
            let t_opt = g.genes.iter().map(|x| x.domain.t_opt_k).sum::<f64>() / n;
            let reaction: ReactionId = rng.random_range(0..params.reaction_count);
            let family = match rng.random_range(0..10) {
                0 => DomainFamily::OxidativeDefense,
                1 => DomainFamily::Pigment,
                2 => DomainFamily::Rhodopsin,
                _ => DomainFamily::Catalytic(reaction),
            };
            let t_opt = if t_opt.is_finite() && t_opt > 0.0 { t_opt } else { 300.0 };
            g.genes.push(Gene {
                domain: Domain { family, efficiency: 0.05, affinity: 0.5, t_opt_k: t_opt, t_width_k: 10.0, absorption_nm: 500.0 },
                functional: true,
            });
            ChangedElement::Inserted { index: (g.genes.len() - 1) as u16, family }
        }
    }
}

/// Insère dans `recipient` une copie d'un gène venu d'une autre lignée
/// (transfert horizontal, ou accélérateur quand celui-ci en est la cause).
pub fn transfer_gene(recipient: &Genome, gene: Gene, cause: GenomeChangeCause) -> GenomeChange {
    recipient.derive(cause, |g| {
        g.genes.push(gene);
        ChangedElement::Inserted { index: (g.genes.len() - 1) as u16, family: gene.domain.family }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::rng::{rng_for, Stream};

    fn sample_genome() -> Genome {
        let d = Domain {
            family: DomainFamily::Catalytic(0),
            efficiency: 1.0,
            affinity: 1.0,
            t_opt_k: 300.0,
            t_width_k: 10.0,
            absorption_nm: 420.0,
        };
        Genome {
            genes: vec![
                Gene { domain: d, functional: true },
                Gene { domain: Domain { family: DomainFamily::Repair, ..d }, functional: true },
            ],
            marker: [0; MARKER_LEN],
        }
    }

    #[test]
    fn every_kind_occurs_and_changes_what_it_should() {
        let g = sample_genome();
        let params = MutationParams {
            weights: [1.0; MUTATION_KIND_COUNT],
            relations: vec![DomainRelation { from: DomainFamily::Repair, to: DomainFamily::Pigment, weight: 1.0 }],
            ..Default::default()
        };
        let mut rng = rng_for(1, Stream::Validation, &[]);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..2000 {
            let change = mutate(&g, &params, &mut rng);
            let GenomeChangeCause::SpontaneousMutation(kind) = change.cause else { panic!("cause inattendue") };
            let m = change.genome;
            seen.insert(kind);
            match kind {
                MutationKind::Duplication | MutationKind::DeNovo => assert_eq!(m.genes.len(), 3),
                MutationKind::DuplicationDivergence => {
                    assert_eq!(m.genes.len(), 3);
                    // La copie d'un gène de réparation devient un pigment médiocre.
                    if let ChangedElement::Inserted { family: DomainFamily::Pigment, index } = change.element {
                        assert!((m.genes[index as usize].domain.efficiency - 0.2).abs() < 1e-12);
                    }
                }
                MutationKind::Deletion => assert_eq!(m.genes.len(), 1),
                MutationKind::NeutralMarker => assert_eq!(m.marker_distance(&g), 1),
                MutationKind::LossOfFunction => assert_eq!(m.functional_genes().count(), 1),
                MutationKind::Point => assert_eq!(m.genes.len(), 2),
            }
        }
        assert_eq!(seen.len(), MUTATION_KIND_COUNT);
    }

    #[test]
    fn repair_lowers_mutation_rate() {
        let params = MutationParams::default();
        let g = sample_genome();
        let mut broken = g.clone();
        broken.genes[1].functional = false;
        let ratio = params.genomic_rate(&broken) / params.genomic_rate(&g);
        assert!((ratio - params.max_repair_factor).abs() < 1e-9);
    }
}
