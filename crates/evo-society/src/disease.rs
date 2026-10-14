//! Épidémies agrégées : les maladies de foule.
//!
//! Une maladie de foule naît là où des gens nombreux et serrés vivent près
//! de leur bétail (rougeole, variole, grippe viennent d'animaux
//! domestiques ; McNeill, 1976). Elle ne reste endémique qu'au-dessus d'une
//! population connectée critique (environ 300 000 pour la rougeole,
//! Bartlett, 1957) ; en dessous, elle s'éteint et revient par vagues.
//! Introduite chez des gens qui ne l'ont jamais connue, elle fait une vague
//! dont la taille finale suit l'équation de Kermack et McKendrick
//! (1927) : z = 1 − exp(−R₀ z).
//!
//! [Simplification] Le moteur porte les maladies attachées aux populations
//! (décision du 7 octobre 2026), à petit génome évolutif. Ici, sur banc
//! d'essai, chaque maladie garde un R₀ et une létalité fixes ; l'intégration
//! de l'étape 7 branchera ces vagues sur les maladies du moteur.

use evo_core::math::Det;
use serde::{Deserialize, Serialize};

pub const MAX_DISEASES: usize = 24;

/// Population connectée sous laquelle une maladie à immunité durable
/// s'éteint.
pub const CRITICAL_COMMUNITY: f64 = 300_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Disease {
    pub id: u32,
    pub name: String,
    pub r0: f64,
    pub lethality: f64,
    /// Immunité durable après guérison (rougeole), ou non (peste).
    pub lasting: bool,
    pub origin_year: i64,
    pub origin_cell: u32,
    pub species: u32,
}

/// Taille finale d'une vague : part des sensibles touchés.
pub fn final_size(r0: f64) -> f64 {
    if r0 <= 1.0 {
        return 0.0;
    }
    // Point fixe, convergence rapide.
    let mut z: f64 = 0.8;
    for _ in 0..40 {
        z = 1.0 - (-r0 * z).dexp();
    }
    z.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kermack_mckendrick() {
        // R₀ = 2 : environ 80 % des sensibles touchés.
        assert!((final_size(2.0) - 0.797).abs() < 0.01);
        assert_eq!(final_size(0.9), 0.0);
        assert!(final_size(12.0) > 0.99);
    }
}
