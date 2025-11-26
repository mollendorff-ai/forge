# Forge

> Zero tokens. Zero emissions. $40K-$132K/year saved.

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/royalbit-forge.svg)](https://crates.io/crates/royalbit-forge)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Green coding for financial models.** YAML uses 33% fewer tokens than Excel. Forge validates locally - no AI needed. Less compute = less CO₂.

## Performance

| Dataset | Time | Throughput |
|---------|------|------------|
| 10K rows | 107ms | 93K rows/sec |
| 100K rows | ~1s | 96K rows/sec |

## Quick Start

```bash
# Install
cargo install royalbit-forge

# Or build from source
git clone https://github.com/royalbit/forge && cd forge
cargo build --release
```

## Commands

```bash
# Core
forge calculate model.yaml          # Evaluate formulas
forge validate model.yaml           # Check without modifying
forge watch model.yaml              # Auto-calculate on save

# Analysis
forge sensitivity model.yaml -v price -r 80,120,10 -o profit
forge goal-seek model.yaml --target profit --value 100000 --vary price
forge break-even model.yaml -o profit -v price
forge variance budget.yaml actual.yaml

# Scenarios
forge calculate model.yaml --scenario optimistic
forge compare model.yaml --scenarios base,optimistic,pessimistic

# Excel
forge export model.yaml output.xlsx
forge import input.xlsx output.yaml
```

## Example Model

```yaml
_forge_version: "1.0.0"

inputs:
  price:
    value: 100
  quantity:
    value: 50
  cost_per_unit:
    value: 60

outputs:
  revenue:
    formula: "=inputs.price * inputs.quantity"
  profit:
    formula: "=outputs.revenue - (inputs.cost_per_unit * inputs.quantity)"

scenarios:
  base:
    price: 100
  optimistic:
    price: 120
```

## Features

**60+ Excel Functions:**
- Financial: NPV, IRR, XNPV, XIRR, PMT, FV, PV, RATE, NPER
- Lookup: MATCH, INDEX, XLOOKUP, VLOOKUP
- Conditional: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS
- Date: TODAY, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH
- Math, Text, Logic, Aggregation functions

**Analysis Tools:**
- Sensitivity analysis (1D and 2D data tables)
- Goal seek with bisection solver
- Break-even analysis
- Budget vs actual variance
- Multi-scenario comparison

**Enterprise Ready:**
- 96K rows/sec throughput
- HTTP API server (`forge-server`)
- MCP server for AI agents (`forge-mcp`)
- LSP server for editors (`forge-lsp`)
- Watch mode for live updates

## Editor Support

| Editor | Status | Features |
|--------|--------|----------|
| **VSCode** | `editors/vscode/` | Syntax highlighting, LSP, commands |
| **Zed** | `editors/zed/` | Native Rust/WASM, LSP, 60+ function highlighting |

Both extensions use `forge-lsp` for validation, completion, hover, and go-to-definition.

## Documentation

| Doc | Description |
|-----|-------------|
| [Forge Protocol](docs/FORGE-PROTOCOL.md) | AI autonomy framework (vendor-neutral) |
| [CHANGELOG](CHANGELOG.md) | Version history and release notes |
| [Architecture](docs/architecture/README.md) | Technical design docs |
| [AI Economics](docs/AI_ECONOMICS.md) | Cost/carbon savings analysis |

## Development

```bash
cargo test              # Run tests (183 passing)
cargo clippy            # Lint (zero warnings)
cargo build --release   # Build optimized binary
```

## Built by AI

**Claude (Opus 4.5) - Principal Engineer**

This tool was developed autonomously using the [Forge Protocol](docs/FORGE-PROTOCOL.md) - a vendor-neutral AI autonomy framework:

| Version | Time | Features |
|---------|------|----------|
| v1.0-v1.2 | ~23.5h | Core engine, 50+ functions |
| v1.4-v2.0 | ~12h | Watch, LSP, MCP, HTTP API |
| v2.1-v3.1 | ~9h | XNPV/XIRR, Scenarios, Sensitivity, Zed |

**Total: ~45 hours autonomous development, 183 tests, zero warnings, 3 ADRs.**

### Proven at Scale

Forge is FOSS - the open-source heart of a larger ecosystem:

| Project Type | AI Role | Protocol |
|--------------|---------|----------|
| FOSS CLI (this) | Principal Engineer | Forge Protocol |
| Backend API | Principal Backend Engineer | Forge Protocol |
| Mobile App | Principal Engineer | Forge Protocol |
| Architecture Docs | Principal AI Architect | Forge Protocol |
| Business Strategy | AI Strategist | Forge Protocol |

**5+ projects, 1 protocol, 1 AI.**

**The Velocity (Nov 25, 2025):** 12 releases, 64 commits, ONE DAY. Human equivalent: 3-6 months.

The protocol is vendor-agnostic: no CLAUDE.md, no lock-in. The best AI wins.

## License

MIT - See [LICENSE](LICENSE)
