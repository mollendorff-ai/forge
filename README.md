# 🔥 mouvify-forge

**Forge your data from YAML blueprints**

A Rust-based YAML formula calculator that transforms structured data files with embedded formulas into calculated results. Think Excel formulas, but for YAML files under version control.

## Why?

**Problem:** You have financial models, calculations, or derived metrics in YAML files (for version control and AI collaboration), but you need to manually recalculate values when assumptions change.

**Solution:** Embed formulas directly in your YAML. `mouvify-forge` evaluates them automatically, resolving dependencies and updating calculated values.

### Why YAML + Formulas for AI Workflows?

**The Token Efficiency Problem:**
When collaborating with AI assistants on financial models or complex calculations, sharing Excel files means:
- Sending screenshots (high token cost, no editability)
- Copying cell values manually (error-prone, loses formulas)
- Re-explaining the model structure every conversation

**The mouvify-forge Solution:**
YAML with embedded formulas is **token-efficient** and **AI-friendly**:
- **Compact format:** A 100-row Excel model becomes ~50 lines of YAML (~500 tokens vs 2000+ for screenshots)
- **Preserves formulas:** AI can see and reason about calculations directly
- **Version controllable:** Git-friendly plain text, not binary .xlsx
- **Human readable:** Both you and AI can read/modify without special tools
- **Self-documenting:** The structure IS the documentation

**Real-world benefit:**
Instead of wasting 2000 tokens showing Claude a screenshot of your revenue model, you share a 50-line YAML file. The AI instantly understands your assumptions, formulas, and dependencies. You save tokens for actual analysis instead of data transfer.

This tool was **built by AI, for AI-assisted workflows**. We practice what we preach.

## Features

- ✅ **Formula evaluation** - Embed `formula: "=expression"` anywhere in YAML
- ✅ **Dependency resolution** - Automatically calculates in the correct order
- ✅ **Variable lookup** - Reference other YAML values by path (e.g., `platform_take_rate`)
- ✅ **Cross-file references** - Include and reference other YAML files like Excel worksheets
- ✅ **Audit trail** - See exactly how each value was calculated
- ✅ **Type-safe** - Rust guarantees no crashes from malformed data
- ✅ **Fast** - Compiled binary, instant evaluation
- ✅ **Reusable** - Works with any YAML structure (financial models, configs, data pipelines)

## Installation

### From crates.io:

```bash
cargo install mouvify-forge
```

### From source with Makefile:

```bash
git clone https://github.com/royalbit/mouvify-forge
cd mouvify-forge

# Install system-wide (default, requires sudo)
make install

# OR install for current user only (no sudo needed)
make install-user

# Uninstall from both locations
make uninstall
```

### Manual installation:

```bash
cargo install --path .
```

### Optimized static build (440KB binary):

For a maximally optimized, portable binary:

```bash
git clone https://github.com/royalbit/mouvify-forge
cd mouvify-forge

# Build statically-linked binary with musl
make build-static

# Compress with UPX (optional, reduces 1.2MB → 440KB)
make build-compressed
```

Result: 440KB executable with zero dependencies

## Quick Start

### Input YAML (before):
```yaml
pricing:
  base_price:
    value: 100
    formula: null  # Manual input

  discount_rate:
    value: 0.10
    formula: null  # Manual input

  final_price:
    value: null  # To be calculated
    formula: "=base_price * (1 - discount_rate)"
```

### Run forge:
```bash
mouvify-forge calculate pricing.yaml --verbose
```

Output:
```
🔥 Mouvify Forge - Calculating formulas
   File: pricing.yaml

📖 Parsing YAML file...
   Found 3 variables with formulas

   pricing.final_price = =base_price * (1 - discount_rate)

🧮 Calculating formulas in dependency order...
✅ Calculation Results:
   pricing.base_price = 100
   pricing.discount_rate = 0.1
   pricing.final_price = 90

✨ File updated successfully!
```

### Output YAML (after):
```yaml
pricing:
  base_price:
    value: 100
    formula: null

  discount_rate:
    value: 0.1
    formula: null

  final_price:
    value: 90.0  # ✅ Calculated!
    formula: "=base_price * (1 - discount_rate)"
```

