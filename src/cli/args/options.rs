//! The options of each command, as clap reads them; the commands and their
//! help are in `args.rs`.

use crate::cli::ReportFormat;
use clap::builder::PossibleValue;
use clap::Args;
use std::path::PathBuf;

/// `--plan` on a what-if command (ADR-076, addendum of 2026-11-26)
const WHAT_IF_PLAN: &str = "Run an uncertain model at plan: each MC.* input at its plan figure (its value:, or else its distribution's likeliest value), as `forge calculate --plan` sets it; a scenario's own number for an input still wins. Without it, a model with MC.* inputs is refused";

#[derive(Args)]
pub struct CalculateArgs {
    /// Path to YAML file (can include other files via an '_includes' section)
    pub file: PathBuf,

    /// Preview changes without writing to file
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Show verbose calculation steps
    #[arg(short, long)]
    pub verbose: bool,

    /// Scenario name to apply (uses variable overrides from 'scenarios' section)
    #[arg(short, long)]
    pub scenario: Option<String>,

    /// Calculate an uncertain model at plan: each MC.* input at its plan
    /// figure (its value:, or else its distribution's likeliest value), as
    /// `forge simulate` does for its value at plan. Never writes the file
    #[arg(long)]
    pub plan: bool,
}

#[derive(Args)]
pub struct AuditArgs {
    /// Path to YAML file
    pub file: PathBuf,

    /// Variable name to audit
    pub variable: String,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Path to YAML file(s) to validate
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Path to the model (.yaml)
    pub input: PathBuf,

    /// Output Excel file path (.xlsx)
    pub output: PathBuf,

    /// Show verbose export steps
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Path to Excel file (.xlsx)
    pub input: PathBuf,

    /// Output YAML file path (or directory if --split-files)
    pub output: PathBuf,

    /// Show verbose import steps
    #[arg(short, long)]
    pub verbose: bool,

    /// Create separate YAML file per worksheet (v4.4.2)
    #[arg(long)]
    pub split_files: bool,

    /// Create multi-document YAML with --- separators (v4.4.2)
    #[arg(long)]
    pub multi_doc: bool,
}

#[derive(Args)]
pub struct WatchArgs {
    /// Path to YAML file to watch
    pub file: PathBuf,

