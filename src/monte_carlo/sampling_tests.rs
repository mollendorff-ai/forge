//! Tests that the engine honours `seed`, `sampling` and `correlations`

use super::config::{CorrelationConfig, MonteCarloConfig};
use super::distributions::Distribution;
use super::engine::MonteCarloEngine;
use std::collections::HashMap;

fn config(sampling: &str, seed: u64, iterations: usize) -> MonteCarloConfig {
    MonteCarloConfig {
        enabled: true,
        iterations,
        sampling: sampling.to_string(),
        seed: Some(seed),
        ..MonteCarloConfig::default()
    }
}

fn samples(
    config: MonteCarloConfig,
    dists: Vec<(&str, Distribution)>,
) -> HashMap<String, Vec<f64>> {
    let mut engine = MonteCarloEngine::new(config).unwrap();
    for (name, dist) in dists {
        engine.add_distribution(name, dist);
    }
    engine.run().unwrap().input_samples
}

fn five_variables() -> Vec<(&'static str, Distribution)> {
    vec![
        ("a.revenue", Distribution::normal(100.0, 10.0).unwrap()),
        (
            "a.costs",
            Distribution::triangular(50.0, 60.0, 90.0).unwrap(),
        ),
        ("b.growth", Distribution::uniform(-0.1, 0.3).unwrap()),
        ("b.duration", Distribution::pert(3.0, 5.0, 12.0).unwrap()),
        ("c.price", Distribution::lognormal(20.0, 4.0).unwrap()),
    ]
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;
    let (mut cov, mut var_x, mut var_y) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (a, b) in x.iter().zip(y) {
        cov = (a - mean_x).mul_add(b - mean_y, cov);
        var_x = (a - mean_x).mul_add(a - mean_x, var_x);
        var_y = (b - mean_y).mul_add(b - mean_y, var_y);
    }
    cov / (var_x * var_y).sqrt()
}

#[test]
fn test_same_seed_gives_same_samples() {
    // Each HashMap gets its own random iteration order, so two engines built
    // the same way used to give different samples for the same seed
    for sampling in ["monte_carlo", "latin_hypercube"] {
        let first = samples(config(sampling, 42, 200), five_variables());
        for _ in 0..10 {
            assert_eq!(
                first,
                samples(config(sampling, 42, 200), five_variables()),
                "{sampling}"
            );
        }
    }
}

#[test]
fn test_different_seeds_give_different_samples() {
    let a = samples(config("monte_carlo", 1, 50), five_variables());
    let b = samples(config("monte_carlo", 2, 50), five_variables());
    assert_ne!(a, b);
}

#[test]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn test_latin_hypercube_puts_one_sample_in_each_stratum() {
    let n = 100;
    let result = samples(
        config("latin_hypercube", 7, n),
        vec![("u", Distribution::uniform(0.0, 1.0).unwrap())],
    );
    let mut counts = vec![0; n];
    for value in &result["u"] {
        counts[(value * n as f64) as usize] += 1;
    }
    assert!(counts.iter().all(|&count| count == 1), "{counts:?}");
}

#[test]
fn test_correlations_are_applied() {
    for coefficient in [0.8, -0.5] {
        let mut config = config("monte_carlo", 11, 20_000);
        config.correlations = vec![CorrelationConfig {
            variables: vec!["revenue".to_string(), "x.costs".to_string()],
            coefficient,
        }];
        let result = samples(
            config,
            vec![
                ("x.revenue", Distribution::normal(100.0, 10.0).unwrap()),
                ("x.costs", Distribution::normal(50.0, 5.0).unwrap()),
                ("x.other", Distribution::normal(0.0, 1.0).unwrap()),
            ],
        );
        let r = pearson(&result["x.revenue"], &result["x.costs"]);
        assert!(
            (r - coefficient).abs() < 0.02,
            "wanted {coefficient}, got {r}"
        );
        // An uncorrelated variable stays uncorrelated
        assert!(pearson(&result["x.revenue"], &result["x.other"]).abs() < 0.03);
        // Each variable keeps its own distribution
        let mean = result["x.costs"].iter().sum::<f64>() / 20_000.0;
        assert!((mean - 50.0).abs() < 0.2, "{mean}");
    }
}

#[test]
fn test_bad_correlation_variable_is_an_error() {
    for variables in [["revenue", "missing"], ["revenue", "x.revenue"]] {
        let mut config = config("monte_carlo", 1, 10);
        config.correlations = vec![CorrelationConfig {
            variables: variables.iter().map(ToString::to_string).collect(),
            coefficient: 0.5,
        }];
        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("x.revenue", Distribution::normal(1.0, 1.0).unwrap());
        assert!(engine.run().is_err(), "{variables:?}");
    }
}

#[test]
fn test_inverse_cdf_matches_reference_values() {
    // Normal, PERT and lognormal references from R (qnorm, qbeta, qlnorm);
    // triangular in closed form: sqrt(6) and 10 - sqrt(14)
    let cases = [
        (
            Distribution::normal(100.0, 15.0).unwrap(),
            0.975,
            129.399_459_768_101,
        ),
        (
            Distribution::pert(10.0, 20.0, 40.0).unwrap(),
            0.3,
            18.200_465_492_438,
        ),
        (
            Distribution::lognormal(100.0, 20.0).unwrap(),
            0.9,
            126.388_578_975_423,
        ),
        (
            Distribution::triangular(0.0, 3.0, 10.0).unwrap(),
            0.2,
            6.0_f64.sqrt(),
        ),
        (
            Distribution::triangular(0.0, 3.0, 10.0).unwrap(),
            0.8,
            10.0 - 14.0_f64.sqrt(),
        ),
        (Distribution::uniform(-1.0, 3.0).unwrap(), 0.25, 0.0),
    ];
    for (dist, p, expected) in cases {
        let value = dist.inverse_cdf(p);
        assert!(
            (value - expected).abs() < 1e-6,
            "{} at {p}: {value}",
            dist.dist_type
        );
    }
    let discrete = Distribution::discrete(vec![1.0, 2.0, 3.0], vec![0.2, 0.5, 0.3]).unwrap();
    assert!((discrete.inverse_cdf(0.1) - 1.0).abs() < f64::EPSILON);
    assert!((discrete.inverse_cdf(0.6) - 2.0).abs() < f64::EPSILON);
    assert!((discrete.inverse_cdf(0.95) - 3.0).abs() < f64::EPSILON);
}
