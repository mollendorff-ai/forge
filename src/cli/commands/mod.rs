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
mod examples;
mod excel_io;
mod functions;
mod prediction;
pub mod results;
mod schema;
mod simulate;
mod update;
mod upgrade;
#[cfg(test)]
mod what_if_tests;

// Re-exports
pub use analysis::{break_even, compare, goal_seek, sensitivity, variance};
pub use audit::audit;
pub use examples::examples;
pub use excel_io::{export, import};
pub use functions::functions;
pub use prediction::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};
pub use schema::schema;
pub use simulate::simulate;
pub use update::update;
pub use upgrade::upgrade;

// Core function re-exports (return structured results, no printing)
pub use analysis::{compare_core, goal_seek_core, sensitivity_core, variance_core};
pub use audit::audit_core;
pub use examples::examples_core;
pub use excel_io::{export_buffer_core, export_core, import_core};
pub use functions::functions_core;
pub use prediction::{
    bayesian_core, bootstrap_core, decision_tree_core, real_options_core, scenarios_core,
    tornado_core,
};
pub use schema::schema_core;
pub use simulate::simulate_core;

// Re-exports for tests (internal functions)
#[cfg(test)]
pub use analysis::{
    calculate_with_override, export_variance_to_excel, export_variance_to_yaml, parse_range,
    print_variance_table, VarianceResult,
};
#[cfg(test)]
pub use audit::{
    build_dependency_tree, extract_references_from_formula, find_variable, print_dependency,
    AuditDependency,
};
#[cfg(test)]
pub use upgrade::split_scalars_to_inputs_outputs;

use crate::core::{ArrayCalculator, UnitValidator};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::ColumnValue;
use crate::writer;
use colored::Colorize;
use std::path::Path;
use std::path::PathBuf;

// Watch-related imports only for non-coverage builds (see ADR-006)
#[cfg(not(coverage))]
use notify::RecursiveMode;
#[cfg(not(coverage))]
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
#[cfg(not(coverage))]
use std::sync::mpsc::channel;
#[cfg(not(coverage))]
use std::time::Duration;

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
        ColumnValue::Number(v) => v.iter().map(|n| serde_json::json!(n)).collect(),
        ColumnValue::Text(v) | ColumnValue::Date(v) => {
            v.iter().map(|t| serde_json::json!(t)).collect()
        },
        ColumnValue::Boolean(v) => v.iter().map(|b| serde_json::json!(b)).collect(),
    }
}

/// A column's values for the terminal, all of them and at full precision, like
/// scalars: `[1, 2, 3]`. Long columns are not cut, because the values are what
/// a reader (or an agent) needs.
fn format_column_values(values: &ColumnValue) -> String {
    let items: Vec<String> = match values {
        ColumnValue::Number(v) => v.iter().map(ToString::to_string).collect(),
        ColumnValue::Text(v) | ColumnValue::Date(v) => v.iter().map(|t| format!("{t:?}")).collect(),
        ColumnValue::Boolean(v) => v.iter().map(|b| b.to_string().to_uppercase()).collect(),
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
            },
        );
    }

    // Build scalar map
    let mut scalars = std::collections::BTreeMap::new();
    for (name, var) in &result.scalars {
        scalars.insert(name.clone(), var.value);
    }

    // Write results if not dry run
    let file_updated = if dry_run {
        false
    } else {
        writer::write_calculated_results(file, &result)?.is_some_and(|n| n > 0)
    };

    Ok(results::CalculationResult {
        tables,
        scalars,
        unit_warnings,
        file_updated,
        dry_run,
    })
}

/// `calculate` and `validate` are deterministic and cannot evaluate `MC.*` distributions; say so
/// up front instead of failing later with an unknown-function or unknown-column error.
fn reject_distributions(model: &crate::types::ParsedModel) -> ForgeResult<()> {
    let mut names: Vec<&str> = model
        .scalars
        .iter()
        .filter(|(_, v)| v.formula.as_deref().is_some_and(|f| f.contains("MC.")))
        .map(|(name, _)| name.as_str())
        .collect();
    if names.is_empty() {
        return Ok(());
    }
    names.sort_unstable();
    Err(ForgeError::Validation(format!(
        "{} use MC.* distributions, which calculate and validate cannot evaluate. Run `forge simulate` on this file instead.",
        names.join(", ")
    )))
}

