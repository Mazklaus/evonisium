//! Suivi d'un monde : cellules occupées, populations, biomasse et voies
//! principales, toutes les N années (diagnostic de déclin).
//!
//!   cargo run --release -p evo-sim --example suivi_monde -- --world ocean
//!     [--seed N] [--level L] [--until ANS] [--every ANS] [--round-years ANS] [--substeps N]
use evo_life::metabolism::guild_label;
use evo_planet::{Gas, PlanetParams, WaterPool};
use evo_sim::orders::OrderKind;
use evo_sim::{World, WorldConfig};
use std::collections::BTreeMap;

fn opt<T: std::str::FromStr>(a: &[String], name: &str) -> Option<T> {
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let key: String = opt(&a, "--world").unwrap_or_else(|| "terre".into());
    let params = PlanetParams::by_key(&key).expect("monde inconnu");
    let mut cfg = WorldConfig::with_planet(params, opt(&a, "--seed").unwrap_or(2026), opt(&a, "--level").unwrap_or(4));
    cfg.step_years = 100_000.0;
    if let Some(r) = opt::<f64>(&a, "--round-years") {
        cfg.evolution.round_years = Some(r);
    }
    if let Some(n) = opt(&a, "--substeps") {
        cfg.eco_substeps = n;
    }
    let until: f64 = opt(&a, "--until").unwrap_or(1e9);
    let every: f64 = opt(&a, "--every").unwrap_or(5e7);
    let mut w = World::new(cfg);
    w.orders.submit(0.0, OrderKind::SeedLife);
    let mut next = every;
    while w.years() < until {
        w.step();
        if w.years() < next && std::env::var("SUIVI_DE").ok().and_then(|v| v.parse::<f64>().ok()).is_none_or(|d| w.years() < d) {
            continue;
        }
        next += every;
        let (mut cells, mut pops, mut biomass, mut po4, mut wet) = (0, 0, 0.0, 0.0, 0);
        let (mut h2, mut temp, mut tmin, mut tmax) = (0.0, 0.0, f64::INFINITY, 0.0f64);
        let mut guilds: BTreeMap<String, usize> = BTreeMap::new();
        let (mut birth, mut death) = (0.0, 0.0);
        for (c, p) in w.communities.iter().enumerate() {
            if w.bio.env[c].water_volume_m3 > 0.0 {
                wet += 1;
                po4 += w.chemistry[c][WaterPool::Po4 as usize];
                h2 += w.chemistry[c][WaterPool::H2 as usize];
                let t = w.bio.env[c].temperature_k;
                temp += t;
                tmin = tmin.min(t);
                tmax = tmax.max(t);
            }
            if p.is_empty() {
                continue;
            }
            cells += 1;
            pops += p.len();
            for q in p {
                biomass += q.biomass;
                birth += q.biomass * q.rates.birth;
                death += q.biomass * q.rates.mortality;
                *guilds.entry(q.phenotype.main_pathway().map_or("aucune".into(), |r| guild_label(1 << r))).or_default() += 1;
            }
        }
        println!(
            "{:.0} Ma : {cells}/{wet} cellules, {pops} populations, biomasse {biomass:.2e} molC (naissance {:.1}, mort {:.1} /an), PO₄ {:.2e} H₂ {:.2e} mol/m³, T {:.0} K ({tmin:.0} à {tmax:.0}), air H₂ {:.1} CH₄ {:.1} Pa | {guilds:?}",
            w.years() / 1e6,
            birth / biomass.max(1e-30),
            death / biomass.max(1e-30),
            po4 / wet.max(1) as f64,
            h2 / wet.max(1) as f64,
            temp / wet.max(1) as f64,
            w.planet.partial_pressure(Gas::H2),
            w.planet.partial_pressure(Gas::Ch4)
        );
        if pops == 0 {
            break;
        }
    }
}
