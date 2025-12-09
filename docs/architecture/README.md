# Forge Architecture Documentation

Technical documentation for Forge's architecture.

## Architecture Documents

| Document | Description |
|----------|-------------|
| [00-OVERVIEW](00-OVERVIEW.md) | System context, principles, high-level architecture |
| [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) | Module boundaries and interactions |
| [02-DATA-MODEL](02-DATA-MODEL.md) | Type system and data structures |
| [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) | Calculation pipeline, 149 functions |
| [04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md) | Graph algorithms, topological sort |
| [05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md) | Bidirectional YAML-Excel conversion |
| [08-API-SERVER-ARCHITECTURE](08-API-SERVER-ARCHITECTURE.md) | HTTP REST API (enterprise) |

## Architecture Decision Records (ADRs)

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](ADR-001-NO-GRPC.md) | HTTP REST Over gRPC | Accepted |
| [ADR-002](ADR-002-VARIANCE-YAML-ONLY.md) | Variance YAML Only | Accepted |
| [ADR-003](ADR-003-EDITOR-EXTENSIONS.md) | Editor Extension Architecture | Superseded |
| [ADR-004a](ADR-004-100-PERCENT-TEST-COVERAGE.md) | 100% Test Coverage Requirement | Accepted |
| [ADR-004b](ADR-004-XLFORMULA-EQUIVALENCE.md) | xlformula_engine Equivalence Testing | Accepted |
| [ADR-005](ADR-005-NO-LSP.md) | No Language Server Protocol | Accepted |
| [ADR-006](ADR-006-COVERAGE-EXCLUSIONS.md) | Coverage Exclusions | Accepted |
| [ADR-007](ADR-007-E2E-GNUMERIC-VALIDATION.md) | E2E Validation via Gnumeric | Accepted |
| [ADR-008](ADR-008-FPA-NATIVE-FUNCTIONS.md) | FP&A-Native Functions | Accepted |
| [ADR-009](ADR-009-YAML-FOR-AI-EFFICIENCY.md) | YAML for AI Token Efficiency | Accepted |
| [ADR-010](ADR-010-GIT-NATIVE-FINANCIAL-MODELING.md) | Git-Native Financial Modeling | Accepted |
| [ADR-012](ADR-012-FEATURE-FLAGS.md) | Feature Flags for Demo/Enterprise | Accepted |
| [ADR-013](ADR-013-FUNCTION-REGISTRY.md) | Function Registry | Accepted |

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
