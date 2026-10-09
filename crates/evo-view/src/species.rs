//! Fiche d'espèce simple (premier jouable) : noms, traits en clair, aire,
//! effectif, origine et statut, et passage vers le générateur d'apparence.
//!
//! Pendant l'ère microbienne, une espèce du moteur est une guilde
//! métabolique, identifiée par sa signature (les réactions qu'elle
//! catalyse) ; ses lignées et ses écotypes en sont les variantes.

use crate::format::Lang;
use crate::frame::{Frame, LineageFrame};
use crate::naming::{metabolism_list, scientific_name};
use evo_life::metabolism::OXYGENIC_PHOTOSYNTHESIS;
use evo_morph::MicrobeTraits;
use evo_planet::grid::GeodesicGrid;
use evo_sim::history::SpeciesView;
use evo_sim::observation::PopulationView;

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

/// Résumé d'une espèce lu dans l'état publié et l'arbre des lignées.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesSummary {
    pub species: u32,
    pub name: String,
    pub scientific: String,
    pub metabolisms: Vec<&'static str>,
    pub pigment_rgb: Option<[u8; 3]>,
    /// Cellules du vivant occupées, et leur part de la planète.
    pub range_cells: u32,
    pub range_share: f32,
    pub biomass: f64,
    pub ecotypes: u32,
    pub photosynthesis_stage: u8,
    pub peak_cell: u32,
    pub origin_cell: u32,
    /// Première lignée de cette signature : date et lignée mère.
    pub born_years: Option<f64>,
    pub parent_lineage: Option<u32>,
    pub lineages: usize,
}

pub fn summary(frame: &Frame, grid: &GeodesicGrid, s: &SpeciesView, lineages: &[LineageFrame], lang: Lang) -> SpeciesSummary {
    let mine: Vec<&LineageFrame> = lineages.iter().filter(|l| l.signature == s.signature).collect();
    let first = mine.iter().min_by(|a, b| a.born_years.total_cmp(&b.born_years).then(a.id.cmp(&b.id)));
    SpeciesSummary {
        species: s.id,
        name: s.name.clone(),
        scientific: scientific_name(frame.planet.seed, grid, s.signature, s.origin_bio_cell),
        metabolisms: metabolism_list(s.signature, lang),
        pigment_rgb: s.pigment_rgb,
        range_cells: s.cells,
        range_share: s.cells as f32 / frame.state.bio_cells.max(1) as f32,
        biomass: s.biomass,
        ecotypes: s.ecotypes,
        photosynthesis_stage: s.photosynthesis_stage,
        peak_cell: s.peak_bio_cell,
        origin_cell: s.origin_bio_cell,
        born_years: first.map(|l| l.born_years),
        parent_lineage: first.and_then(|l| (l.parent != l.id).then_some(l.parent)),
        lineages: mine.iter().filter(|l| l.extinct_years.is_none()).count(),
    }
}

/// Ce que le générateur d'apparence lit d'une population.
pub fn traits_of(p: &PopulationView) -> MicrobeTraits {
    MicrobeTraits {
        lineage: p.lineage,
        signature: p.species,
        pigment_rgb: p.pigment_rgb.or_else(|| p.pigment_nm.map(|nm| evo_life::pigment_colour(nm as f64))),
        gene_count: p.genes as u32,
        phototroph: p.phototroph,
        oxygenic: p.species & (1 << OXYGENIC_PHOTOSYNTHESIS) != 0,
    }
}

/// Membres de la vue microscope d'une cellule.
pub fn microscope_members(populations: &[PopulationView], game_seed: u64) -> Vec<evo_morph::Member> {
    let total: f32 = populations.iter().map(|p| p.biomass).sum();
    populations
        .iter()
        .map(|p| evo_morph::Member {
            form: evo_morph::form(&traits_of(p), game_seed),
            share: if total > 0.0 { p.biomass / total } else { 0.0 },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::tests::sample_frame;

    #[test]
    fn summary_of_the_first_species() {
        let f = sample_frame(8);
        let s = &f.state.species[0];
        let lineage =
            LineageFrame { id: 0, parent: 0, born_years: 0.0, extinct_years: None, origin_cell: s.origin_bio_cell, signature: s.signature };
        let sum = summary(&f, &GeodesicGrid::new(3), s, &[lineage], Lang::Fr);
        assert!(sum.range_cells > 0 && sum.range_share > 0.0 && sum.biomass > 0.0);
        assert!(sum.metabolisms.contains(&"méthanogenèse"));
        assert_eq!(sum.born_years, Some(0.0));
        assert!(sum.parent_lineage.is_none());
        let pop = PopulationView {
            lineage: 0,
            species: s.signature,
            biomass: 1.0,
            growth_per_year: 0.0,
            birth_per_year: 0.0,
            genes: 5,
            pigment_rgb: None,
            pigment_nm: None,
            phototroph: false,
            photosynthesis_stage: 0,
        };
        assert_eq!(microscope_members(&[pop], 1).len(), 1);
        assert_eq!(Status::from_series(&[1.0, 2.0], false), Status::Expanding);
        assert_eq!(Status::from_series(&[1.0, 0.5], false), Status::Declining);
    }
}
