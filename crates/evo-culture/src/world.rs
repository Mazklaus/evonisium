//! Le modèle agrégé : un groupe culturel par population sociale de cellule.
//!
//! Chaque groupe porte la fréquence de ses traits (64 au plus, en creux), la
//! maîtrise de ses techniques, un lexique et une cohésion. Deux régimes :
//!
//! - **à l'équilibre**, quand le pas dépasse [`DYNAMIC_MAX_YEARS`] : un trait
//!   se maintient si le nombre d'apprenants multiplié par la fidélité de
//!   copie compense la perte (λ > μ, et gain de savoir-faire positif pour
//!   une technique), et sa fréquence d'équilibre est appliquée en une fois ;
//!   les groupes reliés partagent leurs traits et leur langue ;
//! - **pas à pas**, au pas d'un an du mode historique : diffusion logistique
//!   dans le groupe (λ·p·(1 − p) − μ·p, résolue exactement), conformisme,
//!   tirage des petits effectifs, maîtrise selon Henrich (2004), passage
//!   entre voisins, inventions, emprunts de mots.
//!
//! Les deux régimes partagent leurs primitives avec les agents détaillés
//! (`cognition::Learning`) : c'est sur eux que le modèle est calé, dans des
//! tests hors partie. Rien ne remonte des agents vers cet état.
//!
//! Le hasard vient du flux `Stream::Culture`, dérivé de la graine, du pas, de
//! la cellule et de l'espèce : la caméra et le curseur n'y ont pas accès.

use crate::catalog::{Context, Family, CATALOG};
use crate::cognition::{Cognition, Learning};
use crate::language::{self, Lexicon, Phonology, MEANINGS};
use crate::myths::{self, Myth, Phenomenon};
use crate::traits::{self, Effects, Resource, TraitDef, TraitId, TraitKind, FIRST_GENERATED};
use evo_core::math::Det;
use evo_core::rng::{rng_for, Stream};
use rand::Rng;
use rand_distr::{Distribution, Poisson};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Au-delà de ce pas, la culture est calculée à l'équilibre.
pub const DYNAMIC_MAX_YEARS: f64 = 100.0;
/// Maîtrise en dessous de laquelle une technique ne sert plus et se perd.
pub const FUNCTIONAL: f64 = 0.25;
/// Maîtrise d'une invention à sa naissance.
pub const INVENTION_MASTERY: f64 = 0.3;
/// Traits au plus par groupe.
pub const MAX_TRAITS: usize = 64;
/// Notables vivants au plus sur la planète.
pub const MAX_NOTABLES: usize = 2000;
/// Notables morts gardés pour l'histoire.
pub const KEPT_DEAD_NOTABLES: usize = 4096;
/// Inventions de techniques par individu connecté et par an, à
/// inventivité 1.
pub const INVENTION_PER_CAPITA_YEAR: f64 = 1e-7;
/// Traits générés (croyances, rites, arts, tabous) par individu connecté et
/// par an, à pensée symbolique 1.
pub const GENERATED_PER_CAPITA_YEAR: f64 = 2e-7;
/// Pensée symbolique minimale pour les traits générés.
pub const SYMBOLIC_MIN: f64 = 0.2;
/// Arrivée d'un trait d'un voisin, par an et par unité de lien, de fidélité
/// et de fréquence chez le voisin.
pub const INTRODUCTION_PER_YEAR: f64 = 0.2;
/// Mélange des fréquences entre voisins, par an et par unité de lien.
pub const MIXING_PER_YEAR: f64 = 0.1;
/// En dessous de ce nombre de porteurs, leur nombre est tiré (Poisson).
pub const SMALL_CARRIERS: f64 = 30.0;

const TAG_GROUP: u64 = 1;
const TAG_EQUILIBRIUM: u64 = 2;
const TAG_LANGUAGE: u64 = 3;
const TAG_FOUND: u64 = 4;
const TAG_WITNESS: u64 = 5;
const TAG_COMPONENT: u64 = 7;
const TAG_NOTABLE: u64 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub id: TraitId,
    /// Part des individus du groupe qui portent le trait.
    pub freq: f64,
    /// Maîtrise d'une technique, entre 0 et 1 (1 pour les autres traits).
    pub mastery: f64,
    /// Année où le trait est entré dans le groupe.
    pub since: f64,
    /// Attrait du trait (copié du registre ou du catalogue).
    pub appeal: f64,
    /// Niche de concurrence d'un trait généré (0 : aucune).
    pub niche: u16,
}

impl Slot {
    pub fn niche(&self) -> Option<u16> {
        (self.niche != 0).then_some(self.niche)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub cell: u32,
    pub species: u32,
    pub population: f64,
    pub context: Context,
    /// Traits triés par identifiant.
    pub traits: Vec<Slot>,
    #[serde(with = "crate::language::lexicon_serde")]
    pub lexicon: Box<Lexicon>,
    pub language: u32,
    pub people: u32,
    pub cohesion: f64,
    /// Voisins de même espèce : indice du groupe et poids du lien (part des
    /// individus en contact).
    #[serde(skip)]
    pub links: Vec<(usize, f64)>,
}

impl Group {
    pub fn slot(&self, id: TraitId) -> Option<&Slot> {
        self.traits.binary_search_by_key(&id, |s| s.id).ok().map(|i| &self.traits[i])
    }

    pub fn has(&self, id: TraitId) -> bool {
        self.slot(id).is_some()
    }

    /// La technique sert (présente et assez maîtrisée).
    pub fn knows(&self, f: Family) -> bool {
        self.slot(f as TraitId).is_some_and(|s| s.mastery >= FUNCTIONAL)
    }

