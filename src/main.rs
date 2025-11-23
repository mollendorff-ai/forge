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
#[command(about = "YAML formula calculator - Forge your data from YAML blueprints", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate all formulas in a YAML file
    Calculate {
        /// Path to YAML file
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

    /// Validate formulas without calculating
    Validate {
        /// Path to YAML file
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

            // Parse YAML file
            let variables = parser::parse_yaml_file(&file)?;

            if variables.is_empty() {
                println!("{}", "⚠️  No formulas found in YAML file".yellow());
                return Ok(());
            }

            println!("   Found {} variables with formulas", variables.len());

            // Try to build dependency graph (will fail on circular deps)
            let mut calculator = Calculator::new(variables);
            match calculator.calculate_all() {
                Ok(_) => {
                    println!("\n{}", "✅ All formulas are valid!".bold().green());
                }
                Err(e) => {
                    println!("\n{}", format!("❌ Validation failed: {}", e).bold().red());
                    return Err(e);
                }
            }

            Ok(())
        }
    }
}
