// During coverage builds, stubbed main doesn't use imports
#![cfg_attr(coverage, allow(unused_imports))]

use clap::{Parser, Subcommand};
#[cfg(feature = "full")]
use colored::Colorize;
use royalbit_forge::cli;
use royalbit_forge::error::ForgeResult;
#[cfg(feature = "full")]
use royalbit_forge::update::{check_for_update, perform_update};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "forge")]
#[cfg_attr(
    not(feature = "full"),
    command(about = "Git-native financial modeling. Zero hallucinations.")
)]
#[cfg_attr(
    feature = "full",
    command(about = "Git-native financial modeling. 160 functions. Zero hallucinations.")
)]
#[cfg_attr(
    not(feature = "full"),
    command(long_about = "Forge Demo - Git-native financial modeling
47 functions | E2E validated against Gnumeric

COMMANDS:
  calculate   - Execute formulas, update values
  validate    - Check model integrity
  audit       - Trace formula dependencies
  functions   - List all supported functions
  export      - YAML -> Excel (.xlsx) with formulas
  import      - Excel -> YAML
  watch       - Auto-calculate on save

EXAMPLES:
  forge calculate model.yaml           # Execute formulas
  forge audit model.yaml profit        # Dependency trace
  forge export model.yaml output.xlsx  # Excel with formulas

Docs: https://github.com/royalbit/forge-demo")
)]
#[cfg_attr(
    feature = "full",
    command(long_about = "Forge Enterprise - Git-native financial modeling
160 functions | 1709 tests | E2E validated against Gnumeric

COMMANDS:
  calculate   - Execute formulas, update values
  validate    - Check model integrity
  audit       - Trace formula dependencies (SOX compliance)
  functions   - List all 160 supported functions
  sensitivity - One/two-variable data tables
  goal-seek   - Find input for target output
  break-even  - Find zero-crossing point
  variance    - Budget vs actual analysis
  compare     - Multi-scenario comparison
  export      - YAML -> Excel (.xlsx) with formulas
  import      - Excel -> YAML
  watch       - Auto-calculate on save
  upgrade     - Upgrade YAML to latest schema

EXAMPLES:
  forge calculate model.yaml                    # Execute formulas
  forge audit model.yaml profit                 # Dependency trace
  forge variance budget.yaml actual.yaml       # Budget vs actual
  forge export model.yaml output.xlsx          # Excel with formulas

Docs: https://royalbit.ca/forge")
)]
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

        /// Scenario name to apply (uses variable overrides from 'scenarios' section)
        #[arg(short, long)]
        scenario: Option<String>,
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
  Run 'calculate' to update all files.

