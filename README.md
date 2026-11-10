# Forge

> 📌 **R&D Prototype** — Interpret claims as hypotheses, not proven facts.

[![CI](https://github.com/mollendorff-ai/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/mollendorff-ai/forge/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/mollendorff-forge.svg)](https://crates.io/crates/mollendorff-forge)
[![Tests](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/lctavares/b8e2fa78efcf26a20ff97b483050b05a/raw/forge-tests.json)](https://github.com/mollendorff-ai/forge/actions/workflows/ci.yml)
[![Functions](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/lctavares/b8e2fa78efcf26a20ff97b483050b05a/raw/forge-functions.json)](https://github.com/mollendorff-ai/forge)
[![Coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/lctavares/b8e2fa78efcf26a20ff97b483050b05a/raw/forge-coverage.json)](https://github.com/mollendorff-ai/forge/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

**Financial models an AI agent can write and a program can calculate.**

YAML-based financial modeling with Excel-style formula evaluation. Git-native and deterministic. Beta software: see [Status and known limitations](#status-and-known-limitations).

> A raw spreadsheet file is mostly XML. A YAML model is small, readable and diffable.

## Status and known limitations

Forge is a release candidate (v10.0.0-rc.11) and an R&D prototype, built with AI assistance under the author's direction (Claude is listed as a co-author in `Cargo.toml`). Do not rely on it for real financial decisions without checking the results independently.

Known problems as of 2026-11-08 (what is planned, and in what order, is in the [roadmap](.asimov/roadmap.yaml)):

- `forge export` writes a formula that Excel cannot express with the same meaning (Monte Carlo functions, Forge's one-column `VLOOKUP`, `IRR(-cost, flows)` and a few others) as its calculated value and lists it.
- `forge export` does not write `scenarios` (Excel has no equivalent); it lists them as not exported. Keep the YAML file as the source of truth.
- `forge calculate` cannot evaluate `MC.*` distributions (for example `examples/monte-carlo.yaml`); it says so and points to `forge simulate`.
- Each analytics command has been checked once end to end, on the `examples/bagel` models, against calculations written outside Forge. That is one model each, not broad coverage.

## The Problem

| Excel + AI | The Cost |
|------------|----------|
| `.xlsx` is compressed XML | Massive token consumption |
| Cell references like `B7:G42` | AI has to guess what that means |
| No semantic structure | Context wasted on formatting |
| Can't diff, can't PR | No version control |
| AI hallucinates numbers | And Excel can't catch it |

**Size:** for one 5-year DCF model, the `.xlsx` file was about 7× the size of the YAML in bytes (about 19× counting the XML inside it). Token counts were not measured, and an agent that reads only the cells it needs avoids most of this cost.

## Examples

Three models of the kind people have, each built through the MCP tools. Every number below is what the command prints. The tests check each one against a calculation written outside Forge (the price seek with 1,000 runs instead of 5,000).

**How much more a month to save 80,000 of interest on a mortgage?** [`examples/mortgage`](examples/mortgage/README.md) is 450,000 at 4.79% over 25 years, compounded twice a year as Canadian fixed rates are. Paying 300 more a month already means 247 payments instead of 300 and 64,596.88 less interest.

```text
$ forge goal-seek examples/mortgage/mortgage.yaml --target saved.interest --value 80000 --vary loan.extra_per_month
   loan.extra_per_month = 397.044208 → saved.interest = 79999.999976
```

A month-by-month loop in Python, solved with SciPy, gives 397.044208. The model holds that loop too, as a table of 300 months in which each month opens with what the month before closed with (`opening: "=PREV(closing, loan.principal)"`): it is paid off with payment 247, in June 2047, and its interest equals the closed form's. [`examples/periods`](examples/periods/README.md) has a monthly plan and a five-year valuation written the same way.

**What price gives a new bagel shop an 80% chance of making money in its first year?** [`examples/bagel`](examples/bagel/README.md) has twelve months of seasonal demand, five cost lines, and supplier prices kept in an included file. In the simulation, opening demand, staffing, the flour price and the wage are uncertain.

```text
$ forge simulate examples/bagel/bagel_simulate.yaml --seek "P(year.profit > 0) = 0.8" --vary price_per_dozen --min 14 --max 24
   assumptions.price_per_dozen = 18.324083
   P(year.profit > 0) there: 80.00%  (2.92% at 14, 100.00% at 24)
   22 simulations of 5000 iterations, the same draws each time
```

At the planned 16 a dozen, the chance is 34.76%. The simulation names what drives the result: opening demand (+0.84), then staff hours (-0.42), the wage (-0.29) and the flour price (-0.09). The tornado shows how far each input can move the year when it changes alone:

```text
$ forge tornado examples/bagel/bagel.yaml
   assumptions.opening_dozens_per_day |██████████████████████████████| swing 114499.75  (output -56463.56 at low, 58036.20 at high)
   assumptions.price_per_dozen        |████████████████████████      | swing 94192.48  (output -46309.92 at low, 47882.56 at high)
   assumptions.staff_hours_per_day    |████████████████████          | swing 76590.00  (output 46740.32 at low, -29849.68 at high)
   assumptions.rent_per_month         |██████                        | swing 24000.00  (output 10386.32 at low, -13613.68 at high)
   assumptions.card_fee_rate          |█                             | swing 4641.93  (output 3880.94 at low, -760.99 at high)
```

**Where did the household's October go?** [`examples/household`](examples/household/README.md) compares a budget with what came in and went out. Under `income` higher is better, and under `expenses` lower is better.

```text
$ forge variance examples/household/budget.yaml examples/household/actual.yaml
Variable                      Budget       Actual     Variance      Var %   Status
income.side_work                 400          150         -250     -62.5%     ⚠️ ❌
expenses.groceries               700       812.43       112.43      16.1%     ⚠️ ❌
expenses.transport               180         96.5        -83.5     -46.4%     ⚠️ ✅
expenses.eating_out              250       388.75       138.75      55.5%     ⚠️ ❌
expenses.total                  3100      3305.85       205.85       6.6%        ❌
result.left_over                2500      2044.15      -455.85     -18.2%     ⚠️ ❌
```

(Six of its thirteen lines.) The same bagel model also runs scenarios, break-even, a decision tree, real options, a bootstrap and a Bayesian network: see its [README](examples/bagel/README.md).

---

## AI Integration

Any AI coding agent can use Forge — Claude Code, ChatGPT, Gemini, Cursor, Copilot, Aider, or anything that writes files and runs commands:

```text
1. AI writes a YAML model
2. forge calculate model.yaml
3. Deterministic result
```

That's it. No SDK. No API key. No protocol. **Files are the universal interface.**

### Example: AI Agent Swarm

```bash
# You describe the problem to your AI coding assistant
"Model a SaaS acquisition: $10M, 3-year earnout,
 15% growth base case, Monte Carlo on churn risk"

# AI spawns agents, each writes a scenario
├── valuation.yaml      # DCF with earnout structure
├── scenarios.yaml      # Bull/base/bear cases
└── simulation.yaml     # Monte Carlo on churn

# Forge evaluates deterministically
forge calculate valuation.yaml
forge scenarios scenarios.yaml
forge simulate simulation.yaml -n 10000

# Results feed back to AI for synthesis
```

Results come from the formula evaluator, not from a model's own arithmetic, and each one traces to a formula.

MCP server and REST API available for tighter integration (see below).

---

## Validation status

Forge's results are checked against other engines. None of the checks is complete. The unit tests run on every push; the two external suites run weekly in CI and on every pull request that changes the source. [VALIDATION.md](VALIDATION.md) lists what each check covers, its latest result and the command to run it. What a stable release will and will not promise is in [docs/STABILITY.md](docs/STABILITY.md).

| Check | What it covers | Latest result (2026-11-09) |
| --- | --- | --- |
| Unit tests (this repo) | 1,681 tests and 2 doctests, including Black-Scholes and binomial option prices checked against QuantLib reference values | Pass; run on every push |
| [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) | 1,436 formula cases. A case passes only if `forge calculate`, Gnumeric's recalculation of the exported `.xlsx` (LibreOffice's for the 70 cases Gnumeric cannot read), and Forge after export and import all match the expected value | 1,435 pass, 1 skipped; weekly in CI |
| [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r) | Math, date and financial functions, and analytics (Monte Carlo, bootstrap, tornado, sensitivity, decision trees, real options, Bayesian networks) compared with R | 119 function checks pass; 70 of 77 analytics specs compared with R, 7 skipped; weekly in CI |

Next steps: compare more of the skipped analytics specs (elasticity, the exponential distribution, one-point ranges).

---

## What Excel Can't Do

Forge isn't Excel 2.0. It's a purpose-built FP&A engine with 7 analysis types that a plain spreadsheet does not include:

### 1. Monte Carlo Simulation

Model uncertainty with probability distributions. Run 10,000+ iterations to get P10/P50/P90 outcomes.

```yaml
assumptions:
  revenue_growth:
    formula: "=MC.Normal(0.15, 0.05)"  # Mean 15%, StdDev 5%

  project_cost:
    formula: "=MC.Triangular(80000, 100000, 150000)"  # Min/Mode/Max
```

**6 distributions:** Normal, Triangular, Uniform, PERT, Lognormal, Discrete

### 2. Bootstrap Resampling

Generate confidence intervals from historical data without assuming a distribution.

```yaml
bootstrap:
  iterations: 10000
  confidence_levels: [0.90, 0.95, 0.99]
  data: [0.05, -0.02, 0.08, 0.03, ...]  # Historical returns
  method: percentile                    # or basic, or bca (for skewed data)
```

Percentile, basic and BCa intervals; compared with R's `boot` package (see [VALIDATION.md](VALIDATION.md)). BCa stops with an error when it is undefined for the data (for example, every resample is on one side of the estimate).

### 3. Decision Trees

Model sequential decisions with backward induction. Forge finds the optimal path automatically.

```yaml
decision_tree:
  root:
    type: decision
    name: "Invest in R&D?"
    branches:
      invest:
        cost: 2000000
        next: tech_outcome
      dont_invest:
        value: 0
```

### 4. Real Options Analysis

Quantify the value of managerial flexibility using Black-Scholes and binomial methods.

| Option Type | What It Values |
|-------------|----------------|
| **Defer** | Wait before investing |
| **Expand** | Scale up if successful |
| **Contract** | Scale down if weak |
| **Abandon** | Exit and recover salvage |
| **Switch** | Change inputs/outputs |

Unit tests check Black-Scholes and binomial prices against QuantLib reference values.

### 5. Tornado Diagrams

One-at-a-time sensitivity analysis. Instantly see what drives variance.

```bash
forge tornado examples/bagel/bagel.yaml   # the output is under Examples above
```

### 6. Bayesian Networks

Probabilistic graphical models for causal reasoning. Model how risks cascade.

```yaml
bayesian_network:
  nodes:
    economic_conditions:
      states: ["good", "neutral", "bad"]
      prior: [0.3, 0.5, 0.2]

    company_revenue:
      states: ["high", "medium", "low"]
      parents: ["economic_conditions"]       # several parents: rows like "good, high"
      cpt:
        good:    [0.60, 0.30, 0.10]          # one row per parent state
        neutral: [0.30, 0.50, 0.20]
        bad:     [0.10, 0.30, 0.60]
```

Query it with `forge bayesian examples/bayesian.yaml -q company_revenue -e economic_conditions=bad`.

A unit test checks a small network against a hand calculation of a pgmpy example.

### 7. Scenario Analysis

Probability-weighted scenarios with expected value calculation.

```yaml
scenarios:
  bull_case:
    probability: 0.30
    scalars:
      revenue_growth: 0.25

  bear_case:
    probability: 0.20
    scalars:
      revenue_growth: -0.10
```

---

## MCP Integration (Optional)

For Claude Desktop or MCP-compatible hosts, Forge exposes 20 tools via Model Context Protocol:

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge",
      "args": ["mcp"]
    }
  }
}
```

### Available Tools

**Core (5)**

| Tool | What It Does |
|------|--------------|
| `forge_validate` | Check YAML for formula errors |
| `forge_calculate` | Execute all formulas |
| `forge_audit` | Trace formula dependencies |
| `forge_export` | YAML → Excel |
| `forge_import` | Excel → YAML |

**Analysis (5)**

| Tool | What It Does |
|------|--------------|
| `forge_sensitivity` | 1D/2D what-if analysis |
| `forge_goal_seek` | Find input for target output |
| `forge_break_even` | Find where output = 0 |
| `forge_variance` | Budget vs actual analysis |
| `forge_compare` | Multi-scenario comparison |

**Engines (7)**

| Tool | What It Does |
|------|--------------|
| `forge_simulate` | Run Monte Carlo simulation with probabilistic distributions |
| `forge_scenarios` | Run probability-weighted scenario analysis (Base/Bull/Bear) |
| `forge_decision_tree` | Analyze decision trees using backward induction |
| `forge_real_options` | Value managerial flexibility using real options pricing |
| `forge_tornado` | Generate tornado sensitivity diagram |
| `forge_bootstrap` | Non-parametric bootstrap resampling for confidence intervals |
| `forge_bayesian` | Bayesian network inference with optional evidence |

**Discovery (3)**

| Tool | What It Does |
|------|--------------|
| `forge_schema` | Get JSON Schema for Forge YAML model formats |
| `forge_functions` | List all 175 supported functions with descriptions |
| `forge_examples` | Get runnable YAML examples for all capabilities |

All tools accept inline YAML via `content` parameter — no host filesystem access required. Works with sandboxed clients (Claude.ai, Cursor, containers).

See [docs/MCP.md](docs/MCP.md) for the full integration guide.

---

## 175 Functions

175 functions: most are Excel-compatible, plus 7 Excel doesn't have: 6 for FP&A, and `PREV` for the row above. Not every Excel function is covered; `forge functions` lists what is.

| Category | Examples |
|----------|----------|
| **Financial** | NPV, IRR, MIRR, XNPV, XIRR, PMT, PV, FV, RATE, NPER |
| **Date** | TODAY, DATE, YEAR, MONTH, DATEDIF, EDATE, EOMONTH, NETWORKDAYS |
| **Conditional** | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, MAXIFS, MINIFS |
| **Math** | ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER |
| **Logic** | IF, AND, OR, LET, SWITCH, INDIRECT, LAMBDA |
| **Lookup** | MATCH, INDEX, VLOOKUP, HLOOKUP, XLOOKUP, CHOOSE |
| **Statistical** | MEDIAN, VAR, STDEV, PERCENTILE, QUARTILE, CORREL |
| **Text** | CONCAT, TRIM, UPPER, LOWER, LEN, MID |
| **Aggregation** | SUM, AVERAGE, MIN, MAX, COUNT |
| **Array** | UNIQUE, COUNTUNIQUE, FILTER, SORT |
| **FP&A-Native** | VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, BREAKEVEN_UNITS, BREAKEVEN_REVENUE, SCENARIO |

Run `forge functions` for full syntax and examples.

---

## FP&A Functions Excel Doesn't Have

Every analyst builds these manually. Forge has them built-in:

```yaml
# VARIANCE - What Excel makes you do manually
variance_analysis:
  revenue_var: "=VARIANCE(actual.revenue, budget.revenue)"           # Returns: -5000
  revenue_pct: "=VARIANCE_PCT(actual.revenue, budget.revenue)"       # Returns: -5%
  revenue_status: "=VARIANCE_STATUS(actual.revenue, budget.revenue)" # Returns a numeric status code

# BREAKEVEN - Instant unit economics
breakeven:
  units_required: "=BREAKEVEN_UNITS(500000, 150, 60)"     # Returns: 5,556 units
  revenue_required: "=BREAKEVEN_REVENUE(500000, 0.60)"    # Returns: $833,333
```

**Type-aware variance:** Costs use inverted logic (under budget = BEAT).

---

## Quick Start

```bash
# Install the prebuilt binary with Homebrew (macOS, Linux); this repository is the tap
brew tap mollendorff-ai/forge https://github.com/mollendorff-ai/forge
brew install mollendorff-ai/forge/forge

# Or with cargo-binstall (releases after 10.0.0-rc.11; Windows too)
cargo binstall mollendorff-forge

# Or build from source
cargo install --path .

# Validate a model
forge validate model.yaml

# Calculate with scenario
forge calculate model.yaml --scenario optimistic

# Export to Excel with formulas (see known limitations)
forge export model.yaml output.xlsx
```

---

## Editors

Forge models are YAML files, so any editor with YAML support works. Two things help:

- **The schema**: [`schema/forge.schema.json`](schema/forge.schema.json) (which checks a file against the schema of the format its `_forge_version` names) gives completion of keys and marks a misspelt key or a text where a number belongs as you type, in any editor that runs yaml-language-server (VS Code with the YAML extension, Zed, Neovim, Helix). Put this first line in a model, or see [schema/README.md](schema/README.md) for the settings:
  ```yaml
  # yaml-language-server: $schema=https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json
  ```
- **VS Code**: [`editors/vscode`](editors/vscode/README.md) highlights formulas (functions, names, `@include` values, numbers) inside any YAML file and applies the schema to `*.forge.yaml` files. It is not on the Marketplace yet; the README says how to build and install it.

There is no language server: run `forge validate` and `forge calculate` in a terminal, or `forge watch` to recalculate on every save.

---

## Example: 5-Year DCF Model

```yaml
_forge_version: "5.2.0"

assumptions:
  revenue_y1: 1000000
  growth_rate: 0.15
  gross_margin: 0.65
  opex_pct: 0.30
  tax_rate: 0.25
  discount_rate: 0.10

projections:
  year: {start: 2027, count: 5, unit: year}
  # Each year's revenue is the year before, grown
  revenue: "=PREV(revenue * (1 + assumptions.growth_rate), assumptions.revenue_y1)"
  gross_profit: "=revenue * assumptions.gross_margin"
  opex: "=revenue * assumptions.opex_pct"
  ebit: "=gross_profit - opex"
  tax: "=ebit * assumptions.tax_rate"
  net_income: "=ebit - tax"

valuation:
  total_revenue:
    formula: "=SUM(projections.revenue)"
  avg_gross_margin:
    formula: "=SUM(projections.gross_profit) / SUM(projections.revenue)"
  npv_cash_flows:
    formula: "=NPV(assumptions.discount_rate, projections.net_income)"

scenarios:
  base:
    growth_rate: 0.15
  bull:
    growth_rate: 0.25
    gross_margin: 0.70
  bear:
    growth_rate: 0.05
    gross_margin: 0.55
```

```bash
forge calculate model.yaml --dry-run                  # preview results without writing the file
forge calculate model.yaml --scenario bull --dry-run
forge sensitivity model.yaml -v assumptions.growth_rate -r 0.05,0.30,0.05 -o valuation.npv_cash_flows
```

`forge calculate` prints every calculated value, including each table column. Without `--dry-run`, it also writes the calculated scalar values back into the file and keeps a `.bak` copy; table columns keep their formulas in the file. The MCP `forge_calculate` tool returns the column values under `tables.<name>.values`.

---

## Commands

```bash
# Core Operations
forge validate <files...>           # Validate YAML model(s)
forge calculate <file>              # Execute all formulas
forge audit <file> <variable>       # Trace formula dependencies

# Analysis
forge sensitivity <file> -v VAR -r RANGE -o OUTPUT
forge goal-seek <file> --target VAR --value N --vary INPUT
forge break-even <file> -o OUTPUT -v INPUT
forge variance <budget> <actual> --threshold PCT

# Prediction & Simulation
forge simulate <file> --iterations N    # Monte Carlo
forge scenarios <file>                  # Scenario analysis
forge decision-tree <file>              # Decision trees
forge real-options <file>               # Real options
forge tornado <file>                    # Tornado diagrams
forge bootstrap <file>                  # Bootstrap resampling

# Excel Bridge
forge export <yaml> <xlsx>          # YAML → Excel with formulas
forge import <xlsx> <yaml>          # Excel → YAML

# AI Integration
forge mcp                           # Start MCP server
forge serve --port 8080             # Start REST API

# Reference
forge functions                     # List all 175 functions
forge schema v5                     # Show JSON schema
forge examples monte-carlo          # Show example YAML
```

### Side by side: `forge report`

`forge report` shows a calculated file's scalars as a table: one row per group, one column per key, and groups with the same keys lined up. It reads what `forge calculate` wrote and never writes the file. `--format markdown` pastes into a document; `csv` and `json` keep full precision.

```bash
forge calculate options.yaml && forge report options.yaml --format markdown
```

See [docs/REPORT.md](docs/REPORT.md).

---

## Documentation Commands

```bash
# View JSON schemas for model validation
forge schema                    # List available versions
forge schema v1                 # Show v1.0.0 schema (scalar-only)
forge schema v5.1               # Show the 5.1.0 model format schema (use for new models)
forge schema v5.2               # 5.1.0 plus the period column (ADR-053)
forge schema v5 > schema.json   # Pipe to file for IDE use

# View runnable examples for Forge capabilities
forge examples                  # List all examples
forge examples monte-carlo      # Monte Carlo simulation
forge examples scenarios        # Probability-weighted scenarios
forge examples decision-tree    # Sequential decision modeling
forge examples real-options     # Option pricing for flexibility
forge examples tornado          # Sensitivity analysis
forge examples bootstrap        # Confidence intervals
forge examples bayesian         # Probabilistic graphical models
forge examples variance         # Budget vs actual analysis
forge examples breakeven        # Break-even calculations

forge examples monte-carlo --run  # Show and execute example
forge examples --json             # Machine-readable list
```

---

## Intended use

Forge is new and there is no adoption data yet. These are the uses it is designed for:

| Team | Intended use |
| --- | --- |
| **FP&A** | 3-statement models, budget vs actual, rolling forecasts |
| **M&A** | DCF valuations, sensitivity analysis, scenario comparison |
| **Consulting** | Client financial models with version control |
| **Fintech** | Automated projections via API, embedded calculations |
| **AI Agents** | A coding agent writes YAML and Forge evaluates it |

---

## Quality Assurance

| Metric | Value |
| --- | --- |
| **Unit tests** | 1,480 unit tests and 2 doctests (counted 2026-11-06) |
| **Functions** | 175, of which 6 are FP&A-native and one (`PREV`) reads the row above |
| **Lints** | clippy pedantic is enabled and CI fails on warnings |
| **External validation** | Partial; see [Validation status](#validation-status) |

### Test Architecture

```text
forge (this repo)             validation repos
├── 1,681 tests               ├── forge-e2e-gnumeric: 1,436 formula specs
├── #[cfg(test)] modules      └── forge-e2e-r: 119 function checks, 77 analytics specs
└── 2 doctests
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [CHANGELOG](CHANGELOG.md) | Version history |
| [Roadmap](.asimov/roadmap.yaml) | What is next and why, in order; current state and what is unverified |
| [Architecture](docs/architecture/) | Technical design and ADRs |
| [Examples](docs/EXAMPLES.md) | YAML model examples |
| [Features](docs/FEATURES.md) | Full feature reference |
| [Market Analysis](docs/MARKET_ANALYSIS.md) | Investment thesis and positioning |
| [JSON Schema](schema/) | Model validation schema |

---

## Development

```bash
cargo test                              # Run all tests
cargo clippy -- -D warnings             # Lint (pedantic)
make check                              # Full CI check
```

---

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

This is the standard dual-license used by the Rust ecosystem (rustc, Tokio, Serde, etc.).

---

**Built with [Asimov](https://github.com/mollendorff-ai/asimov)** - The AI autonomy framework.
