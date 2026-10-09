//! Populations écologiques d'une cellule et leur dynamique.
//!
//! Une population est une guilde métabolique dans une cellule : sa biomasse,
//! sa lignée et le génome de son écotype résident. Les populations d'une
//! cellule se partagent la chimie de la couche d'eau et la lumière ; leurs
//! déchets sont les ressources des autres (la nécromasse devient matière
//! organique dissoute).
//!
//! Le phosphore suit le carbone de la biomasse au rapport de Redfield : la
//! croissance puise dans le phosphate, la mort le rend.
//!
//! [Simplification] Écologie quasi stationnaire : à chaque pas planétaire, la
//! dynamique rapide des microbes est intégrée sur quelques jours seulement, le
//! système se rapprochant pas après pas de son équilibre ; la planète en tire
//! des flux annuels qu'elle applique sur tout le pas.

use crate::growth::{edibility, growth_rates_with, Conditions, GrowthRates, Physiology};
use crate::metabolism::{EnergySource, REACTIONS};
use crate::phenotype::{Capacities, Phenotype};
use evo_core::math::Det;
use evo_core::units::watts_to_kj_per_year;
use evo_genetics::Genome;
use evo_planet::{CellEnvironment, WaterChemistry, WaterPool, WATER_POOL_COUNT};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Population {
    pub lineage: u32,
    pub genome: Arc<Genome>,
    pub phenotype: Arc<Phenotype>,
    /// Biomasse, mol de carbone.
    pub biomass: f64,
    /// Dernière évaluation de r(g, c) dans la cellule.
    pub rates: GrowthRates,
}

impl Population {
    pub fn signature(&self) -> u32 {
        self.phenotype.signature
    }

    /// Effectif (nombre de cellules).
    pub fn census(&self, physio: &Physiology) -> f64 {
        self.biomass / physio.carbon_per_cell
    }
}

/// Lumière d'une cellule, partagée par la biomasse phototrophe.
#[derive(Clone, Copy, Debug)]
pub struct CellContext<'a> {
    pub env: &'a CellEnvironment,
    /// Biomasse qui absorbe 63 % de la lumière, mol de carbone par m².
    pub light_biomass_per_m2: f64,
}

impl CellContext<'_> {
    /// Lumière absorbée par mole de carbone phototrophe, kJ·molC⁻¹·an⁻¹,
    /// quand la biomasse phototrophe vaut `photo_biomass`.
    pub fn light_per_biomass(&self, photo_biomass: f64) -> f64 {
        let b_ref = self.light_biomass_per_m2 * self.env.water_area_m2.max(1.0);
        let incoming = watts_to_kj_per_year(self.env.light_par_w_m2 * self.env.water_area_m2);
        let x = photo_biomass / b_ref;
        if x < 1e-9 {
            // Limite à faible biomasse : toute la lumière est disponible.
            incoming / b_ref
        } else {
            incoming * (-(-x).dexp_m1()) / photo_biomass
        }
    }

    pub fn photo_biomass(pops: &[Population]) -> f64 {
        pops.iter().filter(|p| p.phenotype.phototroph).map(|p| p.biomass).sum()
    }

    /// Conditions physiques vues par les organismes quand la biomasse
    /// phototrophe de la cellule vaut `photo_biomass`, sans proie ni
    /// prédateur.
    pub fn conditions(&self, photo_biomass: f64) -> Conditions {
        Conditions::new(self.env.temperature_k, self.env.uv_w_m2, self.light_per_biomass(photo_biomass))
    }

    /// Conditions vues par les organismes de la communauté `pops` : lumière
    /// partagée, proies et pression des prédateurs (d'après leur dernière
    /// évaluation).
    pub fn conditions_of(&self, pops: &[Population], physio: &Physiology) -> Conditions {
        let mut cond = self.conditions(Self::photo_biomass(pops));
        let v = self.env.water_volume_m3;
        if v <= 0.0 {
            return cond;
        }
        // Les proies sont toujours listées : un mutant phagotrophe jugé dans
        // une cellule qui n'en a pas encore doit les voir.
        for p in pops {
            cond.prey.push(p.phenotype.body_size, p.biomass / v);
        }
        for (j, p) in pops.iter().enumerate() {
            let demand = p.rates.prey_demand() * p.biomass;
            if p.phenotype.engulfment <= 0.0 || demand <= 0.0 {
                continue;
            }
            let edible = edible_biomass(pops, j, physio);
            if edible > 0.0 {
                cond.predators.push(p.phenotype.cell_size, demand / edible);
            }
        }
        cond
    }
}