    fn insert(&mut self, slot: Slot, registry: &BTreeMap<TraitId, TraitDef>) {
        match self.traits.binary_search_by_key(&slot.id, |s| s.id) {
            Ok(i) => self.traits[i] = slot,
            Err(i) => self.traits.insert(i, slot),
        }
        if self.traits.len() > MAX_TRAITS {
            // On évince le trait généré le moins répandu ; les techniques
            // passent avant.
            let victim = self
                .traits
                .iter()
                .enumerate()
                .filter(|(_, s)| s.id >= FIRST_GENERATED && registry.get(&s.id).is_none_or(|d| d.kind.generated()))
                .min_by(|a, b| a.1.freq.total_cmp(&b.1.freq).then(b.1.id.cmp(&a.1.id)))
                .map(|(i, _)| i)
                .unwrap_or(self.traits.len() - 1);
            self.traits.remove(victim);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub id: u32,
    pub cognition: Cognition,
    pub learning: Learning,
    /// Gain culturel du dernier pas (meilleur groupe).
    pub gain: f64,
    /// Pas consécutifs à gain positif.
    pub positive_steps: u32,
    /// Année d'entrée en mode historique.
    pub historical_since: Option<f64>,
}

impl Species {
    pub fn historical(&self) -> bool {
        self.historical_since.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeopleRecord {
    pub id: u32,
    pub species: u32,
    pub parent: Option<u32>,
    pub born_year: f64,
    pub ended_year: Option<f64>,
    pub name: String,
    /// Langue du groupe fondateur.
    pub language: u32,
    pub founder_cell: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanguageRecord {
    pub id: u32,
    pub species: u32,
    pub parent: Option<u32>,
    pub born_year: f64,
    pub ended_year: Option<f64>,
    pub name: String,
    pub phonology: Phonology,
    /// Lexique du groupe fondateur, pour les mots affichés.
    #[serde(with = "crate::language::lexicon_serde")]
    pub lexicon: Box<Lexicon>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotableKind {
    Inventor,
    Transmitter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notable {
    pub id: u32,
    pub kind: NotableKind,
    pub species: u32,
    pub people: u32,
    pub cell: u32,
    pub trait_id: TraitId,
    pub born_year: f64,
    pub death_year: f64,
    pub name: String,
}

/// Phénomène vu par un groupe et resté inexpliqué.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unexplained {
    pub year: f64,
    pub cell: u32,
    pub species: u32,
    pub people: u32,
    pub phenomenon: Phenomenon,
    pub intervention: bool,
    pub myth: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counters {
    pub inventions: u64,
    pub generated: u64,
    pub introductions: u64,
    pub losses: u64,
    pub notables: u64,
    pub myths: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CultureWorld {
    pub seed: u64,
    pub year: f64,
    /// Pas effectués : il entre dans le hasard de chaque pas.
    pub tick: u64,
    pub species: Vec<Species>,
    pub groups: Vec<Group>,
    pub registry: BTreeMap<TraitId, TraitDef>,
    pub peoples: Vec<PeopleRecord>,
    pub languages: Vec<LanguageRecord>,
    pub myths: Vec<Myth>,
    pub notables: Vec<Notable>,
    pub unexplained: Vec<Unexplained>,
    pub counters: Counters,
    next_trait: TraitId,
    next_notable: u32,
    cell_links: BTreeMap<u32, Vec<(u32, f64)>>,
    #[serde(skip)]
    index: BTreeMap<(u32, u32), usize>,
    #[serde(skip)]
    dirty: bool,
}

/// Solution exacte de dp/dt = λ·p·(1 − p) − μ·p sur `gens` générations.
pub fn sis(p: f64, lambda: f64, mu: f64, gens: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if lambda <= 1e-12 {
        return p * (-mu * gens).dexp();
    }
    let r = lambda - mu;
    let out = if r.abs() < 1e-9 {
        p / (1.0 + lambda * p * gens)
    } else {
        let k = r / lambda;
        let e = (r * gens).dexp();
        k * p * e / (k + p * (e - 1.0))
    };
    out.clamp(0.0, 1.0)
}

/// Fréquence d'équilibre : 1 − μ/λ, ou zéro si λ ≤ μ.
pub fn sis_equilibrium(lambda: f64, mu: f64) -> f64 {
    if lambda > mu {
        1.0 - mu / lambda
    } else {
        0.0
    }
}

fn poisson<R: Rng>(rng: &mut R, mean: f64) -> u64 {
    if mean <= 0.0 {
        return 0;
    }
    if mean < 1e-3 {
        return u64::from(rng.random_bool(mean));
    }
    Poisson::new(mean).map(|d| d.sample(rng) as u64).unwrap_or(0)
}

/// Distance culturelle : 1 − Jaccard pondéré des fréquences.
pub fn cultural_distance(a: &[Slot], b: &[Slot]) -> f64 {
    let (mut i, mut j) = (0, 0);
    let (mut lo, mut hi) = (0.0, 0.0);
    while i < a.len() || j < b.len() {
        let (x, y) = match (a.get(i), b.get(j)) {
            (Some(s), Some(t)) if s.id == t.id => {
                i += 1;
                j += 1;
                (s.freq, t.freq)
            }
            (Some(s), Some(t)) if s.id < t.id => {
                i += 1;
                (s.freq, 0.0)
            }
            (Some(_), Some(t)) => {
                j += 1;
                (0.0, t.freq)
            }
            (Some(s), None) => {
                i += 1;
                (s.freq, 0.0)
            }
            (None, Some(t)) => {
                j += 1;
                (0.0, t.freq)
            }
            (None, None) => break,
        };
        lo += x.min(y);
        hi += x.max(y);
    }
    if hi <= 0.0 {
        0.0
    } else {
        1.0 - lo / hi
    }
}

/// Ce qu'un pas change dans un groupe, calculé sur l'état d'avant le pas.
struct Plan {
    traits: Vec<Slot>,
    lost: u64,
    /// Traits arrivés des voisins, avec le peuple qui les apporte.
    introduced: Vec<(TraitId, u32)>,
    invention: Option<Family>,
    generated: Option<Generated>,
    words: Vec<(usize, u32)>,
    cohesion: f64,
}

/// Contenu d'un trait généré, avant son inscription au registre.
struct Generated {
    kind: TraitKind,
    name: String,
    effects: Effects,
    appeal: f64,
    topic: u8,
}

/// Ce qu'un groupe reçoit de ses voisins pour un trait.
#[derive(Clone, Copy, Debug)]
struct Import {
    id: TraitId,
    /// Somme des poids × fréquences.
    pressure: f64,
    /// Fréquence moyenne pondérée chez les voisins qui l'ont.
    freq: f64,
    weight: f64,
    mastery: f64,
    /// Peuple du voisin qui le porte le plus.
    people: u32,
    best: f64,
    appeal: f64,
    niche: u16,
}

impl CultureWorld {
    pub fn new(seed: u64) -> CultureWorld {
        CultureWorld {
            seed,
            year: 0.0,
            tick: 0,
            species: Vec::new(),
            groups: Vec::new(),
            registry: BTreeMap::new(),
            peoples: Vec::new(),
            languages: Vec::new(),
            myths: Vec::new(),
            notables: Vec::new(),
            unexplained: Vec::new(),
            counters: Counters::default(),
            next_trait: FIRST_GENERATED,
            next_notable: 0,
            cell_links: BTreeMap::new(),
            index: BTreeMap::new(),
            dirty: true,
        }
    }

    /// Ajoute une espèce. Sans apprentissage social, elle n'aura pas de
    /// groupes.
    pub fn add_species(&mut self, id: u32, cognition: Cognition) {
        let learning = Learning::of(&cognition);
        let s = Species { id, cognition, learning, gain: f64::NEG_INFINITY, positive_steps: 0, historical_since: None };
        match self.species.binary_search_by_key(&id, |s| s.id) {
            Ok(i) => {
                let old = &self.species[i];
                self.species[i] =
                    Species { gain: old.gain, positive_steps: old.positive_steps, historical_since: old.historical_since, ..s };
            }
            Err(i) => self.species.insert(i, s),
        }
    }

    pub fn species(&self, id: u32) -> Option<&Species> {
        self.species.binary_search_by_key(&id, |s| s.id).ok().map(|i| &self.species[i])
    }

    fn species_index(&self, id: u32) -> usize {
        self.species.binary_search_by_key(&id, |s| s.id).expect("espèce inconnue de la culture")
    }

    /// Lien symétrique entre deux cellules (part des individus en contact).
    pub fn link(&mut self, a: u32, b: u32, weight: f64) {
        for (x, y) in [(a, b), (b, a)] {
            let l = self.cell_links.entry(x).or_default();
            match l.binary_search_by_key(&y, |e| e.0) {
                Ok(i) => l[i].1 = weight,
                Err(i) => l.insert(i, (y, weight)),
            }
        }
        self.dirty = true;
    }

    pub fn group_index(&mut self, cell: u32, species: u32) -> Option<usize> {
        self.refresh();
        self.index.get(&(cell, species)).copied()
    }

    pub fn group(&self, cell: u32, species: u32) -> Option<&Group> {
        self.groups.iter().find(|g| g.cell == cell && g.species == species)
    }

    /// Fonde un groupe : nouveau peuple et nouvelle langue, ou héritage du
    /// groupe de même espèce de la cellule `from` (colonisation).
    pub fn add_group(&mut self, cell: u32, species: u32, population: f64, context: Context, from: Option<u32>) -> bool {
        let sp = self.species_index(species);
        if !self.species[sp].learning.social() || self.group(cell, species).is_some() {
            return false;
        }
        let parent = from.and_then(|c| self.group(c, species).cloned());
        let g = match parent {
            Some(p) => {
                let traits = p
                    .traits
                    .iter()
                    .filter(|s| match self.registry_kind(s.id) {
                        TraitKind::Technique(f) => f.technique().variant(&context).is_some(),
                        _ => true,
                    })
                    .copied()
                    .collect();
                Group { cell, species, population, context, traits, links: Vec::new(), ..p }
            }
            None => {
                let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_FOUND, cell as u64, species as u64, self.tick]);
                let lexicon = language::random_lexicon(&mut rng);
                let phonology = Phonology::random(&mut rng);
                let language = self.languages.len() as u32;
                self.languages.push(LanguageRecord {
                    id: language,
                    species,
                    parent: None,
                    born_year: self.year,
                    ended_year: None,
                    name: phonology.name(lexicon[1]),
                    phonology: phonology.clone(),
                    lexicon: lexicon.clone(),
                });
                let people = self.peoples.len() as u32;
                self.peoples.push(PeopleRecord {
                    id: people,
                    species,
                    parent: None,
                    born_year: self.year,
                    ended_year: None,
                    name: phonology.name(lexicon[0]),
                    language,
                    founder_cell: cell,
                });
                let cohesion = base_cohesion(&self.species[sp].cognition);
                Group { cell, species, population, context, traits: Vec::new(), lexicon, language, people, cohesion, links: Vec::new() }
            }
        };
        self.groups.push(g);
        self.groups.sort_by_key(|g| (g.cell, g.species));
        self.dirty = true;
        true
    }

    /// Pose un trait dans un groupe (bancs d'essai, scénarios).
    pub fn seed_trait(&mut self, cell: u32, species: u32, id: TraitId, freq: f64, mastery: f64) {
        let year = self.year;
        if let Some(g) = self.groups.iter().position(|g| g.cell == cell && g.species == species) {
            let slot = self.slot_for(id, freq.clamp(0.0, 1.0), mastery.clamp(0.0, 1.0), year);
            let reg = std::mem::take(&mut self.registry);
            self.groups[g].insert(slot, &reg);
            self.registry = reg;
        }
    }

    pub fn remove_group(&mut self, cell: u32, species: u32) {
        self.groups.retain(|g| !(g.cell == cell && g.species == species));
        self.dirty = true;
    }

    pub fn set_population(&mut self, cell: u32, species: u32, population: f64) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.cell == cell && g.species == species) {
            g.population = population.max(0.0);
        }
    }

    fn refresh(&mut self) {
        if !self.dirty {
            return;
        }
        self.index = self.groups.iter().enumerate().map(|(i, g)| ((g.cell, g.species), i)).collect();
        let mut links = Vec::with_capacity(self.groups.len());
        for g in &self.groups {
            let mut l = Vec::new();
            if let Some(cl) = self.cell_links.get(&g.cell) {
                for &(c, w) in cl {
                    if let Some(&j) = self.index.get(&(c, g.species)) {
                        if w > 0.0 {
                            l.push((j, w));
                        }
                    }
                }
            }
            links.push(l);
        }
        for (g, l) in self.groups.iter_mut().zip(links) {
            g.links = l;
        }
        self.dirty = false;
    }

    fn registry_kind(&self, id: TraitId) -> TraitKind {
        if id < FIRST_GENERATED {
            TraitKind::Technique(Family::from_index(id as usize).expect("technique hors catalogue"))
        } else {
            self.registry.get(&id).map(|d| d.kind).unwrap_or(TraitKind::Belief)
        }
    }

    /// Individus connectés d'un groupe : les siens et ceux de ses voisins,
    /// au poids de leur lien.
    pub fn connected(&self, g: usize) -> f64 {
        let grp = &self.groups[g];
        grp.population + grp.links.iter().map(|&(j, w)| w * self.groups[j].population).sum::<f64>()
    }

    /// Attrait d'un trait : biais de succès pour une technique, de contenu
    /// pour un trait généré.
    /// Case d'un trait, avec son attrait et sa niche.
    pub fn slot_for(&self, id: TraitId, freq: f64, mastery: f64, since: f64) -> Slot {
        Slot { id, freq, mastery, since, appeal: self.appeal(id), niche: self.niche(id).unwrap_or(0) }
    }

    fn appeal(&self, id: TraitId) -> f64 {
        match self.registry_kind(id) {
            TraitKind::Technique(f) => 1.0 + f.technique().benefit,
            _ => self.registry.get(&id).map(|d| d.appeal).unwrap_or(1.0),
        }
    }

    /// Avance la culture de `dt` années : pas à pas si `dt` est court, à
    /// l'équilibre sinon. Évalue ensuite le gain culturel de chaque espèce
    /// et le critère de bascule.
    pub fn step(&mut self, dt: f64) {
        self.refresh();
        if dt <= DYNAMIC_MAX_YEARS {
            let n = dt.ceil().max(1.0) as u64;
            let sub = dt / n as f64;
            for _ in 0..n {
                self.dynamic(sub);
                self.year += sub;
                self.tick += 1;
                self.relabel();
                self.housekeeping(sub);
            }
        } else {
            self.equilibrium(dt);
            self.year += dt;
            self.tick += 1;
            self.relabel();
            self.housekeeping(dt);
        }
        self.monitor();
    }

    /// Gain culturel et critère de bascule : gain positif sur deux pas
    /// consécutifs.
    fn monitor(&mut self) {
        for s in 0..self.species.len() {
            let id = self.species[s].id;
            let l = self.species[s].learning;
            let gain = (0..self.groups.len())
                .filter(|&g| self.groups[g].species == id)
                .map(|g| l.cultural_gain(self.connected(g)))
                .fold(f64::NEG_INFINITY, f64::max);
            let sp = &mut self.species[s];
            sp.gain = gain;
            if gain > 0.0 {
                sp.positive_steps += 1;
            } else {
                sp.positive_steps = 0;
            }
            if sp.positive_steps >= 2 && sp.historical_since.is_none() {
                sp.historical_since = Some(self.year);
            }
            if gain == f64::NEG_INFINITY {
                // Plus aucun groupe : l'espèce quitte le mode historique.
                sp.historical_since = None;
            }
        }
    }

    /// Le monde est en mode historique dès qu'une espèce y est.
    pub fn historical(&self) -> bool {
        self.species.iter().any(|s| s.historical())
    }

    fn imports(&self, g: usize) -> Vec<Import> {
        let grp = &self.groups[g];
        let mut all: Vec<(&Slot, f64, u32)> = Vec::with_capacity(grp.links.len() * 32);
        for &(j, w) in &grp.links {
            let nb = &self.groups[j];
            all.extend(nb.traits.iter().map(|s| (s, w, nb.people)));
        }
        all.sort_by_key(|e| e.0.id);
        let mut out: Vec<Import> = Vec::with_capacity(all.len());
        for (s, w, people) in all {
            let p = w * s.freq;
            match out.last_mut() {
                Some(e) if e.id == s.id => {
                    if p > e.best {
                        e.best = p;
                        e.people = people;
                    }
                    e.pressure += p;
                    e.freq = (e.freq * e.weight + s.freq * w) / (e.weight + w);
                    e.weight += w;
                    e.mastery = e.mastery.max(s.mastery);
                }
                _ => out.push(Import {
                    id: s.id,
                    pressure: p,
                    freq: s.freq,
                    weight: w,
                    mastery: s.mastery,
                    people,
                    best: p,
                    appeal: s.appeal,
                    niche: s.niche,
                }),
            }
        }
        out
    }

    fn niche(&self, id: TraitId) -> Option<u16> {
        if id < FIRST_GENERATED {
            return None;
        }
        self.registry.get(&id).and_then(|d| d.kind.niche(d.topic))
    }

    fn dynamic(&mut self, dt: f64) {
        // Chaque groupe est calculé sur l'état d'avant le pas, en parallèle :
        // l'ordre et le nombre de cœurs ne changent rien.
        let boosts = self.notable_effects();
        let plans: Vec<Plan> = (0..self.groups.len()).into_par_iter().map(|g| self.plan_group(g, dt, &boosts)).collect();
        for (g, plan) in plans.into_iter().enumerate() {
            self.apply(g, plan);
        }
    }

    /// Effets des notables vivants : (groupe, trait) → (facteur de λ,
    /// plancher d'un porteur).
    fn notable_effects(&self) -> BTreeMap<(u32, u32, TraitId), (f64, bool)> {
        let mut out = BTreeMap::new();
        for n in self.notables.iter().filter(|n| n.death_year > self.year) {
            let e = out.entry((n.cell, n.species, n.trait_id)).or_insert((1.0, false));
            match n.kind {
                NotableKind::Inventor => e.1 = true,
                // Biais de prestige : le passeur double la transmission de
                // ce qu'il apporte, pendant sa vie.
                NotableKind::Transmitter => e.0 = 2.0,
            }
        }
        out
    }

    fn plan_group(&self, g: usize, dt: f64, boosts: &BTreeMap<(u32, u32, TraitId), (f64, bool)>) -> Plan {
        let grp = &self.groups[g];
        let species = self.species(grp.species).expect("espèce");
        let l = species.learning;
        let (cell, sid) = (grp.cell, grp.species);
        let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_GROUP, cell as u64, sid as u64, self.tick]);
        let imports = self.imports(g);
        let connected = self.connected(g);
        let n = grp.population.max(1.0);
        let gens = dt / l.generation_years;
        let lambda = l.lambda();
        let ctx = grp.context;
        let ln_connected = connected.max(1.0).dln();
        // Dérive : un trait neutre finit par être remplacé, en 2·N
        // générations en moyenne.
        let tau = (2.0 * connected * l.generation_years).clamp(500.0, 1e6);
        let drift = 1.0 - (-dt / tau).dexp();
        // Part déjà prise de chaque niche de traits générés.
        let mut taken: Vec<(u16, f64)> = Vec::new();
        for s in &grp.traits {
            if let Some(k) = s.niche() {
                match taken.iter_mut().find(|t| t.0 == k) {
                    Some(t) => t.1 += s.freq,
                    None => taken.push((k, s.freq)),
                }
            }
        }
        let taken_of = |k: u16| taken.iter().find(|t| t.0 == k).map(|t| t.1).unwrap_or(0.0);
        let mut cohesion = base_cohesion(&species.cognition);
        let mut traits: Vec<Slot> = Vec::with_capacity(grp.traits.len() + 2);
        let mut lost = 0u64;
        for s in &grp.traits {
            let (boost, floor_one) =
                if boosts.is_empty() { (1.0, false) } else { boosts.get(&(cell, sid, s.id)).copied().unwrap_or((1.0, false)) };
            let niche = s.niche();
            // Les traits d'une même niche se partagent les apprenants.
            let room = niche.map(|k| (1.0 - (taken_of(k) - s.freq)).clamp(0.0, 1.0)).unwrap_or(1.0);
            let lam = lambda * s.appeal * boost * room;
            let mut p = sis(s.freq, lam, 1.0, gens);
            p += l.conformism * p * (1.0 - p) * (2.0 * p - 1.0) * gens.min(1.0);
            let mut m = s.mastery;
            // Les techniques se mêlent entre voisins ; un trait neutre ne passe
            // que par la transmission, dans la place que lui laisse sa niche.
            let mixed = if niche.is_none() { imports.binary_search_by_key(&s.id, |e| e.id).ok() } else { None };
            if let Some(i) = mixed {
                let e = imports[i];
                let k = (MIXING_PER_YEAR * e.weight * dt).min(1.0);
                p += k * (e.freq - p);
                if e.mastery > m {
                    m += k * (e.mastery - m);
                }
            }
            p = p.clamp(0.0, 1.0);
            let carriers = p * n;
            if carriers < SMALL_CARRIERS {
                p = poisson(&mut rng, carriers) as f64 / n;
            }
            if floor_one {
                p = p.max(1.0 / n);
            }
            let mut keep = p > 0.0;
            if s.id < FIRST_GENERATED {
                match CATALOG[s.id as usize].feasible(&ctx, &species.cognition, &l) {
                    Some(v) => {
                        m += l.skill_gain_ln(v.difficulty, ln_connected) / v.difficulty * gens;
                        m = m.clamp(l.individual_mastery(v.difficulty), 1.0);
                        keep &= m >= FUNCTIONAL;
                    }
                    None => keep = false,
                }
            } else {
                keep &= !rng.random_bool(drift);
            }
            if keep {
                if s.id >= FIRST_GENERATED {
                    if let Some(d) = self.registry.get(&s.id) {
                        cohesion += p * d.effects.cohesion;
                    }
                }
                traits.push(Slot { freq: p, mastery: m, ..*s });
            } else {
                lost += 1;
            }
        }
        // Arrivées depuis les voisins.
        let mut introduced = Vec::new();
        for e in &imports {
            if grp.has(e.id) {
                continue;
            }
            // Un trait neutre n'entre que s'il reste de la place dans sa
            // niche.
            let room = if e.niche != 0 { (1.0 - taken_of(e.niche)).max(0.0) } else { 1.0 };
            let rate = INTRODUCTION_PER_YEAR * l.fidelity * e.pressure * room * dt;
            let chance = if rate < 1e-3 { rate } else { 1.0 - (-rate).dexp() };
            if !rng.random_bool(chance) {
                continue;
            }
            if e.id < FIRST_GENERATED {
                if CATALOG[e.id as usize].feasible(&ctx, &species.cognition, &l).is_none() {
                    continue;
                }
            } else if !self.registry.contains_key(&e.id) {
                continue;
            }
            let p0 = (2.0 / n).min(0.05);
            traits.push(Slot { id: e.id, freq: p0, mastery: e.mastery, since: self.year, appeal: e.appeal, niche: e.niche });
            introduced.push((e.id, e.people));
        }
        traits.sort_by_key(|s| s.id);
        if traits.len() > MAX_TRAITS {
            // On évince les traits générés les moins répandus.
            let mut order: Vec<usize> = (0..traits.len()).collect();
            order.sort_by(|&a, &b| {
                let (x, y) = (&traits[a], &traits[b]);
                (x.id < FIRST_GENERATED).cmp(&(y.id < FIRST_GENERATED)).then(x.freq.total_cmp(&y.freq)).then(y.id.cmp(&x.id))
            });
            let drop: std::collections::BTreeSet<usize> = order[..traits.len() - MAX_TRAITS].iter().copied().collect();
            lost += drop.len() as u64;
            traits = traits.into_iter().enumerate().filter(|(i, _)| !drop.contains(i)).map(|(_, s)| s).collect();
        }
        // Inventions.
        let mut invention = None;
        let rate = INVENTION_PER_CAPITA_YEAR * l.inventiveness * connected * dt;
        if rate > 0.0 && rng.random_bool(1.0 - (-rate).dexp()) {
            invention = self.invent(g, &mut rng);
        }
        let mut generated = None;
        if l.symbolic >= SYMBOLIC_MIN {
            let rate = GENERATED_PER_CAPITA_YEAR * l.symbolic * l.symbolic * connected * dt;
            if rng.random_bool(1.0 - (-rate).dexp()) {
                generated = Some(self.draw_generated(g, &mut rng));
            }
        }
        // Mots : alignement sur la majorité des voisins (conformisme
        // linguistique), puis renouvellement propre.
        let mut words = Vec::new();
        let mut lrng = rng_for(self.seed, Stream::Culture, &[TAG_LANGUAGE, cell as u64, sid as u64, self.tick]);
        let contact: f64 = grp.links.iter().map(|l| l.1).sum();
        let k = poisson(&mut lrng, MEANINGS as f64 * language::BORROW_PER_YEAR * contact * dt);
        for _ in 0..k {
            let m = lrng.random_range(0..MEANINGS);
            let mut votes: Vec<(u32, f64)> = vec![(grp.lexicon[m], language::SELF_WEIGHT)];
            for &(j, w) in &grp.links {
                let f = self.groups[j].lexicon[m];
                match votes.iter_mut().find(|v| v.0 == f) {
                    Some(v) => v.1 += w,
                    None => votes.push((f, w)),
                }
            }
            // Une fois sur deux, on copie un voisin au hasard (au poids du
            // lien) : les innovations peuvent gagner, les frontières bougent.
            let best = if lrng.random_bool(language::VOTER_SHARE) {
                let mut u = lrng.random::<f64>() * contact;
                let mut pick = votes[0];
                for &(j, w) in &grp.links {
                    u -= w;
                    if u <= 0.0 {
                        pick = (self.groups[j].lexicon[m], w);
                        break;
                    }
                }
                pick
            } else {
                votes.iter().fold(votes[0], |b, v| if v.1 > b.1 { *v } else { b })
            };
            if best.0 != grp.lexicon[m] {
                words.push((m, best.0));
            }
        }
        let k = poisson(&mut lrng, MEANINGS as f64 * language::REPLACEMENT_PER_YEAR * dt);
        for _ in 0..k {
            words.push((lrng.random_range(0..MEANINGS), lrng.random()));
        }
        Plan { traits, lost, introduced, invention, generated, words, cohesion: cohesion.clamp(0.0, 1.0) }
    }

    fn apply(&mut self, g: usize, plan: Plan) {
        let (cell, sid, people) = (self.groups[g].cell, self.groups[g].species, self.groups[g].people);
        let historical = self.species(sid).is_some_and(|s| s.historical());
        let n = self.groups[g].population.max(1.0);
        self.groups[g].traits = plan.traits;
        self.counters.losses += plan.lost;
        self.counters.introductions += plan.introduced.len() as u64;
        for (m, f) in plan.words {
            self.groups[g].lexicon[m] = f;
        }
        let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_NOTABLE, cell as u64, sid as u64, self.tick]);
        if historical {
            for &(id, from) in &plan.introduced {
                if from != people && id < FIRST_GENERATED {
                    self.add_notable(NotableKind::Transmitter, g, id, &mut rng);
                }
            }
        }
        if let Some(f) = plan.invention {
            let slot = self.slot_for(f as TraitId, 1.0 / n, INVENTION_MASTERY, self.year);
            self.groups[g].insert(slot, &self.registry);
            self.counters.inventions += 1;
            if historical {
                self.add_notable(NotableKind::Inventor, g, f as TraitId, &mut rng);
            }
        }
        if let Some(d) = plan.generated {
            let id = self.register(d, g);
            let slot = self.slot_for(id, 1.0 / n, 1.0, self.year);
            self.groups[g].insert(slot, &self.registry);
        }
        self.groups[g].cohesion = plan.cohesion;
    }

