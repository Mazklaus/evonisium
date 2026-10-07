//! Génétique des populations.
//!
//! - [`fixation_probability`] : formule de Kimura (diffusion) pour une
//!   population haploïde de Wright-Fisher.
//! - [`wright_fisher_fixes`] : simulation explicite, génération par génération,
//!   qui sert de référence pour valider la formule.
//! - [`OriginFixation`] : régime « apparition puis fixation » utilisé pour les
//!   microbes, où l'on tire l'apparition des mutations puis leur fixation au
//!   lieu de simuler les générations.
//! - [`DiploidPopulation`] : individus diploïdes à un locus, reproduction
//!   sexuée par appariement au hasard, pour vérifier Hardy-Weinberg.

use rand::Rng;
use rand_distr::{Binomial, Distribution};

/// ln(eˣ − 1) sans dépassement pour les grands x.
#[inline]
fn ln_expm1(x: f64) -> f64 {
    if x > 30.0 {
        x + (-(-x).exp()).ln_1p()
    } else {
        x.exp_m1().ln()
    }
}

/// Probabilité de fixation d'un allèle de fréquence initiale `p0` et de
/// coefficient de sélection `s`, dans une population haploïde d'effectif
/// efficace `ne` (Kimura, 1962) :
///
/// u(p₀) = (1 − e^(−2·Nₑ·s·p₀)) / (1 − e^(−2·Nₑ·s))
pub fn fixation_probability(s: f64, ne: f64, p0: f64) -> f64 {
    let b = 2.0 * ne * s;
    let a = b * p0;
    if b.abs() < 1e-12 {
        return p0;
    }
    if b > 0.0 {
        (-a).exp_m1() / (-b).exp_m1()
    } else {
        // Allèle désavantageux : rapport de deux grands nombres, en logarithmes.
        (ln_expm1(-a) - ln_expm1(-b)).exp()
    }
}

/// Probabilité qu'un nouveau mutant unique se fixe (p₀ = 1/N, Nₑ = N).
pub fn new_mutant_fixation(s: f64, n: f64) -> f64 {
    fixation_probability(s, n, 1.0 / n)
}

/// Simule une population haploïde de Wright-Fisher de taille `n` à partir
/// d'une copie d'un allèle d'avantage `s`, jusqu'à sa perte ou sa fixation.
pub fn wright_fisher_fixes(n: u64, s: f64, rng: &mut impl Rng) -> bool {
    let mut k = 1u64;
    while k > 0 && k < n {
        let p = k as f64 / n as f64;
        let p_sel = p * (1.0 + s) / (1.0 + p * s);
        k = Binomial::new(n, p_sel).expect("loi binomiale").sample(rng);
    }
    k == n
}

/// Régime « apparition puis fixation ».
///
/// Pendant `generations` générations, une population d'effectif efficace Nₑ
/// produit en moyenne Nₑ·U·G mutants. On en évalue `candidates` tirés au
/// hasard, chacun représentant Nₑ·U·G / candidates mutants de même effet.
/// Chaque candidat se fixe avec la probabilité qu'au moins une de ses copies
/// se fixe. Si plusieurs réussissent, le plus avantageux l'emporte.
///
/// [Simplification] Une seule substitution par population et par pas :
/// l'interférence clonale entre mutations simultanées est ignorée.
#[derive(Clone, Debug, PartialEq)]
pub struct OriginFixation {
    pub candidates: usize,
    /// Plafond de l'effectif efficace (les grandes populations microbiennes
    /// ont un Nₑ bien inférieur à leur effectif réel).
    pub ne_cap: f64,
}

impl Default for OriginFixation {
    fn default() -> Self {
        Self { candidates: 8, ne_cap: 1e8 }
    }
}

impl OriginFixation {
    pub fn effective_size(&self, census: f64) -> f64 {
        census.min(self.ne_cap).max(1.0)
    }