BATCH VALIDATION:
  forge validate file1.yaml file2.yaml file3.yaml
  Validates multiple files in sequence, reporting all errors.")]
    /// Validate formulas without calculating
    Validate {
        /// Path to YAML file(s) to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    #[command(long_about = "Export v1.0.0 array model to Excel .xlsx format.

Converts YAML column arrays to Excel worksheets with full formula support.
Each table becomes a separate worksheet. Formulas are translated to Excel syntax.

SUPPORTED FEATURES (Phase 3.1 - Basic Export):
  Table columns -> Excel columns (A, B, C, ...)
  Data values (Number, Text, Date, Boolean)
  Multiple tables -> Multiple worksheets
  Scalars -> Dedicated \"Scalars\" worksheet

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
  Excel worksheets -> YAML tables
  Data values (Number, Text, Boolean)
  Multiple worksheets -> One YAML file (one-to-one)
  \"Scalars\" sheet -> Scalar section

WORKFLOW:
  1. Import existing Excel -> YAML
  2. Work with AI + Forge (version control!)
  3. Export back to Excel
  4. Round-trip: Excel -> YAML -> Excel

EXAMPLE:
  forge import quarterly_pl.xlsx quarterly_pl.yaml

NOTE: Formulas are preserved as Excel syntax (Phase 4.1).
      Formula translation to YAML syntax coming in Phase 4.3.")]
    /// Import Excel .xlsx file to YAML v1.0.0
    Import {
        /// Path to Excel file (.xlsx)
        input: PathBuf,

        /// Output YAML file path (or directory if --split-files)
        output: PathBuf,

        /// Show verbose import steps
        #[arg(short, long)]
        verbose: bool,

        /// Create separate YAML file per worksheet (v4.4.2)
        #[arg(long)]
        split_files: bool,

        /// Create multi-document YAML with --- separators (v4.4.2)
        #[arg(long)]
        multi_doc: bool,
    },

    #[command(long_about = "Watch YAML files and auto-calculate on changes.

Monitors the specified file (and all included files) for changes.
When a change is detected, automatically runs validation/calculation.

FEATURES:
  Real-time file monitoring
  Auto-calculate on save
  Debounced updates (waits for file write to complete)
  Watches included files too
  Clear error messages on formula issues

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

    #[cfg(feature = "full")]
    #[command(long_about = "Compare calculation results across multiple scenarios.

Runs calculations for each specified scenario and displays results side-by-side.
Useful for sensitivity analysis and what-if modeling.

SCENARIOS IN YAML:
  Define scenarios in your model file:

  scenarios:
    base:
      growth_rate: 0.05
      churn_rate: 0.02
    optimistic:
      growth_rate: 0.12
      churn_rate: 0.01
    pessimistic:
      growth_rate: 0.02
      churn_rate: 0.05

EXAMPLE:
  forge compare model.yaml --scenarios base,optimistic,pessimistic

OUTPUT:
  Scenario Comparison: model.yaml
  Variable          Base      Optimistic  Pessimistic
  revenue           $1.2M     $1.8M       $0.9M
  profit            $200K     $450K       -$50K")]
    /// Compare results across multiple scenarios (enterprise only)
    Compare {
        /// Path to YAML file
        file: PathBuf,

        /// Comma-separated list of scenario names to compare
        #[arg(short, long, value_delimiter = ',')]
        scenarios: Vec<String>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Compare budget vs actual with variance analysis.

Calculates variances between two YAML files (budget and actual).
Shows absolute variance, percentage variance, and favorability status.

INPUTS:
  Both files must be YAML format (use 'forge import' for Excel files first).
  Variables are matched by name across both files.

VARIANCE TYPES:
  For revenue/income: actual > budget = favorable
  For expenses/costs: actual < budget = favorable

THRESHOLD:
  Use --threshold to flag significant variances (default: 10%)

OUTPUT FORMATS:
  Terminal table (default)
  YAML: forge variance budget.yaml actual.yaml -o report.yaml
  Excel: forge variance budget.yaml actual.yaml -o report.xlsx

EXAMPLES:
  forge variance budget.yaml actual.yaml
  forge variance budget.yaml actual.yaml --threshold 5
  forge variance budget.yaml actual.yaml -o variance_report.xlsx

See ADR-002 for design rationale on YAML-only inputs.")]
    /// Compare budget vs actual with variance analysis (enterprise only)
    Variance {
        /// Path to budget YAML file
        budget: PathBuf,

        /// Path to actual YAML file
        actual: PathBuf,

        /// Variance threshold percentage for alerts (default: 10)
        #[arg(short, long, default_value = "10")]
        threshold: f64,

        /// Output file (optional: .yaml or .xlsx)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Run sensitivity analysis by varying one or two inputs.

Varies the specified input variable(s) across a range and shows how the
output variable changes. Essential for understanding model behavior and risk.

ONE-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --range 0.01,0.15,0.02 --output npv

  Shows how NPV changes as growth_rate varies from 1% to 15% in 2% steps.

TWO-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --vary2 discount_rate \\
      --range 0.01,0.15,0.02 --range2 0.05,0.15,0.05 --output npv

  Shows a matrix of NPV values for each combination of inputs.

RANGE FORMAT:
  start,end,step - e.g., 0.01,0.15,0.02 means 0.01, 0.03, 0.05, ..., 0.15

EXAMPLES:
  forge sensitivity model.yaml -v growth_rate -r 0.05,0.20,0.05 -o profit
  forge sensitivity model.yaml -v price -v2 volume -r 10,50,10 -r2 100,500,100 -o revenue")]
    /// Run sensitivity analysis on model variables (enterprise only)
    Sensitivity {
        /// Path to YAML file
        file: PathBuf,

        /// Variable to vary (scalar name)
        #[arg(short, long)]
        vary: String,

        /// Range for first variable: start,end,step
        #[arg(short, long)]
        range: String,

        /// Second variable to vary (for 2D analysis)
        #[arg(long)]
        vary2: Option<String>,

        /// Range for second variable: start,end,step
        #[arg(long)]
        range2: Option<String>,

        /// Output variable to observe
        #[arg(short, long)]
        output: String,

        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Find the input value needed to achieve a target output.

Uses numerical methods (bisection) to find what input value produces
the desired output. Useful for answering 'what price do I need?' questions.

EXAMPLES:
  forge goal-seek model.yaml --target profit --value 100000 --vary price
  -> Find the price needed to achieve $100,000 profit

  forge goal-seek model.yaml --target npv --value 0 --vary discount_rate
  -> Find the discount rate that makes NPV = 0 (IRR)

OPTIONS:
  --min, --max: Override automatic bounds for the search
  --tolerance: Precision of the result (default: 0.0001)")]
    /// Find input value to achieve target output (enterprise only)
    GoalSeek {
        /// Path to YAML file
        file: PathBuf,

        /// Target variable to achieve
        #[arg(short, long)]
        target: String,

        /// Desired value for target
        #[arg(long)]
        value: f64,

        /// Variable to adjust
        #[arg(short, long)]
        vary: String,

        /// Minimum bound for search (optional)
        #[arg(long)]
        min: Option<f64>,

        /// Maximum bound for search (optional)
        #[arg(long)]
        max: Option<f64>,

        /// Solution tolerance (default: 0.0001)
        #[arg(long, default_value = "0.0001")]
        tolerance: f64,

        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Find the break-even point where output equals zero.

Special case of goal-seek that finds where a variable crosses zero.
Common for finding break-even units, prices, or margins.

EXAMPLES:
  forge break-even model.yaml --output profit --vary units
  -> Find units needed to break even (profit = 0)

  forge break-even model.yaml --output net_margin --vary price
  -> Find minimum price for positive margin")]
    /// Find break-even point (where output = 0) (enterprise only)
    BreakEven {
        /// Path to YAML file
        file: PathBuf,

        /// Output variable to find zero crossing
        #[arg(short, long)]
        output: String,

        /// Variable to adjust
        #[arg(short, long)]
        vary: String,

        /// Minimum bound for search (optional)
        #[arg(long)]
        min: Option<f64>,

        /// Maximum bound for search (optional)
        #[arg(long)]
        max: Option<f64>,

        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Check for updates and optionally self-update the binary.

Downloads the latest release from GitHub and replaces the current binary.
Checksums are verified before installation.

EXAMPLES:
  forge update              # Check and install updates
  forge update --check      # Check only, don't install

PLATFORMS:
  Linux x86_64, Linux ARM64, macOS Intel, macOS ARM, Windows x64")]
    /// Check for updates and self-update (enterprise only)
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,
    },

    #[command(
        long_about = "List all supported Excel-compatible functions by category.

Forge supports 58 Excel functions for financial modeling. Use this command
to see all available functions organized by category.

CATEGORIES:
  Financial   - NPV, IRR, XNPV, XIRR, PMT, PV, FV, RATE, NPER (9)
  Lookup      - MATCH, INDEX, VLOOKUP, XLOOKUP, CHOOSE, OFFSET (6)
  Conditional - SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, MAXIFS, MINIFS (8)
  Array       - UNIQUE, COUNTUNIQUE, FILTER, SORT (4)
  Aggregation - SUM, AVERAGE, MIN, MAX, COUNT (5)
  Math        - ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER, ABS (9)
  Text        - CONCAT, TRIM, UPPER, LOWER, LEN, MID (6)
  Date        - TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH (8)
  Logic       - IF, AND, OR (3)

EXAMPLES:
  forge functions           # List all functions
  forge functions --json    # Output as JSON (for tooling)"
    )]
    /// List all supported Excel-compatible functions
    Functions {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    #[cfg(feature = "full")]
    #[command(long_about = "Upgrade YAML files to latest schema version (v5.0.0).

Automatically migrates YAML files and all included files to the latest schema.
Creates backups before modifying files.

TRANSFORMATIONS:
  - Updates _forge_version to 5.0.0
  - Splits scalars into inputs/outputs based on formula presence:
    - Scalars with value only -> inputs section
    - Scalars with formula -> outputs section
  - Adds _name field for multi-document files
  - Preserves all existing metadata

RECURSIVE PROCESSING:
  If the file has _includes, all included files are upgraded FIRST.
  Circular includes are detected and handled.

EXAMPLES:
  forge upgrade model.yaml              # Upgrade file and includes
  forge upgrade model.yaml --dry-run    # Preview changes only
  forge upgrade model.yaml --to 5.0.0   # Explicit target version

BACKUP:
  Original files are backed up as .yaml.bak before modification.")]
    /// Upgrade YAML files to latest schema version (enterprise only)
    Upgrade {
        /// Path to YAML file to upgrade
        file: PathBuf,

        /// Preview changes without modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Target schema version (default: 5.0.0)
        #[arg(long, default_value = "5.0.0")]
        to: String,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

/// CLI entry point - excluded from coverage (ADR-006)
/// Parses CLI args and dispatches to library functions.
/// Cannot unit test: reads from std::env::args()
/// Tested via: cli_integration_tests.rs
#[cfg(not(coverage))]
fn main() -> ForgeResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Calculate {
            file,
            dry_run,
            verbose,
            scenario,
        } => cli::calculate(file, dry_run, verbose, scenario),

        Commands::Audit { file, variable } => cli::audit(file, variable),

        Commands::Validate { files } => cli::validate(files),

        Commands::Export {
            input,
            output,
            verbose,
        } => cli::export(input, output, verbose),

        Commands::Import {
            input,
            output,
            verbose,
            split_files,
            multi_doc,
        } => cli::import(input, output, verbose, split_files, multi_doc),

        Commands::Watch {
            file,
            validate,
            verbose,
        } => cli::watch(file, validate, verbose),

        #[cfg(feature = "full")]
        Commands::Compare {
            file,
            scenarios,
            verbose,
        } => cli::compare(file, scenarios, verbose),

        #[cfg(feature = "full")]
        Commands::Variance {
            budget,
            actual,
            threshold,
            output,
            verbose,
        } => cli::variance(budget, actual, threshold, output, verbose),

        #[cfg(feature = "full")]
        Commands::Sensitivity {
            file,
            vary,
            range,
            vary2,
            range2,
            output,
            verbose,
        } => cli::sensitivity(file, vary, range, vary2, range2, output, verbose),

        #[cfg(feature = "full")]
        Commands::GoalSeek {
            file,
            target,
            value,
            vary,
            min,
            max,
            tolerance,
            verbose,
        } => cli::goal_seek(file, target, value, vary, min, max, tolerance, verbose),

        #[cfg(feature = "full")]
        Commands::BreakEven {
            file,
            output,
            vary,
            min,
            max,
            verbose,
        } => cli::break_even(file, output, vary, min, max, verbose),

        #[cfg(feature = "full")]
        Commands::Update { check } => {
            println!("{}", "Forge - Update".bold().green());
            println!();

            println!("  Checking for updates...");

            match check_for_update() {
                Ok(version_info) => {
                    println!("  Current version: {}", version_info.current.bright_blue());
                    println!("  Latest version:  {}", version_info.latest.bright_blue());
                    println!();

                    if version_info.update_available {
                        println!("  {} New version available!", "UPDATE".bold().yellow());

                        if check {
                            println!();
                            println!("  Run {} to install the update.", "forge update".bold());
                            return Ok(());
                        }

                        if let Some(download_url) = version_info.download_url {
                            println!();
                            match perform_update(
                                &download_url,
                                version_info.checksums_url.as_deref(),
                            ) {
                                Ok(()) => {
                                    println!();
                                    println!(
                                        "{} Updated to version {}",
                                        "Success:".bold().green(),
                                        version_info.latest
                                    );
                                    println!();
                                    println!("  Run {} to verify.", "forge --version".bold());
                                }
                                Err(e) => {
                                    eprintln!();
                                    eprintln!("{} {}", "Error:".bold().red(), e);
                                    eprintln!();
                                    eprintln!("  You can manually update:");
                                    eprintln!("    curl -L {} | tar xz", download_url);
                                    eprintln!("    sudo mv forge /usr/local/bin/");
                                    return Err(royalbit_forge::error::ForgeError::Validation(
                                        e.to_string(),
                                    ));
                                }
                            }
                        } else {
                            eprintln!();
                            eprintln!(
                                "{} No binary available for this platform",
                                "Error:".bold().red()
                            );
                            eprintln!("  Visit https://github.com/royalbit/forge/releases/latest");
                            return Err(royalbit_forge::error::ForgeError::Validation(
                                "No binary for platform".to_string(),
                            ));
                        }
                    } else {
                        println!(
                            "  {} You're running the latest version!",
                            "OK".bold().green()
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".bold().red(), e);
                    eprintln!();
                    eprintln!(
                        "  Check manually: https://github.com/royalbit/forge/releases/latest"
                    );
                    Err(royalbit_forge::error::ForgeError::Validation(e))
                }
            }
        }

        Commands::Functions { json } => cli::functions(json),

        #[cfg(feature = "full")]
        Commands::Upgrade {
            file,
            dry_run,
            to,
            verbose,
        } => cli::upgrade(file, dry_run, to, verbose),
    }
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
fn main() -> ForgeResult<()> {
    Ok(())
}
