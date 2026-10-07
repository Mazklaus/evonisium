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

use crate::growth::{growth_rates, Conditions, GrowthRates, Physiology};
use crate::metabolism::{EnergySource, REACTIONS};
use crate::phenotype::Phenotype;
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
        let b_ref = self.light_biomass_per_m2 * self.env.area_m2;
        let incoming = watts_to_kj_per_year(self.env.light_par_w_m2 * self.env.area_m2);
        let x = photo_biomass / b_ref;
        if x < 1e-9 {
            // Limite à faible biomasse : toute la lumière est disponible.
            incoming / b_ref
        } else {
            incoming * (-(-x).exp_m1()) / photo_biomass
        }
    }

    pub fn photo_biomass(pops: &[Population]) -> f64 {
        pops.iter().filter(|p| p.phenotype.phototroph).map(|p| p.biomass).sum()
    }

    /// Conditions vues par les organismes quand la biomasse phototrophe de la
    /// cellule vaut `photo_biomass`.
    pub fn conditions(&self, photo_biomass: f64) -> Conditions {
        Conditions { temperature_k: self.env.temperature_k, uv_w_m2: self.env.uv_w_m2, light_kj: self.light_per_biomass(photo_biomass) }
    }
}

/// Évalue toutes les populations d'une cellule dans l'état courant.
pub fn evaluate(pops: &mut [Population], ctx: &CellContext, chem: &WaterChemistry, physio: &Physiology) {
    let cond = ctx.conditions(CellContext::photo_biomass(pops));
    for p in pops.iter_mut() {
        p.rates = growth_rates(&p.phenotype, &cond, chem, physio);
    }
}

/// Avance la communauté d'une cellule de `dt` années (pas explicite court).
/// Le carbone et le phosphore sont conservés exactement : la croissance puise
/// dans le carbone inorganique ou organique dissous et dans le phosphate, la
/// mort rend la biomasse en carbone organique dissous et en phosphate. Les
/// voies lumineuses consomment leur donneur et rejettent leurs produits en
/// proportion du carbone qu'elles fixent. Renvoie l'O₂ produit par la
/// photosynthèse oxygénique pendant le pas, en moles (production brute).
pub fn substep(pops: &mut [Population], ctx: &CellContext, chem: &mut WaterChemistry, dt: f64, physio: &Physiology) -> f64 {
    let mut oxygen = 0.0;
    let volume = ctx.env.water_volume_m3;
    let cp = physio.carbon_to_phosphorus;
    evaluate(pops, ctx, chem, physio);

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
            let units = if r.is_light() { growth * p.rates.light_fixation_share(r.id as usize) } else { q * b };
            for &(pool, k) in r.inputs {
                demand[pool as usize] += units * k;
            }
        }
        demand[WaterPool::Doc as usize] += growth * p.rates.heterotroph_share;
        demand[WaterPool::Dic as usize] += growth * (1.0 - p.rates.heterotroph_share);
        demand[WaterPool::Po4 as usize] += growth / cp;
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

    for p in pops.iter_mut() {
        let b = p.biomass * dt;
        let potential = p.rates.birth * b;
        // Une voie limitée par l'un de ses intrants l'est en entier.
        let mut energy_scale = p.rates.supplement_kj;
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
        let mut carbon_phi: f64 = factor[WaterPool::Po4 as usize];
        if het > 0.0 {
            carbon_phi = carbon_phi.min(factor[WaterPool::Doc as usize]);
        }
        if het < 1.0 {
            carbon_phi = carbon_phi.min(factor[WaterPool::Dic as usize]);
        }
        let births = potential * energy_ratio.min(1.0) * carbon_phi;
        let scale = if potential > 0.0 { births / potential } else { 0.0 };
        // Stoichiométrie des voies lumineuses, au prorata du carbone fixé.
        for r in REACTIONS.iter().filter(|r| r.is_light()) {
            if p.rates.reaction[r.id as usize] <= 0.0 {
                continue;
            }
            let fixed = potential * p.rates.light_fixation_share(r.id as usize) * scale.min(phi_of(r));
            for &(pool, k) in r.inputs {
                chem[pool as usize] -= fixed * k / volume;
            }
            for &(pool, k) in r.outputs {
                chem[pool as usize] += fixed * k / volume;
                if pool == WaterPool::O2 {
                    oxygen += fixed * k;
                }
            }
        }
        let deaths = p.biomass * (-(-p.rates.mortality * dt).exp_m1());
        chem[WaterPool::Doc as usize] -= births * het / volume;
        chem[WaterPool::Dic as usize] -= births * (1.0 - het) / volume;
        chem[WaterPool::Po4 as usize] -= births / cp / volume;
        chem[WaterPool::Doc as usize] += deaths / volume;
        chem[WaterPool::Po4 as usize] += deaths / cp / volume;
        p.biomass += births - deaths;
    }
    for c in chem.iter_mut() {
        if *c < 0.0 {
            // Arrondis seulement : le partage garantit la positivité.
            debug_assert!(*c > -1e-9, "concentration négative {c}");
            *c = 0.0;
        }
    }
    oxygen
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
        };
        let phenotype = Arc::new(Phenotype::from_genome(&genome, physio));
        Population { lineage: 0, genome: Arc::new(genome), phenotype, biomass, rates: GrowthRates::default() }
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
        for _ in 0..2000 {
            substep(&mut pops, &ctx, &mut chem, 1.0 / 3650.0, &physio);
        }
        let after = carbon(&pops, &chem, e.water_volume_m3);
        assert!((after - before).abs() / before < 1e-9, "avant {before}, après {after}");
        let p_after = phosphorus(&pops, &chem, e.water_volume_m3, &physio);
        assert!((p_after - p_before).abs() / p_before < 1e-9, "phosphore avant {p_before}, après {p_after}");
        // Les méthanogènes ont consommé l'hydrogène et produit du méthane.
        assert!(chem[WaterPool::H2 as usize] < 5e-2);
        assert!(chem[WaterPool::Ch4 as usize] > 0.0);
        // Les fermenteurs vivent de la nécromasse des méthanogènes.
        assert!(pops[1].biomass > 0.0);
        assert!(chem.iter().all(|&c| c >= 0.0));
    }
}
