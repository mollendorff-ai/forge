# Forge CLI Reference

> Auto-generated from `forge --help`. Do not edit manually.

## Main Help

```
Forge - Deterministic YAML formula validation
96K rows/sec | 60+ Excel functions | Zero AI tokens | Zero emissions

PERFORMANCE:
  10K rows in 107ms | 100K rows in ~1s | Linear O(n) scaling

COMMANDS:
  calculate   - Evaluate formulas in YAML files
  validate    - Check formulas without modifying
  functions   - List all 58 supported Excel functions
  sensitivity - One/two-variable data tables
  goal-seek   - Find input value for target output
  break-even  - Find where output crosses zero
  variance    - Budget vs actual analysis
  compare     - Compare scenarios side-by-side
  export      - YAML to Excel (.xlsx)
  import      - Excel to YAML
  watch       - Auto-calculate on file changes
  audit       - Show formula dependency chain
  update      - Check for updates and self-update

EXAMPLES:
  forge calculate model.yaml                    # Evaluate formulas
  forge sensitivity m.yaml -v price -r 80,120,10 -o profit
  forge goal-seek m.yaml --target profit --value 100000 --vary price
  forge variance budget.yaml actual.yaml       # Budget vs actual

Docs: https://github.com/royalbit/forge

Usage: forge <COMMAND>

Commands:
  calculate    Calculate all formulas in a YAML file
  audit        Show audit trail for a specific variable
  validate     Validate formulas without calculating
  export       Export v1.0.0 array model to Excel .xlsx
  import       Import Excel .xlsx file to YAML v1.0.0
  watch        Watch YAML files and auto-calculate on changes
  compare      Compare results across multiple scenarios
  variance     Compare budget vs actual with variance analysis
  sensitivity  Run sensitivity analysis on model variables
  goal-seek    Find input value to achieve target output
  break-even   Find break-even point (where output = 0)
  update       Check for updates and self-update
  functions    List all supported Excel-compatible functions
  upgrade      Upgrade YAML files to latest schema version
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## calculate

```
Calculate all formulas in a YAML file.

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
    formula: "=@pricing.base_price * volume - @costs.total"

IMPORTANT: Calculate updates ALL files in the chain (Excel-style)!
  If pricing.yaml has stale formulas, they will be recalculated too.
  This ensures data integrity across all referenced files.

Use --dry-run to preview changes without modifying files.

Usage: forge calculate [OPTIONS] <FILE>

Arguments:
  <FILE>
          Path to YAML file (can include other files via 'includes' section)

Options:
  -n, --dry-run
          Preview changes without writing to file

  -v, --verbose
          Show verbose calculation steps

  -s, --scenario <SCENARIO>
          Scenario name to apply (uses variable overrides from 'scenarios' section)

  -h, --help
          Print help (see a summary with '-h')
```

## validate

```
Validate formulas without calculating.

Checks that all formula values match their calculations across ALL files
(main file + all included files). Detects stale values that need recalculation.

CROSS-FILE REFERENCES:
  Validates formulas using @alias.variable syntax:

  includes:
    - file: pricing.yaml
      as: pricing

  Formula example:
    formula: "=@pricing.base_price * 10"

NOTE: Validation checks ALL files in the chain.
  If any included file has stale values, validation will fail.
  Run 'calculate' to update all files.

BATCH VALIDATION:
  forge validate file1.yaml file2.yaml file3.yaml
  Validates multiple files in sequence, reporting all errors.

Usage: forge validate <FILES>...

Arguments:
  <FILES>...
          Path to YAML file(s) to validate

Options:
  -h, --help
          Print help (see a summary with '-h')
```

## audit

```
Show audit trail for a specific variable

Usage: forge audit <FILE> <VARIABLE>

Arguments:
  <FILE>      Path to YAML file
  <VARIABLE>  Variable name to audit

Options:
  -h, --help  Print help
```

## export

```
Export v1.0.0 array model to Excel .xlsx format.

Converts YAML column arrays to Excel worksheets with full formula support.
Each table becomes a separate worksheet. Formulas are translated to Excel syntax.

SUPPORTED FEATURES (Phase 3.1 - Basic Export):
  ✅ Table columns → Excel columns (A, B, C, ...)
  ✅ Data values (Number, Text, Date, Boolean)
  ✅ Multiple tables → Multiple worksheets
  ✅ Scalars → Dedicated "Scalars" worksheet

COMING SOON (Phase 3.2+):
  ⏳ Row formulas → Excel cell formulas (=A2-B2)
  ⏳ Cross-table references (=Sheet!Column)
  ⏳ Aggregation formulas (=SUM(Sheet!A:A))