    /// Choisit l'invention d'un groupe : une technique du catalogue dont les
    /// prérequis sont maîtrisés et que le corps permet, les plus faciles
    /// plus souvent.
    fn invent<R: Rng>(&self, g: usize, rng: &mut R) -> Option<Family> {
        let grp = &self.groups[g];
        let sp = self.species(grp.species)?;
        let mut cands: Vec<(Family, f64)> = Vec::new();
        for t in CATALOG.iter() {
            if grp.has(t.family as TraitId) || !t.prerequisites.iter().all(|&p| grp.knows(p)) {
                continue;
            }
            if let Some(v) = t.feasible(&grp.context, &sp.cognition, &sp.learning) {
                cands.push((t.family, 1.0 / v.difficulty));
            }
        }
        let total: f64 = cands.iter().map(|c| c.1).sum();
        if total <= 0.0 {
            return None;
        }
        let mut u = rng.random::<f64>() * total;
        for (f, w) in &cands {
            u -= w;
            if u <= 0.0 {
                return Some(*f);
            }
        }
        cands.last().map(|c| c.0)
    }

    fn phonology_of(&self, g: usize) -> Phonology {
        self.languages[self.groups[g].language as usize].phonology.clone()
    }

    fn draw_generated<R: Rng>(&self, g: usize, rng: &mut R) -> Generated {
        let kind = match rng.random_range(0..20) {
            0..=6 => TraitKind::Belief,
            7..=11 => TraitKind::Rite,
            12..=16 => TraitKind::Art,
            _ => TraitKind::Taboo(Resource::ALL[rng.random_range(0..Resource::ALL.len())]),
        };
        let ph = self.phonology_of(g);
        let (name, effects, appeal, topic) = traits::generate(rng, kind, &ph);
        Generated { kind, name, effects, appeal, topic }
    }

