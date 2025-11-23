use clap::{Parser, Subcommand};
use royalbit_forge::cli;
use royalbit_forge::error::ForgeResult;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "forge")]
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
  forge calculate model.yaml
  forge validate financials.yaml
  forge calculate --dry-run --verbose assumptions.yaml")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(long_about = "Calculate all formulas in a YAML file.

Evaluates formulas in dependency order and updates values in ALL files
(main file + all included files) - just like Excel updates all worksheets.

CROSS-FILE REFERENCES:
  Add 'includes:' section to reference other files:

  includes:
    - file: pricing.yaml
      as: pricing
    - file: costs.yaml
      as: costs

  Then use @alias.variable in formulas:
    formula: \"=@pricing.base_price * volume - @costs.total\"

IMPORTANT: Calculate updates ALL files in the chain (Excel-style)!
  If pricing.yaml has stale formulas, they will be recalculated too.
  This ensures data integrity across all referenced files.

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

Checks that all formula values match their calculations across ALL files
(main file + all included files). Detects stale values that need recalculation.

CROSS-FILE REFERENCES:
  Validates formulas using @alias.variable syntax:

  includes:
    - file: pricing.yaml
      as: pricing

  Formula example:
    formula: \"=@pricing.base_price * 10\"

NOTE: Validation checks ALL files in the chain.
  If any included file has stale values, validation will fail.
  Run 'calculate' to update all files.")]
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
        } => cli::calculate(file, dry_run, verbose),

        Commands::Audit { file, variable } => cli::audit(file, variable),

        Commands::Validate { file } => cli::validate(file),
    }
}
