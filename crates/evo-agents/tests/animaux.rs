//! Un animal du moteur : ses traits de comportement sont ceux que le moteur
//! fait jouer dans la prédation, pas des valeurs devinées.

use evo_agents::traits::{proxy, Traits};
use evo_genetics::{Domain, DomainFamily, Gene, Genome, Organelle};
use evo_life::metabolism::{AEROBIC_RESPIRATION, FERMENTATION};
use evo_life::{body_plan, GrowthRates, Phenotype, Physiology};

fn gene(family: DomainFamily, efficiency: f64, affinity: f64, nm: f64) -> Gene {
    Gene { domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: nm }, functional: true }
}

#[test]
fn an_animal_reads_the_engine_traits() {
    use DomainFamily::*;
    let physio = Physiology::default();
    // Le corps du banc du constructeur de phénotypes : muscles, nerfs,
    // matrice, sens, deux types cellulaires.
    let mut g = Genome::new(
        vec![
            gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0),
            gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0),
            gene(Cytoskeleton, 1.0, 1.0, 500.0),
            gene(Signalling, 1.0, 1.0, 500.0),
            gene(Adhesion, 1.2, 1.0, 600.0),
            gene(Regulator, 1.0, 0.5, 750.0),
            gene(Pigment, 0.2, 1.0, 500.0),
            gene(Motor, 0.8, 1.0, 620.0),
            gene(Neural, 1.0, 1.0, 620.0),
            gene(Matrix, 0.5, 1.0, 620.0),
            gene(Sensor, 0.5, 1.0, 620.0),
        ],
        [0; 32],
    );
    g.organelles.push(Organelle {
        genes: vec![gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0)],
        origin_lineage: 1,
        acquired_years: 0.0,
    });
    let p = Phenotype::from_genome(&g, &physio);
    assert_eq!(evo_sim::world::animal_bits(&p) & 1, 1, "le banc n'est plus un animal : {:?}", p.animal);
    let plan = body_plan(&g, &p, &physio);
    let rates = GrowthRates { heterotroph_share: 1.0, prey_share: 0.5, ..GrowthRates::default() };
    let t = Traits::of(&p, &plan, &rates, true);
    let a = &p.animal;
    assert_eq!(t.length_m, a.length_m);
    assert_eq!(t.cruise_m_s, a.speed_m_s);
    assert!(t.sprint_m_s > t.cruise_m_s);
    assert_eq!(t.perception_m, a.detection_m);
    assert!(t.defence >= a.defense);
    assert!(t.nervous > 0.0 && t.nervous <= 1.0);
    assert!(t.moves());
    assert_eq!(t.proxies & (proxy::PERCEPTION | proxy::NERVOUS | proxy::MUSCLE), 0, "traits encore devinés : {:b}", t.proxies);
}
