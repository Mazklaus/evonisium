//! L'état culturel publié : ce que le client et le son lisent à chaque pas.
//!
//! Il s'ajoute à l'état publié du moteur (doc d'architecture, étape 6 :
//! « peuples, langues, traits et leurs fréquences, notables, mythes,
//! généalogie des peuples »). Il est lu seul, ne change jamais l'histoire, et
//! tout y est en identifiants stables : un peuple, une langue, un trait, un
//! mythe ou un notable garde son identifiant toute la partie.
//!
//! Format (version [`FORMAT_VERSION`]) :
//!
//! - `species` : une ligne par espèce à apprentissage social (mode, fidélité,
//!   gain culturel, entrée en mode historique) ;
//! - `groups` : une ligne par groupe culturel (cellule × espèce) : peuple,
//!   langue, cohésion, effectif, traits et leur fréquence (en creux) ;
//! - `traits` : le registre des traits portés (techniques avec leur nom selon
//!   la voie la plus portée, traits générés avec leur genre et leur nom) ;
//! - `peoples` : tous les peuples, vivants et éteints, avec leur parent :
//!   c'est la généalogie, l'arbre des cultures en sort ;
//! - `languages` : idem pour les langues, avec quelques mots ;
//! - `notables` : les notables vivants et les derniers morts ;
//! - `myths` : tous les mythes, avec leur récit ;
//! - `unexplained` : les phénomènes vus et restés inexpliqués.

