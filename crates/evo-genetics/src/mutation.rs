//! Mutations. Aucune n'a d'effet prédéfini : elles modifient le génome, et
//! leur conséquence découle du phénotype que le chantier Organismes en tire.

use crate::genome::{Domain, DomainFamily, Gene, Genome, GenomeChange, GenomeChangeCause, ReactionId, MARKER_LEN};
use rand::Rng;
use rand_distr::{Distribution, Normal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
}

/// Taux et poids relatifs des classes de mutations.
#[derive(Clone, Debug, PartialEq)]
pub struct MutationParams {
    /// Mutations par gène et par génération en l'absence de réparation.
    pub rate_per_gene: f64,
    /// Réduction maximale du taux par la réparation de l'ADN.
    pub max_repair_factor: f64,
    /// Poids relatifs : ponctuelle, perte, duplication, délétion, neutre, de novo.
    pub weights: [f64; 6],
    /// Écart type des petits pas (relatif pour efficacité, affinité, largeur).
    pub small_step: f64,
    /// Écart type des grands sauts, et leur part parmi les mutations ponctuelles.
    pub large_step: f64,
    pub large_step_share: f64,
    /// Écart type des pas de température optimale, K.
    pub t_step_k: f64,
    /// Nombre de réactions du catalogue (pour les gènes de novo).
    pub reaction_count: u8,
}

impl Default for MutationParams {
    fn default() -> Self {
        Self {
            // Avec une réparation efficace (facteur 10), un génome minimal de
            // 4 à 10 gènes mute au rythme de la règle de Drake (≈ 0,003).
            rate_per_gene: 3e-3,
            max_repair_factor: 10.0,
            weights: [0.70, 0.08, 0.06, 0.06, 0.099, 0.001],
            small_step: 0.08,
            large_step: 0.5,
            large_step_share: 0.05,
            t_step_k: 1.5,
            reaction_count: 7,
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
pub const MUTATION_KINDS: [MutationKind; 6] = [
    MutationKind::Point,
    MutationKind::LossOfFunction,
    MutationKind::Duplication,
    MutationKind::Deletion,
    MutationKind::NeutralMarker,
    MutationKind::DeNovo,
];

fn pick_kind(params: &MutationParams, rng: &mut impl Rng) -> MutationKind {
    const KINDS: [MutationKind; 6] = MUTATION_KINDS;
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
    let z: f64 = Normal::new(0.0, 1.0).expect("loi normale").sample(rng);
    match rng.random_range(0..4) {
        0 => domain.efficiency = (domain.efficiency * (sd * z).exp()).max(1e-6),
        1 => domain.affinity = (domain.affinity * (sd * z).exp()).max(1e-6),
        2 => domain.t_opt_k += z * params.t_step_k * if large { 6.0 } else { 1.0 },
        _ => domain.t_width_k = (domain.t_width_k * (sd * z).exp()).clamp(0.5, 80.0),
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
    genome.derive(GenomeChangeCause::Mutation(kind), |g| apply(g, kind, params, rng))
}

fn apply(g: &mut Genome, kind: MutationKind, params: &MutationParams, rng: &mut impl Rng) {
    match kind {
        MutationKind::Point => {
            let i = rng.random_range(0..g.genes.len());
            point_mutation(&mut g.genes[i].domain, params, rng);
        }
        MutationKind::LossOfFunction => {
            let i = rng.random_range(0..g.genes.len());
            g.genes[i].functional = false;
        }
        MutationKind::Duplication => {
            let i = rng.random_range(0..g.genes.len());
            let copy = g.genes[i];
            g.genes.insert(i + 1, copy);
        }
        MutationKind::Deletion => {
            let i = rng.random_range(0..g.genes.len());
            g.genes.remove(i);
        }
        MutationKind::NeutralMarker => {
            let i = rng.random_range(0..MARKER_LEN);
            g.marker[i] = (g.marker[i] + rng.random_range(1..4)) % 4;
        }
        MutationKind::DeNovo => {
            let n = g.genes.len().max(1) as f64;
            let t_opt = g.genes.iter().map(|x| x.domain.t_opt_k).sum::<f64>() / n;
            let reaction: ReactionId = rng.random_range(0..params.reaction_count);
            let family = match rng.random_range(0..10) {
                0 => DomainFamily::OxidativeDefense,
                1 => DomainFamily::Pigment,
                _ => DomainFamily::Catalytic(reaction),
            };
            let t_opt = if t_opt.is_finite() && t_opt > 0.0 { t_opt } else { 300.0 };
            g.genes.push(Gene {
                domain: Domain { family, efficiency: 0.05, affinity: 0.5, t_opt_k: t_opt, t_width_k: 10.0 },
                functional: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::rng::{rng_for, Stream};

    fn sample_genome() -> Genome {
        let d = Domain { family: DomainFamily::Catalytic(0), efficiency: 1.0, affinity: 1.0, t_opt_k: 300.0, t_width_k: 10.0 };
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
        let params = MutationParams { weights: [1.0; 6], ..Default::default() };
        let mut rng = rng_for(1, Stream::Validation, &[]);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..2000 {
            let change = mutate(&g, &params, &mut rng);
            let GenomeChangeCause::Mutation(kind) = change.cause;
            let m = change.genome;
            seen.insert(kind);
            match kind {
                MutationKind::Duplication | MutationKind::DeNovo => assert_eq!(m.genes.len(), 3),
                MutationKind::Deletion => assert_eq!(m.genes.len(), 1),
                MutationKind::NeutralMarker => assert_eq!(m.marker_distance(&g), 1),
                MutationKind::LossOfFunction => assert_eq!(m.functional_genes().count(), 1),
                MutationKind::Point => assert_eq!(m.genes.len(), 2),
            }
        }
        assert_eq!(seen.len(), 6);
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