/// Biomasse que le prédateur `j` peut englober dans la communauté (lui
/// excepté), pondérée par la facilité à englober chaque proie.
fn edible_biomass(pops: &[Population], j: usize, physio: &Physiology) -> f64 {
    let size = pops[j].phenotype.cell_size;
    pops.iter().enumerate().filter(|&(q, _)| q != j).map(|(_, q)| q.biomass * edibility(size, q.phenotype.body_size, physio)).sum()
}

/// Évalue toutes les populations d'une cellule dans l'état courant.
pub fn evaluate(pops: &mut [Population], ctx: &CellContext, chem: &WaterChemistry, physio: &Physiology) {
    let caps = capacities(pops, ctx, physio);
    evaluate_with(pops, &caps, ctx, chem, physio);
}

/// Capacités des populations d'une cellule à sa température (constantes
/// pendant les sous-pas de l'écologie d'un pas).
pub fn capacities(pops: &[Population], ctx: &CellContext, physio: &Physiology) -> Vec<Capacities> {
    pops.iter().map(|p| p.phenotype.capacities(ctx.env.temperature_k, physio)).collect()
}

/// [`evaluate`] avec les capacités déjà calculées (une par population).
pub fn evaluate_with(pops: &mut [Population], caps: &[Capacities], ctx: &CellContext, chem: &WaterChemistry, physio: &Physiology) {
    let cond = ctx.conditions_of(pops, physio);
    for (p, c) in pops.iter_mut().zip(caps) {
        p.rates = growth_rates_with(&p.phenotype, c, &cond, chem, physio);
    }
}

/// Avance la communauté d'une cellule de `dt` années (pas explicite court).
/// Le carbone et le phosphore sont conservés exactement : la croissance puise
/// dans le carbone inorganique ou organique dissous et dans le phosphate, la
/// mort rend la biomasse en carbone organique dissous et en phosphate. Les
/// voies lumineuses consomment leur donneur et rejettent leurs produits en
/// proportion du carbone qu'elles fixent.
///
/// Une part de la nécromasse forme des particules qui coulent hors de la
/// couche (pompe biologique) : elle n'entre pas dans l'eau et est renvoyée
/// à l'appelant, qui la confie aux réservoirs globaux avec son phosphore.
pub fn substep(pops: &mut [Population], ctx: &CellContext, chem: &mut WaterChemistry, dt: f64, physio: &Physiology) -> SubstepOutput {
    let caps = capacities(pops, ctx, physio);
    substep_with(pops, &caps, ctx, chem, dt, physio)
}

