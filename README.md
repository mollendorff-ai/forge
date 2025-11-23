# 🔥 mouvify-forge

**Forge your data from YAML blueprints**

A Rust-based YAML formula calculator that transforms structured data files with embedded formulas into calculated results. Think Excel formulas, but for YAML files under version control.

## Why?

**Problem:** You have financial models, calculations, or derived metrics in YAML files (for version control and AI collaboration), but you need to manually recalculate values when assumptions change.

**Solution:** Embed formulas directly in your YAML. `mouvify-forge` evaluates them automatically, resolving dependencies and updating calculated values.

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

### Standard installation:

```bash
cargo install --path .
```

### Optimized static build (440KB binary):

For a maximally optimized, portable binary:

```bash
git clone https://github.com/royalbit/mouvify-forge
cd mouvify-forge

# Build statically-linked binary with musl
cargo build --release --target x86_64-unknown-linux-musl

# Compress with UPX (optional, reduces 1.2MB → 440KB)
upx --best --lzma target/x86_64-unknown-linux-musl/release/mouvify-forge
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

The project includes a Makefile with common build and test targets:

### Quick start:
```bash
make help                 # Show all available commands
make build-compressed     # Build optimized 440KB binary
make test                 # Validate and test all examples
```

### Build targets:
```bash
make build                # Standard release build
make build-static         # Static musl build (1.2MB)
make build-compressed     # Static + UPX compression (440KB)
```

### Test targets:
```bash
make test-validate        # Validate all test-data/*.yaml files
make test-calculate       # Dry-run calculations on test files
make test                 # Run both validation and calculation tests
```

The `test-data/` directory contains example YAML files demonstrating various formula patterns:
- `test.yaml` - Basic calculations
- `test_financial.yaml` - Financial metrics (CAC, LTV, unit economics)
- `test_platform.yaml` - Platform economics
- `test_underscore.yaml` - Variable name resolution examples

### Cargo commands:
```bash
cargo test                # Run unit tests
cargo build --release     # Build release binary
RUST_LOG=debug cargo run -- calculate file.yaml  # Debug logging
```

## License

Proprietary - Copyright © 2025 RoyalBit Inc. All rights reserved.

Built for Mouvify. Not licensed for external use.

## Why "Forge"?

A forge is where raw materials are transformed into refined tools. Similarly, `mouvify-forge` transforms raw YAML data (with formulas) into calculated, refined results. You're forging your data! 🔥
