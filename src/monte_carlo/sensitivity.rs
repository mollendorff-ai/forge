//! Sensitivity Analysis for Monte Carlo Simulations
//!
//! Computes Spearman rank correlation coefficients between inputs and outputs
//! to identify which input variables have the greatest impact on results.

use std::collections::HashMap;

/// Sensitivity analysis results
#[derive(Debug, Clone)]
pub struct SensitivityAnalysis {
    /// Input variable name -> (output name -> Spearman correlation)
    pub correlations: HashMap<String, HashMap<String, f64>>,
    /// Sorted list of inputs by absolute impact on each output
    pub tornado_data: HashMap<String, Vec<TornadoBar>>,
}

/// Single bar in a tornado diagram
#[derive(Debug, Clone)]
pub struct TornadoBar {
    /// Input variable name
    pub variable: String,
    /// Spearman rank correlation coefficient (-1 to 1)
    pub correlation: f64,
    /// Absolute value of correlation (for sorting)
    pub impact: f64,
}

impl SensitivityAnalysis {
    /// Compute sensitivity analysis from input and output samples
    pub fn compute(
        input_samples: &HashMap<String, Vec<f64>>,
        output_samples: &HashMap<String, Vec<f64>>,
    ) -> Self {
        let mut correlations: HashMap<String, HashMap<String, f64>> = HashMap::new();
        let mut tornado_data: HashMap<String, Vec<TornadoBar>> = HashMap::new();

        for (output_name, output_values) in output_samples {
            let mut output_correlations: HashMap<String, f64> = HashMap::new();
            let mut bars: Vec<TornadoBar> = Vec::new();

            for (input_name, input_values) in input_samples {
                let rho = spearman_correlation(input_values, output_values);
                output_correlations.insert(input_name.clone(), rho);

                bars.push(TornadoBar {
                    variable: input_name.clone(),
                    correlation: rho,
                    impact: rho.abs(),
                });
            }

            // Sort bars by absolute impact (descending)
            bars.sort_by(|a, b| {
                b.impact
                    .partial_cmp(&a.impact)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for (input_name, rho) in output_correlations {
                correlations
                    .entry(input_name)
                    .or_default()
                    .insert(output_name.clone(), rho);
            }

            tornado_data.insert(output_name.clone(), bars);
        }

        SensitivityAnalysis {
            correlations,
            tornado_data,
        }
    }

    /// Get the top N most impactful inputs for a given output
    pub fn top_drivers(&self, output: &str, n: usize) -> Vec<&TornadoBar> {
        self.tornado_data
            .get(output)
            .map(|bars| bars.iter().take(n).collect())
            .unwrap_or_default()
    }

    /// Get correlation between an input and output
    pub fn get_correlation(&self, input: &str, output: &str) -> Option<f64> {
        self.correlations.get(input)?.get(output).copied()
    }

    /// Generate tornado diagram data in a format suitable for charting
    pub fn to_tornado_json(&self, output: &str) -> Option<String> {
        let bars = self.tornado_data.get(output)?;

        let json_bars: Vec<String> = bars
            .iter()
            .map(|bar| {
                format!(
                    r#"{{"variable":"{}","correlation":{:.4},"impact":{:.4}}}"#,
                    bar.variable, bar.correlation, bar.impact
                )
            })
            .collect();

        Some(format!(
            r#"{{"output":"{}","sensitivity":[{}]}}"#,
            output,
            json_bars.join(",")
        ))
    }
}

/// Compute Spearman rank correlation coefficient
pub fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.is_empty() {
        return 0.0;
    }

    let n = x.len();

    // Compute ranks
    let x_ranks = compute_ranks(x);
    let y_ranks = compute_ranks(y);

    // Compute Pearson correlation of ranks
    pearson_correlation(&x_ranks, &y_ranks)
}

/// Compute ranks with tie handling (average ranks for ties)
fn compute_ranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();

    // Create index-value pairs and sort
    let mut indexed: Vec<(usize, f64)> = values.iter().cloned().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; n];

    // Handle ties by averaging ranks
    let mut i = 0;
    while i < n {
        let mut j = i;
        // Find all elements with the same value
        while j < n && (indexed[j].1 - indexed[i].1).abs() < 1e-10 {
            j += 1;
        }

        // Average rank for tied elements
        let avg_rank = (i + j + 1) as f64 / 2.0; // Ranks are 1-based

        for item in indexed.iter().take(j).skip(i) {
            ranks[item.0] = avg_rank;
        }

        i = j;
    }

    ranks
}

