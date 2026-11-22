//! `forge validate`: whether the stored values still follow their formulas,
//! and whether the model still gives what its `expect:` says, in the file as
//! written and in each scenario that has one (ADR-074). One function,
//! [`validate_core`], does the checking for the command line, the MCP
//! server, the REST API and Forge Studio; the command prints what it returns.

use super::names::{expected_name, scalar};
use super::results::{ExpectationCheck, ValidationMismatch, ValidationResult};
use super::{apply_scenario, format_number, reject_distributions};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::to_15_digits;
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::{Expected, ParsedModel};
use colored::Colorize;
use std::path::{Path, PathBuf};

/// How far a stored `value:` may be from its formula's result
const STORED_TOLERANCE: f64 = 0.0001;

/// Whether `got` is what an expectation says, within 10⁻⁶ × max(1, |expected|):
/// absolute below 1, relative above it (ADR-074, question 3).
#[must_use]
pub fn meets(expected: f64, got: f64) -> bool {
    (got - expected).abs() <= 1e-6 * expected.abs().max(1.0)
}

/// Validate a file and return structured results (no printing): its stored
/// values, and its expectations in the file as written and in each scenario.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, a table's columns differ
/// in length, or the model, or a scenario with an `expect:`, does not
/// calculate.
pub fn validate_core(file: &Path) -> ForgeResult<ValidationResult> {
    validate_core_with(file, true)
}

/// As [`validate_core`], with the expectations checked or not.
///
/// Forge Studio checks an edit without them: an edit that moves a figure
/// away from its expectation is how a model is explored (ADR-074, question 6).
///
/// # Errors
///
/// As [`validate_core`].
pub fn validate_core_with(file: &Path, expectations: bool) -> ForgeResult<ValidationResult> {
    validate_model(&parser::parse_model(file)?, expectations)
}

fn validate_model(model: &ParsedModel, expectations: bool) -> ForgeResult<ValidationResult> {
    reject_distributions(model)?;
    model
        .check_table_lengths()
        .map_err(ForgeError::Validation)?;
    let calculated = ArrayCalculator::new(model.clone()).calculate_all()?;
    let mismatches = stale_values(model, &calculated);
    let mut result = ValidationResult {
        tables_valid: true,
        scalars_valid: mismatches.is_empty(),
        table_count: model.tables.len(),
        scalar_count: model.scalars.len(),
        mismatches,
        unstored: model
            .scalars
            .values()
            .filter(|v| v.formula.is_some() && v.value.is_none())
            .count(),
        expectations_checked: 0,
        base_checked: 0,
        scenarios_checked: 0,
        expectations_met: true,
        expectation_differences: Vec::new(),
    };
    if expectations {
        check_expectations(model, &calculated, &mut result)?;
    }
    Ok(result)
}

/// Each stored value that is not what its formula gives. A formula with no
/// stored value has nothing to compare.
fn stale_values(model: &ParsedModel, calculated: &ParsedModel) -> Vec<ValidationMismatch> {
    calculated
        .scalars
        .iter()
        .filter_map(|(name, var)| {
            let expected_value = var.value?;
            let current_value = model.scalars.get(name)?.value?;
            let diff = (current_value - expected_value).abs();
            (diff > STORED_TOLERANCE).then(|| ValidationMismatch {
                name: name.clone(),
                current_value,
                expected_value,
                diff,
            })
        })
        .collect()
}

/// The expectations of the file as written, against `base` (the model as
/// calculated), then of each scenario that has some, calculated under it, in
/// the order of their names.
fn check_expectations(
    model: &ParsedModel,
    base: &ParsedModel,
    result: &mut ValidationResult,
) -> ForgeResult<()> {
    let mut differences = unmet(checks(None, &model.expect, base));
    let mut checked = model.expect.len();
    let mut names: Vec<&String> = model
        .scenarios
        .iter()
        .filter(|(_, s)| !s.expect.is_empty())
        .map(|(name, _)| name)
        .collect();
    names.sort_unstable();
    for name in &names {
        let mut under = model.clone();
        apply_scenario(&mut under, name)?;
        let calculated = ArrayCalculator::new(under).calculate_all().map_err(|e| {
            ForgeError::Validation(format!("Scenario '{name}' does not calculate: {e}"))
        })?;
        let expect = &model.scenarios[*name].expect;
        differences.extend(unmet(checks(Some(name), expect, &calculated)));
        checked += expect.len();
    }
    result.expectations_checked = checked;
    result.base_checked = model.expect.len();
    result.scenarios_checked = names.len();
    result.expectations_met = differences.is_empty();
    result.expectation_differences = differences;
    Ok(())
}

