//! Zones actives : où le niveau 4 est tiré (document Vision, « Individus
//! échantillonnés ... dans les zones actives : innovation, spéciation en
//! cours, colonisation, lignée suivie par le joueur »).
//!
//! Le choix des zones lit le monde sans l'écrire. La zone regardée vient du
//! canal d'observation ; les autres (lignées suivies, innovations récentes,
//! fondations de lignées multicellulaires) existent quelle que soit la
//! caméra.

use crate::sample::{has_individuals, Sample, SAMPLE_SIZE};
use evo_core::events::EventKind;
use evo_sim::World;

/// Pourquoi une cellule est active.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reason {
    /// La caméra la regarde.
    Observed,
    /// Le joueur suit une lignée qui y vit.
    Followed,
    /// Une innovation ou une nouvelle lignée y est apparue récemment.
    Innovation,
}

/// Durée pendant laquelle une innovation garde sa cellule active, années.
pub const INNOVATION_WINDOW_YEARS: f64 = 2.0e6;

/// Cellules du vivant actives, sans doublon, dans un ordre stable.
pub fn active_cells(world: &World) -> Vec<(u32, Reason)> {
    let mut out: Vec<(u32, Reason)> = Vec::new();
    let add = |c: u32, r: Reason, out: &mut Vec<(u32, Reason)>| {
        if (c as usize) < world.communities.len() && !out.iter().any(|x| x.0 == c) {
            out.push((c, r));
        }
    };
    if let Some(z) = world.interest {
        if let Some(&b) = world.bio.parent.get(z.center_cell as usize) {
            add(b, Reason::Observed, &mut out);
        }
    }
    for (c, pops) in world.communities.iter().enumerate() {
        if pops.iter().any(|p| world.marked.contains(&p.lineage) && has_individuals(p)) {
            add(c as u32, Reason::Followed, &mut out);
        }
    }
    let now = world.years();
    for e in world.events.events.iter().rev() {
        if now - e.years > INNOVATION_WINDOW_YEARS {
            break;
        }
        if let (Some(c), EventKind::Innovation { .. } | EventKind::NewLineage { .. }) = (e.cell, &e.kind) {
            if world.communities.get(c as usize).is_some_and(|p| p.iter().any(has_individuals)) {
                add(c, Reason::Innovation, &mut out);
            }
        }
    }
    out
}

/// Échantillons des populations à individus d'une cellule du vivant, la
/// plus abondante d'abord, au plus `max_species`.
pub fn sample_cell(world: &World, bio_cell: usize, max_species: usize, size: usize) -> Vec<Sample> {
    let Some(pops) = world.communities.get(bio_cell) else { return Vec::new() };
    let mut order: Vec<usize> = (0..pops.len()).filter(|&i| has_individuals(&pops[i])).collect();
    order.sort_by(|&a, &b| pops[b].biomass.total_cmp(&pops[a].biomass).then(a.cmp(&b)));
    order.into_iter().take(max_species).filter_map(|i| Sample::draw(world, bio_cell, i, size)).collect()
}

/// Échantillons de toutes les zones actives.
pub fn sample_active(world: &World, max_species: usize) -> Vec<(Reason, Sample)> {
    let mut out = Vec::new();
    for (c, r) in active_cells(world) {
        for s in sample_cell(world, c as usize, max_species, SAMPLE_SIZE) {
            out.push((r, s));
        }
    }
    out
}
