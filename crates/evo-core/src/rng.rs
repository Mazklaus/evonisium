//! Hasard à graine, un flux indépendant par système.
//!
//! Chaque tirage est dérivé de la graine de la partie, du système qui tire et
//! d'identifiants stables (pas de temps, cellule, population). Le résultat ne
//! dépend donc ni de l'ordre d'exécution des fils ni du nombre de coeurs :
//! la même graine redonne exactement la même histoire.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Générateur utilisé partout dans la simulation.
pub type SimRng = ChaCha8Rng;

/// Systèmes qui consomment du hasard. Ajouter une variante ne change pas les
/// tirages des autres systèmes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stream {
    PlanetGeneration = 1,
    Seeding = 2,
    Mutation = 3,
    Migration = 4,
    Validation = 5,
    Benchmark = 6,
}

/// Mélangeur SplitMix64 : diffuse chaque bit d'entrée sur toute la sortie.
#[inline]
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Graine dérivée de la graine maître, du flux et d'identifiants stables.
pub fn derive_seed(master: u64, stream: Stream, parts: &[u64]) -> u64 {
    let mut h = splitmix64(master ^ splitmix64(stream as u64));
    for &p in parts {
        h = splitmix64(h ^ p);
    }
    h
}

/// Générateur prêt à l'emploi pour un système et des identifiants donnés.
pub fn rng_for(master: u64, stream: Stream, parts: &[u64]) -> SimRng {
    SimRng::seed_from_u64(derive_seed(master, stream, parts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn same_inputs_same_draws() {
        let a: Vec<u64> = (0..8).map(|_| 0).scan(rng_for(42, Stream::Mutation, &[1, 2]), |r, _| Some(r.random())).collect();
        let b: Vec<u64> = (0..8).map(|_| 0).scan(rng_for(42, Stream::Mutation, &[1, 2]), |r, _| Some(r.random())).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn streams_and_parts_are_independent() {
        let base = derive_seed(42, Stream::Mutation, &[1, 2]);
        assert_ne!(base, derive_seed(42, Stream::Migration, &[1, 2]));
        assert_ne!(base, derive_seed(42, Stream::Mutation, &[2, 1]));
        assert_ne!(base, derive_seed(43, Stream::Mutation, &[1, 2]));
    }
}