## Usage

### Calculate formulas in a file:
```bash
mouvify-forge calculate models/assumptions.yaml
```

### Dry-run (preview changes):
```bash
mouvify-forge calculate models/assumptions.yaml --dry-run
```

### Show audit trail for a specific variable:
```bash
mouvify-forge audit models/assumptions.yaml gross_margin
```

### Validate formulas (check for errors):
```bash
mouvify-forge validate models/assumptions.yaml
```

## Formula Syntax

Formulas are simple math expressions with variable references:

### Supported operators:
- Arithmetic: `+`, `-`, `*`, `/`, `^` (power)
- Parentheses: `(`, `)`
- Functions: `sqrt()`, `abs()`, `min()`, `max()`

### Variable references:
Reference other YAML values by their key path:

```yaml
# Simple reference
gross_margin:
  value: 0.90
  formula: "=1 - platform_take_rate"

# Nested reference with dot notation
unit_economics:
  ltv:
    value: null
    formula: "=revenue.annual / churn_rate"

# Math expressions
payback_months:
  value: null
  formula: "=cac / (revenue.monthly * gross_margin)"
```

## Cross-File References

Split your models across multiple YAML files and reference them like Excel worksheets:

### Include files with aliases:
```yaml
# main.yaml
includes:
  - file: pricing.yaml
    as: pricing
  - file: costs.yaml
    as: costs

# Reference variables from included files with @alias.variable
revenue:
  value: null
  formula: "=@pricing.base_price * volume"

margin:
  value: null
  formula: "=revenue - @costs.total_cost"
```

### Included files can have formulas too:
```yaml
# pricing.yaml
base_price:
  value: 100
  formula: null

discount:
  value: 0.15
  formula: null

final_price:
  value: null
  formula: "=base_price * (1 - discount)"
```

### Benefits:
- **Modular models** - Separate assumptions, revenue, costs into different files
- **Reusable components** - Include the same pricing model in multiple scenarios
- **Token-efficient** - Share compact YAML files instead of Excel screenshots when working with AI
- **No collisions** - Each included file has its own namespace via the `as` alias

## Use Cases

### 1. Financial Models
Keep financial calculations in version-controlled YAML:
```yaml
saas_metrics:
  arr:
    value: null
    formula: "=mrr * 12"

  ltv_cac_ratio:
    value: null
    formula: "=ltv / cac"
```

### 2. Configuration with Derived Values
Calculate configs from base values:
```yaml
server:
  max_connections:
    value: 1000
    formula: null

  thread_pool_size:
    value: null
    formula: "=max_connections / 10"
```

### 3. Data Pipelines
Define transformations declaratively:
```yaml
analytics:
  conversion_rate:
    value: null
    formula: "=purchases / visitors"

  revenue_per_visitor:
    value: null
    formula: "=total_revenue / visitors"
```

## Architecture

### Data Flow

```
Input YAML
    ↓
Parse & Extract Variables
    ↓
Build Dependency Graph
    ↓
Topological Sort (resolve order)
    ↓
Evaluate Formulas (in order)
    ↓
Update Values
    ↓
Write Output YAML
```

### Code Structure

The project is organized into clean, modular Rust modules:

```
src/
├── lib.rs              # Public library API
├── main.rs             # CLI entry point
├── cli/                # Command-line interface
│   ├── mod.rs
│   └── commands.rs     # Command handlers (calculate, validate, audit)
├── core/               # Core calculation engine
│   ├── mod.rs
│   └── calculator.rs   # Formula evaluation & dependency resolution
├── parser/             # YAML parsing with includes
│   └── mod.rs          # Variable extraction & cross-file refs
├── writer/             # YAML output generation
│   └── mod.rs          # Single-file & multi-file updates
├── error.rs            # Error types & result handling
└── types.rs            # Core data structures (Variable, ParsedYaml)
```

**Why this structure?**
- **Modularity:** Easy to add new commands or formula features
- **Testability:** Each module can be tested independently
- **Library-ready:** `lib.rs` exposes a clean API for embedding mouvify-forge in other tools
- **Maintainability:** Clear separation of concerns (parsing ≠ calculation ≠ output)

