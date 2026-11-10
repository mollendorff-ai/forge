// During coverage builds, stubbed main doesn't use imports
#![cfg_attr(coverage, allow(unused_imports))]

use clap::{Parser, Subcommand};
use mollendorff_forge::api::{run_api_server, server::ApiConfig};
use mollendorff_forge::cli;
use mollendorff_forge::error::{ForgeError, ForgeResult};
use mollendorff_forge::mcp::ForgeMcpServer;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "Git-native financial modeling in YAML. 175 functions. Beta.")]
#[command(long_about = "Forge - Git-native financial modeling
175 functions | Beta | Validation suites: forge-e2e-gnumeric, forge-e2e-r

COMMANDS:
  calculate     - Execute formulas, update values
  validate      - Check model integrity
  audit         - Trace formula dependencies
  functions     - List all 175 supported functions
  schema        - Display JSON schema for model validation
  examples      - Show runnable YAML examples
  simulate      - Monte Carlo simulation with distributions
  scenarios     - Probability-weighted scenario analysis
  decision-tree - Sequential decisions with backward induction
  real-options  - Value defer/expand/abandon flexibility
  tornado       - One-at-a-time sensitivity diagrams
  bootstrap     - Non-parametric confidence intervals
  bayesian      - Bayesian network inference
  sensitivity   - One/two-variable data tables
  goal-seek     - Find input for target output
  break-even    - Find zero-crossing point
  variance      - Budget vs actual analysis
  compare       - Multi-scenario comparison
  report        - Scalars side by side, from a calculated file
  export        - YAML -> Excel (.xlsx) with formulas
  import        - Excel -> YAML
  watch         - Auto-calculate on save
  upgrade       - Upgrade YAML to latest schema
  update        - Check for updates and self-update
  mcp           - Start MCP server for AI integration
  serve         - Start HTTP REST API server

EXAMPLES:
  forge calculate model.yaml                    # Execute formulas
  forge simulate model.yaml --iterations 10000  # Monte Carlo
  forge scenarios model.yaml                    # Scenario analysis
  forge decision-tree model.yaml                # Decision tree
  forge tornado model.yaml                      # Sensitivity diagram
  forge variance budget.yaml actual.yaml        # Budget vs actual
  forge report model.yaml --format markdown     # Scalars as a table
  forge schema v5                               # Show JSON schema
  forge examples monte-carlo                    # Show Monte Carlo example

Docs: https://mollendorff.ai/forge")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(long_about = "Calculate all formulas in a YAML file.

Evaluates formulas in dependency order and writes each scalar formula's
result next to it in the file.

CROSS-FILE REFERENCES:
  Add an '_includes' section to read the scalars of other files:

  _includes:
    - file: pricing.yaml
      as: pricing
    - file: costs.yaml
      as: costs

  Then use @alias.name in formulas:
    formula: \"=@pricing.base_price * volume - @costs.total\"

  An included file is calculated first, so its own formulas count, but only
  the file given is written. Run 'forge calculate costs.yaml' to store the
  results of an included file in it.

