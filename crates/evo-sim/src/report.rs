//! Mise en forme lisible de l'état du monde.

use crate::world::Summary;
use evo_genetics::{GenomeChangeCause, MutationKind, GENOME_CHANGE_CAUSE_COUNT};
use evo_life::metabolism::{guild_label, PHOTOSYNTHESIS_STAGES};
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
    let g = &s.globals;
    let _ = writeln!(out, "Temps : {}", format_years(s.years));
    let _ = writeln!(
        out,
        "Atmosphère : {:.2} bar ; O₂ {:.2e} ; CO₂ {:.0} Pa ; CH₄ {:.0} ppb. Climat : {:.1} K en moyenne, glace {:.0} %, océan {:.0} % de la surface",
        g.pressure_pa / 1e5,
        g.o2_mixing,
        g.co2_pa,
        g.ch4_ppb,
        g.mean_temperature_k,
        100.0 * g.ice_fraction,
        100.0 * g.ocean_fraction
    );
    let _ = writeln!(
        out,
        "Oxygène : production brute {:.2e} mol/an, libération nette {:.2e}, puits {:.2e} ; carbone organique enfoui {:.2e} mol/an",
        g.o2_production, g.o2_release, g.o2_sinks, g.organic_burial
    );
    let _ = writeln!(
        out,
        "Chemin de la photosynthèse : {} ; accélérateur {}",
        PHOTOSYNTHESIS_STAGES[g.photosynthesis_stage as usize],
        if g.accelerator_on { "actif" } else { "inactif" }
    );
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
    let _ = writeln!(
        out,
        "Tunnel stochastique : {} tentatives, {} réussites ; modifications fixées par cause : {}",
        s.stats.tunnel_attempts,
        s.stats.tunnel_successes,
        causes(&s.stats.fixed_changes_by_cause)
    );
    let _ = writeln!(out, "Bilans : carbone {:.1e}, phosphore {:.1e} (écarts relatifs)", s.carbon_error, s.phosphorus_error);
    out
}

/// Compteurs par cause, en texte (causes non nulles seulement).
pub fn causes(counts: &[u64; GENOME_CHANGE_CAUSE_COUNT]) -> String {
    let all = [
        GenomeChangeCause::SpontaneousMutation(MutationKind::Point),
        GenomeChangeCause::InducedMutation(MutationKind::Point),
        GenomeChangeCause::Recombination,
        GenomeChangeCause::HorizontalTransfer,
        GenomeChangeCause::Endosymbiosis,
        GenomeChangeCause::Accelerator,
        GenomeChangeCause::ArtificialSelection,
        GenomeChangeCause::SocietyTechnique,
    ];
    let parts: Vec<String> = all.iter().filter(|c| counts[c.index()] > 0).map(|c| format!("{} {}", c.label(), counts[c.index()])).collect();
    if parts.is_empty() {
        "aucune".into()
    } else {
        parts.join(", ")
    }
}
