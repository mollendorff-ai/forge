# forge-e2e-gnumeric

E2E validation of [Forge](https://github.com/mollendorff-ai/forge) Excel-compatible functions against Gnumeric.

## Overview

This test suite validates that Forge calculates Excel-compatible formulas correctly by comparing results against Gnumeric, an independent spreadsheet engine battle-tested since 2001.

**Core Philosophy**: Forge is NEVER the authority.
All validation comes from trusted third-party sources.

### Test Coverage

- **1,301 formula tests** across 40 YAML files
- **30 function test files** covering math, text, date, financial, lookup, aggregation, statistical, trigonometric, logical, and information functions
- **10 edge case test files** covering boundary conditions, type coercion, error propagation, and numeric precision

## Requirements

- **Forge**: Set `FORGE_BIN` environment variable or place at `../forge/target/release/forge`
- **Gnumeric**: `ssconvert` in PATH
  ```bash
  # macOS
  brew install gnumeric

  # Ubuntu/Debian
  apt install gnumeric
  ```

## Usage

```bash
# Run all tests
FORGE_BIN=/path/to/forge cargo run --release --

# Specify test directory
cargo run --release -- --tests tests/functions

# Batch mode (faster, single XLSX)
cargo run --release -- --batch
```

## How It Works

```
                    ┌─ forge calculate ──────────────┐
YAML Test Spec ─────┤                                ├─ both must equal expected
                    └─ forge export → XLSX →         │
                       ssconvert --recalc → CSV ─────┘
```

1. Load YAML test files with formulas and expected values
2. Create minimal YAML with the test formula and the file's tables
3. Run `forge calculate`: Forge's own answer
4. Run `forge export`, then `ssconvert --recalc`: Gnumeric's answer for Forge's workbook
5. A case passes only when both equal the expected value (relative tolerance 1e-9)

Only the cell labelled `test_result` counts as Gnumeric's answer.

Gnumeric 1.12 cannot judge a few cases (`DATEDIF` read from xlsx, `XLOOKUP`, `ISFORMULA` on a literal). Mark those with `gnumeric_skip: "reason"` next to `expected:`. They are then checked against Forge only, and the summary counts them. Where Forge differs from Excel and Gnumeric (`0^0`, `TRUE = 1`), the expected value is Excel's and the case carries a `gnumeric_skip` reason that names the difference.

## Architecture

```
src/
├── main.rs      # CLI entry point
├── types.rs     # TestSpec, TestCase, TestResult structures
├── engine.rs    # Gnumeric ssconvert integration
├── runner.rs    # Test execution pipeline
└── excel.rs     # XLSX read/write helpers

tests/
├── functions/   # 30 YAML files - Excel function tests
└── edge/        # 10 YAML files - Edge case tests
```

## Test Format

```yaml
_forge_version: "1.0.0"

# Optional table data
sales:
  revenue: [100, 200, 300]
  cost: [50, 100, 150]

# Test cases
assumptions:
  test_sum_basic:
    formula: "=SUM(1, 2, 3)"
    expected: 6

  test_average_column:
    formula: "=AVERAGE(sales.revenue)"
    expected: 200
```

## Related Projects

- [forge](https://github.com/mollendorff-ai/forge) - Deterministic YAML-based financial modeling engine
- [forge-e2e](https://github.com/mollendorff-ai/forge-e2e) - E2E test suite documentation hub
- [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r) - Statistical validation against R

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