Use --dry-run to preview changes without modifying files.")]
    /// Calculate all formulas in a YAML file
    Calculate {
        /// Path to YAML file (can include other files via an '_includes' section)
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

Checks that the result stored next to each formula is what the formula
calculates to, with the included files calculated first.

CROSS-FILE REFERENCES:
  Formulas may read included files with @alias.name:

  _includes:
    - file: pricing.yaml
      as: pricing

  Formula example:
    formula: \"=@pricing.base_price * 10\"

NOTE: Only the stored results of the files given are checked. To check an
  included file's own stored results, give it too:
  forge validate model.yaml pricing.yaml

BATCH VALIDATION:
  forge validate file1.yaml file2.yaml file3.yaml
  Validates multiple files in sequence, reporting all errors.")]
    /// Validate formulas without calculating
    Validate {
        /// Path to YAML file(s) to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    #[command(
        long_about = "Export a model to an Excel .xlsx workbook, formulas included.

The model is calculated first, and each formula cell carries Forge's result.

LAYOUT:
  Each table -> a sheet, one column per data or formula column, in file order
  Each scalar group -> a sheet with Name (A) and Value (B), one scalar per row
  Ungrouped scalars -> a sheet named Scalars
  An included file -> its own sheets (costs.Scalars); @costs.x reads its cells

Row formulas and scalar formulas become live Excel formulas. A formula Excel
cannot express with the same meaning (MC.*, LET, COUNTUNIQUE, ...) is written
as its value, with the Forge formula in a cell note, and listed. Scenarios
are not exported (Excel has no equivalent).

EXAMPLE:
  forge export examples/bagel/bagel.yaml bagel.xlsx"
    )]
    /// Export a model to an Excel .xlsx workbook
    Export {
        /// Path to the model (.yaml)
        input: PathBuf,

        /// Output Excel file path (.xlsx)
        output: PathBuf,

        /// Show verbose export steps
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "Import an Excel .xlsx workbook as a Forge model (format 5.1.0).

Reads the layout forge export writes, and plain workbooks too:
  A sheet whose header is Name, Value -> a scalar group (Scalars -> ungrouped)
  A sheet costs.Scalars of an exported include -> costs.yaml and @costs.x
  Any other sheet -> a table, header in the first row

Excel formulas are translated to Forge formulas (table.column, group.name).
A formula Forge cannot express (a reference to another row, a single table
cell in a scalar formula, named ranges) is imported as its value and listed.

WORKFLOW:
  forge import model.xlsx model.yaml     # Excel -> YAML
  forge calculate model.yaml             # work with the YAML, in git
  forge export model.yaml model.xlsx     # back to Excel

--split-files writes one file per sheet (refused when sheets refer to each
other); --multi-doc writes one file with one document per sheet."
    )]
    /// Import an Excel .xlsx workbook as a Forge model
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

Monitors the folder of the specified file: a change to any .yaml file in it
(the model, or an included file kept beside it) runs it again. An included
file in another folder is not watched.
When a change is detected, automatically runs validation/calculation.

FEATURES:
  Real-time file monitoring
  Auto-calculate on save
  Debounced updates (waits for file write to complete)
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
    /// Compare results across multiple scenarios
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

    #[command(
        long_about = "Show a calculated model's scalars side by side: one row per group,
one column per key.

Groups that have the same keys share a table; other groups get their own,
in file order. Inputs (values written with no formula) come before formulas.

report reads the values that `forge calculate` wrote. It does not calculate
and never writes the file. A formula without a value is shown as — and named
in a note on standard error; tables are not shown.

NUMBERS:
  table and markdown: two decimals and thousands separators (1,450.00); a
  number under 1 keeps four significant digits (0.0479)
  csv and json: full precision, no separators

EXAMPLES:
  forge calculate options.yaml
  forge report options.yaml
  forge report options.yaml --format markdown
  forge report options.yaml --group oak_suite,elm_loft --keys rent,year1_monthly
  forge report options.yaml --no-inputs --format csv"
    )]
    /// Show a calculated model's scalars side by side
    Report {
        /// Path to a calculated YAML file
        file: PathBuf,

        /// Show only these groups (comma-separated)
        #[arg(long, value_delimiter = ',')]
        group: Vec<String>,

        /// Show only these keys, in this order, as one table (comma-separated)
        #[arg(long, value_delimiter = ',', conflicts_with = "no_inputs")]
        keys: Vec<String>,

        /// Output format
        #[arg(long, default_value = "table", value_parser = ["table", "csv", "json", "markdown"])]
        format: String,

        /// Leave out inputs, the values written with no formula
        #[arg(long)]
        no_inputs: bool,
    },

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
    /// Compare budget vs actual with variance analysis
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
    /// Run sensitivity analysis on model variables
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
  --tolerance: How close the output must get to the target (default: 0.0001)")]
    /// Find input value to achieve target output
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

        /// How close the output must get to the target (default: 0.0001)
        #[arg(long, default_value = "0.0001")]
        tolerance: f64,

        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },

    #[command(long_about = "Find the break-even point where output equals zero.

Special case of goal-seek that finds where a variable crosses zero.
Common for finding break-even units, prices, or margins.