/// [`substep`] avec les capacités déjà calculées (une par population).
pub fn substep_with(
    pops: &mut [Population],
    caps: &[Capacities],
    ctx: &CellContext,
    chem: &mut WaterChemistry,
    dt: f64,
    physio: &Physiology,
) -> SubstepOutput {
    let mut out = SubstepOutput::default();
    let volume = ctx.env.water_volume_m3;
    let cp = physio.carbon_to_phosphorus;
    evaluate_with(pops, caps, ctx, chem, physio);

    // Demande de chaque pool, en moles, pour le pas.
    let mut demand = [0.0; WATER_POOL_COUNT];
    for p in pops.iter() {
        let b = p.biomass * dt;
        let growth = p.rates.birth * b;
        for r in REACTIONS.iter() {
            let q = p.rates.reaction[r.id as usize];
            if q <= 0.0 {
                continue;
            }
            let units = if r.is_light() { growth * p.rates.fixation_share(r.id as usize) } else { q * b };
            for &(pool, k) in r.inputs {
                demand[pool as usize] += units * k;
            }
            for &(pool, k) in r.fixation {
                if k < 0.0 {
                    demand[pool as usize] -= growth * p.rates.fixation_share(r.id as usize) * k;
                }
            }
        }
        demand[WaterPool::Doc as usize] += growth * p.rates.doc_share;
        demand[WaterPool::Dic as usize] += growth * (1.0 - p.rates.heterotroph_share);
        demand[WaterPool::Po4 as usize] += growth / cp;
        // Digestion aérobie des proies.
        demand[WaterPool::O2 as usize] += p.rates.prey_uptake * p.rates.prey_aerobic_share * b;
    }
    // Prédation : chaque phagotrophe prend ce qu'il demande (digestion et
    // croissance) aux proies qu'il peut englober, au prorata de leur
    // biomasse ; une proie ne perd pas plus de la moitié de sa biomasse par
    // sous-pas. Le carbone pris aux proies est retiré avant la croissance ;
    // leur phosphore rejoint le phosphate de l'eau, où la croissance le
    // reprend. `got[j]` : part de sa demande que le prédateur `j` obtient.
    let mut got: Vec<f64> = Vec::new();
    let mut taken: Vec<f64> = Vec::new();
    if pops.iter().any(|p| p.phenotype.engulfment > 0.0 && p.rates.prey_demand() > 0.0) {
        let n = pops.len();
        got = vec![0.0; n];
        taken = vec![0.0; n];
        let mut loss = vec![0.0; n];
        let mut alloc: Vec<(usize, usize, f64)> = Vec::new();
        let wants: Vec<f64> = pops.iter().map(|p| p.rates.prey_demand() * p.biomass * dt).collect();
        for j in 0..n {
            let want = wants[j];
            if pops[j].phenotype.engulfment <= 0.0 || want <= 0.0 {
                continue;
            }
            let edible = edible_biomass(pops, j, physio);
            if edible <= 0.0 {
                continue;
            }
            let size = pops[j].phenotype.cell_size;
            for q in 0..n {
                let e = if q == j { 0.0 } else { edibility(size, pops[q].phenotype.body_size, physio) };
                if e > 0.0 {
                    let a = want * e * pops[q].biomass / edible;
                    alloc.push((j, q, a));
                    loss[q] += a;
                }
            }
        }
        let phi: Vec<f64> = (0..n).map(|q| if loss[q] > 0.5 * pops[q].biomass { 0.5 * pops[q].biomass / loss[q] } else { 1.0 }).collect();
        for &(j, q, a) in &alloc {
            got[j] += a * phi[q];
        }
        for q in 0..n {
            let l = loss[q] * phi[q];
            pops[q].biomass -= l;
            chem[WaterPool::Po4 as usize] += l / cp / volume;
        }
        for j in 0..n {
            let want = wants[j];
            taken[j] = got[j];
            got[j] = if want > 0.0 { (got[j] / want).min(1.0) } else { 0.0 };
        }
    }
    // Facteur de partage quand la demande dépasse le stock.
    let mut factor = [1.0; WATER_POOL_COUNT];
    for i in 0..WATER_POOL_COUNT {
        let available = chem[i].max(0.0) * volume * 0.95;
        if demand[i] > available {
            factor[i] = if demand[i] > 0.0 { available / demand[i] } else { 1.0 };
        }
    }
    let phi_of = |r: &crate::metabolism::Reaction| r.inputs.iter().map(|&(pool, _)| factor[pool as usize]).fold(1.0, f64::min);

    for (j, p) in pops.iter_mut().enumerate() {
        let b = p.biomass * dt;
        let potential = p.rates.birth * b;
        let prey_got = got.get(j).copied().unwrap_or(0.0);
        // Une voie limitée par l'un de ses intrants l'est en entier.
        let mut energy_scale = p.rates.supplement_kj;
        // Proies : leur digestion aérobie dépend aussi de l'O₂ disponible.
        let o2_phi = factor[WaterPool::O2 as usize];
        let digest_phi = prey_got * (p.rates.prey_aerobic_share * o2_phi + 1.0 - p.rates.prey_aerobic_share);
        energy_scale += p.rates.prey_energy_kj * digest_phi;
        for r in REACTIONS.iter() {
            let q = p.rates.reaction[r.id as usize];
            if q <= 0.0 {
                continue;
            }
            let phi = phi_of(r);
            if r.is_light() {
                energy_scale += q * phi;
                continue;
            }
            for &(pool, k) in r.inputs {
                chem[pool as usize] -= q * k * b * phi / volume;
            }
            for &(pool, k) in r.outputs {
                chem[pool as usize] += q * k * b * phi / volume;
            }
            let e = match r.energy {
                EnergySource::Chemical { dg_kj, .. } => q * dg_kj * (1.0 + physio.electron_transport_gain * p.phenotype.electron_transport),
                EnergySource::Light { .. } => q,
            };
            energy_scale += e * phi;
        }
        // Croissance réellement permise par l'énergie, le carbone et le phosphore obtenus.
        let energy_ratio = if p.rates.energy_kj > 0.0 { energy_scale / p.rates.energy_kj } else { 0.0 };
        let het = p.rates.heterotroph_share;
        let doc_share = p.rates.doc_share;
        let mut carbon_phi: f64 = factor[WaterPool::Po4 as usize];
        if doc_share > 0.0 {
            carbon_phi = carbon_phi.min(factor[WaterPool::Doc as usize]);
        }
        if p.rates.prey_share > 0.0 {
            carbon_phi = carbon_phi.min(prey_got);
        }
        if het < 1.0 {
            carbon_phi = carbon_phi.min(factor[WaterPool::Dic as usize]);
        }
        let scale = energy_ratio.min(1.0) * carbon_phi;
        // Chaque voie autotrophe fixe sa part du carbone, et pas plus que son
        // donneur d'électrons ne le permet (pouvoir réducteur des autotrophes
        // chimiques, donneur des voies lumineuses) : la biomasse fabriquée
        // porte exactement les électrons consommés.
        let mut births = potential * het * scale;
        for r in REACTIONS.iter() {
            let share = p.rates.fixation_share(r.id as usize);
            if share <= 0.0 || r.heterotrophic {
                continue;
            }
            let reductant = if r.is_light() {
                if p.rates.reaction[r.id as usize] <= 0.0 {
                    continue;
                }
                phi_of(r)
            } else {
                r.fixation.iter().filter(|&&(_, k)| k < 0.0).map(|&(pool, _)| factor[pool as usize]).fold(1.0, f64::min)
            };
            let fixed = potential * share * scale.min(reductant.max(0.0));
            if fixed <= 0.0 {
                continue;
            }
            births += fixed;
            chem[WaterPool::Dic as usize] -= fixed / volume;
            // Voie lumineuse : son donneur et ses produits ; voie chimique : le
            // réducteur de la fixation (coefficients négatifs : consommés).
            type Flows = &'static [(WaterPool, f64)];
            let (inputs, outputs): (Flows, Flows) = if r.is_light() { (r.inputs, r.outputs) } else { (&[], r.fixation) };
            for &(pool, k) in inputs {
                chem[pool as usize] -= fixed * k / volume;
            }
            for &(pool, k) in outputs {
                chem[pool as usize] += fixed * k / volume;
                if r.is_light() && pool == WaterPool::O2 {
                    out.oxygen += fixed * k;
                }
            }
        }
        // La prédation est retirée plus haut, explicitement.
        let deaths = p.biomass * (-(-(p.rates.mortality - p.rates.predation) * dt).dexp_m1());
        if let Some(&carbon) = taken.get(j).filter(|&&c| c > 0.0) {
            // Carbone pris aux proies : digéré (respiration ou fermentation),
            // incorporé, et le reste rendu à l'eau en matière organique.
            let digested = p.rates.prey_uptake * b * digest_phi;
            let aerobic = p.rates.prey_uptake * b * prey_got * p.rates.prey_aerobic_share * o2_phi;
            let anaerobic = digested - aerobic;
            let incorporated = potential * p.rates.prey_share * scale;
            let rest = (carbon - digested - incorporated).max(0.0);
            chem[WaterPool::O2 as usize] -= aerobic / volume;
            chem[WaterPool::Dic as usize] += (aerobic + 0.5 * anaerobic) / volume;
            chem[WaterPool::Ch4 as usize] += 0.5 * anaerobic / volume;
            chem[WaterPool::Doc as usize] += rest / volume;
            debug_assert!(digested + incorporated <= carbon * (1.0 + 1e-9) + 1e-12, "proies : {digested} + {incorporated} > {carbon}");
        }
        chem[WaterPool::Doc as usize] -= potential * doc_share * scale / volume;
        chem[WaterPool::Po4 as usize] -= births / cp / volume;
        let sinking = deaths * physio.sinking_share;
        out.sinking_carbon += sinking;
        chem[WaterPool::Doc as usize] += (deaths - sinking) / volume;
        chem[WaterPool::Po4 as usize] += (deaths - sinking) / cp / volume;
        p.biomass += births - deaths;
    }
    for c in chem.iter_mut() {
        if *c < 0.0 {
            // Arrondis seulement : le partage garantit la positivité.
            debug_assert!(*c > -1e-9, "concentration négative {c}");
            *c = 0.0;
        }
    }
    out
}

