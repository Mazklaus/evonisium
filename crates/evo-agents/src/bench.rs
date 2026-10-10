//! Banc d'essai : échantillons sans monde, pour les tests et la porte du
//! client. Une colonie eucaryote sexuée à deux types cellulaires, dans une
//! eau de référence ; ses traits peuvent être imposés pour figurer un corps
//! d'animal que le moteur ne produit pas encore.

use crate::sample::{Development, PopRates, Sample};
use crate::traits::Diet;
use evo_genetics::{Domain, DomainFamily, Gene, Genome, MutationParams, Organelle};
use evo_life::metabolism::*;
use evo_life::{growth_rates, Conditions, Phenotype, Physiology};
use evo_planet::{WaterPool, WATER_POOL_COUNT};
use std::sync::Arc;

fn gene(family: DomainFamily, efficiency: f64, affinity: f64, nm: f64) -> Gene {
    Gene { domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: nm }, functional: true }
}

/// Génome d'une colonie eucaryote sexuée ; `heterotroph` retire la
/// photosynthèse.
pub fn colony_genome(heterotroph: bool) -> Genome {
    use DomainFamily::*;
    let mut genes = vec![
        gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0),
        gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0),
        gene(Signalling, 1.0, 1.0, 500.0),
        gene(Adhesion, 1.2, 1.0, 600.0),
        gene(Cytoskeleton, 2.0, 1.0, 500.0),
        gene(Meiosis, 1.0, 1.0, 500.0),
        gene(Regulator, 1.0, 0.5, 500.0),
    ];
    if !heterotroph {
        genes.push(gene(Pigment, 1.0, 1.0, 650.0));
        genes.push(gene(Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 1.0, 1.0, 500.0));
    }
    let mut g = Genome::new(genes, [1; 32]);
    // Un organite respiratoire : eucaryote, donc sexué avec sa méiose.
    g.organelles.push(Organelle {
        genes: vec![gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0)],
        origin_lineage: 1,
        acquired_years: 0.0,
    });
    g
}

/// Développement dans une eau de référence (300 K, un peu d'oxygène).
pub fn development(genome: &Genome, aquatic: bool) -> Development {
    let physio = Physiology::default();
    let mut chem = [0.0; WATER_POOL_COUNT];
    chem[WaterPool::Dic as usize] = 8.0;
    chem[WaterPool::Doc as usize] = 0.5;
    chem[WaterPool::O2 as usize] = 0.05;
    chem[WaterPool::Po4 as usize] = 1e-3;
    chem[WaterPool::H2s as usize] = 1e-2;
    let conditions = Conditions::new(300.0, 0.0, 5e4);
    let p = Phenotype::from_genome(genome, &physio);
    let resident = growth_rates(&p, &conditions, &chem, &physio);
    Development {
        physio,
        mutation: MutationParams { reaction_count: REACTION_COUNT as u8, ..Default::default() },
        conditions,
        chemistry: chem,
        resident,
        aquatic,
    }
}

/// Échantillon d'essai d'une colonie.
pub fn sample(rates: PopRates, size: usize, seed: u64, heterotroph: bool) -> Sample {
    let g = colony_genome(heterotroph);
    let dev = development(&g, true);
    Sample::synthetic(Arc::new(g), dev, rates, size, 60.0, seed)
}

/// Impose un corps d'animal à un échantillon d'essai (longueur, vitesse de
/// pointe, portée des sens, régime) et coupe les mutations, pour que les
/// enfants gardent ce corps.
pub fn with_body(mut s: Sample, length_m: f64, sprint_m_s: f64, perception_m: f64, diet: Diet) -> Sample {
    s.development.mutation.rate_per_gene = 0.0;
    for g in s.genotypes.iter_mut() {
        let t = &mut g.traits;
        t.length_m = length_m;
        t.sprint_m_s = sprint_m_s;
        t.cruise_m_s = 0.3 * sprint_m_s;
        t.perception_m = perception_m;
        t.diet = diet;
        t.nervous = t.nervous.max(0.6);
        t.reaction_s = 0.3;
        t.sociality = if diet == Diet::Predator { 0.1 } else { 0.8 };
    }
    s
}
