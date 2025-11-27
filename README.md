# Forge

> Zero hallucinations. Zero tokens. Zero emissions.

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/royalbit-forge.svg)](https://crates.io/crates/royalbit-forge)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**AI hallucinates numbers. Forge doesn't.**

When you ask AI to calculate financials, it guesses. It approximates. It confidently gives you wrong answers. Forge executes formulas deterministically—same input, same output, every time.

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

## Why Forge Exists

**AI doesn't calculate. It predicts what calculations would look like.**

This isn't a bug—it's how LLMs work. They generate the most *probable* next token, not the *correct* answer. For text, this often works. For numbers, it's dangerous.

| Ask AI to... | What Actually Happens |
|--------------|----------------------|
| Calculate NPV | Generates probable-looking formula (maybe wrong) |
| Sum a column | Predicts what a sum would look like (may skip rows) |
| Apply XIRR | Pattern-matches from training data (possibly outdated) |
| Validate a model | Says "looks correct" (no actual verification occurred) |

**Research shows:** AI hallucination rates range from 1.3% for simple tasks to 29% for specialized professional questions. Financial calculations are specialized.

**The pattern:**

```
AI inference (probabilistic) → Confident wrong answers
Local execution (deterministic) → Verifiable correct answers
```

Forge is part of the [Forge Protocol](https://github.com/royalbit/forge-protocol) philosophy:
- **Forge Protocol**: Ground AI in file-based truth for project context
- **Forge Calculator**: Ground calculations in deterministic local execution

**The Forge Protocol doesn't fix AI. It compensates for architectural limitations.**

📖 **[Read the full analysis](https://github.com/royalbit/forge-protocol/blob/main/docs/AI_REALITY.md)** — why AI "hallucinates," vendor limits, research citations.

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

**v4.0 Rich Metadata Schema:**
- Per-field metadata: unit, notes, source, validation_status, last_updated
- Cross-file references: `_includes` + `@namespace.field` syntax
- Unit consistency validation (warns on CAD + % mismatches)
- Excel export with metadata as cell comments

**Enterprise Ready:**
- 96K rows/sec throughput
- 900+ formula enterprise model validated
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
| [**Forge Protocol**](https://github.com/royalbit/forge-protocol) | The AI autonomy framework that powers this project |
| [CHANGELOG](CHANGELOG.md) | Version history and release notes |
| [Architecture](docs/architecture/README.md) | Technical design docs |
| [AI Economics](docs/AI_ECONOMICS.md) | Cost/carbon savings analysis |

## Development

```bash
cargo test              # Run tests (220 passing)
cargo clippy            # Lint (zero warnings)
cargo build --release   # Build optimized binary
```

## Built by AI, Powered by the Forge Protocol

**Claude (Opus 4.5) - Principal Engineer**

This project birthed the [**Forge Protocol**](https://github.com/royalbit/forge-protocol). We (Rex + Claude) built v1.0 through v3.1 together, discovering what worked: bounded sessions, quality gates, shipping discipline. Those hard-won lessons became the protocol.

Now it's circular: **Forge uses the Forge Protocol to build Forge.**

| Version | Time | Features |
|---------|------|----------|
| v1.0-v1.2 | ~23.5h | Core engine, 50+ functions |
| v1.4-v2.0 | ~12h | Watch, LSP, MCP, HTTP API |
| v2.1-v3.1 | ~9h | XNPV/XIRR, Scenarios, Sensitivity, Zed |
| v4.0 | ~4h | Rich metadata, cross-file refs, unit validation |

**Total: ~49 hours autonomous development, 220 tests, zero warnings, 3 ADRs.**

### The Protocol at Scale

The Forge Protocol now powers an entire ecosystem:

| Project Type | AI Role | Status |
|--------------|---------|--------|
| **Forge** (this) | Principal Engineer | Production |
| Backend API | Principal Backend Engineer | Production |
| Mobile App | Principal Engineer | Production |
| Architecture Docs | Principal AI Architect | Production |
| Business Strategy | AI Strategist | Production |

**5+ projects, 1 protocol, 1 AI.**

**The Velocity (Nov 25, 2025):** 12 releases, 64 commits, ONE DAY.

### Self-Healing Protocol (Unattended Autonomy)

The protocol includes a **self-healing mechanism** for long autonomous sessions:

```
Problem:  Auto-compact loses rules → AI "forgets" guidelines
Solution: Re-read rules from disk, not memory
```

- **CLAUDE.md**: Core rules + "re-read warmup.yaml after compaction"
- **warmup.yaml**: Full protocol with checkpoint triggers
- **.claude_checkpoint.yaml**: Session state breadcrumbs on disk

This enables 8-10hr unattended sessions. See [forge-protocol](https://github.com/royalbit/forge-protocol) for full documentation.

The protocol is vendor-agnostic: works with any AI that reads CLAUDE.md.

## License

MIT - See [LICENSE](LICENSE)
