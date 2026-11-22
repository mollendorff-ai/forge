//! The command line: every command and its help text, each command's flags
//! in `flags.rs`. `run.rs` runs them.

use crate::cli::ReportFormat;
use clap::builder::PossibleValue;
use clap::{Parser, Subcommand};

mod flags;
pub use flags::{
    AuditArgs, BayesianArgs, BootstrapArgs, BreakEvenArgs, CalculateArgs, CompareArgs,
    DecisionTreeArgs, ExamplesArgs, ExportArgs, FunctionsArgs, GoalSeekArgs, ImportArgs, McpArgs,
    RealOptionsArgs, ReportArgs, ScenariosArgs, SchemaArgs, SensitivityArgs, ServeArgs,
    SimulateArgs, TornadoArgs, UpdateArgs, UpgradeArgs, ValidateArgs, VarianceArgs, WatchArgs,
};

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "Git-native financial modeling in YAML. 203 functions. Release candidate.")]
#[command(long_about = "Forge - Git-native financial modeling
203 functions | Release candidate | Validated against Gnumeric, R and Gambit (validation/)

COMMANDS:
  calculate     - Execute formulas, update values
  validate      - Check model integrity
  audit         - Trace formula dependencies
  functions     - List all 203 supported functions
  schema        - Display JSON schema for model validation
  examples      - Show runnable YAML examples
  simulate      - Monte Carlo simulation with distributions
  scenarios     - Probability-weighted scenario analysis
  decision-tree - Sequential decisions with backward induction
  real-options  - Value defer/expand/abandon flexibility
  tornado       - One-at-a-time sensitivity diagrams
  game          - Dominated strategies and equilibria of games
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
#[command(version = env!("FORGE_VERSION_TEXT"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

When a value changes, the file as it was is kept next to it as
model.yaml.bak. Use --dry-run to preview changes without modifying files.

Nothing is written under --scenario (a scenario states its results under its
expect:, which forge validate checks) or --plan, which calculates an uncertain
model (MC.* inputs) with each uncertain input at its plan figure.")]
    /// Calculate all formulas in a YAML file
    Calculate(CalculateArgs),

    /// Show audit trail for a specific variable
    Audit(AuditArgs),

    #[command(
        long_about = "Validate a model's stored results, and what it states it should give.

Checks that the result stored next to each formula is what the formula
calculates to, with the included files calculated first, and that each
expectation (expect:) is met.

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

EXPECTED RESULTS (format 5.6.0, ADR-074): what the file and each scenario
  should give, by any name forge calculate prints (a game's results too), as
  a number or a formula over numbers. Each scenario with one is calculated,
  and every result that differs by more than 1e-6 (relative above 1) is named
  with the scenario and both numbers:
  scenarios: {costly: {scalars: {c: 0.5}, expect: {kinship.barrier: =2/11}}}

BATCH VALIDATION:
  forge validate file1.yaml file2.yaml file3.yaml
  Validates multiple files in sequence, reporting all errors."
    )]
    /// Validate formulas without calculating
    Validate(ValidateArgs),

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
as its value (an MC.* input as its plan value), with the Forge formula in a
cell note, and listed. Scenarios are not exported (Excel has no equivalent).

EXAMPLE:
  forge export examples/bagel/bagel.yaml bagel.xlsx"
    )]
    /// Export a model to an Excel .xlsx workbook
    Export(ExportArgs),

    #[command(
        long_about = "Import an Excel .xlsx workbook as a Forge model (format 5.1.0).

Reads the layout forge export writes, and plain workbooks too:
  A sheet whose header is Name, Value -> a scalar group (Scalars -> ungrouped)
  A sheet costs.Scalars of an exported include -> costs.yaml and @costs.x
  Any other sheet -> a table, header in the first row

Excel formulas are translated to Forge formulas (table.column, group.name).
A formula Forge cannot express (a reference to another row, a single table
cell in a scalar formula, a range where Forge reads the row's value, named
ranges) is imported as its value and listed.

