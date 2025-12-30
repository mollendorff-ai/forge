# Forge Version History

Archived version details from roadmap.yaml. For current roadmap, see [roadmap.yaml](../roadmap.yaml).

## Summary

| Version | Date | Highlights |
|---------|------|------------|
| v8.0.0 | 2025-12 | Monte Carlo Simulation (FP&A probabilistic analysis) |
| v7.2.0 | 2025-12-10 | 100% Test Integrity (15-Agent Parallel, 2,486 tests, 0 fake) |
| v7.1.1 | 2025-12-10 | XLSX Roundtrip 100% (formula translator fix) |
| v7.1.0 | 2025-12-10 | 100% Real Test Coverage (7-Agent Parallel) |
| v7.0.2 | 2025-12-09 | FP&A Accuracy Hotfix (6 fake tests fixed) |
| v7.0.1 | 2025-12-09 | Fake Test Remediation (71% hardcoded values fixed) |
| v7.0.0 | 2025-12-09 | 100% Function Coverage - Production Ready |
| v6.x | 2025-12-09 | Category Completion (v6.1.0 - v6.15.0) |
| v5.x - v6.0.0 | 2025-12-08-09 | Function testing & validation, E2E Gnumeric suite |
| v4.x - v5.x | 2025-11-26 - 2025-12-08 | Rich metadata, feature flags, function registry |
| v3.0.0 | 2025-11-25 | MCP Enhancements (10 tools for AI-finance integration) |
| v2.5.x | 2025-11-25 | Sensitivity analysis, goal-seek, break-even |
| v2.4.0 | 2025-11-25 | Performance validation (96K rows/sec) |
| v2.3.0 | 2025-11-25 | Variance analysis (budget vs actual) |
| v2.2.x | 2025-11-24 | XNPV/XIRR, scenarios, compare command |
| v2.1.0 | 2025-11-24 | Audit command for dependency tracing |
| v2.0.0 | 2025-11-24 | HTTP API server (forge-server) |
| v1.7.0 | 2025-11-24 | MCP server (forge-mcp) |
| v1.6.0 | 2025-11-24 | HTTP server foundation |
| v1.5.0 | 2025-11-24 | LSP server (forge-lsp) |
| v1.4.0 | 2025-11-24 | Watch mode (auto-calculate on save) |
| v1.3.0 | 2025-11-24 | Financial functions (NPV, IRR, PMT, etc.) |
| v1.2.0 | 2025-11-24 | Lookup functions (MATCH, INDEX, XLOOKUP) |
| v1.1.0 | 2025-11-24 | Conditional aggregations (SUMIF, COUNTIF, etc.) |
| v1.0.0 | 2025-11-23 | Array model, Excel export/import, 60+ functions |
| v0.2.0 | 2025-11-23 | Excel-compatible formulas (xlformula_engine) |
| v0.1.3 | 2025-11-23 | Initial release (basic formula evaluation) |

## Development Stats

- **Total development time**: ~40 hours autonomous
- **Tests**: 2,133 passing (1,297 unit + 836 E2E)
- **Functions**: 173 (167 Excel + 6 FP&A)
- **Coverage**: 100% function coverage
- **Warnings**: ZERO (clippy -D warnings)
- **Built by**: Claude AI using RoyalBit Asimov

## v8.0.0 - Monte Carlo Simulation (2025-12)

Probabilistic FP&A analysis with uncertainty quantification.

**Major Features:**

**Monte Carlo Engine:**
- Latin Hypercube Sampling (LHS) for efficient convergence
- Random sampling for high-iteration scenarios
- Configurable iteration counts (1K - 1M simulations)
- Deterministic seeding for reproducible results

**Probability Distributions:**
- `MC.Normal(mean, std_dev)` - Gaussian distribution
- `MC.Triangular(min, mode, max)` - Three-point estimates
- `MC.Uniform(min, max)` - Equal probability range
- `MC.PERT(min, mode, max)` - Beta distribution variant
- `MC.Lognormal(mean, std_dev)` - Multiplicative processes

**Output Analytics:**
- Percentile analysis (P10, P50, P90, custom)
- Probability thresholds (P(X > threshold))
- Sensitivity analysis (correlation coefficients)
- Full distribution statistics (mean, std dev, min, max)

**CLI Command:**
```bash
forge monte-carlo model.yaml --output results.yaml
```

**Performance Benchmarks:**
- 10K iterations, 20 variables: <5s
- 100K iterations, 20 variables: <30s
- 1M iterations, 20 variables: <5min
- Linear scaling with variable count

**Integration:**
- HTTP API endpoint: `POST /api/v1/monte-carlo`
- MCP tool: `forge_monte_carlo`
- Batch processing support

**Use Cases:**
- NPV/IRR uncertainty quantification
- Revenue/cost forecasting with confidence intervals
- Risk-adjusted valuations
- Capital budgeting under uncertainty
- Portfolio optimization

**Breaking Changes:**
- None (additive feature)

**Migration Guide:**
- Add `monte_carlo:` section to YAML models
- Replace scalar assumptions with `MC.*()` distributions
- Run `forge monte-carlo` instead of `forge calculate`

## v7.2.0 - 100% Test Integrity (2025-12-10)

15-Agent Parallel Execution achieving complete test integrity.

