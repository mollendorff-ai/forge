# Changelog

All notable changes to Forge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Open Source Release

- **License**: MIT OR Apache-2.0 (standard Rust dual license)
- **Documentation**: Updated ADRs, README, CONTRIBUTING, editor extensions
- **Codebase cleanup**: Simplified project.yaml, roadmap, and source comments

### Code Quality (FOSS-004)

- **Fixed 1566 clippy pedantic/nursery/cargo warnings** across 180+ files — zero warnings remain
- Moved lint configuration from 20 Makefile `-A` flags to `Cargo.toml` `[lints.clippy]` section
- Pre-commit hook now enforces pedantic linting via `make lint`
- Added `# Errors`, `# Panics` doc sections to all public functions
- Added `#[must_use]` to pure functions throughout the codebase
- Replaced `push_str(&format!(...))` with `write!`/`writeln!` for string building
- Replaced `as f64` with `f64::from()` where lossless conversion exists
- Added underscore separators to numeric literals for readability
- Used `mul_add()` for fused multiply-add operations
- Documented all `#[allow(clippy::...)]` exceptions with invariant rationale

## [10.0.0-alpha.12] - 2026-02-16

### Changed

- **FOSS-004**: Fixed all 1566 clippy pedantic/nursery/cargo warnings → zero warnings
  - Moved lint config from Makefile `-A` flags to `[lints.clippy]` in Cargo.toml
  - Removed 20 Makefile lint suppressions that masked all warnings
  - Pre-commit hook now enforces pedantic linting via `make lint`
  - All `#[allow(clippy::...)]` annotations are in-code with documented justifications
  - Categories fixed: `cast_lossless`, `missing_errors_doc`, `missing_panics_doc`,
    `must_use_candidate`, `option_if_let_else`, `manual_let_else`, `match_same_arms`,
    `items_after_statements`, `needless_pass_by_value`, `unused_self`, `format_push_string`,
    `unreadable_literal`, `implicit_hasher`, `unnecessary_wraps`, `assigning_clones`,
    `or_fun_call`, `too_many_lines`, `cognitive_complexity`, `doc_markdown`,
    `needless_collect`, `branches_sharing_code`, and more
  - Unfixable: `multiple_crate_versions` (transitive deps, allowed in Cargo.toml),
    `cast_precision_loss` for `usize as f64` (no `f64::from(usize)` in Rust),
    `float_cmp` in test modules (financial math validated against Excel/Gnumeric/R)

## [10.0.0-alpha.11] - 2026-02-16

### Changed

- **BREAKING**: Consolidated 3 binaries into single `forge` CLI
  - `forge-mcp` → `forge mcp` subcommand
  - `forge-server` → `forge serve` subcommand (with `--host`, `--port` args)
  - Simpler installation, smaller disk footprint
- Removed `(enterprise only)` from all `--help` text (13 occurrences)
- Updated MCP config: `"command": "forge", "args": ["mcp"]`
- Updated CI/CD workflows for single binary (UPX, archives, checks)
- Updated documentation: FEATURES.md, ADRs (005, 006, 012, 08), CHANGELOG_ARCHIVE

## [10.0.0-alpha.10] - 2026-02-16

### Security

- **bytes** 1.11.0 → 1.11.1: Fix integer overflow in `BytesMut::reserve`

### Dependencies (Breaking)

- **jsonschema** 0.38 → 0.42: Updated `validator_for` API
- **calamine** 0.32 → 0.33
- **rand** 0.9 → 0.10: Migrated to `RngExt` trait, updated RNG construction
- **rand_distr** 0.5 → 0.6
- **rust_xlsxwriter** 0.92 → 0.93

### Dependencies (Compatible)

- 87 packages updated via `cargo update` (anyhow, clap, chrono, tokio, etc.)

## [10.0.0-alpha.9] - 2026-02-16

### Changed

- **Makefile**: Removed legacy `install-system`, `install-user`, `install` targets
- **Makefile**: `uninstall` now only handles `~/.cargo/bin`
- **project.yaml**: Updated `install_path` references to `~/.cargo/bin`
- Standardized all install paths on `~/.cargo/bin` (single location)

## [10.0.0-alpha.8] - 2026-01-24

### Changed

