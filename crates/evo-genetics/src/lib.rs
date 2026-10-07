//! Génétique et évolution, version de l'étape 1.
//!
//! - [`genome`] : génome d'éléments génétiques dont chaque gène porte un
//!   domaine protéique paramétré (version réduite du génome à domaines du
//!   document Génétique : pas encore de réseau de régulation).
//! - [`mutation`] : les classes de mutations utiles aux unicellulaires.
//! - [`popgen`] : génétique des populations (Kimura, Wright-Fisher,
//!   Hardy-Weinberg) et régime « apparition puis fixation ».

pub mod genome;
pub mod lineage;
pub mod mutation;
pub mod popgen;

pub use genome::{Domain, DomainFamily, Gene, Genome, GenomeChange, GenomeChangeCause, ReactionId};
pub use lineage::{LineageRecord, LineageRegistry};
pub use mutation::{mutate, mutate_with_kind, MutationKind, MutationParams, MUTATION_KINDS};
pub use popgen::{fixation_probability, OriginFixation};
