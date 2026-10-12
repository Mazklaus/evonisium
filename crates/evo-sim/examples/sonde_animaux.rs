//! Sonde des animaux : relit une sauvegarde et mesure, dans les cellules où
//! vivent des eucaryotes multicellulaires, ce que vaudraient des variantes
//! hétérotrophes mobiles des résidents (proies, naissances, coefficient de
//! sélection).
use evo_genetics::{Domain, DomainFamily, Gene, Genome};
use evo_life::community::CellContext;
use evo_life::{growth_rates, selection_coefficient, Phenotype};
use evo_sim::World;
use std::path::Path;

fn add(g: &mut Genome, family: DomainFamily, efficiency: f64) {
    let t = g.genes.first().map(|x| (x.domain.t_opt_k, x.domain.t_width_k)).unwrap_or((300.0, 10.0));
    g.genes.push(Gene {
        domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: t.0, t_width_k: t.1, absorption_nm: 500.0 },
        functional: true,
    });
}

fn strip(g: &mut Genome, light: u32) {
    let is_light = |x: &Gene| matches!(x.domain.family, DomainFamily::Catalytic(r) if light & (1 << r) != 0);
    g.genes.retain(|x| !is_light(x));
    add(g, DomainFamily::OxidativeDefense, 0.6);
    for o in &mut g.organelles {
        for x in &mut o.genes {
            if is_light(x) || matches!(x.domain.family, DomainFamily::WaterOxidation) {
                x.functional = false;
            }
        }
    }
}

type Variant<'a> = (&'a str, Box<dyn Fn(&mut Genome)>);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = World::load_file(Path::new(&args[1])).expect("lecture");
    let physio = world.config.physiology.clone();
    let skip: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
    let max_cells: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(6);
    println!("{:.3} Ga", world.years() / 1e9);
    for (c, pops) in world.communities.iter().enumerate() {
        for p in pops.iter().filter(|p| p.phenotype.is_eukaryote()) {
            println!(
                "eucaryote cellule {c} : {:.2e} mol C, terre {}, cellules {}",
                p.biomass,
                p.phenotype.is_terrestrial() as u8,
                p.phenotype.cells()
            );
        }
    }
    let mut shown = 0;
    for c in 0..world.communities.len() {
        let pops = &world.communities[c];
        let Some(host) = pops.iter().find(|p| {
            (p.phenotype.is_multicellular() || std::env::var_os("SONDE_EUK").is_some())
                && p.phenotype.is_eukaryote()
                && !p.phenotype.is_terrestrial()
        }) else {
            continue;
        };
        if c < skip {
            continue;
        }
        if shown >= max_cells {
            break;
        }
        shown += 1;
        let ctx = CellContext { env: &world.bio.env[c], light_biomass_per_m2: world.config.light_biomass_per_m2 };
        let cond = ctx.conditions_of(pops, &physio);
        let chem = &world.chemistry[c];
        println!(
            "cellule {c} : volume {:.2e} m³, O₂ {:.2e}, {} populations",
            world.bio.env[c].water_volume_m3,
            chem[evo_planet::WaterPool::O2 as usize],
            pops.len()
        );
        for p in pops {
            let ph = &p.phenotype;
            println!(
                "  pop {:.2e} molC ({:.2e} mol/m³) taille {:.1} cellules {:.0} types {} euk {} terre {} photo {} sig {:#x} engl {:.2} motil {:.2} r {:.2} naiss {:.2} proies {:.2} pred {:.2}",
                p.biomass,
                p.biomass / world.bio.env[c].water_volume_m3,
                ph.body_size,
                ph.cells(),
                ph.cell_types(),
                ph.is_eukaryote() as u8,
                ph.is_terrestrial() as u8,
                ph.phototroph as u8,
                ph.signature,
                ph.engulfment,
                ph.animal.motility,
                p.rates.r,
                p.rates.birth,
                p.rates.prey_share,
                p.rates.predation
            );
        }
        let resident = growth_rates(&host.phenotype, &cond, chem, &physio);
        for (k, o) in host.genome.organelles.iter().enumerate() {
            let fams: Vec<String> = o.genes.iter().map(|g| format!("{:?}", g.domain.family)).collect();
            println!("    organite {k} : {}", fams.join(" "));
        }
        let light = (1u32 << 5) | (1 << 6) | (1 << 7) | (1 << 8);
        let variants: Vec<Variant> = vec![
            ("résident", Box::new(|_g: &mut Genome| {})),
            ("sans photosynthèse", Box::new(move |g: &mut Genome| strip(g, light))),
            (
                "+ engloutir",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                }),
            ),
            (
                "+ moteurs",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                    add(g, DomainFamily::Motor, 1.0);
                }),
            ),
            (
                "+ sens, nerfs",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                    add(g, DomainFamily::Motor, 1.0);
                    add(g, DomainFamily::Sensor, 1.0);
                    add(g, DomainFamily::Neural, 0.5);
                }),
            ),
            (
                "+ matrice",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                    add(g, DomainFamily::Motor, 1.0);
                    add(g, DomainFamily::Sensor, 1.0);
                    add(g, DomainFamily::Neural, 0.5);
                    add(g, DomainFamily::Matrix, 1.0);
                }),
            ),
            (
                "+ grand corps",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                    add(g, DomainFamily::Motor, 1.0);
                    add(g, DomainFamily::Sensor, 1.0);
                    add(g, DomainFamily::Neural, 0.5);
                    add(g, DomainFamily::Matrix, 1.0);
                    add(g, DomainFamily::Adhesion, 1.5);
                }),
            ),
            (
                "+ très grand",
                Box::new(move |g: &mut Genome| {
                    strip(g, light);
                    add(g, DomainFamily::Cytoskeleton, 0.5);
                    add(g, DomainFamily::Motor, 1.5);
                    add(g, DomainFamily::Sensor, 1.0);
                    add(g, DomainFamily::Neural, 1.0);
                    add(g, DomainFamily::Matrix, 2.0);
                    add(g, DomainFamily::Adhesion, 3.0);
                }),
            ),
        ];
        for (name, f) in &variants {
            let mut g = (*host.genome).clone();
            f(&mut g);
            let ph = Phenotype::from_genome(&g, &physio);
            let r = growth_rates(&ph, &cond, chem, &physio);
            let s = selection_coefficient(&r, &resident, &physio);
            println!(
                "    {name:<20} s {s:+.3} r {:+.2} naiss {:.2} mort {:.2} uv {:.1} pred {:.1} proies dispo {:.2e} prise {:.2} part {:.2} entretien {:.0} énergie {:.0} | taille {:.1} bouche {:.2e} cellules {:.0} motil {:.2} vitesse {:.1e} viab {:?}",
                r.r,
                r.birth,
                r.mortality,
                r.uv_mortality,
                r.predation,
                cond.edible_prey(&ph, &physio),
                r.prey_uptake,
                r.prey_share,
                ph.maintenance_kj,
                r.energy_kj,
                ph.body_size,
                ph.animal.gape,
                ph.cells(),
                ph.animal.motility,
                ph.animal.speed_m_s,
                ph.animal.non_viable
            );
        }
    }
}
