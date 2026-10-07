//! Taux de croissance et coefficient de sélection (document Organismes,
//! section « Sorties calculées pour les autres chantiers ») :
//!
//! r(g, c) = (E_assimilée − E_entretien − E_activité) / E_descendant
//!           − (m_fond + m_prédation + m_maladie + m_stress)
//!
//! s = (r_mutant − r_résident) × T_génération
//!
//! Les rapports sont exprimés par mole de carbone de biomasse : E_descendant
//! est le coût de fabrication d'une mole de carbone de biomasse. À l'étape 1,
//! il n'y a ni prédation, ni maladie, ni activité motrice.

use crate::metabolism::{EnergySource, REACTIONS, REACTION_COUNT};
use crate::phenotype::Phenotype;
use evo_planet::{WaterChemistry, WaterPool};

/// Constantes physiologiques des unicellulaires. Ce sont des ordres de
/// grandeur de microbiologie, à calibrer ; aucune n'est propre à la Terre.
#[derive(Clone, Debug, PartialEq)]
pub struct Physiology {
    /// Débit maximal d'une enzyme d'efficacité 1, mol de substrat par mole
    /// de carbone de biomasse et par an.
    pub max_uptake: f64,
    /// Part de la lumière absorbée convertie en énergie chimique pour une
    /// capacité photosynthétique de 1.
    pub photo_efficiency: f64,
    /// Coût de fabrication d'une mole de carbone de biomasse, kJ, à partir du
    /// CO₂ (autotrophes) ou de matière organique (hétérotrophes).
    pub autotroph_biomass_kj: f64,
    pub heterotroph_biomass_kj: f64,
    /// Entretien, kJ·molC⁻¹·an⁻¹ : base, réplication par gène présent
    /// (pseudogènes compris), expression par gène fonctionnel, et coût des
    /// protéines d'une voie, qui croît avec le carré de son activité totale et
    /// de son affinité.
    pub base_maintenance_kj: f64,
    pub genome_cost_kj: f64,
    pub expression_cost_kj: f64,
    pub gene_cost_kj: f64,
    /// Largeur thermique de référence, K.
    pub thermal_reference_width_k: f64,
    /// Mortalité de fond, an⁻¹.
    pub background_mortality: f64,
    /// Mortalité maximale par stress oxydant sans défense, an⁻¹, et
    /// concentration de demi-effet de l'O₂, mol·m⁻³.
    pub oxygen_stress_mortality: f64,
    pub oxygen_stress_half: f64,
    /// Concentration d'O₂ qui divise par deux l'activité des voies anaérobies.
    pub oxygen_inhibition_half: f64,
    /// Taux de croissance maximal, an⁻¹ (doublement en une heure environ).
    pub max_growth: f64,
    /// Carbone d'une cellule, mol (environ 10⁻¹³ g de carbone).
    pub carbon_per_cell: f64,
}

impl Default for Physiology {
    fn default() -> Self {
        Self {
            max_uptake: 2.0e4,
            photo_efficiency: 0.05,
            autotroph_biomass_kj: 2000.0,
            heterotroph_biomass_kj: 400.0,
            base_maintenance_kj: 2000.0,
            genome_cost_kj: 200.0,
            expression_cost_kj: 300.0,
            gene_cost_kj: 2000.0,
            thermal_reference_width_k: 10.0,
            background_mortality: 20.0,
            oxygen_stress_mortality: 200.0,
            oxygen_stress_half: 1e-3,
            oxygen_inhibition_half: 1e-4,
            max_growth: 6000.0,
            carbon_per_cell: 1e-14,
        }
    }
}

/// Résultat de l'évaluation d'un phénotype dans un milieu.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrowthRates {
    /// Vitesse de chaque voie : mol de substrat limitant (ou mol de carbone
    /// fixé pour la photosynthèse) par mole de carbone de biomasse et par an.
    pub reaction: [f64; REACTION_COUNT],
    /// Énergie assimilée, kJ·molC⁻¹·an⁻¹.
    pub energy_kj: f64,
    /// Part de l'énergie tirée de voies hétérotrophes.
    pub heterotroph_share: f64,
    /// Coût de fabrication d'une mole de carbone de biomasse, kJ.
    pub biomass_cost_kj: f64,
    /// Taux de naissance permis par le surplus d'énergie, an⁻¹.
    pub birth: f64,
    /// Mortalité totale, an⁻¹.
    pub mortality: f64,
    /// Taux de croissance net r, an⁻¹.
    pub r: f64,
}

impl GrowthRates {
    /// Durée d'une génération, an.
    pub fn generation_time(&self, physio: &Physiology) -> f64 {
        std::f64::consts::LN_2 / self.birth.max(physio.background_mortality)
    }
}

#[inline]
fn monod(c: f64, k: f64) -> f64 {
    let c = c.max(0.0);
    c / (k + c)
}

