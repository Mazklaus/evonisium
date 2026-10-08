//! Grandes transitions de l'étape 4 : endosymbiose, sexe, cellules complexes.
//!
//! Rien n'est déclenché par un compteur : chaque transition est un candidat
//! du régime « apparition puis fixation », jugé comme une mutation par son
//! coefficient de sélection (document Organismes, « Grandes transitions
//! évolutives » ; document Génétique, « Grandes transitions évolutives à
//! rendre possibles »).
//!
//! - Endosymbiose : un phagotrophe englobe des proies ; rarement, une proie
//!   survit et reste. Le génome candidat est celui de l'hôte augmenté d'un
//!   organite qui porte les gènes d'énergie de la proie. Le nombre de
//!   rétentions attendues pendant le tour est l'offre de « mutants » ;
//!   l'organite se fixe s'il rend l'hôte plus apte (une mitochondrie rend la
//!   respiration des proies à pleine capacité dans une grande cellule ; un
//!   plaste apporte la photosynthèse).
//! - Sexe : chez un eucaryote doté d'une recombinase de méiose, les mutations
//!   avantageuses apparues dans des individus différents se réunissent par
//!   recombinaison au lieu de se concurrencer (Fisher et Muller).
//!
//! [Simplification] La probabilité qu'une proie englobée survive et reste est
//! un paramètre réglable, bien plus élevé que dans la réalité (« endosymbiose
//! facilitée », document Génétique), calé sur la chronologie terrestre.
//! [Simplification] Sexe isogame et facultatif, sans coût direct : seuls ses
//! effets sur la combinaison des mutations sont modélisés.

use crate::world::WorldConfig;
use evo_genetics::{ChangedElement, DomainFamily, Gene, Genome, GenomeChange, GenomeChangeCause, Organelle};
use evo_life::community::Population;
use evo_life::growth::edibility;
use evo_life::Phenotype;
use rand::Rng;

/// Réglages des transitions.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransitionParams {
    /// Probabilité qu'une proie englobée survive et reste dans son hôte
    /// (par proie englobée).
    pub retention_probability: f64,
    /// Proies tirées comme partenaires candidats par génotype et par tour.
    pub endosymbiosis_candidates: usize,
    /// Organites au plus par cellule.
    pub max_organelles: usize,
    /// Durée sur laquelle un endosymbiote devient un organite dépendant, ans.
    pub integration_years: f64,
}

impl Default for TransitionParams {
    fn default() -> Self {
        Self { retention_probability: 3e-29, endosymbiosis_candidates: 1, max_organelles: 2, integration_years: 100e6 }
    }
}

/// Un gène d'énergie que l'organite garde (voies, pigments, chaîne de
/// transport, oxydation de l'eau, défense).
fn kept_by_organelle(g: &Gene) -> bool {
    g.functional
        && matches!(
            g.domain.family,
            DomainFamily::Catalytic(_)
                | DomainFamily::Pigment
                | DomainFamily::Cytochrome
                | DomainFamily::WaterOxidation
                | DomainFamily::OxidativeDefense
        )
}

/// Partenaire candidat : génome de l'hôte augmenté de l'organite, et nombre
/// de rétentions attendues pendant le tour dans tout le groupe.
pub struct Partner {
    pub change: GenomeChange,
    pub copies: f64,
}

