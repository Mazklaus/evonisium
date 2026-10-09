//! Monde simulé sans affichage (étapes 1 et 2) : planète vivante, monde
//! microbien, file d'ordres, historique, état publié, porte de l'étape 2.

pub mod bench;
pub mod disturbance;
pub mod equivalence;
pub mod evolution;
pub mod gate;
pub mod history;
pub mod influence;
pub mod observation;
pub mod orders;
pub mod report;
pub mod save;
pub mod transitions;
pub mod world;

pub use evolution::{AcceleratorParams, EvolutionParams};
pub use history::{CellView, ClimateMode, EventView, History, PublishedState, Sample, SpeciesView};
pub use influence::{InfluenceParams, InfluenceReserve, InfluenceView};
pub use observation::{Focus, InterestZone};
pub use orders::{Intervention, Order, OrderKind, OrderQueue};
pub use world::{Seeding, Summary, World, WorldConfig};