## Status

**✅ WORKING** - mouvify-forge is fully functional!

Tested with:
- ✅ Simple calculations (pricing, margins)
- ✅ Complex financial models (CAC, LTV, weighted averages)
- ✅ Deeply nested YAML structures
- ✅ Multi-level formula dependencies

## Roadmap

- [x] Basic formula evaluation
- [x] Dependency resolution
- [x] Smart variable name resolution (short names + full paths)
- [x] File writing with calculated values
- [x] Verbose output with colored results
- [x] Dry-run mode for safe testing
- [x] Circular dependency detection
- [x] Cross-file references with includes
- [ ] Audit trail generation (coming soon)
- [ ] Formula debugging mode
- [ ] Integration tests with real-world YAML
- [ ] Performance optimization for large files
- [ ] Support for arrays/lists in formulas
- [ ] Custom function definitions

## Development

The project includes a comprehensive Makefile with build, test, lint, and install targets:

### Quick start:
```bash
make help                 # Show all available commands
make build                # Build with pre/post checks (lint + tests)
make build-compressed     # Build optimized 440KB binary
make test-all             # Run all tests (40 total)
```

### Build targets:
```bash
make build                # Standard release build (with pre/post checks)
make build-static         # Static musl build (1.2MB)
make build-compressed     # Static + UPX compression (440KB)
make pre-build            # Run lint + unit tests (before build)
make post-build           # Run E2E tests (after build)
```

### Install targets:
```bash
make install              # Install to /usr/local/bin (system-wide, default)
make install-user         # Install to ~/.local/bin (user-only, no sudo)
make install-system       # Same as install (system-wide)
make uninstall            # Uninstall from both locations
```

### Lint targets (pedantic clippy):
```bash
make lint                 # Run pedantic clippy checks
make lint-fix             # Auto-fix clippy warnings
```

### Test targets:
```bash
make test                 # Run all cargo tests (40 tests)
make test-unit            # Run unit tests only
make test-integration     # Run integration tests only
make test-e2e             # Run E2E tests with actual YAML files
make test-validate        # Validate all test-data files
make test-calculate       # Dry-run calculations on test files
make test-all             # Run ALL tests (40 total)
make test-coverage        # Show test coverage summary
```

### Test Coverage

40 tests covering:
- **Unit tests (9):** Core parsing, calculation, and writing logic
- **E2E tests (25):** Full workflows with real YAML files
- **Validation tests (5):** Data integrity and stale value detection
- **Library test (1):** Public API validation

The `test-data/` directory contains example YAML files demonstrating various formula patterns:
- `test.yaml` - Basic calculations
- `test_financial.yaml` - Financial metrics (CAC, LTV, unit economics)
- `test_platform.yaml` - Platform economics
- `test_underscore.yaml` - Variable name resolution examples
- `includes_*.yaml` - Cross-file reference examples

### Cargo commands:
```bash
cargo test                # Run all 40 tests
cargo build --release     # Build release binary
RUST_LOG=debug cargo run -- calculate file.yaml  # Debug logging
cargo clippy              # Run linter
```

## License

MIT License - Copyright © 2025 RoyalBit Inc.

Yes, it's MIT. Yes, you can use it. Yes, even commercially.

**But here's the twist:**
If this tool saves you hours of Excel hell, consider:
1. ⭐ Starring the repo (helps others find it)
2. 🐛 Filing issues when you find bugs (makes it better for everyone)
3. 🎁 Contributing code (because open source runs on coffee and pull requests)

**The actual legal part:**
Permission is hereby granted, free of charge, to any person obtaining a copy of this software... (you know the drill, see [LICENSE](LICENSE) for the boring but important legal text).

**TL;DR:** Do whatever you want with it. We're not the license police. But if you save a million dollars with this tool, buying us a coffee would be cool. ☕

## Why "Forge"?

A forge is where raw materials are transformed into refined tools. Similarly, `mouvify-forge` transforms raw YAML data (with formulas) into calculated, refined results. You're forging your data! 🔥
