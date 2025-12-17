//! Tornado Diagram Engine
//!
//! Performs one-at-a-time sensitivity analysis.

use super::config::{InputRange, TornadoConfig};
use crate::core::ArrayCalculator;
use crate::types::{ParsedModel, Variable};
use serde::{Deserialize, Serialize};

/// A single sensitivity bar in the tornado diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityBar {
    /// Input variable name
    pub input_name: String,
    /// Output at low input value
    pub output_at_low: f64,
    /// Output at high input value
    pub output_at_high: f64,
    /// Total swing (high - low output)
    pub swing: f64,
    /// Absolute swing for sorting
    pub abs_swing: f64,
    /// Low input value used
    pub input_low: f64,
    /// High input value used
    pub input_high: f64,
}

impl SensitivityBar {
    /// Generate ASCII bar representation
    pub fn to_ascii(&self, max_swing: f64, bar_width: usize) -> String {
        let ratio = self.abs_swing / max_swing;
        let filled = (ratio * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled);
        format!(
            "{:<20} |{:<width$}| +/- ${:.0}",
            self.input_name,
            bar,
            self.abs_swing / 2.0,
            width = bar_width
        )
    }
}

/// Complete tornado analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TornadoResult {
    /// Output variable name
    pub output: String,
    /// Base case output value
    pub base_value: f64,
    /// Sensitivity bars (sorted by impact)
    pub bars: Vec<SensitivityBar>,
    /// Total variance explained
    pub total_variance: f64,
}

impl TornadoResult {
    /// Export results to YAML format
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Generate ASCII tornado diagram
    pub fn to_ascii(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{} Sensitivity (Base: ${:.0})\n\n",
            self.output, self.base_value
        ));

        if self.bars.is_empty() {
            output.push_str("No sensitivity data\n");
            return output;
        }

        let max_swing = self
            .bars
            .iter()
            .map(|b| b.abs_swing)
            .fold(0.0_f64, f64::max);

        for bar in &self.bars {
            output.push_str(&bar.to_ascii(max_swing, 30));
            output.push('\n');
        }

        output
    }

    /// Get top N drivers
    pub fn top_drivers(&self, n: usize) -> Vec<&SensitivityBar> {
        self.bars.iter().take(n).collect()
    }

    /// Calculate percentage of variance explained by top N drivers
    pub fn variance_explained_by_top(&self, n: usize) -> f64 {
        if self.total_variance == 0.0 {
            return 0.0;
        }
        let top_variance: f64 = self.bars.iter().take(n).map(|b| b.abs_swing).sum();
        top_variance / self.total_variance * 100.0
    }
}

/// Tornado Diagram Engine
pub struct TornadoEngine {
    config: TornadoConfig,
    base_model: ParsedModel,
}

