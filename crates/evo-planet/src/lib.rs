//! Planète et environnement : une planète vivante.
//!
//! - [`params`] : paramètres de planète et préréglages (Terre archéenne et
//!   mondes exotiques de la première vague).
//! - [`grid`] : grille géodésique commune.
//! - [`tectonics`] : plaques, dorsales, subduction, collisions, relief.
//! - [`climate`] : climat à l'équilibre, effet de serre, glace, ozone.
//! - [`geochem`] : atmosphère, océan profond, sédiments, cycles du carbone,
//!   de l'oxygène, du fer, du manganèse et du phosphore.
//! - [`environment`] : vecteur d'environnement par cellule et échanges de la
//!   couche d'eau de surface avec les réservoirs.
//! - [`pools`] : liste unique des pools chimiques.

pub mod climate;
pub mod environment;
pub mod generate;
pub mod geochem;
pub mod grid;
pub mod params;
pub mod pools;
pub mod tectonics;

pub use environment::{CellChange, CellEnvironment, ExchangeTargets, Planet, WaterChemistry, TECTONIC_STEP_YEARS};
pub use geochem::{Gas, GlobalReservoirs, GASES, GAS_COUNT};
pub use grid::GeodesicGrid;
pub use params::PlanetParams;
pub use pools::{Pool, WaterPool, WATER_POOLS, WATER_POOL_COUNT};
