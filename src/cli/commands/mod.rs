//! CLI commands for Forge
//!
//! This module provides all CLI command implementations:
//! - calculate: Calculate formulas in YAML files
//! - validate: Validate YAML files for consistency
//! - watch: Watch files for changes and recalculate
//! - audit: Show calculation dependency chain
//! - export/import: Excel file I/O
//! - `variance/sensitivity/goal_seek/break_even`: Analysis tools
//! - compare: Scenario comparison
//! - report: Scalars side by side, from a calculated file
//! - functions: List supported functions
//! - simulate: Monte Carlo simulation
//! - upgrade: Schema migration
//! - scenarios: Scenario analysis
//! - `decision_tree`: Decision tree analysis
//! - `real_options`: Real options valuation
//! - tornado: Tornado sensitivity diagrams
//! - bootstrap: Bootstrap resampling
//! - bayesian: Bayesian network inference

mod analysis;
mod audit;
#[cfg(feature = "excel")]
mod carried;
mod compare;
mod examples;
// Export and import need Excel (feature `excel`); printing them and base64
// belong to the command line (feature `cli`)
#[cfg(feature = "excel")]
mod excel_io;
mod functions;
mod names;
pub(crate) use names::{input_name, scalar};
mod prediction;
mod probability_seek;
mod report;
pub mod results;
mod schema;
mod simulate;
// Self-update downloads releases (feature `cli`)
#[cfg(feature = "cli")]
mod update;
mod upgrade;
// Watch mode brings the file watcher (feature `cli`)
#[cfg(feature = "cli")]
mod watch;

// Re-exports
pub use analysis::{break_even, goal_seek, sensitivity, sensitivity_in_format, variance};
pub use audit::audit;
pub use compare::{compare, comparison_json, render_comparison, terminal_width};
#[cfg(feature = "cli")]
pub use examples::examples;
#[cfg(feature = "cli")]
pub use excel_io::{export, import};
pub use functions::functions;
pub use prediction::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};
pub use report::{report, ReportFormat, ReportOptions};
pub use schema::schema;
pub use simulate::simulate;
#[cfg(feature = "cli")]
pub use update::update;
pub use upgrade::upgrade;
#[cfg(feature = "cli")]
pub use watch::watch;

// Core function re-exports (return structured results, no printing)
pub use analysis::{goal_seek_core, pct_is_meaningful, sensitivity_core, variance_core};
pub use audit::audit_core;
pub use compare::compare_core;
pub use examples::examples_core;
#[cfg(feature = "cli")]
pub use excel_io::export_buffer_core;
#[cfg(feature = "excel")]
pub use excel_io::{
    export_bytes_core, export_core, import_core, import_text_core, ExportBytes, ImportText,
};
pub use functions::functions_core;
pub use prediction::{
    bayesian_core, bootstrap_core, decision_tree_core, real_options_core, scenarios_core,
    tornado_core,
};
pub use probability_seek::{probability_seek, probability_seek_core, SeekGoal, SeekResult};
pub use report::{
    build_report, display_number, render, report_core, Cell, Report, ReportRow, ReportSection,
};
pub use schema::schema_core;
pub use simulate::{simulate_core, SimulateOptions};

// Re-exports for tests (internal functions)
#[cfg(test)]
pub use analysis::{
    calculate_with_override, export_variance_to_excel, export_variance_to_yaml, parse_range,
    print_variance_table, VarianceResult,
};
#[cfg(test)]
pub use upgrade::split_scalars_to_inputs_outputs;

use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::to_15_digits;
use crate::core::{ArrayCalculator, UnitValidator};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::{ColumnValue, ParsedModel};
use crate::writer;
use colored::Colorize;
use std::path::Path;
use std::path::PathBuf;

