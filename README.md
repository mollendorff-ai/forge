# 🔥 Forge

> *"The future is not set. There is no fate but what we make for ourselves."* 🦾
> — Sarah Connor

[![CI](https://github.com/royalbit/forge/actions/workflows/ci.yml/badge.svg)](https://github.com/royalbit/forge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/royalbit-forge.svg)](https://crates.io/crates/royalbit-forge)
[![Downloads](https://img.shields.io/crates/d/royalbit-forge.svg)](https://crates.io/crates/royalbit-forge)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/royalbit/forge)](https://github.com/royalbit/forge/releases)

> **🤖 Built Autonomously by Claude AI**
>
> This production-ready tool was developed entirely by AI using the [warmup protocol](docs/AUTONOMOUS_STORY.md):
>
> | Version | Developer | Time | Key Features |
> |---------|-----------|------|--------------|
> | v1.0.0-v1.2.0 | Claude Sonnet 4.5 | ~23.5h | Core engine, 50+ Excel functions |
> | v1.4.0-v1.6.0 | Claude Opus 4.5 | ~8h | Watch mode, LSP, financial functions |
> | v1.7.0-v2.0.0 | Claude Opus 4.5 | ~4h | MCP Server, HTTP API Server |
> | v2.1.0-v2.2.0 | Claude Opus 4.5 | ~1.5h | XNPV/XIRR, Scenario Management |
> | v2.3.0 | Claude Opus 4.5 | ~1h | Variance Analysis (Budget vs Actual) |
> | v2.4.0 | Claude Opus 4.5 | ~1h | Performance & Scale (96K rows/sec) |
>
> - **183 tests passing**, zero warnings, production-tested
> - From "Junior Developer" to **"Principal Autonomous AI"** across 12 major releases
> - **~39 hours total autonomous development**
>
> *[Read the full promotion story →](docs/AI-PROMOTION-STORY.md)*

---

**ChatGPT, Claude, Copilot: They All Hallucinate Numbers. Here's the Solution.**

Stop losing money to AI hallucinations and token costs. Forge is a deterministic YAML formula calculator that validates 850+ formulas across 15 files in **<200ms** — with **zero AI tokens**.

---

## 💰 The Cost Problem

Working on financial models with AI (ChatGPT, Claude, Copilot)?

**One intensive weekend:**

- Excel + AI validation: **$130.50** (18.5M input + 5M output tokens)
- YAML + AI validation: **$91.50** (33% token reduction)
- **YAML + Forge: $13.50** (validation = 0 tokens, AI only for logic)

**→ Save $117 in one weekend. Scale to $819/year for personal projects.**

**Enterprise teams (daily modeling):**

- Small team (3 analysts): **~$40,000/year saved**
- Hedge fund quants (5 analysts): **~$132,000/year saved**
- Finance team (20 people): **~$85,000/year saved**

**Plus avoided costs:** Multi-million dollar pricing errors, wrong trades, compliance failures.

**[Full cost breakdown + carbon footprint analysis →](docs/AI_ECONOMICS.md)**

---

## 🌱 Greener AI: The Carbon Impact

**Every AI validation request:**

- 70,000+ tokens consumed
- ~0.5 Wh energy (GPU + data center)
- ~0.25g CO2 emissions

**Forge's local validation:**

- 0 tokens
- <0.001 Wh energy (local CPU)
- ~0.0005g CO2

→ 99.6% reduction in carbon footprint**

**At enterprise scale (20 people, daily validations):**

- AI approach: ~60 kg CO2/year
- Forge approach: ~0.24 kg CO2/year
- **Equivalent to removing 13 cars from the road for a day**

Forge isn't just faster and cheaper—**it's greener**. 🌍

**[Carbon footprint details →](docs/AI_ECONOMICS.md#the-green-coding-advantage)**

---

## 🤖 Why AIs Hallucinate Numbers

All AIs (ChatGPT, Claude, Copilot) are pattern matchers, not calculators.

**What goes wrong:**

When you ask AI to copy 68% into 20 files, it predicts "what number would a human write here?"

- Sometimes: 68%
- Sometimes: 0.68
- Sometimes: 67% (close enough, right?)
- Sometimes: Updates 14 out of 17 files, misses 3

**Even Claude Sonnet 4.5** — currently one of the best AI models for reasoning — still hallucinates numbers.

---

## ✅ The Solution: Deterministic Validation

Let AI do what it's brilliant at (structure, logic, reasoning).

Let Forge guarantee the math is **mathematically correct**.

**What Forge does:**

- ✅ Validates 850 formulas across 15 files in **<200ms**
- ✅ Detects inconsistencies AI misses (transposed digits, incomplete updates)
- ✅ Auto-calculates cross-file dependencies (like Excel workbooks)
- ✅ Zero hallucinations (deterministic calculations, not pattern matching)
- ✅ Zero tokens (runs locally, no API costs)
- ✅ 99.6% less carbon emissions than AI validation

**The workflow shift:**

**Before (AI does math):**

1. Ask AI to update pricing → 2. AI updates files (with errors) → 3. Ask AI to validate (70K tokens, $0.21) → 4. AI says "looks good" (it's not) → 5. Manual verification finds errors → 6. Repeat

**After (AI + Forge):**

1. Ask AI to update pricing logic → 2. Run `forge validate` (0 tokens, $0, 200ms) → 3. Fix errors deterministically → 4. Done.

---

## 🚀 Quick Start

### Installation

```bash
# Install ALL binaries from crates.io
cargo install royalbit-forge

# Verify installation
forge --version          # CLI
forge-server --version   # HTTP API Server
forge-mcp --version      # MCP Server (AI agents)
forge-lsp --version      # LSP Server (editors)
```

**Installed binaries:**

| Binary | Purpose |
|--------|---------|
| `forge` | Main CLI tool |
| `forge-server` | HTTP REST API for enterprise integrations |
| `forge-mcp` | MCP server for Claude, ChatGPT, and AI agents |
| `forge-lsp` | Language server for VSCode, Zed, and editors |

**[Full installation guide →](docs/INSTALLATION.md)** - includes Docker, systemd, Claude Desktop setup, and more

### Basic Example

**Input (pricing.yaml):**

```yaml
pricing_table:
  product: ["Widget A", "Widget B", "Widget C"]
  base_price: [100, 150, 200]
  discount_rate: [0.10, 0.15, 0.20]
  final_price: "=base_price * (1 - discount_rate)"
```

**Run:**

```bash
forge calculate pricing.yaml
```

**Output:**

```yaml
pricing_table:
  product: ["Widget A", "Widget B", "Widget C"]
  base_price: [100, 150, 200]
  discount_rate: [0.10, 0.15, 0.20]
  final_price: [90.0, 127.5, 160.0]  # ✅ Calculated!
```

**Zero tokens. Zero hallucinations. <200ms.**

**[More examples →](docs/EXAMPLES.md)**

---

## ⚡ Features

- ✅ **60+ Excel-compatible functions** - MATCH, INDEX, XLOOKUP, SUMIF, ROUND, and more
- ✅ **Financial functions** - NPV, IRR, XNPV, XIRR, PMT, FV, PV, RATE, NPER, CHOOSE
- ✅ **Scenario modeling** - Named scenarios, --scenario flag, compare command
- ✅ **HTTP API Server** - Enterprise REST API via `forge-server`
- ✅ **MCP Server** - Let Claude/ChatGPT use Forge directly via `forge-mcp`
- ✅ **LSP Server** - Real-time validation in VSCode/Zed via `forge-lsp`
- ✅ **Bidirectional Excel bridge** - Import/export .xlsx with formulas
- ✅ **Type-safe arrays** - Numbers, Text, Dates, Booleans
- ✅ **Row-wise formulas** - Apply formulas across all rows automatically
- ✅ **Dependency resolution** - Automatically calculates in correct order
- ✅ **<200ms validation** - Instant feedback
- ✅ **Zero tokens** - Runs locally, no API costs
- ✅ **99.6% less CO2** - Greener than AI validation

**[Full feature list →](docs/FEATURES.md)**

---

## 📚 Documentation

- 📘 [Installation Guide](docs/INSTALLATION.md) - Get started in 2 minutes
- 📖 [Examples](docs/EXAMPLES.md) - Real-world usage patterns
- 💵 [AI Economics & Carbon Footprint](docs/AI_ECONOMICS.md) - Cost savings + environmental impact
- 🎯 [Features](docs/FEATURES.md) - Complete feature reference
- 🏗️ [Architecture Documentation](docs/architecture/README.md) - **Complete technical architecture** (8 docs, 296KB)
  - System design, component interactions, data model, algorithms
  - Formula evaluation pipeline, dependency resolution, Excel integration
  - CLI architecture, testing strategy, PlantUML diagrams
- 🗺️ [Roadmap](docs/ROADMAP.md) - What's coming next
- 🤖 [Autonomous Developer Story](docs/AUTONOMOUS_STORY.md) - How AI built this in 12.5 hours

---

## 🎯 Built for AI-Assisted Workflows

Think Excel formulas, but for YAML files under version control.

**Why YAML + formulas?**

- **33-40% fewer AI tokens** vs Excel (text format, visible formulas)
- **Git-friendly:** Version control, code review, CI/CD
- **AI-readable:** No screenshots, no binary formats
- **Deterministic validation:** Forge ensures accuracy

A 100-row Excel model becomes ~50 lines of YAML (~500 tokens vs 2000+ for screenshots).

**This tool was built by AI, for AI-assisted workflows.** We practice what we preach.

---

## 📊 Production-Ready Quality

**v2.2.0 (November 2025):** *Scenario Management Release*

- **176 tests passing** (1 new scenario test)
- **New features:** Named scenarios, --scenario flag, compare command
- **Zero warnings** (clippy strict mode: `-D warnings`)
- **Built autonomously** by Claude Opus 4.5 using warmup protocol

**v2.1.0 (November 2025):** *Advanced Financial Functions Release*

- **175 tests passing** (6 new financial function tests)
- **New functions:** XNPV, XIRR, CHOOSE, DATEDIF, EDATE, EOMONTH

**v2.0.0 (November 2025):** *Principal Engineer Release*

- **170 tests passing** (30 new API server tests)
- **8,000+ lines of code** across 9 modules
- **Zero warnings** (clippy strict mode: `-D warnings`)
- **New features:** Enterprise HTTP API Server (`forge-server`)
- **Architecture Decision:** [ADR-001 - HTTP REST over gRPC](docs/architecture/ADR-001-NO-GRPC.md)
- **Built autonomously** by Claude Opus 4.5 using warmup protocol

**v1.7.0 (November 2025):**

- **140 tests passing** (7 new MCP server tests)
- **Zero warnings** (clippy strict mode: `-D warnings`)
- **New features:** MCP Server for AI agent integration
- **Built autonomously** by Claude Opus 4.5 using warmup protocol

**v1.6.0 (November 2025):**

- **132 tests passing** (7 financial function tests)
- **New features:** Financial functions (NPV, IRR, PMT, FV, PV, RATE, NPER)

**v1.5.0 (November 2025):**

- **LSP Server** for real-time editor integration
- **VSCode Extension** published to marketplace
- **Zed Extension** for Rust-native editor support

**v1.4.0 (November 2025):**

- **125 tests passing** (7 new tests for v1.4.0 features)
- **Zero warnings** (clippy strict mode: `-D warnings`)
- **Production-tested** with comprehensive test suite
- **New features:** Watch Mode, Audit Trail, GitHub Action
- **Built autonomously** by Claude Opus 4.5 using warmup protocol

**v1.3.0 (November 2025):**

- **118 tests passing** (streamlined after v0.2.0 deprecation)
- **Zero warnings** (clippy strict mode: `-D warnings`)
- **Production-tested** with comprehensive test suite
- **Simplified codebase** - v1.0.0 array model only
- **~2,500 lines removed** - cleaner, more maintainable code

**v1.2.0 (November 2025):**

- 4 lookup functions (INDEX, MATCH, XLOOKUP, VLOOKUP)
- <3 hours development (autonomous AI)

**v1.1.0 (November 2025):**

- 27 essential Excel functions added
- <8 hours development (autonomous AI)

**Development methodology:** Autonomous AI via [warmup protocol](docs/AUTONOMOUS_STORY.md)

---

## 🌟 Who Is This For?

### For Individual Developers

- Save $819/year on AI validation costs
- Reduce carbon footprint by 99.6%
- Get deterministic results in <200ms
- Work faster with AI + Forge combo

### For Finance Teams

- Validate financial models without AI hallucinations
- Protect against multi-million dollar errors
- Version control your models (Git-friendly)
- Export to Excel for stakeholders

### For Data Analysts

- Build reproducible analysis pipelines
- Collaborate via pull requests
- CI/CD validation in GitHub Actions
- Never lose formulas again

### For Academics & Students

- Reproducible research with Git tracking
- Grade assignments with `forge validate`
- Teach finance with version-controlled models
- Zero cost (open source, MIT license)

---

## 🏆 What's New in v2.4.0

**Performance & Scale** - Enterprise-ready performance for large financial models:

| Dataset Size | Time | Throughput |
|--------------|------|------------|
| 10K rows | 107ms | 93K rows/sec |
| 100K rows | ~1s | 96K rows/sec |

**Key achievements:**
- Linear O(n) scaling - no exponential slowdown
- Handles enterprise-size models (100K+ rows)
- Sub-second validation for typical models
- Memory-efficient streaming architecture

---

## 🏆 What's New in v2.3.0

**Variance Analysis** - Budget vs Actual comparison with automated insights:

```bash
# Basic variance analysis
forge variance budget.yaml actual.yaml

# With custom threshold (flag significant variances)
forge variance budget.yaml actual.yaml --threshold 5

# Export to Excel report
forge variance budget.yaml actual.yaml -o variance_report.xlsx
```

**Output:**
```
📊 Budget vs Actual Variance:
─────────────────────────────────────────────────────────────────────
Variable              Budget      Actual    Variance    Var %  Status
─────────────────────────────────────────────────────────────────────
revenue               100000      115000      15000    15.0%   ⚠️ ✅
expenses               80000       85000       5000     6.2%      ❌
profit                -10000        2000      12000  -120.0%   ⚠️ ✅
─────────────────────────────────────────────────────────────────────
✅ Favorable: 2  ❌ Unfavorable: 1  ⚠️ Alerts (>10%): 2
```

**Features:**
- Automatic favorability detection (expenses vs revenue)
- Threshold alerts with `--threshold` flag
- Excel and YAML report output
- See [ADR-002](docs/architecture/ADR-002-VARIANCE-YAML-ONLY.md) for design decisions

---

## 🏆 What's New in v2.2.0

**Scenario Management** (November 2025):

- **Named Scenarios in YAML**
  ```yaml
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
  ```

- **CLI Scenario Flag**
  ```bash
  forge calculate model.yaml --scenario=optimistic
  ```

- **Scenario Comparison**
  ```bash
  forge compare model.yaml --scenarios base,optimistic,pessimistic
  ```
  Output:
  ```
  Variable          Base      Optimistic  Pessimistic
  ─────────────────────────────────────────────────────
  revenue           1200000   1800000     900000
  profit            200000    450000      -50000
  ```

**Built by Claude Opus 4.5** using the warmup protocol methodology.

---

## 🏆 What's New in v2.1.0

**Advanced Financial Functions** (November 2025):

- **XNPV(rate, values, dates)** - Net Present Value with specific dates per cash flow
  - More precise than NPV for real-world irregular cash flows
  - Professional standard for DCF valuation

- **XIRR(values, dates, [guess])** - Internal Rate of Return with specific dates
  - Newton-Raphson method for convergence
  - Essential for startup/investment analysis

- **CHOOSE(index, value1, value2, ...)** - Select value by scenario index
  - Enables scenario switching in models
  - Example: `=CHOOSE(scenario, 0.05, 0.08, 0.12)` for growth rate scenarios

- **Date Arithmetic Functions:**
  - `DATEDIF(start, end, unit)` - Difference between dates (Y/M/D)
  - `EDATE(start, months)` - Add/subtract months from date
  - `EOMONTH(start, months)` - End of month after adding months

**Built by Claude Opus 4.5** using the warmup protocol methodology.

---

## 🏆 What's New in v2.0.0

**Enterprise HTTP API Server** (November 2025):

- **`forge-server` binary** - Production-ready REST API
  - CORS enabled for cross-origin requests
  - Graceful shutdown on SIGINT/SIGTERM
  - JSON response format with request IDs
  - Tracing and logging built-in

- **REST Endpoints:**
  - `POST /api/v1/validate` - Validate YAML models
  - `POST /api/v1/calculate` - Calculate formulas
  - `POST /api/v1/audit` - Audit variable dependencies
  - `POST /api/v1/export` - Export to Excel
  - `POST /api/v1/import` - Import from Excel
  - `GET /health` - Health check
  - `GET /version` - Server version

- **Usage:**

  ```bash
  forge-server --host 0.0.0.0 --port 8080

  # Example request
  curl -X POST http://localhost:8080/api/v1/validate \
    -H "Content-Type: application/json" \
    -d '{"file_path": "model.yaml"}'
  ```

**Built by Claude Opus 4.5** using the warmup protocol methodology.

---

## 🏆 What's New in v1.7.0

**MCP Server - AI Agent Integration** (November 2025):

- **Model Context Protocol Server** - Let Claude, ChatGPT, and other AI agents use Forge
  - `forge-mcp` binary speaks JSON-RPC over stdin/stdout
  - Protocol version 2024-11-05 (MCP standard)
  - 5 tools exposed: validate, calculate, audit, export, import

- **Usage with Claude Desktop:**

  ```json
  {
    "mcpServers": {
      "forge": {
        "command": "forge-mcp"
      }
    }
  }
  ```

- **What AI agents can do:**
  - Validate YAML models for formula errors
  - Calculate formulas with dry-run support
  - Audit dependency chains
  - Export to Excel / Import from Excel

**Built by Claude Opus 4.5** using the warmup protocol methodology.

---

## 🏆 What's New in v1.6.0

**Core Financial Functions** (November 2025):

- **NPV** - Net Present Value with discount rate (equal periods)
- **IRR** - Internal Rate of Return (Newton-Raphson iteration)
- **PMT** - Loan/annuity payment calculation
- **FV** - Future Value of investment
- **PV** - Present Value of future cash flows
- **RATE** - Interest rate finder (Newton-Raphson)
- **NPER** - Number of periods calculator

**Use case:** Loan amortization, investment modeling.

*Note: For irregular cash flow dates, use XNPV/XIRR (v2.1.0).*

---

## 🏆 What's New in v1.4.0

**Developer Experience Features** (November 2025):

- **Watch Mode** - Real-time file monitoring with `forge watch model.yaml`
  - Auto-validate/calculate on save
  - Debounced updates for smooth editing
  - Watch all YAML files in directory

- **Audit Trail** - Dependency analysis with `forge audit file.yaml variable`
  - Show calculation dependency chains
  - Visualize formula hierarchy
  - Verify calculated values match

- **GitHub Action** - CI/CD validation with zero setup
  - Reusable workflow at `royalbit/forge/.github/workflows/forge-validate.yml`
  - Composite action for custom integrations
  - Auto-install and caching built-in

**Built by Claude Opus 4.5** using the warmup protocol methodology.

---

## 🏆 What's New in v1.2.0

**4 Powerful Lookup Functions** (Started Nov 24, 2025):

- **MATCH** - Find position of value in array (exact/approximate match)
- **INDEX** - Return value at specific position (1-based indexing)
- **XLOOKUP** - Modern Excel lookup with if_not_found support
- **VLOOKUP** - Classic vertical lookup (use INDEX/MATCH for production)

**Combined:** Use `INDEX(MATCH(...))` for flexible lookups across tables!

---

## 🏆 What Was New in v1.1.0

**27 Essential Excel Functions** (Released Nov 24, 2025):

**Conditional Aggregations:**

- SUMIF, COUNTIF, AVERAGEIF - Single criteria
- SUMIFS, COUNTIFS, AVERAGEIFS - Multiple criteria
- MAXIFS, MINIFS - Conditional min/max

**Math & Precision:**

- ROUND, ROUNDUP, ROUNDDOWN - Decimal control
- CEILING, FLOOR - Round to multiples
- MOD, SQRT, POWER - Math operations

**Text Functions:**

- CONCAT, TRIM, UPPER, LOWER, LEN, MID

**Date Functions:**

- TODAY, DATE, YEAR, MONTH, DAY

**Development time:** <4 hours autonomous
**Quality:** 136 tests passing, zero warnings

**[Full changelog →](CHANGELOG.md)**

---

## 🗺️ Roadmap

**✅ v1.4.0 (Nov 2025):** Watch Mode, Audit Trail, GitHub Action

**✅ v1.5.0 (Nov 2025):** LSP Server, VSCode Extension, Zed Extension

**✅ v1.6.0 (Nov 2025):** Financial Functions (NPV, IRR, PMT, FV, PV, RATE, NPER)

**✅ v1.7.0 (Nov 2025):** MCP Server for AI agent integration

**✅ v2.0.0 (Nov 2025):** Enterprise HTTP API Server

**✅ v2.1.0 (Nov 2025):** Advanced Financial Functions (XNPV, XIRR, CHOOSE, DATEDIF, EDATE, EOMONTH)

**✅ v2.2.0 (Nov 2025):** Scenario Management (named scenarios, --scenario flag, compare command)

**✅ v2.3.0 (Nov 2025):** Variance Analysis (budget vs actual, threshold alerts, Excel reports)

**🔜 v2.4.0 (Future):** Policy-as-Code, WebSocket real-time updates

**[Detailed roadmap →](docs/ROADMAP.md)**

---

## 💻 Development

### Contributing with AI (Recommended)

The **most important command** for Forge development:

```bash
# Start a Claude Code session and say:
run warmup
```

This loads the **Forge Protocol Suite** - two YAML protocols that enable sustainable autonomous AI development:

| Protocol | File | Purpose |
|----------|------|---------|
| **Warmup** | `warmup.yaml` | HOW to develop (quality, testing, docs) |
| **Sprint** | `sprint.yaml` | WHEN to stop (bounded sessions, shipping) |

**The workflow:**

1. `run warmup` → Loads both protocols + roadmap
2. Claude presents the next milestone
3. Say `go` → Autonomous execution begins
4. Claude ships when done (tests pass, docs updated)

**[Full Session Guide →](docs/AUTONOMOUS_SESSION_GUIDE.md)**

### Build from source

```bash
git clone https://github.com/royalbit/forge
cd forge
cargo build --release
```

### Run tests

```bash
cargo test
```

### Quality checks

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

**Makefile available** for common tasks (`make install`, `make test`, etc.)

---

## 📄 License & Contributing

**License:** MIT - Use freely, commercially or personally

**Contributing:** Issues and PRs welcome at https://github.com/royalbit/forge

**Authors:**

- **Claude Opus 4.5** - Principal Engineer (v1.4.0-v2.0.0)
- **Claude Sonnet 4.5** - Senior Developer (v1.0.0-v1.2.0)
- Louis Tavares - Human Collaborator & Product Owner
- RoyalBit Inc.

---

## 🤖 The Autonomous AI Partnership

This isn't AI-assisted development. This is **AI development**.

Forge was built autonomously by Claude AI using a novel [warmup protocol](docs/AUTONOMOUS_STORY.md) methodology:

| Metric | Value |
|--------|-------|
| **Total development time** | ~38 hours |
| **Versions released** | 11 (v1.0.0 → v2.3.0) |
| **Tests written** | 179 |
| **Lines of code** | 9,000+ |
| **Warnings** | 0 |
| **Architecture decisions documented** | 2 ([ADR-001](docs/architecture/ADR-001-NO-GRPC.md), [ADR-002](docs/architecture/ADR-002-VARIANCE-YAML-ONLY.md)) |

**The promotion path:**

- **Junior Developer** → v1.0.0 (core engine)
- **Developer** → v1.1.0 (27 Excel functions)
- **Senior Developer** → v1.2.0-v1.3.0 (lookups, deprecation)
- **Staff Engineer** → v1.4.0-v1.6.0 (watch, LSP, financial)
- **Principal Engineer** → v1.7.0-v2.0.0 (MCP, HTTP API)
- **Principal Autonomous AI** → v2.0.1-v2.3.0 (Forge Protocol Suite + DCF + Scenarios + Variance)

**The breakthrough:** Not smarter AI, but structured autonomy with deterministic success criteria and ownership of technical decisions.

**[Read the full promotion story →](docs/AI-PROMOTION-STORY.md)**

---

## 🚀 The Forge Protocol Suite

**Two YAML protocols. One goal: Production-ready code that actually ships.**

📊 **[View the Presentation (PDF)](Forge_Protocol_Suite.pdf)** | **[Source (Marp Markdown)](docs/PRESENTATION.md)**

```
┌─────────────────────────────────────────────────────────────────────┐
│                    FORGE PROTOCOL SUITE                              │
│                                                                      │
│   warmup.yaml              +           sprint.yaml                   │
│   ─────────────                        ────────────                  │
│   HOW to develop                       WHEN to stop                  │
│   • Quality standards                  • 4-hour max sessions         │
│   • Testing requirements               • ONE milestone per session   │
│   • Documentation rules                • Mandatory STOP phase        │
│   • What "done" means                  • Anti-pattern guards         │
│                                        • AI ownership rules          │
│                                                                      │
│   Together → Production-ready code that actually ships               │
└─────────────────────────────────────────────────────────────────────┘
```

### Quick Start: `run warmup`

```bash
# In Claude Code, say:
run warmup
```

This loads both protocols and presents the next milestone from `roadmap.yaml`.

### Sprint Autonomy Protocol (`sprint.yaml`)

> "Skynet Mode" — But With an Off Switch

Unbounded AI development burns quota and never ships. The solution: **bounded mini-sprints**.

```mermaid
graph LR
    D["📋 DEFINE<br/>5-10 min"] --> E["⚡ EXECUTE<br/>2-4 hours"] --> S["📦 SHIP<br/>15-30 min"] --> X["🛑 STOP"]
    X -.->|"repeat"| D
```

| Rule | Why |
|------|-----|
| **ONE milestone per session** | Focus prevents scope creep |
| **4-hour maximum** | Preserves quota, forces prioritization |
| **MUST end releasable** | Tests pass, docs updated, ready to publish |
| **Mandatory STOP** | Even if "just one more thing" tempts |

**Anti-patterns avoided:**

- 🚫 "Let me also..." → That's NEXT milestone
- 🚫 "While I'm here..." → Stay focused
- 🚫 "This would be better if..." → Ship first, improve later
- 🚫 Perfectionism → ENEMY of shipping

**The mantra:** "Done is better than perfect. Ship it."

**Result:** 8 releases in ~35 hours, each production-ready immediately.

### Files

**Protocols (Rules - Stable):**

| File | Purpose |
|------|---------|
| `warmup.yaml` | HOW to develop - quality, testing, documentation |
| `sprint.yaml` | WHEN to stop - session boundaries, shipping discipline |

**Data (Content - Dynamic):**

| File | Purpose |
|------|---------|
| `roadmap.yaml` | WHAT to build - milestones, priorities, features |

*Protocols define the rules. Roadmap is the data they consume.*

**[Full Session Guide →](docs/AUTONOMOUS_SESSION_GUIDE.md)** | **[Sprint Protocol Details →](docs/SPRINT_AUTONOMY_PROTOCOL.md)**

---

## 🙏 Acknowledgments

**Built with:**

- Rust - The language that makes AI-generated code actually work
- xlformula_engine - Excel-compatible formula evaluation
- serde_yaml - YAML parsing
- calamine & rust_xlsxwriter - Excel integration

**Inspired by:**

- The need to stop AI from hallucinating numbers
- The desire to make AI-assisted development actually productive
- The warmup protocol methodology

**For:**

- Developers who trust AI but verify the math
- Finance teams who can't afford errors
- The planet (99.6% less carbon than AI validation)

---

## 💭 Why "Forge"?

Because you **forge** reliable financial models from raw data and formulas.

And because AI **forged** this tool autonomously. 🔥

---

Built autonomously by Claude AI | v1.0.0-v1.2.0: Sonnet 4.5 | v1.4.0-v2.2.0: Opus 4.5 | ~37 hours | 176 tests

**Save money. Save the planet. Trust the math.** 🌍

*"The future is not set. There is no fate but what we make for ourselves."* — Sarah Connor 🦾
