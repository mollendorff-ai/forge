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
                    "Correlation between {} and {} must be between -1 and 1, got {}",
                    var1, var2, rho
                ));
            }

            let i = *var_index
                .get(var1.as_str())
                .ok_or_else(|| format!("Variable {} not found", var1))?;
            let j = *var_index
                .get(var2.as_str())
                .ok_or_else(|| format!("Variable {} not found", var2))?;

            // Symmetric matrix
            coefficients[i * n + j] = *rho;
            coefficients[j * n + i] = *rho;
        }

        let mut matrix = CorrelationMatrix {
            variables,
            coefficients,
            cholesky_factor: None,
        };

        // Compute Cholesky decomposition
        matrix.compute_cholesky()?;

        Ok(matrix)
    }

    /// Create identity correlation matrix (no correlations)
    pub fn identity(variables: Vec<String>) -> Self {
        let n = variables.len();
        let mut coefficients = vec![0.0; n * n];
        for i in 0..n {
            coefficients[i * n + i] = 1.0;
        }

        // Identity matrix is its own Cholesky factor
        let cholesky_factor = Some(coefficients.clone());

        CorrelationMatrix {
            variables,
            coefficients,
            cholesky_factor,
        }
    }

    /// Get the dimension of the matrix
    pub fn dim(&self) -> usize {
        self.variables.len()
    }

    /// Compute Cholesky decomposition (L where Σ = L * L^T)
    fn compute_cholesky(&mut self) -> Result<(), String> {
        let n = self.dim();
        let mut l = vec![0.0; n * n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum = 0.0;

                if i == j {
                    // Diagonal elements
                    for k in 0..j {
                        sum += l[j * n + k] * l[j * n + k];
                    }
                    let diag = self.coefficients[j * n + j] - sum;
                    if diag <= 0.0 {
                        return Err(format!(
                            "Correlation matrix is not positive definite (element [{},{}])",
                            i, j
                        ));
                    }
                    l[j * n + j] = diag.sqrt();
                } else {
                    // Off-diagonal elements
                    for k in 0..j {
                        sum += l[i * n + k] * l[j * n + k];
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
                correlated[i] += l[i * n + j] * independent_samples[j];
            }
        }

        Ok(correlated)
    }

    /// Get correlation coefficient between two variables
    pub fn get_correlation(&self, var1: &str, var2: &str) -> Option<f64> {
        let n = self.dim();
        let i = self.variables.iter().position(|v| v == var1)?;
        let j = self.variables.iter().position(|v| v == var2)?;
        Some(self.coefficients[i * n + j])
    }

    /// Get variable index
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
                    reconstructed[i * n + j] += l[i * n + k] * l[j * n + k];
                }
            }
        }

        // Compare with original
        for (i, (r, c)) in reconstructed
            .iter()
            .zip(matrix.coefficients.iter())
            .enumerate()
        {
            assert!(
                (r - c).abs() < 1e-10,
                "Mismatch at index {}: {} vs {}",
                i,
                r,
                c
            );
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
}
