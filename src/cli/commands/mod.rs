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
// forge edit and forge_edit, on files on disk (ADR-075); forge-wasm calls
// crate::edit itself
#[cfg(feature = "cli")]
mod edit;
mod examples;
// Export and import need Excel (feature `excel`); printing them and base64
// belong to the command line (feature `cli`)
#[cfg(feature = "excel")]
mod excel_io;
mod functions;
mod game;
mod names;
mod plan;
pub(crate) use names::{input_name, scalar};
pub use plan::calculate_plan_core;
#[cfg(test)]
pub(crate) use plan::LAUNCH;
mod prediction;
mod probability_seek;
mod report;
pub mod results;
mod schema;
mod simulate;
mod templates;
// Self-update downloads releases (feature `cli`)
#[cfg(feature = "cli")]
mod update;
mod upgrade;
mod validate;
// Watch mode brings the file watcher (feature `cli`)
#[cfg(feature = "cli")]
mod watch;

// Re-exports
pub use analysis::{break_even, goal_seek, sensitivity, sensitivity_in_format, variance};
pub use audit::audit;
pub use compare::{compare, comparison_json, render_comparison, terminal_width};
#[cfg(feature = "cli")]
pub use edit::{edit, edit_core, EditArgs, EditResult};
#[cfg(feature = "cli")]
pub use examples::examples;
#[cfg(feature = "cli")]
pub use excel_io::{export, import};
pub use functions::functions;
#[cfg(feature = "cli")]
pub use game::GameArgs;
pub use game::{game, game_core};
pub use prediction::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};
pub use report::{report, ReportFormat, ReportOptions};
pub use schema::schema;
pub use simulate::simulate;
pub use templates::templates;
#[cfg(feature = "cli")]
pub use update::update;
pub use upgrade::upgrade;
pub use validate::{validate, validate_core, validate_core_with};
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
pub use templates::templates_core;