/// Format a number for display, removing unnecessary decimal places
#[must_use]
pub fn format_number(n: f64) -> String {
    // Round to 6 decimal places for display (sufficient for most financial calculations)
    let rounded = (n * 1e6).round() / 1e6;
    // Format with up to 6 decimal places, removing trailing zeros
    format!("{rounded:.6}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// A column's values as plain JSON (numbers, strings, booleans), for the MCP
/// and JSON results. A number that is not finite becomes null.
fn column_json_values(values: &ColumnValue) -> Vec<serde_json::Value> {
    match values {
        ColumnValue::Number(v) => v
            .iter()
            .map(|n| serde_json::json!(to_15_digits(*n)))
            .collect(),
        ColumnValue::Text(v) | ColumnValue::Date(v) => {
            v.iter().map(|t| serde_json::json!(t)).collect()
        },
        ColumnValue::Boolean(v) => v.iter().map(|b| serde_json::json!(b)).collect(),
        // An empty cell is null (ADR-059)
        ColumnValue::Blanks(v) => v
            .iter()
            .map(|n| {
                n.map_or(serde_json::Value::Null, |n| {
                    serde_json::json!(to_15_digits(n))
                })
            })
            .collect(),
    }
}

/// A column's values as its `format:` shows them (ADR-061); an empty cell
/// is "".
#[must_use]
pub fn formatted_column(values: &ColumnValue, format: &str) -> Vec<String> {
    use crate::core::array_calculator::evaluator::{text::number_format::format_value, Value};
    let show = |value: Value| format_value(&value, format).unwrap_or_default();
    match values {
        ColumnValue::Number(v) => v.iter().map(|n| show(Value::Number(*n))).collect(),
        ColumnValue::Blanks(v) => v
            .iter()
            .map(|n| n.map_or_else(String::new, |n| show(Value::Number(n))))
            .collect(),
        ColumnValue::Text(v) | ColumnValue::Date(v) => {
            v.iter().map(|t| show(Value::Text(t.clone()))).collect()
        },
        ColumnValue::Boolean(v) => v.iter().map(|b| show(Value::Boolean(*b))).collect(),
    }
}

/// A number as a `format:` shows it (ADR-061), or `None` when the format
/// cannot show it. Only how it is printed changes, never the value.
#[must_use]
pub fn number_shown_by(value: f64, format: &str) -> Option<String> {
    use crate::core::array_calculator::evaluator::{text::number_format::format_value, Value};
    format_value(&Value::Number(value), format).ok()
}

/// The `format:` of each scalar of a model that has one (ADR-061).
#[must_use]
pub fn scalar_formats(
    model: &crate::types::ParsedModel,
) -> std::collections::BTreeMap<String, String> {
    model
        .scalars
        .iter()
        .filter_map(|(name, v)| Some((name.clone(), v.metadata.format.clone()?)))
        .collect()
}

/// A column's values for the terminal, all of them and to 15 significant
/// digits, like scalars: `[1, 2, 3]`. Long columns are not cut, because the values are what
/// a reader (or an agent) needs.
fn format_column_values(values: &ColumnValue) -> String {
    let items: Vec<String> = match values {
        ColumnValue::Number(v) => v.iter().map(|n| to_15_digits(*n).to_string()).collect(),
        ColumnValue::Text(v) | ColumnValue::Date(v) => v.iter().map(|t| format!("{t:?}")).collect(),
        ColumnValue::Boolean(v) => v.iter().map(|b| b.to_string().to_uppercase()).collect(),
        ColumnValue::Blanks(v) => v
            .iter()
            .map(|n| n.map_or_else(|| "null".to_string(), |n| to_15_digits(n).to_string()))
            .collect(),
    };
    format!("[{}]", items.join(", "))
}

/// Calculate formulas and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, calculation fails,
/// or results cannot be written back to the file.
pub fn calculate_core(
    file: &Path,
    dry_run: bool,
    scenario: Option<&str>,
) -> ForgeResult<results::CalculationResult> {
    let mut model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    // Apply scenario overrides if specified
    if let Some(scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
    }

    // Unit consistency validation
    let unit_validator = UnitValidator::new(&model);
    let unit_warnings: Vec<String> = unit_validator
        .validate()
        .iter()
        .map(ToString::to_string)
        .collect();

    // Calculate
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Build table summaries
    let mut tables = std::collections::BTreeMap::new();
    for (table_name, table) in &result.tables {
        let row_count = table
            .columns
            .values()
            .next()
            .map_or(0, |col| col.values.len());
        tables.insert(
            table_name.clone(),
            results::TableSummary {
                name: table_name.clone(),
                column_count: table.columns.len(),
                row_count,
                // Model order; the `values` map below is keyed by name.
                columns: table
                    .column_names()
                    .into_iter()
                    .filter(|name| table.columns.contains_key(*name))
                    .cloned()
                    .collect(),
                values: table
                    .columns
                    .iter()
                    .map(|(name, column)| (name.clone(), column_json_values(&column.values)))
                    .collect(),
                formatted: table
                    .columns
                    .iter()
                    .filter_map(|(name, column)| {
                        let format = table.format_of(name)?;
                        Some((name.clone(), formatted_column(&column.values, format)))
                    })
                    .collect(),
            },
        );
    }

    // Build scalar map
    let mut scalars = std::collections::BTreeMap::new();
    let mut dates = std::collections::BTreeSet::new();
    let mut formatted = std::collections::BTreeMap::new();
    for (name, var) in &result.scalars {
        scalars.insert(name.clone(), var.value.map(to_15_digits));
        if let Some(text) = var.formatted() {
            formatted.insert(name.clone(), text);
        }
        if var.date_text().is_some() {
            dates.insert(name.clone());
        }
    }

    // Write results if not dry run
    let file_updated = if dry_run {
        false
    } else {
        writer::write_calculated_results(file, &result)? > 0
    };

    Ok(results::CalculationResult {
        tables,
        scalars,
        dates,
        formatted,
        unit_warnings,
        file_updated,
        dry_run,
    })
}

/// `calculate` and `validate` are deterministic and cannot evaluate `MC.*` distributions; say so
/// up front instead of failing later with an unknown-function or unknown-column error.
fn reject_distributions(model: &crate::types::ParsedModel) -> ForgeResult<()> {
    let mut names: Vec<String> = model
        .scalars
        .iter()
        .filter(|(_, v)| v.formula.as_deref().is_some_and(|f| f.contains("MC.")))
        .map(|(name, _)| name.clone())
        .collect();
    for (table_name, table) in &model.tables {
        names.extend(
            table
                .row_formulas
                .iter()
                .filter(|(_, f)| f.contains("MC."))
                .map(|(column, _)| format!("{table_name}.{column}")),
        );
    }
    if names.is_empty() {
        return Ok(());
    }
    names.sort_unstable();
    Err(ForgeError::Validation(format!(
        "{} use MC.* distributions, which calculate and validate cannot evaluate. Run `forge simulate` on this file instead.",
        names.join(", ")
    )))
}

/// Print the unit warnings of a model, if it has any.
fn print_unit_warnings(c: &mut Console<'_>, model: &ParsedModel) {
    let unit_validator = UnitValidator::new(model);
    let unit_warnings = unit_validator.validate();
    if !unit_warnings.is_empty() {
        outln!(c, "{}", "⚠️  Unit Consistency Warnings:".yellow().bold());
        for warning in &unit_warnings {
            outln!(c, "   {}", warning.to_string().yellow());
        }
        outln!(c);
    }
}

/// Print each table's columns, in the order of the file, which is the same on
/// every run.
fn print_table_results(c: &mut Console<'_>, result: &ParsedModel) {
    // same on every run
    for (table_name, table) in &result.tables {
        outln!(c, "   📊 Table: {}", table_name.bright_blue().bold());
        let calculated = table
            .column_names()
            .into_iter()
            .filter_map(|name| table.columns.get(name).map(|column| (name, column)));
        for (col_name, column) in calculated {
            let raw = format_column_values(&column.values);
            // With a format (ADR-061): the values as it shows them, then the
            // full values underneath, which are what the file holds
            match table.format_of(col_name) {
                Some(format) => {
                    // `;` between values, since a format may put commas in them
                    let shown = formatted_column(&column.values, format).join("; ");
                    outln!(c, "      {} = [{shown}]", col_name.cyan());
                    outln!(
                        c,
                        "      {}   {}",
                        " ".repeat(col_name.chars().count()),
                        raw.dimmed()
                    );
                },
                None => outln!(c, "      {} = {raw}", col_name.cyan()),
            }
        }
    }
}

/// Print each scalar's value, with the formatted one beside it where it differs.
fn print_scalar_results(c: &mut Console<'_>, result: &ParsedModel) {
    if !result.scalars.is_empty() {
        outln!(c, "\n   📐 Scalars:");
        for (name, var) in &result.scalars {
            if let Some(value) = var.value {
                let full = var
                    .date_text()
                    .unwrap_or_else(|| to_15_digits(value).to_string());
                // With a format (ADR-061): as it shows the value, then the
                // full value when the two differ
                match var.formatted().filter(|f| *f != full) {
                    Some(shown) => outln!(
                        c,
                        "      {} = {}  {}",
                        name.bright_blue(),
                        shown.bold(),
                        format!("({full})").dimmed()
                    ),
                    None => outln!(c, "      {} = {}", name.bright_blue(), full.bold()),
                }
            }
        }
    }
}

/// Write the results into the file, or say that a dry run wrote nothing.
fn write_back(
    c: &mut Console<'_>,
    file: &Path,
    result: &ParsedModel,
    dry_run: bool,
) -> ForgeResult<()> {
    if dry_run {
        outln!(c, "{}", "📋 Dry run complete - no changes written".yellow());
    } else {
        match writer::write_calculated_results(file, result)? {
            0 => {
                outln!(
                    c,
                    "{}",
                    format!(
                        "✅ {} is up to date (no stored result changed)",
                        file.display()
                    )
                    .green()
                );
            },
            n => {
                outln!(
                    c,
                    "{}",
                    format!(
                        "💾 {n} value{} updated in {}",
                        if n == 1 { "" } else { "s" },
                        file.display()
                    )
                    .bold()
                    .green()
                );
                outln!(
                    c,
                    "{}",
                    format!("   Backup saved to {}.bak", file.display()).dimmed()
                );
            },
        }
    }
    Ok(())
}

/// Execute the calculate command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, calculation fails,
/// or results cannot be written back to the file.
pub fn calculate(
    c: &mut Console<'_>,
    file: &Path,
    dry_run: bool,
    verbose: bool,
    scenario: Option<&str>,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Calculating formulas".bold().green());
    outln!(c, "   File: {}", file.display());
    if let Some(s) = scenario {
        outln!(c, "   Scenario: {}", s.bright_yellow().bold());
    }
    outln!(c);

    if dry_run {
        outln!(
            c,
            "{}",
            "📋 DRY RUN MODE - No changes will be written\n".yellow()
        );
    }

    // Parse file
    if verbose {
        outln!(c, "{}", "📖 Parsing YAML file...".cyan());
    }

    let mut model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if verbose {
        outln!(
            c,
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
        if !model.scenarios.is_empty() {
            outln!(
                c,
                "   Found {} scenarios: {:?}",
                model.scenarios.len(),
                model.scenario_names()
            );
        }
        outln!(c);
    }

    // Apply scenario overrides if specified
    if let Some(scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
        if verbose {
            outln!(
                c,
                "{}",
                format!("📊 Applied scenario: {scenario_name}").cyan()
            );
        }
    }

    print_unit_warnings(c, &model);

    // Calculate using ArrayCalculator
    if verbose {
        outln!(c, "{}", "🧮 Calculating tables and scalars...".cyan());
    }

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Display results
    outln!(c, "{}", "✅ Calculation Results:".bold().green());

    print_table_results(c, &result);

    print_scalar_results(c, &result);
    outln!(c);

    write_back(c, file, &result, dry_run)?;

    Ok(())
}

/// Validate a single file and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed or calculation fails.
pub fn validate_core(file: &Path) -> ForgeResult<results::ValidationResult> {
    const TOLERANCE: f64 = 0.0001;

    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let calculator = ArrayCalculator::new(model.clone());
    let calculated = calculator.calculate_all()?;

    let mut mismatches = Vec::new();
    for (var_name, var) in &calculated.scalars {
        if let Some(calculated_value) = var.value {
            if let Some(original) = model.scalars.get(var_name) {
                if let Some(current_value) = original.value {
                    let diff = (current_value - calculated_value).abs();
                    if diff > TOLERANCE {
                        mismatches.push(results::ValidationMismatch {
                            name: var_name.clone(),
                            current_value,
                            expected_value: calculated_value,
                            diff,
                        });
                    }
                }
            }
        }
    }

    let scalars_valid = mismatches.is_empty();
    Ok(results::ValidationResult {
        tables_valid: true,
        scalars_valid,
        table_count,
        scalar_count,
        mismatches,
    })
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

    let mut all_passed = true;
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
                all_passed = false;
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

    if all_passed {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!(
            "{} file(s) failed validation",
            failed_files.len()
        )))
    }
}

/// Validate a single file
fn validate_single_file(c: &mut Console<'_>, file: &std::path::Path) -> ForgeResult<()> {
    const TOLERANCE: f64 = 0.0001; // Floating point comparison tolerance

    // Parse YAML file
    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if model.tables.is_empty() && model.scalars.is_empty() {
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

    // Calculate what values SHOULD be based on formulas
    let calculator = ArrayCalculator::new(model.clone());
    let calculated = match calculator.calculate_all() {
        Ok(vals) => vals,
        Err(e) => {
            outln!(
                c,
                "\n{}",
                format!("❌ Formula validation failed: {e}").bold().red()
            );
            return Err(e);
        },
    };

    // Compare calculated values vs. current values in file. A formula with no
    // stored value (never calculated) has nothing to compare.
    let mut mismatches = Vec::new();
    let unstored = model
        .scalars
        .values()
        .filter(|v| v.formula.is_some() && v.value.is_none())
        .count();

    for (var_name, var) in &calculated.scalars {
        if let Some(calculated_value) = var.value {
            if let Some(original) = model.scalars.get(var_name) {
                if let Some(current_value) = original.value {
                    // Check if values match within tolerance
                    let diff = (current_value - calculated_value).abs();
                    if diff > TOLERANCE {
                        mismatches.push((var_name.clone(), current_value, calculated_value, diff));
                    }
                }
            }
        }
    }

    report_validation(c, &mismatches, unstored)
}

/// What `forge validate` found: every stored value matches its formula, or
/// each one that does not, with what it should be.
fn report_validation(
    c: &mut Console<'_>,
    mismatches: &[(String, f64, f64, f64)],
    unstored: usize,
) -> ForgeResult<()> {
    outln!(c);
    if mismatches.is_empty() {
        outln!(c, "{}", "✅ All tables are valid!".bold().green());
        if unstored == 0 {
            outln!(
                c,
                "{}",
                "✅ All scalar values match their formulas!".bold().green()
            );
        } else {
            outln!(c, "{}", "✅ The formulas are valid.".bold().green());
            outln!(c, "   {unstored} formula(s) have no stored value yet; run 'forge calculate' to store them"
            );
        }
        Ok(())
    } else {
        outln!(
            c,
            "{}",
            format!("❌ Found {} value mismatches!", mismatches.len())
                .bold()
                .red()
        );
        outln!(
            c,
            "{}",
            "   The formulas are valid; the stored values are out of date.\n".yellow()
        );

        for (name, current, expected, diff) in mismatches {
            outln!(c, "   {}", name.bright_blue().bold());
            // Format numbers with reasonable precision (remove trailing zeros)
            outln!(c, "      Current:  {}", format_number(*current).red());
            outln!(c, "      Expected: {}", format_number(*expected).green());
            outln!(c, "      Diff:     {}", format!("{diff:.6}").yellow());
            outln!(c);
        }

        outln!(
            c,
            "{}",
            "💡 Run 'forge calculate' to update values".bold().yellow()
        );

        Err(crate::error::ForgeError::Validation(
            "Stored values are out of date (the formulas are valid): run forge calculate"
                .to_string(),
        ))
    }
}

/// Apply scenario overrides to the model
///
/// # Errors
///
/// Returns an error if the named scenario does not exist in the model.
pub fn apply_scenario(
    model: &mut crate::types::ParsedModel,
    scenario_name: &str,
) -> ForgeResult<()> {
    let scenario = model.scenarios.get(scenario_name).ok_or_else(|| {
        let available: Vec<_> = model.scenarios.keys().collect();
        ForgeError::Validation(format!(
            "Scenario '{scenario_name}' not found. Available scenarios: {available:?}"
        ))
    })?;

    // Clone the overrides to avoid borrow checker issues
    let overrides = scenario.overrides.clone();

    // Apply overrides to scalars (resolving grouped names like "rate" → "assumptions.rate",
    // and `@namespace.name` to a scalar of an included file)
    let dates = scenario.dates.clone();
    for (var_name, override_value) in &overrides {
        if dates.contains(var_name) {
            model.override_date(var_name, *override_value)
        } else {
            model.override_scalar(var_name, Some(*override_value), None)
        }
        .map_err(|e| ForgeError::Validation(format!("Scenario '{scenario_name}': {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod calculate_tests {
    use super::*;
    use crate::cli::console::capture;
    use std::io::Write;

    #[test]
    fn calculate_rejects_mc_distributions_with_a_clear_message() {
        let mut file = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            "_forge_version: \"5.0.0\"\nassumptions:\n  growth:\n    value: null\n    formula: \"=MC.Normal(0.15, 0.05)\"\n  doubled:\n    value: null\n    formula: \"=assumptions.growth * 2\"\n"
        )
        .unwrap();
        let err = capture(|c| calculate(c, file.path(), true, false, None))
            .0
            .unwrap_err()
            .to_string();
        assert!(err.contains("assumptions.growth"), "{err}");
        assert!(err.contains("forge simulate"), "{err}");
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
    }

    #[test]
    fn calculate_core_returns_table_values() {
        let mut file = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            "_forge_version: \"5.0.0\"\nt:\n  n: [1, 2, 3]\n  sq: \"=n * n\"\n  label: '=IF(n > 1, \"big\", \"small\")'\n  even: \"=MOD(n, 2) = 0\"\n"
        )
        .unwrap();
        let result = calculate_core(file.path(), true, None).unwrap();
        let values = &result.tables["t"].values;
        assert_eq!(
            values["sq"],
            vec![
                serde_json::json!(1.0),
                serde_json::json!(4.0),
                serde_json::json!(9.0)
            ]
        );
        assert_eq!(
            values["label"],
            vec![
                serde_json::json!("small"),
                serde_json::json!("big"),
                serde_json::json!("big")
            ]
        );
        assert_eq!(
            values["even"],
            vec![
                serde_json::json!(false),
                serde_json::json!(true),
                serde_json::json!(false)
            ]
        );
        // Input columns are included too
        assert_eq!(values["n"].len(), 3);
    }

    #[test]
    fn table_values_serialize_as_plain_json_arrays() {
        let mut file = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            "_forge_version: \"5.0.0\"\nt:\n  n: [1, 2]\n  d: \"=n * 2\"\n"
        )
        .unwrap();
        let result = calculate_core(file.path(), true, None).unwrap();
        let json = serde_json::to_value(&result.tables["t"]).unwrap();
        assert_eq!(json["values"]["d"], serde_json::json!([2.0, 4.0]));
    }

    #[test]
    fn format_column_values_prints_every_value() {
        let long = ColumnValue::Number((1..=60).map(f64::from).collect());
        let printed = format_column_values(&long);
        assert!(printed.starts_with("[1, 2, "), "{printed}");
        assert!(printed.ends_with(", 59, 60]"), "{printed}");
        assert_eq!(
            format_column_values(&ColumnValue::Text(vec!["a".into(), "b".into()])),
            "[\"a\", \"b\"]"
        );
        assert_eq!(
            format_column_values(&ColumnValue::Boolean(vec![true, false])),
            "[TRUE, FALSE]"
        );
    }

    #[test]
    fn quota_forecast_example_validates() {
        validate_core(std::path::Path::new("examples/quota_forecast.yaml"))
            .unwrap()
            .scalars_valid
            .then_some(())
            .expect("stored values match the formulas");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod what_if {
        //! Scenarios and goal seek must change the numbers, find them, or fail; never quietly do neither.

        use super::{calculate_core, scenarios_core};
        use std::path::PathBuf;

        const COSTS: &str =
            "_forge_version: \"5.0.0\"\nprices:\n  flour:\n    value: 2\n    formula: null\n";

        /// A model where profit = units * (price - @costs.prices.flour).
        fn write_model(dir: &std::path::Path, override_line: &str) -> PathBuf {
            std::fs::write(dir.join("costs.yaml"), COSTS).unwrap();
            let model = format!(
                "_forge_version: \"5.0.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\n\
         inputs:\n  price:\n    value: 5\n    formula: null\n  units:\n    value: 100\n    formula: null\n\
         out:\n  profit:\n    value: null\n    formula: \"=inputs.units * (inputs.price - @costs.prices.flour)\"\n\
         scenarios:\n  base:\n    probability: 0.5\n    scalars: {{}}\n  shock:\n    probability: 0.5\n    scalars:\n      {override_line}\n"
            );
            let path = dir.join("model.yaml");
            std::fs::write(&path, model).unwrap();
            path
        }

        fn shock_profit(results: &crate::scenarios::ScenarioResults) -> f64 {
            results
                .scenarios
                .iter()
                .find(|s| s.name == "shock")
                .unwrap()
                .scalars["out.profit"]
        }

        #[test]
        fn test_misspelled_override_is_an_error_with_a_suggestion() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "prise: 6");
            let err = scenarios_core(&path, None).unwrap_err().to_string();
            assert!(err.contains("'prise' does not name a scalar"), "{err}");
            assert!(err.contains("Did you mean 'inputs.price'"), "{err}");
            // `forge calculate --scenario` takes the same path
            let err = calculate_core(&path, true, Some("shock"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("'prise' does not name a scalar"), "{err}");
        }

        #[test]
        fn test_bare_and_dotted_overrides_change_the_result() {
            let dir = tempfile::tempdir().unwrap();
            for line in ["price: 6", "inputs.price: 6"] {
                let path = write_model(dir.path(), line);
                let results = scenarios_core(&path, None).unwrap();
                assert!((shock_profit(&results) - 400.0).abs() < 1e-9, "{line}");
            }
        }

        #[test]
        fn test_override_of_an_included_value() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "\"@costs.prices.flour\": 3");
            let results = scenarios_core(&path, None).unwrap();
            assert!((shock_profit(&results) - 200.0).abs() < 1e-9);
            let calc = calculate_core(&path, true, Some("shock")).unwrap();
            assert!((calc.scalars["out.profit"].unwrap() - 200.0).abs() < 1e-9);
        }

        #[test]
        fn test_override_of_an_unknown_include_or_name_is_an_error() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "\"@rates.prices.flour\": 3");
            let err = scenarios_core(&path, None).unwrap_err().to_string();
            assert!(
                err.contains("no `_includes` entry with `as: rates`"),
                "{err}"
            );
            let path = write_model(dir.path(), "\"@costs.prices.flower\": 3");
            let err = scenarios_core(&path, None).unwrap_err().to_string();
            assert!(err.contains("Did you mean 'prices.flour'"), "{err}");
        }

        #[test]
        fn test_outputs_ranges_and_filter() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "price: 1");
            let results = scenarios_core(&path, Some("shock")).unwrap();
            assert_eq!(results.scenarios.len(), 1);
            assert_eq!(results.scenarios[0].name, "shock");
            // base 300, shock -100: half the probability is positive
            assert!((results.probability_positive["out.profit"] - 0.5).abs() < 1e-9);
            assert_eq!(results.ranges["out.profit"], (-100.0, 300.0));
            let err = scenarios_core(&path, Some("nope")).unwrap_err().to_string();
            assert!(err.contains("Scenario 'nope' not found"), "{err}");
        }

        /// A steep output (each unit of input moves it by 25,000) used to stop with
        /// `converged: false` when the answer was already within a cent: the loop
        /// compared the width of the input range with a tolerance meant for the output.
        #[test]
        fn test_goal_seek_converges_on_a_steep_output() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("steep.yaml");
            std::fs::write(
                &path,
                "_forge_version: \"5.0.0\"\ninputs:\n  price:\n    value: 16\n    formula: null\n\
         out:\n  profit:\n    value: null\n    formula: \"=inputs.price * 25000 - 399000\"\n",
            )
            .unwrap();
            for target in [0.0, 60_000.0] {
                let r = super::goal_seek_core(
                    &path,
                    "out.profit",
                    target,
                    "inputs.price",
                    (Some(10.0), Some(40.0)),
                    0.0001,
                )
                .unwrap();
                assert!(r.converged, "{target}: {}", r.error);
                assert!(r.error <= 0.0001);
                assert!((r.solution - (target + 399_000.0) / 25_000.0).abs() < 1e-8);
            }
        }
    }

    mod validate_gaps {
        //! Two gaps `forge validate` had (T1-VALIDATE-GAPS): a table whose columns
        //! have different lengths passed `validate` when no formula read it, and a scalar name
        //! with a typo was reported as an unknown column with no suggestion.

        use crate::cli::forge;
        use std::path::Path;

        const RAGGED: &str = r#"_forge_version: "5.1.0"
sales:
  month: [Jan, Feb, Mar]
  units: [10, 20]
"#;

        const TYPO: &str = r#"_forge_version: "5.1.0"
a:
  prce: 100
b:
  total:
    formula: "=a.price * 2"
"#;

        fn write(dir: &Path, name: &str, text: &str) -> String {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("write");
            path.to_string_lossy().into_owned()
        }

        #[test]
        fn validate_refuses_a_table_with_columns_of_different_lengths() {
            let dir = tempfile::tempdir().expect("temp dir");
            let file = write(dir.path(), "ragged.yaml", RAGGED);
            let out = forge(&["validate", &file]);
            let text = out.both();
            assert_eq!(out.code, 1, "{text}");
            assert!(!text.contains("All tables are valid"), "{text}");
            assert!(text.contains("sales"), "names the table: {text}");
            assert!(
                text.contains("units") && text.contains("2 rows") && text.contains("3 rows"),
                "names the column and both lengths: {text}"
            );
        }

        #[test]
        fn export_writes_a_table_with_columns_of_different_lengths_and_warns() {
            // Columns of different lengths are ranges in the workbook (the Gnumeric
            // suite's fixtures rely on it), so export warns instead of refusing.
            let dir = tempfile::tempdir().expect("temp dir");
            let file = write(dir.path(), "ragged.yaml", RAGGED);
            let output = dir.path().join("out.xlsx");
            let out = forge(&["export", &file, output.to_str().expect("utf-8")]);
            let text = out.both();
            assert!(out.success(), "{text}");
            assert!(output.exists(), "the workbook is written");
            assert!(text.contains("units") && text.contains("2 rows"), "{text}");
        }

        #[test]
        fn a_misspelled_scalar_suggests_the_closest_name() {
            let dir = tempfile::tempdir().expect("temp dir");
            let file = write(dir.path(), "typo.yaml", TYPO);
            let out = forge(&["validate", &file]);
            let text = out.both();
            assert_eq!(out.code, 1, "{text}");
            assert!(text.contains("a.price"), "{text}");
            assert!(
                text.contains("a.prce"),
                "suggests the name that exists: {text}"
            );
            assert!(
                !text.contains("Unknown column"),
                "it is a name, not a column: {text}"
            );
        }
    }

    mod printed_noise {
        //! The bagel dry run printed `year.worst_month_profit = -7935.533406400002`:
        //! binary rounding noise in the last digits. Numbers Forge prints have at
        //! most 15 significant digits, as in Excel (`core::precision`).

        use crate::cli::forge;

        #[test]
        fn the_bagel_dry_run_prints_the_worst_month_without_noise() {
            let ran = forge(&["calculate", "examples/bagel/bagel.yaml", "--dry-run"]);
            assert!(ran.success(), "{}", ran.both());
            assert!(
                ran.out
                    .contains("year.worst_month_profit = -7935.5334064\n"),
                "{}",
                ran.out
            );
            assert!(!ran.out.contains("7935.533406400002"), "{}", ran.out);
        }
    }
}

