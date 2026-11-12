//! `forge simulate --seek`: the value of one input at which an output meets
//! a threshold with a given probability ("the price at which the year makes
//! money in 80% of runs").
//!
//! Every simulation uses the same configuration and seed, so the uncertain
//! inputs are drawn identically for every value tried, and the probability
//! is a step function of the input. Bisection finds the step where it first
//! reaches the target.

use super::simulate::{prepare, run_model, SimulateOptions};
use crate::error::{ForgeError, ForgeResult};
use crate::monte_carlo::statistics::{evaluate_threshold, parse_threshold};
use serde::Serialize;
use std::path::Path;

/// What to seek: `P(<output> <op> <value>) = <probability>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SeekGoal {
    /// The output (a scalar, or one row of a column: `months.profit (row 1)`)
    pub output: String,
    /// The threshold, as in `thresholds:` (`"> 0"`)
    pub threshold: String,
    /// The probability to reach, between 0 and 1
    pub probability: f64,
}

impl SeekGoal {
    /// Read `P(year.profit > 0) = 0.8` (a probability may be written `80%`).
    ///
    /// # Errors
    ///
    /// Returns an error that shows the expected form.
    pub fn parse(text: &str) -> Result<Self, String> {
        let form =
            || format!("--seek '{text}': write it as \"P(year.profit > 0) = 0.8\" (or = 80%)");
        let (inner, target) = text.rsplit_once('=').ok_or_else(form)?;
        let inner = inner.trim();
        let inner = inner
            .strip_prefix("P(")
            .or_else(|| inner.strip_prefix("p("))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(form)?;
        let target = target.trim();
        let probability = target
            .strip_suffix('%')
            .map_or_else(
                || target.parse::<f64>(),
                |percent| percent.trim().parse::<f64>().map(|p| p / 100.0),
            )
            .map_err(|_| form())?;
        if !(probability > 0.0 && probability < 1.0) {
            return Err(format!(
                "--seek '{text}': the probability must be between 0 and 1 (0% and 100%), not included"
            ));
        }
        let split = inner.find(['>', '<', '=']).ok_or_else(form)?;
        let (output, threshold) = inner.split_at(split);
        let output = output.trim().to_string();
        let threshold = threshold.trim().to_string();
        parse_threshold(&threshold).map_err(|e| format!("--seek '{text}': {e}"))?;
        if output.is_empty() {
            return Err(form());
        }
        Ok(Self {
            output,
            threshold,
            probability,
        })
    }
}

/// The answer.
#[derive(Debug, Clone, Serialize)]
pub struct SeekResult {
    /// The input varied
    pub input: String,
    /// The value found
    pub value: f64,
    /// The probability at that value
    pub probability: f64,
    /// The goal, as given
    pub goal: String,
    /// The probability at each end of the range searched
    pub probability_at_min: f64,
    pub probability_at_max: f64,
    /// Simulations run
    pub simulations: usize,
    /// Iterations in each
    pub iterations: usize,
}