// Re-exports for tests (internal functions)
#[cfg(test)]
pub use analysis::{
    calculate_with_override, export_variance_to_excel, export_variance_to_yaml, parse_range,
    print_variance_table,
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

/// A column's values for the terminal, as [`calculate_core`] gives them
/// (already to 15 significant digits): `[1, 2, 3]`, texts and dates quoted,
/// booleans as `TRUE`, an empty cell as `null`. Long columns are not cut,
/// because the values are what a reader (or an agent) needs.
fn format_column_values(values: &[serde_json::Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|value| match value {
            serde_json::Value::String(t) => format!("{t:?}"),
            serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
            other => other
                .as_f64()
                .map_or_else(|| "null".to_string(), |n| n.to_string()),
        })
        .collect();
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
    let read = model_read(&model);
    reject_distributions(&model)?;

    // Apply scenario overrides if specified
    if let Some(scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
    }
    calculated(
        file,
        model,
        dry_run,
        scenario,
        read,
        std::collections::BTreeMap::new(),
    )
}

/// What `model` holds as read from its file, before a scenario or the plan
/// changes it.
pub(super) fn model_read(model: &ParsedModel) -> results::ModelRead {
    results::ModelRead {
        tables: model.tables.len(),
        scalars: model.scalars.len(),
        scenarios: model.scenario_names().into_iter().cloned().collect(),
    }
}

/// Where the units of `model` do not agree, one line each.
fn unit_warnings(model: &ParsedModel) -> Vec<String> {
    UnitValidator::new(model)
        .validate()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// `model` calculated, as [`calculate_core`] answers it: `read` is the file
/// as read, and `at_plan` the plan figures it was set to, if any.
pub(super) fn calculated(
    file: &Path,
    model: crate::types::ParsedModel,
    dry_run: bool,
    scenario: Option<&str>,
    read: results::ModelRead,
    at_plan: std::collections::BTreeMap<String, f64>,
) -> ForgeResult<results::CalculationResult> {
    let unit_warnings = unit_warnings(&model);

    // What this run should give (ADR-074): the file's, or the scenario's
    let expect = scenario
        .and_then(|name| model.scenarios.get(name))
        .map_or_else(|| model.expect.clone(), |s| s.expect.clone());

    // Calculate
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;
    let expectations = validate::checks(scenario, &expect, &result);

    // Build table summaries, in the model's order
    let mut tables = indexmap::IndexMap::new();
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

    // Build scalar map, in the model's order
    let mut scalars = indexmap::IndexMap::new();
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

    // Write results if not dry run, and never a scenario's (ADR-074)
    let values_updated = if dry_run || scenario.is_some() {
        0
    } else {
        writer::write_calculated_results(file, &result)?
    };

    Ok(results::CalculationResult {
        tables,
        scalars,
        dates,
        formatted,
        unit_warnings,
        file_updated: values_updated > 0,
        values_updated,
        dry_run,
        read,
        at_plan,
        expectations,
    })
}

/// `calculate` and `validate` are deterministic and cannot evaluate `MC.*` distributions; say so
/// up front instead of failing later with an unknown-function or unknown-column error.
fn reject_distributions(model: &crate::types::ParsedModel) -> ForgeResult<()> {
    refuse_distributions(model, "calculate and validate", "`forge calculate --plan`")
}

/// The check of [`reject_distributions`] for `command`, a what-if command
/// that calculates the model as calculate does, named in the message so that
/// it says which command cannot run the file; its `--plan` runs it at plan
/// (ADR-076, addendum of 2026-11-26).
fn reject_distributions_in(model: &crate::types::ParsedModel, command: &str) -> ForgeResult<()> {
    refuse_distributions(
        model,
        command,
        "the same command with `--plan` (`plan: true` over MCP and REST)",
    )
}

/// `model` refused if it has an uncertain input: `command` cannot evaluate
/// it, and `forge simulate` or `plan_with` can.
fn refuse_distributions(
    model: &crate::types::ParsedModel,
    command: &str,
    plan_with: &str,
) -> ForgeResult<()> {
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
        "{} use MC.* distributions, which {command} cannot evaluate. Run `forge simulate` on this file instead, or {plan_with} for its plan.",
        names.join(", ")
    )))
}

/// Print the unit warnings of a calculation, if it has any.
fn print_unit_warnings(c: &mut Console<'_>, unit_warnings: &[String]) {
    if !unit_warnings.is_empty() {
        outln!(c, "{}", "⚠️  Unit Consistency Warnings:".yellow().bold());
        for warning in unit_warnings {
            outln!(c, "   {}", warning.yellow());
        }
        outln!(c);
    }
}