WORKFLOW:
  forge import model.xlsx model.yaml     # Excel -> YAML
  forge calculate model.yaml             # work with the YAML, in git
  forge export model.yaml model.xlsx     # back to Excel

--split-files writes one file per sheet (refused when sheets refer to each
other); --multi-doc writes one file with one document per sheet."
    )]
    /// Import an Excel .xlsx workbook as a Forge model
    Import(ImportArgs),

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
    Watch(WatchArgs),

    #[command(long_about = "Compare calculation results across multiple scenarios.

Runs calculations for each specified scenario and displays results side-by-side.
Useful for sensitivity analysis and what-if modeling.

SCENARIOS IN YAML:
  A scenario sets scalars, by their full name (group.name). Two forms; use
  one of them for all the scenarios of a file.

  Flat, the overrides directly:

  scenarios:
    optimistic:
      assumptions.growth_rate: 0.12
      assumptions.churn_rate: 0.01
    pessimistic:
      assumptions.growth_rate: 0.02
      assumptions.churn_rate: 0.05

  Full, with a description and scalars (and a probability, optional here):

  scenarios:
    optimistic:
      description: Faster growth, less churn
      scalars:
        assumptions.growth_rate: 0.12
        assumptions.churn_rate: 0.01
    pessimistic:
      description: Slow growth, more churn
      scalars:
        assumptions.growth_rate: 0.02
        assumptions.churn_rate: 0.05

  compare does not weigh the scenarios, so it needs no probability. forge
  scenarios weighs those that have one, whose probabilities must sum to 1,
  and names the rest.

EXAMPLE:
  forge compare model.yaml --scenarios optimistic,pessimistic
  forge compare model.yaml --scenarios optimistic,pessimistic --format csv

OUTPUT:
  variable                 optimistic  pessimistic
  assumptions.growth_rate        0.12         0.02
  assumptions.churn_rate         0.01         0.05
  assumptions.customers         1,200        1,200
  assumptions.price                49           49
  summary.revenue              65,268       57,036

FORMATS:
  table (the default), csv, json and markdown, as in forge report. csv and
  json print the numbers to 15 significant digits and nothing else; table and
  markdown show them as forge report does (1,450.00; whole numbers without
  decimals; an input as written).

WIDE TABLES:
  A table wider than the terminal is printed in blocks of columns that fit,
  each with the row names: one row per scalar, or one row per scenario when
  that needs fewer blocks. COLUMNS sets the width; output to a file or a pipe
  is not wrapped.")]
    /// Compare results across multiple scenarios
    Compare(CompareArgs),

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
  number under 1 keeps four significant digits (0.0479). A column whose
  values are all whole numbers has no decimals (1,700, 3, 0), decided per
  column, so 3 and 3.50 never share one. An input is shown as written where
  rounding would change it (0.14975)
  csv and json: 15 significant digits, no separators

WIDE TABLES:
  A model with many keys makes a table wider than a phone or a narrow
  terminal. --keys narrows it to the columns you choose, in your order:
  forge report options.yaml --keys rent,year1_monthly

EXAMPLES:
  forge calculate options.yaml
  forge report options.yaml
  forge report options.yaml --format markdown
  forge report options.yaml --group oak_suite,elm_loft --keys rent,year1_monthly
  forge report options.yaml --no-inputs --format csv"
    )]
    /// Show a calculated model's scalars side by side
    Report(ReportArgs),

    #[command(long_about = "Compare budget vs actual with variance analysis.

Calculates variances between two YAML files (budget and actual).
Shows absolute variance, percentage variance, and favorability status.

INPUTS:
  Both files must be YAML format (use 'forge import' for Excel files first).
  Variables are matched by name across both files.

VARIANCE TYPES:
  For revenue/income: actual > budget = favorable
  For expenses/costs: actual < budget = favorable