/// Évalue r(g, c) pour un phénotype dans une cellule. `light_kj` est la
/// lumière absorbée disponible par mole de carbone de biomasse phototrophe.
pub fn growth_rates(p: &Phenotype, temperature_k: f64, chem: &WaterChemistry, light_kj: f64, physio: &Physiology) -> GrowthRates {
    let o2 = chem[WaterPool::O2 as usize].max(0.0);
    let anaerobic_factor = physio.oxygen_inhibition_half / (physio.oxygen_inhibition_half + o2);
    let mut out = GrowthRates::default();
    let (mut energy, mut het_energy) = (0.0, 0.0);

    for reaction in REACTIONS.iter() {
        if p.signature & (1 << reaction.id) == 0 {
            continue;
        }
        let (cap, affinity) = p.capacity(reaction.id, temperature_k, physio);
        if cap <= 0.0 {
            continue;
        }
        let mut limitation = match reaction.cosubstrate {
            Some((pool, k)) => monod(chem[pool as usize], k),
            None => 1.0,
        };
        if reaction.oxygen_sensitive {
            limitation *= anaerobic_factor;
        }
        let (rate, e) = match reaction.energy {
            EnergySource::Chemical { substrate, half_saturation, dg_kj } => {
                let k = half_saturation / affinity.max(1e-6);
                let q = physio.max_uptake * cap * monod(chem[substrate as usize], k) * limitation;
                (q, q * dg_kj)
            }
            EnergySource::Light => {
                let e = light_kj * physio.photo_efficiency * cap.min(1.0) * p.pigment * limitation;
                (e / physio.autotroph_biomass_kj, e)
            }
        };
        out.reaction[reaction.id as usize] = rate;
        energy += e;
        if reaction.heterotrophic {
            het_energy += e;
        }
    }

    let het = if energy > 0.0 { het_energy / energy } else { 0.0 };
    let cost = het * physio.heterotroph_biomass_kj + (1.0 - het) * physio.autotroph_biomass_kj;
    let surplus = energy - p.maintenance_kj;
    let birth = (surplus / cost).clamp(0.0, physio.max_growth);
    // Un déficit d'énergie consomme la biomasse : mortalité de famine.
    let starvation = (-surplus).max(0.0) / cost;
    let oxygen_stress = physio.oxygen_stress_mortality * o2 / (o2 + physio.oxygen_stress_half) * (1.0 - p.oxygen_defense);
    let mortality = physio.background_mortality + starvation + oxygen_stress;

    out.energy_kj = energy;
    out.heterotroph_share = het;
    out.biomass_cost_kj = cost;
    out.birth = birth;
    out.mortality = mortality;
    out.r = birth - mortality;
    out
}

/// s = (r_mutant − r_résident) × T_génération du résident.
pub fn selection_coefficient(mutant: &GrowthRates, resident: &GrowthRates, physio: &Physiology) -> f64 {
    (mutant.r - resident.r) * resident.generation_time(physio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metabolism::*;
    use evo_genetics::{Domain, DomainFamily, Gene, Genome};
    use evo_planet::WATER_POOL_COUNT;

    fn genome(genes: &[(DomainFamily, f64)]) -> Genome {
        Genome {
            genes: genes
                .iter()
                .map(|&(family, efficiency)| Gene {
                    domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: 300.0, t_width_k: 10.0 },
                    functional: true,
                })
                .collect(),
            marker: [0; 32],
        }
    }

    fn chem(h2: f64, o2: f64) -> WaterChemistry {
        let mut c = [0.0; WATER_POOL_COUNT];
        c[WaterPool::Dic as usize] = 8.0;
        c[WaterPool::H2 as usize] = h2;
        c[WaterPool::O2 as usize] = o2;
        c
    }

    #[test]
    fn methanogen_grows_with_hydrogen_and_dies_without() {
        let physio = Physiology::default();
        let p = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]), &physio);
        assert!(growth_rates(&p, 300.0, &chem(8e-4, 0.0), 0.0, &physio).r > 0.0);
        assert!(growth_rates(&p, 300.0, &chem(0.0, 0.0), 0.0, &physio).r < 0.0);
        // Trop froid pour ses enzymes.
        assert!(growth_rates(&p, 260.0, &chem(8e-4, 0.0), 0.0, &physio).r < 0.0);
        // L'oxygène bloque la méthanogenèse et tue l'anaérobie sans défense.
        assert!(growth_rates(&p, 300.0, &chem(8e-4, 0.2), 0.0, &physio).r < 0.0);
    }

    #[test]
    fn photosynthesis_needs_a_pigment() {
        let physio = Physiology::default();
        let without = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(OXYGENIC_PHOTOSYNTHESIS), 1.0)]), &physio);
        let with = Phenotype::from_genome(
            &genome(&[(DomainFamily::Catalytic(OXYGENIC_PHOTOSYNTHESIS), 1.0), (DomainFamily::Pigment, 1.0)]),
            &physio,
        );
        assert_eq!(without.signature, 0);
        assert!(with.phototroph);
        assert!(growth_rates(&with, 300.0, &chem(0.0, 0.0), 1e6, &physio).r > 0.0);
    }

    #[test]
    fn better_adapted_mutant_has_positive_selection_coefficient() {
        let physio = Physiology::default();
        let resident = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]), &physio);
        let mut g = genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]);
        g.genes[0].domain.t_opt_k = 290.0;
        let mutant = Phenotype::from_genome(&g, &physio);
        let c = chem(8e-4, 0.0);
        let r_res = growth_rates(&resident, 288.0, &c, 0.0, &physio);
        let r_mut = growth_rates(&mutant, 288.0, &c, 0.0, &physio);
        assert!(selection_coefficient(&r_mut, &r_res, &physio) > 0.0);
        assert!(selection_coefficient(&r_res, &r_mut, &physio) < 0.0);
    }
}
