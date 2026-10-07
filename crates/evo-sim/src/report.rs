//! Mise en forme lisible de l'état du monde.

use crate::world::Summary;
use evo_life::metabolism::guild_label;
use evo_planet::WATER_POOLS;
use std::fmt::Write;

/// Années en texte court (ka, Ma, Ga).
pub fn format_years(y: f64) -> String {
    if y >= 1e9 {
        format!("{:.2} Ga", y / 1e9)
    } else if y >= 1e6 {
        format!("{:.2} Ma", y / 1e6)
    } else if y >= 1e3 {
        format!("{:.1} ka", y / 1e3)
    } else {
        format!("{y:.0} ans")
    }
}

pub fn format_summary(s: &Summary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Temps : {}", format_years(s.years));
    let _ = writeln!(
        out,
        "Cellules océaniques colonisées : {} / {} ; populations : {} ; biomasse : {:.3e} mol C",
        s.colonised_cells, s.ocean_cells, s.populations, s.biomass
    );
    let _ = writeln!(out, "Lignées : {} vivantes sur {} apparues", s.lineages_living, s.lineages_total);
    let _ = writeln!(out, "Gènes par génome : {:.1} en moyenne, dont {:.1} fonctionnels", s.mean_genes, s.mean_functional_genes);
    let _ = writeln!(out, "Écart thermique moyen (optimum des enzymes − température locale) : {:.2} K", s.thermal_mismatch_k);
    let _ = writeln!(
        out,
        "Substitutions : {} ; colonisations : {} ; remplacements par des migrants : {} ; extinctions locales : {}",
        s.stats.substitutions, s.stats.colonisations, s.stats.migrant_replacements, s.stats.local_extinctions
    );
    let _ = writeln!(out, "Guildes métaboliques :");
    let mut guilds: Vec<_> = s.guilds.iter().collect();
    guilds.sort_by(|a, b| b.1 .1.total_cmp(&a.1 .1));
    for (sig, (count, biomass)) in guilds {
        let _ = writeln!(out, "  {:<55} {:>7} populations  {:>10.3e} mol C", guild_label(*sig), count, biomass);
    }
    let _ = write!(out, "Chimie moyenne de l'eau de surface (mol/m³) :");
    for p in WATER_POOLS {
        let _ = write!(out, " {} {:.3e} ;", p.label(), s.mean_chemistry[p as usize]);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Bilan de carbone : écart relatif {:.1e}", s.carbon_error);
    out
}
