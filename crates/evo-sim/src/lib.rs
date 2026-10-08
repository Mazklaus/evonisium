//! Monde simulé sans affichage (étapes 1 et 2) : planète vivante, monde
//! microbien, file d'ordres, historique, état publié, porte de l'étape 2.

pub mod bench;
pub mod evolution;
pub mod gate;
pub mod history;
pub mod orders;
pub mod report;
pub mod world;

pub use evolution::{AcceleratorParams, EvolutionParams};
pub use history::{CellView, ClimateMode, History, PublishedState, Sample};
pub use orders::{Order, OrderKind, OrderQueue};
pub use world::{Seeding, Summary, World, WorldConfig};
