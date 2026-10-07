//! Tests de validation scientifique de l'étape 1 (porte de l'étape) :
//! la simulation est comparée à des résultats théoriques connus.
//!
//! Chaque test utilise une graine fixe : il est déterministe. Les seuils sont
//! fixés à environ 4 écarts types, ce qui laisse une marge confortable sans
//! masquer une erreur de modèle.

use evo_core::rng::{rng_for, Stream};
use evo_genetics::popgen::{
    fixation_probability, hardy_weinberg_chi2, new_mutant_fixation, wright_fisher_fixes, DiploidPopulation, OriginFixation,
};

/// Fraction de répliques où un mutant unique se fixe, et son écart type.
fn simulated_fixation(n: u64, s: f64, replicates: u32, seed: u64) -> (f64, f64) {
    let mut rng = rng_for(seed, Stream::Validation, &[n, s.to_bits()]);
    let fixed = (0..replicates).filter(|_| wright_fisher_fixes(n, s, &mut rng)).count();
    let p = fixed as f64 / replicates as f64;
    (p, (p * (1.0 - p) / replicates as f64).sqrt())
}

fn assert_matches_kimura(n: u64, s: f64, replicates: u32) {
    let theory = new_mutant_fixation(s, n as f64);
    let (sim, se) = simulated_fixation(n, s, replicates, 2026);
    let se_theory = (theory * (1.0 - theory) / replicates as f64).sqrt();
    let tol = 4.0 * se.max(se_theory);
    println!("N = {n}, s = {s}: Kimura {theory:.5}, Wright-Fisher {sim:.5} ± {se:.5}");
    assert!((sim - theory).abs() < tol, "N = {n}, s = {s} : Kimura {theory}, simulé {sim} (tolérance {tol})");
}

#[test]
fn kimura_beneficial_mutation() {
    assert_matches_kimura(100, 0.02, 40_000);
    assert_matches_kimura(500, 0.01, 40_000);
}

#[test]
fn kimura_neutral_mutation() {
    assert_matches_kimura(100, 0.0, 40_000);
}

#[test]
fn kimura_deleterious_mutation() {
    assert_matches_kimura(100, -0.005, 40_000);
}

#[test]
fn kimura_strong_selection_in_small_population() {
    assert_matches_kimura(50, 0.1, 20_000);
}

#[test]
fn kimura_from_intermediate_frequency() {
    // Formule générale u(p₀) pour un allèle déjà présent à 20 % : on simule
    // directement depuis cette fréquence.
    let (n, s, p0) = (100u64, 0.01, 0.2);
    let theory = fixation_probability(s, n as f64, p0);
    let mut rng = rng_for(5, Stream::Validation, &[]);
    let reps = 20_000;
    let mut fixed = 0;
    for _ in 0..reps {
        let mut k = (p0 * n as f64) as u64;
        while k > 0 && k < n {
            let p = k as f64 / n as f64;
            let p_sel = p * (1.0 + s) / (1.0 + p * s);
            k = rand_distr::Distribution::sample(&rand_distr::Binomial::new(n, p_sel).unwrap(), &mut rng);
        }
        fixed += (k == n) as u32;
    }
    let sim = fixed as f64 / reps as f64;
    let se = (theory * (1.0 - theory) / reps as f64).sqrt();
    assert!((sim - theory).abs() < 4.0 * se, "théorie {theory}, simulé {sim}");
}

#[test]
fn hardy_weinberg_reached_in_one_generation() {
    // Départ très loin de l'équilibre : uniquement des homozygotes.
    let n = 100_000;
    let start = DiploidPopulation { individuals: (0..n).map(|i| if i % 10 < 3 { [1, 1] } else { [0, 0] }).collect() };
    assert_eq!(start.genotype_counts()[1], 0);
    let p = start.allele_frequency();
    let mut rng = rng_for(11, Stream::Validation, &[]);
    let next = start.next_generation(n, &mut rng);
    let counts = next.genotype_counts();
    let chi2 = hardy_weinberg_chi2(counts);
    let het = counts[1] as f64 / n as f64;
    println!("p = {p}, hétérozygotes {het:.4} (attendu {:.4}), χ² = {chi2:.3}", 2.0 * p * (1.0 - p));
    // Seuil du χ² à 1 degré de liberté pour un risque de 0,1 %.
    assert!(chi2 < 10.83, "χ² = {chi2}");
    assert!((het - 2.0 * p * (1.0 - p)).abs() < 0.01);
}