    /// Only validate (don't calculate)
    #[arg(long)]
    pub validate: bool,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct CompareArgs {
    /// Path to YAML file
    pub file: PathBuf,

    /// Comma-separated list of scenario names to compare
    #[arg(short, long, value_delimiter = ',')]
    pub scenarios: Vec<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: ReportFormat,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct ReportArgs {
    /// Path to a calculated YAML file
    pub file: PathBuf,

    /// Show only these groups (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub group: Vec<String>,

    /// Show only these keys, in this order, as one table (comma-separated);
    /// narrows a wide table. Not with --no-inputs: --keys already chooses the columns
    #[arg(long, value_delimiter = ',', conflicts_with = "no_inputs")]
    pub keys: Vec<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: ReportFormat,

    /// Leave out inputs, the values written with no formula. Not with --keys,
    /// which already chooses the columns
    #[arg(long)]
    pub no_inputs: bool,
}

#[derive(Args)]
pub struct VarianceArgs {
    /// Path to budget YAML file
    pub budget: PathBuf,

    /// Path to actual YAML file
    pub actual: PathBuf,

    /// Variance threshold percentage for alerts (default: 10)
    #[arg(short, long, default_value = "10")]
    pub threshold: f64,

    /// Output file (optional: .yaml or .xlsx)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Exit with 1 when any line exceeds the threshold
    #[arg(long)]
    pub fail_on_alert: bool,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct SensitivityArgs {
    /// Path to YAML file
    pub file: PathBuf,

    /// Variable to vary (scalar name)
    #[arg(long)]
    pub vary: String,

    /// Range for first variable: start,end,step
    #[arg(short, long)]
    pub range: String,

    /// Second variable to vary (for 2D analysis)
    #[arg(long)]
    pub vary2: Option<String>,

    /// Range for second variable: start,end,step
    #[arg(long)]
    pub range2: Option<String>,

    /// Output variable to observe (--output is accepted too)
    #[arg(long = "output-var", visible_alias = "output")]
    pub output_var: String,

    /// Output format: table (default) or json
    #[arg(long, default_value = "table", value_parser = ["table", "json"])]
    pub format: String,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct GoalSeekArgs {
    /// Path to YAML file
    pub file: PathBuf,

    /// Target variable to achieve
    #[arg(short, long)]
    pub target: String,

    /// Desired value for target
    #[arg(long)]
    pub value: f64,

    /// Variable to adjust
    #[arg(long)]
    pub vary: String,

    /// Minimum bound for search (optional)
    #[arg(long)]
    pub min: Option<f64>,

    /// Maximum bound for search (optional)
    #[arg(long)]
    pub max: Option<f64>,

    /// How close the output must get to the target (default: 0.0001)
    #[arg(long, default_value = "0.0001")]
    pub tolerance: f64,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct BreakEvenArgs {
    /// Path to YAML file
    pub file: PathBuf,

    /// Output variable to find zero crossing (--output is accepted too)
    #[arg(long = "output-var", visible_alias = "output")]
    pub output_var: String,

    /// Variable to adjust
    #[arg(long)]
    pub vary: String,

    /// Minimum bound for search (optional)
    #[arg(long)]
    pub min: Option<f64>,

    /// Maximum bound for search (optional)
    #[arg(long)]
    pub max: Option<f64>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct SimulateArgs {
    /// Path to YAML file with `monte_carlo`: section
    pub file: PathBuf,

    /// Number of iterations (overrides YAML config)
    #[arg(short = 'n', long)]
    pub iterations: Option<usize>,

    /// Random seed for reproducibility
    #[arg(long)]
    pub seed: Option<u64>,

    /// Sampling method: `monte_carlo` or `latin_hypercube`
    #[arg(long)]
    pub sampling: Option<String>,

    /// Apply a scenario's overrides first (an override of an MC.* input fixes it)
    #[arg(long)]
    pub scenario: Option<String>,

    /// Find the value of --vary that meets a probability, e.g. "P(year.profit > 0) = 0.8"
    #[arg(long, requires_all = ["vary", "min", "max"])]
    pub seek: Option<String>,

    /// Input to vary for --seek (a scalar with a plain value)
    #[arg(long, requires = "seek")]
    pub vary: Option<String>,

    /// Lowest value of --vary to try
    #[arg(long, requires = "seek", allow_hyphen_values = true)]
    pub min: Option<f64>,

    /// Highest value of --vary to try
    #[arg(long, requires = "seek", allow_hyphen_values = true)]
    pub max: Option<f64>,

    /// Show only this output of `monte_carlo.outputs` (repeat for more). A name
    /// can be written without its group when only one output has it
    #[arg(long = "output-var", value_name = "OUTPUT", conflicts_with = "seek")]
    pub output_var: Vec<String>,

    /// Output file (.json, .yaml or .xlsx)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct ScenariosArgs {
    /// Path to YAML file with scenarios section
    pub file: PathBuf,

    /// Run specific scenario only
    #[arg(short, long)]
    pub scenario: Option<String>,

    /// Output file (.yaml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct DecisionTreeArgs {
    /// Path to YAML file with `decision_tree` section
    pub file: PathBuf,

    /// Export as DOT graph (for Graphviz visualization)
    #[arg(long)]
    pub dot: bool,

    /// Output file (.yaml or .dot)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct RealOptionsArgs {
    /// Path to YAML file with `real_options` section
    pub file: PathBuf,

    /// Value specific option only
    #[arg(long)]
    pub option: Option<String>,

    /// Compare with traditional NPV
    #[arg(long)]
    pub compare_npv: bool,

    /// Output file (.yaml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct TornadoArgs {
    /// Path to YAML file with tornado section
    pub file: PathBuf,

    /// Override output variable to analyze
    #[arg(long)]
    pub output_var: Option<String>,

    /// Output file (.yaml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run an uncertain model at plan
    #[arg(long, help = WHAT_IF_PLAN)]
    pub plan: bool,
}

#[derive(Args)]
pub struct BootstrapArgs {
    /// Path to YAML file with bootstrap section
    pub file: PathBuf,

    /// Number of iterations (overrides YAML config)
    #[arg(short = 'n', long)]
    pub iterations: Option<usize>,

    /// Random seed for reproducibility
    #[arg(long)]
    pub seed: Option<u64>,

    /// Confidence levels (e.g., 0.90,0.95,0.99)
    #[arg(long, value_delimiter = ',')]
    pub confidence: Option<Vec<f64>>,

    /// Output file (.yaml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct BayesianArgs {
    /// Path to YAML file with `bayesian_network` section
    pub file: PathBuf,

    /// Target variable to query
    #[arg(short, long)]
    pub query: Option<String>,

    /// Evidence in format var=state (can repeat)
    #[arg(short, long, value_delimiter = ',')]
    pub evidence: Vec<String>,

    /// Work the network out under a scenario's overrides (a probability may
    /// be a formula over the model's scalars)
    #[arg(short, long)]
    pub scenario: Option<String>,

    /// Output file (.yaml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct TemplatesArgs {
    /// The template to show; omit to list them
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct FunctionsArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SchemaArgs {
    /// Schema version to display (v1, v5, v5.1 ... v5.6, 1.0.0, 5.0.0, 5.1.0 ... 5.6.0)
    pub version: Option<String>,

    /// List available schema versions
    #[arg(short, long)]
    pub list: bool,
}

#[derive(Args)]
pub struct ExamplesArgs {
    /// Example name (monte-carlo, scenarios, decision-tree, etc.)
    pub name: Option<String>,

    /// Execute the example after displaying it
    #[arg(long)]
    pub run: bool,

    /// Output as JSON (for tooling)
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct UpgradeArgs {
    /// Path to YAML file to upgrade
    pub file: PathBuf,

    /// Preview changes without modifying files
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Target model format (default: 5.1.0)
    #[arg(long, default_value = "5.1.0")]
    pub to: String,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Only check for updates, don't install
    #[arg(short = 'c', long)]
    pub check: bool,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct McpArgs {
    /// Folder every file path must stay inside (default: the current folder)
    #[arg(long, value_name = "DIR")]
    pub root: Option<std::path::PathBuf>,

    /// Allow any path the user can reach (no folder limit)
    #[arg(long, conflicts_with = "root")]
    pub any_path: bool,
}

#[derive(Args)]
pub struct ServeArgs {
    /// Host address to bind to (use 0.0.0.0 for all interfaces)
    #[arg(short = 'H', long, default_value = "127.0.0.1", env = "FORGE_HOST")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080", env = "FORGE_PORT")]
    pub port: u16,

    /// Token required on /api/v1/* (prefer the environment variable, which
    /// other users cannot see in the process list)
    #[arg(long, env = "FORGE_API_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Browser origin allowed to call the API, e.g. <http://localhost:3000>
    /// (repeatable; none by default)
    #[arg(long = "cors-origin", value_name = "ORIGIN")]
    pub cors_origins: Vec<String>,
    /// Folder every file path must stay inside (none by default)
    #[arg(long, value_name = "DIR")]
    pub root: Option<std::path::PathBuf>,
}

/// `--format` of `forge compare` and `forge report`: clap reads the name and
/// lists the four in the help.
impl clap::ValueEnum for ReportFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Table, Self::Csv, Self::Json, Self::Markdown]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Table => "table",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Markdown => "markdown",
        }))
    }
}