/// Execute the calculate command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, calculation fails,
/// or results cannot be written back to the file.
#[allow(clippy::too_many_lines)]
pub fn calculate(
    file: &Path,
    dry_run: bool,
    verbose: bool,
    scenario: Option<&str>,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Calculating formulas".bold().green());
    println!("   File: {}", file.display());
    if let Some(s) = scenario {
        println!("   Scenario: {}", s.bright_yellow().bold());
    }
    println!();

    if dry_run {
        println!(
            "{}",
            "📋 DRY RUN MODE - No changes will be written\n".yellow()
        );
    }

    // Parse file
    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    let mut model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
        if !model.scenarios.is_empty() {
            println!(
                "   Found {} scenarios: {:?}",
                model.scenarios.len(),
                model.scenario_names()
            );
        }
        println!();
    }

    // Apply scenario overrides if specified
    if let Some(scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
        if verbose {
            println!("{}", format!("📊 Applied scenario: {scenario_name}").cyan());
        }
    }

    // Unit consistency validation (v4.0)
    let unit_validator = UnitValidator::new(&model);
    let unit_warnings = unit_validator.validate();
    if !unit_warnings.is_empty() {
        println!("{}", "⚠️  Unit Consistency Warnings:".yellow().bold());
        for warning in &unit_warnings {
            println!("   {}", warning.to_string().yellow());
        }
        println!();
    }

    // Calculate using ArrayCalculator
    if verbose {
        println!("{}", "🧮 Calculating tables and scalars...".cyan());
    }

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Display results
    println!("{}", "✅ Calculation Results:".bold().green());

    // Show table results in model order (the order of the file), which is the
    // same on every run
    for (table_name, table) in &result.tables {
        println!("   📊 Table: {}", table_name.bright_blue().bold());
        for col_name in table.column_names() {
            let Some(column) = table.columns.get(col_name) else {
                continue;
            };
            println!(
                "      {} = {}",
                col_name.cyan(),
                format_column_values(&column.values)
            );
        }
    }

    // Show scalar results
    if !result.scalars.is_empty() {
        println!("\n   📐 Scalars:");
        for (name, var) in &result.scalars {
            if let Some(value) = var.value {
                println!(
                    "      {} = {}",
                    name.bright_blue(),
                    format!("{value}").bold()
                );
            }
        }
    }
    println!();

    // Write results back to file (v4.3.0)
    if dry_run {
        println!("{}", "📋 Dry run complete - no changes written".yellow());
    } else {
        match writer::write_calculated_results(file, &result)? {
            Some(0) => {
                println!(
                    "{}",
                    format!(
                        "✅ {} is up to date (no stored result changed)",
                        file.display()
                    )
                    .green()
                );
            },
            Some(n) => {
                println!(
                    "{}",
                    format!(
                        "💾 {n} value{} updated in {}",
                        if n == 1 { "" } else { "s" },
                        file.display()
                    )
                    .bold()
                    .green()
                );
                println!(
                    "{}",
                    format!("   Backup saved to {}.bak", file.display()).dimmed()
                );
            },
            None => {
                // Multi-document YAML - write-back not supported (v4.4.2)
                println!(
                    "{}",
                    "⚠️  Multi-document YAML - write-back not supported yet".yellow()
                );
                println!(
                    "{}",
                    "   Results displayed above. Split into separate files to persist.".dimmed()
                );
            },
        }
    }

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
pub fn validate(files: &[PathBuf]) -> ForgeResult<()> {
    let file_count = files.len();
    let is_batch = file_count > 1;

    if is_batch {
        println!(
            "{}",
            format!("✅ Validating {file_count} files").bold().green()
        );
        println!();
    }

    let mut all_passed = true;
    let mut failed_files: Vec<String> = Vec::new();

    for file in files {
        if is_batch {
            println!("{}", format!("─── {} ───", file.display()).cyan());
        } else {
            println!("{}", "✅ Validating model".bold().green());
            println!("   File: {}\n", file.display());
        }

        match validate_single_file(file) {
            Ok(()) => {
                if is_batch {
                    println!("{}", format!("   ✅ {} - OK", file.display()).green());
                    println!();
                }
            },
            Err(e) => {
                if !is_batch {
                    // Single file mode - propagate original error directly
                    return Err(e);
                }
                all_passed = false;
                failed_files.push(format!("{}: {}", file.display(), e));
                println!("{}", format!("   ❌ {} - FAILED", file.display()).red());
                println!("      {}", e.to_string().red());
                println!();
            },
        }
    }

    // Summary for batch validation
    if is_batch {
        println!("{}", "─".repeat(50));
        let passed = file_count - failed_files.len();
        println!(
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
fn validate_single_file(file: &std::path::Path) -> ForgeResult<()> {
    const TOLERANCE: f64 = 0.0001; // Floating point comparison tolerance

    // Parse YAML file
    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if model.tables.is_empty() && model.scalars.is_empty() {
        println!("{}", "⚠️  No tables or scalars found in YAML file".yellow());
        return Ok(());
    }

    println!(
        "   Found {} tables, {} scalars",
        model.tables.len(),
        model.scalars.len()
    );

    // Note: Table column length validation is deferred to calculation time
    // Row-wise operations will validate at runtime in array_calculator

    // Calculate what values SHOULD be based on formulas
    let calculator = ArrayCalculator::new(model.clone());
    let calculated = match calculator.calculate_all() {
        Ok(vals) => vals,
        Err(e) => {
            println!(
                "\n{}",
                format!("❌ Formula validation failed: {e}").bold().red()
            );
            return Err(e);
        },
    };

    // Compare calculated values vs. current values in file
    let mut mismatches = Vec::new();

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

    // Report results
    println!();
    if mismatches.is_empty() {
        println!("{}", "✅ All tables are valid!".bold().green());
        println!(
            "{}",
            "✅ All scalar values match their formulas!".bold().green()
        );
        Ok(())
    } else {
        println!(
            "{}",
            format!("❌ Found {} value mismatches!", mismatches.len())
                .bold()
                .red()
        );
        println!("{}", "   File needs recalculation!\n".yellow());

        for (name, current, expected, diff) in &mismatches {
            println!("   {}", name.bright_blue().bold());
            // Format numbers with reasonable precision (remove trailing zeros)
            println!("      Current:  {}", format_number(*current).clone().red());
            println!(
                "      Expected: {}",
                format_number(*expected).clone().green()
            );
            println!("      Diff:     {}", format!("{diff:.6}").yellow());
            println!();
        }

        println!(
            "{}",
            "💡 Run 'forge calculate' to update values".bold().yellow()
        );

        Err(crate::error::ForgeError::Validation(
            "Values do not match formulas - file needs recalculation".to_string(),
        ))
    }
}

/// Execute the watch command
///
/// # Errors
///
/// Returns an error if the file does not exist, the directory cannot be watched,
/// or file system event handling fails.
///
/// # Coverage Exclusion (ADR-006)
/// Contains infinite loop waiting for file system events - cannot unit test.
/// Tested via: `cli_integration_tests.rs` (manual termination after initial run)
#[cfg(not(coverage))]
pub fn watch(file: &Path, validate_only: bool, verbose: bool) -> ForgeResult<()> {
    println!("{}", "👁️  Forge - Watch Mode".bold().green());
    println!("   Watching: {}", file.display());
    println!(
        "   Mode: {}",
        if validate_only {
            "validate only"
        } else {
            "calculate"
        }
    );
    println!("   Press {} to stop\n", "Ctrl+C".bold().yellow());

    // Verify file exists
    if !file.exists() {
        return Err(ForgeError::Validation(format!(
            "File not found: {}",
            file.display()
        )));
    }

    // Get canonical path and parent directory
    let canonical_path = file.canonicalize().map_err(ForgeError::Io)?;
    let parent_dir = canonical_path
        .parent()
        .ok_or_else(|| ForgeError::Validation("Cannot determine parent directory".to_string()))?;

    // Create channel for file system events
    let (tx, rx) = channel();

    // Create a debouncer to avoid rapid-fire events during file saves
    let mut debouncer = new_debouncer(Duration::from_millis(200), tx)
        .map_err(|e| ForgeError::Validation(format!("Failed to create file watcher: {e}")))?;

    // Watch the parent directory (watches all YAML files in that directory)
    debouncer
        .watcher()
        .watch(parent_dir, RecursiveMode::NonRecursive)
        .map_err(|e| ForgeError::Validation(format!("Failed to watch directory: {e}")))?;

    if verbose {
        println!(
            "   {} {}",
            "Watching directory:".cyan(),
            parent_dir.display()
        );
    }

    // Run initial validation/calculation
    println!("{}", "🔄 Initial run...".cyan());
    run_watch_action(file, validate_only, verbose);
    println!();

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Check if any event is for our file (or any .yaml file in directory)
                let relevant = events.iter().any(|event| {
                    if event.kind != DebouncedEventKind::Any {
                        return false;
                    }
                    // Check if it's our main file
                    if let Ok(event_canonical) = event.path.canonicalize() {
                        if event_canonical == canonical_path {
                            return true;
                        }
                    }
                    // Check if filename matches our file
                    if let Some(filename) = event.path.file_name() {
                        if let Some(our_filename) = canonical_path.file_name() {
                            if filename == our_filename {
                                return true;
                            }
                        }
                        // Also trigger on any .yaml file changes in the directory
                        if let Some(ext) = event.path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
                                return true;
                            }
                        }
                    }
                    false
                });

                if relevant {
                    // Clear screen for fresh output (optional, can be verbose mode only)
                    if verbose {
                        print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen
                    }
                    println!(
                        "\n{} {}",
                        "🔄 Change detected at".cyan(),
                        chrono_lite_timestamp().cyan()
                    );
                    run_watch_action(file, validate_only, verbose);
                    println!();
                }
            },
            Ok(Err(error)) => {
                eprintln!("{} Watch error: {}", "❌".red(), error);
            },
            Err(e) => {
                eprintln!("{} Channel error: {}", "❌".red(), e);
                break;
            },
        }
    }

    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn watch(file: &Path, _validate_only: bool, _verbose: bool) -> ForgeResult<()> {
    // Validate file exists (testable error path)
    if !file.exists() {
        return Err(ForgeError::Validation(format!(
            "File not found: {}",
            file.display()
        )));
    }
    Ok(())
}

