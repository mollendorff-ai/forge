use crate::core::Calculator;
use crate::error::ForgeResult;
use crate::parser;
use crate::writer;
use colored::Colorize;
use std::path::PathBuf;

/// Execute the calculate command
pub fn calculate(file: PathBuf, dry_run: bool, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Calculating formulas".bold().green());
    println!("   File: {}\n", file.display());

    if dry_run {
        println!("{}", "📋 DRY RUN MODE - No changes will be written\n".yellow());
    }

    // Parse YAML file and extract variables with formulas (including referenced files)
    if verbose {
        println!("{}", "📖 Parsing YAML file and includes...".cyan());
    }
    let parsed = parser::parse_yaml_with_includes(&file)?;

    if verbose {
        println!("   Found {} variables with formulas\n", parsed.variables.len());
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
        println!("{}", "🧮 Calculating formulas in dependency order...".cyan());
    }
    let mut calculator = Calculator::new(parsed.variables.clone());
    let results = calculator.calculate_all()?;

    // Display results
    println!("{}", "✅ Calculation Results:".bold().green());
    for (var_name, value) in &results {
        println!("   {} = {}", var_name.bright_blue(), format!("{value}").bold());
    }
    println!();

    // Write back to ALL files (main + includes) - Excel-style (unless dry run)
    if dry_run {
        println!("{}", "📋 Dry run complete - no changes written".yellow());
    } else {
        if verbose {
            println!("{}", "💾 Writing updated values to all files (main + includes)...".cyan());
        }
        writer::update_all_yaml_files(&file, &parsed, &results, &parsed.variables)?;

        if parsed.includes.is_empty() {
            println!("{}", "✨ File updated successfully!".bold().green());
        } else {
            println!("{}", format!("✨ {} files updated successfully! (main + {} includes)",
                1 + parsed.includes.len(), parsed.includes.len()).bold().green());
        }
    }

    Ok(())
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
            println!("\n{}", format!("❌ Formula validation failed: {e}").bold().red());
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
                    mismatches.push((
                        var_name.clone(),
                        current_value,
                        *calculated_value,
                        diff,
                    ));
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
            println!("      Current:  {}", format!("{current}").red());
            println!("      Expected: {}", format!("{expected}").green());
            println!("      Diff:     {}", format!("{diff:.6}").yellow());
            println!();
        }

        println!(
            "{}",
            "💡 Run 'forge calculate' to update values"
                .bold()
                .yellow()
        );

        Err(crate::error::ForgeError::Validation(
            "Values do not match formulas - file needs recalculation".to_string(),
        ))
    }
}
