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
    // Truncation is mathematically impossible: ratio is 0..=1, bar_width is small
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    #[must_use]
    pub fn to_ascii(&self, max_swing: f64, bar_width: usize) -> String {
        let ratio = self.abs_swing / max_swing;
        let filled = (ratio * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled);
        format!(
            "{:<20} |{:<width$}| swing {:.0}",
            self.input_name,
            bar,
            self.abs_swing,
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
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(&crate::core::precision::Rounded(self))
            .unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&crate::core::precision::Rounded(self))
    }

    /// Generate ASCII tornado diagram
    pub fn to_ascii(&self) -> String {
        use std::fmt::Write;

        let mut output = String::new();

        let _ = write!(
            output,
            "{} Sensitivity (Base: ${:.0})\n\n",
            self.output, self.base_value
        );

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
    #[must_use]
    pub fn top_drivers(&self, n: usize) -> Vec<&SensitivityBar> {
        self.bars.iter().take(n).collect()
    }

    /// Calculate percentage of variance explained by top N drivers
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: TornadoConfig, base_model: ParsedModel) -> Result<Self, String> {
        config.validate()?;
        let mut config = config;
        config.output = resolve_scalar_name(&base_model, &config.output)
            .map_err(|error| format!("Tornado output '{}': {error}", config.output))?;
        for input in &mut config.inputs {
            input.name = resolve_scalar_name(&base_model, &input.name)
                .map_err(|error| format!("Tornado input '{}': {error}", input.name))?;
            base_model.refuse_date_input(&input.name, "tornado")?;
        }
        Ok(Self { config, base_model })
    }

    /// Run the sensitivity analysis
    ///
    /// # Errors
    ///
    /// Returns an error if the base model or any sensitivity calculation fails.
    pub fn analyze(&self) -> Result<TornadoResult, String> {
        // Calculate base case
        let base_value = self.calculate_output(&self.base_model)?;

        // Calculate sensitivity for each input
        let mut bars: Vec<SensitivityBar> = Vec::new();

        for input in &self.config.inputs {
            let bar = self.calculate_sensitivity(input)?;
            bars.push(bar);
        }

        // Largest swing first. Ties keep a fixed order (by name), so the same model
        // always prints the same diagram.
        bars.sort_by(|a, b| {
            b.abs_swing
                .total_cmp(&a.abs_swing)
                .then_with(|| a.input_name.cmp(&b.input_name))
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
    fn calculate_sensitivity(&self, input: &InputRange) -> Result<SensitivityBar, String> {
        // Calculate output at low input value
        let output_at_low = self.calculate_with_override(&input.name, input.low)?;

        // Calculate output at high input value
        let output_at_high = self.calculate_with_override(&input.name, input.high)?;

        // Bars are ranked by the change in the output, the same number that is shown.
        let swing = output_at_high - output_at_low;
        let abs_swing = swing.abs();

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
    #[must_use]
    pub const fn config(&self) -> &TornadoConfig {
        &self.config
    }
}

/// Resolve exact scalar IDs first, then allow a unique unqualified leaf name.
///
/// Forge v5 stores grouped inputs/outputs as IDs such as `inputs.price` and
/// `outputs.revenue`. Tornado configs commonly use the leaf names (`price`,
/// `revenue`). Previously an unmatched leaf was silently inserted as a new
/// scalar, creating duplicate variables and leaving formulas bound to the
/// original grouped scalar. Its effect depended on formula name resolution.
/// Reject ambiguous/missing names rather than inventing variables.
fn resolve_scalar_name(model: &ParsedModel, name: &str) -> Result<String, String> {
    if model.scalars.contains_key(name) {
        return Ok(name.to_string());
    }
    if name.contains('.') {
        return Err("not found; use an existing scalar ID".to_string());
    }

    let mut matches: Vec<&str> = model
        .scalars
        .keys()
        .map(String::as_str)
        .filter(|candidate| candidate.rsplit('.').next() == Some(name))
        .collect();
    matches.sort_unstable();
    match matches.as_slice() {
        [resolved] => Ok((*resolved).to_string()),
        [] => Err("not found; use an existing scalar ID".to_string()),
        _ => Err(format!(
            "ambiguous; use a fully qualified scalar ID (matches: {})",
            matches.join(", ")
        )),
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

        // Profit swings by 150,000 for cost_rate (0.5 to 0.7), 120,000 for revenue
        // (800k to 1.2M) and 40,000 for tax_rate (0.2 to 0.3).
        let names: Vec<&str> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
        assert_eq!(names, ["cost_rate", "revenue", "tax_rate"]);
        assert!((result.bars[0].abs_swing - 150_000.0).abs() < 1.0);
        assert!((result.bars[1].abs_swing - 120_000.0).abs() < 1.0);
        assert!((result.bars[2].abs_swing - 40_000.0).abs() < 1.0);
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
