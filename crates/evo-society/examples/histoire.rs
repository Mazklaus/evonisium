//! Déroule une histoire sur un banc et en imprime le résumé.
//!
//! `cargo run --release -p evo-society --example histoire -- [terre|desert|ocean|deux|moderne] [graine] [années]`

use evo_society::bench;
use evo_society::society::Label;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let which = args.get(1).map(String::as_str).unwrap_or("terre");
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let years: i64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(17_000);
    let mut b = match which {
        "desert" => bench::desert(seed),
        "ocean" => bench::ocean(seed),
        "deux" => bench::two_species(seed),
        "moderne" => bench::modern(seed),
        _ => bench::earth(seed),
    };
    println!("{} cellules, {} sociétés au départ", b.world.cells.len(), b.world.alive_societies().count());
    let t0 = Instant::now();
    let mut seen = 0usize;
    let (mut d_epi, mut d_war, mut famine) = (0.0, 0.0, 0.0);
    let ncat = evo_society::catalog::catalog().len();
    let mut first: Vec<Option<i64>> = vec![None; ncat];
    for i in 1..=years {
        b.step();
        if i % 25 == 0 {
            for soc in b.world.alive_societies() {
                for t in soc.known.iter() {
                    first[t as usize].get_or_insert(b.world.year);
                }
            }
        }
        for c in b.world.cells.iter().filter_map(|c| c.settlement.as_ref()) {
            d_epi += c.deaths_epidemic;
            d_war += c.deaths_war;
            if c.food_pc < 0.85 {
                famine += c.pop;
            }
        }
        let every = if years <= 400 { 25 } else { 250 };
        if i % every == 0 {
            let pop: f64 = b.world.cells.iter().filter_map(|c| c.settlement.as_ref()).map(|s| s.pop).sum();
            let crisis: f64 = b.world.alive_societies().filter(|s| s.crisis > 0).map(|s| s.agg.population).sum();
            let cat = evo_society::catalog::catalog();
            let (ar, cu) = (cat.id("araire").unwrap(), cat.id("culture_cereales").unwrap());
            let (mut crop, mut soil, mut fpop, mut kar, mut kcu, mut lv) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            for c in &b.world.cells {
                crop += c.eco.cropland_km2;
                lv += c.eco.livestock;
                if let Some(s) = &c.settlement {
                    if c.eco.cropland_km2 > 0.0 {
                        soil += c.eco.soil * s.pop;
                        fpop += s.pop;
                    }
                    if s.known.has(ar) {
                        kar += s.pop;
                    }
                    if s.known.has(cu) {
                        kcu += s.pop;
                    }
                }
            }
            println!(
                "           champs {:.0} k km², sol {:.2}, bétail {:.0} M, agriculteurs {:.1} M, araire {:.1} M, céréales {:.1} M",
                crop / 1e3,
                soil / fpop.max(1.0),
                lv / 1e6,
                fpop / 1e6,
                kar / 1e6,
                kcu / 1e6
            );
            println!(
                "    {:>6} pop {:.1} M, morts/an : épidémies {:.2} M, guerres {:.2} M ; en famine {:.1} M·an/an ; en crise {:.1} M",
                b.world.year,
                pop / 1e6,
                d_epi / (every as f64 * 1e6),
                d_war / (every as f64 * 1e6),
                famine / (every as f64 * 1e6),
                crisis / 1e6
            );
            let f = b.world.forcing_parts();
            if f.fossil_co2 + f.land_co2 + f.methane > 0.01 {
                let modern: f64 = b.world.alive_societies().map(|s| s.agg.population * s.agg.modern_energy).sum::<f64>() / pop.max(1.0);
                println!(
                    "           forçage : CO₂ fossile {:.3}, CO₂ des terres {:.3}, méthane {:.3}, aérosols {:.3} W/m² ; énergie moderne {:.3}",
                    f.fossil_co2, f.land_co2, f.methane, f.aerosol, modern
                );
            }
            d_epi = 0.0;
            d_war = 0.0;
            famine = 0.0;
        }
        for e in &b.world.milestones[seen..] {
            println!("  {:>6} {:?} {}", e.year, e.kind, e.text);
        }
        seen = b.world.milestones.len();
        if i % 1000 == 0 {
            let w = &b.world;
            let alive: Vec<_> = w.alive_societies().collect();
            let count = |l: Label| alive.iter().filter(|s| s.label == l).count();
            let pop: f64 = w.cells.iter().filter_map(|c| c.settlement.as_ref()).map(|s| s.pop).sum();
            let gdp: f64 = w.cells.iter().filter_map(|c| c.settlement.as_ref()).map(|s| s.pop * s.gdp_pc).sum();
            let food: f64 = w.cells.iter().filter_map(|c| c.settlement.as_ref()).map(|s| s.pop * s.food_pc).sum();
            let tech = alive.iter().map(|s| s.agg.techniques).max().unwrap_or(0);
            println!(
                "{:>6} pop {:>7.1} M  soc {:>4} (b{} t{} c{} É{} E{} M{})  pib/h {:.2} rations/h {:.2}  tech max {:>3}  forçage {:.3}  {:.0} a/s  {:?}",
                w.year,
                pop / 1e6,
                alive.len(),
                count(Label::Bands),
                count(Label::Tribe),
                count(Label::Chiefdom),
                count(Label::State),
                count(Label::Empire),
                count(Label::ModernState),
                gdp / pop.max(1.0),
                food / pop.max(1.0),
                tech,
                w.forcing_w_m2(),
                i as f64 / t0.elapsed().as_secs_f64(),
                w.counters,
            );
            if let Some(top) = alive.iter().max_by_key(|s| s.agg.techniques) {
                let cat = evo_society::catalog::catalog();
                let last: Vec<&str> = top.known.iter().map(|t| cat.get(t).key.as_str()).collect();
                println!(
                    "        tête : {} {:?} pop {:.0} k, {} cellules, urbain {:.2}, centr {:.2} admin {:.2} ; {}",
                    top.name(),
                    top.label,
                    top.agg.population / 1e3,
                    top.cells.len(),
                    top.agg.urban,
                    top.dims.centralisation,
                    top.dims.admin,
                    last[last.len().saturating_sub(8)..].join(" ")
                );
                let cell = &w.cells[top.capital as usize];
                if let (Some(st), Some(y)) = (&cell.settlement, &cell.year) {
                    let l = &cell.land;
                    println!(
                        "        capitale ({:.0},{:.0}) {:.0} km² npp {:.2} T {:.0} pluie {:.0} côte {} fleuve {:.1} : pop {:.0} k, sauvage {:.0} k poisson {:.0} k grain {:.0} k racines {:.0} k bétail {:.0} k, champs {:.0} km², culture {:?}",
                        l.lat_deg, l.lon_deg, l.area_km2, l.npp(), l.temp_c, l.rain_mm, l.coast, l.river, st.pop / 1e3,
                        y.produced[0] / 1e3, y.produced[5] / 1e3, y.produced[1] / 1e3, y.produced[2] / 1e3, y.produced[4] / 1e3, cell.eco.cropland_km2, st.crop.map(|c| c.species)
                    );
                }
            }
        }
    }
    let cat = evo_society::catalog::catalog();
    if let Some(top) = b.world.alive_societies().max_by_key(|s| s.agg.techniques) {
        let names: Vec<&str> = top.known.iter().map(|t| cat.get(t).key.as_str()).collect();
        println!("techniques des {} : {}", top.name(), names.join(" "));
    }
    println!("bilan carbone {:.3e} t C sur {:.3e}", b.world.carbon_balance(), b.world.carbon_initial);
    let mut dated: Vec<(i64, &str)> =
        first.iter().enumerate().filter_map(|(t, y)| y.map(|y| (y, cat.get(t as u16).key.as_str()))).collect();
    dated.sort();
    let line: Vec<String> = dated.iter().map(|(y, k)| format!("{k} {y}")).collect();
    println!("premières dates : {}", line.join(", "));
    if let Some(best) = b.world.alive_societies().max_by_key(|s| s.known.count()) {
        let mut ready = Vec::new();
        for t in 0..cat.len() as u16 {
            if best.known.has(t) || !cat.ready(t, &best.known) {
                continue;
            }
            let tech = cat.get(t);
            let sci: Vec<String> = tech
                .science
                .iter()
                .filter(|&&(d, l)| best.sciences[d.index()].precision < l)
                .map(|&(d, l)| format!("{}<{l}:{:.2}", d.key(), best.sciences[d.index()].precision))
                .collect();
            ready.push(format!("{}[{}]", tech.key, sci.join(" ")));
        }
        println!("à portée des {} (science qui manque) : {}", best.name(), ready.join(", "));
    }
}