/// Compute Pearson correlation coefficient
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n_samples = x.len() as f64;

    let mean_x: f64 = x.iter().sum::<f64>() / n_samples;
    let mean_y: f64 = y.iter().sum::<f64>() / n_samples;

    let mut num = 0.0;
    let mut denom_x = 0.0;
    let mut denom_y = 0.0;

    for (xi, yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        num += dx * dy;
        denom_x += dx * dx;
        denom_y += dy * dy;
    }

    let denom = (denom_x * denom_y).sqrt();

    if denom < 1e-10 {
        return 0.0;
    }

    num / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spearman_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        let rho = spearman_correlation(&x, &y);
        assert!((rho - 1.0).abs() < 1e-10, "Expected 1.0, got {}", rho);
    }

    #[test]
    fn test_spearman_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![50.0, 40.0, 30.0, 20.0, 10.0];

        let rho = spearman_correlation(&x, &y);
        assert!((rho + 1.0).abs() < 1e-10, "Expected -1.0, got {}", rho);
    }

    #[test]
    fn test_spearman_moderate_correlation() {
        // Partially correlated data
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 1.0, 4.0, 2.0, 5.0]; // Mixed order

        let rho = spearman_correlation(&x, &y);
        // This data has Spearman rho = 0.5 (moderate positive correlation)
        assert!((rho - 0.5).abs() < 0.1, "Expected ~0.5, got {}", rho);
    }

    #[test]
    fn test_spearman_with_ties() {
        let x = vec![1.0, 2.0, 2.0, 4.0, 5.0]; // Two 2.0s
        let y = vec![10.0, 20.0, 25.0, 40.0, 50.0];

        let rho = spearman_correlation(&x, &y);
        assert!(rho > 0.9, "Expected high positive correlation, got {}", rho);
    }

    #[test]
    fn test_compute_ranks_simple() {
        let values = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        let ranks = compute_ranks(&values);

        // 1.0 appears twice at positions 0 and 1 after sorting → average rank 1.5
        // 3.0 is rank 3
        // 4.0 is rank 4
        // 5.0 is rank 5
        assert!((ranks[0] - 3.0).abs() < 1e-10); // 3.0
        assert!((ranks[1] - 1.5).abs() < 1e-10); // 1.0 (tied)
        assert!((ranks[2] - 4.0).abs() < 1e-10); // 4.0
        assert!((ranks[3] - 1.5).abs() < 1e-10); // 1.0 (tied)
        assert!((ranks[4] - 5.0).abs() < 1e-10); // 5.0
    }

    #[test]
    fn test_sensitivity_analysis() {
        let mut inputs: HashMap<String, Vec<f64>> = HashMap::new();
        inputs.insert(
            "revenue".to_string(),
            vec![100.0, 110.0, 120.0, 130.0, 140.0],
        );
        inputs.insert("costs".to_string(), vec![50.0, 55.0, 52.0, 58.0, 60.0]);

        let mut outputs: HashMap<String, Vec<f64>> = HashMap::new();
        // Profit = revenue - costs (strongly correlated with revenue)
        outputs.insert("profit".to_string(), vec![50.0, 55.0, 68.0, 72.0, 80.0]);

        let analysis = SensitivityAnalysis::compute(&inputs, &outputs);

        // Revenue should have higher impact than costs
        let revenue_impact = analysis.get_correlation("revenue", "profit").unwrap();
        let costs_impact = analysis.get_correlation("costs", "profit").unwrap();

        assert!(
            revenue_impact.abs() > costs_impact.abs(),
            "Revenue impact {} should be greater than costs impact {}",
            revenue_impact,
            costs_impact
        );
    }

    #[test]
    fn test_tornado_ordering() {
        let mut inputs: HashMap<String, Vec<f64>> = HashMap::new();
        inputs.insert("high_impact".to_string(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        inputs.insert("low_impact".to_string(), vec![5.0, 4.0, 3.0, 2.0, 1.0]);
        inputs.insert("no_impact".to_string(), vec![3.0, 1.0, 4.0, 2.0, 5.0]);

        let mut outputs: HashMap<String, Vec<f64>> = HashMap::new();
        outputs.insert("result".to_string(), vec![10.0, 20.0, 30.0, 40.0, 50.0]); // Matches high_impact

        let analysis = SensitivityAnalysis::compute(&inputs, &outputs);

        let top = analysis.top_drivers("result", 3);
        assert_eq!(top.len(), 3);

        // High impact should be first (positive correlation)
        assert_eq!(top[0].variable, "high_impact");
        assert!(top[0].correlation > 0.9);

        // Low impact should be second (negative correlation, same magnitude)
        assert_eq!(top[1].variable, "low_impact");
        assert!(top[1].correlation < -0.9);
    }

    #[test]
    fn test_tornado_json() {
        let mut inputs: HashMap<String, Vec<f64>> = HashMap::new();
        inputs.insert("a".to_string(), vec![1.0, 2.0, 3.0]);
        inputs.insert("b".to_string(), vec![3.0, 2.0, 1.0]);

        let mut outputs: HashMap<String, Vec<f64>> = HashMap::new();
        outputs.insert("out".to_string(), vec![10.0, 20.0, 30.0]);

        let analysis = SensitivityAnalysis::compute(&inputs, &outputs);

        let json = analysis.to_tornado_json("out").unwrap();
        assert!(json.contains("\"output\":\"out\""));
        assert!(json.contains("\"sensitivity\":"));
        assert!(json.contains("\"variable\":"));
    }
}
