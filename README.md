# Forge

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-1709_passing-brightgreen)](https://github.com/royalbit/forge)
[![Functions](https://img.shields.io/badge/functions-81-blue)](https://github.com/royalbit/forge)
[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](LICENSE)

**Financial modeling for the AI era. Git-native. Excel-compatible.**

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
| **Deterministic execution** | Same input = same output, every time |
| **Excel export** | Your CFO still gets `.xlsx` with working formulas |
| **Audit command** | Instant dependency trace for any variable |
| **E2E validated** | Formulas verified against Gnumeric/LibreOffice |

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

## Example Model

```yaml
_forge_version: "1.0.0"

# Inputs - the assumptions
assumptions:
  price: 100
  units_sold: [1000, 1200, 1500, 1800, 2000]
  cost_per_unit: 60
  tax_rate: 0.25

# Projections - calculated from assumptions
projections:
  revenue: "=assumptions.price * assumptions.units_sold"
  cogs: "=assumptions.cost_per_unit * assumptions.units_sold"
  gross_profit: "=projections.revenue - projections.cogs"
  tax: "=projections.gross_profit * assumptions.tax_rate"
  net_income: "=projections.gross_profit - projections.tax"

# Aggregations - summary metrics
summary:
  total_revenue: "=SUM(projections.revenue)"
  avg_margin: "=AVERAGE(projections.gross_profit / projections.revenue)"
  npv_income: "=NPV(0.10, projections.net_income)"

# Scenarios for sensitivity
scenarios:
  base:
    price: 100
  optimistic:
    price: 120
    units_sold: [1200, 1500, 1800, 2200, 2500]
  pessimistic:
    price: 85
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
