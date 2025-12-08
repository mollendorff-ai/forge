# Forge

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-1709_passing-brightgreen)](https://github.com/royalbit/forge)
[![Functions](https://img.shields.io/badge/functions-81-blue)](https://github.com/royalbit/forge)
[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](LICENSE)

**Financial modeling for the AI era. Git-native. Excel import/export. FP&A functions Excel forgot.**

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

## Battle-Tested Math

**Don't trust us. Trust LibreOffice.**

Every Forge formula is E2E tested against [Gnumeric](https://www.gnumeric.org/) / LibreOffice - open-source spreadsheet engines with **200+ million users**:

```
Forge calculates → Exports XLSX → Gnumeric recalculates → Results compared
```

| Validation | Status |
|------------|--------|
| Formulas E2E validated | 60 |
| Validation engine | Gnumeric (GNOME spreadsheet) |
| Test method | Export → Recalculate → Compare |

If Gnumeric agrees with Forge, **the math is right**. No "trust me" - just proof.

## Why YAML Wins

| Factor | Excel | YAML |
|--------|-------|------|
| Token efficiency | ~100K tokens | ~2K tokens |
| AI training data | Rare | Ubiquitous (K8s, CI/CD, configs) |
| Semantic clarity | `=B7*C3` | `revenue: "=price * units"` |
| Diff-friendly | No | Yes |
| Code review | Impossible | Native |

**AI models have seen millions of YAML files.** Kubernetes configs. GitHub Actions. Docker Compose. CloudFormation. Ansible playbooks.

YAML is a first-class citizen in AI. Excel is a tourist.

### The Workflow Difference

```
Excel + AI:    Parse XML → Burn tokens → Guess cell refs → Hope it's right

Forge + AI:   Read YAML → Minimal tokens → Clear semantics → Verify with Forge
```

### Built at AI Speed

This codebase: **1 person + AI. 15 days. 1,709 tests.**

| What | Forge | Typical Project |
|------|-------|-----------------|
| Time to build | 15 days | 6+ months |
| Tests | 1,709 | Maybe 100 |
| Coverage | 89% | "Later" |
| Warnings | 0 | Ignored |

That's the point: **AI + token-efficient formats = 10x output.**

Your analysts get the same multiplier. But only if they stop feeding Excel to AI.

## The Forge Solution

| Feature | Business Value |
|---------|----------------|
| **YAML-based models** | Git-native, diff-friendly, PR-reviewable |
| **81 Excel functions** | NPV, IRR, XIRR, PMT, VLOOKUP - all the finance essentials |
| **6 FP&A-native functions** | VARIANCE, BREAKEVEN - what Excel should have had |
| **Deterministic execution** | Same input = same output, every time |
| **Excel export** | Your CFO still gets `.xlsx` with working formulas |
| **Audit command** | Instant dependency trace for any variable |
| **E2E validated** | 60 formulas verified against Gnumeric (200M+ users) |

### FP&A Functions Excel Doesn't Have

Every analyst builds these manually. Forge has them built-in:

| Forge Function | What It Does | Excel Equivalent |
|----------------|--------------|------------------|
| `VARIANCE(actual, budget)` | Budget variance | `=actual - budget` (manual) |
| `VARIANCE_PCT(actual, budget)` | Variance as % | `=(actual - budget) / budget` (manual) |
| `VARIANCE_STATUS(actual, budget)` | BEAT / MISS / ON_TARGET | Nested IF statements |
| `BREAKEVEN_UNITS(fixed, price, var_cost)` | Units to break even | Manual formula |
| `BREAKEVEN_REVENUE(fixed, margin_pct)` | Revenue to break even | Manual formula |
| `SCENARIO(name, variable)` | Get scenario value | No equivalent |

```yaml
# What takes 3 nested IFs in Excel:
status: "=VARIANCE_STATUS(actual.revenue, budget.revenue)"  # Returns: BEAT, MISS, or ON_TARGET

# What takes a manual formula in Excel:
units_needed: "=BREAKEVEN_UNITS(500000, 150, 60)"  # Returns: 5,556 units
```

## Who Uses Forge

| Team | Use Case |
|------|----------|
| **FP&A** | 3-statement models, budget vs actual, rolling forecasts |
| **M&A** | DCF valuations, sensitivity analysis, scenario comparison |
| **Consulting** | Client financial models with version control |
| **Fintech** | Automated projections via API, embedded calculations |

## Quick Start

```bash
# Install
cargo install royalbit-forge

# Or download binary from releases
curl -L https://github.com/royalbit/forge/releases/latest/download/forge-linux -o forge
chmod +x forge

# Validate a model
forge validate model.yaml

# Calculate with scenario
forge calculate model.yaml --scenario optimistic

# Export to Excel (formulas intact)
forge export model.yaml output.xlsx
```

## Example: 5-Year DCF Model

```yaml
_forge_version: "5.0.0"

# ══════════════════════════════════════════════════════════════════════════════
# ASSUMPTIONS - What-if inputs (override via scenarios)
# ══════════════════════════════════════════════════════════════════════════════
assumptions:
  revenue_y1: 1000000
  growth_rate: 0.15
  gross_margin: 0.65
  opex_pct: 0.30
  tax_rate: 0.25
  discount_rate: 0.10

# ══════════════════════════════════════════════════════════════════════════════
# PROJECTIONS - 5-year P&L (row-wise formulas applied to each period)
# ══════════════════════════════════════════════════════════════════════════════
projections:
  year: [1, 2, 3, 4, 5]
  revenue: "=assumptions.revenue_y1 * (1 + assumptions.growth_rate) ^ (year - 1)"
  gross_profit: "=revenue * assumptions.gross_margin"
  opex: "=revenue * assumptions.opex_pct"
  ebit: "=gross_profit - opex"
  tax: "=MAX(0, ebit * assumptions.tax_rate)"
  net_income: "=ebit - tax"

# ══════════════════════════════════════════════════════════════════════════════
# VALUATION - Summary metrics and DCF
# ══════════════════════════════════════════════════════════════════════════════
valuation:
  total_revenue: "=SUM(projections.revenue)"
  avg_margin: "=AVERAGE(projections.gross_profit / projections.revenue)"
  npv_cash_flows: "=NPV(assumptions.discount_rate, projections.net_income)"
  irr: "=IRR(projections.net_income)"

# ══════════════════════════════════════════════════════════════════════════════
# SCENARIOS - Override assumptions for sensitivity analysis
# ══════════════════════════════════════════════════════════════════════════════
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

Run it:
```bash
forge calculate model.yaml --scenario bull    # What-if: aggressive growth
forge sensitivity model.yaml -v growth_rate -r 0.05,0.30,0.05 -o npv_cash_flows
forge export model.yaml valuation.xlsx        # CFO gets Excel with live formulas
```

## Example: Forge-Native Functions (Not in Excel)

Forge includes 6 functions designed specifically for FP&A workflows:

```yaml
_forge_version: "5.0.0"

# ══════════════════════════════════════════════════════════════════════════════
# BUDGET vs ACTUAL ANALYSIS - Forge-native variance functions
# ══════════════════════════════════════════════════════════════════════════════
budget:
  revenue: [100000, 120000, 150000]
  expenses: [80000, 90000, 100000]

actual:
  revenue: [95000, 125000, 145000]
  expenses: [85000, 88000, 105000]

# VARIANCE - No Excel equivalent
variance_analysis:
  revenue_var: "=VARIANCE(actual.revenue, budget.revenue)"           # -5000, 5000, -5000
  revenue_var_pct: "=VARIANCE_PCT(actual.revenue, budget.revenue)"   # -5%, 4.2%, -3.3%
  revenue_status: "=VARIANCE_STATUS(actual.revenue, budget.revenue)" # MISS, BEAT, MISS

  # For costs, "under budget" is favorable
  expense_status: "=VARIANCE_STATUS(actual.expenses, budget.expenses, \"cost\")"

# ══════════════════════════════════════════════════════════════════════════════
# BREAK-EVEN ANALYSIS - Instant unit economics
# ══════════════════════════════════════════════════════════════════════════════
unit_economics:
  fixed_costs: 500000
  unit_price: 150
  variable_cost: 60
  contribution_margin_pct: 0.60

breakeven:
  units_required: "=BREAKEVEN_UNITS(unit_economics.fixed_costs, unit_economics.unit_price, unit_economics.variable_cost)"
  # Result: 5,556 units (500000 / (150 - 60))

  revenue_required: "=BREAKEVEN_REVENUE(unit_economics.fixed_costs, unit_economics.contribution_margin_pct)"
  # Result: $833,333 (500000 / 0.60)
```

## Commands

```bash
# Core Operations
forge calculate model.yaml              # Execute all formulas
forge validate model.yaml               # Check model integrity
forge audit model.yaml net_income       # Trace formula dependencies

# Analysis
forge sensitivity model.yaml -v price -r 80,120,10 -o net_income
forge goal-seek model.yaml --target net_income --value 100000 --vary price
forge break-even model.yaml -o net_income -v price
forge variance budget.yaml actual.yaml --threshold 5

# Scenarios
forge calculate model.yaml --scenario optimistic
forge compare model.yaml --scenarios base,optimistic,pessimistic

# Excel Bridge
forge export model.yaml output.xlsx    # YAML -> Excel with formulas
forge import input.xlsx output.yaml    # Excel -> YAML

# Reference
forge functions                        # List all 81 functions
```

## 81 Supported Functions

| Category | Count | Functions |
|----------|-------|-----------|
| **Financial** | 13 | NPV, IRR, MIRR, XNPV, XIRR, PMT, PV, FV, RATE, NPER, SLN, DB, DDB |
| **Date** | 11 | TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH, NETWORKDAYS, WORKDAY, YEARFRAC |
| **Conditional** | 8 | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS, MAXIFS, MINIFS |
| **Math** | 9 | ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER, ABS |
| **Logic** | 7 | IF, AND, OR, LET, SWITCH, INDIRECT, LAMBDA |
| **Lookup** | 6 | MATCH, INDEX, VLOOKUP, XLOOKUP, CHOOSE, OFFSET |
| **Statistical** | 6 | MEDIAN, VAR, STDEV, PERCENTILE, QUARTILE, CORREL |
| **Text** | 6 | CONCAT, TRIM, UPPER, LOWER, LEN, MID |
| **Forge-Native** | 6 | SCENARIO, VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, BREAKEVEN_UNITS, BREAKEVEN_REVENUE |
| **Aggregation** | 5 | SUM, AVERAGE, MIN, MAX, COUNT |
| **Array** | 4 | UNIQUE, COUNTUNIQUE, FILTER, SORT |

Run `forge functions` for full syntax and examples.

## Enterprise Features

### Audit Trail (`forge audit`)

```bash
$ forge audit model.yaml net_income

net_income
  formula: "=projections.gross_profit - projections.tax"
  dependencies:
    gross_profit
      formula: "=projections.revenue - projections.cogs"
      dependencies:
        revenue -> assumptions.price * assumptions.units_sold
        cogs -> assumptions.cost_per_unit * assumptions.units_sold
    tax
      formula: "=projections.gross_profit * assumptions.tax_rate"
```

SOX compliance: Every formula's lineage is traceable in one command.

### Variance Analysis (`forge variance`)

```bash
$ forge variance budget.yaml actual.yaml --threshold 10

Variable          Budget      Actual    Variance    Var %    Status
revenue           500,000     485,000   -15,000     -3.0%    OK
expenses          300,000     325,000   +25,000     +8.3%    OK
net_income        200,000     160,000   -40,000    -20.0%    ALERT
```

### API Server (`forge serve`)

```bash
# Start REST API
forge serve --port 8080

# POST /calculate
curl -X POST http://localhost:8080/calculate \
  -H "Content-Type: application/json" \
  -d '{"path": "model.yaml", "scenario": "optimistic"}'
```

### MCP Integration (`forge mcp`)

Integrate with Claude Desktop and other AI tools via Model Context Protocol.

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

## Quality Assurance

| Metric | Value |
|--------|-------|
| **Tests** | 1709 passing |
| **Functions** | 81 in evaluator |
| **E2E Validated** | 44 formulas against Gnumeric |
| **Warnings** | 0 |
| **Coverage** | 89.23% |

### E2E Validation

Unlike unit tests that verify code, Forge's E2E tests verify **formulas against real spreadsheet engines**:

```
Forge YAML -> Export to XLSX -> Gnumeric recalculates -> Compare results
```

This means: If Gnumeric (battle-proven, millions of users) agrees with Forge, the math is right.

## ROI Calculator

**For a 10-analyst FP&A team:**

| Current State | With Forge |
|---------------|------------|
| 40% time on Excel maintenance | Automated via YAML |
| Manual version control | Git branching |
| No formula review process | PR-based review |
| Audit prep: 2 weeks | `forge audit`: 2 seconds |

**Conservative estimate: 10 hours/analyst/week saved = $150K/year**

## Integration Paths

| Method | Use Case |
|--------|----------|
| **CLI** | Batch processing, CI/CD pipelines |
| **REST API** | Web applications, microservices |
| **MCP Server** | AI agent integration |
| **Library** | Rust/WASM embedding |

## Documentation

| Document | Description |
|----------|-------------|
| [CHANGELOG](CHANGELOG.md) | Version history |
| [Architecture](docs/architecture/) | Technical design |
| [AI Economics](docs/AI_ECONOMICS.md) | Cost savings analysis |
| [JSON Schema](schema/) | Model validation schema |

## Development

```bash
cargo test                              # 1709 tests
cargo clippy -- -D warnings             # Zero warnings policy
cargo test --features e2e-libreoffice   # E2E validation (requires Gnumeric)
```

## License

**Proprietary R&D** - See [LICENSE](LICENSE)

This is a **research and development project**. Not available for commercial use.

| Use Case | Status |
|----------|--------|
| View & study source code | Permitted |
| Personal educational use | Permitted |
| Commercial use | **Prohibited** |
| Enterprise deployment | **Prohibited** |
| Distribution/modification | **Prohibited** |

**This is not a product. This is R&D.**

You can see what's possible. You can't have it. Yet.

---

**Built with [RoyalBit Asimov](https://github.com/royalbit/asimov)** - The AI autonomy framework.