use crate::catalog::{Family, CATALOG};
use crate::cognition::Mode;
use crate::language::MEANING_LABELS;
use crate::myths::{Myth, Phenomenon};
use crate::traits::{Effects, TraitId, TraitKind, FIRST_GENERATED};
use crate::world::{CultureWorld, NotableKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT_VERSION: u32 = 1;
/// Notables morts publiés (les plus récents).
pub const PUBLISHED_DEAD_NOTABLES: usize = 256;
/// Mots publiés par langue.
pub const PUBLISHED_WORDS: usize = 12;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CulturalState {
    pub version: u32,
    pub year: f64,
    /// Le monde est en mode historique.
    pub historical: bool,
    pub species: Vec<SpeciesView>,
    pub groups: Vec<GroupView>,
    pub traits: Vec<TraitView>,
    pub peoples: Vec<PeopleView>,
    pub languages: Vec<LanguageView>,
    pub notables: Vec<NotableView>,
    pub myths: Vec<Myth>,
    pub unexplained: Vec<UnexplainedView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesView {
    pub species: u32,
    pub mode: Mode,
    pub fidelity: f32,
    /// Gain culturel du dernier pas : positif, la culture est cumulative.
    pub gain: f32,
    pub historical_since: Option<f64>,
    pub groups: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupView {
    pub cell: u32,
    pub species: u32,
    pub people: u32,
    pub language: u32,
    pub population: f32,
    pub cohesion: f32,
    pub traits: Vec<TraitFreq>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitFreq {
    pub id: TraitId,
    pub freq: f32,
    /// Maîtrise d'une technique (1 pour les autres traits).
    pub mastery: f32,
    /// Générations de transmission dans ce groupe.
    pub generations: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitView {
    pub id: TraitId,
    pub kind: TraitKind,
    pub name: String,
    pub effects: Effects,
    /// Origine d'un trait généré (les techniques s'inventent plusieurs fois
    /// et n'en ont pas une seule).
    pub origin_year: Option<f64>,
    pub origin_cell: Option<u32>,
    pub origin_species: Option<u32>,
    /// Groupes qui le portent.
    pub groups: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeopleView {
    pub id: u32,
    pub species: u32,
    pub name: String,
    pub parent: Option<u32>,
    pub born_year: f64,
    pub ended_year: Option<f64>,
    pub language: Option<u32>,
    pub groups: u32,
    pub population: f32,
    /// Cellule du groupe le plus peuplé (ou de fondation, s'il est éteint).
    pub core_cell: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanguageView {
    pub id: u32,
    pub species: u32,
    pub name: String,
    pub parent: Option<u32>,
    pub born_year: f64,
    pub ended_year: Option<f64>,
    /// Quelques mots : (sens, mot).
    pub words: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotableView {
    pub id: u32,
    pub kind: NotableKind,
    pub name: String,
    pub species: u32,
    pub people: u32,
    pub cell: u32,
    pub trait_id: TraitId,
    pub born_year: f64,
    pub death_year: f64,
    pub alive: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnexplainedView {
    pub year: f64,
    pub cell: u32,
    pub people: u32,
    pub phenomenon: Phenomenon,
    pub intervention: bool,
    pub myth: Option<u32>,
}

impl CulturalState {
    pub fn of(w: &CultureWorld) -> CulturalState {
        let mut trait_groups: BTreeMap<TraitId, u32> = BTreeMap::new();
        let mut route_names: BTreeMap<TraitId, BTreeMap<&'static str, f64>> = BTreeMap::new();
        let mut people_stats: BTreeMap<u32, (u32, f64, u32, f64, u32)> = BTreeMap::new();
        let groups: Vec<GroupView> = w
            .groups
            .iter()
            .map(|g| {
                let e = people_stats.entry(g.people).or_insert((0, 0.0, g.cell, -1.0, g.language));
                e.0 += 1;
                e.1 += g.population;
                if g.population > e.3 {
                    e.2 = g.cell;
                    e.3 = g.population;
                    e.4 = g.language;
                }
                let traits = g
                    .traits
                    .iter()
                    .map(|s| {
                        *trait_groups.entry(s.id).or_default() += 1;
                        if s.id < FIRST_GENERATED {
                            if let Some(v) = CATALOG[s.id as usize].variant(&g.context) {
                                *route_names.entry(s.id).or_default().entry(v.name).or_default() += s.freq * g.population;
                            }
                        }
                        TraitFreq { id: s.id, freq: s.freq as f32, mastery: s.mastery as f32, generations: w.generations(g, s) as f32 }
                    })
                    .collect();
                GroupView {
                    cell: g.cell,
                    species: g.species,
                    people: g.people,
                    language: g.language,
                    population: g.population as f32,
                    cohesion: g.cohesion as f32,
                    traits,
                }
            })
            .collect();
        let mut traits = Vec::new();
        for (&id, &n) in &trait_groups {
            if id < FIRST_GENERATED {
                let f = Family::from_index(id as usize).expect("technique");
                let name = route_names
                    .get(&id)
                    .and_then(|m| m.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|e| e.0.to_string()))
                    .unwrap_or_else(|| CATALOG[f.index()].variants[0].name.to_string());
                traits.push(TraitView {
                    id,
                    kind: TraitKind::Technique(f),
                    name,
                    effects: Effects::default(),
                    origin_year: None,
                    origin_cell: None,
                    origin_species: None,
                    groups: n,
                });
            } else if let Some(d) = w.registry.get(&id) {
                traits.push(TraitView {
                    id,
                    kind: d.kind,
                    name: d.name.clone(),
                    effects: d.effects,
                    origin_year: Some(d.origin_year),
                    origin_cell: Some(d.origin_cell),
                    origin_species: Some(d.origin_species),
                    groups: n,
                });
            }
        }
        let peoples = w
            .peoples
            .iter()
            .map(|p| {
                let s = people_stats.get(&p.id);
                PeopleView {
                    id: p.id,
                    species: p.species,
                    name: p.name.clone(),
                    parent: p.parent,
                    born_year: p.born_year,
                    ended_year: p.ended_year,
                    language: s.map(|s| s.4),
                    groups: s.map(|s| s.0).unwrap_or(0),
                    population: s.map(|s| s.1 as f32).unwrap_or(0.0),
                    core_cell: s.map(|s| s.2).unwrap_or(p.founder_cell),
                }
            })
            .collect();
        let languages = w
            .languages
            .iter()
            .map(|l| LanguageView {
                id: l.id,
                species: l.species,
                name: l.name.clone(),
                parent: l.parent,
                born_year: l.born_year,
                ended_year: l.ended_year,
                words: (0..PUBLISHED_WORDS).map(|m| (MEANING_LABELS[m].to_string(), l.phonology.word(l.lexicon[m]))).collect(),
            })
            .collect();
        let mut dead: Vec<_> = w.notables.iter().filter(|n| n.death_year <= w.year).collect();
        dead.sort_by(|a, b| b.death_year.total_cmp(&a.death_year).then(a.id.cmp(&b.id)));
        dead.truncate(PUBLISHED_DEAD_NOTABLES);
        let mut notables: Vec<NotableView> = w
            .notables
            .iter()
            .filter(|n| n.death_year > w.year)
            .chain(dead)
            .map(|n| NotableView {
                id: n.id,
                kind: n.kind,
                name: n.name.clone(),
                species: n.species,
                people: n.people,
                cell: n.cell,
                trait_id: n.trait_id,
                born_year: n.born_year,
                death_year: n.death_year,
                alive: n.death_year > w.year,
            })
            .collect();
        notables.sort_by_key(|n| n.id);
        let species = w
            .species
            .iter()
            .filter(|s| s.learning.social())
            .map(|s| SpeciesView {
                species: s.id,
                mode: s.learning.mode,
                fidelity: s.learning.fidelity as f32,
                gain: s.gain as f32,
                historical_since: s.historical_since,
                groups: w.groups.iter().filter(|g| g.species == s.id).count() as u32,
            })
            .collect();
        CulturalState {
            version: FORMAT_VERSION,
            year: w.year,
            historical: w.historical(),
            species,
            groups,
            traits,
            peoples,
            languages,
            notables,
            myths: w.myths.clone(),
            unexplained: w
                .unexplained
                .iter()
                .map(|u| UnexplainedView {
                    year: u.year,
                    cell: u.cell,
                    people: u.people,
                    phenomenon: u.phenomenon,
                    intervention: u.intervention,
                    myth: u.myth,
                })
                .collect(),
        }
    }

    /// Empreinte de l'état, pour vérifier un rejeu.
    pub fn fingerprint(&self) -> u64 {
        use evo_core::rng::splitmix64;
        let mut h = splitmix64(self.year.to_bits());
        let mut mix = |x: u64| h = splitmix64(h ^ x);
        for g in &self.groups {
            mix(((g.cell as u64) << 32) | g.species as u64);
            mix(((g.people as u64) << 32) | g.language as u64);
            mix(g.cohesion.to_bits() as u64);
            for t in &g.traits {
                mix(t.id as u64);
                mix(((t.freq.to_bits() as u64) << 32) | t.mastery.to_bits() as u64);
            }
        }
        for t in &self.traits {
            mix(t.id as u64 ^ (t.groups as u64) << 40);
            for b in t.name.bytes() {
                mix(b as u64);
            }
        }
        for p in &self.peoples {
            mix(p.id as u64 ^ (p.parent.map(|x| x as u64 + 1).unwrap_or(0) << 32));
        }
        for l in &self.languages {
            mix(l.id as u64 ^ (l.parent.map(|x| x as u64 + 1).unwrap_or(0) << 32));
            for (_, w) in &l.words {
                for b in w.bytes() {
                    mix(b as u64);
                }
            }
        }
        for n in &self.notables {
            mix(n.id as u64 ^ (n.trait_id as u64) << 32);
            mix(n.death_year.to_bits());
            for b in n.name.bytes() {
                mix(b as u64);
            }
        }
        for m in &self.myths {
            for b in m.text.bytes() {
                mix(b as u64);
            }
        }
        mix(self.unexplained.len() as u64);
        h
    }

    pub fn notable_kind_label(k: NotableKind) -> &'static str {
        match k {
            NotableKind::Inventor => "inventeur",
            NotableKind::Transmitter => "passeur",
        }
    }
}