    fn register(&mut self, d: Generated, g: usize) -> TraitId {
        let id = self.next_trait;
        self.next_trait += 1;
        let grp = &self.groups[g];
        if d.kind.niche(d.topic).is_some() && !matches!(d.kind, TraitKind::Myth(_)) {
            self.counters.generated += 1;
        }
        self.registry.insert(
            id,
            TraitDef {
                id,
                kind: d.kind,
                name: d.name,
                effects: d.effects,
                appeal: d.appeal,
                topic: d.topic,
                origin_year: self.year,
                origin_cell: grp.cell,
                origin_species: grp.species,
            },
        );
        id
    }

    fn add_notable<R: Rng>(&mut self, kind: NotableKind, g: usize, trait_id: TraitId, rng: &mut R) {
        let alive = self.notables.iter().filter(|n| n.death_year > self.year).count();
        if alive >= MAX_NOTABLES {
            return;
        }
        let grp = &self.groups[g];
        let l = self.species(grp.species).map(|s| s.learning).expect("espèce");
        let ph = self.languages[grp.language as usize].phonology.clone();
        let id = self.next_notable;
        self.next_notable += 1;
        let life = l.generation_years * rng.random_range(0.5..1.5);
        self.notables.push(Notable {
            id,
            kind,
            species: grp.species,
            people: grp.people,
            cell: grp.cell,
            trait_id,
            born_year: self.year,
            death_year: self.year + life,
            name: ph.name(rng.random()),
        });
        self.counters.notables += 1;
    }

