use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
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
pub fn calculate(file: PathBuf, dry_run: bool, verbose: bool, scenario: Option<String>) -> ForgeResult<()> {
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
            println!("{}", format!("📊 Applied scenario: {}", scenario_name).cyan());
        }
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

    // TODO: Implement v1.0.0 writer
    if dry_run {
        println!("{}", "📋 Dry run complete - no changes written".yellow());
    } else {
        println!("{}", "⚠️  File writing not yet implemented".yellow());
        println!(
            "{}",
            "   Results calculated successfully but not written back".yellow()
        );
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
fn find_variable(model: &crate::types::ParsedModel, name: &str) -> ForgeResult<(String, Option<String>, Option<f64>)> {
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
                    dep.children = build_dependency_tree(model, &ref_name, &scalar.formula, depth + 1)?;
                }
            } else if let Some(agg) = model.aggregations.get(&ref_name) {
                dep.dep_type = "Aggregation".to_string();
                dep.formula = Some(agg.clone());
                dep.children = build_dependency_tree(model, &ref_name, &Some(agg.clone()), depth + 1)?;
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
        "SUM", "AVERAGE", "AVG", "MAX", "MIN", "COUNT", "PRODUCT",
        "SUMIF", "COUNTIF", "AVERAGEIF", "SUMIFS", "COUNTIFS", "AVERAGEIFS",
        "MAXIFS", "MINIFS", "ROUND", "ROUNDUP", "ROUNDDOWN", "CEILING", "FLOOR",
        "SQRT", "POWER", "MOD", "ABS", "IF", "AND", "OR", "NOT",
        "CONCAT", "UPPER", "LOWER", "TRIM", "LEN", "MID",
        "TODAY", "DATE", "YEAR", "MONTH", "DAY",
        "MATCH", "INDEX", "XLOOKUP", "VLOOKUP", "IFERROR",
        "TRUE", "FALSE",
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

/// Execute the validate command
pub fn validate(file: PathBuf) -> ForgeResult<()> {
    println!("{}", "✅ Validating model".bold().green());
    println!("   File: {}\n", file.display());

    // Parse YAML file
    let model = parser::parse_model(&file)?;

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
pub fn import(input: PathBuf, output: PathBuf, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Import".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}\n", output.display());

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

    // Write YAML file
    if verbose {
        println!("{}", "💾 Writing YAML file...".cyan());
    }

    // Serialize model to YAML
    let yaml_string = serde_yaml::to_string(&model).map_err(ForgeError::Yaml)?;

    fs::write(&output, yaml_string).map_err(ForgeError::Io)?;

    println!("{}", "✅ Import Complete!".bold().green());
    println!("   YAML file: {}\n", output.display());

    println!("{}", "✅ Phase 4: Excel Import Complete!".bold().green());
    println!("   ✅ Excel worksheets → YAML tables");
    println!("   ✅ Data values imported");
    println!("   ✅ Multiple worksheets → One YAML file");
    println!("   ✅ Scalars sheet detected");
    println!("   ✅ Formula translation (Excel → YAML syntax)");
    println!("   ✅ Supports 60+ Excel functions (IFERROR, SUMIF, VLOOKUP, etc.)\n");

    Ok(())
}

/// Execute the watch command
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
        println!("   {} {}", "Watching directory:".cyan(), parent_dir.display());
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
                crate::types::Variable {
                    path: var_name.clone(),
                    value: Some(*override_value),
                    formula: None,
                },
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
