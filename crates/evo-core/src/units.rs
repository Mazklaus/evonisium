//! Conventions d'unités : SI partout, temps interne en années terrestres
//! depuis la formation de la planète (convention du modèle de données commun).
//!
//! Seules des constantes physiques universelles figurent ici ; tout ce qui
//! dépend de la planète vient des paramètres de planète.

/// Secondes dans une année terrestre (unité de l'horloge interne, pas une
/// propriété de la planète simulée).
pub const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

/// Constante de Stefan-Boltzmann, W·m⁻²·K⁻⁴.
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;

/// Constante gravitationnelle, m³·kg⁻¹·s⁻².
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;

/// Conversion d'une puissance en watts vers des kilojoules par an.
#[inline]
pub fn watts_to_kj_per_year(w: f64) -> f64 {
    w * SECONDS_PER_YEAR / 1000.0
}