/// Find the value of `input` in `[min, max]` at which `goal` is first met.
///
/// # Errors
///
/// Returns an error if the model or settings are invalid, `input` is not an
/// input of the model (a scalar without a formula), the output is not
/// simulated, or the probability is not reached within the range.
#[allow(clippy::too_many_arguments)] // the CLI's arguments, one each
pub fn probability_seek_core(
    file: &Path,
    options: SimulateOptions<'_>,
    goal: &SeekGoal,
    input: &str,
    min: f64,
    max: f64,
) -> ForgeResult<SeekResult> {
    if min >= max || !min.is_finite() || !max.is_finite() {
        return Err(ForgeError::Validation(format!(
            "--min ({min}) must be below --max ({max})"
        )));
    }
    let (mut config, model) = prepare(file, options)?;
    // Simulate only the output the goal is about
    let output = config
        .outputs
        .iter()
        .find(|o| o.variable == goal.output)
        .cloned()
        .ok_or_else(|| {
            ForgeError::Validation(format!(
                "--seek: '{}' is not an output of monte_carlo (outputs: {})",
                goal.output,
                config
                    .outputs
                    .iter()
                    .map(|o| o.variable.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
    config.outputs = vec![output];

    let resolved = model
        .resolve_scalar_name(input)
        .map_err(ForgeError::Validation)?;
    if model.scalars[&resolved].formula.is_some() {
        return Err(ForgeError::Validation(format!(
            "--vary {resolved}: it has a formula; vary an input (a scalar with a plain value)"
        )));
    }

    let (op, threshold) = parse_threshold(&goal.threshold).map_err(ForgeError::Validation)?;
    let plan = crate::core::array_calculator::CalcPlan::new();
    let mut simulations = 0;
    let mut probability_at = |value: f64| -> ForgeResult<f64> {
        let mut trial = model.clone();
        trial
            .override_scalar(&resolved, Some(value), None)
            .map_err(ForgeError::Validation)?;
        let result = run_model(&config, &trial, &plan)?;
        simulations += 1;
        Ok(evaluate_threshold(
            &result.outputs[&goal.output].samples,
            &op,
            threshold,
        ))
    };

    let at_min = probability_at(min)?;
    let at_max = probability_at(max)?;
    let reaches = |p: f64| p >= goal.probability;
    let (mut lo, mut hi, rising) = match (reaches(at_min), reaches(at_max)) {
        (false, true) => (min, max, true),
        (true, false) => (min, max, false),
        _ => {
            return Err(ForgeError::Validation(format!(
                "{} is {:.2}% at {min} and {:.2}% at {max}: the goal {:.2}% is not crossed in that range",
                format_args!("P({} {})", goal.output, goal.threshold),
                at_min * 100.0,
                at_max * 100.0,
                goal.probability * 100.0
            )))
        },
    };
    // The probability changes in steps; narrow down to where it first
    // reaches the goal, to a millionth of the range
    let mut at_answer = if rising { at_max } else { at_min };
    let tolerance = (max - min) * 1e-6;
    // 60 halvings reach any tolerance a f64 range can hold
    for _ in 0..60 {
        if hi - lo <= tolerance {
            break;
        }
        let mid = lo.midpoint(hi);
        let p = probability_at(mid)?;
        if reaches(p) == rising {
            hi = mid;
            if rising {
                at_answer = p;
            }
        } else {
            lo = mid;
            if !rising {
                at_answer = p;
            }
        }
    }
    let value = if rising { hi } else { lo };
    Ok(SeekResult {
        input: resolved,
        value,
        probability: at_answer,
        goal: format!(
            "P({} {}) = {}",
            goal.output, goal.threshold, goal.probability
        ),
        probability_at_min: at_min,
        probability_at_max: at_max,
        simulations,
        iterations: config.iterations,
    })
}

/// Execute `forge simulate --seek`.
///
/// # Errors
///
/// Returns an error if the goal cannot be read or [`probability_seek_core`] fails.
pub fn probability_seek(
    file: &Path,
    options: SimulateOptions<'_>,
    seek: &str,
    vary: &str,
    min: f64,
    max: f64,
) -> ForgeResult<()> {
    use colored::Colorize;
    let goal = SeekGoal::parse(seek).map_err(ForgeError::Validation)?;
    println!("{}", "🎯 Forge - Probability Seek".bold().green());
    println!("   File: {}", file.display());
    println!(
        "   Goal: P({} {}) = {:.2}%",
        goal.output,
        goal.threshold,
        goal.probability * 100.0
    );
    println!("   Vary: {vary} from {min} to {max}");
    println!();
    let result = probability_seek_core(file, options, &goal, vary, min, max)?;
    println!(
        "   {} = {}",
        result.input.bright_blue().bold(),
        super::display_number(result.value).bold()
    );
    println!(
        "   P({} {}) there: {:.2}%  ({:.2}% at {min}, {:.2}% at {max})",
        goal.output,
        goal.threshold,
        result.probability * 100.0,
        result.probability_at_min * 100.0,
        result.probability_at_max * 100.0
    );
    println!(
        "   {} simulations of {} iterations, the same draws each time",
        result.simulations, result.iterations
    );
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goals_are_read_in_both_forms() {
        let goal = SeekGoal::parse("P(year.profit > 0) = 0.8").unwrap();
        assert_eq!(
            goal,
            SeekGoal {
                output: "year.profit".to_string(),
                threshold: "> 0".to_string(),
                probability: 0.8
            }
        );
        let goal = SeekGoal::parse("P(months.profit (row 1) >= -500) = 95%").unwrap();
        assert_eq!(goal.output, "months.profit (row 1)");
        assert_eq!(goal.threshold, ">= -500");
        assert!((goal.probability - 0.95).abs() < 1e-12);
        for bad in [
            "year.profit > 0",
            "P(year.profit) = 0.8",
            "P(x > 0) = 1.5",
            "P(x > a) = 0.5",
        ] {
            assert!(SeekGoal::parse(bad).is_err(), "{bad}");
        }
    }
}
