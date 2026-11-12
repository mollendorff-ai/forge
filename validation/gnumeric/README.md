# forge-e2e-gnumeric

E2E validation of [Forge](https://github.com/mollendorff-ai/forge) Excel-compatible functions against Gnumeric.

## Overview

This test suite validates that Forge calculates Excel-compatible formulas correctly by comparing results against Gnumeric, an independent spreadsheet engine battle-tested since 2001.

**Core Philosophy**: Forge is NEVER the authority.
All validation comes from trusted third-party sources.

### Test Coverage

- **1,446 formula tests** across 46 YAML files
- **30 function test files** covering math, text, date, financial, lookup, aggregation, statistical, trigonometric, logical, and information functions
- **10 edge case test files** covering boundary conditions, type coercion, error propagation, and numeric precision

## Requirements

- **Forge**: `cargo build --release` at the root of this repository (the suite runs `../../target/release/forge`), or set `FORGE_BIN`
- **Gnumeric**: `ssconvert` in PATH
  ```bash
  # macOS
  brew install gnumeric

  # Ubuntu/Debian
  apt install gnumeric
  ```
- **LibreOffice 24.8 or later**: `soffice` in PATH, for the 70 cases Gnumeric 1.12 cannot judge (each has a `libreoffice:` key saying why: no `LET`, `UNIQUE` or `XLOOKUP`, `DATEDIF` read from an xlsx file gives #VALUE!, and a few results where Gnumeric differs from Excel). Without it those cases fail and say so.
  ```bash
  # macOS
  brew install --cask libreoffice

  # Ubuntu 24.04 (its own LibreOffice is 24.2, without LET)
  sudo apt install -t noble-backports libreoffice-calc-nogui
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
                    ┌─ forge calculate ──────────────────────────┐
YAML Test Spec ─────┤                                            ├─ all three must equal expected
                    ├─ forge export → XLSX → ssconvert --recalc ─┤  (or LibreOffice)
                    └─ forge export → forge import → calculate ──┘
```

1. Load YAML test files with formulas and expected values
2. Create minimal YAML with the test formula and the file's tables
3. Run `forge calculate`: Forge's own answer
4. Run `forge export`, then `ssconvert --recalc`: Gnumeric's answer for Forge's workbook
5. Run `forge import` on the same workbook and `forge calculate` on the result: the answer after a round trip through Excel's file format
6. A case passes only when all three equal the expected value (relative tolerance 1e-9)

Only the cell labelled `test_result` counts as Gnumeric's answer.

A case with `libreoffice: "<why Gnumeric cannot>"` is recalculated by LibreOffice instead (`soffice --headless --convert-to csv`, every sheet). LibreOffice shows the results cached in an xlsx file unless told otherwise, and Forge caches its own results, so the harness runs it with a profile of its own that recalculates every formula on load. A case with `gnumeric_skip: "<why>"` is checked against Forge only: four are left, results where both engines differ from Excel, which was checked by hand.

Gnumeric 1.12 cannot judge some cases (`LET`, `UNIQUE`, `XLOOKUP`, `DATEDIF` read from xlsx, `ISFORMULA` on a literal). Mark those with `libreoffice: "reason"` next to `expected:`, and LibreOffice judges them. Only where neither engine gives Excel's answer, mark the case `gnumeric_skip: "reason"`: it is then checked against Forge only, and the summary counts it.

A case whose formula `forge export` writes as its value (a function Excel lacks) is not calculated by Gnumeric at all, so it fails unless it has a `gnumeric_skip` that says why. A spec can read included files with `_includes` (paths relative to the spec), as a Forge model does. A spec's tables are copied into each generated model: lists of numbers or text, row formulas, and period columns (`month: {start: 2026-12, count: 5, unit: month}`, model format 5.2.0). Where Excel and Gnumeric differ from each other, the case carries a `gnumeric_skip` reason that names the difference.

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

## Where it lives

This suite is part of Forge's repository (ADR-057, 2026-11-12); until then it was [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric), now archived, whose history is in this repository's history. It has no version of its own and is not published. Forge's Validation workflow runs it against the build of the same commit, so a change here and the Forge change it goes with are one pull request. A pull request that changes or removes an expected value here says `changes expected values` in its title (CONTRIBUTING.md).

The other suite is [forge-e2e-r](../r), in `validation/r`.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option, as Forge is.
