//! Journal des modifications de génome (document Génétique, « Une seule porte
//! d'entrée pour modifier un génome ») : chaque modification fixée note sa
//! cause, la lignée touchée, la date, la cellule et l'élément modifié. Il sert
//! à l'arbre du vivant (« ce gène vient d'un transfert »), à la mesure de
//! l'effet des accélérateurs et, plus tard, aux sciences des peuples. Le
//! joueur n'y a pas accès.
//!
//! [Simplification] Chez les microbes, seules les modifications qui se fixent
//! dans une population sont journalisées ; les mutants seulement évalués ne le
//! sont pas. Au-delà d'un plafond d'entrées, le journal ne garde que les
//! compteurs par cause, pour borner la mémoire des très longues parties.

use crate::genome::{ChangedElement, GenomeChangeCause, GENOME_CHANGE_CAUSE_COUNT};
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JournalEntry {
    pub years: f64,
    pub lineage: u32,
    pub cell: u32,
    pub cause: GenomeChangeCause,
    pub element: ChangedElement,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenomeJournal {
    pub entries: Vec<JournalEntry>,
    /// Modifications fixées par cause, plafond ou non.
    pub by_cause: [u64; GENOME_CHANGE_CAUSE_COUNT],
    /// Nombre maximal d'entrées détaillées conservées.
    pub max_entries: usize,
}

impl Default for GenomeJournal {
    fn default() -> Self {
        Self { entries: Vec::new(), by_cause: [0; GENOME_CHANGE_CAUSE_COUNT], max_entries: 5_000_000 }
    }
}

impl GenomeJournal {
    pub fn record(&mut self, entry: JournalEntry) {
        self.by_cause[entry.cause.index()] += 1;
        if self.entries.len() < self.max_entries {
            self.entries.push(entry);
        }
    }

    /// Entrées d'une lignée, dans l'ordre chronologique.
    pub fn of_lineage(&self, lineage: u32) -> impl Iterator<Item = &JournalEntry> {
        self.entries.iter().filter(move |e| e.lineage == lineage)
    }

    pub fn total(&self) -> u64 {
        self.by_cause.iter().sum()
    }

    pub fn memory_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<JournalEntry>()
    }

    /// Export tabulé : date, lignée, cellule, cause, élément.
    pub fn to_tsv(&self) -> String {
        let mut out = String::from("annees\tlignee\tcellule\tcause\telement\n");
        for e in &self.entries {
            let detail = match e.cause {
                GenomeChangeCause::SpontaneousMutation(k) | GenomeChangeCause::InducedMutation(k) => format!(" ({k:?})"),
                _ => String::new(),
            };
            let _ = writeln!(out, "{:.0}\t{}\t{}\t{}{}\t{:?}", e.years, e.lineage, e.cell, e.cause.label(), detail, e.element);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::DomainFamily;

    #[test]
    fn cap_keeps_counters() {
        let mut j = GenomeJournal { max_entries: 2, ..Default::default() };
        for i in 0..5 {
            j.record(JournalEntry {
                years: i as f64,
                lineage: 1,
                cell: 0,
                cause: GenomeChangeCause::HorizontalTransfer,
                element: ChangedElement::Inserted { index: 0, family: DomainFamily::Pigment },
            });
        }
        assert_eq!(j.entries.len(), 2);
        assert_eq!(j.total(), 5);
        assert_eq!(j.of_lineage(1).count(), 2);
        assert!(j.to_tsv().contains("transfert horizontal"));
    }
}
