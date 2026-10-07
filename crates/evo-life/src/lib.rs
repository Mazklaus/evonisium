//! Organismes et écosystèmes : le monde microbien.
//!
//! - [`metabolism`] : catalogue des voies métaboliques, parentés des domaines
//!   et chemin vers la photosynthèse.
//! - [`phenotype`] : phénotype dérivé du génome (calculé une fois, mis en cache).
//! - [`growth`] : taux de croissance r(g, c) et coefficient de sélection s,
//!   selon les formules du document Organismes.
//! - [`community`] : populations par cellule (guilde métabolique × cellule)
//!   et leur dynamique couplée à la chimie de l'eau.
//! - [`spectrum`] : lumière disponible selon la longueur d'onde, sous l'étoile
//!   de la partie, et couleur des pigments.

pub mod community;
pub mod growth;
pub mod metabolism;
pub mod phenotype;
pub mod spectrum;

pub use community::{CellContext, Population};
pub use growth::{growth_rates, selection_coefficient, Conditions, GrowthRates, Physiology};
pub use phenotype::Phenotype;
pub use spectrum::{pigment_colour, LightSpectrum};