/// Ce qui sort d'une cellule pendant un sous-pas, en moles.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SubstepOutput {
    /// O₂ produit par la photosynthèse oxygénique (production brute).
    pub oxygen: f64,
    /// Carbone des particules qui coulent ; elles emportent leur phosphore
    /// au rapport de la biomasse.
    pub sinking_carbon: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metabolism::*;
    use evo_genetics::{Domain, DomainFamily, Gene};

    fn env() -> CellEnvironment {
        CellEnvironment {
            latitude_rad: 0.0,
            elevation_m: -3000.0,
            is_ocean: true,
            area_m2: 1e10,
            water_volume_m3: 1e12,
            water_area_m2: 1e10,
            flushing_per_year: 0.0,
            rain_mm_yr: 1000.0,
            temperature_k: 300.0,
            seasonal_amplitude_k: 0.0,
            light_par_w_m2: 50.0,
            uv_w_m2: 10.0,
            ph: 7.2,
            salinity: 35.0,
            pressure_pa: 6e5,
            vent_h2_supply: 0.0,
            vent_h2s_supply: 0.0,
            vent_fe_supply: 0.0,
            vent_mn_supply: 0.0,
            ice_cover: 0.0,
        }
    }

    fn pop(reactions: &[u8], biomass: f64, physio: &Physiology) -> Population {
        let genome = Genome {
            genes: reactions
                .iter()
                .map(|&r| Gene {
                    domain: Domain {
                        family: DomainFamily::Catalytic(r),
                        efficiency: 1.0,
                        affinity: 1.0,
                        t_opt_k: 300.0,
                        t_width_k: 10.0,
                        absorption_nm: 500.0,
                    },
                    functional: true,
                })
                .collect(),
            marker: [0; 32],
            organelles: Vec::new(),
        };
        let phenotype = Arc::new(Phenotype::from_genome(&genome, physio));
        Population { lineage: 0, genome: Arc::new(genome), phenotype, biomass, rates: GrowthRates::default() }
    }

    fn with_genes(genes: &[(DomainFamily, f64)], biomass: f64, physio: &Physiology) -> Population {
        let genome = Genome::new(
            genes
                .iter()
                .map(|&(family, efficiency)| Gene {
                    domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: 500.0 },
                    functional: true,
                })
                .collect(),
            [0; 32],
        );
        let phenotype = Arc::new(Phenotype::from_genome(&genome, physio));
        Population { lineage: 0, genome: Arc::new(genome), phenotype, biomass, rates: GrowthRates::default() }
    }

    #[test]
    fn phagotrophs_eat_smaller_cells_and_conserve_carbon() {
        use DomainFamily::*;
        let physio = Physiology::default();
        let e = env();
        let ctx = CellContext { env: &e, light_biomass_per_m2: 0.1 };
        let mut chem = [0.0; WATER_POOL_COUNT];
        chem[WaterPool::Dic as usize] = 8.0;
        chem[WaterPool::H2 as usize] = 5e-2;
        chem[WaterPool::O2 as usize] = 1e-2;
        chem[WaterPool::Po4 as usize] = 1e-3;
        let prey = with_genes(&[(Catalytic(METHANOGENESIS), 1.0)], 1e9, &physio);
        let predator = with_genes(
            &[(Catalytic(FERMENTATION), 1.0), (Catalytic(AEROBIC_RESPIRATION), 0.5), (Cytoskeleton, 0.5), (OxidativeDefense, 1.0)],
            1e7,
            &physio,
        );
        assert!(predator.phenotype.is_phagotroph());
        let mut pops = vec![prey, predator];
        let v = e.water_volume_m3;
        let before = carbon(&pops, &chem, v);
        let p_before = phosphorus(&pops, &chem, v, &physio);
        let mut sunk = 0.0;
        let start = pops[1].biomass;
        for _ in 0..30 {
            sunk += substep(&mut pops, &ctx, &mut chem, 1.0 / 3650.0, &physio).sinking_carbon;
        }
        assert!(pops[1].rates.prey_uptake > 0.0);
        assert!(pops[0].rates.predation > 0.0);
        assert!(pops[1].biomass > start, "le prédateur croît : {} contre {start}", pops[1].biomass);
        let after = carbon(&pops, &chem, v) + sunk;
        assert!((after - before).abs() / before < 1e-9, "carbone avant {before}, après {after}");
        let p_after = phosphorus(&pops, &chem, v, &physio) + sunk / physio.carbon_to_phosphorus;
        assert!((p_after - p_before).abs() / p_before < 1e-9, "phosphore avant {p_before}, après {p_after}");
    }

    fn carbon(pops: &[Population], chem: &WaterChemistry, v: f64) -> f64 {
        pops.iter().map(|p| p.biomass).sum::<f64>()
            + (chem[WaterPool::Dic as usize] + chem[WaterPool::Doc as usize] + chem[WaterPool::Ch4 as usize]) * v
    }

    fn phosphorus(pops: &[Population], chem: &WaterChemistry, v: f64, physio: &Physiology) -> f64 {
        pops.iter().map(|p| p.biomass).sum::<f64>() / physio.carbon_to_phosphorus + chem[WaterPool::Po4 as usize] * v
    }

    #[test]
    fn closed_community_conserves_carbon_and_recycles_necromass() {
        let physio = Physiology::default();
        let e = env();
        let ctx = CellContext { env: &e, light_biomass_per_m2: 0.1 };
        let mut chem = [0.0; WATER_POOL_COUNT];
        chem[WaterPool::Dic as usize] = 8.0;
        chem[WaterPool::H2 as usize] = 5e-2;
        chem[WaterPool::Po4 as usize] = 1e-3;
        let mut pops = vec![pop(&[METHANOGENESIS], 1e3, &physio), pop(&[FERMENTATION], 1e3, &physio)];
        let before = carbon(&pops, &chem, e.water_volume_m3);
        let p_before = phosphorus(&pops, &chem, e.water_volume_m3, &physio);
        // Les particules qui coulent sortent de la cellule : on les compte à part.
        let mut sunk = 0.0;
        for _ in 0..2000 {
            sunk += substep(&mut pops, &ctx, &mut chem, 1.0 / 3650.0, &physio).sinking_carbon;
        }
        assert!(sunk > 0.0);
        let after = carbon(&pops, &chem, e.water_volume_m3) + sunk;
        assert!((after - before).abs() / before < 1e-9, "avant {before}, après {after}");
        let p_after = phosphorus(&pops, &chem, e.water_volume_m3, &physio) + sunk / physio.carbon_to_phosphorus;
        assert!((p_after - p_before).abs() / p_before < 1e-9, "phosphore avant {p_before}, après {p_after}");
        // Les méthanogènes ont consommé l'hydrogène et produit du méthane.
        assert!(chem[WaterPool::H2 as usize] < 5e-2);
        assert!(chem[WaterPool::Ch4 as usize] > 0.0);
        // Les fermenteurs vivent de la nécromasse des méthanogènes.
        assert!(pops[1].biomass > 0.0);
        assert!(chem.iter().all(|&c| c >= 0.0));
    }
}
