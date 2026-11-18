//! Correlation Matrix and Cholesky Decomposition
//!
//! Handles correlated random variable generation using Cholesky decomposition.
//! This allows Monte Carlo simulations to model dependencies between inputs.

use std::collections::HashMap;

/// Correlation matrix for correlated sampling
#[derive(Debug, Clone)]
pub struct CorrelationMatrix {
    /// Variable names in order
    pub variables: Vec<String>,
    /// Correlation coefficients (symmetric matrix stored as flat vector)
    pub coefficients: Vec<f64>,
    /// Lower triangular Cholesky factor
    cholesky_factor: Option<Vec<f64>>,
}

impl CorrelationMatrix {
    /// Create a new correlation matrix from variable pairs
    ///
    /// # Errors
    ///
    /// Returns an error if any correlation coefficient is outside [-1, 1]
    /// or the resulting matrix is not positive definite.
    pub fn new(correlations: &[(String, String, f64)]) -> Result<Self, String> {
        // Collect unique variables
        let mut variables: Vec<String> = correlations
            .iter()
            .flat_map(|(a, b, _)| vec![a.clone(), b.clone()])
            .collect();
        variables.sort();
        variables.dedup();

        let n = variables.len();
        let mut coefficients = vec![0.0; n * n];

        // Set diagonal to 1.0 (perfect self-correlation)
        for i in 0..n {
            coefficients[i * n + i] = 1.0;
        }

        // Build variable index map
        let var_index: HashMap<&str, usize> = variables
            .iter()
            .enumerate()
            .map(|(i, v)| (v.as_str(), i))
            .collect();

        // Fill in correlations
        for (var1, var2, rho) in correlations {
            // Validate correlation coefficient
            if *rho < -1.0 || *rho > 1.0 {
                return Err(format!(
                    "Correlation between {var1} and {var2} must be between -1 and 1, got {rho}"
                ));
            }

            let i = *var_index
                .get(var1.as_str())
                .ok_or_else(|| format!("Variable {var1} not found"))?;
            let j = *var_index
                .get(var2.as_str())
                .ok_or_else(|| format!("Variable {var2} not found"))?;

            // Symmetric matrix
            coefficients[i * n + j] = *rho;
            coefficients[j * n + i] = *rho;
        }

        let mut matrix = Self {
            variables,
            coefficients,
            cholesky_factor: None,
        };

        // Compute Cholesky decomposition
        matrix.compute_cholesky()?;

        Ok(matrix)
    }

    /// Create identity correlation matrix (no correlations)
    #[must_use]
    pub fn identity(variables: Vec<String>) -> Self {
        let n = variables.len();
        let mut coefficients = vec![0.0; n * n];
        for i in 0..n {
            coefficients[i * n + i] = 1.0;
        }

        // Identity matrix is its own Cholesky factor
        let cholesky_factor = Some(coefficients.clone());

        Self {
            variables,
            coefficients,
            cholesky_factor,
        }
    }

    /// Get the dimension of the matrix
    #[must_use]
    pub const fn dim(&self) -> usize {
        self.variables.len()
    }

    /// Compute Cholesky decomposition (L where Σ = L * L^T)
    fn compute_cholesky(&mut self) -> Result<(), String> {
        let n = self.dim();
        let mut l = vec![0.0_f64; n * n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum: f64 = 0.0;

                if i == j {
                    // Diagonal elements
                    for k in 0..j {
                        sum = l[j * n + k].mul_add(l[j * n + k], sum);
                    }
                    let diag = self.coefficients[j * n + j] - sum;
                    if diag <= 0.0 {
                        // The first input's diagonal is 1, so this is the second or later
                        return Err(format!(
                            "the correlations of {} and {} cannot all hold at once (the matrix \
                             is not positive definite at {}): loosen the coefficients that \
                             contradict each other, or use one input for two with a \
                             coefficient of 1 or -1",
                            self.variables[..i].join(", "),
                            self.variables[i],
                            self.variables[i]
                        ));
                    }
                    l[j * n + j] = diag.sqrt();
                } else {
                    // Off-diagonal elements
                    for k in 0..j {
                        sum = l[i * n + k].mul_add(l[j * n + k], sum);
                    }
                    l[i * n + j] = (self.coefficients[i * n + j] - sum) / l[j * n + j];
                }
            }
        }

