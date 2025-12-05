use crate::core::{ArrayCalculator, UnitValidator};
use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
use crate::writer;
use colored::Colorize;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

/// Format a number for display, removing unnecessary decimal places
fn format_number(n: f64) -> String {
    // Round to 6 decimal places for display (sufficient for most financial calculations)
    // This also handles f32 precision artifacts from xlformula_engine
    let rounded = (n * 1e6).round() / 1e6;
    // Format with up to 6 decimal places, removing trailing zeros
    format!("{:.6}", rounded)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Execute the calculate command
pub fn calculate(
    file: PathBuf,
    dry_run: bool,
    verbose: bool,
    scenario: Option<String>,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Calculating formulas".bold().green());
    println!("   File: {}", file.display());
    if let Some(ref s) = scenario {
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

    let mut model = parser::parse_model(&file)?;

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
    if let Some(ref scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
        if verbose {
            println!(
                "{}",
                format!("📊 Applied scenario: {}", scenario_name).cyan()
            );
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
        let wrote = writer::write_calculated_results(&file, &result)?;
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

/// Execute the audit command - show calculation dependency chain
pub fn audit(file: PathBuf, variable: String) -> ForgeResult<()> {
    println!("{}", "🔍 Forge - Audit Trail".bold().green());
    println!("   File: {}", file.display());
    println!("   Variable: {}\n", variable.bright_blue().bold());

    // Parse the model
    let model = parser::parse_model(&file)?;

    // Try to find the variable
    let (var_type, formula, current_value) = find_variable(&model, &variable)?;

    println!("{}", "📋 Variable Information:".bold().cyan());
    println!("   Type: {}", var_type.cyan());
    if let Some(val) = current_value {
        println!("   Current Value: {}", format_number(val).bold().green());
    }
    if let Some(ref f) = formula {
        println!("   Formula: {}", f.bright_yellow());
    }
    println!();

    // Build and display dependency tree
    if formula.is_some() {
        println!("{}", "🌳 Dependency Tree:".bold().cyan());
        let deps = build_dependency_tree(&model, &variable, &formula, 0)?;

        if deps.is_empty() {
            println!("   No dependencies (literal value)");
        } else {
            for dep in &deps {
                print_dependency(dep, 1);
            }
        }
        println!();
    }

    // Calculate and verify
    println!("{}", "🧮 Calculation Chain:".bold().cyan());
    let calculator = ArrayCalculator::new(model.clone());
    match calculator.calculate_all() {
        Ok(result) => {
            // Find the calculated value
            if let Some(scalar) = result.scalars.get(&variable) {
                if let Some(calc_val) = scalar.value {
                    println!("   Calculated: {}", format_number(calc_val).bold().green());

                    // Check if it matches current value
                    if let Some(curr) = current_value {
                        let diff = (curr - calc_val).abs();
                        if diff < 0.0001 {
                            println!("   {} Values match!", "✅".green());
                        } else {
                            println!("   {} Value mismatch!", "⚠️".yellow());
                            println!("      Current:    {}", format_number(curr).red());
                            println!("      Calculated: {}", format_number(calc_val).green());
                        }
                    }
                }
            } else {
                // Check in tables
                for (table_name, table) in &result.tables {
                    if let Some(col) = table.columns.get(&variable) {
                        println!("   Table: {}", table_name.bright_blue());
                        println!("   Column values: {:?}", col.values);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("   {} Calculation error: {}", "❌".red(), e);
        }
    }

    println!();
    println!("{}", "✅ Audit complete".bold().green());
    Ok(())
}

/// Represents a dependency in the audit tree
struct AuditDependency {
    name: String,
    dep_type: String,
    formula: Option<String>,
    value: Option<f64>,
    children: Vec<AuditDependency>,
}

/// Find a variable in the model and return its type, formula, and current value
fn find_variable(
    model: &crate::types::ParsedModel,
    name: &str,
) -> ForgeResult<(String, Option<String>, Option<f64>)> {
    // Check scalars first
    if let Some(scalar) = model.scalars.get(name) {
        let formula = scalar.formula.clone();
        let value = scalar.value;
        return Ok(("Scalar".to_string(), formula, value));
    }

    // Check aggregations
    if let Some(agg_formula) = model.aggregations.get(name) {
        return Ok(("Aggregation".to_string(), Some(agg_formula.clone()), None));
    }

    // Check table columns
    for (table_name, table) in &model.tables {
        if table.columns.contains_key(name) {
            let formula = table.row_formulas.get(name).cloned();
            return Ok((format!("Column in table '{}'", table_name), formula, None));
        }
    }

    Err(ForgeError::Validation(format!(
        "Variable '{}' not found in model. Available:\n  Scalars: {:?}\n  Aggregations: {:?}\n  Tables: {:?}",
        name,
        model.scalars.keys().collect::<Vec<_>>(),
        model.aggregations.keys().collect::<Vec<_>>(),
        model.tables.keys().collect::<Vec<_>>()
    )))
}

/// Build the dependency tree for a variable
fn build_dependency_tree(
    model: &crate::types::ParsedModel,
    _name: &str,
    formula: &Option<String>,
    depth: usize,
) -> ForgeResult<Vec<AuditDependency>> {
    // Prevent infinite recursion
    if depth > 20 {
        return Ok(vec![]);
    }

    let mut deps = Vec::new();

    if let Some(f) = formula {
        // Extract references from formula
        let refs = extract_references_from_formula(f);

        for ref_name in refs {
            let mut dep = AuditDependency {
                name: ref_name.clone(),
                dep_type: "Unknown".to_string(),
                formula: None,
                value: None,
                children: vec![],
            };

            // Try to find this reference in the model
            if let Some(scalar) = model.scalars.get(&ref_name) {
                dep.dep_type = "Scalar".to_string();
                dep.formula = scalar.formula.clone();
                dep.value = scalar.value;

                // Recursively get children
                if scalar.formula.is_some() {
                    dep.children =
                        build_dependency_tree(model, &ref_name, &scalar.formula, depth + 1)?;
                }
            } else if let Some(agg) = model.aggregations.get(&ref_name) {
                dep.dep_type = "Aggregation".to_string();
                dep.formula = Some(agg.clone());
                dep.children =
                    build_dependency_tree(model, &ref_name, &Some(agg.clone()), depth + 1)?;
            } else {
                // Check if it's a table column
                for (table_name, table) in &model.tables {
                    if table.columns.contains_key(&ref_name) {
                        dep.dep_type = format!("Column[{}]", table_name);
                        dep.formula = table.row_formulas.get(&ref_name).cloned();
                        break;
                    }
                }
            }

            deps.push(dep);
        }
    }

    Ok(deps)
}

/// Extract variable references from a formula
fn extract_references_from_formula(formula: &str) -> Vec<String> {
    let formula = formula.trim_start_matches('=');
    let mut refs = Vec::new();

    // Known function names to exclude
    let functions = [
        "SUM",
        "AVERAGE",
        "AVG",
        "MAX",
        "MIN",
        "COUNT",
        "PRODUCT",
        "SUMIF",
        "COUNTIF",
        "AVERAGEIF",
        "SUMIFS",
        "COUNTIFS",
        "AVERAGEIFS",
        "MAXIFS",
        "MINIFS",
        "ROUND",
        "ROUNDUP",
        "ROUNDDOWN",
        "CEILING",
        "FLOOR",
        "SQRT",
        "POWER",
        "MOD",
        "ABS",
        "IF",
        "AND",
        "OR",
        "NOT",
        "CONCAT",
        "UPPER",
        "LOWER",
        "TRIM",
        "LEN",
        "MID",
        "TODAY",
        "DATE",
        "YEAR",
        "MONTH",
        "DAY",
        "MATCH",
        "INDEX",
        "XLOOKUP",
        "VLOOKUP",
        "IFERROR",
        "TRUE",
        "FALSE",
        "UNIQUE",
        "COUNTUNIQUE",
    ];

    for word in formula.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() {
            continue;
        }
        // Skip if starts with number
        if word.chars().next().unwrap().is_numeric() {
            continue;
        }
        // Skip function names
        if functions.contains(&word.to_uppercase().as_str()) {
            continue;
        }
        // Skip if already added
        if !refs.contains(&word.to_string()) {
            refs.push(word.to_string());
        }
    }

    refs
}

/// Print a dependency with indentation
fn print_dependency(dep: &AuditDependency, indent: usize) {
    let prefix = "   ".repeat(indent);
    let arrow = if indent > 0 { "└─ " } else { "" };

    print!("{}{}{} ", prefix, arrow, dep.name.bright_blue());
    print!("({})", dep.dep_type.cyan());

    if let Some(val) = dep.value {
        print!(" = {}", format_number(val).green());
    }

    if let Some(ref f) = dep.formula {
        print!(" {}", f.yellow());
    }

    println!();

    for child in &dep.children {
        print_dependency(child, indent + 1);
    }
}

/// Execute the validate command for one or more files
pub fn validate(files: Vec<PathBuf>) -> ForgeResult<()> {
    let file_count = files.len();
    let is_batch = file_count > 1;

    if is_batch {
        println!(
            "{}",
            format!("✅ Validating {} files", file_count).bold().green()
        );
        println!();
    }

    let mut all_passed = true;
    let mut failed_files: Vec<String> = Vec::new();

    for file in &files {
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
            }
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
            }
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

    // Validate tables
    for (name, table) in &model.tables {
        if let Err(e) = table.validate_lengths() {
            println!(
                "\n{}",
                format!("❌ Table '{}' validation failed: {}", name, e)
                    .bold()
                    .red()
            );
            return Err(ForgeError::Validation(format!(
                "Table '{}' validation failed: {}",
                name, e
            )));
        }
    }

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
        }
    };

    // Compare calculated values vs. current values in file
    let mut mismatches = Vec::new();
    const TOLERANCE: f64 = 0.0001; // Floating point comparison tolerance

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
            println!(
                "      Current:  {}",
                format_number(*current).to_string().red()
            );
            println!(
                "      Expected: {}",
                format_number(*expected).to_string().green()
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

/// Execute the export command
pub fn export(input: PathBuf, output: PathBuf, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Export".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}\n", output.display());

    // Parse the YAML file
    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    let model = parser::parse_model(&input)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars\n",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Export to Excel
    if verbose {
        println!("{}", "📊 Exporting to Excel...".cyan());
    }

    let exporter = ExcelExporter::new(model);
    exporter.export(&output)?;

    println!("{}", "✅ Export Complete!".bold().green());
    println!("   Excel file: {}\n", output.display());

    println!("{}", "✅ Phase 3: Excel Export Complete!".bold().green());
    println!("   ✅ Table columns → Excel columns");
    println!("   ✅ Data values exported");
    println!("   ✅ Multiple worksheets");
    println!("   ✅ Scalars worksheet");
    println!("   ✅ Row formulas → Excel cell formulas (=A2-B2)");
    println!("   ✅ Cross-table references (=Sheet!Column)");
    println!("   ✅ Supports 60+ Excel functions (IFERROR, SUMIF, VLOOKUP, etc.)\n");

    Ok(())
}

/// Execute the import command
pub fn import(
    input: PathBuf,
    output: PathBuf,
    verbose: bool,
    split_files: bool,
    multi_doc: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Import".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}", output.display());
    if split_files {
        println!("   Mode:   Split files (one YAML per sheet)");
    } else if multi_doc {
        println!("   Mode:   Multi-document YAML");
    }
    println!();

    // Import Excel file
    if verbose {
        println!("{}", "📖 Reading Excel file...".cyan());
    }

    let importer = ExcelImporter::new(&input);
    let model = importer.import()?;

    if verbose {
        println!("   Found {} tables", model.tables.len());
        println!("   Found {} scalars\n", model.scalars.len());

        for (table_name, table) in &model.tables {
            println!("   📊 Table: {}", table_name.bright_blue());
            println!(
                "      {} columns, {} rows",
                table.columns.len(),
                table.row_count()
            );
        }
        println!();
    }

    // Write YAML file(s) based on mode
    if verbose {
        println!("{}", "💾 Writing YAML file(s)...".cyan());
    }

    if split_files {
        // Create output directory if it doesn't exist
        fs::create_dir_all(&output).map_err(ForgeError::Io)?;

        // Write each table to a separate file
        for (table_name, table) in &model.tables {
            let mut table_model = crate::types::ParsedModel::new();
            table_model.tables.insert(table_name.clone(), table.clone());

            let file_path = output.join(format!("{}.yaml", table_name));
            let yaml_string = format!(
                "_forge_version: \"1.0.0\"\n_name: \"{}\"\n\n{}",
                table_name,
                serde_yaml::to_string(&table_model.tables).map_err(ForgeError::Yaml)?
            );
            fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;

            if verbose {
                println!("   Created: {}", file_path.display());
            }
        }

        // Write scalars to separate file if present
        if !model.scalars.is_empty() {
            let file_path = output.join("scalars.yaml");
            let mut scalar_model = crate::types::ParsedModel::new();
            scalar_model.scalars = model.scalars.clone();

            let yaml_string = format!(
                "_forge_version: \"1.0.0\"\n_name: \"scalars\"\n\n{}",
                serde_yaml::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?
            );
            fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;

            if verbose {
                println!("   Created: {}", file_path.display());
            }
        }

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   Output directory: {}\n", output.display());
    } else if multi_doc {
        // Write as multi-document YAML
        let mut yaml_output = String::new();

        for (table_name, table) in &model.tables {
            let mut table_model = crate::types::ParsedModel::new();
            table_model.tables.insert(table_name.clone(), table.clone());

            yaml_output.push_str("---\n");
            yaml_output.push_str("_forge_version: \"1.0.0\"\n");
            yaml_output.push_str(&format!("_name: \"{}\"\n\n", table_name));
            yaml_output
                .push_str(&serde_yaml::to_string(&table_model.tables).map_err(ForgeError::Yaml)?);
            yaml_output.push('\n');
        }

        // Add scalars as separate document if present
        if !model.scalars.is_empty() {
            let mut scalar_model = crate::types::ParsedModel::new();
            scalar_model.scalars = model.scalars.clone();

            yaml_output.push_str("---\n");
            yaml_output.push_str("_forge_version: \"1.0.0\"\n");
            yaml_output.push_str("_name: \"scalars\"\n\n");
            yaml_output
                .push_str(&serde_yaml::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?);
        }

        fs::write(&output, yaml_output).map_err(ForgeError::Io)?;

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    } else {
        // Default: single file with all tables
        let yaml_string = serde_yaml::to_string(&model).map_err(ForgeError::Yaml)?;
        fs::write(&output, yaml_string).map_err(ForgeError::Io)?;

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    }

    println!("{}", "✅ Phase 4: Excel Import Complete!".bold().green());
    println!("   ✅ Excel worksheets → YAML tables");
    println!("   ✅ Data values imported");
    if split_files {
        println!("   ✅ Multiple worksheets → Separate YAML files (v4.4.2)");
    } else if multi_doc {
        println!("   ✅ Multiple worksheets → Multi-document YAML (v4.4.2)");
    } else {
        println!("   ✅ Multiple worksheets → One YAML file");
    }
    println!("   ✅ Scalars sheet detected");
    println!("   ✅ Formula translation (Excel → YAML syntax)");
    println!("   ✅ Supports 60+ Excel functions (IFERROR, SUMIF, VLOOKUP, etc.)\n");

    Ok(())
}

/// Execute the watch command
///
/// # Coverage Exclusion (ADR-006)
/// Contains infinite loop waiting for file system events - cannot unit test.
/// Tested via: cli_integration_tests.rs (manual termination after initial run)
#[cfg(not(coverage))]
pub fn watch(file: PathBuf, validate_only: bool, verbose: bool) -> ForgeResult<()> {
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
        .map_err(|e| ForgeError::Validation(format!("Failed to create file watcher: {}", e)))?;

    // Watch the parent directory (watches all YAML files in that directory)
    debouncer
        .watcher()
        .watch(parent_dir, RecursiveMode::NonRecursive)
        .map_err(|e| ForgeError::Validation(format!("Failed to watch directory: {}", e)))?;

    if verbose {
        println!(
            "   {} {}",
            "Watching directory:".cyan(),
            parent_dir.display()
        );
    }

    // Run initial validation/calculation
    println!("{}", "🔄 Initial run...".cyan());
    run_watch_action(&file, validate_only, verbose);
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
                        if let Some(name_str) = filename.to_str() {
                            if name_str.ends_with(".yaml") || name_str.ends_with(".yml") {
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
                    run_watch_action(&file, validate_only, verbose);
                    println!();
                }
            }
            Ok(Err(error)) => {
                eprintln!("{} Watch error: {}", "❌".red(), error);
            }
            Err(e) => {
                eprintln!("{} Channel error: {}", "❌".red(), e);
                break;
            }
        }
    }

    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn watch(file: PathBuf, _validate_only: bool, _verbose: bool) -> ForgeResult<()> {
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
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
}

/// Run the watch action (validate or calculate)
fn run_watch_action(file: &Path, validate_only: bool, verbose: bool) {
    if validate_only {
        match validate_internal(file, verbose) {
            Ok(_) => println!("{}", "✅ Validation passed".bold().green()),
            Err(e) => println!("{} {}", "❌ Validation failed:".bold().red(), e),
        }
    } else {
        match calculate_internal(file, verbose) {
            Ok(_) => println!("{}", "✅ Calculation complete".bold().green()),
            Err(e) => println!("{} {}", "❌ Calculation failed:".bold().red(), e),
        }
    }
}

/// Internal validation function for watch mode
fn validate_internal(file: &Path, verbose: bool) -> ForgeResult<()> {
    let model = parser::parse_model(file)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Validate tables
    for (name, table) in &model.tables {
        table.validate_lengths().map_err(|e| {
            ForgeError::Validation(format!("Table '{}' validation failed: {}", name, e))
        })?;
    }

    // Calculate and compare
    let calculator = ArrayCalculator::new(model.clone());
    let calculated = calculator.calculate_all()?;

    // Check for mismatches
    const TOLERANCE: f64 = 0.0001;
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
                format!("  {} current={} expected={}", name, current, expected)
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
fn apply_scenario(model: &mut crate::types::ParsedModel, scenario_name: &str) -> ForgeResult<()> {
    let scenario = model.scenarios.get(scenario_name).ok_or_else(|| {
        let available: Vec<_> = model.scenarios.keys().collect();
        ForgeError::Validation(format!(
            "Scenario '{}' not found. Available scenarios: {:?}",
            scenario_name, available
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

/// Execute the compare command - compare results across scenarios
pub fn compare(file: PathBuf, scenarios: Vec<String>, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Scenario Comparison".bold().green());
    println!("   File: {}", file.display());
    println!(
        "   Scenarios: {}\n",
        scenarios.join(", ").bright_yellow().bold()
    );

    // Parse model
    let base_model = parser::parse_model(&file)?;

    // Validate scenarios exist
    for scenario_name in &scenarios {
        if !base_model.scenarios.contains_key(scenario_name) {
            let available: Vec<_> = base_model.scenarios.keys().collect();
            return Err(ForgeError::Validation(format!(
                "Scenario '{}' not found. Available: {:?}",
                scenario_name, available
            )));
        }
    }

    if verbose {
        println!(
            "   Found {} tables, {} scalars, {} scenarios",
            base_model.tables.len(),
            base_model.scalars.len(),
            base_model.scenarios.len()
        );
    }

    // Calculate results for each scenario
    let mut results: Vec<(String, crate::types::ParsedModel)> = Vec::new();

    for scenario_name in &scenarios {
        let mut model = base_model.clone();
        apply_scenario(&mut model, scenario_name)?;

        let calculator = ArrayCalculator::new(model);
        let calculated = calculator.calculate_all()?;
        results.push((scenario_name.clone(), calculated));
    }

    // Collect all scalar names
    let mut all_scalars: Vec<String> = results
        .iter()
        .flat_map(|(_, m)| m.scalars.keys().cloned())
        .collect();
    all_scalars.sort();
    all_scalars.dedup();

    // Print comparison table
    println!("\n{}", "📊 Scenario Comparison:".bold().cyan());
    println!("{}", "─".repeat(20 + scenarios.len() * 15));

    // Header row
    print!("{:<20}", "Variable".bold());
    for scenario_name in &scenarios {
        print!("{:>15}", scenario_name.bright_yellow().bold());
    }
    println!();
    println!("{}", "─".repeat(20 + scenarios.len() * 15));

    // Data rows
    for scalar_name in &all_scalars {
        print!("{:<20}", scalar_name.bright_blue());

        for (_, result_model) in &results {
            if let Some(var) = result_model.scalars.get(scalar_name) {
                if let Some(value) = var.value {
                    print!("{:>15}", format_number(value).green());
                } else {
                    print!("{:>15}", "-".dimmed());
                }
            } else {
                print!("{:>15}", "-".dimmed());
            }
        }
        println!();
    }

    println!("{}", "─".repeat(20 + scenarios.len() * 15));
    println!("\n{}", "✅ Comparison complete".bold().green());

    Ok(())
}

/// Variance result for a single variable
#[derive(Debug, Clone)]
struct VarianceResult {
    name: String,
    budget: f64,
    actual: f64,
    variance: f64,
    variance_pct: f64,
    is_favorable: bool,
    exceeds_threshold: bool,
}

/// Execute the variance command - budget vs actual analysis
pub fn variance(
    budget_path: PathBuf,
    actual_path: PathBuf,
    threshold: f64,
    output: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Variance Analysis".bold().green());
    println!("   Budget: {}", budget_path.display());
    println!("   Actual: {}", actual_path.display());
    println!("   Threshold: {}%\n", threshold);

    // Parse both files
    if verbose {
        println!("{}", "📖 Parsing YAML files...".cyan());
    }

    let budget_model = parser::parse_model(&budget_path)?;
    let actual_model = parser::parse_model(&actual_path)?;

    // Calculate both models
    if verbose {
        println!("{}", "🧮 Calculating formulas...".cyan());
    }

    let budget_calculator = ArrayCalculator::new(budget_model);
    let budget_result = budget_calculator.calculate_all()?;

    let actual_calculator = ArrayCalculator::new(actual_model);
    let actual_result = actual_calculator.calculate_all()?;

    // Collect scalar variances
    let mut variances: Vec<VarianceResult> = Vec::new();

    // Get all scalar names from both models
    let mut all_scalars: Vec<String> = budget_result
        .scalars
        .keys()
        .chain(actual_result.scalars.keys())
        .cloned()
        .collect();
    all_scalars.sort();
    all_scalars.dedup();

    for name in &all_scalars {
        let budget_val = budget_result
            .scalars
            .get(name)
            .and_then(|v| v.value)
            .unwrap_or(0.0);
        let actual_val = actual_result
            .scalars
            .get(name)
            .and_then(|v| v.value)
            .unwrap_or(0.0);

        let variance_abs = actual_val - budget_val;
        let variance_pct = if budget_val.abs() > 0.0001 {
            (variance_abs / budget_val) * 100.0
        } else {
            0.0
        };

        // Determine favorability (heuristic based on name)
        let is_expense = name.to_lowercase().contains("expense")
            || name.to_lowercase().contains("cost")
            || name.to_lowercase().contains("cogs");
        let is_favorable = if is_expense {
            actual_val <= budget_val // Lower expenses = favorable
        } else {
            actual_val >= budget_val // Higher revenue/profit = favorable
        };

        let exceeds_threshold = variance_pct.abs() >= threshold;

        variances.push(VarianceResult {
            name: name.clone(),
            budget: budget_val,
            actual: actual_val,
            variance: variance_abs,
            variance_pct,
            is_favorable,
            exceeds_threshold,
        });
    }

    // Handle output
    if let Some(output_path) = output {
        let extension = output_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "xlsx" => {
                export_variance_to_excel(&output_path, &variances, threshold)?;
                println!(
                    "{}",
                    format!("✅ Variance report exported to {}", output_path.display())
                        .bold()
                        .green()
                );
            }
            "yaml" | "yml" => {
                export_variance_to_yaml(&output_path, &variances, threshold)?;
                println!(
                    "{}",
                    format!("✅ Variance report exported to {}", output_path.display())
                        .bold()
                        .green()
                );
            }
            _ => {
                return Err(ForgeError::Export(format!(
                    "Unsupported output format: {}. Use .xlsx or .yaml",
                    extension
                )));
            }
        }
    } else {
        // Print to terminal
        print_variance_table(&variances, threshold);
    }

    // Summary
    let favorable_count = variances.iter().filter(|v| v.is_favorable).count();
    let unfavorable_count = variances.len() - favorable_count;
    let alert_count = variances.iter().filter(|v| v.exceeds_threshold).count();

    println!();
    println!(
        "   {} Favorable: {}  {} Unfavorable: {}  {} Alerts (>{:.0}%): {}",
        "✅".green(),
        favorable_count.to_string().green(),
        "❌".red(),
        unfavorable_count.to_string().red(),
        "⚠️".yellow(),
        threshold,
        alert_count.to_string().yellow()
    );

    Ok(())
}

/// Print variance results as a table
fn print_variance_table(variances: &[VarianceResult], threshold: f64) {
    println!("\n{}", "📊 Budget vs Actual Variance:".bold().cyan());
    println!("{}", "─".repeat(85));

    // Header
    println!(
        "{:<20} {:>12} {:>12} {:>12} {:>10} {:>8}",
        "Variable".bold(),
        "Budget".bold(),
        "Actual".bold(),
        "Variance".bold(),
        "Var %".bold(),
        "Status".bold()
    );
    println!("{}", "─".repeat(85));

    // Data rows
    for v in variances {
        let var_str = format_number(v.variance);
        let pct_str = format!("{:.1}%", v.variance_pct);

        let status = if v.exceeds_threshold && !v.is_favorable {
            "⚠️ ❌".to_string()
        } else if v.exceeds_threshold {
            "⚠️ ✅".to_string()
        } else if v.is_favorable {
            "✅".to_string()
        } else {
            "❌".to_string()
        };

        // Color the variance based on favorability
        let var_colored = if v.is_favorable {
            var_str.green()
        } else {
            var_str.red()
        };
        let pct_colored = if v.is_favorable {
            pct_str.green()
        } else {
            pct_str.red()
        };

        println!(
            "{:<20} {:>12} {:>12} {:>12} {:>10} {:>8}",
            v.name.bright_blue(),
            format_number(v.budget),
            format_number(v.actual),
            var_colored,
            pct_colored,
            status
        );
    }

    println!("{}", "─".repeat(85));
    println!("   {} = exceeds {:.0}% threshold", "⚠️".yellow(), threshold);
}

/// Export variance report to Excel
fn export_variance_to_excel(
    output: &Path,
    variances: &[VarianceResult],
    threshold: f64,
) -> ForgeResult<()> {
    use rust_xlsxwriter::{Format, Workbook};

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Set column widths
    worksheet.set_column_width(0, 20).ok();
    worksheet.set_column_width(1, 12).ok();
    worksheet.set_column_width(2, 12).ok();
    worksheet.set_column_width(3, 12).ok();
    worksheet.set_column_width(4, 10).ok();
    worksheet.set_column_width(5, 10).ok();

    // Header format
    let header_format = Format::new().set_bold();

    // Headers
    worksheet
        .write_string_with_format(0, 0, "Variable", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 1, "Budget", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 2, "Actual", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 3, "Variance", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 4, "Var %", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 5, "Status", &header_format)
        .ok();

    // Data rows
    for (i, v) in variances.iter().enumerate() {
        let row = (i + 1) as u32;

        worksheet.write_string(row, 0, &v.name).ok();
        worksheet.write_number(row, 1, v.budget).ok();
        worksheet.write_number(row, 2, v.actual).ok();
        worksheet.write_number(row, 3, v.variance).ok();
        worksheet.write_number(row, 4, v.variance_pct / 100.0).ok(); // As decimal for %

        let status = if v.exceeds_threshold && !v.is_favorable {
            "ALERT - Unfavorable"
        } else if v.exceeds_threshold {
            "ALERT - Favorable"
        } else if v.is_favorable {
            "Favorable"
        } else {
            "Unfavorable"
        };
        worksheet.write_string(row, 5, status).ok();
    }

    // Add metadata row
    let meta_row = (variances.len() + 3) as u32;
    worksheet
        .write_string(meta_row, 0, format!("Threshold: {}%", threshold))
        .ok();
    worksheet
        .write_string(meta_row + 1, 0, "Generated by Forge v2.3.0")
        .ok();

    workbook
        .save(output)
        .map_err(|e| ForgeError::Export(e.to_string()))?;

    Ok(())
}

/// Export variance report to YAML
fn export_variance_to_yaml(
    output: &Path,
    variances: &[VarianceResult],
    threshold: f64,
) -> ForgeResult<()> {
    use std::io::Write;

    let mut content = String::new();
    content.push_str("# Forge Variance Analysis Report\n");
    content.push_str("# Generated by Forge v2.3.0\n");
    content.push_str(&format!("# Threshold: {}%\n\n", threshold));

    content.push_str("metadata:\n");
    content.push_str(&format!("  threshold_pct: {}\n", threshold));
    content.push_str(&format!("  total_items: {}\n", variances.len()));
    content.push_str(&format!(
        "  favorable_count: {}\n",
        variances.iter().filter(|v| v.is_favorable).count()
    ));
    content.push_str(&format!(
        "  alert_count: {}\n\n",
        variances.iter().filter(|v| v.exceeds_threshold).count()
    ));

    content.push_str("variances:\n");
    for v in variances {
        content.push_str(&format!("  {}:\n", v.name));
        content.push_str(&format!("    budget: {}\n", v.budget));
        content.push_str(&format!("    actual: {}\n", v.actual));
        content.push_str(&format!("    variance: {}\n", v.variance));
        content.push_str(&format!("    variance_pct: {:.2}\n", v.variance_pct));
        content.push_str(&format!("    is_favorable: {}\n", v.is_favorable));
        content.push_str(&format!("    exceeds_threshold: {}\n", v.exceeds_threshold));
    }

    let mut file = fs::File::create(output)
        .map_err(|e| ForgeError::Export(format!("Failed to create file: {}", e)))?;
    file.write_all(content.as_bytes())
        .map_err(|e| ForgeError::Export(format!("Failed to write file: {}", e)))?;

    Ok(())
}

/// Parse a range string "start,end,step" into a vector of values
fn parse_range(range: &str) -> ForgeResult<Vec<f64>> {
    let parts: Vec<&str> = range.split(',').collect();
    if parts.len() != 3 {
        return Err(ForgeError::Validation(format!(
            "Invalid range format '{}'. Expected: start,end,step (e.g., 0.01,0.15,0.02)",
            range
        )));
    }

    let start: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid start value: '{}'", parts[0])))?;
    let end: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid end value: '{}'", parts[1])))?;
    let step: f64 = parts[2]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid step value: '{}'", parts[2])))?;

    if step <= 0.0 {
        return Err(ForgeError::Validation("Step must be positive".to_string()));
    }
    if start > end {
        return Err(ForgeError::Validation(
            "Start must be less than or equal to end".to_string(),
        ));
    }

    let mut values = Vec::new();
    let mut current = start;
    while current <= end + step * 0.001 {
        // Small tolerance for floating point
        values.push(current);
        current += step;
    }

    Ok(values)
}

/// Calculate model with a specific variable override and return the output value
fn calculate_with_override(
    base_model: &crate::types::ParsedModel,
    var_name: &str,
    var_value: f64,
    output_name: &str,
) -> ForgeResult<f64> {
    let mut model = base_model.clone();

    // Override the variable
    if let Some(scalar) = model.scalars.get_mut(var_name) {
        scalar.value = Some(var_value);
        scalar.formula = None; // Clear formula since we're using override
    } else {
        // Create new scalar
        model.scalars.insert(
            var_name.to_string(),
            crate::types::Variable::new(var_name.to_string(), Some(var_value), None),
        );
    }

    // Calculate
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Get output value
    if let Some(scalar) = result.scalars.get(output_name) {
        scalar.value.ok_or_else(|| {
            ForgeError::Validation(format!("Output variable '{}' has no value", output_name))
        })
    } else {
        Err(ForgeError::Validation(format!(
            "Output variable '{}' not found in model",
            output_name
        )))
    }
}

/// Execute the sensitivity command
pub fn sensitivity(
    file: PathBuf,
    vary: String,
    range: String,
    vary2: Option<String>,
    range2: Option<String>,
    output: String,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Sensitivity Analysis".bold().green());
    println!("   File: {}", file.display());
    println!("   Vary: {} ({})", vary.bright_yellow(), range);
    if let Some(ref v2) = vary2 {
        println!(
            "   Vary2: {} ({})",
            v2.bright_yellow(),
            range2.as_deref().unwrap_or("?")
        );
    }
    println!("   Output: {}\n", output.bright_blue());

    // Parse model
    let base_model = parser::parse_model(&file)?;

    // Validate that vary variable exists
    if !base_model.scalars.contains_key(&vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    // Parse range
    let values1 = parse_range(&range)?;

    if verbose {
        println!(
            "   Range 1: {} values from {} to {}",
            values1.len(),
            values1.first().unwrap_or(&0.0),
            values1.last().unwrap_or(&0.0)
        );
    }

    // Two-variable analysis
    if let (Some(ref v2), Some(ref r2)) = (&vary2, &range2) {
        // Validate second variable
        if !base_model.scalars.contains_key(v2) {
            return Err(ForgeError::Validation(format!(
                "Variable '{}' not found. Available scalars: {:?}",
                v2,
                base_model.scalars.keys().collect::<Vec<_>>()
            )));
        }

        let values2 = parse_range(r2)?;

        if verbose {
            println!(
                "   Range 2: {} values from {} to {}",
                values2.len(),
                values2.first().unwrap_or(&0.0),
                values2.last().unwrap_or(&0.0)
            );
        }

        // Calculate matrix
        println!(
            "\n{} {} → {}",
            "📊 Sensitivity Matrix:".bold().cyan(),
            format!("({}, {})", vary, v2).yellow(),
            output.bright_blue()
        );

        // Header row
        print!("{:>12}", vary.bright_yellow());
        for val2 in &values2 {
            print!("{:>12}", format!("{:.4}", val2).dimmed());
        }
        println!();
        println!("{}", "─".repeat(12 + values2.len() * 12));

        // Data rows
        for val1 in &values1 {
            print!("{:>12}", format!("{:.4}", val1).bright_yellow());

            for val2 in &values2 {
                // Override both variables
                let mut model = base_model.clone();

                if let Some(s) = model.scalars.get_mut(&vary) {
                    s.value = Some(*val1);
                    s.formula = None;
                }
                if let Some(s) = model.scalars.get_mut(v2) {
                    s.value = Some(*val2);
                    s.formula = None;
                }

                let calculator = ArrayCalculator::new(model);
                match calculator.calculate_all() {
                    Ok(result) => {
                        if let Some(scalar) = result.scalars.get(&output) {
                            if let Some(v) = scalar.value {
                                print!("{:>12}", format_number(v).green());
                            } else {
                                print!("{:>12}", "-".dimmed());
                            }
                        } else {
                            print!("{:>12}", "?".red());
                        }
                    }
                    Err(_) => {
                        print!("{:>12}", "ERR".red());
                    }
                }
            }
            println!();
        }
    } else {
        // One-variable analysis
        println!(
            "\n{} {} → {}",
            "📊 Sensitivity Table:".bold().cyan(),
            vary.yellow(),
            output.bright_blue()
        );
        println!("{}", "─".repeat(30));
        println!("{:>12} {:>15}", vary.bold(), output.bold());
        println!("{}", "─".repeat(30));

        for val in &values1 {
            match calculate_with_override(&base_model, &vary, *val, &output) {
                Ok(result) => {
                    println!(
                        "{:>12} {:>15}",
                        format!("{:.4}", val).bright_yellow(),
                        format_number(result).green()
                    );
                }
                Err(e) => {
                    println!(
                        "{:>12} {:>15}",
                        format!("{:.4}", val).bright_yellow(),
                        format!("ERR: {}", e).red()
                    );
                }
            }
        }
        println!("{}", "─".repeat(30));
    }

    println!("\n{}", "✅ Sensitivity analysis complete".bold().green());
    Ok(())
}

/// Execute the goal-seek command
#[allow(clippy::too_many_arguments)]
pub fn goal_seek(
    file: PathBuf,
    target: String,
    value: f64,
    vary: String,
    min: Option<f64>,
    max: Option<f64>,
    tolerance: f64,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Goal Seek".bold().green());
    println!("   File: {}", file.display());
    println!("   Target: {} = {}", target.bright_blue(), value);
    println!("   Vary: {}", vary.bright_yellow());
    println!("   Tolerance: {}\n", tolerance);

    // Parse model
    let base_model = parser::parse_model(&file)?;

    // Validate variables
    if !base_model.scalars.contains_key(&vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    // Get current value of vary to set default bounds
    let current_value = base_model
        .scalars
        .get(&vary)
        .and_then(|s| s.value)
        .unwrap_or(1.0);

    // Set bounds (default: 0.01x to 100x current value)
    let lower = min.unwrap_or_else(|| {
        if current_value > 0.0 {
            current_value * 0.01
        } else if current_value < 0.0 {
            current_value * 100.0
        } else {
            -1000.0
        }
    });
    let upper = max.unwrap_or(if current_value > 0.0 {
        current_value * 100.0
    } else if current_value < 0.0 {
        current_value * 0.01
    } else {
        1000.0
    });

    if verbose {
        println!("   Current value of {}: {}", vary, current_value);
        println!("   Search bounds: [{}, {}]", lower, upper);
    }

    // Bisection method
    let max_iterations = 100;
    let mut low = lower;
    let mut high = upper;

    // Check bounds first
    let f_low = calculate_with_override(&base_model, &vary, low, &target)? - value;
    let f_high = calculate_with_override(&base_model, &vary, high, &target)? - value;

    if verbose {
        println!("   f({}) = {} (target diff: {})", low, f_low + value, f_low);
        println!(
            "   f({}) = {} (target diff: {})",
            high,
            f_high + value,
            f_high
        );
    }

    // Check if solution exists in range (signs should differ)
    if f_low * f_high > 0.0 {
        // Try to find a better range by expanding
        println!(
            "{}",
            "⚠️  No sign change in initial range - expanding search...".yellow()
        );

        // Try expanding the range
        let mut found = false;
        for factor in [10.0, 100.0, 1000.0] {
            let exp_low = if lower > 0.0 {
                lower / factor
            } else {
                lower * factor
            };
            let exp_high = if upper > 0.0 {
                upper * factor
            } else {
                upper / factor
            };

            let f_exp_low = calculate_with_override(&base_model, &vary, exp_low, &target)? - value;
            let f_exp_high =
                calculate_with_override(&base_model, &vary, exp_high, &target)? - value;

            if f_exp_low * f_exp_high <= 0.0 {
                low = exp_low;
                high = exp_high;
                found = true;
                if verbose {
                    println!("   Found valid range: [{}, {}]", low, high);
                }
                break;
            }
        }

        if !found {
            return Err(ForgeError::Validation(format!(
                "No solution found in search range. The target value {} may not be achievable by varying '{}'.",
                value, vary
            )));
        }
    }

    // Bisection iteration
    let mut mid = (low + high) / 2.0;
    let mut iteration = 0;

    while (high - low) > tolerance && iteration < max_iterations {
        mid = (low + high) / 2.0;
        let f_mid = calculate_with_override(&base_model, &vary, mid, &target)? - value;

        if verbose && iteration % 10 == 0 {
            println!(
                "   Iteration {}: {} = {} (diff: {:.6})",
                iteration,
                vary,
                mid,
                f_mid.abs()
            );
        }

        let f_low_check = calculate_with_override(&base_model, &vary, low, &target)? - value;

        if f_mid.abs() < tolerance {
            break;
        }

        if f_low_check * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
        }

        iteration += 1;
    }

    // Final result
    let final_value = calculate_with_override(&base_model, &vary, mid, &target)?;

    println!("{}", "─".repeat(50));
    println!(
        "{}",
        format!("🎯 Solution found in {} iterations:", iteration)
            .bold()
            .green()
    );
    println!(
        "   {} = {} → {} = {}",
        vary.bright_yellow().bold(),
        format_number(mid).bold().green(),
        target.bright_blue(),
        format_number(final_value).green()
    );

    let error = (final_value - value).abs();
    if error < tolerance {
        println!("   {} Within tolerance", "✅".green());
    } else {
        println!(
            "   {} Error: {} (tolerance: {})",
            "⚠️".yellow(),
            error,
            tolerance
        );
    }

    println!("{}", "─".repeat(50));
    Ok(())
}

/// Execute the break-even command
pub fn break_even(
    file: PathBuf,
    output: String,
    vary: String,
    min: Option<f64>,
    max: Option<f64>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Break-Even Analysis".bold().green());
    println!("   Finding where {} = 0\n", output.bright_blue());

    // Break-even is just goal-seek with value = 0
    goal_seek(file, output, 0.0, vary, min, max, 0.0001, verbose)
}

/// Function category with functions and descriptions
struct FunctionCategory {
    name: &'static str,
    functions: Vec<(&'static str, &'static str)>,
}

/// Execute the functions command - list all supported Excel-compatible functions
pub fn functions(json_output: bool) -> ForgeResult<()> {
    let categories = vec![
        FunctionCategory {
            name: "Financial",
            functions: vec![
                ("NPV", "Net Present Value - =NPV(rate, cashflow1, cashflow2, ...)"),
                ("IRR", "Internal Rate of Return - =IRR(values, [guess])"),
                ("MIRR", "Modified IRR - =MIRR(values, finance_rate, reinvest_rate)"),
                ("XNPV", "NPV with irregular dates - =XNPV(rate, values, dates)"),
                ("XIRR", "IRR with irregular dates - =XIRR(values, dates, [guess])"),
                ("PMT", "Payment for a loan - =PMT(rate, nper, pv, [fv], [type])"),
                ("PV", "Present Value - =PV(rate, nper, pmt, [fv], [type])"),
                ("FV", "Future Value - =FV(rate, nper, pmt, [pv], [type])"),
                ("RATE", "Interest rate - =RATE(nper, pmt, pv, [fv], [type], [guess])"),
                ("NPER", "Number of periods - =NPER(rate, pmt, pv, [fv], [type])"),
                ("SLN", "Straight-line depreciation - =SLN(cost, salvage, life)"),
                ("DB", "Declining balance depreciation - =DB(cost, salvage, life, period)"),
                ("DDB", "Double declining balance - =DDB(cost, salvage, life, period)"),
            ],
        },
        FunctionCategory {
            name: "Lookup",
            functions: vec![
                ("MATCH", "Find position in array - =MATCH(value, array, [type])"),
                ("INDEX", "Get value by position - =INDEX(array, row, [col])"),
                ("VLOOKUP", "Vertical lookup - =VLOOKUP(value, table, col, [approx])"),
                ("XLOOKUP", "Modern lookup - =XLOOKUP(value, lookup, return, [not_found], [match], [search])"),
                ("CHOOSE", "Pick nth value - =CHOOSE(index, value1, value2, ...)"),
                ("OFFSET", "Dynamic range slice - =OFFSET(array, rows, [height])"),
            ],
        },
        FunctionCategory {
            name: "Conditional",
            functions: vec![
                ("SUMIF", "Sum if condition - =SUMIF(range, criteria, [sum_range])"),
                ("COUNTIF", "Count if condition - =COUNTIF(range, criteria)"),
                ("AVERAGEIF", "Average if condition - =AVERAGEIF(range, criteria, [avg_range])"),
                ("SUMIFS", "Sum with multiple conditions - =SUMIFS(sum_range, range1, criteria1, ...)"),
                ("COUNTIFS", "Count with multiple conditions - =COUNTIFS(range1, criteria1, ...)"),
                ("AVERAGEIFS", "Average with multiple conditions - =AVERAGEIFS(avg_range, range1, criteria1, ...)"),
                ("MAXIFS", "Max with conditions - =MAXIFS(max_range, range1, criteria1, ...)"),
                ("MINIFS", "Min with conditions - =MINIFS(min_range, range1, criteria1, ...)"),
            ],
        },
        FunctionCategory {
            name: "Array",
            functions: vec![
                ("UNIQUE", "Get unique values - =UNIQUE(array)"),
                ("COUNTUNIQUE", "Count unique values - =COUNTUNIQUE(array)"),
                ("FILTER", "Filter by criteria - =FILTER(array, include)"),
                ("SORT", "Sort values - =SORT(array, [order])"),
            ],
        },
        FunctionCategory {
            name: "Aggregation",
            functions: vec![
                ("SUM", "Sum values - =SUM(value1, value2, ...)"),
                ("AVERAGE", "Average values - =AVERAGE(value1, value2, ...) or =AVG(...)"),
                ("MIN", "Minimum value - =MIN(value1, value2, ...)"),
                ("MAX", "Maximum value - =MAX(value1, value2, ...)"),
                ("COUNT", "Count values - =COUNT(array)"),
            ],
        },
        FunctionCategory {
            name: "Math",
            functions: vec![
                ("ROUND", "Round to digits - =ROUND(value, digits)"),
                ("ROUNDUP", "Round up - =ROUNDUP(value, digits)"),
                ("ROUNDDOWN", "Round down - =ROUNDDOWN(value, digits)"),
                ("CEILING", "Round up to significance - =CEILING(value, significance)"),
                ("FLOOR", "Round down to significance - =FLOOR(value, significance)"),
                ("MOD", "Modulo/remainder - =MOD(value, divisor)"),
                ("SQRT", "Square root - =SQRT(value)"),
                ("POWER", "Power/exponent - =POWER(base, exponent)"),
                ("ABS", "Absolute value - =ABS(value)"),
            ],
        },
        FunctionCategory {
            name: "Text",
            functions: vec![
                ("CONCAT", "Concatenate strings - =CONCAT(text1, text2, ...)"),
                ("TRIM", "Remove extra spaces - =TRIM(text)"),
                ("UPPER", "Convert to uppercase - =UPPER(text)"),
                ("LOWER", "Convert to lowercase - =LOWER(text)"),
                ("LEN", "Length of text - =LEN(text)"),
                ("MID", "Extract substring - =MID(text, start, length)"),
            ],
        },
        FunctionCategory {
            name: "Date",
            functions: vec![
                ("TODAY", "Current date - =TODAY()"),
                ("DATE", "Create date - =DATE(year, month, day)"),
                ("YEAR", "Extract year - =YEAR(date)"),
                ("MONTH", "Extract month - =MONTH(date)"),
                ("DAY", "Extract day - =DAY(date)"),
                ("DATEDIF", "Date difference - =DATEDIF(start, end, unit)"),
                ("EDATE", "Add months to date - =EDATE(start, months)"),
                ("EOMONTH", "End of month - =EOMONTH(start, months)"),
                ("NETWORKDAYS", "Working days between dates - =NETWORKDAYS(start, end)"),
                ("WORKDAY", "Date after N working days - =WORKDAY(start, days)"),
                ("YEARFRAC", "Fraction of year - =YEARFRAC(start, end, [basis])"),
            ],
        },
        FunctionCategory {
            name: "Logic",
            functions: vec![
                ("IF", "Conditional - =IF(condition, true_value, false_value)"),
                ("AND", "Logical AND - =AND(condition1, condition2, ...)"),
                ("OR", "Logical OR - =OR(condition1, condition2, ...)"),
                ("LET", "Named variables - =LET(name, value, ..., calculation)"),
                ("SWITCH", "Multi-match - =SWITCH(expr, val1, result1, ..., [default])"),
                ("INDIRECT", "String to ref - =INDIRECT(\"table.column\")"),
                ("LAMBDA", "Anonymous func - =LAMBDA(x, x*2)(5)"),
            ],
        },
        FunctionCategory {
            name: "Statistical",
            functions: vec![
                ("MEDIAN", "Middle value - =MEDIAN(array)"),
                ("VAR", "Variance (sample) - =VAR(array)"),
                ("STDEV", "Standard deviation (sample) - =STDEV(array)"),
                ("PERCENTILE", "Percentile value - =PERCENTILE(array, k)"),
                ("QUARTILE", "Quartile value - =QUARTILE(array, quart)"),
                ("CORREL", "Correlation coefficient - =CORREL(array1, array2)"),
            ],
        },
        FunctionCategory {
            name: "Forge-Native",
            functions: vec![
                ("SCENARIO", "Get scenario value - =SCENARIO(name, variable)"),
                ("VARIANCE", "Budget variance - =VARIANCE(actual, budget)"),
                ("VARIANCE_PCT", "Variance percent - =VARIANCE_PCT(actual, budget)"),
                ("VARIANCE_STATUS", "Variance status - =VARIANCE_STATUS(actual, budget, [type])"),
                ("BREAKEVEN_UNITS", "Break-even units - =BREAKEVEN_UNITS(fixed, price, var_cost)"),
                ("BREAKEVEN_REVENUE", "Break-even revenue - =BREAKEVEN_REVENUE(fixed, margin_pct)"),
            ],
        },
    ];

    // Count total functions
    let total: usize = categories.iter().map(|c| c.functions.len()).sum();

    if json_output {
        // JSON output for tooling
        let json = serde_json::json!({
            "total": total,
            "categories": categories.iter().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "count": c.functions.len(),
                    "functions": c.functions.iter().map(|(name, desc)| {
                        serde_json::json!({
                            "name": name,
                            "description": desc
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        // Human-readable output
        println!("{}", "🔥 Forge - Supported Functions".bold().green());
        println!();
        println!(
            "{}",
            format!(
                "   {} Excel-compatible functions for financial modeling",
                total
            )
            .bright_white()
        );
        println!();
        println!("{}", "═".repeat(70));

        for category in &categories {
            println!();
            println!(
                "{} ({})",
                category.name.bold().cyan(),
                category.functions.len()
            );
            println!("{}", "─".repeat(70));

            for (name, desc) in &category.functions {
                println!("  {:12} {}", name.bold().yellow(), desc.bright_white());
            }
        }

        println!();
        println!("{}", "═".repeat(70));
        println!();
        println!(
            "{}",
            "Use these functions in your YAML formulas: formula: \"=NPV(0.1, cashflows)\""
                .bright_black()
        );
        println!();
    }

    Ok(())
}

/// Execute the upgrade command - migrate YAML files to latest schema version
pub fn upgrade(
    file: PathBuf,
    dry_run: bool,
    target_version: String,
    verbose: bool,
) -> ForgeResult<()> {
    use std::collections::HashSet;

    println!("{}", "🔥 Forge - Schema Upgrade".bold().green());
    println!();
    println!("   File:    {}", file.display());
    println!("   Target:  v{}", target_version);
    if dry_run {
        println!("   Mode:    {} (no files modified)", "DRY RUN".yellow());
    }
    println!();

    // Track upgraded files to avoid circular processing
    let mut upgraded_files: HashSet<PathBuf> = HashSet::new();

    // Perform upgrade recursively
    let changes = upgrade_file_recursive(
        &file,
        &target_version,
        dry_run,
        verbose,
        &mut upgraded_files,
    )?;

    // Summary
    println!();
    println!("{}", "═".repeat(70));
    println!();
    if dry_run {
        println!(
            "{} {} file(s) would be upgraded",
            "DRY RUN:".yellow().bold(),
            changes
        );
        println!();
        println!("   Run without --dry-run to apply changes.");
    } else {
        println!(
            "{} {} file(s) upgraded to v{}",
            "✅".green(),
            changes,
            target_version
        );
    }
    println!();

    Ok(())
}

/// Recursively upgrade a file and its includes
fn upgrade_file_recursive(
    file: &Path,
    target_version: &str,
    dry_run: bool,
    verbose: bool,
    upgraded_files: &mut std::collections::HashSet<PathBuf>,
) -> ForgeResult<usize> {
    // Canonicalize path to handle relative paths
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

    // Skip if already processed (circular include protection)
    if upgraded_files.contains(&canonical) {
        if verbose {
            println!(
                "   {} {} (already processed)",
                "⏭️".dimmed(),
                file.display()
            );
        }
        return Ok(0);
    }
    upgraded_files.insert(canonical.clone());

    // Read and parse the file
    let content = fs::read_to_string(file)
        .map_err(|e| ForgeError::IO(format!("Failed to read {}: {}", file.display(), e)))?;

    // Parse as YAML Value to manipulate
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| ForgeError::Parse(format!("Failed to parse {}: {}", file.display(), e)))?;

    let mut changes = 0;

    // First, recursively upgrade any included files
    if let Some(serde_yaml::Value::Sequence(include_list)) = yaml.get("_includes").cloned() {
        let parent_dir = file.parent().unwrap_or(Path::new("."));
        for include in include_list {
            if let Some(include_file) = include.get("file").and_then(|f| f.as_str()) {
                let include_path = parent_dir.join(include_file);
                if include_path.exists() {
                    changes += upgrade_file_recursive(
                        &include_path,
                        target_version,
                        dry_run,
                        verbose,
                        upgraded_files,
                    )?;
                }
            }
        }
    }

    // Get current version
    let current_version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    // Check if upgrade needed
    if current_version == target_version {
        if verbose {
            println!(
                "   {} {} (already v{})",
                "✓".green(),
                file.display(),
                target_version
            );
        }
        return Ok(changes);
    }

    println!(
        "   {} {} (v{} → v{})",
        if dry_run {
            "→".yellow()
        } else {
            "↑".cyan()
        },
        file.display(),
        current_version,
        target_version
    );

    // Perform transformations
    let yaml_map = yaml
        .as_mapping_mut()
        .ok_or_else(|| ForgeError::Parse("Root must be a YAML mapping".to_string()))?;

    // 1. Update _forge_version
    yaml_map.insert(
        serde_yaml::Value::String("_forge_version".to_string()),
        serde_yaml::Value::String(target_version.to_string()),
    );

    // 2. Split scalars into inputs/outputs if upgrading to 5.0.0
    if target_version == "5.0.0" {
        split_scalars_to_inputs_outputs(yaml_map, verbose)?;
    }

    if !dry_run {
        // Create backup
        let backup_path = file.with_extension("yaml.bak");
        fs::copy(file, &backup_path)
            .map_err(|e| ForgeError::IO(format!("Failed to create backup: {}", e)))?;
        if verbose {
            println!("      {} Backup: {}", "📋".dimmed(), backup_path.display());
        }

        // Write upgraded content
        let upgraded_content = serde_yaml::to_string(&yaml)
            .map_err(|e| ForgeError::IO(format!("Failed to serialize YAML: {}", e)))?;

        // Preserve comments by writing a header
        let final_content = format!(
            "# Upgraded to Forge v{} by 'forge upgrade'\n{}",
            target_version, upgraded_content
        );

        fs::write(file, final_content)
            .map_err(|e| ForgeError::IO(format!("Failed to write {}: {}", file.display(), e)))?;
    }

    Ok(changes + 1)
}

/// Split scalars section into inputs and outputs based on formula presence
fn split_scalars_to_inputs_outputs(
    yaml_map: &mut serde_yaml::Mapping,
    verbose: bool,
) -> ForgeResult<()> {
    // Check if there's a top-level scalars-like structure (not in a table)
    // In v4.x, scalars are scattered at root level or in sections
    // We need to identify them and split into inputs/outputs

    let mut inputs: serde_yaml::Mapping = serde_yaml::Mapping::new();
    let mut outputs: serde_yaml::Mapping = serde_yaml::Mapping::new();
    let mut keys_to_remove: Vec<serde_yaml::Value> = Vec::new();

    // Preserve existing inputs/outputs if they exist
    if let Some(existing_inputs) = yaml_map.get(serde_yaml::Value::String("inputs".to_string())) {
        if let Some(map) = existing_inputs.as_mapping() {
            inputs = map.clone();
        }
    }
    if let Some(existing_outputs) = yaml_map.get(serde_yaml::Value::String("outputs".to_string())) {
        if let Some(map) = existing_outputs.as_mapping() {
            outputs = map.clone();
        }
    }

    // Look for scalar-like entries at root level
    // These are mappings with 'value' and optionally 'formula' keys
    for (key, value) in yaml_map.iter() {
        let key_str = key.as_str().unwrap_or("");

        // Skip special keys and existing sections
        if key_str.starts_with('_')
            || key_str == "inputs"
            || key_str == "outputs"
            || key_str == "scenarios"
        {
            continue;
        }

        // Check if this looks like a scalar (has 'value' key)
        if let Some(mapping) = value.as_mapping() {
            let value_key = serde_yaml::Value::String("value".to_string());
            let formula_key = serde_yaml::Value::String("formula".to_string());
            if mapping.contains_key(&value_key) {
                let has_formula = mapping.contains_key(&formula_key)
                    && mapping
                        .get(&formula_key)
                        .map(|f| !f.is_null() && f.as_str().map(|s| !s.is_empty()).unwrap_or(false))
                        .unwrap_or(false);

                if has_formula {
                    outputs.insert(key.clone(), value.clone());
                    if verbose {
                        println!(
                            "      {} {} → outputs (has formula)",
                            "📤".dimmed(),
                            key_str
                        );
                    }
                } else {
                    inputs.insert(key.clone(), value.clone());
                    if verbose {
                        println!("      {} {} → inputs (value only)", "📥".dimmed(), key_str);
                    }
                }
                keys_to_remove.push(key.clone());
            }
        }
    }

    // Remove moved keys
    for key in keys_to_remove {
        yaml_map.remove(&key);
    }

    // Add inputs and outputs sections if they have content
    if !inputs.is_empty() {
        yaml_map.insert(
            serde_yaml::Value::String("inputs".to_string()),
            serde_yaml::Value::Mapping(inputs),
        );
    }
    if !outputs.is_empty() {
        yaml_map.insert(
            serde_yaml::Value::String("outputs".to_string()),
            serde_yaml::Value::Mapping(outputs),
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
