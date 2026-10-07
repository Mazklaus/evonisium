//! Planète et environnement, version de l'étape 1 : une grille fixe.
//!
//! À cette étape la planète ne bouge pas (ni tectonique, ni climat
//! transitoire) : elle fournit une grille géodésique, un relief, un climat
//! moyen par cellule et la chimie de la couche d'eau de surface, que le vivant
//! consomme et modifie. Les modules dynamiques arrivent à l'étape 2.

pub mod environment;
pub mod generate;
pub mod grid;
pub mod params;
pub mod pools;

pub use environment::{CellEnvironment, Planet, WaterChemistry};
pub use grid::GeodesicGrid;
pub use params::PlanetParams;
pub use pools::{Pool, WaterPool, WATER_POOLS, WATER_POOL_COUNT};