#[cfg(test)]
mod command_output {
    //! What `calculate` and `validate` print, in the cases the other tests do
    //! not reach: empty cells, units, scenarios, batches, a result already stored.

    use super::*;
    use crate::cli::console::capture;
    use std::fs;

    const GOOD: &str = "_forge_version: \"5.0.0\"\ninputs:\n  a:\n    value: 2\n    formula: null\noutputs:\n  b:\n    value: 4\n    formula: \"=a * 2\"\n";

    fn model(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn an_empty_cell_shows_as_blank_null_or_nothing() {
        let column = ColumnValue::Blanks(vec![Some(1.5), None]);
        assert_eq!(format_column_values(&column), "[1.5, null]");
        assert_eq!(formatted_column(&column, "0.00"), ["1.50", ""]);
        assert_eq!(
            column_json_values(&column),
            [serde_json::json!(1.5), serde_json::Value::Null]
        );
    }

    #[test]
    fn booleans_are_shown_by_a_format_as_they_are() {
        let column = ColumnValue::Boolean(vec![true, false]);
        assert_eq!(formatted_column(&column, "General"), ["TRUE", "FALSE"]);
    }

    #[test]
    fn units_that_do_not_add_are_warned_about() {
        let mut model = ParsedModel::new();
        let mut table = crate::types::Table::new("data".to_string());
        for (name, unit) in [("revenue", "CAD"), ("item_count", "units")] {
            let mut column =
                crate::types::Column::new(name.to_string(), ColumnValue::Number(vec![1.0]));
            column.metadata.unit = Some(unit.to_string());
            table.add_column(column);
        }
        table
            .row_formulas
            .insert("bad".to_string(), "=revenue + item_count".to_string());
        model.add_table(table);
        let ((), captured) = capture(|c| print_unit_warnings(c, &model));
        assert!(
            captured.out.contains("Unit Consistency Warnings"),
            "{}",
            captured.out
        );

        let ((), captured) = capture(|c| print_unit_warnings(c, &ParsedModel::new()));
        assert_eq!(captured.out, "");
    }

    #[test]
    fn calculating_says_what_verbose_and_a_scenario_ask_for() {
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\nrate: 2\ndouble: {formula: \"=rate * 2\"}\nscenarios:\n  high:\n    probability: 1\n    scalars:\n      rate: 5\n",
        );
        let (result, captured) = capture(|c| calculate(c, &file, true, true, Some("high")));
        result.unwrap();
        for line in [
            "Scenario: high",
            "Parsing YAML file",
            "Found 0 tables, 2 scalars",
            "Found 1 scenarios",
            "Applied scenario: high",
            "Calculating tables and scalars",
            "double = 10",
        ] {
            assert!(captured.out.contains(line), "{line} in {}", captured.out);
        }
        let (result, captured) = capture(|c| calculate(c, &file, true, false, Some("high")));
        result.unwrap();
        assert!(
            !captured.out.contains("Applied scenario"),
            "{}",
            captured.out
        );
        let (result, _) = capture(|c| calculate(c, &file, true, false, Some("absent")));
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn calculating_twice_finds_nothing_to_update() {
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "m.yaml",
            &GOOD.replace("value: 4", "value: null"),
        );
        let (result, captured) = capture(|c| calculate(c, &file, false, false, None));
        result.unwrap();
        assert!(captured.out.contains("1 value updated"), "{}", captured.out);
        let (result, captured) = capture(|c| calculate(c, &file, false, false, None));
        result.unwrap();
        assert!(captured.out.contains("is up to date"), "{}", captured.out);
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
    fn text_columns_print_as_they_are_and_an_empty_scalar_is_left_out() {
        let text = ColumnValue::Text(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(format_column_values(&text), "[\"a\", \"b\"]");
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "t.yaml",
            "_forge_version: \"5.1.0\"\nt:\n  n: [1, 2]\n  label: '=IF(n > 1, \"big\", \"small\")'\nempty: {value: null, formula: null}\n",
        );
        let (result, captured) = capture(|c| calculate(c, &file, true, true, None));
        result.unwrap();
        assert!(
            captured.out.contains("label = [\"small\", \"big\"]"),
            "{}",
            captured.out
        );
        assert!(
            !captured.out.contains("Found 0 scenarios"),
            "{}",
            captured.out
        );
    }

    #[test]
    fn results_that_cannot_be_written_back_are_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "m.yaml",
            &GOOD.replace("value: 4", "value: null"),
        );
        // A folder where the backup goes
        fs::create_dir(dir.path().join("m.yaml.bak")).unwrap();
        let (result, _) = capture(|c| calculate(c, &file, false, false, None));
        assert!(result.is_err());
        assert!(calculate_core(&file, false, None).is_err());
        assert!(fs::read_to_string(&file).unwrap().contains("value: null"));
    }
}
