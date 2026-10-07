//! Registre des lignées : la matière de l'arbre du vivant.
//!
//! [Simplification] Étape 1 : une lignée naît quand un mutant au métabolisme
//! nouveau pour sa cellule s'installe (innovation métabolique). Les
//! substitutions à l'intérieur d'une lignée ne créent pas de noeud ; le
//! regroupement en espèces par distance génétique et isolement reproductif
//! viendra avec l'arbre du vivant de l'étape 3.

use crate::genome::Genome;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LineageRecord {
    pub id: u32,
    /// Lignée mère (elle-même pour les cellules déposées).
    pub parent: u32,
    pub born_years: f64,
    pub origin_cell: u32,
    /// Réactions catalysées à la fondation (bits).
    pub signature: u32,
    /// Génome fondateur archivé.
    pub founder: Arc<Genome>,
    pub extinct_years: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct LineageRegistry {
    pub records: Vec<LineageRecord>,
}

impl LineageRegistry {
    /// Enregistre une lignée ; les identifiants ne sont jamais réutilisés.
    pub fn found(&mut self, parent: Option<u32>, years: f64, cell: u32, signature: u32, founder: Arc<Genome>) -> u32 {
        let id = self.records.len() as u32;
        self.records.push(LineageRecord {
            id,
            parent: parent.unwrap_or(id),
            born_years: years,
            origin_cell: cell,
            signature,
            founder,
            extinct_years: None,
        });
        id
    }

    pub fn living(&self) -> impl Iterator<Item = &LineageRecord> {
        self.records.iter().filter(|r| r.extinct_years.is_none())
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
            + self.records.iter().map(|r| r.founder.memory_bytes()).sum::<usize>()
    }
}
