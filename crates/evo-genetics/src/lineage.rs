//! Registre des lignées : la matière de l'arbre du vivant.
//!
//! [Simplification] Étape 1 : une lignée naît quand un mutant au métabolisme
//! nouveau pour sa cellule s'installe (innovation métabolique). Les
//! substitutions à l'intérieur d'une lignée ne créent pas de noeud ; le
//! regroupement en espèces par distance génétique et isolement reproductif
//! viendra avec l'arbre du vivant de l'étape 3.
//!
//! [Simplification] Étape 2 : une partie de plusieurs milliards d'années fonde
//! des millions de lignées microbiennes, presque toutes éphémères. Le génome
//! fondateur d'une lignée éteinte sans lignée fille est oublié (la fiche
//! reste) ; l'archivage complet sur disque arrive avec SQLite à l'étape 3.
//!
//! [Simplification] Étape 5 : les corps complexes fondent des dizaines de
//! millions de lignées par partie. Une lignée éteinte depuis plus de
//! `PRUNE_AFTER_YEARS` et sans descendance gardée sort du registre (les
//! identifiants restent uniques et croissants, le registre devient creux) :
//! l'arbre garde les lignées vivantes, leurs ancêtres et les extinctions
//! récentes.

use crate::genome::Genome;
use std::sync::Arc;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineageRecord {
    pub id: u32,
    /// Lignée mère (elle-même pour les cellules déposées).
    pub parent: u32,
    pub born_years: f64,
    pub origin_cell: u32,
    /// Réactions catalysées à la fondation (bits).
    pub signature: u32,
    /// Génome fondateur, tant que la lignée vit (rangé à part dans les
    /// sauvegardes).
    #[serde(skip)]
    pub founder: Option<Arc<Genome>>,
    pub extinct_years: Option<f64>,
    /// Lignées filles fondées.
    pub children: u32,
    /// Lignées filles encore dans le registre.
    #[serde(default)]
    pub kept_children: u32,
}

/// Délai après l'extinction au-delà duquel une lignée sans descendance gardée
/// sort du registre, en années.
pub const PRUNE_AFTER_YEARS: f64 = 50e6;

/// Intervalle entre deux élagages du registre, en années.
pub const PRUNE_EVERY_YEARS: f64 = 10e6;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LineageRegistry {
    /// Fiches gardées, par identifiant croissant (avec des trous).
    pub records: Vec<LineageRecord>,
    /// Lignées vivantes, par identifiant croissant.
    living: Vec<u32>,
    /// Identifiant de la prochaine lignée : nombre de lignées fondées.
    #[serde(default)]
    next_id: u32,
    /// Date du dernier élagage.
    #[serde(default)]
    pruned_years: f64,
}

impl LineageRegistry {
    /// Enregistre une lignée ; les identifiants ne sont jamais réutilisés.
    pub fn found(&mut self, parent: Option<u32>, years: f64, cell: u32, signature: u32, founder: Arc<Genome>) -> u32 {
        let id = self.total() as u32;
        self.next_id = id + 1;
        if let Some(p) = parent.and_then(|p| self.index(p)) {
            self.records[p].children += 1;
            self.records[p].kept_children += 1;
        }
        self.records.push(LineageRecord {
            id,
            parent: parent.unwrap_or(id),
            born_years: years,
            origin_cell: cell,
            signature,
            founder: Some(founder),
            extinct_years: None,
            children: 0,
            kept_children: 0,
        });
        self.living.push(id);
        id
    }

    /// Nombre de lignées fondées depuis le début de la partie.
    pub fn total(&self) -> usize {
        // Registres d'avant l'élagage : rien n'en est encore sorti.
        (self.next_id as usize).max(self.records.last().map_or(0, |r| r.id as usize + 1))
    }

    /// Place de la fiche d'une lignée, si elle est encore au registre.
    pub fn index(&self, id: u32) -> Option<usize> {
        self.records.binary_search_by_key(&id, |r| r.id).ok()
    }

    pub fn get(&self, id: u32) -> Option<&LineageRecord> {
        self.index(id).map(|i| &self.records[i])
    }

    /// Fiches des lignées fondées à partir de l'identifiant `id`.
    pub fn since(&self, id: u32) -> &[LineageRecord] {
        &self.records[self.records.partition_point(|r| r.id < id)..]
    }

    pub fn living(&self) -> impl Iterator<Item = &LineageRecord> {
        self.living.iter().filter_map(|&id| self.get(id))
    }

    pub fn living_count(&self) -> usize {
        self.living.len()
    }