fn unmet(checks: Vec<ExpectationCheck>) -> Vec<ExpectationCheck> {
    checks.into_iter().filter(|c| !c.met).collect()
}

/// Each expectation of `expect` against `calculated`, the model calculated
/// as the file is written or under `scenario`.
pub(super) fn checks(
    scenario: Option<&str>,
    expect: &[Expected],
    calculated: &ParsedModel,
) -> Vec<ExpectationCheck> {
    expect
        .iter()
        .map(|e| {
            let found = expected_name(calculated, &e.name).and_then(|full| {
                let value = scalar(calculated, &full).and_then(|v| v.value);
                value
                    .map(|got| (full, got))
                    .ok_or_else(|| "the model gives no number by this name".to_string())
            });
            let (path, got, reason) = match found {
                Ok((full, got)) => (Some(full), Some(got), None),
                Err(why) => (None, None, Some(why)),
            };
            // Compared in full, reported to 15 digits as every result is
            ExpectationCheck {
                scenario: scenario.map(str::to_string),
                name: e.name.clone(),
                path,
                met: got.is_some_and(|g| meets(e.value, g)),
                expected: to_15_digits(e.value),
                formula: e.formula.clone(),
                got: got.map(to_15_digits),
                diff: got.map(|g| to_15_digits((g - e.value).abs())),
                reason,
            }
        })
        .collect()
}

/// Execute the validate command for one or more files
///
/// # Errors
///
/// Returns an error if any file fails validation or cannot be parsed.
pub fn validate(c: &mut Console<'_>, files: &[PathBuf]) -> ForgeResult<()> {
    let file_count = files.len();
    let is_batch = file_count > 1;

    if is_batch {
        outln!(
            c,
            "{}",
            format!("✅ Validating {file_count} files").bold().green()
        );
        outln!(c);
    }

    let mut failed_files: Vec<String> = Vec::new();

    for file in files {
        if is_batch {
            outln!(c, "{}", format!("─── {} ───", file.display()).cyan());
        } else {
            outln!(c, "{}", "✅ Validating model".bold().green());
            outln!(c, "   File: {}\n", file.display());
        }

        match validate_single_file(c, file) {
            Ok(()) => {
                if is_batch {
                    outln!(c, "{}", format!("   ✅ {} - OK", file.display()).green());
                    outln!(c);
                }
            },
            Err(e) => {
                if !is_batch {
                    // Single file mode - propagate original error directly
                    return Err(e);
                }
                failed_files.push(format!("{}: {}", file.display(), e));
                outln!(c, "{}", format!("   ❌ {} - FAILED", file.display()).red());
                outln!(c, "      {}", e.to_string().red());
                outln!(c);
            },
        }
    }

    // Summary for batch validation
    if is_batch {
        outln!(c, "{}", "─".repeat(50));
        let passed = file_count - failed_files.len();
        outln!(
            c,
            "   {} passed, {} failed out of {} files",
            passed.to_string().green(),
            failed_files.len().to_string().red(),
            file_count
        );
    }

    if failed_files.is_empty() {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!(
            "{} file(s) failed validation",
            failed_files.len()
        )))
    }
}

/// Validate a single file, printing what [`validate_model`] finds.
fn validate_single_file(c: &mut Console<'_>, file: &Path) -> ForgeResult<()> {
    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    let states = !model.expect.is_empty() || model.scenarios.values().any(|s| !s.expect.is_empty());
    if model.tables.is_empty() && model.scalars.is_empty() && !states {
        outln!(
            c,
            "{}",
            "⚠️  No tables or scalars found in YAML file".yellow()
        );
        return Ok(());
    }

    outln!(
        c,
        "   Found {} tables, {} scalars",
        model.tables.len(),
        model.scalars.len()
    );

    // A table nothing reads would otherwise pass with columns of different
    // lengths; a formula that reads one finds out when it runs.
    if let Err(e) = model.check_table_lengths() {
        outln!(c, "\n{}", format!("❌ {e}").bold().red());
        return Err(ForgeError::Validation(e));
    }

    let result = validate_model(&model, true).inspect_err(|e| {
        outln!(
            c,
            "\n{}",
            format!("❌ Formula validation failed: {e}").bold().red()
        );
    })?;
    report_validation(c, &result)
}

