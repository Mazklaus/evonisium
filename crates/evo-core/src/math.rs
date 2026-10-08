//! Mathématiques déterministes.
//!
//! Les fonctions transcendantes de la bibliothèque standard (`exp`, `ln`,
//! `sin`…) appellent la bibliothèque mathématique du système : leurs
//! derniers bits diffèrent entre Linux, Windows et macOS, et une partie
//! rejouée divergerait. Le moteur passe donc par `libm`, écrite en Rust et
//! identique partout. `sqrt` et les quatre opérations, exactement arrondies
//! par la norme IEEE 754, restent celles du processeur.
//!
//! L'usage des fonctions de la bibliothèque standard est refusé par
//! `clippy.toml` à la racine du dépôt.

use rand::Rng;

/// Fonctions transcendantes déterministes sur `f64`.
pub trait Det: Copy {
    fn dexp(self) -> Self;
    fn dexp_m1(self) -> Self;
    fn dln(self) -> Self;
    fn dln_1p(self) -> Self;
    fn dpowf(self, e: Self) -> Self;
    fn dsin(self) -> Self;
    fn dcos(self) -> Self;
    fn dsin_cos(self) -> (Self, Self);
    fn dasin(self) -> Self;
    fn dacos(self) -> Self;
    fn datan2(self, x: Self) -> Self;
}

impl Det for f64 {
    #[inline]
    fn dexp(self) -> f64 {
        libm::exp(self)
    }
    #[inline]
    fn dexp_m1(self) -> f64 {
        libm::expm1(self)
    }
    #[inline]
    fn dln(self) -> f64 {
        libm::log(self)
    }
    #[inline]
    fn dln_1p(self) -> f64 {
        libm::log1p(self)
    }
    #[inline]
    fn dpowf(self, e: f64) -> f64 {
        libm::pow(self, e)
    }
    #[inline]
    fn dsin(self) -> f64 {
        libm::sin(self)
    }
    #[inline]
    fn dcos(self) -> f64 {
        libm::cos(self)
    }
    #[inline]
    fn dsin_cos(self) -> (f64, f64) {
        libm::sincos(self)
    }
    #[inline]
    fn dasin(self) -> f64 {
        libm::asin(self)
    }
    #[inline]
    fn dacos(self) -> f64 {
        libm::acos(self)
    }
    #[inline]
    fn datan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
}

/// Tirage d'une loi normale centrée réduite (Box-Muller), déterministe sur
/// toutes les plateformes.
pub fn standard_normal(rng: &mut impl Rng) -> f64 {
    // u1 dans ]0, 1] : le logarithme reste fini.
    let u1 = 1.0 - rng.random::<f64>();
    let u2: f64 = rng.random();
    (-2.0 * u1.dln()).sqrt() * (std::f64::consts::TAU * u2).dcos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::disallowed_methods)]
    fn libm_agrees_with_std_to_a_few_ulps() {
        for i in 1..2000 {
            let x = i as f64 * 0.0137 - 9.0;
            let close = |a: f64, b: f64| (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs()).max(1e-300);
            assert!(close(x.dexp(), x.exp()));
            assert!(close(x.dsin(), x.sin()) || (x.dsin() - x.sin()).abs() < 1e-15);
            if x > 0.0 {
                assert!(close(x.dln(), x.ln()) || (x.dln() - x.ln()).abs() < 1e-15);
                assert!(close(x.dpowf(0.3), x.powf(0.3)));
            }
        }
    }

    #[test]
    fn normal_draws_have_unit_variance() {
        let mut rng = crate::rng::rng_for(1, crate::rng::Stream::Mutation, &[0]);
        let n = 20_000;
        let v: Vec<f64> = (0..n).map(|_| standard_normal(&mut rng)).collect();
        let mean = v.iter().sum::<f64>() / n as f64;
        let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.03 && (var - 1.0).abs() < 0.05, "{mean} {var}");
    }
}
