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
    /// Génome fondateur archivé (oublié pour une lignée éteinte sans fille).
    pub founder: Option<Arc<Genome>>,
    pub extinct_years: Option<f64>,
    /// Lignées filles fondées.
    pub children: u32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LineageRegistry {
    pub records: Vec<LineageRecord>,
    /// Lignées vivantes, par identifiant croissant.
    living: Vec<u32>,
}

impl LineageRegistry {
    /// Enregistre une lignée ; les identifiants ne sont jamais réutilisés.
    pub fn found(&mut self, parent: Option<u32>, years: f64, cell: u32, signature: u32, founder: Arc<Genome>) -> u32 {
        let id = self.records.len() as u32;
        if let Some(p) = parent {
            self.records[p as usize].children += 1;
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
        });
        self.living.push(id);
        id
    }

    pub fn living(&self) -> impl Iterator<Item = &LineageRecord> {
        self.living.iter().map(|&id| &self.records[id as usize])
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
            let rec = &mut records[id as usize];
            rec.extinct_years = Some(years);
            if rec.children == 0 {
                rec.founder = None;
                leaves += 1;
            } else {
                clades.push(id);
            }
            false
        });
        (clades, leaves)
    }

    /// Profondeur d'une lignée dans l'arbre (0 pour une lignée déposée).
    pub fn depth(&self, mut id: u32) -> usize {
        let mut d = 0;
        while self.records[id as usize].parent != id {
            id = self.records[id as usize].parent;
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
