//! Sonde : prépare un monde mûr (ou le relit) et mesure le coefficient de
//! sélection de variantes d'organisation (cytosquelette, adhésion…) contre
//! les résidents.
use evo_genetics::{Domain, DomainFamily, Gene};
use evo_life::community::CellContext;
use evo_life::{growth_rates, selection_coefficient, Phenotype};
use evo_sim::orders::OrderKind;
use evo_sim::{World, WorldConfig};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = Path::new("/tmp/claude-0/runs/mur-l4.sauvegarde");
    let mut world = if path.exists() && !args.iter().any(|a| a == "--prepare") {
        World::load_file(path).expect("lecture")
    } else {
        let mut cfg = WorldConfig::new(2026, 4);
        cfg.step_years = 200_000.0;
        let mut w = World::new(cfg);
        w.orders.submit(0.0, OrderKind::SeedLife);
        let years: f64 =
            args.iter().position(|a| a == "--prepare").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(200e6);
        while w.years() < years {
            w.step();
        }
        w.save_file(path).expect("écriture");
        w
    };
    let mut physio = world.config.physiology.clone();
    if let Some(c) = std::env::var("SONDE_CYTO_COST").ok().and_then(|v| v.parse().ok()) {
        physio.cytoskeleton_cost_kj = c;
    }
    if let Ok(c) = std::env::var("SONDE_ENGULF") {
        let v: Vec<f64> = c.split(',').map(|x| x.parse().unwrap()).collect();
        (physio.engulf_min_ratio, physio.engulf_full_ratio) = (v[0], v[1]);
    }
    let mut tried = 0;
    let mut best: Vec<(f64, f64)> = vec![(f64::MIN, 0.0); 6];
    let cytos = [0.0, 0.05, 0.1, 0.2, 0.4, 0.8];
    let mut wins = [0usize; 6];
    for c in 0..world.communities.len() {
        let pops = &world.communities[c];
        if pops.is_empty() {
            continue;
        }
        let ctx = CellContext { env: &world.bio.env[c], light_biomass_per_m2: world.config.light_biomass_per_m2 };
        let cond = ctx.conditions_of(pops, &physio);
        for p in pops {
            if p.phenotype.enzymes.iter().all(|e| e.reaction != 1 && e.reaction != 2) || p.rates.birth <= 0.0 {
                continue;
            }
            tried += 1;
            let resident = growth_rates(&p.phenotype, &cond, &world.chemistry[c], &physio);
            for (k, &cy) in cytos.iter().enumerate() {
                let mut g = (*p.genome).clone();
                g.genes.retain(|x| x.domain.family != DomainFamily::Cytoskeleton);
                if cy > 0.0 {
                    g.genes.push(Gene {
                        domain: Domain {
                            family: DomainFamily::Cytoskeleton,
                            efficiency: cy,
                            affinity: 1.0,
                            t_opt_k: 300.0,
                            t_width_k: 10.0,
                            absorption_nm: 500.0,
                        },
                        functional: true,
                    });
                }
                let ph = Phenotype::from_genome(&g, &physio);
                let r = growth_rates(&ph, &cond, &world.chemistry[c], &physio);
                let s = selection_coefficient(&r, &resident, &physio);
                if s > 0.0 {
                    wins[k] += 1;
                }
                if s > best[k].0 {
                    best[k] = (s, r.prey_uptake);
                }
            }
        }
    }
    println!("populations hétérotrophes essayées : {tried}");
    // Endosymbiose : rétentions attendues et coefficient de sélection.
    let mut found = Vec::new();
    for c in 0..world.communities.len() {
        let pops = &world.communities[c];
        let ctx = CellContext { env: &world.bio.env[c], light_biomass_per_m2: world.config.light_biomass_per_m2 };
        let cond = ctx.conditions_of(pops, &physio);
        for (i, p) in pops.iter().enumerate() {
            if !p.phenotype.is_phagotroph() {
                continue;
            }
            let ne = p.census(&physio);
            let mut rng = evo_core::rng::rng_for(1, evo_core::rng::Stream::Validation, &[c as u64, i as u64]);
            for partner in evo_sim::transitions::engulfed_partners(p, i, pops, ne, 1e5, world.years(), &world.config, &mut rng) {
                let ph = Phenotype::from_genome(&partner.change.genome, &physio);
                let resident = growth_rates(&p.phenotype, &cond, &world.chemistry[c], &physio);
                let r = growth_rates(&ph, &cond, &world.chemistry[c], &physio);
                let s = selection_coefficient(&r, &resident, &physio);
                found.push((partner.copies, s, ph.is_eukaryote(), r.prey_aerobic_share));
            }
        }
    }
    found.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("partenaires candidats : {}", found.len());
    for f in found.iter().take(8) {
        println!("  rétentions/tour {:.2e} s {:.3} aérobie {:.2}", f.0, f.1, f.3);
    }
    let pos: Vec<_> = found.iter().filter(|f| f.1 > 0.0).collect();
    let total: f64 = pos.iter().map(|f| f.0 * (2.0 * f.1).min(1.0)).sum();
    println!("  s>0 : {} ; fixations attendues par tour ≈ {:.2e}", pos.len(), total);
    for (k, &cy) in cytos.iter().enumerate() {
        println!(
            "cytosquelette {cy:.2} (taille {:.2}) : s>0 dans {} cas, meilleur s {:.3} (proies {:.1})",
            1.0 + physio.cytoskeleton_size * cy,
            wins[k],
            best[k].0,
            best[k].1
        );
    }
    // Multicellularité : variantes d'adhésion des proies exposées aux
    // prédateurs.
    let variants: [(f64, f64); 7] = [(0.2, 600.0), (0.4, 600.0), (0.6, 600.0), (0.9, 600.0), (1.2, 600.0), (0.9, 400.0), (0.9, 900.0)];
    let mut wins = vec![0usize; variants.len()];
    let mut best = vec![f64::MIN; variants.len()];
    let mut exposed = 0;
    let mut preds = Vec::new();
    for c in 0..world.communities.len() {
        let pops = &world.communities[c];
        let ctx = CellContext { env: &world.bio.env[c], light_biomass_per_m2: world.config.light_biomass_per_m2 };
        let cond = ctx.conditions_of(pops, &physio);
        for p in pops {
            let resident = growth_rates(&p.phenotype, &cond, &world.chemistry[c], &physio);
            preds.push(resident.predation);
            if resident.predation < 1.0 || resident.birth <= 0.0 {
                continue;
            }
            exposed += 1;
            for (k, &(a, nm)) in variants.iter().enumerate() {
                let mut g = (*p.genome).clone();
                g.genes.push(Gene {
                    domain: Domain {
                        family: DomainFamily::Adhesion,
                        efficiency: a,
                        affinity: 1.0,
                        t_opt_k: 300.0,
                        t_width_k: 10.0,
                        absorption_nm: nm,
                    },
                    functional: true,
                });
                let ph = Phenotype::from_genome(&g, &physio);
                let r = growth_rates(&ph, &cond, &world.chemistry[c], &physio);
                let s = selection_coefficient(&r, &resident, &physio);
                if s > 0.0 {
                    wins[k] += 1;
                }
                best[k] = best[k].max(s);
            }
        }
    }
    let mut ph: Vec<(f64, f64, f64, f64, f64)> = world
        .communities
        .iter()
        .flatten()
        .filter(|p| p.phenotype.is_phagotroph())
        .map(|p| (p.rates.prey_uptake, p.rates.prey_share, p.phenotype.cell_size, p.biomass, p.rates.r))
        .collect();
    ph.sort_by(|a, b| a.0.total_cmp(&b.0));
    let m = ph.len().max(1);
    if !ph.is_empty() {
        println!(
            "phagotrophes : {} ; proies digérées médiane {:.1} q90 {:.1} ; part du carbone des proies médiane {:.2} ; taille médiane {:.1}",
            ph.len(),
            ph[m / 2].0,
            ph[9 * m / 10].0,
            {
                let mut v: Vec<f64> = ph.iter().map(|x| x.1).collect();
                v.sort_by(f64::total_cmp);
                v[m / 2]
            },
            {
                let mut v: Vec<f64> = ph.iter().map(|x| x.2).collect();
                v.sort_by(f64::total_cmp);
                v[m / 2]
            },
        );
        let mut sizes: Vec<f64> =
            world.communities.iter().flatten().filter(|p| !p.phenotype.is_phagotroph()).map(|p| p.phenotype.body_size).collect();
        sizes.sort_by(f64::total_cmp);
        let k = sizes.len();
        println!("tailles des non-phagotrophes : médiane {:.2} q90 {:.2} max {:.2}", sizes[k / 2], sizes[9 * k / 10], sizes[k - 1]);
    }
    preds.sort_by(f64::total_cmp);
    let n = preds.len();
    println!(
        "prédation (an⁻¹) : médiane {:.2} q90 {:.1} q99 {:.1} max {:.1} ; populations exposées (>1) : {exposed}",
        preds[n / 2],
        preds[9 * n / 10],
        preds[99 * n / 100],
        preds[n - 1]
    );
    for (k, &(a, nm)) in variants.iter().enumerate() {
        let g = evo_genetics::Genome::new(
            vec![Gene {
                domain: Domain {
                    family: DomainFamily::Adhesion,
                    efficiency: a,
                    affinity: 1.0,
                    t_opt_k: 300.0,
                    t_width_k: 10.0,
                    absorption_nm: nm,
                },
                functional: true,
            }],
            [0; 32],
        );
        let ph = Phenotype::from_genome(&g, &physio);
        println!(
            "adhésion {a:.1} ({nm} nm) : {} cellules, taille vue {:.1} ; s>0 dans {} cas, meilleur s {:.2}",
            ph.cells(),
            ph.body_size,
            wins[k],
            best[k]
        );
    }

    // Suite de la partie : organisation tous les 5 Ma.
    let years: f64 = args.iter().position(|a| a == "--run").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let end = world.years() + years;
    let mut next = world.years() + 5e6;
    while world.years() < end {
        world.step();
        if world.years() >= next {
            next += 5e6;
            let (mut n_ph, mut b_ph, mut b, mut max_size) = (0, 0.0, 0.0, 1.0f64);
            for p in world.communities.iter().flatten() {
                b += p.biomass;
                max_size = max_size.max(p.phenotype.cell_size);
                if p.phenotype.is_phagotroph() {
                    n_ph += 1;
                    b_ph += p.biomass;
                }
            }
            println!(
                "{:.0} Ma : {n_ph} populations phagotrophes ({:.2} % de la biomasse), taille max {max_size:.2}, substitutions {}",
                world.years() / 1e6,
                100.0 * b_ph / b,
                world.stats.substitutions
            );
        }
    }
}