#[test]
fn hardy_weinberg_holds_over_generations_and_allele_frequency_drifts_slowly() {
    let n = 20_000;
    let mut pop = DiploidPopulation { individuals: (0..n).map(|i| [(i % 2) as u8, (i % 5 == 0) as u8]).collect() };
    let p0 = pop.allele_frequency();
    let mut rng = rng_for(12, Stream::Validation, &[]);
    for _ in 0..10 {
        pop = pop.next_generation(n, &mut rng);
        assert!(hardy_weinberg_chi2(pop.genotype_counts()) < 10.83);
    }
    // Dérive attendue après t générations : variance ≈ p(1−p)·t / 2N.
    let sd = (p0 * (1.0 - p0) * 10.0 / (2.0 * n as f64)).sqrt();
    assert!((pop.allele_frequency() - p0).abs() < 4.0 * sd);
}

#[test]
fn origin_fixation_neutral_substitution_rate_equals_mutation_rate() {
    // Théorie neutre de Kimura : le taux de substitution vaut le taux de
    // mutation, quelle que soit la taille de la population.
    let regime = OriginFixation { candidates: 8, ne_cap: 1e12 };
    let mut rng = rng_for(13, Stream::Validation, &[]);
    for &(ne, u) in &[(1_000.0, 1e-4), (1e6, 1e-7)] {
        // Pas courts : au plus une substitution par pas est retenue, il faut
        // donc que deux substitutions dans le même pas restent rares.
        let generations = 100.0;
        let steps = 500_000;
        let copies = regime.mutants_per_candidate(ne, u, generations);
        let mut substitutions = 0u32;
        for _ in 0..steps {
            let any = (0..regime.candidates).any(|_| regime.candidate_fixes(0.0, ne, copies, &mut rng));
            substitutions += any as u32;
        }
        let rate = substitutions as f64 / (steps as f64 * generations);
        let expected_count = u * generations * steps as f64;
        let tol = 4.0 * expected_count.sqrt() / (steps as f64 * generations);
        println!("Nₑ = {ne}, u = {u}: taux de substitution {rate:.3e}");
        assert!((rate - u).abs() < tol, "Nₑ = {ne} : taux {rate}, attendu {u}");
    }
}

#[test]
fn origin_fixation_matches_explicit_wright_fisher_with_rare_mutations() {
    // Population de 50, mutations rares (N·u = 0,02) toutes avantageuses de
    // s = 0,05 : on compare le nombre de substitutions du régime à celui d'une
    // simulation explicite où chaque mutant est suivi jusqu'à sa fixation ou sa
    // perte.
    let (n, u, s) = (50u64, 4e-4, 0.05);
    let generations = 400_000u64;
    let mut rng = rng_for(14, Stream::Validation, &[]);
    let mut explicit = 0u32;
    for _ in 0..generations {
        let mutants = rand_distr::Distribution::sample(&rand_distr::Poisson::new(n as f64 * u).unwrap(), &mut rng) as u32;
        for _ in 0..mutants {
            explicit += wright_fisher_fixes(n, s, &mut rng) as u32;
        }
    }
    let regime = OriginFixation { candidates: 1, ne_cap: 1e12 };
    let block = 10.0;
    let copies = regime.mutants_per_candidate(n as f64, u, block);
    let mut origin_fixation = 0u32;
    for _ in 0..(generations as f64 / block) as u32 {
        origin_fixation += regime.candidate_fixes(s, n as f64, copies, &mut rng) as u32;
    }
    let expected = n as f64 * u * new_mutant_fixation(s, n as f64) * generations as f64;
    println!("substitutions attendues {expected:.1}, explicites {explicit}, régime {origin_fixation}");
    let tol = 4.0 * expected.sqrt();
    assert!((explicit as f64 - expected).abs() < tol);
    assert!((origin_fixation as f64 - expected).abs() < tol);
}
