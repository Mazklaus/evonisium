//! Peuples et langues : des étiquettes émergentes, comme l'espèce.
//!
//! À chaque pas, les groupes d'une même étiquette restent ensemble tant
//! qu'ils sont reliés de proche en proche par des groupes en contact et
//! assez proches du groupe le plus peuplé (le cœur). Ceux qui s'en
//! détachent, par isolement ou par éloignement du cœur, forment une
//! nouvelle étiquette par ensemble connexe, fille de l'ancienne : l'arbre
//! des langues et l'arbre des cultures en découlent. Une étiquette sans
//! groupe est close. [Simplification] Deux étiquettes ne fusionnent jamais.
//!
//! - Langue : distance lexicale (part des sens qui diffèrent, sur
//!   l'expressivité de l'espèce), cœur sous 0,5 : la moitié du vocabulaire
//!   de base changée fait une autre langue, en 4 à 5 ka d'isolement au
//!   rythme de Swadesh.
//! - Peuple : moitié distance culturelle (Jaccard pondéré des traits),
//!   moitié distance lexicale, cœur sous 0,6.

use crate::language::{self, Lexicon};
use crate::world::{CultureWorld, LanguageRecord, PeopleRecord};
use evo_core::rng::{rng_for, Stream};
use std::collections::BTreeMap;

const LANGUAGE_CORE: f64 = 0.5;
const PEOPLE_CORE: f64 = 0.6;
const TAG_LABEL: u64 = 11;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Language,
    People,
}

impl CultureWorld {
    fn meanings(&self, species: u32) -> usize {
        self.species(species).map(|s| s.learning.expressivity as usize).unwrap_or(language::MEANINGS)
    }

    fn distance(&self, kind: Label, a: usize, b: usize) -> f64 {
        let (ga, gb) = (&self.groups[a], &self.groups[b]);
        let lex = language::distance(&ga.lexicon, &gb.lexicon, self.meanings(ga.species));
        match kind {
            Label::Language => lex,
            Label::People => 0.5 * lex + 0.5 * crate::world::cultural_distance(&ga.traits, &gb.traits),
        }
    }

    fn label_of(&self, kind: Label, g: usize) -> u32 {
        match kind {
            Label::Language => self.groups[g].language,
            Label::People => self.groups[g].people,
        }
    }

    /// Recalcule les langues puis les peuples.
    pub(crate) fn relabel(&mut self) {
        self.relabel_kind(Label::Language);
        self.relabel_kind(Label::People);
        let year = self.year;
        let mut live_l = vec![false; self.languages.len()];
        let mut live_p = vec![false; self.peoples.len()];
        for g in &self.groups {
            live_l[g.language as usize] = true;
            live_p[g.people as usize] = true;
        }
        for (r, live) in self.languages.iter_mut().zip(live_l) {
            if !live && r.ended_year.is_none() {
                r.ended_year = Some(year);
            }
        }
        for (r, live) in self.peoples.iter_mut().zip(live_p) {
            if !live && r.ended_year.is_none() {
                r.ended_year = Some(year);
            }
        }
    }

    /// Cœur d'un ensemble de groupes : le plus peuplé, puis le mieux relié.
    fn core_of(&self, gs: &[usize]) -> usize {
        *gs.iter()
            .max_by(|&&a, &&b| {
                let (x, y) = (&self.groups[a], &self.groups[b]);
                x.population.total_cmp(&y.population).then(x.links.len().cmp(&y.links.len())).then(b.cmp(&a))
            })
            .expect("au moins un groupe")
    }

    fn set_label(&mut self, kind: Label, g: usize, id: u32) {
        match kind {
            Label::Language => self.groups[g].language = id,
            Label::People => self.groups[g].people = id,
        }
    }

    fn relabel_kind(&mut self, kind: Label) {
        let core_max = match kind {
            Label::Language => LANGUAGE_CORE,
            Label::People => PEOPLE_CORE,
        };
        let n = self.groups.len();
        let mut members: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for g in 0..n {
            members.entry(self.label_of(kind, g)).or_default().push(g);
        }
        let cores: BTreeMap<u32, usize> = members.iter().map(|(&l, gs)| (l, self.core_of(gs))).collect();
        // Restent : les groupes reliés au cœur de leur étiquette par des
        // groupes de la même étiquette assez proches de ce cœur.
        let mut stays = vec![false; n];
        for (&l, &c) in &cores {
            stays[c] = true;
            let mut stack = vec![c];
            while let Some(g) = stack.pop() {
                for &(j, _) in &self.groups[g].links {
                    if !stays[j] && self.label_of(kind, j) == l && self.distance(kind, j, c) < core_max {
                        stays[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        // Les autres rejoignent, s'il y en a une assez proche, l'étiquette
        // d'un voisin resté : une frontière qui bouge ne fait pas naître une
        // langue par an.
        let mut changed = true;
        while changed {
            changed = false;
            for g in 0..n {
                if stays[g] {
                    continue;
                }
                let mut best: Option<(f64, u32)> = None;
                for &(j, _) in &self.groups[g].links {
                    if !stays[j] {
                        continue;
                    }
                    let l = self.label_of(kind, j);
                    let d = self.distance(kind, g, cores[&l]);
                    if d < core_max && best.is_none_or(|b| d < b.0 || (d == b.0 && l < b.1)) {
                        best = Some((d, l));
                    }
                }
                if let Some((_, l)) = best {
                    self.set_label(kind, g, l);
                    stays[g] = true;
                    changed = true;
                }
            }
        }
        // Les derniers forment une étiquette nouvelle par ensemble connexe,
        // fille de celle de leur groupe le plus peuplé.
        let mut done = stays;
        for s in 0..n {
            if done[s] {
                continue;
            }
            done[s] = true;
            let mut part = vec![s];
            let mut i = 0;
            while i < part.len() {
                let g = part[i];
                i += 1;
                for &(j, _) in &self.groups[g].links {
                    if !done[j] {
                        done[j] = true;
                        part.push(j);
                    }
                }
            }
            let founder = self.core_of(&part);
            let parent = self.label_of(kind, founder);
            let id = self.split(kind, parent, founder);
            for g in part {
                self.set_label(kind, g, id);
            }
        }
    }

    fn split(&mut self, kind: Label, parent: u32, founder: usize) -> u32 {
        let grp = &self.groups[founder];
        let lexicon: Box<Lexicon> = grp.lexicon.clone();
        let (cell, species) = (grp.cell, grp.species);
        match kind {
            Label::Language => {
                let id = self.languages.len() as u32;
                let mut rng = rng_for(self.seed, Stream::Culture, &[TAG_LABEL, 0, id as u64, self.tick]);
                let mut phonology = self.languages[parent as usize].phonology.clone();
                phonology.shift(&mut rng);
                self.languages.push(LanguageRecord {
                    id,
                    species,
                    parent: Some(parent),
                    born_year: self.year,
                    ended_year: None,
                    name: phonology.name(lexicon[1]),
                    phonology,
                    lexicon,
                });
                id
            }
            Label::People => {
                let id = self.peoples.len() as u32;
                let language = grp.language;
                let name = self.languages[language as usize].phonology.name(lexicon[0]);
                self.peoples.push(PeopleRecord {
                    id,
                    species,
                    parent: Some(parent),
                    born_year: self.year,
                    ended_year: None,
                    name,
                    language,
                    founder_cell: cell,
                });
                id
            }
        }
    }
}