        self.cholesky_factor = Some(l);
        Ok(())
    }

    /// Transform independent standard normal samples to correlated samples
    ///
    /// Takes a vector of independent N(0,1) samples and returns correlated samples
    /// using the formula: Y = L * Z where L is the Cholesky factor and Z is independent.
    ///
    /// # Errors
    ///
    /// Returns an error if the number of samples does not match the matrix dimension
    /// or the Cholesky factor has not been computed.
    pub fn correlate(&self, independent_samples: &[f64]) -> Result<Vec<f64>, String> {
        let n = self.dim();
        if independent_samples.len() != n {
            return Err(format!(
                "Expected {} samples, got {}",
                n,
                independent_samples.len()
            ));
        }

        let l = self
            .cholesky_factor
            .as_ref()
            .ok_or("Cholesky factor not computed")?;

        let mut correlated = vec![0.0; n];

        for i in 0..n {
            for j in 0..=i {
                correlated[i] = l[i * n + j].mul_add(independent_samples[j], correlated[i]);
            }
        }

        Ok(correlated)
    }

    /// Get correlation coefficient between two variables
    #[must_use]
    pub fn get_correlation(&self, var1: &str, var2: &str) -> Option<f64> {
        let n = self.dim();
        let i = self.variables.iter().position(|v| v == var1)?;
        let j = self.variables.iter().position(|v| v == var2)?;
        Some(self.coefficients[i * n + j])
    }

    /// Get variable index
    #[must_use]
    pub fn get_index(&self, var: &str) -> Option<usize> {
        self.variables.iter().position(|v| v == var)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let vars = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let matrix = CorrelationMatrix::identity(vars);

        assert_eq!(matrix.dim(), 3);
        assert_eq!(matrix.get_correlation("a", "a"), Some(1.0));
        assert_eq!(matrix.get_correlation("a", "b"), Some(0.0));
        assert_eq!(matrix.get_correlation("b", "c"), Some(0.0));
    }

    #[test]
    fn test_simple_correlation() {
        let correlations = vec![("a".to_string(), "b".to_string(), 0.5)];
        let matrix = CorrelationMatrix::new(&correlations).unwrap();

        assert_eq!(matrix.dim(), 2);
        assert_eq!(matrix.get_correlation("a", "b"), Some(0.5));
        assert_eq!(matrix.get_correlation("b", "a"), Some(0.5)); // Symmetric
        assert_eq!(matrix.get_correlation("a", "a"), Some(1.0));
    }

    #[test]
    fn test_invalid_correlation() {
        let correlations = vec![("a".to_string(), "b".to_string(), 1.5)]; // > 1.0
        let result = CorrelationMatrix::new(&correlations);
        assert!(result.is_err());
    }

    #[test]
    fn test_correlate_identity() {
        let vars = vec!["a".to_string(), "b".to_string()];
        let matrix = CorrelationMatrix::identity(vars);

        let samples = vec![1.0, 2.0];
        let correlated = matrix.correlate(&samples).unwrap();

        // Identity matrix: output equals input
        assert!((correlated[0] - 1.0).abs() < 1e-10);
        assert!((correlated[1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_correlate_positive() {
        let correlations = vec![("a".to_string(), "b".to_string(), 0.8)];
        let matrix = CorrelationMatrix::new(&correlations).unwrap();

        // With high positive correlation, when a is high, b should also be high
        let samples = vec![2.0, 0.0]; // a is 2σ above mean, b starts at 0
        let correlated = matrix.correlate(&samples).unwrap();

        // a should be unchanged (first variable)
        assert!((correlated[0] - 2.0).abs() < 1e-10);
        // b should be pulled up due to correlation
        assert!(correlated[1] > 0.0);
    }

    #[test]
    fn test_cholesky_decomposition_accuracy() {
        // Test that L * L^T = original correlation matrix
        let correlations = vec![
            ("a".to_string(), "b".to_string(), 0.7),
            ("a".to_string(), "c".to_string(), 0.3),
            ("b".to_string(), "c".to_string(), 0.5),
        ];
        let matrix = CorrelationMatrix::new(&correlations).unwrap();

        let l = matrix.cholesky_factor.as_ref().unwrap();
        let n = matrix.dim();

        // Compute L * L^T
        let mut reconstructed = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    reconstructed[i * n + j] =
                        l[i * n + k].mul_add(l[j * n + k], reconstructed[i * n + j]);
                }
            }
        }

        // Compare with original
        for (i, (r, c)) in reconstructed
            .iter()
            .zip(matrix.coefficients.iter())
            .enumerate()
        {
            assert!((r - c).abs() < 1e-10, "Mismatch at index {i}: {r} vs {c}");
        }
    }

    #[test]
    fn test_three_variable_correlation() {
        let correlations = vec![
            ("revenue".to_string(), "costs".to_string(), 0.6),
            ("revenue".to_string(), "growth".to_string(), 0.4),
            ("costs".to_string(), "growth".to_string(), 0.2),
        ];
        let matrix = CorrelationMatrix::new(&correlations).unwrap();

        assert_eq!(matrix.dim(), 3);
        assert_eq!(matrix.get_correlation("revenue", "costs"), Some(0.6));
        assert_eq!(matrix.get_correlation("revenue", "growth"), Some(0.4));
        assert_eq!(matrix.get_correlation("costs", "growth"), Some(0.2));
    }

    fn pair(a: &str, b: &str, rho: f64) -> (String, String, f64) {
        (a.to_string(), b.to_string(), rho)
    }

    /// Coefficients that no set of inputs can have together are refused,
    /// naming the inputs. Before, the message named a cell of the matrix
    /// ("element [2,2]"), which a model does not show.
    #[test]
    fn coefficients_that_cannot_hold_together_are_refused_naming_the_inputs() {
        // a moves with b and b with c, so a cannot move against c
        let message = CorrelationMatrix::new(&[
            pair("a", "b", 0.9),
            pair("b", "c", 0.9),
            pair("a", "c", -0.9),
        ])
        .unwrap_err();
        assert_eq!(
            message,
            "the correlations of a, b and c cannot all hold at once (the matrix is not positive \
             definite at c): loosen the coefficients that contradict each other, or use one \
             input for two with a coefficient of 1 or -1"
        );
        // Two inputs that move as one are one input
        let message = CorrelationMatrix::new(&[pair("a", "b", -1.0)]).unwrap_err();
        assert!(
            message.starts_with("the correlations of a and b cannot all hold at once"),
            "{message}"
        );
        // Four inputs: the first that cannot hold, and those before it
        let message = CorrelationMatrix::new(&[
            pair("a", "b", 0.5),
            pair("b", "c", 0.99),
            pair("a", "c", -0.9),
            pair("c", "d", 0.1),
        ])
        .unwrap_err();
        assert!(
            message.starts_with("the correlations of a, b and c cannot"),
            "{message}"
        );
        assert!(CorrelationMatrix::new(&[pair("a", "b", 0.999)]).is_ok());
    }

    mod vineyard_example {
        //! `examples/vineyard`: a season of Les Trois Pentes, whose harvest and
        //! price move against each other (a Gaussian copula, coefficient -0.7),
        //! with a frost (MC.Discrete) that moves neither.
        //!
        //! Written from the README's description, not the model's formulas,
        //! with numpy 2.5 and scipy 1.18:
        //! ```python
        //! HA, TPH, PF = [10, 8, 6], [7.5, 5.5, 8.0], [1.00, 1.35, 0.90]
        //! def season(y, price, frost):
        //!     tonnes = sum(h * t for h, t in zip(HA, TPH)) * y * (1 - frost)
        //!     revenue = sum(h * t * f for h, t, f in zip(HA, TPH, PF)) * y * (1 - frost) * price
        //!     return revenue - 24 * 7000 - 180 * tonnes - 45000 - 40000
        //! def pert(a, m, b):
        //!     return stats.beta(1 + 4*(m-a)/(b-a), 1 + 4*(b-m)/(b-a), loc=a, scale=b-a)
        //! def lognormal(mean, sd):
        //!     return stats.lognorm(np.sqrt(np.log1p(sd**2 / mean**2)),
        //!                          scale=mean / np.sqrt(1 + sd**2 / mean**2))
        //! YIELD, PRICE = pert(0.5, 1.0, 1.3), lognormal(2000, 500)
        //! season(1.0, 2000, 0.0)  # the plan: 72140
        //! # From Forge's own 10,000 draws (seed 7, Latin hypercube): every run's
        //! # profit within 2.3e-10 of Forge's; mean 42889.015893, P5 -58926.619114,
        //! # P50 39680.667474, P95 153625.617063, P(loss) 0.2555, the worst 5%
        //! # -79829.454058; rank correlation of the draws -0.6902
        //! # An independent simulation, 2,000,000 runs:
        //! rng = np.random.default_rng(2026)
        //! z = rng.standard_normal((N, 2)) @ np.linalg.cholesky([[1, rho], [rho, 1]]).T
        //! u = stats.norm.cdf(z); frost = np.where(rng.random(N) < 0.1, 0.3, 0.0)
        //! p = season(YIELD.ppf(u[:, 0]), PRICE.ppf(u[:, 1]), frost)
        //! # rho -0.7: mean 43081.24, P5 -58669.62, P(loss) 0.2585, worst 5% -79647.08
        //! # rho 0:    mean 51831.60, P5 -89137.82, P(loss) 0.3350, worst 5% -111583.51
        //! ```
        //! One standard error at 10,000 runs, from 400 such simulations: the
        //! mean 619 (rho -0.7) and 1,057 (rho 0), P(loss) 0.0043 and 0.0048, P5
        //! 1,157 and 1,401, the worst 5% 1,251 and 1,303, the draws' rank
        //! correlation 0.0057 and 0.01. The tests allow four.

        use super::super::super::sensitivity::spearman_correlation;
        use crate::cli::{simulate_core, SimulateOptions};
        use crate::core::ArrayCalculator;
        use crate::monte_carlo::engine::SimulationResult;
        use std::f64::consts::PI;
        use std::path::Path;

        const RISK: &str = "examples/vineyard/vineyard_risk.yaml";
        const INDEPENDENT: &str = "examples/vineyard/vineyard_independent.yaml";
        const PLAN: &str = "examples/vineyard/vineyard.yaml";

        fn run(file: &str, sampling: Option<&str>) -> SimulationResult {
            let options = SimulateOptions {
                sampling,
                ..SimulateOptions::default()
            };
            let result = simulate_core(Path::new(file), options).unwrap();
            assert_eq!(result.iterations_completed, 10_000);
            result
        }

        fn within(got: f64, want: f64, tolerance: f64, what: &str) {
            assert!((got - want).abs() <= tolerance, "{what}: {got}, not {want}");
        }

        /// Share of the draws below a value
        fn below(samples: &[f64], value: f64) -> f64 {
            let count = samples.iter().filter(|&&x| x < value).count();
            crate::core::precision::count(count) / crate::core::precision::count(samples.len())
        }

        #[test]
        fn the_draws_have_the_copulas_rank_correlation_and_keep_their_distributions() {
            // scipy's quantiles of the PERT and the lognormal
            let quantiles = [
                (0.05, 0.708_506_966_943_357_7, 1_294.129_018_057_045_6),
                (0.25, 0.860_006_261_189_086_2, 1_643.390_600_053_718_3),
                (0.50, 0.974_524_047_285_445_1, 1_940.285_000_290_663_7),
                (0.75, 1.080_877_015_746_446_5, 2_290.816_244_312_144_6),
                (0.95, 1.197_779_520_413_123, 2_909.065_348_063_303),
            ];
            for (file, sampling, rho, tolerance) in [
                (RISK, None, -0.7, 4.0 * 0.0057),
                (RISK, Some("monte_carlo"), -0.7, 4.0 * 0.0057),
                (INDEPENDENT, None, 0.0, 4.0 * 0.01),
            ] {
                let result = run(file, sampling);
                let draws = &result.input_samples;
                let crop = &draws["harvest.yield_factor"];
                let price = &draws["harvest.price_per_tonne"];
                // A Gaussian copula's rank correlation is (6 / pi) asin(rho / 2):
                // -0.6829 for -0.7
                let copula = 6.0 / PI * (rho / 2.0_f64).asin();
                let what = format!("{file} {sampling:?}");
                within(spearman_correlation(crop, price), copula, tolerance, &what);
                // Each input keeps its own distribution: at each of scipy's
                // quantiles, the share of draws below it is within the 5%
                // Kolmogorov-Smirnov bound for 10,000 draws, 1.358 / 100
                for (p, crop_q, price_q) in quantiles {
                    within(
                        below(crop, crop_q),
                        p,
                        0.013_58,
                        &format!("{what} yield {p}"),
                    );
                    within(
                        below(price, price_q),
                        p,
                        0.013_58,
                        &format!("{what} price {p}"),
                    );
                }
                // One spring in ten has a frost, and it kills 30% of the crop
                let frost = &draws["harvest.frost_loss"];
                assert!(frost.iter().all(|&x| x == 0.0 || (x - 0.3).abs() < 1e-12));
                within(
                    below(frost, 0.15),
                    0.9,
                    4.0 * 0.003,
                    &format!("{what} frost"),
                );
            }
        }

        #[test]
        fn the_season_is_what_python_gets_from_forges_own_draws() {
            let result = run(RISK, None);
            let profit = &result.outputs["season.profit"];
            within(profit.at_plan.unwrap(), 72_140.0, 1e-6, "at plan");
            let stats = &profit.statistics;
            within(stats.mean, 42_889.015_893, 1e-6, "mean");
            within(stats.percentiles[&5], -58_926.619_114, 1e-6, "P5");
            within(stats.median, 39_680.667_474, 1e-6, "P50");
            within(stats.percentiles[&95], 153_625.617_063, 1e-6, "P95");
            within(profit.threshold_probabilities["< 0"], 0.2555, 1e-12, "loss");
            within(
                profit.expected_shortfall[0].1,
                -79_829.454_058,
                1e-6,
                "worst 5%",
            );
            // The price drives the season most; a bigger crop barely helps,
            // since it sells for less
            assert_eq!(profit.drivers[0].input, "harvest.price_per_tonne");
            assert_eq!(profit.drivers[2].input, "harvest.yield_factor");
            assert!(profit.drivers[2].rank_correlation.abs() < 0.2);
            // Three of the seasons, calculated again by the plan with that
            // season's draws
            for run in [0, 4_999, 9_999] {
                let mut model = crate::parser::parse_model(Path::new(PLAN)).unwrap();
                for (input, samples) in &result.input_samples {
                    model
                        .override_scalar(input, Some(samples[run]), None)
                        .unwrap();
                }
                let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
                let value = calculated.scalars["season.profit"].value.unwrap();
                within(value, profit.samples[run], 1e-6, &format!("run {run}"));
            }
        }

        /// The correlation narrows the seasons and lowers their mean: a big
        /// crop never sells at a high price, so neither the best nor the worst
        /// seasons are as far out. Each figure is numpy's, within four
        /// standard errors.
        #[test]
        fn the_correlation_narrows_the_seasons_and_lowers_their_mean() {
            let figures = |result: &SimulationResult| {
                let profit = &result.outputs["season.profit"];
                (
                    profit.statistics.mean,
                    profit.statistics.percentiles[&5],
                    profit.threshold_probabilities["< 0"],
                    profit.expected_shortfall[0].1,
                    profit.statistics.std_dev,
                )
            };
            let numpy_correlated = (43_081.24, -58_669.62, 0.2585, -79_647.08);
            let correlated = [run(RISK, None), run(RISK, Some("monte_carlo"))];
            for result in &correlated {
                let (mean, p5, loss, worst, _) = figures(result);
                within(mean, numpy_correlated.0, 4.0 * 619.0, "mean");
                within(p5, numpy_correlated.1, 4.0 * 1_157.0, "P5");
                within(loss, numpy_correlated.2, 4.0 * 0.0043, "loss");
                within(worst, numpy_correlated.3, 4.0 * 1_251.0, "worst 5%");
            }
            let independent = figures(&run(INDEPENDENT, None));
            within(independent.0, 51_831.60, 4.0 * 1_057.0, "mean");
            within(independent.1, -89_137.82, 4.0 * 1_401.0, "P5");
            within(independent.2, 0.3350, 4.0 * 0.0048, "loss");
            within(independent.3, -111_583.51, 4.0 * 1_303.0, "worst 5%");

            let with = figures(&correlated[0]);
            assert!(with.0 < independent.0, "a lower mean");
            assert!(with.2 < independent.2, "fewer losses");
            assert!(with.3 > independent.3, "a milder worst 5%");
            assert!(with.4 < independent.4 * 0.7, "a third less spread");
        }

        #[test]
        fn the_independent_file_is_the_risk_file_with_the_correlation_at_zero() {
            let body = |file: &str| {
                let text = std::fs::read_to_string(file).unwrap();
                let start = text.find("_includes:").unwrap();
                text[start..].to_string()
            };
            let risk = body(RISK).replace(
                "    # A big regional harvest sells cheap, a short one dear\n    \
                 - variables: [yield_factor, price_per_tonne]\n      coefficient: -0.7",
                "    # As if the region's harvest did not move the price\n    \
                 - variables: [yield_factor, price_per_tonne]\n      coefficient: 0",
            );
            assert_eq!(risk, body(INDEPENDENT));
        }

        /// A frost that comes with big harvests and with high prices, which
        /// themselves move apart, is refused before anything is drawn.
        #[test]
        fn coefficients_that_contradict_each_other_are_refused() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::copy(
                "examples/vineyard/costs.yaml",
                dir.path().join("costs.yaml"),
            )
            .unwrap();
            let text = std::fs::read_to_string(RISK).unwrap().replace(
                "      coefficient: -0.7\n",
                "      coefficient: -0.7\n    \
                 - variables: [frost_loss, yield_factor]\n      coefficient: 0.9\n    \
                 - variables: [frost_loss, price_per_tonne]\n      coefficient: 0.9\n",
            );
            let file = dir.path().join("vineyard_risk.yaml");
            std::fs::write(&file, text).unwrap();
            let error = simulate_core(&file, SimulateOptions::default())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(
                    "the correlations of harvest.frost_loss, harvest.price_per_tonne and \
                     harvest.yield_factor cannot all hold at once"
                ),
                "{error}"
            );
        }
    }
}
