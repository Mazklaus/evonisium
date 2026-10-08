//! Génétique et évolution.
//!
//! - [`genome`] : génome d'éléments génétiques dont chaque gène porte un
//!   domaine protéique paramétré (version réduite du génome à domaines du
//!   document Génétique : pas encore de réseau de régulation), et l'interface
//!   unique de modification qui note la cause de chaque changement.
//! - [`mutation`] : les classes de mutations utiles aux unicellulaires, dont
//!   la duplication suivie de divergence vers une famille apparentée, et le
//!   transfert horizontal de gènes.
//! - [`popgen`] : génétique des populations (Kimura, Wright-Fisher,
//!   Hardy-Weinberg), régime « apparition puis fixation » et tunnel
//!   stochastique.
//! - [`journal`] : journal des modifications de génome fixées.

pub mod genome;
pub mod journal;
pub mod lineage;
pub mod mutation;
pub mod popgen;

pub use genome::{
    ChangedElement, Domain, DomainFamily, DomainRelation, Gene, Genome, GenomeChange, GenomeChangeCause, Organelle, ReactionId,
    GENOME_CHANGE_CAUSE_COUNT, REGULATOR_SENSE_NM,
};
pub use journal::{GenomeJournal, JournalEntry};
pub use lineage::{LineageRecord, LineageRegistry};
pub use mutation::{
    mutate, mutate_again, mutate_with_kind, transfer_gene, MutationKind, MutationParams, MUTATION_KINDS, MUTATION_KIND_COUNT,
};
pub use popgen::{fixation_probability, tunnel_probability, OriginFixation};