    /// Déclare éteintes, à la date `years`, les lignées vivantes absentes de
    /// `present` (identifiants triés) ; renvoie celles qui avaient fondé au
    /// moins une lignée fille (les clades), les autres n'étant que comptées.
    pub fn retire_absent(&mut self, present: &[u32], years: f64) -> (Vec<u32>, usize) {
        let (mut clades, mut leaves) = (Vec::new(), 0);
        let records = &mut self.records;
        self.living.retain(|&id| {
            if present.binary_search(&id).is_ok() {
                return true;
            }
            let Ok(i) = records.binary_search_by_key(&id, |r| r.id) else { return false };
            let rec = &mut records[i];
            rec.extinct_years = Some(years);
            // [Simplification] Le génome fondateur d'une lignée éteinte est
            // oublié, clade ou non : la signature suffit à l'arbre, et les
            // millions de lignées des corps complexes ne tiendraient pas en
            // mémoire avec leurs génomes.
            rec.founder = None;
            if rec.children == 0 {
                leaves += 1;
            } else {
                clades.push(id);
            }
            false
        });
        (clades, leaves)
    }

    /// Retire du registre les lignées éteintes avant `years -
    /// PRUNE_AFTER_YEARS` qui n'ont plus de lignée fille au registre (en
    /// remontant : une mère peut sortir avec sa dernière fille). Renvoie le
    /// nombre de fiches retirées.
    pub fn prune(&mut self, years: f64) -> usize {
        if years < self.pruned_years + PRUNE_EVERY_YEARS {
            return 0;
        }
        self.pruned_years = years;
        let limit = years - PRUNE_AFTER_YEARS;
        let mut gone = vec![false; self.records.len()];
        // Une fille a toujours un identifiant plus grand que sa mère : un
        // seul passage à rebours suffit.
        for i in (0..self.records.len()).rev() {
            let r = &self.records[i];
            if r.kept_children > 0 || r.extinct_years.is_none_or(|y| y > limit) {
                continue;
            }
            gone[i] = true;
            if r.parent != r.id {
                let parent = r.parent;
                if let Ok(p) = self.records[..i].binary_search_by_key(&parent, |r| r.id) {
                    self.records[p].kept_children -= 1;
                }
            }
        }
        let removed = gone.iter().filter(|&&g| g).count();
        if removed > 0 {
            self.next_id = self.total() as u32;
            let mut k = 0;
            self.records.retain(|_| {
                k += 1;
                !gone[k - 1]
            });
            self.records.shrink_to(self.records.len() + self.records.len() / 4);
        }
        removed
    }

    /// Recompte les filles gardées (registres d'avant l'élagage).
    pub fn repair(&mut self) {
        self.next_id = self.total() as u32;
        for r in &mut self.records {
            r.kept_children = 0;
        }
        for i in 0..self.records.len() {
            let (id, parent) = (self.records[i].id, self.records[i].parent);
            if parent != id {
                if let Some(p) = self.index(parent) {
                    self.records[p].kept_children += 1;
                }
            }
        }
    }

    /// Profondeur d'une lignée dans l'arbre (0 pour une lignée déposée).
    pub fn depth(&self, mut id: u32) -> usize {
        let mut d = 0;
        while let Some(r) = self.get(id).filter(|r| r.parent != id) {
            id = r.parent;
            d += 1;
        }
        d
    }

    pub fn memory_bytes(&self) -> usize {
        self.records.capacity() * std::mem::size_of::<LineageRecord>()
            + self.living.capacity() * 4
            + self.records.iter().filter_map(|r| r.founder.as_ref()).map(|g| g.memory_bytes()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pruning_keeps_living_ancestry_and_unique_ids() {
        let g = Arc::new(Genome::new(Vec::new(), [0; crate::genome::MARKER_LEN]));
        let mut reg = LineageRegistry::default();
        let root = reg.found(None, 0.0, 0, 1, g.clone());
        let a = reg.found(Some(root), 1e6, 0, 2, g.clone());
        let b = reg.found(Some(a), 2e6, 0, 3, g.clone());
        let leaf = reg.found(Some(root), 3e6, 0, 4, g.clone());
        let late = reg.found(Some(root), 4e6, 0, 5, g.clone());
        // `b` vit ; `leaf` s'éteint tôt, `late` récemment.
        reg.retire_absent(&[root, a, b, late], 5e6);
        reg.retire_absent(&[root, a, b], 90e6);
        assert_eq!(reg.prune(100e6), 1, "seule la feuille éteinte depuis longtemps sort");
        assert!(reg.get(leaf).is_none() && reg.get(late).is_some());
        assert_eq!(reg.depth(b), 2);
        // `a` et `b` s'éteignent : la branche sort en entier, la racine vit.
        reg.retire_absent(&[root], 101e6);
        assert_eq!(reg.prune(105e6), 0, "pas avant l'intervalle");
        assert_eq!(reg.prune(200e6), 3);
        assert_eq!(reg.records.iter().map(|r| r.id).collect::<Vec<_>>(), vec![root]);
        assert_eq!(reg.found(Some(root), 201e6, 0, 6, g), 5, "identifiants jamais réutilisés");
        assert_eq!(reg.total(), 6);
        assert_eq!(reg.living_count(), 2);
    }
}