/// Get a simple timestamp without external dependencies
#[cfg(any(not(coverage), test))]
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02} UTC")
}

/// Run the watch action (validate or calculate)
#[cfg(any(not(coverage), test))]
fn run_watch_action(file: &Path, validate_only: bool, verbose: bool) {
    if validate_only {
        match validate_internal(file, verbose) {
            Ok(()) => println!("{}", "✅ Validation passed".bold().green()),
            Err(e) => println!("{} {}", "❌ Validation failed:".bold().red(), e),
        }
    } else {
        match calculate_internal(file, verbose) {
            Ok(()) => println!("{}", "✅ Calculation complete".bold().green()),
            Err(e) => println!("{} {}", "❌ Calculation failed:".bold().red(), e),
        }
    }
}

/// Internal validation function for watch mode
#[cfg(any(not(coverage), test))]
fn validate_internal(file: &Path, verbose: bool) -> ForgeResult<()> {
    const TOLERANCE: f64 = 0.0001;

    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Note: Table column length validation is deferred to calculation time
    // Row-wise operations will validate at runtime in array_calculator

    // Calculate and compare
    let calculator = ArrayCalculator::new(model.clone());
    let calculated = calculator.calculate_all()?;

    // Check for mismatches
    let mut mismatches = Vec::new();

    for (var_name, var) in &calculated.scalars {
        if let Some(calculated_value) = var.value {
            if let Some(original) = model.scalars.get(var_name) {
                if let Some(current_value) = original.value {
                    let diff = (current_value - calculated_value).abs();
                    if diff > TOLERANCE {
                        mismatches.push((var_name.clone(), current_value, calculated_value));
                    }
                }
            }
        }
    }

    if !mismatches.is_empty() {
        let msg = mismatches
            .iter()
            .map(|(name, current, expected)| {
                format!("  {name} current={current} expected={expected}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(ForgeError::Validation(format!(
            "{} value mismatches:\n{}",
            mismatches.len(),
            msg
        )));
    }

    Ok(())
}

