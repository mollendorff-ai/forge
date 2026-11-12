# Forge Architecture Documentation

Technical documentation for Forge's architecture.

## Architecture Documents

| Document | Description |
|----------|-------------|
| [00-OVERVIEW](00-OVERVIEW.md) | System context and principles (architecture sections out of date, see its note) |
| [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) | Module boundaries and interactions (superseded design, see its note) |
| [02-DATA-MODEL](02-DATA-MODEL.md) | Type system and data structures (superseded design, see its note) |
| [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) | Calculation pipeline (superseded design, see its note) |
| [04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md) | Graph algorithms, topological sort (superseded design, see its note) |
| [05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md) | Bidirectional YAML-Excel conversion (superseded design, see its note) |
| [08-API-SERVER-ARCHITECTURE](08-API-SERVER-ARCHITECTURE.md) | HTTP REST API |

## Architecture Decision Records (ADRs)

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](ADR-001-NO-GRPC.md) | HTTP REST Over gRPC | Accepted |
| [ADR-002](ADR-002-VARIANCE-YAML-ONLY.md) | Variance YAML Only | Accepted |
| [ADR-004a](ADR-004-100-PERCENT-TEST-COVERAGE.md) | 100% Test Coverage Requirement | Accepted (a goal, not enforced) |
| [ADR-011](ADR-011-XLFORMULA-EQUIVALENCE.md) | xlformula_engine Equivalence Testing | Superseded |
| [ADR-005](ADR-005-NO-LSP.md) | No Language Server Protocol | Accepted |
| [ADR-006](ADR-006-COVERAGE-EXCLUSIONS.md) | Coverage Exclusions | Accepted |
| [ADR-007](ADR-007-E2E-GNUMERIC-VALIDATION.md) | E2E Validation via Gnumeric | Accepted |
| [ADR-008](ADR-008-FPA-NATIVE-FUNCTIONS.md) | FP&A-Native Functions | Accepted |
| [ADR-009](ADR-009-YAML-FOR-AI-EFFICIENCY.md) | YAML for AI Token Efficiency | Accepted |
| [ADR-010](ADR-010-GIT-NATIVE-FINANCIAL-MODELING.md) | Git-Native Financial Modeling | Accepted |
| [ADR-012](ADR-012-FEATURE-FLAGS.md) | Feature Flags for Demo/Enterprise | Superseded |
| [ADR-013](ADR-013-FUNCTION-REGISTRY.md) | Function Registry | Implemented |
| [ADR-014](ADR-014-schema-versioning.md) | Schema Versioning | Partly superseded |
| [ADR-015](ADR-015-FUNCTION-SCALAR-CLASSIFICATION.md) | Function Scalar Classification | Partly superseded |
| [ADR-016](ADR-016-monte-carlo-architecture.md) | Monte Carlo Architecture | Accepted (§7 superseded) |
| [ADR-017](ADR-017-MONTE-CARLO-SEQUENTIAL.md) | Monte Carlo Sequential | Accepted |
| [ADR-018](ADR-018-SCENARIO-ANALYSIS.md) | Scenario Analysis | Accepted |
| [ADR-019](ADR-019-DECISION-TREES.md) | Decision Trees | Accepted |
| [ADR-020](ADR-020-REAL-OPTIONS.md) | Real Options | Accepted |
| [ADR-021](ADR-021-TORNADO-DIAGRAMS.md) | Tornado Diagrams | Accepted |
| [ADR-022](ADR-022-BOOTSTRAP-RESAMPLING.md) | Bootstrap Resampling | Accepted |
| [ADR-023](ADR-023-BAYESIAN-NETWORKS.md) | Bayesian Networks | Accepted |
| [ADR-024](ADR-024-SELF-UPDATE.md) | Self-Update | Accepted (restored) |
| [ADR-025](ADR-025-FEATURE-GATE-INVERSION.md) | Feature Gate Inversion | Superseded |
| [ADR-026](ADR-026-FPGA-HFT-ACCELERATION.md) | FPGA HFT Acceleration | Deferred |
| [ADR-027](ADR-027-E2E-TEST-MIGRATION.md) | E2E Test Migration | Partly superseded |
| [ADR-028](ADR-028-CRYSTAL-ANALYSIS.md) | Crystal Analysis | Not scheduled |
| [ADR-030](ADR-030-GTM-LICENSING-STRATEGY.md) | GTM Licensing Strategy | Superseded |
| [ADR-031](ADR-031-LICENSE-ELASTIC-2.0.md) | License: MIT OR Apache-2.0 | Accepted |
| [ADR-032](ADR-032-CLI-SCHEMA-EXAMPLES.md) | CLI Schema Examples | Accepted |
| [ADR-033](ADR-033-EXCEL-EXPORT.md) | Excel Export | Implemented |
| [ADR-034](ADR-034-EXCEL-IMPORT.md) | Excel Import | Implemented |
| [ADR-035](ADR-035-RICH-METADATA.md) | Rich Metadata | Implemented |
| [ADR-036](ADR-036-TESTING-PHILOSOPHY.md) | E2E Testing Philosophy | Accepted |
| [ADR-037](ADR-037-EXTERNAL-VALIDATION-ENGINES.md) | External Validation Engines | Accepted |
| [ADR-038](ADR-038-EDGE-CASE-DISCOVERY.md) | Edge Case Discovery | Accepted |
| [ADR-039](ADR-039-STATISTICAL-VALIDATION.md) | Statistical Validation | Accepted |
| [ADR-040](ADR-040-FINANCIAL-ANALYTICS-VALIDATION.md) | Financial Analytics Validation | Accepted |
| [ADR-041](ADR-041-AUTO-GENERATED-EXPECTED.md) | Auto-Generated Expected Values | Not implemented |
| [ADR-042](ADR-042-FUZZING-STRATEGY.md) | Property-Based Fuzzing Strategy | Not implemented |
| [ADR-043](ADR-043-REGRESSION-DETECTION.md) | Regression Detection | Not implemented |
| [ADR-044](ADR-044-SMART-ROUTING.md) | Smart Test Routing | Superseded |
| [ADR-045](ADR-045-COVERAGE-TRACKING.md) | Function-Level Coverage Tracking | Not implemented |
| [ADR-046](ADR-046-R-VALIDATED-ANALYTICS.md) | R-Validated Analytics E2E Tests | Accepted |
| [ADR-047](ADR-047-MCP-CORE-EXTRACTION.md) | Dual CLI/MCP Output (Core Extraction) | Accepted |
| [ADR-048](ADR-048-RMCP-MIGRATION.md) | rmcp Migration | Accepted |
| [ADR-049](adr-049-mcp-inline-content.md) | MCP Inline Content for Sandboxed Clients | Accepted |
| [ADR-050](ADR-050-KEEP-ALL-ANALYSIS-ENGINES.md) | Keep All Analysis Engines Supported | Accepted |
| [ADR-051](ADR-051-SCALAR-SHORT-FORM.md) | Scalar Short Form, No Nulls, Results Written Back in Place | Accepted |
| [ADR-052](ADR-052-MULTI-DOCUMENT-ONE-NAMESPACE.md) | The Documents of a File Share One Namespace | Accepted |
| [ADR-053](ADR-053-PERIODS-AND-THE-ROW-ABOVE.md) | Periods and the Row Above | Accepted |
| [ADR-054](ADR-054-A-DATE-IS-A-DATE.md) | A Date Is a Date | Accepted |
| [ADR-055](ADR-055-FORGE-IN-THE-BROWSER.md) | Forge in the Browser (forge-wasm) | Accepted, not yet built |
| [ADR-056](ADR-056-FORGE-STUDIO.md) | Forge Studio, a Web Interface in This Repository | Accepted, not yet built |
| [ADR-057](ADR-057-VALIDATION-IN-THIS-REPOSITORY.md) | The Validation Suites Live in This Repository | Proposed, not yet built |

## Architecture Principles

1. **Determinism Over Intelligence** - Mathematical calculations, not AI
2. **Excel Compatibility** - 1:1 mapping with Excel data structures
3. **Type Safety** - Rust's type system prevents errors
4. **Explicit Over Implicit** - Clear errors, no magic
5. **Backwards Compatibility** - Old models continue to work

## Reading Order

**New to Forge:**
1. [00-OVERVIEW](00-OVERVIEW.md) - Start here
2. [02-DATA-MODEL](02-DATA-MODEL.md) - Core data structures
3. [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) - How formulas work

**Adding Features:**
- New functions: [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md)
- Excel conversion: [05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md)
- API endpoints: [08-API-SERVER-ARCHITECTURE](08-API-SERVER-ARCHITECTURE.md)
