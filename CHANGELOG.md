# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- **Forge's own evaluator was never tested.** The runner only had Gnumeric recalculate the exported workbook. Each case now also runs `forge calculate`, and passes only when both agree with the expected value.
- A cell anywhere in the sheet that happened to equal the expected number counted as a pass (a table value could satisfy a quartile test). Only the cell labelled `test_result` counts now.
- Batch mode failed every case ("Failed to open CSV"). It now runs one workbook per source file, with that file's tables.
- `e2e_statistical_complete.yaml`: the data column `esc_quartile_test` should be `quartile_test`, so `forge calculate` could not run the file.
- Tables named `scenarios` were dropped from generated models, so every scenario case failed in Forge.
- `MOD(-10, 3)` expected -1; Excel, Gnumeric and Forge all give 2.
- README listed `--all`, which does not exist, and 1,909 tests; the suite has 1,301.

### Added

- `gnumeric_skip` on a case: Gnumeric 1.12 cannot judge it, so only Forge is checked. 37 cases use it.

## [1.1.1] - 2026-02-16

### Changed

- **Replace colored TUI with TAP v14 output**
  - Removed `colored` crate dependency
  - Output now uses TAP (Test Anything Protocol) version 14 format
  - YAML diagnostic blocks on failures (formula, expected, actual, error)
  - `# SKIP reason` directive for skipped tests
  - Diagnostic comments for config (forge path, engine version, test dir, mode)
  - Summary as TAP comment after results
  - Machine-parseable output enables CI/CD integration and downstream tooling

## [1.0.1] - 2026-01-24

### Changed

- **Rebranding: RoyalBit to Möllendorff AI**
  - Updated all references from RoyalBit to Möllendorff
  - **Why rebrand?** The "RoyalBit" name (company founded 2006) was hijacked by unrelated cryptocurrency scammers:
    - UK FCA issued official warning (Oct 2024) about "Royalbit Miners" - unauthorized firm
    - Multiple fraudulent domains: royalbit.ltd (trust score 38/100), royalbit.top, royal-bit.club
    - Classic HYIP Ponzi schemes offering impossible returns (155-580% in days)
    - Sources: [FCA Warning](https://www.fca.org.uk/news/warnings/royalbit-miners), [Scam Detector](https://www.scam-detector.com/validator/royalbit-ltd-review/)

## [1.0.0] - 2026-01-02

### Added

- Test runner with streaming and batch modes
- Gnumeric ssconvert integration for formula validation
- 1,909+ formula tests across 40 YAML files
- Function tests: math, text, date, financial, lookup, aggregation, statistical, trigonometric, logical, information
- Edge case tests: type coercion, error propagation, numeric precision
- Multi-sheet CSV parsing for flexible test organization
- Table data extraction for context-aware formula testing