EXAMPLES:
  forge break-even model.yaml --output profit --vary units
  -> Find units needed to break even (profit = 0)

  forge break-even model.yaml --output net_margin --vary price
  -> Find minimum price for positive margin")]
    /// Find break-even point (where output = 0)
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

    #[command(long_about = "Run Monte Carlo simulation for probabilistic analysis.

Uses probability distributions (MC.Normal, MC.Triangular, etc.) to model
uncertainty in input variables and calculate output distributions.

DISTRIBUTIONS:
  MC.Normal(mean, stdev)        - Symmetric uncertainty
  MC.Triangular(min, mode, max) - Expert estimates (min/likely/max)
  MC.Uniform(min, max)          - Equal probability in range
  MC.PERT(min, mode, max)       - Smooth project estimates
  MC.Lognormal(mean, stdev)     - Non-negative values (prices, revenue)
  MC.Discrete(vals, probs)      - Custom scenarios with probabilities

YAML CONFIGURATION:
  monte_carlo:
    enabled: true
    iterations: 10000
    sampling: latin_hypercube  # spreads samples evenly; steadier results per iteration
    seed: 12345                # For reproducibility
    outputs:
      - variable: valuation.npv
        percentiles: [10, 50, 90]
        thresholds: [\"> 0\", \"> 250000\"]

  assumptions:
    revenue:
      formula: \"=MC.Normal(1000000, 150000)\"
    costs:
      formula: \"=MC.Triangular(400000, 500000, 600000)\"

An MC.* input can also be in an included file (read as @costs.flour).

OUTPUT:
  - Statistics: mean, median, std dev, min, max
  - Percentiles: P5, P10, P25, P50, P75, P90, P95 and those asked for
  - Probabilities: P(NPV > 0), P(NPV > 250000)
  - Drivers: the sampled inputs ranked by rank correlation with each output

EXAMPLES:
  forge simulate model.yaml                    # Use YAML config
  forge simulate model.yaml -n 10000           # Override iterations
  forge simulate model.yaml --seed 42          # Reproducible
  forge simulate model.yaml -o results.json    # JSON output
  forge simulate model.yaml --scenario rainy   # Under a scenario's overrides
  forge simulate model.yaml --seek \"P(year.profit > 0) = 0.8\" --vary price --min 14 --max 24
                                               # The price at which 80% of runs make money")]
    /// Run Monte Carlo simulation
    Simulate {
        /// Path to YAML file with `monte_carlo`: section
        file: PathBuf,

        /// Number of iterations (overrides YAML config)
        #[arg(short = 'n', long)]
        iterations: Option<usize>,

        /// Random seed for reproducibility
        #[arg(long)]
        seed: Option<u64>,

        /// Sampling method: `monte_carlo` or `latin_hypercube`
        #[arg(long)]
        sampling: Option<String>,

        /// Apply a scenario's overrides first (an override of an MC.* input fixes it)
        #[arg(long)]
        scenario: Option<String>,

        /// Find the value of --vary that meets a probability, e.g. "P(year.profit > 0) = 0.8"
        #[arg(long, requires_all = ["vary", "min", "max"])]
        seek: Option<String>,

        /// Input to vary for --seek (a scalar with a plain value)
        #[arg(long, requires = "seek")]
        vary: Option<String>,

        /// Lowest value of --vary to try
        #[arg(long, requires = "seek", allow_hyphen_values = true)]
        min: Option<f64>,

        /// Highest value of --vary to try
        #[arg(long, requires = "seek", allow_hyphen_values = true)]
        max: Option<f64>,

        /// Output file (.json, .yaml or .xlsx)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "Run probability-weighted scenario analysis (Base/Bull/Bear cases).

Scenarios are discrete outcomes with assigned probabilities. Unlike Monte Carlo
(continuous distributions), scenarios model mutually exclusive futures.

YAML CONFIGURATION:
  scenarios:
    base_case:
      probability: 0.50
      description: \"Market grows 5%\"
      scalars:
        revenue_growth: 0.05
    bull_case:
      probability: 0.30
      scalars:
        revenue_growth: 0.15
    bear_case:
      probability: 0.20
      scalars:
        revenue_growth: -0.10

OUTPUT:
  - Per-scenario results with all calculated outputs
  - Expected value (probability-weighted) for each output
  - Risk profile showing best/worst case outcomes

EXAMPLES:
  forge scenarios model.yaml                    # Run all scenarios
  forge scenarios model.yaml --scenario bull    # Run specific scenario
  forge scenarios model.yaml -o results.yaml    # Export results"
    )]
    /// Run scenario analysis with probability weights
    Scenarios {
        /// Path to YAML file with scenarios section
        file: PathBuf,

        /// Run specific scenario only
        #[arg(short, long)]
        scenario: Option<String>,

        /// Output file (.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Analyze decision trees using backward induction.

Decision trees model sequential choices and uncertain outcomes.
Uses backward induction (rollback) to find optimal decision policy.

NODE TYPES:
  decision - Choice point (we control), solved by max(child values)
  chance   - Uncertainty (we don't control), solved by expected value
  terminal - End state with known value

YAML CONFIGURATION:
  decision_tree:
    name: \"R&D Investment\"
    root:
      type: decision
      name: \"Invest?\"
      branches:
        invest:
          cost: 2000000
          next: tech_outcome
        dont_invest:
          value: 0
    nodes:
      tech_outcome:
        type: chance
        branches:
          success:
            probability: 0.60
            value: 5000000
          failure:
            probability: 0.40
            value: -2000000

OUTPUT:
  - Optimal path through tree
  - Expected value at root
  - Decision policy (what to do at each decision node)
  - Risk profile (best/worst case)

EXAMPLES:
  forge decision-tree model.yaml              # Analyze tree
  forge decision-tree model.yaml --dot        # Export DOT for Graphviz
  forge decision-tree model.yaml -o out.yaml  # Export results")]
    /// Analyze decision trees with backward induction
    DecisionTree {
        /// Path to YAML file with `decision_tree` section
        file: PathBuf,

        /// Export as DOT graph (for Graphviz visualization)
        #[arg(long)]
        dot: bool,

        /// Output file (.yaml or .dot)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "Value managerial flexibility using real options analysis.

Real options quantify the value of flexibility to defer, expand, contract,
or abandon projects. Uses Black-Scholes or Binomial Tree pricing.

OPTION TYPES:
  defer    - Wait before investing (value of learning)
  expand   - Scale up if successful
  contract - Scale down if weak
  abandon  - Exit and recover salvage value
  switch   - Change inputs/outputs

YAML CONFIGURATION:
  real_options:
    name: \"Phased Factory\"
    method: binomial
    underlying:
      current_value: 10000000
      volatility: 0.30
      risk_free_rate: 0.05
      time_horizon: 3
    options:
      - type: defer
        name: \"Wait up to 2 years\"
        max_deferral: 2
        exercise_cost: 8000000
      - type: abandon
        name: \"Sell assets\"
        salvage_value: 3000000

OUTPUT:
  - Value of each option
  - Total option value
  - Project value with options
  - Decision recommendation

EXAMPLES:
  forge real-options model.yaml               # Value all options
  forge real-options model.yaml --option defer  # Value specific option
  forge real-options model.yaml --compare-npv   # Compare with traditional NPV"
    )]
    /// Value real options (defer/expand/abandon)
    RealOptions {
        /// Path to YAML file with `real_options` section
        file: PathBuf,

        /// Value specific option only
        #[arg(long)]
        option: Option<String>,

        /// Compare with traditional NPV
        #[arg(long)]
        compare_npv: bool,

        /// Output file (.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Generate tornado diagram for sensitivity analysis.

Tornado diagrams show which inputs have the most impact on outputs.
Each input is varied one-at-a-time while others stay at base values.

YAML CONFIGURATION:
  tornado:
    output: npv
    inputs:
      - name: revenue_growth
        low: 0.02
        high: 0.08
      - name: discount_rate
        low: 0.08
        high: 0.12
      - name: operating_margin
        low: 0.15
        high: 0.25

OUTPUT:
  - Bars sorted by impact (largest first)
  - Base value reference
  - Low and high values for each input

EXAMPLES:
  forge tornado model.yaml                  # Generate diagram
  forge tornado model.yaml --output npv     # Override output variable
  forge tornado model.yaml -o results.yaml  # Export results")]
    /// Generate tornado sensitivity diagram
    Tornado {
        /// Path to YAML file with tornado section
        file: PathBuf,

        /// Override output variable to analyze
        #[arg(long)]
        output_var: Option<String>,

        /// Output file (.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Run bootstrap resampling for confidence intervals.

Bootstrap is a non-parametric method that resamples from historical data
with replacement. No distribution assumptions required.

YAML CONFIGURATION:
  bootstrap:
    iterations: 10000
    confidence_levels: [0.90, 0.95, 0.99]
    seed: 12345
    data: [0.05, -0.02, 0.08, 0.03, -0.05, 0.12]
    statistic: mean  # or median, std, var

OUTPUT:
  - Original statistic value
  - Bootstrap mean and standard error
  - Confidence intervals at each level
  - Bias estimate

EXAMPLES:
  forge bootstrap model.yaml                    # Run analysis
  forge bootstrap model.yaml -n 50000           # Override iterations
  forge bootstrap model.yaml --confidence 0.99  # Set confidence level")]
    /// Bootstrap resampling for confidence intervals
    Bootstrap {
        /// Path to YAML file with bootstrap section
        file: PathBuf,

        /// Number of iterations (overrides YAML config)
        #[arg(short = 'n', long)]
        iterations: Option<usize>,

        /// Random seed for reproducibility
        #[arg(long)]
        seed: Option<u64>,

        /// Confidence levels (e.g., 0.90,0.95,0.99)
        #[arg(long, value_delimiter = ',')]
        confidence: Option<Vec<f64>>,

        /// Output file (.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(long_about = "Run Bayesian network inference.

Bayesian networks are probabilistic graphical models for causal reasoning.
Uses Variable Elimination algorithm for efficient inference.

YAML CONFIGURATION:
  bayesian_network:
    name: \"Credit Risk\"
    nodes:
      economic_conditions:
        type: discrete
        states: [good, neutral, bad]
        prior: [0.3, 0.5, 0.2]
      default_probability:
        type: discrete
        states: [low, medium, high]
        parents: [economic_conditions]
        cpt:
          good: [0.8, 0.15, 0.05]
          neutral: [0.4, 0.4, 0.2]
          bad: [0.1, 0.3, 0.6]

  A node with several parents (parents: [a, b]) has one CPT row per
  combination of their states, in that order: \"x, y\": [...]

EXAMPLES:
  forge bayesian model.yaml                           # Query all nodes
  forge bayesian model.yaml --query default_prob      # Query specific node
  forge bayesian model.yaml -e economy=bad            # Set evidence")]
    /// Bayesian network inference
    Bayesian {
        /// Path to YAML file with `bayesian_network` section
        file: PathBuf,

        /// Target variable to query
        #[arg(short, long)]
        query: Option<String>,

        /// Evidence in format var=state (can repeat)
        #[arg(short, long, value_delimiter = ',')]
        evidence: Vec<String>,

        /// Output file (.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "List all supported Excel-compatible functions by category.

Most functions are Excel-compatible; a few are FP&A-native functions Excel
does not have. The list is generated from the function registry and shows the
total and the count for each category.

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

    #[command(long_about = "Display JSON schema for validating Forge YAML models.

The versions are model formats (_forge_version), not Forge versions:
  v1.0.0 - Scalars only
  v5.0.0 - Tables, row formulas and the analysis sections
  v5.1.0 - v5.0.0 plus the short form: plain-number inputs, formulas
           without value: null (ADR-051)
  v5.2.0 - v5.1.0 plus the period column: a table column written as
           {start: 2026-12, count: 300, unit: month} (ADR-053)

EXAMPLES:
  forge schema                # List available versions
  forge schema v5.1           # Show the v5.1.0 schema
  forge schema v5.1 > s.json  # Pipe to file for IDE use")]
    /// Display JSON schema for model validation
    Schema {
        /// Schema version to display (v1, v5, v5.1, v5.2, 1.0.0, 5.0.0, 5.1.0, 5.2.0)
        version: Option<String>,

        /// List available schema versions
        #[arg(short, long)]
        list: bool,
    },

    #[command(
        long_about = "Display runnable example YAML models for Forge capabilities.

Examples demonstrate Forge-specific features beyond Excel formulas:
  monte-carlo   - Probabilistic simulation with distributions
  scenarios     - Probability-weighted scenario analysis
  decision-tree - Sequential decisions with backward induction
  real-options  - Option pricing for managerial flexibility
  tornado       - One-at-a-time sensitivity analysis
  bootstrap     - Non-parametric confidence intervals
  bayesian      - Probabilistic graphical models
  variance      - Budget vs actual analysis
  breakeven     - Break-even calculations

EXAMPLES:
  forge examples                    # List all examples
  forge examples monte-carlo        # Show Monte Carlo example
  forge examples monte-carlo --run  # Show and execute example
  forge examples --json             # List as JSON (for tooling)"
    )]
    /// Show example YAML models for Forge capabilities
    Examples {
        /// Example name (monte-carlo, scenarios, decision-tree, etc.)
        name: Option<String>,

        /// Execute the example after displaying it
        #[arg(long)]
        run: bool,

        /// Output as JSON (for tooling)
        #[arg(long)]
        json: bool,
    },

    #[command(
        long_about = "Upgrade a model and its includes to the latest model format (5.1.0).

_forge_version is the model format version, not the Forge version.
Creates backups before modifying files.

TO 5.1.0 (default, ADR-051):
  - `name: {value: 16, formula: null}` becomes `name: 16`
  - `value: null` and `formula: null` lines are removed
  - Edited in place: comments, quotes and layout are kept
  - Checked: the converted file must read as the same model, or it is not written

TO 5.2.0 (ADR-053):
  - As to 5.1.0, with _forge_version 5.2.0: the format a table with a
    period column needs. A 5.1.0 file only changes that line.

TO 5.0.0 (from 1.0.0):
  - Splits scalars into inputs/outputs based on formula presence
  - Preserves all existing metadata

RECURSIVE PROCESSING:
  If the file has _includes, all included files are upgraded FIRST.
  Circular includes are detected and handled.

EXAMPLES:
  forge upgrade model.yaml              # Upgrade file and includes
  forge upgrade model.yaml --dry-run    # Preview changes only
  forge upgrade model.yaml --to 5.2.0   # For period columns
  forge upgrade model.yaml --to 5.0.0   # Older format, long form

BACKUP:
  Original files are backed up as .yaml.bak before modification."
    )]
    /// Upgrade YAML files to latest schema version
    Upgrade {
        /// Path to YAML file to upgrade
        file: PathBuf,

        /// Preview changes without modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Target model format (default: 5.1.0)
        #[arg(long, default_value = "5.1.0")]
        to: String,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "Check for updates and optionally install the latest version.

Downloads and installs the latest Forge release from GitHub.
Supports all platforms: Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), Windows.

EXAMPLES:
  forge update              # Check and install update (with confirmation)
  forge update --check      # Check only, don't install
  forge update --verbose    # Show detailed progress

INSTALLATION:
  - Downloads the correct binary for your platform
  - Backs up the current binary to forge.bak
  - Installs the new version in place
  - Preserves permissions

NOTE:
  Requires curl to be installed (available on all supported platforms).
  The update replaces the current binary - restart forge to use the new version."
    )]
    /// Check for updates and install latest version
    Update {
        /// Only check for updates, don't install
        #[arg(short = 'c', long)]
        check: bool,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(
        long_about = "Start MCP (Model Context Protocol) server for AI integration.

Runs a JSON-RPC server over stdin/stdout for use with Claude Desktop,
Claude Code, and other MCP-compatible AI hosts.

CONFIGURATION:
  Add to your MCP client settings (e.g., Claude Desktop):

  {
    \"mcpServers\": {
      \"forge\": {
        \"command\": \"forge\",
        \"args\": [\"mcp\"]
      }
    }
  }

