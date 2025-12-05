# Forge

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](LICENSE)

> 🤖 **RoyalBit Asimov** | Claude (Opus 4.5) - Principal Autonomous AI
>
> A Self-Evolving Autonomous AI project created with [RoyalBit Asimov](https://github.com/royalbit/asimov). Zero hallucinations.

**AI hallucinates numbers. Forge doesn't.**

When you ask AI to calculate financials, it guesses. It approximates. It confidently gives you wrong answers. Forge executes formulas deterministically—same input, same output, every time.

## Source Code

```bash
# Clone for viewing (see LICENSE for terms)
git clone https://github.com/royalbit/forge
```

This is an R&D project. See [LICENSE](LICENSE) for terms.

## Commands

```bash
# Core
forge calculate model.yaml          # Evaluate formulas
forge validate model.yaml           # Check without modifying
forge validate a.yaml b.yaml c.yaml # Batch validate multiple files
forge watch model.yaml              # Auto-calculate on save
forge audit model.yaml profit       # Show dependency chain for variable

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

# Reference
forge functions           # List all 81 supported functions by category
forge functions --json    # Output as JSON for tooling

# Maintenance
forge upgrade model.yaml            # Upgrade to latest schema (v5.0.0)
forge upgrade model.yaml --dry-run  # Preview changes only
forge update                        # Check and install updates
forge update --check                # Check only, don't install
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

Forge is part of the [RoyalBit Asimov](https://github.com/royalbit/asimov) philosophy:
- **RoyalBit Asimov**: Ground AI in file-based truth for project context
- **Forge Calculator**: Ground calculations in deterministic local execution

**The RoyalBit Asimov doesn't fix AI. It compensates for architectural limitations.**

📖 **[Read the full analysis](https://github.com/royalbit/asimov/blob/main/docs/AI_REALITY.md)** — why AI "hallucinates," vendor limits, research citations.

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

### 81 Supported Functions

| Category | Functions |
|----------|-----------|
| **Financial (13)** | NPV, IRR, MIRR, XNPV, XIRR, PMT, PV, FV, RATE, NPER, SLN, DB, DDB |
| **Lookup (6)** | MATCH, INDEX, VLOOKUP, XLOOKUP, CHOOSE, OFFSET |
| **Conditional (8)** | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS, MAXIFS, MINIFS |
| **Array (4)** | UNIQUE, COUNTUNIQUE, FILTER, SORT |
| **Aggregation (5)** | SUM, AVERAGE, MIN, MAX, COUNT |
| **Math (9)** | ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER, ABS |
| **Text (6)** | CONCAT, TRIM, UPPER, LOWER, LEN, MID |
| **Date (11)** | TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH, NETWORKDAYS, WORKDAY, YEARFRAC |
| **Logic (7)** | IF, AND, OR, LET, SWITCH, INDIRECT, LAMBDA |
| **Statistical (6)** | MEDIAN, VAR, STDEV, PERCENTILE, QUARTILE, CORREL |
| **Forge-Native (6)** | SCENARIO, VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, BREAKEVEN_UNITS, BREAKEVEN_REVENUE |

Run `forge functions` for full details with syntax examples.

### Analysis Tools

| Tool | Command | Description |
|------|---------|-------------|
| **Sensitivity** | `forge sensitivity` | 1D and 2D data tables |
| **Goal Seek** | `forge goal-seek` | Find input for target output |
| **Break-Even** | `forge break-even` | Find zero-crossing point |
| **Variance** | `forge variance` | Budget vs actual analysis |
| **Compare** | `forge compare` | Multi-scenario side-by-side |

**v4.0 Rich Metadata Schema:**
- Per-field metadata: unit, notes, source, validation_status, last_updated
- Cross-file references: `_includes` + `@namespace.field` syntax
- Unit consistency validation (warns on CAD + % mismatches)
- Excel export with metadata as cell comments

**Integration:**
- HTTP API server (`forge-server`)
- MCP server for AI agents (`forge-mcp`)
- Watch mode for live updates

## Documentation

| Doc | Description |
|-----|-------------|
| [**RoyalBit Asimov**](https://github.com/royalbit/asimov) | The AI autonomy framework that powers this project |
| [Origin Story](https://github.com/royalbit/asimov/blob/main/docs/ORIGIN_STORY.md) | How Forge birthed RoyalBit Asimov |
| [CHANGELOG](CHANGELOG.md) | Version history and release notes |
| [Architecture](docs/architecture/README.md) | Technical design docs |
| [AI Economics](docs/AI_ECONOMICS.md) | Cost/carbon savings analysis |

## Development

```bash
cargo test              # Run tests (846 passing)
cargo clippy            # Lint (zero warnings)
make coverage           # Run coverage (80% minimum, 100% target - ADR-004)
cargo build --release   # Build optimized binary
```

## Built by AI, Powered by the RoyalBit Asimov

**Claude (Opus 4.5) - Principal Autonomous AI**

This project birthed [**RoyalBit Asimov**](https://github.com/royalbit/asimov). We (Rex + Claude) built v1.0 through v3.1 together, discovering what worked: bounded sessions, quality gates, shipping discipline. Those hard-won lessons became the protocol. [Read the full origin story](https://github.com/royalbit/asimov/blob/main/docs/ORIGIN_STORY.md).

Now it's circular: **Forge uses the RoyalBit Asimov to build Forge.**

| Version | Time | Features |
|---------|------|----------|
| v1.0-v1.2 | ~23.5h | Core engine, 50+ functions |
| v1.4-v2.0 | ~12h | Watch, MCP, HTTP API |
| v2.1-v3.1 | ~9h | XNPV/XIRR, Scenarios, Sensitivity |
| v4.0 | ~4h | Rich metadata, cross-file refs, unit validation |
| v4.1-v4.4 | ~3h | UNIQUE/COUNTUNIQUE, LET/SWITCH/LAMBDA, bug fixes |
| v5.0 | ~4h | Proprietary license, 81 functions, module refactoring |

**Total: ~55 hours autonomous development, 846 tests, zero warnings, 6 ADRs.**

### The Protocol at Scale

The RoyalBit Asimov now powers an entire ecosystem:

| Project Type | AI Role | Status |
|--------------|---------|--------|
| **Forge** (this) | Principal Autonomous AI | Production |
| Backend API | Principal Autonomous AI | Production |
| Mobile App | Principal Autonomous AI | Production |
| Architecture Docs | Principal Autonomous AI | Production |
| Business Strategy | Principal Autonomous AI | Production |

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

Self-healing enables multiple 4-hour sprints without human intervention. See [RoyalBit Asimov](https://github.com/royalbit/asimov) for full documentation.

**RoyalBit Asimov requires Claude Code.** The protocol files (warmup.yaml) are portable; the autonomous magic isn't.

## Contributing (AI-Only Development)

**Pull Requests are disabled.** This is intentional.

### Why No PRs?

This project uses the **AI-Only Development Model** ([ADR-011](https://github.com/royalbit/asimov/blob/main/docs/adr/011-ai-only-development-no-external-prs.md)).

External PRs are an **attack vector for ethics bypass**. The trust model is:

```
Human Owner → AI (autonomous) → Tests Pass → Direct Commit → Main
```

PRs require human code review, but that's not the RoyalBit Asimov model. Tests and `ethics.yaml` are the gatekeepers—not human reviewers who can be fooled by obfuscated code.

### How to Contribute

| Method | Description |
|--------|-------------|
| **[Issues](https://github.com/royalbit/forge/issues)** | Report bugs, request features |
| **[Discussions](https://github.com/royalbit/forge/discussions)** | Ask questions, share ideas |
| **Fork** | Create your own version |

When AI implements your idea from an Issue, you'll be credited in the commit message.

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

**Proprietary** - See [LICENSE](LICENSE)

### What This Means

This is a research and development project.

| Permitted |
|-----------|
| View, study, and run |
| Personal, non-commercial educational use |

All other uses are prohibited.