EXAMPLE:
  forge export quarterly_pl.yaml quarterly_pl.xlsx

NOTE: Only works with v1.0.0 array models. v0.2.0 scalar models are not supported.

Usage: forge export [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>
          Path to v1.0.0 YAML file (must have 'tables' section)

  <OUTPUT>
          Output Excel file path (.xlsx)

Options:
  -v, --verbose
          Show verbose export steps

  -h, --help
          Print help (see a summary with '-h')
```

## import

```
Import Excel .xlsx file to YAML v1.0.0 format.

Converts Excel worksheets to YAML tables with formula preservation.
Each worksheet becomes a table in the output YAML file.

SUPPORTED FEATURES (Phase 4.1 - Basic Import):
  ✅ Excel worksheets → YAML tables
  ✅ Data values (Number, Text, Boolean)
  ✅ Multiple worksheets → One YAML file (one-to-one)
  ✅ "Scalars" sheet → Scalar section
  ⏳ Formula translation (coming in Phase 4.3)

WORKFLOW:
  1. Import existing Excel → YAML
  2. Work with AI + Forge (version control!)
  3. Export back to Excel
  4. Round-trip: Excel → YAML → Excel

EXAMPLE:
  forge import quarterly_pl.xlsx quarterly_pl.yaml

NOTE: Formulas are preserved as Excel syntax (Phase 4.1).
      Formula translation to YAML syntax coming in Phase 4.3.

Usage: forge import [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>
          Path to Excel file (.xlsx)

  <OUTPUT>
          Output YAML file path (or directory if --split-files)

Options:
  -v, --verbose
          Show verbose import steps

      --split-files
          Create separate YAML file per worksheet (v4.4.2)

      --multi-doc
          Create multi-document YAML with --- separators (v4.4.2)

  -h, --help
          Print help (see a summary with '-h')
```

## watch

```
Watch YAML files and auto-calculate on changes.

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

Press Ctrl+C to stop watching.

Usage: forge watch [OPTIONS] <FILE>

Arguments:
  <FILE>
          Path to YAML file to watch

Options:
      --validate
          Only validate (don't calculate)

  -v, --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## compare

```
Compare calculation results across multiple scenarios.

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
  ─────────────────────────────────────────────────
  Variable          Base      Optimistic  Pessimistic
  revenue           $1.2M     $1.8M       $0.9M
  profit            $200K     $450K       -$50K

Usage: forge compare [OPTIONS] <FILE>

Arguments:
  <FILE>
          Path to YAML file

Options:
  -s, --scenarios <SCENARIOS>
          Comma-separated list of scenario names to compare

  -v, --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## variance

```
Compare budget vs actual with variance analysis.

Calculates variances between two YAML files (budget and actual).
Shows absolute variance, percentage variance, and favorability status.

INPUTS:
  Both files must be YAML format (use 'forge import' for Excel files first).
  Variables are matched by name across both files.

VARIANCE TYPES:
  For revenue/income: actual > budget = favorable (✅)
  For expenses/costs: actual < budget = favorable (✅)

THRESHOLD:
  Use --threshold to flag significant variances (default: 10%)
  Variances exceeding threshold are marked with ⚠️

OUTPUT FORMATS:
  Terminal table (default)
  YAML: forge variance budget.yaml actual.yaml -o report.yaml
  Excel: forge variance budget.yaml actual.yaml -o report.xlsx

EXAMPLES:
  forge variance budget.yaml actual.yaml
  forge variance budget.yaml actual.yaml --threshold 5
  forge variance budget.yaml actual.yaml -o variance_report.xlsx

See ADR-002 for design rationale on YAML-only inputs.

Usage: forge variance [OPTIONS] <BUDGET> <ACTUAL>

Arguments:
  <BUDGET>
          Path to budget YAML file

  <ACTUAL>
          Path to actual YAML file

Options:
  -t, --threshold <THRESHOLD>
          Variance threshold percentage for alerts (default: 10)
          
          [default: 10]

  -o, --output <OUTPUT>
          Output file (optional: .yaml or .xlsx)

  -v, --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## sensitivity

```
Run sensitivity analysis by varying one or two inputs.

Varies the specified input variable(s) across a range and shows how the
output variable changes. Essential for understanding model behavior and risk.

ONE-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --range 0.01,0.15,0.02 --output npv

  Shows how NPV changes as growth_rate varies from 1% to 15% in 2% steps.

TWO-VARIABLE ANALYSIS:
  forge sensitivity model.yaml --vary growth_rate --vary2 discount_rate \
      --range 0.01,0.15,0.02 --range2 0.05,0.15,0.05 --output npv

  Shows a matrix of NPV values for each combination of inputs.

RANGE FORMAT:
  start,end,step - e.g., 0.01,0.15,0.02 means 0.01, 0.03, 0.05, ..., 0.15

EXAMPLES:
  forge sensitivity model.yaml -v growth_rate -r 0.05,0.20,0.05 -o profit
  forge sensitivity model.yaml -v price -v2 volume -r 10,50,10 -r2 100,500,100 -o revenue

Usage: forge sensitivity [OPTIONS] --vary <VARY> --range <RANGE> --output <OUTPUT> <FILE>

Arguments:
  <FILE>
          Path to YAML file

Options:
  -v, --vary <VARY>
          Variable to vary (scalar name)

  -r, --range <RANGE>
          Range for first variable: start,end,step

      --vary2 <VARY2>
          Second variable to vary (for 2D analysis)

      --range2 <RANGE2>
          Range for second variable: start,end,step

  -o, --output <OUTPUT>
          Output variable to observe

      --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## goal-seek

```
Find the input value needed to achieve a target output.

Uses numerical methods (bisection) to find what input value produces
the desired output. Useful for answering 'what price do I need?' questions.

EXAMPLES:
  forge goal-seek model.yaml --target profit --value 100000 --vary price
  → Find the price needed to achieve $100,000 profit

  forge goal-seek model.yaml --target npv --value 0 --vary discount_rate
  → Find the discount rate that makes NPV = 0 (IRR)

OPTIONS:
  --min, --max: Override automatic bounds for the search
  --tolerance: Precision of the result (default: 0.0001)

Usage: forge goal-seek [OPTIONS] --target <TARGET> --value <VALUE> --vary <VARY> <FILE>

Arguments:
  <FILE>
          Path to YAML file

Options:
  -t, --target <TARGET>
          Target variable to achieve

      --value <VALUE>
          Desired value for target

  -v, --vary <VARY>
          Variable to adjust

      --min <MIN>
          Minimum bound for search (optional)

      --max <MAX>
          Maximum bound for search (optional)

      --tolerance <TOLERANCE>
          Solution tolerance (default: 0.0001)
          
          [default: 0.0001]

      --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## break-even

```
Find the break-even point where output equals zero.

Special case of goal-seek that finds where a variable crosses zero.
Common for finding break-even units, prices, or margins.

EXAMPLES:
  forge break-even model.yaml --output profit --vary units
  → Find units needed to break even (profit = 0)

  forge break-even model.yaml --output net_margin --vary price
  → Find minimum price for positive margin

Usage: forge break-even [OPTIONS] --output <OUTPUT> --vary <VARY> <FILE>

Arguments:
  <FILE>
          Path to YAML file

Options:
  -o, --output <OUTPUT>
          Output variable to find zero crossing

  -v, --vary <VARY>
          Variable to adjust

      --min <MIN>
          Minimum bound for search (optional)

      --max <MAX>
          Maximum bound for search (optional)

      --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## update

```
Check for updates and optionally self-update the binary.

Downloads the latest release from GitHub and replaces the current binary.
Checksums are verified before installation.

EXAMPLES:
  forge update              # Check and install updates
  forge update --check      # Check only, don't install

PLATFORMS:
  Linux x86_64, Linux ARM64, macOS Intel, macOS ARM, Windows x64

Usage: forge update [OPTIONS]

Options:
      --check
          Only check for updates, don't install

  -h, --help
          Print help (see a summary with '-h')
```

## functions

```
List all supported Excel-compatible functions by category.

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
  forge functions --json    # Output as JSON (for tooling)

Usage: forge functions [OPTIONS]

Options:
      --json
          Output as JSON

  -h, --help
          Print help (see a summary with '-h')
```

## upgrade

```
Upgrade YAML files to latest schema version (v5.0.0).

Automatically migrates YAML files and all included files to the latest schema.
Creates backups before modifying files.

TRANSFORMATIONS:
  - Updates _forge_version to 5.0.0
  - Splits scalars into inputs/outputs based on formula presence:
    - Scalars with value only → inputs section
    - Scalars with formula → outputs section
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
  Original files are backed up as .yaml.bak before modification.

Usage: forge upgrade [OPTIONS] <FILE>

Arguments:
  <FILE>
          Path to YAML file to upgrade

Options:
  -n, --dry-run
          Preview changes without modifying files

      --to <TO>
          Target schema version (default: 5.0.0)
          
          [default: 5.0.0]

  -v, --verbose
          Show verbose output

  -h, --help
          Print help (see a summary with '-h')
```