AVAILABLE TOOLS (20):
  Core:      forge_validate, forge_calculate, forge_audit, forge_export, forge_import
  Analysis:  forge_sensitivity, forge_goal_seek, forge_break_even, forge_variance, forge_compare
  Engines:   forge_simulate, forge_scenarios, forge_decision_tree, forge_real_options,
             forge_tornado, forge_bootstrap, forge_bayesian
  Discovery: forge_schema, forge_functions, forge_examples

EXAMPLE:
  forge mcp   # Start MCP server (reads JSON-RPC from stdin)"
    )]
    /// Start MCP server for AI integration (JSON-RPC over stdio)
    Mcp,

    #[command(long_about = "Start HTTP REST API server.

Provides RESTful endpoints for all Forge operations:
  POST /api/v1/validate  - Validate YAML model files
  POST /api/v1/calculate - Calculate formulas (with dry-run support)
  POST /api/v1/audit     - Audit variable dependency trees
  POST /api/v1/export    - Export YAML to Excel (.xlsx)
  POST /api/v1/import    - Import Excel to YAML

Additional endpoints:
  GET  /health           - Health check
  GET  /version          - Server version info
  GET  /                 - API documentation

Security:
  The API reads and writes the files that requests name, as your user.
  By default it listens on 127.0.0.1, rejects requests whose Host header
  is not this machine, and does not allow browser requests from other
  websites (no CORS). To listen on another address, set a token; requests
  to /api/v1/* must then send 'Authorization: Bearer <token>'.

Features:
  Graceful shutdown on SIGINT/SIGTERM
  JSON response format with request IDs
  Tracing and structured logging

EXAMPLES:
  forge serve                                        # Start on localhost:8080
  FORGE_API_TOKEN=... forge serve --host 0.0.0.0     # All interfaces, token required
  forge serve --cors-origin http://localhost:3000    # Allow one web app")]
    /// Start HTTP REST API server
    Serve {
        /// Host address to bind to (use 0.0.0.0 for all interfaces)
        #[arg(short = 'H', long, default_value = "127.0.0.1", env = "FORGE_HOST")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080", env = "FORGE_PORT")]
        port: u16,

        /// Token required on /api/v1/* (prefer the environment variable, which
        /// other users cannot see in the process list)
        #[arg(long, env = "FORGE_API_TOKEN", hide_env_values = true)]
        token: Option<String>,

        /// Browser origin allowed to call the API, e.g. <http://localhost:3000>
        /// (repeatable; none by default)
        #[arg(long = "cors-origin", value_name = "ORIGIN")]
        cors_origins: Vec<String>,
    },
}

/// CLI entry point - excluded from coverage (ADR-006)
/// Parses CLI args and dispatches to library functions.
/// Cannot unit test: reads from `std::env::args()`
/// Tested via: `cli_integration_tests.rs`
#[cfg(not(coverage))]
fn main() {
    // Print the readable message, not the Debug form a returned Result would give.
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(coverage))]
#[allow(clippy::too_many_lines)] // CLI dispatch function — splitting would fragment command handling
fn run() -> ForgeResult<()> {
    let cli = Cli::parse();

    // `forge functions | head` must end quietly, like other command-line tools.
    // The servers keep Rust's default, so a client that disconnects is an
    // error on that connection, not the end of the process.
    #[cfg(unix)]
    if !matches!(cli.command, Commands::Mcp | Commands::Serve { .. }) {
        // SAFETY: restoring the default action for SIGPIPE before any output.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }

    match cli.command {
        Commands::Calculate {
            file,
            dry_run,
            verbose,
            scenario,
        } => cli::calculate(&file, dry_run, verbose, scenario.as_deref()),

        Commands::Audit { file, variable } => cli::audit(&file, &variable),

        Commands::Validate { files } => cli::validate(&files),

        Commands::Export {
            input,
            output,
            verbose,
        } => cli::export(&input, &output, verbose),

        Commands::Import {
            input,
            output,
            verbose,
            split_files,
            multi_doc,
        } => cli::import(&input, &output, verbose, split_files, multi_doc),

        Commands::Watch {
            file,
            validate,
            verbose,
        } => cli::watch(&file, validate, verbose),

        Commands::Compare {
            file,
            scenarios,
            verbose,
        } => cli::compare(&file, &scenarios, verbose),

        Commands::Report {
            file,
            group,
            keys,
            format,
            no_inputs,
        } => cli::report(
            &file,
            &cli::ReportOptions {
                groups: group,
                keys,
                no_inputs,
            },
            format.parse::<cli::ReportFormat>()?,
        ),

        Commands::Variance {
            budget,
            actual,
            threshold,
            output,
            verbose,
        } => cli::variance(&budget, &actual, threshold, output.as_deref(), verbose),

        Commands::Sensitivity {
            file,
            vary,
            range,
            vary2,
            range2,
            output,
            verbose,
        } => cli::sensitivity(
            &file,
            &vary,
            &range,
            vary2.as_deref(),
            range2.as_deref(),
            &output,
            verbose,
        ),

        Commands::GoalSeek {
            file,
            target,
            value,
            vary,
            min,
            max,
            tolerance,
            verbose,
        } => cli::goal_seek(&file, &target, value, &vary, (min, max), tolerance, verbose),

        Commands::BreakEven {
            file,
            output,
            vary,
            min,
            max,
            verbose,
        } => cli::break_even(&file, &output, &vary, min, max, verbose),

        Commands::Simulate {
            file,
            iterations,
            seed,
            sampling,
            output,
            verbose,
            scenario,
            seek,
            vary,
            min,
            max,
        } => {
            let options = cli::SimulateOptions {
                iterations,
                seed,
                sampling: sampling.as_deref(),
                scenario: scenario.as_deref(),
            };
            match (seek, vary, min, max) {
                (Some(seek), Some(vary), Some(min), Some(max)) => {
                    cli::probability_seek(&file, options, &seek, &vary, min, max)
                },
                _ => cli::simulate(&file, options, output, verbose),
            }
        },

        Commands::Scenarios {
            file,
            scenario,
            output,
            verbose,
        } => cli::scenarios(&file, scenario.as_deref(), output, verbose),

        Commands::DecisionTree {
            file,
            dot,
            output,
            verbose,
        } => cli::decision_tree(&file, dot, output, verbose),

        Commands::RealOptions {
            file,
            option,
            compare_npv,
            output,
            verbose,
        } => cli::real_options(&file, option.as_deref(), compare_npv, output, verbose),

        Commands::Tornado {
            file,
            output_var,
            output,
            verbose,
        } => cli::tornado(&file, output_var.as_deref(), output, verbose),

        Commands::Bootstrap {
            file,
            iterations,
            seed,
            confidence,
            output,
            verbose,
        } => cli::bootstrap(&file, iterations, seed, confidence, output, verbose),

        Commands::Bayesian {
            file,
            query,
            evidence,
            output,
            verbose,
        } => cli::bayesian(&file, query.as_deref(), &evidence, output, verbose),

        Commands::Functions { json } => cli::functions(json),

        Commands::Schema { version, list } => cli::schema(version.as_deref(), list),

        Commands::Examples { name, run, json } => cli::examples(name, run, json),

        Commands::Upgrade {
            file,
            dry_run,
            to,
            verbose,
        } => cli::upgrade(&file, dry_run, &to, verbose),

        Commands::Update { check, verbose } => cli::update(check, verbose),

        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| ForgeError::Validation(format!("Failed to create runtime: {e}")))?;
            rt.block_on(async {
                use rmcp::{transport::io, ServiceExt};
                let service = ForgeMcpServer::new()
                    .serve(io::stdio())
                    .await
                    .map_err(|e| ForgeError::Validation(format!("MCP server error: {e}")))?;
                service
                    .waiting()
                    .await
                    .map_err(|e| ForgeError::Validation(format!("MCP server error: {e}")))?;
                Ok(())
            })
        },

        Commands::Serve {
            host,
            port,
            token,
            cors_origins,
        } => {
            let config = ApiConfig {
                host,
                port,
                token,
                cors_origins,
            };
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| ForgeError::Validation(format!("Failed to create runtime: {e}")))?;
            rt.block_on(run_api_server(config))
                .map_err(|e| ForgeError::Validation(format!("Server error: {e}")))?;
            Ok(())
        },
    }
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
fn main() -> ForgeResult<()> {
    Ok(())
}
