//! Fiche d'espèce simple (premier jouable) : noms, traits en clair, aire,
//! effectif, origine et statut, et passage vers le générateur d'apparence.

use crate::format::Lang;
use crate::frame::{Frame, LineageFrame, PopulationFrame};
use crate::naming::{metabolism_list, species_names, SpeciesNames};
use evo_life::metabolism::OXYGENIC_PHOTOSYNTHESIS;
use evo_morph::MicrobeTraits;
use evo_planet::grid::GeodesicGrid;

/// Tendance de l'effectif.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Expanding,
    Stable,
    Declining,
    Extinct,
}

impl Status {
    /// À partir de la biomasse totale sur les derniers pas (du plus ancien au
    /// plus récent).
    pub fn from_series(series: &[f64], extinct: bool) -> Status {
        if extinct {
            return Status::Extinct;
        }
        match (series.first(), series.last()) {
            (Some(&a), Some(&b)) if a > 0.0 && series.len() >= 2 => {
                let r = b / a;
                if r > 1.25 {
                    Status::Expanding
                } else if r < 0.8 {
                    Status::Declining
                } else {
                    Status::Stable
                }
            }
            _ => Status::Stable,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match (lang, self) {
            (Lang::Fr, Status::Expanding) => "en expansion",
            (Lang::Fr, Status::Stable) => "stable",
            (Lang::Fr, Status::Declining) => "en déclin",
            (Lang::Fr, Status::Extinct) => "éteinte",
            (Lang::En, Status::Expanding) => "expanding",
            (Lang::En, Status::Stable) => "stable",
            (Lang::En, Status::Declining) => "declining",
            (Lang::En, Status::Extinct) => "extinct",
        }
    }
}

/// Résumé d'une espèce lu dans l'image du pas.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesSummary {
    pub lineage: u32,
    pub names: SpeciesNames,
    pub metabolisms: Vec<&'static str>,
    pub pigment_nm: Option<f32>,
    pub range_cells: usize,
    /// Part de la surface de la planète occupée.
    pub range_share: f32,
    pub biomass: f64,
    pub born_years: f64,
    pub parent: Option<u32>,
    pub extinct_years: Option<f64>,
    pub traits: Option<MicrobeTraits>,
}

pub fn summary(frame: &Frame, grid: &GeodesicGrid, lineage: &LineageFrame, lang: Lang) -> SpeciesSummary {
    let range = frame.range_of(lineage.id);
    let pop: Option<PopulationFrame> = range
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .and_then(|&(c, _)| frame.populations_of(c).iter().find(|p| p.lineage == lineage.id).copied());
    let signature = pop.map_or(lineage.signature, |p| p.signature);
    let area: f64 = range.iter().map(|&(c, _)| grid.unit_areas[c]).sum();
    SpeciesSummary {
        lineage: lineage.id,
        names: species_names(frame.planet.seed, grid, lineage.id, signature, lineage.origin_cell, lang),
        metabolisms: metabolism_list(signature, lang),
        pigment_nm: pop.and_then(|p| p.pigment_nm),
        range_cells: range.len(),
        range_share: (area / (4.0 * std::f64::consts::PI)) as f32,
        biomass: range.iter().map(|r| r.1 as f64).sum(),
        born_years: lineage.born_years,
        parent: (lineage.parent != lineage.id).then_some(lineage.parent),
        extinct_years: lineage.extinct_years,
        traits: pop.map(|p| traits_of(&p)),
    }
}

/// Ce que le générateur d'apparence lit d'une population.
pub fn traits_of(p: &PopulationFrame) -> MicrobeTraits {
    MicrobeTraits {
        lineage: p.lineage,
        signature: p.signature,
        pigment_rgb: p.pigment_nm.map(|nm| evo_life::pigment_colour(nm as f64)),
        gene_count: p.gene_count,
        phototroph: p.phototroph,
        oxygenic: p.signature & (1 << OXYGENIC_PHOTOSYNTHESIS) != 0,
    }
}

/// Membres de la vue microscope d'une cellule.
pub fn microscope_members(frame: &Frame, cell: usize) -> Vec<evo_morph::Member> {
    let pops = frame.populations_of(cell);
    let total: f32 = pops.iter().map(|p| p.biomass).sum();
    pops.iter()
        .map(|p| evo_morph::Member {
            form: evo_morph::form(&traits_of(p), frame.planet.seed),
            share: if total > 0.0 { p.biomass / total } else { 0.0 },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::lineages_of;
    use evo_sim::{World, WorldConfig};

    #[test]
    fn summary_of_the_seeded_lineage() {
        let mut w = World::new(WorldConfig::new(8, 3));
        w.seed_life();
        w.step();
        let f = Frame::from_world(&w);
        let ls = lineages_of(&w);
        let s = summary(&f, &w.planet.grid, &ls[0], Lang::Fr);
        assert!(s.range_cells > 0 && s.range_share > 0.0 && s.biomass > 0.0);
        assert!(s.metabolisms.contains(&"méthanogenèse"));
        assert!(s.parent.is_none());
        assert!(s.traits.is_some());
        let members = microscope_members(&f, f.range_of(0)[0].0);
        assert!(!members.is_empty());
        assert_eq!(Status::from_series(&[1.0, 2.0], false), Status::Expanding);
        assert_eq!(Status::from_series(&[1.0, 0.5], false), Status::Declining);
    }
}