**Key Achievements:**
- 2,486 tests passing (up from 1,208 in v7.1.1)
- 189 new edge case tests added
- 90 weak test patterns eliminated
- Zero fake tests, 100% real formula evaluations

**New Features:**
- REPT function (text repetition, Excel-compatible)
- Strict date validation (invalid dates error properly)
- errors.rs module (41 error propagation tests)
- text_edge_cases.rs module (43 edge case tests)

**Test Quality Improvements:**
- Fixed IRR, MIRR, XNPV, XIRR tests with real function calls
- Fixed LAMBDA and LET tests with actual parameter binding
- Converted all hardcoded test values to genuine formulas
- Comprehensive edge case coverage across all function categories

## v7.1.1 - XLSX Roundtrip 100% (2025-12-10)

Formula translator fix achieving 100% roundtrip success.

**Fixed:**
- All 8 failing roundtrip tests now pass (57 total passing)
- Formula translator handles table.column references in any function
- Modularized test suite into tests/roundtrip/ directory

## v7.1.0 - 100% Real Test Coverage (2025-12-10)

7-Agent Parallel Execution expanding test coverage.

**Added:**
- 7 new financial functions (PPMT, IPMT, EFFECT, NOMINAL, PRICEDISC, YIELDDISC, ACCRINT)
- 129 new financial function tests
- New trig.rs test file with comprehensive trigonometric coverage
- 1,320 total unit tests (up from previous)

## v7.0.x Series - Production Ready (2025-12-09)

**v7.0.2**: FP&A Accuracy Hotfix - Fixed 6 fake tests with real function calls
**v7.0.1**: Fake Test Remediation - Fixed 71% hardcoded E2E tests
**v7.0.0**: 100% Function Coverage - All 159 functions with comprehensive E2E tests

## v1.0.0 - Array Model (2025-11-23)

First stable release with Excel-compatible array model.

**Key Features:**
- Type-safe array parsing (Number, Text, Date, Boolean)
- Row-wise formula evaluation
- Excel export/import with formula translation
- 60+ Excel functions
- JSON Schema validation
- 100 tests passing

**Breaking Changes:**
- New array model (column arrays map 1:1 with Excel)
- Backwards compatible with v0.2.0 scalar model

## v1.1.0 - Conditional Aggregations (2025-11-24)

**Functions Added:**
- SUMIF, COUNTIF, AVERAGEIF
- SUMIFS, COUNTIFS, AVERAGEIFS
- MAXIFS, MINIFS
- ROUND, ROUNDUP, ROUNDDOWN
- CEILING, FLOOR, MOD, SQRT, POWER

## v1.2.0 - Lookup Functions (2025-11-24)

**Functions Added:**
- MATCH, INDEX
- XLOOKUP, VLOOKUP, HLOOKUP
- OFFSET

## v1.3.0 - Financial Functions (2025-11-24)

**Functions Added:**
- NPV, IRR, MIRR
- PMT, FV, PV, RATE, NPER
- Date functions (TODAY, YEAR, MONTH, DAY, etc.)

## v1.4.0 - Watch Mode (2025-11-24)

**Command:** `forge watch model.yaml`

Auto-calculate on file save with debounced updates.

## v1.5.0 - LSP Server (2025-11-24)

**Binary:** `forge-lsp`

Language Server Protocol for editor integration.

## v1.6.0 & v1.7.0 - HTTP & MCP Servers (2025-11-24)

**Binaries:**
- `forge-server` - HTTP API server
- `forge-mcp` - Model Context Protocol for AI agents

## v2.0.0 - HTTP API (2025-11-24)

Production-ready HTTP server with:
- POST /calculate, /validate, /export, /import
- CORS support, request tracing
- Job queue for async processing

## v2.1.0 - Audit Command (2025-11-24)

**Command:** `forge audit model.yaml variable_name`

Shows dependency chain and value tracing for any variable.

## v2.2.x - Advanced Financial (2025-11-24)

**Functions Added:**
- XNPV, XIRR (date-based cash flows)

**Commands Added:**
- `forge compare` - Multi-scenario comparison
- `--scenario` flag for calculate command

## v2.3.0 - Variance Analysis (2025-11-25)

**Command:** `forge variance budget.yaml actual.yaml`

Budget vs actual comparison with:
- Absolute and percentage variances
- Favorable/unfavorable detection
- Threshold-based alerts

## v2.4.0 - Performance Validation (2025-11-25)

Validated production-scale performance:
- 96K rows/sec throughput
- 10K rows in 107ms
- 100K rows in ~1s
- Linear O(n) scaling

## v2.5.x - Sensitivity Analysis (2025-11-25)

**Commands Added:**
- `forge sensitivity` - 1D and 2D data tables
- `forge goal-seek` - Find input for target output
- `forge break-even` - Find zero-crossing

**Messaging Updates:**
- v2.5.2: "Zero tokens. Zero emissions."
- v2.5.3: "$40K-$132K/year saved."

## v3.0.0 - MCP Enhancements (2025-11-25)

**MCP Tools Added (5 new):**
- forge_sensitivity
- forge_goal_seek
- forge_break_even
- forge_variance
- forge_compare

Total MCP tools: 10 (5 core + 5 financial analysis)
