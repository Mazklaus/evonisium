//! Phénotype dérivé : ce que le génome donne au corps unicellulaire.
//!
//! Calculé une fois par génome puis partagé par toutes les populations qui le
//! portent (« anatomie mise en cache » de l'échelle commune).
//!
//! Chaque pièce des voies lumineuses a une fonction seule (exaptation,
//! document Organismes, règle 1) : le pigment protège des ultraviolets, le
//! cytochrome et les centres réactionnels améliorent le rendement des voies
//! chimiques (chaîne de transport d'électrons), la rhodopsine pompe des
//! protons à la lumière, le complexe à manganèse détruit les espèces réactives
//! de l'oxygène.

use crate::growth::Physiology;
use crate::metabolism::{EnergySource, ANOXYGENIC_CENTRES, REACTIONS, REACTION_COUNT};
use evo_core::math::Det;
use evo_genetics::{DomainFamily, Genome, ReactionId};

/// Une enzyme exprimée, telle que la lit la physiologie (y compris les
/// centres réactionnels des voies lumineuses).
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
    /// Quantité de pigment (somme des domaines, plafonnée à 1) : protection
    /// contre les ultraviolets.
    pub pigment: f64,
    /// Qualité de capture de la lumière par les pigments sous l'étoile de la
    /// partie (quantité × accord du spectre d'absorption), de 0 à 1.
    pub light_capture: f64,
    /// Pic d'absorption moyen des pigments, nm (couleur affichée).
    pub pigment_nm: Option<f64>,
    /// Qualité de la chaîne de transport d'électrons (cytochromes, et à
    /// moitié les centres réactionnels), de 0 à 1.
    pub electron_transport: f64,
    /// Rhodopsine : quantité × accord du spectre, de 0 à 1.
    pub rhodopsin: f64,
    /// Complexe à manganèse qui oxyde l'eau, de 0 à 1.
    pub water_oxidation: f64,
    /// Défense contre l'oxygène (plafonnée à 1).
    pub oxygen_defense: f64,
    /// Coût d'entretien, kJ par mole de carbone de biomasse et par an.
    pub maintenance_kj: f64,
    /// Nombre de gènes, fonctionnels ou non (temps de réplication).
    pub gene_count: u32,
    /// Voies réellement utilisables (bits) : c'est la guilde métabolique.
    pub signature: u32,
    /// Pigment couplé à la chaîne de transport d'électrons : un peu d'énergie
    /// tirée de la lumière, sans carbone fixé (phototrophie simple).
    pub cyclic_phototrophy: bool,
    /// Capte la lumière d'une façon ou d'une autre.
    pub phototroph: bool,
}

/// Facteur thermique d'une enzyme : gaussienne autour de l'optimum, dont le
/// pic baisse quand la plage s'élargit (compromis généraliste-spécialiste).
#[inline]
pub fn thermal_factor(t: f64, t_opt: f64, width: f64, physio: &Physiology) -> f64 {
    let peak = 2.0 * physio.thermal_reference_width_k / (physio.thermal_reference_width_k + width);
    let x = (t - t_opt) / width;
    peak * (-x * x).dexp()
}

impl Phenotype {
    pub fn from_genome(genome: &Genome, physio: &Physiology) -> Self {
        let mut enzymes = Vec::new();
        // Totaux par voie et par famille : le coût est convexe sur le total,
        // pour qu'une duplication ne rende pas une protéine moins chère.
        let mut reaction_eff = [0.0; REACTION_COUNT];
        let mut reaction_aff = [0.0; REACTION_COUNT];
        let (mut pigment, mut pigment_capture, mut pigment_nm_sum) = (0.0, 0.0, 0.0);
        let (mut cytochrome, mut rhodopsin, mut rhodopsin_capture, mut wox, mut defense, mut repair) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
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
                DomainFamily::Pigment => {
                    pigment += d.efficiency;
                    pigment_capture += d.efficiency * physio.spectrum.match_at(d.absorption_nm);
                    pigment_nm_sum += d.efficiency * d.absorption_nm;
                }
                DomainFamily::Cytochrome => cytochrome += d.efficiency,
                DomainFamily::Rhodopsin => {
                    rhodopsin += d.efficiency;
                    rhodopsin_capture += d.efficiency * physio.spectrum.match_at(d.absorption_nm);
                }
                DomainFamily::WaterOxidation => wox += d.efficiency,
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
        maintenance += physio.gene_cost_kj
            * (pigment * pigment
                + defense * defense
                + wox * wox
                + rhodopsin * rhodopsin
                + 0.25 * cytochrome * cytochrome
                + 0.25 * repair * repair);

        let light_capture = if pigment > 0.0 { (pigment.min(1.0) * pigment_capture / pigment).min(1.0) } else { 0.0 };
        let pigment_nm = (pigment > 0.0).then(|| pigment_nm_sum / pigment);
        let centres: f64 = ANOXYGENIC_CENTRES.iter().map(|&r| reaction_eff[r as usize]).sum::<f64>()
            + reaction_eff[crate::metabolism::OXYGENIC_PHOTOSYNTHESIS as usize];
        let electron_transport = (cytochrome + 0.5 * centres).min(1.0);
        let water_oxidation = wox.min(1.0);
        let has_partner = ANOXYGENIC_CENTRES.iter().any(|&r| reaction_eff[r as usize] > 0.0);

        let signature = enzymes.iter().fold(0u32, |acc, e| {
            let usable = match REACTIONS[e.reaction as usize].energy {
                EnergySource::Light { partner_rc, water_oxidation: needs_wox } => {
                    light_capture > 0.0 && (!partner_rc || has_partner) && (!needs_wox || water_oxidation > 0.0)
                }
                EnergySource::Chemical { .. } => true,
            };
            if usable {
                acc | (1 << e.reaction)
            } else {
                acc
            }
        });
        let cyclic_phototrophy = light_capture > 0.0 && electron_transport > 0.0;
        let rhodopsin_q = if rhodopsin > 0.0 { (rhodopsin.min(1.0) * rhodopsin_capture / rhodopsin).min(1.0) } else { 0.0 };
        let light_signature = REACTIONS.iter().any(|r| r.is_light() && signature & (1 << r.id) != 0);
        Self {
            enzymes,
            pigment: pigment.min(1.0),
            light_capture,
            pigment_nm,
            electron_transport,
            rhodopsin: rhodopsin_q,
            water_oxidation,
            oxygen_defense: (defense + 0.5 * water_oxidation).min(1.0),
            maintenance_kj: maintenance,
            gene_count: genome.genes.len() as u32,
            signature,
            cyclic_phototrophy,
            phototroph: light_signature || cyclic_phototrophy || rhodopsin_q > 0.0,
        }
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
