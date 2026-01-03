# Forge

> 📌 **R&D Prototype** — Interpret claims as hypotheses, not proven facts.

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/royalbit-forge.svg)](https://crates.io/crates/royalbit-forge)
[![Tests](https://img.shields.io/badge/tests-2133_passing-brightgreen)](https://github.com/royalbit/forge)
[![Functions](https://img.shields.io/badge/functions-173-blue)](https://github.com/royalbit/forge)
[![Coverage](https://img.shields.io/badge/coverage-100%25-brightgreen)](https://github.com/royalbit/forge)
[![License: Elastic-2.0](https://img.shields.io/badge/License-Elastic--2.0-blue.svg)](LICENSE)

**AI hallucinates numbers. Forge doesn't.**

YAML-based financial modeling with Excel formula evaluation. Git-native. Deterministic. Battle-tested against Gnumeric and R.

> Excel burns tokens. YAML doesn't.
> AI is trained on millions of YAML files. Not spreadsheets.

## The Problem

| Excel + AI | The Cost |
|------------|----------|
| `.xlsx` is compressed XML | Massive token consumption |
| Cell references like `B7:G42` | AI has to guess what that means |
| No semantic structure | Context wasted on formatting |
| Can't diff, can't PR | No version control |
| AI hallucinates numbers | And Excel can't catch it |

**The math:** A 50KB Excel file can burn 100K+ tokens when parsed. The same model in YAML? Under 2K tokens.

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

No hallucinated numbers. Every result traceable to a formula.

MCP server and REST API available for tighter integration (see below).

---

## Battle-Tested Math

**Don't trust us. Trust Gnumeric and R.**

Every Forge formula is validated against independent, battle-tested open-source tools:

### Dual Validation Architecture

| Validator | Tests | What It Proves |
|-----------|-------|----------------|
| **Gnumeric** | 714 formulas | Excel compatibility - independent spreadsheet engine agrees |
| **R** | 2,957 conditions | Statistical accuracy - FDA/EMA-grade mathematical validation |
| **Roundtrip** | 72 tests | YAML → XLSX → Gnumeric → CSV formula preservation |

### The Validation Process

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Forge YAML     │ ──► │  Export XLSX    │ ──► │  Gnumeric       │
│  =NPV(0.1, cf)  │     │  with formulas  │     │  recalculates   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                                                        ▼
                                                ┌─────────────────┐
                                                │  Compare CSV    │
                                                │  Forge = Gnumeric│
                                                └─────────────────┘
```

**If Gnumeric and R agree with Forge, the math is right.** No "trust me" - just proof.

### Why This Matters

- **No circular dependencies** - Forge doesn't validate itself
- **Peer-reviewed tools** - Gnumeric and R are industry standards
- **Regulatory-grade** - R is used by FDA, EMA for drug approval calculations
- **2,957 statistical conditions** - Not just "does it run" but "is the math correct"

---

## What Excel Can't Do

Forge isn't Excel 2.0. It's a purpose-built FP&A engine with 7 analytical capabilities Excel cannot replicate:

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
```

**R-validated** against the `boot` package.

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

**QuantLib-validated** pricing models.

### 5. Tornado Diagrams

One-at-a-time sensitivity analysis. Instantly see what drives variance.

```text
NPV Sensitivity Analysis (Base: $1.2M)

Revenue Growth    |████████████████████| ± $450K
Discount Rate     |██████████████      | ± $320K
Operating Margin  |██████████          | ± $180K
Tax Rate          |████                | ± $75K
```

### 6. Bayesian Networks

Probabilistic graphical models for causal reasoning. Model how risks cascade.

```yaml
bayesian_network:
  nodes:
    economic_conditions:
      states: ["good", "neutral", "bad"]
      prior: [0.3, 0.5, 0.2]

    default_probability:
      parents: ["economic_conditions", "management_quality"]
      # Conditional probability tables...
```

**pgmpy-validated** inference algorithms.

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

For Claude Desktop or MCP-compatible hosts, Forge exposes 10 tools via Model Context Protocol:

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

| Tool | What It Does |
|------|--------------|
| `forge_validate` | Check YAML for formula errors |
| `forge_calculate` | Execute all formulas |
| `forge_audit` | Trace formula dependencies |
| `forge_sensitivity` | 1D/2D what-if analysis |
| `forge_goal_seek` | Find input for target output |
| `forge_break_even` | Find where output = 0 |
| `forge_variance` | Budget vs actual analysis |
| `forge_compare` | Multi-scenario comparison |
| `forge_export` | YAML → Excel |
| `forge_import` | Excel → YAML |

> **Roadmap Note:** MCP currently exposes 10 of 17 CLI features. The 7 analytical engines
> (Monte Carlo, Bootstrap, Bayesian, Decision Trees, Real Options, Tornado, Scenarios) are
> CLI-only for now. API server has 5 endpoints. Full MCP/API parity + OpenAPI spec planned
> for v10.0.0-beta.1 after E2E validation against R and Gnumeric is complete.

**50x token efficiency.** YAML at a fraction of Excel's token cost.

---

## 173 Functions

All Excel-compatible functions plus 6 FP&A-native functions Excel doesn't have.

| Category | Count | Examples |
|----------|-------|----------|
| **Financial** | 13 | NPV, IRR, MIRR, XNPV, XIRR, PMT, PV, FV, RATE, NPER |
| **Date** | 11 | TODAY, DATE, YEAR, MONTH, DATEDIF, EDATE, EOMONTH, NETWORKDAYS |
| **Conditional** | 8 | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, MAXIFS, MINIFS |
| **Math** | 9 | ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER |
| **Logic** | 7 | IF, AND, OR, LET, SWITCH, INDIRECT, LAMBDA |
| **Lookup** | 6 | MATCH, INDEX, VLOOKUP, HLOOKUP, XLOOKUP, CHOOSE |
| **Statistical** | 6 | MEDIAN, VAR, STDEV, PERCENTILE, QUARTILE, CORREL |
| **Text** | 6 | CONCAT, TRIM, UPPER, LOWER, LEN, MID |
| **Aggregation** | 5 | SUM, AVERAGE, MIN, MAX, COUNT |
| **Array** | 4 | UNIQUE, COUNTUNIQUE, FILTER, SORT |
| **FP&A-Native** | 6 | VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, BREAKEVEN_UNITS, BREAKEVEN_REVENUE, SCENARIO |

Run `forge functions` for full syntax and examples.

---

## FP&A Functions Excel Doesn't Have

Every analyst builds these manually. Forge has them built-in:

```yaml
# VARIANCE - What Excel makes you do manually
variance_analysis:
  revenue_var: "=VARIANCE(actual.revenue, budget.revenue)"           # Returns: -5000
  revenue_pct: "=VARIANCE_PCT(actual.revenue, budget.revenue)"       # Returns: -5%
  revenue_status: "=VARIANCE_STATUS(actual.revenue, budget.revenue)" # Returns: MISS

# BREAKEVEN - Instant unit economics
breakeven:
  units_required: "=BREAKEVEN_UNITS(500000, 150, 60)"     # Returns: 5,556 units
  revenue_required: "=BREAKEVEN_REVENUE(500000, 0.60)"    # Returns: $833,333
```

**Type-aware variance:** Costs use inverted logic (under budget = BEAT).

---

## Quick Start

```bash
# Install from source
cargo install --path .

# Or use the Makefile
make install-forge

# Validate a model
forge validate model.yaml

# Calculate with scenario
forge calculate model.yaml --scenario optimistic

# Export to Excel (formulas intact)
forge export model.yaml output.xlsx
```

---

## Example: 5-Year DCF Model

```yaml
_forge_version: "5.0.0"

assumptions:
  revenue_y1: 1000000
  growth_rate: 0.15
  gross_margin: 0.65
  opex_pct: 0.30
  tax_rate: 0.25
  discount_rate: 0.10

projections:
  year: [1, 2, 3, 4, 5]
  revenue: "=assumptions.revenue_y1 * (1 + assumptions.growth_rate) ^ (year - 1)"
  gross_profit: "=revenue * assumptions.gross_margin"
  opex: "=revenue * assumptions.opex_pct"
  ebit: "=gross_profit - opex"
  tax: "=MAX(0, ebit * assumptions.tax_rate)"
  net_income: "=ebit - tax"

valuation:
  total_revenue: "=SUM(projections.revenue)"
  avg_margin: "=AVERAGE(projections.gross_profit / projections.revenue)"
  npv_cash_flows: "=NPV(assumptions.discount_rate, projections.net_income)"
  irr: "=IRR(projections.net_income)"

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
forge calculate model.yaml --scenario bull
forge sensitivity model.yaml -v growth_rate -r 0.05,0.30,0.05 -o npv_cash_flows
forge export model.yaml valuation.xlsx
```

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
forge functions                     # List all 173 functions
```

---

## Who Uses Forge

| Team | Use Case |
|------|----------|
| **FP&A** | 3-statement models, budget vs actual, rolling forecasts |
| **M&A** | DCF valuations, sensitivity analysis, scenario comparison |
| **Consulting** | Client financial models with version control |
| **Fintech** | Automated projections via API, embedded calculations |
| **AI Agents** | Any coding agent (Claude, GPT, Gemini) writes YAML, Forge evaluates |

---

## Quality Assurance

| Metric | Value |
|--------|-------|
| **Tests** | 2,133 passing |
| **Functions** | 173 (167 Excel + 6 FP&A) |
| **Coverage** | 100% function coverage |
| **Warnings** | 0 (zero warnings policy) |
| **External Validation** | Gnumeric + R |

### Test Architecture

```text
forge (inline unit tests)     forge-e2e (integration/E2E)
├── 1,297 unit tests          ├── 836 E2E tests
├── #[cfg(test)] modules      ├── Gnumeric validation
└── Per-function coverage     └── R statistical validation
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [CHANGELOG](CHANGELOG.md) | Version history |
| [Architecture](docs/architecture/) | Technical design and ADRs |
| [Examples](docs/EXAMPLES.md) | YAML model examples |
| [Features](docs/FEATURES.md) | Full feature reference |
| [Market Analysis](docs/MARKET_ANALYSIS.md) | Investment thesis and positioning |
| [JSON Schema](schema/) | Model validation schema |

---

## Development

```bash
cargo test                              # Run all tests
cargo clippy -- -D warnings             # Zero warnings policy
make check                              # Full CI check
```

---

## License

**Elastic License 2.0** - See [LICENSE](LICENSE)

Forge is **Source Available** - the code is open for inspection, but commercial production use requires a license.

| Use Case | Status |
|----------|--------|
| View, read, audit source code | **Permitted** |
| Evaluation and testing | **Permitted** |
| Internal development | **Permitted** |
| Non-production use | **Permitted** |
| Commercial production use | **License required** |
| Hosted/managed service | **Not permitted** |

### What This Means

```text
┌─────────────────────────────────────────────────────────────┐
│  Source Available ≠ Open Source                            │
│                                                             │
│  • Code is OPEN (you can read it, audit it, learn from it) │
│  • Use is FREE for evaluation and non-production           │
│  • Production use in commercial settings requires license  │
│  • You cannot offer Forge as a hosted service              │
│                                                             │
│  Used by: Elasticsearch, Kibana (Elastic NV)               │
└─────────────────────────────────────────────────────────────┘
```

### Why Elastic License?

Finance needs **auditable code**. You can verify every calculation. No black boxes.

But building enterprise software requires sustainable revenue. Elastic-2.0 balances transparency with commercial viability - without ever converting to open source.

### Commercial Licensing

For production deployment or enterprise support:

**Open a GitHub Issue:** [github.com/royalbit/forge/issues](https://github.com/royalbit/forge/issues) (use `licensing` label)

See [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) for details.

---

**Built with [RoyalBit Asimov](https://github.com/royalbit/asimov)** - The AI autonomy framework.