/// What `forge validate` found: the stored values, then the expectations.
fn report_validation(c: &mut Console<'_>, result: &ValidationResult) -> ForgeResult<()> {
    outln!(c);
    let problems: Vec<String> = [report_stored(c, result), report_expectations(c, result)]
        .into_iter()
        .flatten()
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(ForgeError::Validation(problems.join("; and ")))
    }
}

/// Every stored value matches its formula, or each one that does not, with
/// what it should be. The problem, when there is one.
fn report_stored(c: &mut Console<'_>, result: &ValidationResult) -> Option<String> {
    if result.mismatches.is_empty() {
        outln!(c, "{}", "✅ All tables are valid!".bold().green());
        if result.unstored == 0 {
            outln!(
                c,
                "{}",
                "✅ All scalar values match their formulas!".bold().green()
            );
        } else {
            outln!(c, "{}", "✅ The formulas are valid.".bold().green());
            outln!(
                c,
                "   {} formula(s) have no stored value yet; run 'forge calculate' to store them",
                result.unstored
            );
        }
        return None;
    }
    outln!(
        c,
        "{}",
        format!("❌ Found {} value mismatches!", result.mismatches.len())
            .bold()
            .red()
    );
    outln!(
        c,
        "{}",
        "   The formulas are valid; the stored values are out of date.\n".yellow()
    );
    for m in &result.mismatches {
        outln!(c, "   {}", m.name.bright_blue().bold());
        outln!(
            c,
            "      Current:  {}",
            format_number(m.current_value).red()
        );
        outln!(
            c,
            "      Expected: {}",
            format_number(m.expected_value).green()
        );
        outln!(c, "      Diff:     {}", format!("{:.6}", m.diff).yellow());
        outln!(c);
    }
    outln!(
        c,
        "{}",
        "💡 Run 'forge calculate' to update values".bold().yellow()
    );
    Some("Stored values are out of date (the formulas are valid): run forge calculate".to_string())
}

