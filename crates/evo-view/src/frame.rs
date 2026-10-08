//! Ce que le client lit du moteur à chaque pas : l'état publié
//! (`PublishedState`), photographie immuable et datée du monde à la fin d'un
//! pas, partagée en lecture seule (`Arc`) entre le fil de la simulation et
//! celui du rendu. Le client n'ouvre jamais le monde lui-même.

use evo_engine::LineageView;
use evo_sim::history::{CellView, PublishedState, SpeciesView};
use std::sync::Arc;

/// Paramètres fixes de la planète utiles à l'affichage (le client les
/// connaît depuis la création de la partie).
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetInfo {
    pub name: String,
    pub seed: u64,
    pub level: u32,
    pub radius_m: f64,
    pub star_temperature_k: f64,
}

/// Une lignée, telle que l'arbre du vivant la lit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineageFrame {
    pub id: u32,
    pub parent: u32,
    pub born_years: f64,
    pub extinct_years: Option<f64>,
    /// Cellule d'origine. Les premières cellules d'une grille géodésique
    /// sont celles des niveaux plus grossiers : l'indice d'une cellule du
    /// vivant est aussi celui d'une cellule physique au même endroit.
    pub origin_cell: u32,
    pub signature: u32,
}

impl From<&LineageView> for LineageFrame {
    fn from(l: &LineageView) -> Self {
        Self {
            id: l.id,
            parent: l.parent,
            born_years: l.born_years,
            extinct_years: l.extinct_years,
            origin_cell: l.origin_bio_cell,
            signature: l.signature,
        }
    }
}

/// Un pas publié, avec la planète qui l'accompagne.
#[derive(Clone, Debug)]
pub struct Frame {
    pub state: Arc<PublishedState>,
    pub planet: Arc<PlanetInfo>,
}

impl Frame {
    pub fn new(state: Arc<PublishedState>, planet: Arc<PlanetInfo>) -> Self {
        Self { state, planet }
    }

    pub fn cells(&self) -> &[CellView] {
        &self.state.cells
    }

    pub fn species(&self, id: u32) -> Option<&SpeciesView> {
        self.state.species.iter().find(|s| s.id == id)
    }

    /// Cellules physiques où une espèce domine.
    pub fn dominated_by(&self, species: u32) -> Vec<usize> {
        self.state.cells.iter().enumerate().filter(|(_, c)| c.dominant_guild == species).map(|(i, _)| i).collect()
    }

    /// Guildes les plus répandues (nombre de cellules dominées), au plus `k`.
    pub fn top_guilds(&self, k: usize) -> Vec<(u32, usize)> {
        let mut counts = std::collections::BTreeMap::new();
        for c in &self.state.cells {
            if c.dominant_guild != 0 {
                *counts.entry(c.dominant_guild).or_insert(0usize) += 1;
            }
        }
        let mut v: Vec<(u32, usize)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(k);
        v
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use evo_sim::{World, WorldConfig};

    /// Un petit monde ensemencé, un pas joué, et son état publié.
    pub fn sample_frame(seed: u64) -> Frame {
        let mut w = World::new(WorldConfig::new(seed, 3));
        w.seed_life();
        w.step();
        let planet = Arc::new(PlanetInfo {
            name: w.planet.params.name.clone(),
            seed,
            level: 3,
            radius_m: w.planet.params.radius_m,
            star_temperature_k: w.planet.params.star_temperature_k,
        });
        Frame::new(w.publication.current.clone().unwrap(), planet)
    }

    #[test]
    fn frame_reads_the_published_state() {
        let f = sample_frame(7);
        assert_eq!(f.cells().len(), 642);
        for c in f.cells() {
            if c.is_ocean {
                assert!(c.elevation_m <= 1.0, "{}", c.elevation_m);
            }
        }
        let top = f.top_guilds(3);
        assert!(!top.is_empty());
        assert_eq!(f.dominated_by(top[0].0).len(), top[0].1);
        assert!(f.species(top[0].0).is_some());
    }
}
