//! Tornado Diagram Engine
//!
//! Performs one-at-a-time sensitivity analysis.

use super::config::{InputRange, TornadoConfig};
use crate::core::precision;
use crate::core::ArrayCalculator;
use crate::types::ParsedModel;
use num_traits::ToPrimitive;
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
    /// Said only when the input cannot move the output
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl SensitivityBar {
    /// Generate ASCII bar representation
    // Truncation is mathematically impossible: ratio is 0..=1, bar_width is small
    #[must_use]
    pub fn to_ascii(&self, max_swing: f64, bar_width: usize) -> String {
        let ratio = self.abs_swing / max_swing;
        // A swing is at most the largest, so at most the whole bar; NaN fills none
        let filled = (ratio * precision::count(bar_width))
            .to_usize()
            .unwrap_or(0);
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
        let note = output_at_high.total_cmp(&output_at_low).is_eq().then(|| {
            "Changing this input does not change the output: the output's formula does not use it"
                .to_string()
        });

        Ok(SensitivityBar {
            input_name: input.name.clone(),
            output_at_low,
            output_at_high,
            swing,
            abs_swing,
            input_low: input.low,
            input_high: input.high,
            note,
        })
    }

    /// Calculate output with a specific input override
    fn calculate_with_override(&self, input_name: &str, input_value: f64) -> Result<f64, String> {
        let mut model = self.base_model.clone();
        // The input is the model's or an included file's (`@costs.flour`); `new`
        // found it
        model.set_input(input_name, input_value)?;
        self.calculate_output(&model)
    }

    /// Calculate the output variable value. A formula the output does not use
    /// may fail at an input's end of the range without failing the bar.
    fn calculate_output(&self, model: &ParsedModel) -> Result<f64, String> {
        ArrayCalculator::new(model.clone())
            .calculate_scalar(&self.config.output)
            .map_err(|e| e.to_string())
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
    if name.starts_with('@') {
        return crate::cli::commands::input_name(model, name);
    }
    // A game's result is calculated with the model, which says if it is not one
    if model.scalars.contains_key(name) || model.names_game_result(name) {
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
    use crate::types::Variable;

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

    /// A formula the output does not use may fail at an input's end of the
    /// range: `=1 / (cost_rate - 0.5)` divides by zero at 0.50. The profit
    /// does not use it, so its bar is the profit's, 1,000,000 x (1 - 0.50) x
    /// 0.75 = 375,000 and 1,000,000 x 0.30 x 0.75 = 225,000. The tornado
    /// failed with the division's error.
    #[test]
    fn a_formula_the_output_does_not_use_does_not_stop_the_tornado() {
        let mut model = create_test_model();
        model.scalars.insert(
            "per_point".to_string(),
            Variable::new(
                "per_point".to_string(),
                None,
                Some("=1 / (cost_rate - 0.5)".to_string()),
            ),
        );
        let config =
            TornadoConfig::new("profit").with_input(InputRange::new("cost_rate", 0.50, 0.70));
        let result = TornadoEngine::new(config, model)
            .unwrap()
            .analyze()
            .unwrap();
        let bar = &result.bars[0];
        assert!(
            (bar.output_at_low - 375_000.0).abs() <= 1e-6,
            "{}",
            bar.output_at_low
        );
        assert!(
            (bar.output_at_high - 225_000.0).abs() <= 1e-6,
            "{}",
            bar.output_at_high
        );
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

    /// The inspection game of ADR-068: a harsher penalty lowers the audit
    /// rate and not the cheating. nashpy 0.0.43 and the hand calculation:
    /// the human audits with gain / (gain + penalty), the AI cheats with
    /// audit cost / harm, whatever the penalty.
    #[test]
    fn a_tornado_on_a_games_result_varies_its_payoffs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inspection.yaml");
        std::fs::write(
            &path,
            "_forge_version: \"5.5.0\"
inputs: {gain: 2, penalty: 4, audit_cost: 1, harm: 3}
games:
  inspection:
    players: [ai, human]
    strategies: {ai: [comply, cheat], human: [audit, trust]}
    payoffs:
      comply: {audit: [0, =-inputs.audit_cost], trust: [0, 0]}
      cheat: {audit: [=-inputs.penalty, =-inputs.audit_cost], trust: [=inputs.gain, =-inputs.harm]}
",
        )
        .unwrap();
        let model = crate::parser::parse_model(&path).unwrap();
        let config = |output: &str| {
            let mut config = TornadoConfig::new(output);
            config
                .inputs
                .push(InputRange::new("inputs.penalty", 2.0, 8.0));
            config.inputs.push(InputRange::new("inputs.gain", 1.0, 3.0));
            config
        };
        let audit = TornadoEngine::new(
            config("inspection.equilibrium_1.human.audit"),
            model.clone(),
        )
        .unwrap()
        .analyze()
        .unwrap();
        let bar = |result: &TornadoResult, name: &str| {
            result
                .bars
                .iter()
                .find(|b| b.input_name == name)
                .unwrap()
                .clone()
        };
        // Penalty 2: 2 / 4; penalty 8: 2 / 10. Gain 1: 1 / 5; gain 3: 3 / 7.
        let penalty = bar(&audit, "inputs.penalty");
        assert!((penalty.output_at_low - 0.5).abs() < 1e-12, "{penalty:?}");
        assert!((penalty.output_at_high - 0.2).abs() < 1e-12, "{penalty:?}");
        let gain = bar(&audit, "inputs.gain");
        assert!((gain.output_at_low - 0.2).abs() < 1e-12, "{gain:?}");
        assert!((gain.output_at_high - 3.0 / 7.0).abs() < 1e-12, "{gain:?}");
        let cheat = TornadoEngine::new(config("inspection.equilibrium_1.ai.cheat"), model.clone())
            .unwrap()
            .analyze()
            .unwrap();
        for bar in &cheat.bars {
            assert!((bar.output_at_low - 1.0 / 3.0).abs() < 1e-12, "{bar:?}");
            assert!((bar.output_at_high - 1.0 / 3.0).abs() < 1e-12, "{bar:?}");
        }
        let err = TornadoEngine::new(config("inspection.equilibrium_2.human.audit"), model)
            .unwrap()
            .analyze()
            .unwrap_err();
        assert!(err.contains("it has 1 equilibrium"), "{err}");
    }

    /// Roadmap T1-VARY-INCLUDED: a tornado input of a file the model includes,
    /// written as a scenario override writes it, moves that file's value; each
    /// end of its bar is the plain model with that value. An include that is
    /// not there is refused with the model's includes.
    #[test]
    fn an_input_of_an_included_file_has_its_bar() {
        let file = std::path::Path::new("examples/acquisition/deal.yaml");
        let model = crate::parser::parse_model(file).unwrap();
        let cash = "@target.company.free_cash_flow_next_year";
        let config = |name: &str| {
            let mut config = TornadoConfig::new("valuation.value_created");
            config.inputs.push(InputRange::new(name, 50.0, 70.0));
            config
        };
        let engine = TornadoEngine::new(config(cash), model.clone()).unwrap();
        let bar = &engine.analyze().unwrap().bars[0];
        assert_eq!(bar.input_name, cash);
        for (value, got) in [(50.0, bar.output_at_low), (70.0, bar.output_at_high)] {
            let mut plain = model.clone();
            plain.override_scalar(cash, Some(value), None).unwrap();
            let result = ArrayCalculator::new(plain).calculate_all().unwrap();
            let want = result.scalars["valuation.value_created"].value.unwrap();
            assert!((got - want).abs() <= 1e-9, "{value}: {got}, not {want}");
        }
        let Err(err) = TornadoEngine::new(config("@seller.company.cash"), model) else {
            panic!("an include that is not there")
        };
        assert!(
            err.starts_with("Tornado input '@seller.company.cash'"),
            "{err}"
        );
        assert!(err.contains("buyer, target"), "{err}");
    }
}
