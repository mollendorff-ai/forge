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
//! - simulate: Monte Carlo simulation (enterprise only)
//! - upgrade: Schema migration (enterprise only)
//! - scenarios: Scenario analysis (enterprise only)
//! - `decision_tree`: Decision tree analysis (enterprise only)
//! - `real_options`: Real options valuation (enterprise only)
//! - tornado: Tornado sensitivity diagrams (enterprise only)
//! - bootstrap: Bootstrap resampling (enterprise only)
//! - bayesian: Bayesian network inference (enterprise only)

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
pub use upgrade::{auto_upgrade_schema, needs_schema_upgrade, upgrade};

// Core function re-exports (return structured results, no printing)
pub use analysis::{compare_core, goal_seek_core, sensitivity_core, variance_core};
pub use audit::audit_core;
pub use examples::examples_core;
pub use excel_io::{export_core, import_core};
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
use crate::writer;
use colored::Colorize;
#[cfg(any(not(coverage), test))]
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
    // This also handles f32 precision artifacts from xlformula_engine
    let rounded = (n * 1e6).round() / 1e6;
    // Format with up to 6 decimal places, removing trailing zeros
    format!("{rounded:.6}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
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
    let mut tables = std::collections::HashMap::new();
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
                columns: table.columns.keys().cloned().collect(),
            },
        );
    }

    // Build scalar map
    let mut scalars = std::collections::HashMap::new();
    for (name, var) in &result.scalars {
        scalars.insert(name.clone(), var.value);
    }

    // Write results if not dry run
    let file_updated = if dry_run {
        false
    } else {
        writer::write_calculated_results(file, &result)?
    };

    Ok(results::CalculationResult {
        tables,
        scalars,
        unit_warnings,
        file_updated,
        dry_run,
    })
}

/// Execute the calculate command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, calculation fails,
/// or results cannot be written back to the file.
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

    // Show table results
    for (table_name, table) in &result.tables {
        println!("   📊 Table: {}", table_name.bright_blue().bold());
        for (col_name, column) in &table.columns {
            println!("      {} ({} rows)", col_name.cyan(), column.values.len());
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
        let wrote = writer::write_calculated_results(file, &result)?;
        if wrote {
            println!(
                "{}",
                format!("💾 Results written to {}", file.display())
                    .bold()
                    .green()
            );
            println!(
                "{}",
                format!("   Backup saved to {}.bak", file.display()).dimmed()
            );
        } else {
            // Multi-document YAML - write-back not supported (v4.4.2)
            println!(
                "{}",
                "⚠️  Multi-document YAML - write-back not supported yet".yellow()
            );
            println!(
                "{}",
                "   Results displayed above. Split into separate files to persist.".dimmed()
            );
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

    // Apply overrides to scalars
    for (var_name, override_value) in &overrides {
        if let Some(scalar) = model.scalars.get_mut(var_name) {
            scalar.value = Some(*override_value);
            // Clear formula since we're using override value
            scalar.formula = None;
        } else {
            // Create new scalar with override value
            model.scalars.insert(
                var_name.clone(),
                crate::types::Variable::new(var_name.clone(), Some(*override_value), None),
            );
        }
    }

    Ok(())
}
