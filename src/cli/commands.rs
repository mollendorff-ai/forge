use crate::core::{ArrayCalculator, Calculator};
use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
use crate::types::ForgeVersion;
use crate::writer;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

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
pub fn calculate(file: PathBuf, dry_run: bool, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Calculating formulas".bold().green());
    println!("   File: {}\n", file.display());

    if dry_run {
        println!(
            "{}",
            "📋 DRY RUN MODE - No changes will be written\n".yellow()
        );
    }

    // Parse file and detect version
    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    // Try v1.0.0 first (parse_model auto-detects version)
    let model = parser::parse_model(&file)?;

    match model.version {
        ForgeVersion::V1_0_0 => {
            // v1.0.0 Array Model - use ArrayCalculator
            if verbose {
                println!("   Detected: v1.0.0 Array Model");
                println!(
                    "   Found {} tables, {} scalars\n",
                    model.tables.len(),
                    model.scalars.len()
                );
            }

            // Calculate using ArrayCalculator
            if verbose {
                println!(
                    "{}",
                    "🧮 Calculating tables and scalars...".cyan()
                );
            }

            let calculator = ArrayCalculator::new(model);
            let result = calculator.calculate_all()?;

            // Display results
            println!("{}", "✅ Calculation Results:".bold().green());

            // Show table results
            for (table_name, table) in &result.tables {
                println!("   📊 Table: {}", table_name.bright_blue().bold());
                for (col_name, column) in &table.columns {
                    println!(
                        "      {} ({} rows)",
                        col_name.cyan(),
                        column.values.len()
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

            // TODO: Implement v1.0.0 writer
            if dry_run {
                println!("{}", "📋 Dry run complete - no changes written".yellow());
            } else {
                println!("{}", "⚠️  v1.0.0 file writing not yet implemented".yellow());
                println!("{}", "   Results calculated successfully but not written back".yellow());
            }

            Ok(())
        }
        ForgeVersion::V0_2_0 => {
            // v0.2.0 Scalar Model - use old Calculator (with includes support)
            let parsed = parser::parse_yaml_with_includes(&file)?;

            if verbose {
                println!("   Detected: v0.2.0 Scalar Model");
                println!(
                    "   Found {} variables with formulas\n",
                    parsed.variables.len()
                );
                for (name, var) in &parsed.variables {
                    if let Some(formula) = &var.formula {
                        println!("   {} = {}", name.bright_blue(), formula.dimmed());
                    }
                }
                println!();
            }

            if parsed.variables.is_empty() {
                println!("{}", "⚠️  No formulas found in YAML file".yellow());
                return Ok(());
            }

            // Calculate all formulas
            if verbose {
                println!(
                    "{}",
                    "🧮 Calculating formulas in dependency order...".cyan()
                );
            }
            let mut calculator = Calculator::new(parsed.variables.clone());
            let results = calculator.calculate_all()?;

            // Display results
            println!("{}", "✅ Calculation Results:".bold().green());
            for (var_name, value) in &results {
                println!(
                    "   {} = {}",
                    var_name.bright_blue(),
                    format!("{value}").bold()
                );
            }
            println!();

            // Write back to ALL files (main + includes) - Excel-style (unless dry run)
            if dry_run {
                println!("{}", "📋 Dry run complete - no changes written".yellow());
            } else {
                if verbose {
                    println!(
                        "{}",
                        "💾 Writing updated values to all files (main + includes)...".cyan()
                    );
                }
                writer::update_all_yaml_files(&file, &parsed, &results, &parsed.variables)?;

                if parsed.includes.is_empty() {
                    println!("{}", "✨ File updated successfully!".bold().green());
                } else {
                    println!(
                        "{}",
                        format!(
                            "✨ {} files updated successfully! (main + {} includes)",
                            1 + parsed.includes.len(),
                            parsed.includes.len()
                        )
                        .bold()
                        .green()
                    );
                }
            }

            Ok(())
        }
    }
}

/// Execute the audit command
pub fn audit(file: PathBuf, variable: String) -> ForgeResult<()> {
    println!("🔍 Audit trail for '{variable}' in {file:?}");
    println!();

    // TODO: Implement audit trail
    println!("⚠️  Audit trail not yet implemented");
    Ok(())
}

/// Execute the validate command
pub fn validate(file: PathBuf) -> ForgeResult<()> {
    println!("{}", "✅ Validating formulas".bold().green());
    println!("   File: {}\n", file.display());

    // Parse YAML file and includes - get current values from ALL files
    let parsed = parser::parse_yaml_with_includes(&file)?;
    let variables = parsed.variables;

    if variables.is_empty() {
        println!("{}", "⚠️  No formulas found in YAML file".yellow());
        return Ok(());
    }

    println!("   Found {} variables with formulas", variables.len());

    // Calculate what values SHOULD be based on formulas
    let mut calculator = Calculator::new(variables.clone());
    let calculated_values = match calculator.calculate_all() {
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

    for (var_name, calculated_value) in &calculated_values {
        if let Some(var) = variables.get(var_name) {
            if let Some(current_value) = var.value {
                // Check if values match within tolerance
                let diff = (current_value - calculated_value).abs();
                if diff > TOLERANCE {
                    mismatches.push((var_name.clone(), current_value, *calculated_value, diff));
                }
            }
        }
    }

    // Report results
    println!();
    if mismatches.is_empty() {
        println!("{}", "✅ All formulas are valid!".bold().green());
        println!("{}", "✅ All values match their formulas!".bold().green());
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

    // Verify it's a v1.0.0 model
    if model.version != ForgeVersion::V1_0_0 {
        return Err(ForgeError::Export(
            "Excel export only supports v1.0.0 array models. This file appears to be v0.2.0.".to_string(),
        ));
    }

    if verbose {
        println!("   Detected: v1.0.0 Array Model");
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
            println!("      {} columns, {} rows", table.columns.len(), table.row_count());
        }
        println!();
    }

    // Write YAML file
    if verbose {
        println!("{}", "💾 Writing YAML file...".cyan());
    }

    // Serialize model to YAML
    let yaml_string = serde_yaml::to_string(&model)
        .map_err(ForgeError::Yaml)?;

    fs::write(&output, yaml_string)
        .map_err(ForgeError::Io)?;

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