/// Internal calculation function for watch mode
#[cfg(any(not(coverage), test))]
fn calculate_internal(file: &Path, verbose: bool) -> ForgeResult<()> {
    let model = parser::parse_model(file)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Show summary
    for (table_name, table) in &result.tables {
        println!(
            "   📊 {} ({} columns)",
            table_name.bright_blue(),
            table.columns.len()
        );
    }

    if !result.scalars.is_empty() && verbose {
        println!("   📐 {} scalars calculated", result.scalars.len());
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
        let available: Vec<_> = model.scenarios.keys().collect();
        ForgeError::Validation(format!(
            "Scenario '{scenario_name}' not found. Available scenarios: {available:?}"
        ))
    })?;

    // Clone the overrides to avoid borrow checker issues
    let overrides = scenario.overrides.clone();

    // Apply overrides to scalars (resolving grouped names like "rate" → "assumptions.rate",
    // and `@namespace.name` to a scalar of an included file)
    for (var_name, override_value) in &overrides {
        model
            .override_scalar(var_name, Some(*override_value), None)
            .map_err(|e| ForgeError::Validation(format!("Scenario '{scenario_name}': {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod calculate_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn calculate_rejects_mc_distributions_with_a_clear_message() {
        let mut file = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            "_forge_version: \"5.0.0\"\nassumptions:\n  growth:\n    value: null\n    formula: \"=MC.Normal(0.15, 0.05)\"\n  doubled:\n    value: null\n    formula: \"=assumptions.growth * 2\"\n"
        )
        .unwrap();
        let err = calculate(file.path(), true, false, None)
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
        let err = validate_internal(file.path(), false)
            .unwrap_err()
            .to_string();
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