    fn cohesion_of(&self, g: usize, cog: &Cognition) -> f64 {
        let grp = &self.groups[g];
        let mut c = base_cohesion(cog);
        for s in &grp.traits {
            if s.id >= FIRST_GENERATED {
                if let Some(d) = self.registry.get(&s.id) {
                    c += s.freq * d.effects.cohesion;
                }
            }
        }
        c.clamp(0.0, 1.0)
    }

    /// Composantes connexes des groupes d'une même espèce par leurs liens.
    fn components(&self) -> Vec<usize> {
        let n = self.groups.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for g in 0..n {
            for &(j, _) in &self.groups[g].links {
                let (a, b) = (find(&mut parent, g), find(&mut parent, j));
                if a != b {
                    let (lo, hi) = (a.min(b), a.max(b));
                    parent[hi] = lo;
                }
            }
        }
        (0..n).map(|g| find(&mut parent, g)).collect()
    }

    fn equilibrium(&mut self, dt: f64) {
        let comp = self.components();
        // Inventions du pas, groupe par groupe.
        for g in 0..self.groups.len() {
            let grp = &self.groups[g];
            let l = self.species(grp.species).expect("espèce").learning;
            let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_EQUILIBRIUM, grp.cell as u64, grp.species as u64, self.tick]);
            let connected = self.connected(g);
            let k = poisson(&mut rng, (INVENTION_PER_CAPITA_YEAR * l.inventiveness * connected * dt).min(20.0));
            for _ in 0..k.min(8) {
                if let Some(f) = self.invent(g, &mut rng) {
                    let slot = self.slot_for(f as TraitId, 0.0, INVENTION_MASTERY, self.year + rng.random::<f64>() * dt);
                    self.groups[g].insert(slot, &self.registry);
                    self.counters.inventions += 1;
                } else {
                    break;
                }
            }
            if l.symbolic >= SYMBOLIC_MIN {
                let k = poisson(&mut rng, (GENERATED_PER_CAPITA_YEAR * l.symbolic * l.symbolic * connected * dt).min(4.0));
                for _ in 0..k {
                    let d = self.draw_generated(g, &mut rng);
                    let id = self.register(d, g);
                    let since = self.year + rng.random::<f64>() * dt;
                    let slot = self.slot_for(id, 0.0, 1.0, since);
                    self.groups[g].insert(slot, &self.registry);
                }
            }
        }
        // Réserve de techniques de chaque composante : l'union de celles de
        // ses groupes. Une technique utile gagne toute la composante en bien
        // moins d'un pas ; un trait neutre (croyance, rite, art, tabou,
        // mythe) reste à son groupe et y dérive.
        let mut pools: BTreeMap<usize, BTreeMap<TraitId, Slot>> = BTreeMap::new();
        for (g, grp) in self.groups.iter().enumerate() {
            let pool = pools.entry(comp[g]).or_default();
            for s in grp.traits.iter().filter(|s| s.id < FIRST_GENERATED) {
                pool.entry(s.id)
                    .and_modify(|e| {
                        e.mastery = e.mastery.max(s.mastery);
                        e.since = e.since.min(s.since);
                    })
                    .or_insert(*s);
            }
        }
        // Équilibre de chaque groupe.
        let connected: Vec<f64> = (0..self.groups.len()).map(|g| self.connected(g)).collect();
        let mut lost = 0;
        for g in 0..self.groups.len() {
            let sp = self.species(self.groups[g].species).expect("espèce").clone();
            let l = sp.learning;
            let ctx = self.groups[g].context;
            let mut next = Vec::new();
            // Dérive des traits neutres : remplacés en 2·N générations en
            // moyenne.
            let tau = (2.0 * connected[g] * l.generation_years).clamp(500.0, 1e6);
            let grp = &self.groups[g];
            let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_COMPONENT, grp.cell as u64, grp.species as u64, self.tick]);
            let survive = (-dt / tau).dexp();
            let own: Vec<Slot> = grp.traits.iter().filter(|s| s.id >= FIRST_GENERATED && rng.random_bool(survive)).copied().collect();
            for s in pools[&comp[g]].values().chain(own.iter()) {
                let lam = l.lambda() * s.appeal;
                let p = sis_equilibrium(lam, 1.0);
                let mut m = 1.0;
                let mut ok = p > 0.0;
                if let TraitKind::Technique(f) = self.registry_kind(s.id) {
                    match f.technique().feasible(&ctx, &sp.cognition, &l) {
                        Some(v) => {
                            m = if l.skill_gain(v.difficulty, connected[g]) > 0.0 { 1.0 } else { l.individual_mastery(v.difficulty) };
                            ok &= m >= FUNCTIONAL;
                        }
                        None => ok = false,
                    }
                }
                if ok {
                    next.push(Slot { freq: p, mastery: m, ..*s });
                }
            }
            lost += self.groups[g].traits.iter().filter(|s| next.binary_search_by_key(&s.id, |n| n.id).is_err()).count() as u64;
            // Prérequis : une technique sans les siennes se perd aussi.
            loop {
                let before = next.len();
                let known: Vec<TraitId> = next.iter().map(|s| s.id).collect();
                next.retain(|s| match self.registry_kind(s.id) {
                    TraitKind::Technique(f) => f.technique().prerequisites.iter().all(|p| known.contains(&(*p as TraitId))),
                    _ => true,
                });
                if next.len() == before {
                    break;
                }
            }
            // Concurrence : dans chaque niche, le trait le plus attrayant
            // l'emporte à l'équilibre (exclusion compétitive).
            let mut best: BTreeMap<u16, (f64, f64, TraitId)> = BTreeMap::new();
            for s in &next {
                if let Some(k) = s.niche() {
                    let cand = (s.appeal, -s.since, s.id);
                    let e = best.entry(k).or_insert(cand);
                    if cand.0 > e.0 || (cand.0 == e.0 && (cand.1 > e.1 || (cand.1 == e.1 && cand.2 < e.2))) {
                        *e = cand;
                    }
                }
            }
            next.retain(|s| s.niche().is_none_or(|k| best[&k].2 == s.id));
            next.sort_by_key(|s| s.id);
            if next.len() > MAX_TRAITS {
                next.sort_by(|a, b| {
                    (a.id >= FIRST_GENERATED).cmp(&(b.id >= FIRST_GENERATED)).then(b.freq.total_cmp(&a.freq)).then(a.id.cmp(&b.id))
                });
                next.truncate(MAX_TRAITS);
                next.sort_by_key(|s| s.id);
            }
            self.groups[g].traits = next;
            let c = self.cohesion_of(g, &sp.cognition);
            self.groups[g].cohesion = c;
        }
        self.counters.losses += lost;
        // Langue : renouvellement propre, puis la composante parle comme son
        // groupe le plus peuplé.
        let keep = language::retention(dt);
        for g in 0..self.groups.len() {
            let grp = &self.groups[g];
            let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_LANGUAGE, grp.cell as u64, grp.species as u64, self.tick]);
            let mut lex = grp.lexicon.clone();
            for f in lex.iter_mut() {
                if !rng.random_bool(keep) {
                    *f = rng.random();
                }
            }
            self.groups[g].lexicon = lex;
        }
        let mut core: BTreeMap<usize, usize> = BTreeMap::new();
        for (g, &k) in comp.iter().enumerate() {
            let c = core.entry(k).or_insert(g);
            if self.groups[g].population > self.groups[*c].population {
                *c = g;
            }
        }
        for g in 0..self.groups.len() {
            let c = core[&comp[g]];
            if c != g {
                let lex = self.groups[c].lexicon.clone();
                self.groups[g].lexicon = lex;
            }
        }
    }

    /// Un phénomène est vu dans une cellule. Chaque groupe qui y vit le note
    /// inexpliqué, et peut en faire un mythe. Renvoie les mythes nés.
    pub fn witness(&mut self, cell: u32, phenomenon: Phenomenon, intervention: bool) -> Vec<u32> {
        self.refresh();
        let mut born = Vec::new();
        let gs: Vec<usize> = (0..self.groups.len()).filter(|&g| self.groups[g].cell == cell).collect();
        for g in gs {
            let grp = &self.groups[g];
            let l = self.species(grp.species).expect("espèce").learning;
            let mut rng = rng_for(
                self.seed,
                Stream::Culture,
                &[TAG_WITNESS, cell as u64, grp.species as u64, self.tick, phenomenon as u64, self.unexplained.len() as u64],
            );
            let myth = if rng.random_bool(myths::myth_chance(l.symbolic)) {
                let id = self.myths.len() as u32;
                let people = grp.people;
                let name = self.peoples[people as usize].name.clone();
                let ph = self.phonology_of(g);
                let m = myths::generate(&mut rng, id, people, &name, grp.species, cell, self.year, phenomenon, intervention, &ph);
                let d = Generated {
                    kind: TraitKind::Myth(id),
                    name: m.title.clone(),
                    effects: m.effects,
                    appeal: 1.2,
                    topic: phenomenon as u8,
                };
                let tid = self.register(d, g);
                // Tout le groupe a vu le phénomène : le mythe part répandu.
                let slot = self.slot_for(tid, 0.5, 1.0, self.year);
                self.groups[g].insert(slot, &self.registry);
                self.myths.push(m);
                self.counters.myths += 1;
                born.push(id);
                Some(id)
            } else {
                None
            };
            let grp = &self.groups[g];
            self.unexplained.push(Unexplained {
                year: self.year,
                cell,
                species: grp.species,
                people: grp.people,
                phenomenon,
                intervention,
                myth,
            });
        }
        born
    }

    fn housekeeping(&mut self, dt: f64) {
        // Changements de son, langue par langue.
        let live: Vec<bool> = {
            let mut v = vec![false; self.languages.len()];
            for g in &self.groups {
                v[g.language as usize] = true;
            }
            v
        };
        for (i, lang) in self.languages.iter_mut().enumerate() {
            if !live[i] {
                continue;
            }
            let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_LANGUAGE, u64::MAX, i as u64, self.tick]);
            let k = poisson(&mut rng, (language::SOUND_CHANGE_PER_YEAR * dt).min(6.0));
            for _ in 0..k {
                lang.phonology.shift(&mut rng);
            }
        }
        // Notables morts : on garde les derniers.
        let dead = self.notables.iter().filter(|n| n.death_year <= self.year).count();
        if dead > KEPT_DEAD_NOTABLES {
            let mut extra = dead - KEPT_DEAD_NOTABLES;
            let year = self.year;
            self.notables.retain(|n| {
                if extra > 0 && n.death_year <= year {
                    extra -= 1;
                    false
                } else {
                    true
                }
            });
        }
        // Traits générés sans porteurs : retirés du registre (les mythes
        // restent dans leur propre registre).
        if self.tick.is_multiple_of(64) {
            let mut carried = std::collections::BTreeSet::new();
            for g in &self.groups {
                for s in &g.traits {
                    carried.insert(s.id);
                }
            }
            self.registry.retain(|id, _| carried.contains(id));
        }
    }

    /// Générations pendant lesquelles un trait a été transmis dans un
    /// groupe.
    pub fn generations(&self, g: &Group, slot: &Slot) -> f64 {
        let t = self.species(g.species).map(|s| s.learning.generation_years).unwrap_or(1.0);
        (self.year - slot.since).max(0.0) / t
    }
}

fn base_cohesion(c: &Cognition) -> f64 {
    (0.3 + 0.5 * c.sociality).clamp(0.0, 1.0)
}