    /// Nombre de mutants représentés par chaque candidat.
    pub fn mutants_per_candidate(&self, ne: f64, genomic_rate: f64, generations: f64) -> f64 {
        ne * genomic_rate * generations / self.candidates as f64
    }

    /// Probabilité qu'au moins un de `copies` mutants indépendants se fixe.
    pub fn any_fixes(p_fix: f64, copies: f64) -> f64 {
        if p_fix <= 0.0 || copies <= 0.0 {
            return 0.0;
        }
        if p_fix >= 1.0 {
            return 1.0;
        }
        -(copies * (-p_fix).ln_1p()).exp_m1()
    }

    /// Tire le sort d'un candidat d'avantage `s`.
    pub fn candidate_fixes(&self, s: f64, ne: f64, copies: f64, rng: &mut impl Rng) -> bool {
        let p = fixation_probability(s, ne, 1.0 / ne);
        rng.random::<f64>() < Self::any_fixes(p, copies)
    }
}

/// Population diploïde à un locus bi-allélique (allèles 0 et 1).
#[derive(Clone, Debug, PartialEq)]
pub struct DiploidPopulation {
    pub individuals: Vec<[u8; 2]>,
}

impl DiploidPopulation {
    /// Effectifs des génotypes [00, 01, 11].
    pub fn genotype_counts(&self) -> [u64; 3] {
        let mut c = [0u64; 3];
        for ind in &self.individuals {
            c[(ind[0] + ind[1]) as usize] += 1;
        }
        c
    }

    /// Fréquence de l'allèle 1.
    pub fn allele_frequency(&self) -> f64 {
        let ones: u64 = self.individuals.iter().map(|i| (i[0] + i[1]) as u64).sum();
        ones as f64 / (2 * self.individuals.len()) as f64
    }

    /// Une génération de reproduction sexuée : chaque descendant reçoit un
    /// gamète de deux parents tirés au hasard (méiose : un allèle sur deux).
    pub fn next_generation(&self, size: usize, rng: &mut impl Rng) -> Self {
        let n = self.individuals.len();
        let gamete = |rng: &mut dyn rand::RngCore| {
            let parent = self.individuals[rng.random_range(0..n)];
            parent[rng.random_range(0..2)]
        };
        let individuals = (0..size).map(|_| [gamete(rng), gamete(rng)]).collect();
        Self { individuals }
    }
}

/// Statistique du χ² d'écart à Hardy-Weinberg (1 degré de liberté).
pub fn hardy_weinberg_chi2(counts: [u64; 3]) -> f64 {
    let n = (counts[0] + counts[1] + counts[2]) as f64;
    let p = (2 * counts[2] + counts[1]) as f64 / (2.0 * n);
    let q = 1.0 - p;
    let expected = [q * q * n, 2.0 * p * q * n, p * p * n];
    counts.iter().zip(expected).map(|(&o, e)| (o as f64 - e).powi(2) / e).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimura_limits() {
        // Neutre : 1/N.
        assert!((new_mutant_fixation(0.0, 1000.0) - 1e-3).abs() < 1e-15);
        // Grande population, avantage s : environ 1 − e^(−2s) ≈ 2s.
        let p = new_mutant_fixation(0.01, 1e9);
        assert!((p - (1.0 - (-0.02f64).exp())).abs() < 1e-9);
        // Désavantage fort dans une grande population : pratiquement nul, sans NaN.
        let p = new_mutant_fixation(-0.01, 1e9);
        assert!(p.is_finite() && (0.0..1e-300).contains(&p));
        // Fréquence initiale 1 : fixation certaine.
        assert!((fixation_probability(-0.3, 50.0, 1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn any_fixes_matches_direct_product() {
        let p: f64 = 0.01;
        let direct = 1.0 - (1.0 - p).powi(30);
        assert!((OriginFixation::any_fixes(p, 30.0) - direct).abs() < 1e-12);
    }
}