/// Tire les partenaires candidats d'un phagotrophe : des proies de sa
/// cellule qu'il peut englober, au prorata de ce qu'il en mange.
#[allow(clippy::too_many_arguments)]
pub fn engulfed_partners(
    resident: &Population,
    resident_index: usize,
    pops: &[Population],
    supply_cells: f64,
    dt: f64,
    years: f64,
    cfg: &WorldConfig,
    rng: &mut impl Rng,
) -> Vec<Partner> {
    let physio = &cfg.physiology;
    let params = &cfg.transitions;
    let host = &resident.phenotype;
    if host.engulfment <= 0.0 || resident.genome.organelles.len() >= params.max_organelles {
        return Vec::new();
    }
    let eaten = resident.rates.prey_demand();
    if eaten <= 0.0 {
        return Vec::new();
    }
    let weights: Vec<f64> = pops
        .iter()
        .enumerate()
        .map(|(q, p)| {
            if q == resident_index || p.phenotype.is_multicellular() {
                0.0
            } else {
                p.biomass * edibility(host.cell_size, p.phenotype.body_size, physio)
            }
        })
        .collect();
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let host_carbon = host.cell_volume();
    let mut out = Vec::new();
    for _ in 0..params.endosymbiosis_candidates {
        let mut x = rng.random::<f64>() * total;
        let Some(q) = weights.iter().position(|&w| {
            x -= w;
            x < 0.0 && w > 0.0
        }) else {
            continue;
        };
        let prey = &pops[q];
        // Gènes d'énergie de la proie, et ceux de ses propres organites
        // (endosymbiose secondaire : une algue eucaryote englobée garde son
        // plaste).
        let genes: Vec<Gene> =
            prey.genome.genes.iter().chain(prey.genome.organelle_genes()).filter(|g| kept_by_organelle(g)).copied().collect();
        if genes.is_empty() {
            continue;
        }
        // Un organite qui ne ferait que doubler un organite déjà présent
        // n'apporte rien de neuf.
        let signature = prey.phenotype.signature & evo_life::phenotype::PATHWAY_MASK;
        if resident.genome.organelles.iter().any(|o| organelle_signature(o) == signature) {
            continue;
        }
        // Proies englobées par cellule hôte et par an : carbone mangé, part de
        // cette proie, rapport des masses de carbone d'une cellule.
        let prey_carbon = prey.phenotype.cell_volume() * prey.phenotype.cells();
        let per_host = eaten * weights[q] / total * host_carbon / prey_carbon;
        let copies = supply_cells * per_host * params.retention_probability * dt / params.endosymbiosis_candidates as f64;
        let organelle = Organelle { genes, origin_lineage: prey.lineage, acquired_years: years };
        let change = resident.genome.derive(GenomeChangeCause::Endosymbiosis, |g| {
            g.organelles.push(organelle);
            ChangedElement::Acquired { organelle: (g.organelles.len() - 1) as u16 }
        });
        out.push(Partner { change, copies });
    }
    out
}

/// Voies catalysées par les gènes d'un organite (bits).
fn organelle_signature(o: &Organelle) -> u32 {
    o.genes.iter().filter(|g| g.functional).fold(0, |acc, g| match g.domain.family {
        DomainFamily::Catalytic(r) => acc | (1 << r),
        _ => acc,
    })
}

/// Recombinaison chez un sexué : réunit dans le génome du meilleur mutant les
/// gènes modifiés par d'autres mutants avantageux qui se fixeraient, quand
/// ils touchent des gènes différents d'un génome de même longueur (mutations
/// ponctuelles). Renvoie le génome recombiné s'il diffère du meilleur.
pub fn recombine(resident: &Genome, best: &GenomeChange, others: &[GenomeChange]) -> Option<GenomeChange> {
    let mut genome = best.genome.clone();
    let mut touched: Vec<u16> = match best.element {
        ChangedElement::Gene { index, .. } if best.genome.genes.len() == resident.genes.len() => vec![index],
        _ => return None,
    };
    for o in others {
        let ChangedElement::Gene { index, .. } = o.element else { continue };
        if o.genome.genes.len() != resident.genes.len() || touched.contains(&index) {
            continue;
        }
        genome.genes[index as usize] = o.genome.genes[index as usize];
        touched.push(index);
    }
    (touched.len() > 1).then_some(GenomeChange { genome, cause: GenomeChangeCause::Recombination, element: ChangedElement::Several })
}

/// Le phénotype est-il sexué ?
pub fn is_sexual(p: &Phenotype) -> bool {
    p.sexual
}