WHICH WAY IS BETTER:
  A scalar can say it in the model, as better: higher or better: lower:
    sales: {value: 5000, better: higher}
  Without it, Forge goes by the name and its group: costs, expenses and rent
  are better lower, revenue, sales and income better higher, and a line it
  cannot place has no direction (neither favorable nor unfavorable).

THRESHOLD:
  Use --threshold to flag significant variances (default: 10%).
  The exit code is 0 however many lines exceed it; with --fail-on-alert
  it is 1 when any line does, for a script or CI.

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
    Variance(VarianceArgs),

    #[command(long_about = "Run sensitivity analysis by varying one or two inputs.

Varies the specified input variable(s) across a range and shows how the
output variable changes. Essential for understanding model behavior and risk.

ONE-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --range 0.01,0.15,0.02 --output-var npv

  Shows how NPV changes as growth_rate varies from 1% to 15% in 2% steps.

TWO-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --vary2 discount_rate \\
      --range 0.01,0.15,0.02 --range2 0.05,0.15,0.05 --output-var npv

  Shows a matrix of NPV values for each combination of inputs.

RANGE FORMAT:
  start,end,step - e.g., 0.01,0.15,0.02 means 0.01, 0.03, 0.05, ..., 0.15

A name can be written without its group (price for assumptions.price)
when only one scalar has it.

EXAMPLES:
  forge sensitivity model.yaml --vary growth_rate -r 0.05,0.20,0.05 --output-var profit
  forge sensitivity model.yaml --vary price --vary2 volume -r 10,50,10 --range2 100,500,100 --output-var revenue")]
    /// Run sensitivity analysis on model variables
    Sensitivity(SensitivityArgs),

    #[command(long_about = "Find the input value needed to achieve a target output.

Uses numerical methods (bisection) to find what input value produces
the desired output. Useful for answering 'what price do I need?' questions.

EXAMPLES:
  forge goal-seek model.yaml --target profit --value 100000 --vary price
  -> Find the price needed to achieve $100,000 profit

  forge goal-seek model.yaml --target npv --value 0 --vary discount_rate
  -> Find the discount rate that makes NPV = 0 (IRR)

A name can be written without its group (price for assumptions.price)
when only one scalar has it.

OPTIONS:
  --min, --max: Override automatic bounds for the search
  --tolerance: How close the output must get to the target (default: 0.0001)")]
    /// Find input value to achieve target output
    GoalSeek(GoalSeekArgs),

    #[command(long_about = "Find the break-even point where output equals zero.

Special case of goal-seek that finds where a variable crosses zero.
Common for finding break-even units, prices, or margins.

A name can be written without its group (price for assumptions.price)
when only one scalar has it.

EXAMPLES:
  forge break-even model.yaml --output-var profit --vary units
  -> Find units needed to break even (profit = 0)

  forge break-even model.yaml --output-var net_margin --vary price
  -> Find minimum price for positive margin")]
    /// Find break-even point (where output = 0)
    BreakEven(BreakEvenArgs),

    #[command(long_about = "Run Monte Carlo simulation for probabilistic analysis.

Uses probability distributions (MC.Normal, MC.Triangular, etc.) to model
uncertainty in input variables and calculate output distributions.

DISTRIBUTIONS:
  MC.Normal(mean, stdev)        - Symmetric uncertainty
  MC.Triangular(min, mode, max) - Expert estimates (min/likely/max)
  MC.Uniform(min, max)          - Equal probability in range
  MC.PERT(min, mode, max)       - Smooth project estimates
  MC.Lognormal(mean, stdev)     - Non-negative values (prices, revenue)
  MC.Discrete({0, 0.3}, {0.9, 0.1}) - A few outcomes, each with its probability

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
  forge simulate model.yaml --output-var year.profit
                                               # Only that output of monte_carlo.outputs
  forge simulate model.yaml --seek \"P(year.profit > 0) = 0.8\" --vary price --min 14 --max 24
                                               # The price at which 80% of runs make money")]
    /// Run Monte Carlo simulation
    Simulate(SimulateArgs),

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
    all_cash:
      description: \"A choice, not a chance: no probability\"
      scalars:
        cash_share: 1

  A scenario without a probability is a choice to compare, not a chance to
  weigh: forge scenarios leaves it out and names it, and forge compare puts
  it beside the others. The probabilities of the rest must sum to 1.

