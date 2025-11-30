# Forge Version History

Archived version details from roadmap.yaml. For current roadmap, see [roadmap.yaml](../roadmap.yaml).

## Summary

| Version | Date | Highlights |
|---------|------|------------|
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
- **Tests**: 183 passing
- **Warnings**: ZERO (clippy -D warnings)
- **Built by**: Claude AI using RoyalBit Asimov

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
- `forge-server` - HTTP API for enterprise integration
- `forge-mcp` - Model Context Protocol for AI agents

## v2.0.0 - Enterprise HTTP API (2025-11-24)

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

Validated enterprise-scale performance:
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
