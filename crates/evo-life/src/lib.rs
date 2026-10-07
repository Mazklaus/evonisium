//! Organismes et écosystèmes, version de l'étape 1 : le monde microbien.
//!
//! - [`metabolism`] : catalogue des voies métaboliques.
//! - [`phenotype`] : phénotype dérivé du génome (calculé une fois, mis en cache).
//! - [`growth`] : taux de croissance r(g, c) et coefficient de sélection s,
//!   selon les formules du document Organismes.
//! - [`community`] : populations par cellule (guilde métabolique × cellule)
//!   et leur dynamique couplée à la chimie de l'eau.

pub mod community;
pub mod growth;
pub mod metabolism;
pub mod phenotype;

pub use community::{CellContext, Population};
pub use growth::{growth_rates, selection_coefficient, GrowthRates, Physiology};
pub use phenotype::Phenotype;
