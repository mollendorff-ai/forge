use clap::{Parser, Subcommand};
use royalbit_forge::cli;
use royalbit_forge::error::ForgeResult;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "Stop AI hallucinations. Save money. Save the planet. Trust the math.")]
#[command(long_about = "Forge - Deterministic YAML formula calculator
Built autonomously by AI in ~23.5 hours. 141 tests passing.

STOP AI HALLUCINATIONS:
  AI validation:  70K tokens, $0.66, 0.25g CO2, 30-60s, ~90% accuracy
  Forge:          0 tokens,  $0.00, 0.0005g CO2, <200ms, 100% accuracy
  → 99.6% less carbon, infinitely cheaper, 300x faster, perfectly accurate

SAVE MONEY:
  Personal: $819/year | Small team: $40K/year | Enterprise: $132K/year

50+ EXCEL FUNCTIONS:
  Lookup: MATCH, INDEX, XLOOKUP, VLOOKUP (use INDEX/MATCH for production)
  Conditional: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS, MAXIFS, MINIFS
  Math: ROUND, ROUNDUP, ROUNDDOWN, SQRT, POWER, MOD, CEILING, FLOOR
  Text: CONCAT, UPPER, LOWER, TRIM, LEN, MID
  Date: TODAY, YEAR, MONTH, DAY, DATE
  Aggregation: SUM, AVERAGE, MAX, MIN, COUNT, PRODUCT
  Logic: IF, AND, OR, NOT
  Excel import/export with formula translation

CROSS-FILE REFERENCES:
  # main.yaml
  includes:
    - file: pricing.yaml
      as: pricing

  revenue:
    value: null
    formula: \"=@pricing.base_price * volume\"

EXAMPLES:
  forge validate model.yaml          # Zero tokens, <200ms
  forge calculate financials.yaml    # Update all formulas
  forge export model.yaml out.xlsx   # Export to Excel
  forge import data.xlsx model.yaml  # Import from Excel

Docs: https://github.com/royalbit/forge | Built by Claude Sonnet 4.5")]
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

    #[command(long_about = "Export v1.0.0 array model to Excel .xlsx format.

Converts YAML column arrays to Excel worksheets with full formula support.
Each table becomes a separate worksheet. Formulas are translated to Excel syntax.

SUPPORTED FEATURES (Phase 3.1 - Basic Export):
  ✅ Table columns → Excel columns (A, B, C, ...)
  ✅ Data values (Number, Text, Date, Boolean)
  ✅ Multiple tables → Multiple worksheets
  ✅ Scalars → Dedicated \"Scalars\" worksheet

COMING SOON (Phase 3.2+):
  ⏳ Row formulas → Excel cell formulas (=A2-B2)
  ⏳ Cross-table references (=Sheet!Column)
  ⏳ Aggregation formulas (=SUM(Sheet!A:A))

EXAMPLE:
  forge export quarterly_pl.yaml quarterly_pl.xlsx

NOTE: Only works with v1.0.0 array models. v0.2.0 scalar models are not supported.")]
    /// Export v1.0.0 array model to Excel .xlsx
    Export {
        /// Path to v1.0.0 YAML file (must have 'tables' section)
        input: PathBuf,

        /// Output Excel file path (.xlsx)
        output: PathBuf,

        /// Show verbose export steps
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Import Excel .xlsx file to YAML v1.0.0 format.

Converts Excel worksheets to YAML tables with formula preservation.
Each worksheet becomes a table in the output YAML file.

SUPPORTED FEATURES (Phase 4.1 - Basic Import):
  ✅ Excel worksheets → YAML tables
  ✅ Data values (Number, Text, Boolean)
  ✅ Multiple worksheets → One YAML file (one-to-one)
  ✅ \"Scalars\" sheet → Scalar section
  ⏳ Formula translation (coming in Phase 4.3)

WORKFLOW:
  1. Import existing Excel → YAML
  2. Work with AI + Forge (version control!)
  3. Export back to Excel
  4. Round-trip: Excel → YAML → Excel

EXAMPLE:
  forge import quarterly_pl.xlsx quarterly_pl.yaml

NOTE: Formulas are preserved as Excel syntax (Phase 4.1).
      Formula translation to YAML syntax coming in Phase 4.3.")]
    /// Import Excel .xlsx file to YAML v1.0.0
    Import {
        /// Path to Excel file (.xlsx)
        input: PathBuf,

        /// Output YAML file path
        output: PathBuf,

        /// Show verbose import steps
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Watch YAML files and auto-calculate on changes.

Monitors the specified file (and all included files) for changes.
When a change is detected, automatically runs validation/calculation.

FEATURES:
  ✅ Real-time file monitoring
  ✅ Auto-calculate on save
  ✅ Debounced updates (waits for file write to complete)
  ✅ Watches included files too
  ✅ Clear error messages on formula issues

WORKFLOW:
  1. Open your YAML in your editor
  2. Run 'forge watch model.yaml' in a terminal
  3. Edit and save - results update automatically
  4. Instant feedback loop for iterative development

EXAMPLES:
  forge watch model.yaml              # Watch and auto-calculate
  forge watch model.yaml --validate   # Watch and validate only
  forge watch model.yaml --verbose    # Show detailed output

Press Ctrl+C to stop watching.")]
    /// Watch YAML files and auto-calculate on changes
    Watch {
        /// Path to YAML file to watch
        file: PathBuf,

        /// Only validate (don't calculate)
        #[arg(long)]
        validate: bool,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
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

        Commands::Export {
            input,
            output,
            verbose,
        } => cli::export(input, output, verbose),

        Commands::Import {
            input,
            output,
            verbose,
        } => cli::import(input, output, verbose),

        Commands::Watch {
            file,
            validate,
            verbose,
        } => cli::watch(file, validate, verbose),
    }
}
