//! Cultures branchées sur le moteur (étape 6).
//!
//! Chaque population d'une espèce capable d'apprentissage social porte un
//! groupe culturel (`evo_culture`) dans sa cellule du vivant. Les lectures
//! cognitives viennent du constructeur de phénotypes ; l'espèce culturelle
//! est la lignée devenue sociale la première, héritée par ses lignées
//! filles ; l'effectif du groupe est la biomasse de ses populations divisée
//! par le carbone d'un individu.
//!
//! [Simplification] Une espèce culturelle prend les lectures de sa
//! population la plus abondante ; ses autres populations en diffèrent peu,
//! leurs lignées n'étant séparées que par leurs gènes de métabolisme ou
//! d'organisation.

use evo_culture::{Channels, Cognition, Context, CultureWorld, Learning, Manipulation, Milieu, Organ};
use evo_genetics::LineageRegistry;
use evo_life::community::Population;
use evo_life::phenotype::ManipulationOrgan;
use evo_life::Physiology;
use evo_planet::BioGrid;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Part des individus de deux cellules voisines en contact.
pub const NEIGHBOUR_LINK: f64 = 0.05;

/// Pas d'un an du mode historique, et rythmes fixes des autres phases
/// (doc d'architecture, « Périmètre consolidé de l'étape 6 »), années.
pub const HISTORICAL_STEP_YEARS: f64 = 1.0;
pub const HISTORICAL_ECOLOGY_YEARS: f64 = 10.0;
pub const HISTORICAL_CLIMATE_YEARS: f64 = 100.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cultures {
    pub world: CultureWorld,
    /// Espèce culturelle de chaque lignée sociale déjà vue.
    pub species_of: BTreeMap<u32, u32>,
}

impl Cultures {
    pub fn new(seed: u64) -> Cultures {
        Cultures { world: CultureWorld::new(seed), species_of: BTreeMap::new() }
    }

    /// Espèce culturelle d'une lignée sociale : celle de son plus proche
    /// ancêtre déjà social, ou une espèce nouvelle qui porte son numéro.
    fn species_for(&mut self, lineage: u32, lineages: &LineageRegistry) -> u32 {
        if let Some(&s) = self.species_of.get(&lineage) {
            return s;
        }
        let mut id = lineage;
        let mut found = None;
        while let Some(r) = lineages.get(id) {
            if r.parent == id {
                break;
            }
            id = r.parent;
            if let Some(&s) = self.species_of.get(&id) {
                found = Some(s);
                break;
            }
        }
        let s = found.unwrap_or(lineage);
        self.species_of.insert(lineage, s);
        s
    }

    /// Accorde les espèces et les groupes culturels aux populations.
    pub fn sync(
        &mut self,
        communities: &[Vec<Population>],
        bio: &BioGrid,
        lineages: &LineageRegistry,
        o2_percent: f64,
        physio: &Physiology,
    ) {
        // Espèce, cognition de la population la plus abondante, et effectif
        // et contexte de chaque groupe.
        let mut reps: BTreeMap<u32, (f64, Cognition)> = BTreeMap::new();
        let mut present: BTreeMap<(u32, u32), (f64, Context)> = BTreeMap::new();
        for (c, pops) in communities.iter().enumerate() {
            for p in pops {
                if p.phenotype.animal.cognition.sociality < 0.15 || p.biomass <= 0.0 {
                    continue;
                }
                let cog = cognition_of(p, physio);
                if !Learning::of(&cog).social() {
                    continue;
                }
                let species = self.species_for(p.lineage, lineages);
                let rep = reps.entry(species).or_insert((0.0, cog));
                if p.biomass > rep.0 {
                    *rep = (p.biomass, cog);
                }
                let milieu = if p.phenotype.is_terrestrial() { Milieu::Land } else { Milieu::Water };
                let ctx = Context { milieu, o2_percent, organ: cog.manipulation.organ };
                let e = present.entry((c as u32, species)).or_insert((0.0, ctx));
                e.0 += individuals(p, physio);
            }
        }
        for (&id, &(_, cog)) in &reps {
            self.world.add_species(id, cog);
        }
        let w = &mut self.world;
        let gone: Vec<(u32, u32)> =
            w.groups.iter().map(|g| (g.cell, g.species)).filter(|k| !present.contains_key(k) || !reps.contains_key(&k.1)).collect();
        for (cell, species) in gone {
            w.remove_group(cell, species);
        }
        let mut existing: BTreeSet<(u32, u32)> = w.groups.iter().map(|g| (g.cell, g.species)).collect();
        for g in w.groups.iter_mut() {
            if let Some(&(n, ctx)) = present.get(&(g.cell, g.species)) {
                g.population = n;
                g.context = ctx;
            }
        }
        for (&(cell, species), &(n, ctx)) in &present {
            if existing.contains(&(cell, species)) {
                continue;
            }
            let neighbours: Vec<u32> = bio.grid.neighbours_of(cell as usize).map(|x| x as u32).collect();
            // Colonisation : le groupe hérite du voisin de même espèce.
            let from = neighbours.iter().copied().filter(|&x| existing.contains(&(x, species))).min();
            if w.add_group(cell, species, n, ctx, from) {
                existing.insert((cell, species));
                if !w.linked(cell) {
                    for x in neighbours {
                        w.link(cell, x, NEIGHBOUR_LINK);
                    }
                }
            }
        }
    }
}

/// Carbone d'un individu, mol.
pub fn individual_carbon(p: &Population, physio: &Physiology) -> f64 {
    physio.carbon_per_cell * p.phenotype.cell_volume() * p.phenotype.cells()
}

/// Nombre d'individus d'une population.
pub fn individuals(p: &Population, physio: &Physiology) -> f64 {
    p.biomass / individual_carbon(p, physio).max(f64::MIN_POSITIVE)
}

/// Lectures cognitives d'une population, au format de la culture.
pub fn cognition_of(p: &Population, physio: &Physiology) -> Cognition {
    let r = &p.phenotype.animal.cognition;
    let generation = p.rates.generation_time(physio);
    let organ = match r.organ {
        ManipulationOrgan::None => Organ::None,
        ManipulationOrgan::Hand => Organ::Hand,
        ManipulationOrgan::Mouth => Organ::Mouth,
        ManipulationOrgan::Tentacle => Organ::Tentacle,
    };
    Cognition {
        capacities: r.capacities,
        brain_budget: r.brain_budget,
        manipulation: Manipulation { organ, quality: r.manipulation_quality },
        channels: Channels {
            sound: r.channels[0],
            visual: r.channels[1],
            chemical: r.channels[2],
            tactile: r.channels[3],
            electric: r.channels[4],
        },
        juvenile_years: r.juvenile_share * generation,
        generation_years: generation,
        sociality: r.sociality,
    }
}
