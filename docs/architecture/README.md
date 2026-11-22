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
| [ADR-027](ADR-027-E2E-TEST-MIGRATION.md) | E2E Test Migration | Partly superseded |
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
| [ADR-055](ADR-055-FORGE-IN-THE-BROWSER.md) | Forge in the Browser (forge-wasm) | Accepted; built (forge-wasm, in Forge Studio since 2026-11-12) |
| [ADR-056](ADR-056-FORGE-STUDIO.md) | Forge Studio, a Web Interface in This Repository | Accepted; built, at <https://forge.mollendorff.ai/> |
| [ADR-057](ADR-057-VALIDATION-IN-THIS-REPOSITORY.md) | The Validation Suites Live in This Repository | Accepted; built |
| [ADR-058](ADR-058-AGGREGATES-IN-A-ROW-FORMULA.md) | Aggregates in a Row Formula | Accepted; built |
| [ADR-059](ADR-059-AN-EMPTY-CELL.md) | An Empty Cell in a Number Column | Accepted; built |
| [ADR-060](ADR-060-A-DATE-AS-A-SCALAR.md) | A Date as a Scalar | Accepted; built |
| [ADR-061](ADR-061-A-NUMBER-FORMAT.md) | A Number Format, Next to the Unit | Accepted; built |
| [ADR-062](ADR-062-GENERATED-CASES.md) | Generated Cases Against Gnumeric and LibreOffice, in CI | Accepted; built |
| [ADR-063](ADR-063-ORIGIN-OF-FORGE.md) | Where Forge Came From, and Why It Is Not a Trading Engine | Accepted |
| [ADR-064](ADR-064-ARRAY-CONSTANTS.md) | Array Constants in Formulas | Proposed; built |
| [ADR-065](ADR-065-HOW-TWO-NUMBERS-COMPARE.md) | How Two Numbers Compare | Accepted by merging; checked in Excel (2026-11-18) |
| [ADR-066](ADR-066-A-LAST-SUM-NEAR-ZERO.md) | A Last Sum Near Zero Is Zero | Accepted by merging; measured in Excel (2026-11-18) |
| [ADR-067](ADR-067-A-SCENARIO-WITHOUT-A-PROBABILITY.md) | A Scenario Without a Probability Is Compared, Not Weighted | Accepted (option 1); built |
| [ADR-068](ADR-068-GAMES.md) | Games: a Payoff Table, Its Equilibria, and Tournaments | Accepted; built: games played once, tournaments, `forge game`, the `forge_game` MCP tool, Studio, and `validation/games`. Games of many players and the dynamics are ADR-073 |
| [ADR-069](ADR-069-ONE-MODEL-VALIDATOR.md) | One Model Validator, Without a JSON Schema Engine | Superseded by ADR-072 |
| [ADR-070](ADR-070-STUDIO-DOWNLOAD-BUDGET.md) | What Forge Studio's Download Budget Is For | Accepted |
| [ADR-071](ADR-071-A-YAML-MERGE-KEY-IS-REFUSED.md) | A YAML Merge Key Is Refused, With One Message | Accepted (option 1); built |
| [ADR-072](ADR-072-VALIDATOR-REVISITED.md) | jsonschema Checks Every Model Again | Accepted; built |
| [ADR-073](ADR-073-GAMES-OF-MANY-PLAYERS.md) | Games of Many Players, and Strategies That Spread | Accepted; built: the count form, the symmetric mixed equilibria, the `dynamics:` key and Forge Studio's view of them (2026-11-22) |
| [ADR-074](ADR-074-FORGE-WRITES-THE-MODEL.md) | Forge Writes the Model | Accepted, not yet built |
| [ADR-075](ADR-075-SIMULATIONS-AND-GAMES-ON-THE-CARDS.md) | Simulations and Games on Forge Studio's Cards | Accepted, not yet built |
| [ADR-076](ADR-076-WHAT-STUDIO-SHOWS-FIRST-AND-HOW-IT-HELPS.md) | What Forge Studio Shows First, and How It Helps | Accepted; built (the docs, Tools' whole answer, a set's questions, every model's Model view, the help) |

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