OUTPUT:
  - Per-scenario results with all calculated outputs
  - Expected value (probability-weighted) for each output
  - Risk profile showing best/worst case outcomes
  - The scenarios left out because they have no probability

EXAMPLES:
  forge scenarios model.yaml                    # Run all scenarios
  forge scenarios model.yaml --scenario bull    # Run specific scenario
  forge scenarios model.yaml -o results.yaml    # Export results"
    )]
    /// Run scenario analysis with probability weights
    Scenarios(ScenariosArgs),

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
    DecisionTree(DecisionTreeArgs),

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
    RealOptions(RealOptionsArgs),

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
  forge tornado model.yaml --output-var npv # Override output variable
  forge tornado model.yaml -o results.yaml  # Export results")]
    /// Generate tornado sensitivity diagram
    Tornado(TornadoArgs),

    /// Find the equilibria of the games in a model, and play its tournaments
    Game(crate::cli::commands::GameArgs),

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
    Bootstrap(BootstrapArgs),

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
    Bayesian(BayesianArgs),

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
    Functions(FunctionsArgs),

    #[command(long_about = "Display JSON schema for validating Forge YAML models.

The versions are model formats (_forge_version), not Forge versions:
  v1.0.0 - Scalars only
  v5.0.0 - Tables, row formulas and the analysis sections
  v5.1.0 - v5.0.0 plus the short form: plain-number inputs, formulas
           without value: null (ADR-051)
  v5.2.0 - v5.1.0 plus the period column: a table column written as
           {start: 2026-12, count: 300, unit: month} (ADR-053)
  v5.3.0 - v5.2.0 plus the empty cell, null in a column of numbers (ADR-059)
  v5.4.0 - v5.3.0 plus a date as a scalar (ADR-060) and format: (ADR-061)
  v5.5.0 - v5.4.0 plus games: payoff tables of formulas, and their equilibria (ADR-068)
  v5.6.0 - v5.5.0 plus expect: what the file and each scenario should give,
           checked by forge validate (ADR-074)

EXAMPLES:
  forge schema                # List available versions
  forge schema v5.1           # Show the v5.1.0 schema
  forge schema v5.1 > s.json  # Pipe to file for IDE use")]
    /// Display JSON schema for model validation
    Schema(SchemaArgs),

    #[command(
        long_about = "Display runnable example YAML models for Forge capabilities.

Start a new model from `starter`; the others show Forge's analysis commands:
  starter       - A first model: a group of scalars and a table
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
  forge examples starter > model.yaml  # Start a new model
  forge examples monte-carlo        # Show Monte Carlo example
  forge examples monte-carlo --run  # Show and execute example
  forge examples --json             # List as JSON (for tooling)"
    )]
    /// Show example YAML models for Forge capabilities
    Examples(ExamplesArgs),

    #[command(
        long_about = "Upgrade a model and its includes to the latest model format (5.1.0).

_forge_version is the model format version, not the Forge version.
Creates backups before modifying files.

TO 5.1.0 (default, ADR-051):
  - `name: {value: 16, formula: null}` becomes `name: 16`
  - `value: null` and `formula: null` lines are removed
  - Edited in place: comments, quotes and layout are kept
  - Checked: the converted file must read as the same model, or it is not written

TO 5.2.0, 5.3.0, 5.4.0, 5.5.0 OR 5.6.0 (ADR-053, ADR-059, ADR-060, ADR-061,
ADR-068, ADR-074):
  - As to 5.1.0, with the _forge_version a period column (5.2.0), an empty
    cell (5.3.0), a date or a format: (5.4.0), a game (5.5.0) or expect:
    (5.6.0) needs. 5.1.0 changes that line. A file with a group named
    expect is not upgraded to 5.6.0: rename the group first.

