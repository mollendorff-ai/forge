use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod calculator;
mod error;
mod parser;
mod types;
mod writer;

use calculator::Calculator;
use colored::Colorize;
use error::ForgeResult;

#[derive(Parser)]
#[command(name = "mouvify-forge")]
#[command(about = "YAML formula calculator - Forge your data from YAML blueprints")]
#[command(long_about = "YAML formula calculator with cross-file references.

Embed Excel-style formulas in YAML files and automatically calculate values.

CROSS-FILE REFERENCES:
  Include other YAML files and reference their variables:

  # main.yaml
  includes:
    - file: pricing.yaml
      as: pricing
    - file: costs.yaml
      as: costs

  revenue:
    value: null
    formula: \"=@pricing.base_price * volume\"

  margin:
    value: null
    formula: \"=revenue - @costs.total_cost\"

  Use @alias.variable syntax to reference included variables.

EXAMPLES:
  mouvify-forge calculate model.yaml
  mouvify-forge validate financials.yaml
  mouvify-forge calculate --dry-run --verbose assumptions.yaml")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(long_about = "Calculate all formulas in a YAML file.

Evaluates formulas in dependency order and updates values in the file.

CROSS-FILE REFERENCES:
  Add 'includes:' section to reference other files:

  includes:
    - file: pricing.yaml
      as: pricing
    - file: costs.yaml
      as: costs

  Then use @alias.variable in formulas:
    formula: \"=@pricing.base_price * volume - @costs.total\"

Use --dry-run to preview changes without modifying files.")]
    /// Calculate all formulas in a YAML file
    Calculate {
        /// Path to YAML file (can include other files via 'includes' section)
        file: PathBuf,

        /// Preview changes without writing to file
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Show verbose calculation steps
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show audit trail for a specific variable
    Audit {
        /// Path to YAML file
        file: PathBuf,

        /// Variable name to audit
        variable: String,
    },

    #[command(long_about = "Validate formulas without calculating.

Checks that all formula values match their calculations.
Detects stale values that need recalculation.

CROSS-FILE REFERENCES:
  Validates formulas using @alias.variable syntax:

  includes:
    - file: pricing.yaml
      as: pricing

  Formula example:
    formula: \"=@pricing.base_price * 10\"")]
    /// Validate formulas without calculating
    Validate {
        /// Path to YAML file (can include other files via 'includes' section)
        file: PathBuf,
    },
}

fn main() -> ForgeResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Calculate {
            file,
            dry_run,
            verbose,
        } => {
            println!("{}", "🔥 Mouvify Forge - Calculating formulas".bold().green());
            println!("   File: {}\n", file.display());

            if dry_run {
                println!("{}", "📋 DRY RUN MODE - No changes will be written\n".yellow());
            }

            // Parse YAML file and extract variables with formulas
            if verbose {
                println!("{}", "📖 Parsing YAML file...".cyan());
            }
            let variables = parser::parse_yaml_file(&file)?;

            if verbose {
                println!("   Found {} variables with formulas\n", variables.len());
                for (name, var) in &variables {
                    if let Some(formula) = &var.formula {
                        println!("   {} = {}", name.bright_blue(), formula.dimmed());
                    }
                }
                println!();
            }

            if variables.is_empty() {
                println!("{}", "⚠️  No formulas found in YAML file".yellow());
                return Ok(());
            }

            // Calculate all formulas
            if verbose {
                println!("{}", "🧮 Calculating formulas in dependency order...".cyan());
            }
            let mut calculator = Calculator::new(variables);
            let results = calculator.calculate_all()?;

            // Display results
            println!("{}", "✅ Calculation Results:".bold().green());
            for (var_name, value) in &results {
                println!("   {} = {}", var_name.bright_blue(), format!("{}", value).bold());
            }
            println!();

            // Write back to file (unless dry run)
            if !dry_run {
                if verbose {
                    println!("{}", "💾 Writing updated values to file...".cyan());
                }
                writer::update_yaml_file(&file, &results)?;
                println!("{}", "✨ File updated successfully!".bold().green());
            } else {
                println!("{}", "📋 Dry run complete - no changes written".yellow());
            }

            Ok(())
        }

        Commands::Audit { file, variable } => {
            println!("🔍 Audit trail for '{}' in {:?}", variable, file);
            println!();

            // TODO: Implement audit trail
            println!("⚠️  Audit trail not yet implemented");
            Ok(())
        }

        Commands::Validate { file } => {
            println!("{}", "✅ Validating formulas".bold().green());
            println!("   File: {}\n", file.display());

            // Parse YAML file and get current values
            let variables = parser::parse_yaml_file(&file)?;

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
                    println!("\n{}", format!("❌ Formula validation failed: {}", e).bold().red());
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
                    println!("      Current:  {}", format!("{}", current).red());
                    println!("      Expected: {}", format!("{}", expected).green());
                    println!("      Diff:     {}", format!("{:.6}", diff).yellow());
                    println!();
                }

                println!(
                    "{}",
                    "💡 Run 'mouvify-forge calculate' to update values"
                        .bold()
                        .yellow()
                );

                Err(error::ForgeError::Validation(
                    "Values do not match formulas - file needs recalculation".to_string(),
                ))
            }
        }
    }
}