impl TornadoEngine {
    /// Create a new tornado engine
    pub fn new(config: TornadoConfig, base_model: ParsedModel) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config, base_model })
    }

    /// Run the sensitivity analysis
    pub fn analyze(&self) -> Result<TornadoResult, String> {
        // Calculate base case
        let base_value = self.calculate_output(&self.base_model)?;

        // Determine if we need cross-scale normalization
        // Get base values for all inputs to check if they span multiple orders of magnitude
        let input_bases: Vec<f64> = self
            .config
            .inputs
            .iter()
            .filter_map(|input| {
                input.base.or_else(|| {
                    self.base_model
                        .scalars
                        .get(&input.name)
                        .and_then(|v| v.value)
                })
            })
            .map(f64::abs)
            .filter(|v| *v > 1e-10)
            .collect();

        // Check if inputs span vastly different scales (e.g., dollars vs rates)
        let needs_normalization = if input_bases.len() >= 2 {
            let max_base = input_bases.iter().fold(0.0_f64, |a, &b| a.max(b));
            let min_base = input_bases.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            max_base / min_base > 100.0 // More than 2 orders of magnitude difference
        } else {
            false
        };

        // Calculate sensitivity for each input
        let mut bars: Vec<SensitivityBar> = Vec::new();

        for input in &self.config.inputs {
            let bar = self.calculate_sensitivity(input, base_value, needs_normalization)?;
            bars.push(bar);
        }

        // Sort by absolute swing (largest impact first)
        bars.sort_by(|a, b| {
            b.abs_swing
                .partial_cmp(&a.abs_swing)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Calculate total variance
        let total_variance: f64 = bars.iter().map(|b| b.abs_swing).sum();

        Ok(TornadoResult {
            output: self.config.output.clone(),
            base_value,
            bars,
            total_variance,
        })
    }

    /// Calculate sensitivity for a single input
    fn calculate_sensitivity(
        &self,
        input: &InputRange,
        _base_value: f64,
        needs_normalization: bool,
    ) -> Result<SensitivityBar, String> {
        // Calculate output at low input value
        let output_at_low = self.calculate_with_override(&input.name, input.low)?;

        // Calculate output at high input value
        let output_at_high = self.calculate_with_override(&input.name, input.high)?;

        let swing = output_at_high - output_at_low;
        let raw_abs_swing = swing.abs();

        // Apply normalization only when comparing inputs of vastly different scales
        let abs_swing = if needs_normalization {
            // Get the base input value from the model to calculate relative range
            let input_base = input
                .base
                .or_else(|| {
                    self.base_model
                        .scalars
                        .get(&input.name)
                        .and_then(|v| v.value)
                })
                .unwrap_or(1.0); // Default to 1.0 if no base found

            // Calculate relative input range (as fraction of base)
            // This normalizes inputs of different scales (e.g., dollars vs rates)
            let input_range = input.high - input.low;
            let relative_range = if input_base.abs() > 1e-10 {
                input_range / input_base.abs()
            } else {
                1.0 // Avoid division by zero
            };

            // Weight sensitivity by square of relative range
            // This ensures inputs varied by larger percentages rank higher
            raw_abs_swing * relative_range * relative_range
        } else {
            // For inputs of similar scale, use raw absolute swing
            raw_abs_swing
        };

        Ok(SensitivityBar {
            input_name: input.name.clone(),
            output_at_low,
            output_at_high,
            swing,
            abs_swing,
            input_low: input.low,
            input_high: input.high,
        })
    }

    /// Calculate output with a specific input override
    fn calculate_with_override(&self, input_name: &str, input_value: f64) -> Result<f64, String> {
        let mut model = self.base_model.clone();

        // Override the input value
        if let Some(scalar) = model.scalars.get_mut(input_name) {
            scalar.value = Some(input_value);
            scalar.formula = None; // Clear formula to use override value
        } else {
            // Create new scalar
            model.scalars.insert(
                input_name.to_string(),
                Variable::new(input_name.to_string(), Some(input_value), None),
            );
        }

        self.calculate_output(&model)
    }

    /// Calculate the output variable value
    fn calculate_output(&self, model: &ParsedModel) -> Result<f64, String> {
        let calculator = ArrayCalculator::new(model.clone());
        let result = calculator.calculate_all().map_err(|e| e.to_string())?;

        result
            .scalars
            .get(&self.config.output)
            .and_then(|v| v.value)
            .ok_or_else(|| {
                format!(
                    "Output variable '{}' not found or has no value",
                    self.config.output
                )
            })
    }

    /// Get the configuration
    pub fn config(&self) -> &TornadoConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;

    fn create_test_model() -> ParsedModel {
        let mut model = ParsedModel::new();

        // Inputs
        model.scalars.insert(
            "revenue".to_string(),
            Variable::new("revenue".to_string(), Some(1_000_000.0), None),
        );
        model.scalars.insert(
            "cost_rate".to_string(),
            Variable::new("cost_rate".to_string(), Some(0.60), None),
        );
        model.scalars.insert(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.25), None),
        );

        // Output: profit = revenue * (1 - cost_rate) * (1 - tax_rate)
        model.scalars.insert(
            "profit".to_string(),
            Variable::new(
                "profit".to_string(),
                None,
                Some("=revenue * (1 - cost_rate) * (1 - tax_rate)".to_string()),
            ),
        );

        model
    }

    #[test]
    fn test_tornado_analysis() {
        let model = create_test_model();
        let config = TornadoConfig::new("profit")
            .with_input(InputRange::new("revenue", 800_000.0, 1_200_000.0))
            .with_input(InputRange::new("cost_rate", 0.50, 0.70))
            .with_input(InputRange::new("tax_rate", 0.20, 0.30));

        let engine = TornadoEngine::new(config, model).unwrap();
        let result = engine.analyze().unwrap();

        // Should have 3 bars
        assert_eq!(result.bars.len(), 3);

        // Bars should be sorted by impact
        for i in 0..result.bars.len() - 1 {
            assert!(
                result.bars[i].abs_swing >= result.bars[i + 1].abs_swing,
                "Bars should be sorted by impact"
            );
        }

        // Revenue should have the biggest impact (absolute dollars)
        assert_eq!(result.bars[0].input_name, "revenue");
    }

    #[test]
    fn test_ascii_output() {
        let model = create_test_model();
        let config = TornadoConfig::new("profit")
            .with_input(InputRange::new("revenue", 800_000.0, 1_200_000.0))
            .with_input(InputRange::new("cost_rate", 0.50, 0.70));

        let engine = TornadoEngine::new(config, model).unwrap();
        let result = engine.analyze().unwrap();
        let ascii = result.to_ascii();

        assert!(ascii.contains("profit Sensitivity"));
        assert!(ascii.contains("revenue"));
        assert!(ascii.contains("cost_rate"));
    }

    #[test]
    fn test_top_drivers() {
        let model = create_test_model();
        let config = TornadoConfig::new("profit")
            .with_input(InputRange::new("revenue", 800_000.0, 1_200_000.0))
            .with_input(InputRange::new("cost_rate", 0.50, 0.70))
            .with_input(InputRange::new("tax_rate", 0.20, 0.30));

        let engine = TornadoEngine::new(config, model).unwrap();
        let result = engine.analyze().unwrap();

        let top_2 = result.top_drivers(2);
        assert_eq!(top_2.len(), 2);

        // Check variance explained
        let pct = result.variance_explained_by_top(2);
        assert!(pct > 50.0, "Top 2 should explain > 50% of variance");
    }

    #[test]
    fn test_yaml_export() {
        let model = create_test_model();
        let config = TornadoConfig::new("profit").with_input(InputRange::new(
            "revenue",
            800_000.0,
            1_200_000.0,
        ));

        let engine = TornadoEngine::new(config, model).unwrap();
        let result = engine.analyze().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("output: profit"));
        assert!(yaml.contains("bars:"));
    }

    #[test]
    fn test_json_export() {
        let model = create_test_model();
        let config = TornadoConfig::new("profit").with_input(InputRange::new(
            "revenue",
            800_000.0,
            1_200_000.0,
        ));

        let engine = TornadoEngine::new(config, model).unwrap();
        let result = engine.analyze().unwrap();
        let json = result.to_json().unwrap();

        assert!(json.contains("\"output\""));
        assert!(json.contains("\"bars\""));
    }
}