TO 5.0.0 (from 1.0.0):
  - Splits scalars into inputs/outputs by formula presence, keeping metadata

RECURSIVE PROCESSING:
  If the file has _includes, all included files are upgraded FIRST.
  Circular includes are detected and handled.

EXAMPLES:
  forge upgrade model.yaml              # Upgrade file and includes
  forge upgrade model.yaml --dry-run    # Preview changes only
  forge upgrade model.yaml --to 5.4.0   # Dates as scalars, format: (5.3.0: empty cells)
  forge upgrade model.yaml --to 5.0.0   # Older format, long form

BACKUP:
  Original files are backed up as .yaml.bak before modification."
    )]
    /// Upgrade YAML files to latest schema version
    Upgrade(UpgradeArgs),

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
    Update(UpdateArgs),

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

FILES:
  The tools read and write the files an agent names, as your user, and only
  inside one folder: the folder the server starts in, or --root DIR. A path
  outside it (.. and links resolved), or a model including one, is refused.
  --any-path lifts the limit.

EXAMPLE:
  forge mcp --root ~/models   # Start MCP server (reads JSON-RPC from stdin)"
    )]
    /// Start MCP server for AI integration (JSON-RPC over stdio)
    Mcp(McpArgs),

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
  --root DIR refuses any path outside DIR, as for forge mcp.

Features:
  Graceful shutdown on SIGINT/SIGTERM
  JSON response format with request IDs
  Tracing and structured logging

EXAMPLES:
  forge serve                                        # Start on localhost:8080
  FORGE_API_TOKEN=... forge serve --host 0.0.0.0     # All interfaces, token required
  forge serve --cors-origin http://localhost:3000    # Allow one web app")]
    /// Start HTTP REST API server
    Serve(ServeArgs),
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

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::{CommandFactory, Parser};

    /// Split a help line into words, keeping a quoted part as one word.
    fn words(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut word = String::new();
        let mut quoted = false;
        for c in line.chars() {
            match c {
                // A shell comment or redirect ends the command
                '#' | '>' if !quoted && word.is_empty() => break,
                '"' => quoted = !quoted,
                ' ' if !quoted => {
                    if !word.is_empty() {
                        out.push(std::mem::take(&mut word));
                    }
                },
                _ => word.push(c),
            }
        }
        if !word.is_empty() {
            out.push(word);
        }
        out
    }

    /// Every `forge ...` example in a command's help parses as written.
    #[test]
    fn every_example_in_the_help_parses() {
        for sub in Cli::command().get_subcommands() {
            let help = sub
                .get_long_about()
                .map(ToString::to_string)
                .unwrap_or_default();
            let mut pending = String::new();
            for raw in help.lines() {
                let line = raw.trim();
                if pending.is_empty() && !line.starts_with("forge ") {
                    continue;
                }
                if let Some(start) = line.strip_suffix('\\') {
                    pending.push_str(start);
                    pending.push(' ');
                    continue;
                }
                let full = format!("{pending}{line}");
                pending.clear();
                assert!(
                    Cli::try_parse_from(words(&full)).is_ok(),
                    "help example that does not parse: {full}"
                );
            }
        }
    }

    /// `-o` is the output file and `-v` is `--verbose`, in every command.
    #[test]
    fn short_flags_mean_the_same_in_every_command() {
        for sub in Cli::command().get_subcommands() {
            let name = sub.get_name();
            for arg in sub.get_arguments() {
                let id = arg.get_id().as_str();
                let short = arg.get_short();
                assert!(short != Some('o') || id == "output", "{name} -o is {id}");
                assert!(short != Some('v') || id == "verbose", "{name} -v is {id}");
            }
        }
    }
}