/// The expectations checked: all met, or each one that is not, under the
/// scenario it belongs to. The problem, when there is one.
fn report_expectations(c: &mut Console<'_>, result: &ValidationResult) -> Option<String> {
    if result.expectations_checked == 0 {
        return None;
    }
    let mut runs = Vec::new();
    if result.base_checked > 0 {
        runs.push("the file as written".to_string());
    }
    if result.scenarios_checked > 0 {
        let s = if result.scenarios_checked == 1 {
            ""
        } else {
            "s"
        };
        runs.push(format!("{} scenario{s}", result.scenarios_checked));
    }
    let runs = runs.join(" and ");
    let differences = &result.expectation_differences;
    if differences.is_empty() {
        outln!(
            c,
            "{}",
            format!(
                "✅ {} expected results match ({runs})",
                result.expectations_checked
            )
            .bold()
            .green()
        );
        return None;
    }
    let problem = format!(
        "{} of {} expected results differ",
        differences.len(),
        result.expectations_checked
    );
    outln!(c, "{}\n", format!("❌ {problem} ({runs})").bold().red());
    let mut heading = None;
    for d in differences {
        if heading != Some(&d.scenario) {
            heading = Some(&d.scenario);
            let title = d.scenario.as_ref().map_or_else(
                || "the file as written".to_string(),
                |s| format!("scenario {s}"),
            );
            outln!(c, "   {title}");
        }
        let formula = d
            .formula
            .as_ref()
            .map(|f| format!(" ({f})"))
            .unwrap_or_default();
        let got = d.got.map_or_else(
            || d.reason.clone().unwrap_or_default(),
            |g| format_number(g).red().to_string(),
        );
        outln!(c, "      {}", d.name.bright_blue().bold());
        outln!(
            c,
            "         Expected: {}{formula}",
            format_number(d.expected).green()
        );
        outln!(c, "         Forge:    {got}");
    }
    outln!(c);
    outln!(
        c,
        "{}",
        "💡 The model no longer gives what its expect: says. Change the model, or the \
         expectation if the model is right"
            .bold()
            .yellow()
    );
    Some(problem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;
    use std::fs;
    use std::io::Write;

    const GOOD: &str = "_forge_version: \"5.0.0\"\ninputs:\n  a:\n    value: 2\n    formula: null\noutputs:\n  b:\n    value: 4\n    formula: \"=a * 2\"\n";

    fn model(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn validate_rejects_mc_distributions_with_a_clear_message() {
        let mut file = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            "_forge_version: \"5.0.0\"\nassumptions:\n  growth:\n    value: null\n    formula: \"=MC.Normal(0.15, 0.05)\"\n"
        )
        .unwrap();
        let err = validate_core(file.path()).unwrap_err().to_string();
        assert!(err.contains("assumptions.growth"), "{err}");
        assert!(err.contains("forge simulate"), "{err}");
        let (result, _) = capture(|c| validate(c, &[file.path().to_path_buf()]));
        assert!(result.unwrap_err().to_string().contains("forge simulate"));
    }

    #[test]
    fn quota_forecast_example_validates() {
        validate_core(Path::new("examples/quota_forecast.yaml"))
            .unwrap()
            .scalars_valid
            .then_some(())
            .expect("stored values match the formulas");
    }

    #[test]
    fn validating_a_batch_says_which_files_failed() {
        let dir = tempfile::tempdir().unwrap();
        let good = model(dir.path(), "good.yaml", GOOD);
        let stale = model(
            dir.path(),
            "stale.yaml",
            &GOOD.replace("value: 4", "value: 5"),
        );
        let (result, captured) = capture(|c| validate(c, &[good.clone(), stale]));
        let message = result.unwrap_err().to_string();
        assert!(message.contains("1 file(s) failed validation"), "{message}");
        for line in [
            "Validating 2 files",
            "good.yaml - OK",
            "stale.yaml - FAILED",
            "1 passed",
            "1 failed out of 2 files",
        ] {
            assert!(captured.out.contains(line), "{line} in {}", captured.out);
        }
        let (result, captured) = capture(|c| validate(c, &[good.clone(), good]));
        result.unwrap();
        assert!(captured.out.contains("2 passed"), "{}", captured.out);
    }

    #[test]
    fn validating_one_file_says_what_it_found() {
        let dir = tempfile::tempdir().unwrap();
        let good = model(dir.path(), "good.yaml", GOOD);
        let (result, captured) = capture(|c| validate(c, &[good]));
        result.unwrap();
        assert!(
            captured.out.contains("All scalar values match"),
            "{}",
            captured.out
        );

        let unstored = model(
            dir.path(),
            "u.yaml",
            &GOOD.replace("value: 4", "value: null"),
        );
        let (result, captured) = capture(|c| validate(c, &[unstored]));
        result.unwrap();
        assert!(
            captured.out.contains("1 formula(s) have no stored value"),
            "{}",
            captured.out
        );

        let empty = model(dir.path(), "e.yaml", "_forge_version: \"5.0.0\"\n");
        let (result, captured) = capture(|c| validate(c, &[empty]));
        result.unwrap();
        assert!(
            captured.out.contains("No tables or scalars"),
            "{}",
            captured.out
        );

        let stale = model(dir.path(), "s.yaml", &GOOD.replace("value: 4", "value: 5"));
        let (result, captured) = capture(|c| validate(c, std::slice::from_ref(&stale)));
        assert!(result.unwrap_err().to_string().contains("out of date"));
        assert!(captured.out.contains("Expected: 4"), "{}", captured.out);
        let found = validate_core(&stale).unwrap();
        assert_eq!(found.mismatches.len(), 1);
        assert!(!found.scalars_valid);
    }

    #[test]
    fn a_table_with_columns_of_different_lengths_is_refused_everywhere() {
        let dir = tempfile::tempdir().unwrap();
        let ragged = model(
            dir.path(),
            "r.yaml",
            "_forge_version: \"5.1.0\"\nsales:\n  month: [Jan, Feb, Mar]\n  units: [10, 20]\n",
        );
        let (result, captured) = capture(|c| validate(c, std::slice::from_ref(&ragged)));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("units") && err.contains("2 rows"), "{err}");
        assert!(captured.out.contains("❌"), "{}", captured.out);
        // The MCP server, the REST API and Studio refuse it too
        let core = validate_core(&ragged).unwrap_err().to_string();
        assert_eq!(core, err);
    }

    #[test]
    fn a_formula_that_fails_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let broken = model(
            dir.path(),
            "b.yaml",
            "_forge_version: \"5.1.0\"\na: 1\nout:\n  b: {formula: \"=a / nothing\"}\n",
        );
        let (result, captured) = capture(|c| validate(c, &[broken]));
        assert!(result.is_err());
        assert!(
            captured.out.contains("❌ Formula validation failed:"),
            "{}",
            captured.out
        );
    }

    /// kinship-protocol's spread game, cut down: w*, a game's result, and
    /// scenarios that move them (ADR-074's first use).
    const STATED: &str = "_forge_version: \"5.6.0\"\nw: 0.6\nc: 0\n\
        payoffs: {T: 5, R: 3, P: 1, S: 0}\n\
        kinship:\n  threshold: {formula: \"=(T - c - R) / (T - P)\"}\n\
        games:\n  dilemma:\n    players: [me, you]\n    strategies: [cooperate, defect]\n    symmetric: true\n    \
        payoffs:\n      cooperate: {cooperate: =R, defect: =S}\n      defect: {cooperate: =T - c, defect: =P - c}\n\
        expect:\n  kinship.threshold: 0.5\n  dilemma.equilibria: 1\n\
        scenarios:\n  costly:\n    scalars: {c: 0.5}\n    expect: {threshold: 0.375, dilemma.equilibria: 1}\n  \
        ruinous:\n    scalars: {c: 1}\n    expect: {kinship.threshold: =1/4}\n  \
        unstated:\n    scalars: {c: 9}\n";

    #[test]
    fn every_expectation_is_checked_in_the_file_and_each_scenario_that_has_one() {
        let dir = tempfile::tempdir().unwrap();
        let file = model(dir.path(), "m.yaml", STATED);
        let found = validate_core(&file).unwrap();
        assert!(
            found.expectations_met,
            "{:?}",
            found.expectation_differences
        );
        assert_eq!(found.expectations_checked, 5);
        assert_eq!(found.base_checked, 2);
        assert_eq!(found.scenarios_checked, 2, "unstated has no expect:");
        let (result, captured) = capture(|c| validate(c, &[file]));
        result.unwrap();
        assert!(
            captured
                .out
                .contains("✅ 5 expected results match (the file as written and 2 scenarios)"),
            "{}",
            captured.out
        );
        // Studio's edit check leaves them out
        let without = validate_core_with(
            &model(dir.path(), "x.yaml", &STATED.replace("0.375", "0.4")),
            false,
        )
        .unwrap();
        assert_eq!(without.expectations_checked, 0);
        assert!(without.expectations_met);
    }

    #[test]
    fn a_difference_names_the_scenario_the_result_and_both_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let broken = STATED
            .replace(
                "{threshold: 0.375, dilemma.equilibria: 1}",
                "{threshold: 0.4, dilemma.equilibria: 2}",
            )
            .replace(
                "kinship.threshold: 0.5",
                "kinship.threshold: 0.5\n  kinship.thresold: 0.5",
            )
            .replace("=1/4", "=1/3");
        let file = model(dir.path(), "m.yaml", &broken);
        let found = validate_core(&file).unwrap();
        assert!(!found.expectations_met);
        let seen: Vec<(Option<&str>, &str, f64, Option<f64>)> = found
            .expectation_differences
            .iter()
            .map(|d| (d.scenario.as_deref(), d.name.as_str(), d.expected, d.got))
            .collect();
        assert_eq!(
            seen,
            [
                (None, "kinship.thresold", 0.5, None),
                (Some("costly"), "threshold", 0.4, Some(0.375)),
                (Some("costly"), "dilemma.equilibria", 2.0, Some(1.0)),
                (
                    Some("ruinous"),
                    "kinship.threshold",
                    0.333_333_333_333_333,
                    Some(0.25)
                ),
            ]
        );
        let (result, captured) = capture(|c| validate(c, std::slice::from_ref(&file)));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("4 of 6 expected results differ"), "{err}");
        for line in [
            "❌ 4 of 6 expected results differ (the file as written and 2 scenarios)",
            "   the file as written\n      kinship.thresold\n         Expected: 0.5\n         Forge:    the model gives no number by this name",
            "   scenario costly\n      threshold\n         Expected: 0.4\n         Forge:    0.375\n      dilemma.equilibria\n         Expected: 2\n         Forge:    1\n   scenario ruinous",
            "   scenario ruinous\n      kinship.threshold\n         Expected: 0.333333 (=1/3)\n         Forge:    0.25",
            "💡 The model no longer gives what its expect: says",
        ] {
            assert!(captured.out.contains(line), "{line}\n---\n{}", captured.out);
        }
    }

    #[test]
    fn an_ambiguous_name_and_stale_values_are_each_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        let text = "_forge_version: \"5.6.0\"\na: {x: 1}\nb: {x: 2}\nout:\n  \
                    twice: {formula: \"=a.x * 2\", value: 5}\n\
                    scenarios:\n  one:\n    scalars: {a.x: 3}\n    expect: {out.twice: 6, x: 3}\n";
        let file = model(dir.path(), "m.yaml", text);
        let found = validate_core(&file).unwrap();
        let reasons: Vec<&str> = found
            .expectation_differences
            .iter()
            .map(|d| d.reason.as_deref().unwrap())
            .collect();
        assert_eq!(reasons.len(), 1, "out.twice is 6 under the scenario");
        let reason = reasons[0];
        assert!(reason.contains("'x' could be any of a.x, b.x"), "{reason}");
        assert_eq!(found.base_checked, 0);
        let (result, captured) = capture(|c| validate(c, &[file]));
        let err = result.unwrap_err().to_string();
        assert_eq!(
            err,
            "Validation error: Stored values are out of date (the formulas are valid): run \
             forge calculate; and 1 of 2 expected results differ"
        );
        assert!(captured.out.contains("(1 scenario)"), "{}", captured.out);
    }

    #[test]
    fn a_scenario_that_does_not_calculate_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let text = "_forge_version: \"5.6.0\"\nn: 2\nout:\n  r: {formula: \"=SQRT(n - 3)\"}\n\
                    scenarios:\n  bad:\n    scalars: {n: 1}\n    expect: {out.r: 1}\n  gone:\n    \
                    scalars: {nothing: 1}\n    expect: {out.r: 1}\n";
        let err = validate_core(&model(dir.path(), "m.yaml", &text.replace("n: 2", "n: 4")))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Scenario 'bad' does not calculate:"), "{err}");
        let gone = text.replace("bad:\n    scalars: {n: 1}\n    expect: {out.r: 1}\n  ", "");
        let err = validate_core(&model(dir.path(), "g.yaml", &gone.replace("n: 2", "n: 4")))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Scenario 'gone'"), "{err}");
    }

    #[test]
    fn a_file_of_expectations_and_no_scalars_is_still_checked() {
        let dir = tempfile::tempdir().unwrap();
        let text = "_forge_version: \"5.6.0\"\ngames:\n  g:\n    players: [a, b]\n    \
                    strategies: [x, y]\n    symmetric: true\n    payoffs:\n      \
                    x: {x: 1, y: 0}\n      y: {x: 0, y: 1}\nexpect: {g.equilibria: 3}\n";
        let (result, captured) = capture(|c| validate(c, &[model(dir.path(), "m.yaml", text)]));
        result.unwrap();
        assert!(
            captured
                .out
                .contains("✅ 1 expected results match (the file as written)"),
            "{}",
            captured.out
        );
    }

    #[test]
    fn the_tolerance_is_absolute_below_1_and_relative_above() {
        assert!(meets(0.5, 0.500_000_9));
        assert!(!meets(0.5, 0.500_001_1));
        assert!(!meets(0.0, 0.000_125));
        assert!(meets(3.2e10, 3.2e10 + 30_000.0));
        assert!(!meets(3.2e10, 3.2e10 + 40_000.0));
        assert!(meets(-19.0 / 15.0, -1.266_666_7));
    }
}