/// Print each table's columns, in the order of the file, which is the same on
/// every run.
fn print_table_results(c: &mut Console<'_>, result: &results::CalculationResult) {
    for (table_name, table) in &result.tables {
        outln!(c, "   📊 Table: {}", table_name.bright_blue().bold());
        let calculated = table
            .columns
            .iter()
            .filter_map(|name| table.values.get(name).map(|values| (name, values)));
        for (col_name, values) in calculated {
            let raw = format_column_values(values);
            // With a format (ADR-061): the values as it shows them, then the
            // full values underneath, which are what the file holds
            match table.formatted.get(col_name) {
                Some(formatted) => {
                    // `;` between values, since a format may put commas in them
                    let shown = formatted.join("; ");
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
fn print_scalar_results(c: &mut Console<'_>, result: &results::CalculationResult) {
    if !result.scalars.is_empty() {
        outln!(c, "\n   📐 Scalars:");
        for name in result.scalars.keys() {
            if let Some(full) = result.shown(name) {
                // With a format (ADR-061): as it shows the value, then the
                // full value when the two differ
                match result.formatted.get(name).filter(|f| **f != full) {
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

/// What the run wrote into the file, or that a dry run wrote nothing.
fn print_write_back(c: &mut Console<'_>, file: &Path, result: &results::CalculationResult) {
    if result.dry_run {
        outln!(c, "{}", "📋 Dry run complete - no changes written".yellow());
        return;
    }
    match result.values_updated {
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

/// What `--verbose` says the file holds.
fn print_model_read(c: &mut Console<'_>, read: &results::ModelRead) {
    outln!(
        c,
        "   Found {} tables, {} scalars",
        read.tables,
        read.scalars
    );
    if !read.scenarios.is_empty() {
        outln!(
            c,
            "   Found {} scenarios: {:?}",
            read.scenarios.len(),
            read.scenarios
        );
    }
    outln!(c);
}

/// Execute the calculate command: [`calculate_core`], or
/// [`calculate_plan_core`] with `plan`, and its result on the console.
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
    plan: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Calculating formulas".bold().green());
    outln!(c, "   File: {}", file.display());
    if let Some(s) = scenario {
        outln!(c, "   Scenario: {}", s.bright_yellow().bold());
    }
    outln!(c);

    if plan {
        plan::print_plan_banner(c);
    } else if dry_run {
        outln!(
            c,
            "{}",
            "📋 DRY RUN MODE - No changes will be written\n".yellow()
        );
    }

    if verbose {
        outln!(c, "{}", "📖 Parsing YAML file...".cyan());
    }
    let result = if plan {
        calculate_plan_core(file, scenario)?
    } else {
        calculate_core(file, dry_run, scenario)?
    };

    if verbose {
        print_model_read(c, &result.read);
        if let Some(scenario_name) = scenario {
            outln!(
                c,
                "{}",
                format!("📊 Applied scenario: {scenario_name}").cyan()
            );
        }
    }
    if plan {
        plan::print_at_plan(c, &result.at_plan);
    }
    print_unit_warnings(c, &result.unit_warnings);
    if verbose {
        outln!(c, "{}", "🧮 Calculating tables and scalars...".cyan());
    }

    // Display results
    outln!(c, "{}", "✅ Calculation Results:".bold().green());
    print_table_results(c, &result);
    print_scalar_results(c, &result);
    outln!(c);

    // At plan, or under a scenario, the results are not the file's: writing
    // them gave stored values the file does not give (ADR-074 question 5,
    // ADR-076)
    match scenario {
        _ if plan => {},
        Some(name) if !dry_run => outln!(
            c,
            "{}",
            format!(
                "📋 Under a scenario, nothing is written: the stored values are the file's own. \
                 To state what this scenario gives, list it under scenarios.{name}.expect"
            )
            .yellow()
        ),
        _ => print_write_back(c, file, &result),
    }

    Ok(())
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
        let mut available: Vec<_> = model.scenarios.keys().collect();
        available.sort_unstable();
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

/// A model whose input is an `MC.*` distribution, with weighted scenarios
/// and a tornado section, given to `forge` with these arguments, `FILE`
/// standing for its path: what each what-if command prints for it
/// (roadmap E-MC-MESSAGE).
#[cfg(test)]
pub(crate) fn forge_on_distributions(args: &[&str]) -> crate::cli::Ran {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("launch.yaml");
    std::fs::write(
        &path,
        "_forge_version: \"5.1.0\"\nassumptions:\n  price:\n    value: 20\n    formula: \"=MC.Triangular(15, 20, 25)\"\n  units:\n    value: 100\noutputs:\n  profit:\n    value: 2000\n    formula: \"=assumptions.price * assumptions.units\"\nscenarios:\n  low:\n    probability: 0.5\n    scalars:\n      assumptions.units: 50\n  high:\n    probability: 0.5\n    scalars:\n      assumptions.units: 150\ntornado:\n  output: outputs.profit\n  inputs:\n    - name: assumptions.units\n      low: 50\n      high: 150\n",
    )
    .expect("model written");
    let file = path.to_string_lossy().to_string();
    let args: Vec<&str> = args
        .iter()
        .map(|a| if *a == "FILE" { file.as_str() } else { a })
        .collect();
    crate::cli::forge(&args)
}

/// That `forge <args>` on [`forge_on_distributions`]'s model says which
/// input is a distribution, that `command` cannot evaluate it, and what to
/// run instead, where it used to say the function was unknown.
#[cfg(test)]
pub(crate) fn assert_distributions_refused(args: &[&str], command: &str) {
    let ran = forge_on_distributions(args);
    let said = ran.both();
    assert!(
        said.contains(&format!(
            "assumptions.price use MC.* distributions, which {command} cannot evaluate. Run `forge simulate` on this file instead, or the same command with `--plan` (`plan: true` over MCP and REST) for its plan."
        )),
        "{said}"
    );
    assert!(!said.contains("Unknown function"), "{said}");
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
        let err = capture(|c| calculate(c, file.path(), true, false, None, false))
            .0
            .unwrap_err()
            .to_string();
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
        let printed = format_column_values(&column_json_values(&long));
        assert!(printed.starts_with("[1, 2, "), "{printed}");
        assert!(printed.ends_with(", 59, 60]"), "{printed}");
        assert_eq!(
            format_column_values(&column_json_values(&ColumnValue::Text(vec![
                "a".into(),
                "b".into()
            ]))),
            "[\"a\", \"b\"]"
        );
        assert_eq!(
            format_column_values(&column_json_values(&ColumnValue::Boolean(vec![
                true, false
            ]))),
            "[TRUE, FALSE]"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod gaps {
        //! A column of text or dates, a scalar with no value, and what
        //! `validate` says when the model cannot be read or calculated.

        use super::*;
        use crate::cli::console::capture;
        use crate::cli::forge;

        fn model(dir: &tempfile::TempDir, text: &str) -> String {
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, text).unwrap();
            path.to_string_lossy().into_owned()
        }

        #[test]
        fn columns_are_listed_as_each_kind_writes_its_values() {
            let text = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
            assert_eq!(
                format_column_values(&column_json_values(&ColumnValue::Text(text(&["a", "b"])))),
                "[\"a\", \"b\"]"
            );
            assert_eq!(
                format_column_values(&column_json_values(&ColumnValue::Date(text(&[
                    "2026-01-31"
                ])))),
                "[\"2026-01-31\"]"
            );
            assert_eq!(
                format_column_values(&column_json_values(&ColumnValue::Boolean(vec![
                    true, false
                ]))),
                "[TRUE, FALSE]"
            );
        }

        /// A scalar with no value is not printed; the others are.
        #[test]
        fn a_scalar_with_no_value_is_left_out_of_the_listing() {
            let result = |scalars| -> results::CalculationResult {
                serde_json::from_value(serde_json::json!({
                    "tables": {}, "scalars": scalars, "unit_warnings": [],
                    "file_updated": false, "dry_run": true,
                }))
                .unwrap()
            };
            let some = result(serde_json::json!({"s.x": 2, "s.gap": null}));
            let ((), printed) = capture(|c| print_scalar_results(c, &some));
            assert!(printed.out.contains("s.x = 2"), "{printed:?}");
            assert!(!printed.out.contains("s.gap"), "{printed:?}");
            // And none at all prints nothing
            let none = result(serde_json::json!({}));
            let ((), printed) = capture(|c| print_scalar_results(c, &none));
            assert_eq!(printed.out, "");
        }

        /// A formula never calculated has no stored value to check against,
        /// so the file fails until `forge calculate` stores it; what cannot
        /// be read or calculated is an error, from the library and from the
        /// command.
        #[test]
        fn validate_checks_what_is_stored_and_refuses_the_rest() {
            let dir = tempfile::tempdir().unwrap();
            let unstored = model(
                &dir,
                "_forge_version: \"5.1.0\"\ns:\n  x: 2\n  y: {formula: \"=x * 2\"}\n",
            );
            let result = validate_core(Path::new(&unstored)).unwrap();
            assert!(result.scalars_valid);
            assert_eq!(result.uncalculated, ["s.y"]);
            assert!(!result.passed);
            let ran = forge(&["validate", &unstored]);
            assert_eq!(ran.code, 1, "{ran:?}");
            assert!(
                ran.out.contains("1 formula(s) have no stored result"),
                "{ran:?}"
            );
            assert_eq!(forge(&["calculate", &unstored]).code, 0);
            let ran = forge(&["validate", &unstored]);
            assert_eq!(ran.code, 0, "{ran:?}");
            for (text, error) in [
                (
                    "_forge_version: \"5.1.0\"\ns:\n  x: 0\n  y: {formula: \"=1 / x\"}\n",
                    "Formula evaluation error: s.y: Division by zero (formula: =1 / x)",
                ),
                (
                    "_forge_version: \"5.1.0\"\ns:\n  y: {formula: \"=MC.Uniform(0, 1)\"}\n",
                    "Validation error: s.y use MC.* distributions, which calculate and validate cannot evaluate. Run `forge simulate` on this file instead, or `forge calculate --plan` for its plan.",
                ),
                (
                    "a: [1\n",
                    "YAML parsing error: did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 4",
                ),
            ] {
                let file = model(&dir, text);
                let core = validate_core(Path::new(&file)).unwrap_err().to_string();
                let ran = forge(&["validate", &file]);
                assert_eq!(ran.code, 1, "{ran:?}");
                assert!(ran.err.contains(&core), "{core}\n{ran:?}");
                assert_eq!(core, error);
            }
        }
    }

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
            let err = scenarios_core(&path, None, false).unwrap_err().to_string();
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
                let results = scenarios_core(&path, None, false).unwrap();
                assert!((shock_profit(&results) - 400.0).abs() < 1e-9, "{line}");
            }
        }

        #[test]
        fn test_override_of_an_included_value() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "\"@costs.prices.flour\": 3");
            let results = scenarios_core(&path, None, false).unwrap();
            assert!((shock_profit(&results) - 200.0).abs() < 1e-9);
            let calc = calculate_core(&path, true, Some("shock")).unwrap();
            assert!((calc.scalars["out.profit"].unwrap() - 200.0).abs() < 1e-9);
        }

        #[test]
        fn test_override_of_an_unknown_include_or_name_is_an_error() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "\"@rates.prices.flour\": 3");
            let err = scenarios_core(&path, None, false).unwrap_err().to_string();
            assert!(
                err.contains("no `_includes` entry with `as: rates`"),
                "{err}"
            );
            let path = write_model(dir.path(), "\"@costs.prices.flower\": 3");
            let err = scenarios_core(&path, None, false).unwrap_err().to_string();
            assert!(err.contains("Did you mean 'prices.flour'"), "{err}");
        }

        #[test]
        fn test_outputs_ranges_and_filter() {
            let dir = tempfile::tempdir().unwrap();
            let path = write_model(dir.path(), "price: 1");
            let results = scenarios_core(&path, Some("shock"), false).unwrap();
            assert_eq!(results.scenarios.len(), 1);
            assert_eq!(results.scenarios[0].name, "shock");
            // base 300, shock -100: half the probability is positive
            assert!((results.probability_positive["out.profit"] - 0.5).abs() < 1e-9);
            assert_eq!(results.ranges["out.profit"], (-100.0, 300.0));
            let err = scenarios_core(&path, Some("nope"), false)
                .unwrap_err()
                .to_string();
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
                    false,
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
            assert!(ran.success(), "{ran:?}");
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
    //! What `calculate` prints, in the cases the other tests do not reach:
    //! empty cells, units, scenarios, a result already stored.

    use super::*;
    use crate::cli::console::capture;
    use std::fs;
    use std::path::PathBuf;

    const GOOD: &str = "_forge_version: \"5.0.0\"\ninputs:\n  a:\n    value: 2\n    formula: null\noutputs:\n  b:\n    value: 4\n    formula: \"=a * 2\"\n";

    fn model(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn an_empty_cell_shows_as_blank_null_or_nothing() {
        let column = ColumnValue::Blanks(vec![Some(1.5), None]);
        assert_eq!(
            format_column_values(&column_json_values(&column)),
            "[1.5, null]"
        );
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
        let ((), captured) = capture(|c| print_unit_warnings(c, &unit_warnings(&model)));
        assert!(
            captured.out.contains("Unit Consistency Warnings"),
            "{}",
            captured.out
        );

        let ((), captured) =
            capture(|c| print_unit_warnings(c, &unit_warnings(&ParsedModel::new())));
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
        let (result, captured) = capture(|c| calculate(c, &file, true, true, Some("high"), false));
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
        let (result, captured) = capture(|c| calculate(c, &file, true, false, Some("high"), false));
        result.unwrap();
        assert!(
            !captured.out.contains("Applied scenario"),
            "{}",
            captured.out
        );
        let (result, _) = capture(|c| calculate(c, &file, true, false, Some("absent"), false));
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn verbose_lists_the_scenarios_sorted() {
        // The scenarios are a HashMap: unsorted, the list changed from run to run
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\nrate: 2\nscenarios:\n  zeta:\n    scalars:\n      rate: 1\n  alpha:\n    scalars:\n      rate: 3\n  mid:\n    scalars:\n      rate: 4\n  beta:\n    scalars:\n      rate: 5\n",
        );
        let (result, captured) = capture(|c| calculate(c, &file, true, true, None, false));
        result.unwrap();
        assert!(
            captured
                .out
                .contains(r#"Found 4 scenarios: ["alpha", "beta", "mid", "zeta"]"#),
            "{}",
            captured.out
        );
    }

    #[test]
    fn calculating_under_a_scenario_writes_nothing() {
        // Before ADR-074, the scenario's result replaced the file's own
        // (`value: 1.2` became 0.8), and forge validate then failed
        let dir = tempfile::tempdir().unwrap();
        let text =
            "_forge_version: \"5.5.0\"\nw: 0.6\nout:\n  twice:\n    formula: \"=w * 2\"\n    \
                    value: 1.2\nscenarios:\n  short:\n    scalars: {w: 0.4}\n";
        let file = model(dir.path(), "m.yaml", text);
        let (result, captured) =
            capture(|c| calculate(c, &file, false, false, Some("short"), false));
        result.unwrap();
        assert!(captured.out.contains("out.twice = 0.8"), "{}", captured.out);
        assert!(
            captured.out.contains(
                "📋 Under a scenario, nothing is written: the stored values are the file's own. \
                 To state what this scenario gives, list it under scenarios.short.expect"
            ),
            "{}",
            captured.out
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), text);
        assert!(!file.with_extension("yaml.bak").exists());
        let core = calculate_core(&file, false, Some("short")).unwrap();
        assert!(!core.file_updated);
        assert_eq!(fs::read_to_string(&file).unwrap(), text);
        crate::cli::validate_core(&file)
            .unwrap()
            .scalars_valid
            .then_some(())
            .unwrap();
    }

    #[test]
    fn calculate_reports_the_expectations_of_the_run_it_calculates() {
        // Studio shows them beside the values (ADR-074, question 6)
        let file = Path::new("examples/games/stag-hunt.yaml");
        let base = calculate_core(file, true, None).unwrap();
        let seen: Vec<(&str, f64, bool)> = base
            .expectations
            .iter()
            .map(|e| (e.path.as_deref().unwrap(), e.expected, e.met))
            .collect();
        assert_eq!(
            seen,
            [
                ("kinship.stag_share", 0.75, true),
                ("kinship.crossover_c", 1.0, true),
                ("kinship.coop_wins", 0.0, true),
                ("kinship.stags_at_the_end", 1.0, true),
            ]
        );
        let c3 = calculate_core(file, true, Some("c3")).unwrap();
        let json = serde_json::to_value(&c3).unwrap();
        assert_eq!(json["expectations"][0]["scenario"], "c3");
        assert_eq!(json["expectations"][0]["formula"], "=3/8");
        assert_eq!(json["expectations"][0]["got"], 0.375);
        assert_eq!(json["expectations"][1]["met"], true);
        let plain = calculate_core(Path::new("examples/quota_forecast.yaml"), true, None).unwrap();
        assert!(serde_json::to_value(&plain)
            .unwrap()
            .get("expectations")
            .is_none());
    }

    /// What `forge calculate` prints comes from the result: the model's
    /// order, the values written, the file as read (T1-CLI-FROM-CORES).
    #[test]
    fn the_result_has_all_that_calculate_prints() {
        let dir = tempfile::tempdir().unwrap();
        let text =
            "_forge_version: \"5.1.0\"\nzeta: 2\nalpha: {formula: \"=zeta * 2\", value: null}\n\
                    t2:\n  n: [1]\nt1:\n  m: [2]\nscenarios:\n  high:\n    scalars: {zeta: 3}\n";
        let file = model(dir.path(), "m.yaml", text);
        let first = calculate_core(&file, false, None).unwrap();
        // The model's order here, the names' in JSON
        assert_eq!(first.scalars.keys().collect::<Vec<_>>(), ["zeta", "alpha"]);
        assert_eq!(first.tables.keys().collect::<Vec<_>>(), ["t2", "t1"]);
        let json = serde_json::to_string(&first).unwrap();
        assert!(json.find("\"alpha\"") < json.find("\"zeta\""), "{json}");
        assert!(json.find("\"t1\"") < json.find("\"t2\""), "{json}");
        assert_eq!((first.values_updated, first.file_updated), (1, true));
        assert!(json.contains("\"values_updated\":1"), "{json}");
        assert_eq!((first.read.tables, first.read.scalars), (2, 2));
        assert_eq!(first.read.scenarios, ["high"]);
        let again = calculate_core(&file, false, None).unwrap();
        assert_eq!((again.values_updated, again.file_updated), (0, false));
        let json = serde_json::to_value(&again).unwrap();
        assert!(json.get("values_updated").is_none(), "{json}");
        // A game's results are scalars the file does not have
        let game = calculate_core(Path::new("examples/games/stag-hunt.yaml"), true, None).unwrap();
        assert!(game.scalars.len() > game.read.scalars, "{:?}", game.read);
    }

    #[test]
    fn calculating_twice_finds_nothing_to_update() {
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "m.yaml",
            &GOOD.replace("value: 4", "value: null"),
        );
        let (result, captured) = capture(|c| calculate(c, &file, false, false, None, false));
        result.unwrap();
        assert!(captured.out.contains("1 value updated"), "{}", captured.out);
        let (result, captured) = capture(|c| calculate(c, &file, false, false, None, false));
        result.unwrap();
        assert!(captured.out.contains("is up to date"), "{}", captured.out);
        let two = model(
            dir.path(),
            "two.yaml",
            &format!("{GOOD}  c:\n    value: null\n    formula: \"=a * 3\"\n")
                .replace("value: 4", "value: null"),
        );
        let (result, captured) = capture(|c| calculate(c, &two, false, false, None, false));
        result.unwrap();
        let out = &captured.out;
        assert!(out.contains("💾 2 values updated in"), "{out}");
    }

    #[test]
    fn text_columns_print_as_they_are_and_an_empty_scalar_is_left_out() {
        let text = ColumnValue::Text(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            format_column_values(&column_json_values(&text)),
            "[\"a\", \"b\"]"
        );
        let dir = tempfile::tempdir().unwrap();
        let file = model(
            dir.path(),
            "t.yaml",
            "_forge_version: \"5.1.0\"\nt:\n  n: [1, 2]\n  label: '=IF(n > 1, \"big\", \"small\")'\nempty: {value: null, formula: null}\n",
        );
        let (result, captured) = capture(|c| calculate(c, &file, true, true, None, false));
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
        let (result, _) = capture(|c| calculate(c, &file, false, false, None, false));
        assert!(result.is_err());
        assert!(calculate_core(&file, false, None).is_err());
        assert!(fs::read_to_string(&file).unwrap().contains("value: null"));
    }
}
