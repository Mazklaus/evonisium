//! Phénotype dérivé : ce que le génome donne au corps unicellulaire.
//!
//! Calculé une fois par génome puis partagé par toutes les populations qui le
//! portent (« anatomie mise en cache » de l'échelle commune).

use crate::growth::Physiology;
use crate::metabolism::{EnergySource, REACTIONS, REACTION_COUNT};
use evo_genetics::{DomainFamily, Genome, ReactionId};

/// Une enzyme exprimée, telle que la lit la physiologie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enzyme {
    pub reaction: ReactionId,
    pub efficiency: f64,
    pub affinity: f64,
    pub t_opt_k: f64,
    pub t_width_k: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Phenotype {
    pub enzymes: Vec<Enzyme>,
    /// Capacité de capture de la lumière (somme des pigments, plafonnée à 1).
    pub pigment: f64,
    /// Défense contre l'oxygène (plafonnée à 1).
    pub oxygen_defense: f64,
    /// Coût d'entretien, kJ par mole de carbone de biomasse et par an.
    pub maintenance_kj: f64,
    /// Voies réellement utilisables (bits) : c'est la guilde métabolique.
    pub signature: u32,
    /// Peut capter la lumière.
    pub phototroph: bool,
}

/// Facteur thermique d'une enzyme : gaussienne autour de l'optimum, dont le
/// pic baisse quand la plage s'élargit (compromis généraliste-spécialiste).
#[inline]
pub fn thermal_factor(t: f64, t_opt: f64, width: f64, physio: &Physiology) -> f64 {
    let peak = 2.0 * physio.thermal_reference_width_k / (physio.thermal_reference_width_k + width);
    let x = (t - t_opt) / width;
    peak * (-x * x).exp()
}

impl Phenotype {
    pub fn from_genome(genome: &Genome, physio: &Physiology) -> Self {
        let mut enzymes = Vec::new();
        // Totaux par voie et par famille : le coût est convexe sur le total,
        // pour qu'une duplication ne rende pas une enzyme moins chère.
        let mut reaction_eff = [0.0; REACTION_COUNT];
        let mut reaction_aff = [0.0; REACTION_COUNT];
        let (mut pigment, mut defense, mut repair) = (0.0, 0.0, 0.0);
        let mut maintenance = physio.base_maintenance_kj + physio.genome_cost_kj * genome.genes.len() as f64;
        for gene in genome.functional_genes() {
            let d = gene.domain;
            maintenance += physio.expression_cost_kj;
            match d.family {
                DomainFamily::Catalytic(r) if (r as usize) < REACTION_COUNT => {
                    reaction_eff[r as usize] += d.efficiency;
                    reaction_aff[r as usize] += d.efficiency * d.affinity;
                    enzymes.push(Enzyme {
                        reaction: r,
                        efficiency: d.efficiency,
                        affinity: d.affinity,
                        t_opt_k: d.t_opt_k,
                        t_width_k: d.t_width_k,
                    });
                }
                DomainFamily::Catalytic(_) => {}
                DomainFamily::Pigment => pigment += d.efficiency,
                DomainFamily::OxidativeDefense => defense += d.efficiency,
                // La réparation agit sur le taux de mutation ; ici seul son coût compte.
                DomainFamily::Repair => repair += d.efficiency,
            }
        }
        for r in 0..REACTION_COUNT {
            let e = reaction_eff[r];
            if e > 0.0 {
                let a = reaction_aff[r] / e;
                maintenance += physio.gene_cost_kj * (e * e + 0.25 * a * a);
            }
        }
        maintenance += physio.gene_cost_kj * (pigment * pigment + defense * defense + 0.25 * repair * repair);
        let pigment = pigment.min(1.0);
        let signature = enzymes.iter().fold(0u32, |acc, e| {
            let usable = match REACTIONS[e.reaction as usize].energy {
                EnergySource::Light => pigment > 0.0,
                EnergySource::Chemical { .. } => true,
            };
            if usable {
                acc | (1 << e.reaction)
            } else {
                acc
            }
        });
        let phototroph = REACTIONS.iter().any(|r| r.energy == EnergySource::Light && signature & (1 << r.id) != 0);
        Self { enzymes, pigment, oxygen_defense: defense.min(1.0), maintenance_kj: maintenance, signature, phototroph }
    }

    /// Capacité et affinité moyenne d'une voie à la température `t`.
    pub fn capacity(&self, reaction: ReactionId, t: f64, physio: &Physiology) -> (f64, f64) {
        let (mut cap, mut aff) = (0.0, 0.0);
        for e in self.enzymes.iter().filter(|e| e.reaction == reaction) {
            let c = e.efficiency * thermal_factor(t, e.t_opt_k, e.t_width_k, physio);
            cap += c;
            aff += c * e.affinity;
        }
        if cap > 0.0 {
            (cap, aff / cap)
        } else {
            (0.0, 1.0)
        }
    }

    /// Température optimale moyenne des enzymes, pondérée par l'efficacité.
    pub fn mean_t_opt(&self) -> Option<f64> {
        let w: f64 = self.enzymes.iter().map(|e| e.efficiency).sum();
        (w > 0.0).then(|| self.enzymes.iter().map(|e| e.efficiency * e.t_opt_k).sum::<f64>() / w)
    }
}