- **Install location**: `make install-forge` now installs to `~/.cargo/bin/` instead of `~/bin/`
  - `~/.cargo/bin` is the default Rust bin directory, automatically in PATH for Rust users
  - No need to manually add directory to PATH

## [10.0.0-alpha.7] - 2026-01-24

### Codebase Cleanup

Major cleanup removing orphaned files, consolidating documentation, and fixing organizational issues.

### Deleted (29 files)

- **Documentation**: `docs/INSTALLATION.md`, `docs/VERSION_HISTORY.md`, `FP&A_TEST_COVERAGE_AUDIT.md`
- **ADRs**: `ADR-003-EDITOR-EXTENSIONS.md` (superseded by ADR-005)
- **Schemas**: `forge-v1.0.schema.json.deprecated`, `.asimov/protocols.json`
- **Scripts**: `bin/release.sh`, `bin/session-end.sh`, `references.yaml`
- **Examples**: 4 orphaned .rs debug scripts
- **Test data**: 6 redundant test files, 11 files consolidated into 2

### Renamed/Moved (14 files)

- `ADR-004-XLFORMULA-EQUIVALENCE.md` → `ADR-011-XLFORMULA-EQUIVALENCE.md` (fix duplicate number)
- `ADR-024-SELF-UPDATE-REMOVAL.md` → `ADR-024-SELF-UPDATE.md` (fix misleading title)
- Design docs converted to ADRs: `ADR-033-EXCEL-EXPORT`, `ADR-034-EXCEL-IMPORT`, `ADR-035-RICH-METADATA`
- `test-data/v1.0/` → `test-data/examples/` (contents are v5.0.0, not v1.0.0)
- `test-data/v4_*.yaml` → `test-data/advanced/`
- `test-data/quota_forecast.yaml` → `examples/`

### Updated (14 files)

- Fixed ADR cross-references (ADR-012, ADR-015, ADR-030)
- Updated `00-OVERVIEW.md` stats (60→173 functions, 183→2133 tests)
- Fixed `.markdownlint.json` duplicate keys
- Updated `schema/README.md` to document both v1.0.0 and v5.0.0 schemas
- Removed unused `async-trait` dependency
- Fixed clippy warnings in `schema.rs` and `update.rs`

### Consolidated

- 8 function test files → `test_all_functions.yaml`
- 3 IF test files → `test_if_function.yaml`
- Archived CHANGELOG v7.x and earlier → `docs/CHANGELOG_ARCHIVE.md` (2567→662 lines)

## [10.0.0-alpha.6] - 2026-01-24

### Rebranding: RoyalBit to Mollendorff Group Inc.

Complete organizational rebrand to disassociate from cryptocurrency scammers who hijacked the RoyalBit name.

### Changed

- **Crate renamed**: `royalbit-forge` → `mollendorff-forge`
- **GitHub repository**: `royalbit/forge` → `mollendorff-ai/forge`
- **Website**: `royalbit.ca/forge` → `mollendorff.ai/forge`
- **Author email**: `admin@royalbit.ca` → `admin@mollendorff.ai`
- **All source files**: Updated `use royalbit_forge` → `use mollendorff_forge`
- **Schema $id URLs**: Updated to `mollendorff-ai/forge`
- **Documentation**: All ADRs, README, FEATURES.md updated
- **Editor extensions**: Zed and VSCode extensions updated
- **Config files**: .cargo/config.toml, rustfmt.toml, audit.toml
- **.asimov files**: project.yaml, warmup.json updated

### Why Rebrand?

The "RoyalBit" name (company founded 2006) was hijacked by unrelated cryptocurrency scammers:

- **UK FCA Warning (Oct 2024)**: Official warning about "Royalbit Miners" - unauthorized firm
- **Fraudulent domains**: royalbit.ltd (trust score 38/100), royalbit.top, royal-bit.club
- **HYIP Ponzi schemes**: Offering impossible returns (155-580% in days)
- **Sources**: [FCA Warning](https://www.fca.org.uk/news/warnings/royalbit-miners), [Scam Detector](https://www.scam-detector.com/validator/royalbit-ltd-review/)

## [10.0.0-alpha.5] - 2026-01-03

### Changed

- **Self-update command restored** per ADR-024
  - `forge update` now functional against GitHub releases
  - Valid now that forge is public on GitHub

## [10.0.0-alpha.4] - 2026-01-02

### CLI Documentation: schema + examples commands

Two new CLI commands for improved discoverability and self-documentation.

### Added

- **`forge schema` command** - Display JSON schemas for model validation
  - `forge schema --list` - List available versions (v1.0.0, v5.0.0)
  - `forge schema v1` - Output v1.0.0 schema (scalar-only models)
  - `forge schema v5` - Output v5.0.0 schema (arrays, tables, advanced features)
  - Pipeable: `forge schema v5 > schema.json` for IDE integration

- **`forge examples` command** - Show runnable YAML examples
  - `forge examples` - List all 9 available examples
  - `forge examples <name>` - Display example YAML with comments
  - `forge examples <name> --run` - Display and execute example
  - `forge examples --json` - Machine-readable list for tooling

- **9 Example YAML Files** (`examples/` directory)
  - `monte-carlo.yaml` - Probabilistic simulation with distributions
  - `scenarios.yaml` - Probability-weighted scenario analysis
  - `decision-tree.yaml` - Sequential decisions with backward induction
  - `real-options.yaml` - Option pricing for managerial flexibility
  - `tornado.yaml` - One-at-a-time sensitivity analysis
  - `bootstrap.yaml` - Non-parametric confidence intervals
  - `bayesian.yaml` - Probabilistic graphical models
  - `variance.yaml` - Budget vs actual with VARIANCE functions
  - `breakeven.yaml` - Break-even with BREAKEVEN functions

- **ADR-032** - Documents schema/examples command design decisions

### Technical

- Schemas and examples embedded at compile-time via `include_str!()`
- Zero network dependency, works offline
- Examples guaranteed to match binary version
- 7 new unit tests for schema/examples commands

## [10.0.0-alpha.3] - 2025-12-30

### CI/CD, crates.io, and Multi-Arch Releases

This release establishes the full CI/CD pipeline and multi-platform distribution.

### Added

- **GitHub Actions CI** - Automated test, lint, build on every push
- **GitHub Actions Release** - Multi-arch builds triggered on version tags
- **crates.io publishing** - `mollendorff-forge` published automatically on release
- **README badges** - CI status, crates.io version, license badge
- **forge-e2e CI/CD** - Same pipeline for E2E validation tool

### Release Artifacts (5 platforms)

- `forge-x86_64-unknown-linux-musl.tar.gz` - Linux x86_64 (static musl)
- `forge-aarch64-unknown-linux-musl.tar.gz` - Linux ARM64 (static musl)
- `forge-x86_64-apple-darwin.tar.gz` - macOS Intel
- `forge-aarch64-apple-darwin.tar.gz` - macOS ARM (Apple Silicon)
- `forge-x86_64-pc-windows-msvc.zip` - Windows x86_64

### Fixed

- **proc-macro build failure** - Removed `+crt-static` from GNU targets in `.cargo/config.toml`
  - CI tests run on GNU/glibc (dynamic linking for proc-macros)
  - Release builds use musl targets (fully static)
- **forge-lsp archive error** - Removed non-existent binary from release workflow

### Binary Contents

Each release archive contains:
- `forge` - Main CLI tool
- `forge mcp` - MCP server for Claude Desktop/IDEs (was `forge-mcp`)
- `forge serve` - REST API server (was `forge-server`)

## [10.0.0-alpha.2] - 2025-12-29

### Documentation and README Overhaul

- **README.md** - Complete rewrite highlighting technical capabilities
- **FEATURES.md** - Updated function labels and descriptions
- **MARKET_ANALYSIS.md** - Updated competitive analysis
- **ADR-031** - License architecture decision record
- **Editor extensions** - Updated VSCode and Zed README files

## [10.0.0-alpha.1] - 2025-12-29

### Documentation Overhaul for Public Release

- **README.md** - Rewritten for open source release
- **FEATURES.md** - Updated function descriptions
- **VERSION_HISTORY.md** - Updated with accurate stats

## [10.0.0-alpha.0] - 2025-12-29

### BREAKING: Unified Single Binary

Forge is now a single binary with all 173 functions. Previous feature-gated builds consolidated.

### Removed

- Feature-gated build variants and Cargo.toml feature flags
- ~330 cfg attributes for conditional compilation
- Conditional stub functions in statistical.rs, financial.rs
- Conditional Makefile build targets

### Changed

- Single binary: `forge` (6.3MB) with all 173 functions
- Simplified CLI help
- Updated version to 10.0.0-alpha.0

### Stats

- 1297 tests passing
- 47 files changed, 84 insertions, 659 deletions
- Zero feature gates in codebase

## [9.9.6] - 2025-12-29

### Fixed

- **FORGE-011**: ISERROR now detects NA() as error value
  - Location: `src/core/array_calculator/evaluator/info.rs:18-28`
  - IFERROR now catches NA values like Excel

### Verified

- **FORGE-010**: INVALID - Could not reproduce, arrays load correctly

## [9.9.5] - 2025-12-28

### Changed

- **Test consolidation complete**: All unit tests moved inline per Rust idiom
  - Eliminated `src/core/array_calculator/tests/` directory (40+ files)
  - Eliminated `src/cli/commands/tests/` directory (20+ files)
  - Tests now inline in evaluator files using `#[cfg(test)] mod tests`
  - Integration/E2E tests remain in `../forge-e2e/`

### Evaluators Updated

- `evaluator/math.rs`: math, math_edge_cases, numeric_edge_cases
- `evaluator/trig.rs`: trig tests
- `evaluator/text.rs`: text, text_edge_cases, string_edge_cases
- `evaluator/logical.rs`: logical, logical_edge_cases, comparison_edge_cases
- `evaluator/advanced.rs`: advanced, advanced_function_edge_cases
- `evaluator/aggregation.rs`: aggregation tests
- `evaluator/array.rs`: array, array_function_edge_cases
- `evaluator/conditional.rs`: conditional, conditional_function_edge_cases
- `evaluator/forge.rs`: forge tests
- `evaluator/dates.rs`: date tests
- `evaluator/lookup.rs`: lookup tests

### Test Results

- Unit tests: 1496 passed (consolidated from 2006 - duplicates removed)
- Test coverage: 100% (173/173 functions)
- Zero warnings
- Zero tests/ directories in src/

## [9.9.4] - 2025-12-28

### Added

- **HLOOKUP tests**: 9 comprehensive tests for horizontal lookup function
  - `test_hlookup_exact_match` - exact match with FALSE range_lookup
  - `test_hlookup_range_match` - approximate match finding largest value <= lookup
  - `test_hlookup_default_range_lookup` - default TRUE behavior
  - `test_hlookup_value_not_found` - error handling
  - `test_hlookup_text_exact_match` - text value lookups
  - `test_hlookup_first_element`, `test_hlookup_last_element` - boundary tests
  - `test_hlookup_single_element` - single element array
  - `test_hlookup_range_lookup_boundary` - exact boundary values
  - File: `src/core/array_calculator/tests/lookup/lookup_advanced.rs`

### Verified

- **Monte Carlo tests**: 57 tests confirmed in `src/monte_carlo/*.rs` (inline pattern)
- **Information function tests**: 14 tests confirmed in `src/core/array_calculator/evaluator/info.rs`

### Test Results

- Unit tests: 1965 passed (was 1956, +9 HLOOKUP)
- Test coverage: 100% (173/173 functions)
- All gaps closed

### Notes

- Swarm analysis initially reported 16 untested functions
- Investigation revealed MC and Information functions were already tested inline (Rust pattern)
- Only HLOOKUP was actually missing dedicated tests

## [9.9.3] - 2025-12-21

### Fixed

- **FORGE-003**: DATEDIF Y and YD calculation errors (CRITICAL, 30 tests)
  - Y unit: Now counts only complete years (checks anniversary date)
  - YD unit: Calculates days since last anniversary correctly
  - MD unit: Uses actual days in previous month, not fixed 30
  - YM unit: Adjusts for incomplete months

- **FORGE-004**: COLUMNS(table) returns 1 instead of column count (7 tests)
  - COLUMNS now checks if argument is a table reference and returns column count
  - File: `src/core/array_calculator/evaluator/lookup.rs`

- **FORGE-005**: Table validation rejects valid test data (10 tests)
  - Deferred column length validation to calculation time
  - Only validates when row formulas are present (row-wise operations)
  - Tables with independent columns (used as separate arrays) now load correctly

- **TRIM function**: Now collapses multiple internal spaces to single space (Excel behavior)
  - Was only removing leading/trailing spaces
  - File: `src/core/array_calculator/evaluator/text.rs`

### Test Results

- E2E tests: 602 passed, 40 failed (was: 569 passed, 49 failed)
- Fixed 33 test cases
- Remaining 40 failures are forge-e2e test file issues:
  - Test files using undefined functions (D, GCD, FACT)
  - Test files with wrong argument counts (XIRR, PERCENTILE, COUNTUNIQUE)
  - Features not supported by `forge calculate` (scenarios, MC functions)
  - Expected behavior differences (floating point epsilon comparison)

### Notes

- FORGE-001 (Boolean equality with numbers) was already working correctly
- Unit tests: All 1956 tests pass

## [9.9.2] - 2025-12-17

### Added

- **FORGE-EXPORT-001**: Excel export for v5.0.0 scalar models (CRITICAL)
  - Scalar groups now export to separate worksheets (e.g., `utilities`, `scenario_probs`, `analysis`)
  - Each scalar group becomes its own worksheet with Name/Value columns
  - Cross-group formula references translated to Excel syntax: `scenario_probs.p_aligned` → `'scenario_probs'!B2`
  - Monte Carlo formulas (`MC.*`) export as calculated values with original formula in cell comment
  - Scalars without dots (e.g., `tax_rate`) go to default "Scalars" worksheet

### Changed

- `ExcelExporter::export_scalars()` now creates grouped worksheets instead of single "Scalars" sheet
- Added `ScalarLocation` struct to track worksheet and row for each scalar
- Added `translate_grouped_scalar_formula()` for cross-worksheet references

### Impact

- Unblocked DANEEL publication - all game theory models can now export to Excel
- Models affected: game-theory-asi-race.yaml, game-theory-asi-race-mc.yaml, game-theory-asi-bridge.yaml, alignment-bayesian-network.yaml, asi-decision-tree.yaml, asimov-real-options.yaml

## [9.9.1] - 2025-12-17

### Fixed

- **FORGE-MC-001**: Monte Carlo dependent formula evaluation broken (CRITICAL)
  - `forge simulate` sampled MC.* functions correctly but didn't propagate values through dependent formulas
  - All downstream calculations returned 0.0 instead of computed values
  - Fix: Changed simulate command to use `run_with_evaluator()` instead of `run()`
  - Each iteration now: substitutes sampled values → runs ArrayCalculator → extracts computed outputs
  - Added regression test

### Impact

- Unblocked DANEEL paper probabilistic analysis
- All Monte Carlo models with dependent calculations now work correctly

## [9.8.0] - 2025-12-17

### Added

- **ADR-027**: E2E Test Migration Strategy
- **MIGRATION_INVENTORY.md**: Complete test file inventory

### Changed

- **E2E Test Migration to forge-e2e**: Completed full migration using 8 parallel agents
  - Phase 1 (4 agents): Documentation & ADRs
  - Phase 2 (4 agents): Test file migration
  - Phase 3: Cleanup & validation

### Migration Summary

| Category | Files | Tests | Destination |
|----------|-------|-------|-------------|
| Roundtrip | 10 | 59 | forge-e2e/tests/e2e/roundtrip/ |
| Edge Cases | 9 | 143 | forge-e2e/tests/e2e/edge/ |
| E2E CLI | 7 | 70 | forge-e2e/tests/ |
| YAML Data | 34 | - | forge-e2e/tests/e2e/ |

### forge-e2e ADRs Created

- ADR-001: Testing Philosophy (unit vs e2e vs roundtrip)
- ADR-002: External Validation Engines (Gnumeric, R, Python)
- ADR-003: Edge Case Discovery Process
- ADR-004: Statistical Function Validation

### Result

- forge unit tests: 1,643 passing
- forge-e2e: 272+ tests migrated
- Context window now manageable for Claude

## [9.7.0] - 2025-12-17

### Verified

- **BUG-010 to BUG-013**: Confirmed already fixed by v9.6.0 changes
  - BUG-010: Boolean-number comparison in IF (`=IF(TRUE = 1, 1, 0)`) - works
  - BUG-011: String literal comparison in IF (`=IF("ABC" = "abc", 1, 0)`) - works
  - BUG-012: TRIM internal spaces consistent (`=LEN(TRIM("  a  b  "))` = 4) - works
  - BUG-013: `0^0` returns 1 (Excel convention) - works

### Added

- ADR-026: FPGA/HFT Acceleration (deferred post-capitalization)
- Performance optimization milestones to roadmap (SIMD, Rayon, GPU, Algorithmic)

### Changed

- Cleaned up roadmap: removed completed items, all history in CHANGELOG.md
- All 13 bugs (BUG-001 through BUG-013) now resolved

## [9.6.1] - 2025-12-17

### Fixed

- **Release asset naming**: Fixed Makefile to use expected naming convention
  - Binaries now correctly renamed before GitHub release upload

## [9.6.0] - 2025-12-17

### BUG-003 to BUG-009 Fixes - Edge Cases

Fixed multiple bugs discovered by E2E testing.

### Fixed

- **BUG-003**: TRUE/FALSE boolean literals now recognized in formulas
- **BUG-004**: Boolean vs Number comparison works (TRUE = 1, FALSE = 0)
- **BUG-005**: DATE rollover for invalid day params (e.g., Feb 29 in non-leap year → Mar 1)
- **BUG-006**: Verified ISERROR already implemented (no changes needed)
- **BUG-007**: EOMONTH function now available
- **BUG-008**: & concatenation operator added to tokenizer and evaluator
- **BUG-009**: DATE() now returns Excel serial number (not text string)

### Added

- `Token::Boolean(bool)` to tokenizer for TRUE/FALSE literals
- `Expr::Boolean(bool)` to parser AST
- `&` operator support for string concatenation
- Day overflow/underflow handling in DATE function
- Edge case test YAMLs

### Changed

- DATE() returns `Value::Number` (Excel serial) instead of `Value::Text`
- `values_equal()` handles Boolean vs Number comparisons
- String comparison remains case-insensitive (correct Excel behavior)

## [9.5.0] - 2025-12-17

### BUG-002 Fix - DATE Subtraction Now Works

Fixed date subtraction returning "Left operand must be a number" error.

### Fixed

- **DATE subtraction**: `=DATE(2024,12,31) - DATE(2024,1,1)` now returns `365`
- All date arithmetic operations now work correctly

### Changed

- `Value::as_number()`: Now coerces date strings (YYYY-MM-DD) to Excel serial numbers
- Added tests for date subtraction (leap year, non-leap year scenarios)

### Root Cause

The `as_number()` method didn't recognize date strings. DATE() returns "2024-06-15" (text) but binary operators require numbers. Fixed by adding date string → Excel serial coercion in `as_number()`.

## [9.4.0] - 2025-12-16

### BUG-001 Fix - Calculate Engine Missing Functions

Fixed mismatched feature gates that caused some functions to be listed but not evaluable.

### Fixed

- **Trig functions**: SIN, COS, TAN, ASIN, ACOS, ATAN now work in calculate
- **Math functions**: EXP, LN, LOG10, INT, SIGN, TRUNC, PI now work in calculate

### Changed

- `evaluator/math.rs`: Removed incorrect feature gates from math functions
- `evaluator/mod.rs`: Trig module always included
- `evaluator/trig.rs`: Corrected function availability

### Root Cause

Function registry listed functions as available but evaluator had mismatched feature gates, causing functions to appear in `forge functions` but fail in `forge calculate`.

## [9.3.0] - 2025-12-16

### Build Simplification - Full-Featured Default

Changed build system so default build includes all features.

### Changed

- **Default build is full-featured** - `cargo build` produces complete binary
- **Tests run everything by default** - `cargo test` covers all ~2700 tests
- Updated ~489 cfg attributes across the codebase
- Updated Makefile build targets
- Updated Cargo.toml binary definitions

### Added

- **ADR-025**: Build simplification architecture decision record

### Rationale

- Improves developer experience (no feature flags needed)
- Ensures complete test coverage by default
- Simplifies CI/CD pipeline

## [9.2.0] - 2025-12-16

### Self-Update Command Removal - Dead Code Cleanup

Removed the non-functional `forge update` command.

### Removed

- `src/update.rs` (28KB) - Update checking and binary replacement logic
- `tests/update_tests.rs` (4KB) - Unit tests for update module
- `Update` command from CLI
- 3 integration tests for update command

### Changed

- Updated README.md - removed update from commands list
- Updated docs/cli/README.md - removed update section

### Added

- **ADR-024**: Self-Update Removal architecture decision record

### Rationale

The `forge update` command pointed to a non-existent release endpoint. ~32KB of dead code removed. (Restored in v10.0.0-alpha.5 after GitHub releases were set up.)

## [9.1.0] - 2025-12-15

### CLI Commands for Prediction Methods

Added CLI commands for all prediction analysis methods.

### Added

- `scenarios` command - probability-weighted scenario analysis
- `decision-tree` command - backward induction solver
- `real-options` command - Black-Scholes and Binomial Tree pricing
- `tornado` command - sensitivity visualization
- `bootstrap` command - non-parametric confidence intervals
- `bayesian` command - Bayesian network inference

### Changed

- Updated --help header with current stats (2703 tests)
- Updated documentation with Monte Carlo and prediction methods

## [9.0.0] - 2025-12-15

### Bayesian Networks - Causal Probabilistic Modeling

Module for probabilistic graphical models with belief propagation.

### Added

- **Bayesian Networks module** (`src/bayesian/`)
  - Directed acyclic graphs (DAG) with conditional probability tables
  - Variable elimination inference (exact inference)
  - Evidence propagation and belief updating
  - pgmpy-validated calculations
- **ADR-023**: Bayesian Networks architecture decision record

### Features

- Discrete nodes with finite states
- Conditional probability tables (CPTs)
- Forward and backward reasoning
- Most likely explanation (MPE)
- YAML/JSON export of results

## [8.7.0] - 2025-12-15

### Bootstrap Resampling - Non-Parametric Confidence Intervals

Module for statistical inference without distribution assumptions.

### Added

- **Bootstrap module** (`src/bootstrap/`)
  - Resampling with replacement
  - Percentile confidence intervals
  - BCa (bias-corrected accelerated) method
  - Multiple statistics: mean, median, std, var, percentile
- **ADR-022**: Bootstrap Resampling architecture decision record

## [8.6.0] - 2025-12-15

### Tornado Diagrams - Sensitivity Visualization

Module for one-at-a-time sensitivity analysis.

### Added

- **Tornado module** (`src/tornado/`)
  - Input range sensitivity calculation
  - Ranked impact visualization
  - JSON/CSV export for charting
  - R sensitivity package validated
- **ADR-021**: Tornado Diagrams architecture decision record

## [8.5.0] - 2025-12-15

### Real Options Analysis - Valuing Managerial Flexibility

Module for options-based project valuation.

### Added

- **Real Options module** (`src/real_options/`)
  - Black-Scholes closed-form pricing
  - Binomial Tree (Cox-Ross-Rubinstein) for American options
  - Greeks: delta, gamma, vega, theta, rho
  - Option types: defer, expand, contract, abandon, switch, compound
  - QuantLib-validated calculations
- **ADR-020**: Real Options architecture decision record

## [8.4.0] - 2025-12-15

### Decision Trees - Sequential Decision Modeling

Module for backward induction analysis.

### Added

- **Decision Trees module** (`src/decision_trees/`)
  - Backward induction solver
  - Node types: Decision, Chance, Terminal
  - Optimal path identification
  - DOT export for Graphviz visualization
  - SciPy/NumPy validated
- **ADR-019**: Decision Trees architecture decision record

## [8.3.0] - 2025-12-15

### Scenario Analysis - Discrete Branching

Module for Base/Bull/Bear analysis with probability weights.

### Added

- **Scenarios module** (`src/scenarios/`)
  - Probability-weighted expected value calculation
  - Discrete mutually exclusive outcomes
  - Per-scenario and aggregate reporting
  - R weighted.mean() validated
- **ADR-018**: Scenario Analysis architecture decision record

### Stats for v8.3.0-v9.0.0

| Module | Tests | Status |
|--------|-------|--------|
| Scenario Analysis | 18 | Pass |
| Decision Trees | 15 | Pass |
| Real Options | 32 | Pass |
| Tornado Diagrams | 19 | Pass |
| Bootstrap | 21 | Pass |
| Bayesian Networks | 23 | Pass |
| **Total New Tests** | **128** | Pass |

---

For versions prior to v8.0.0, see [Changelog Archive](docs/CHANGELOG_ARCHIVE.md)
