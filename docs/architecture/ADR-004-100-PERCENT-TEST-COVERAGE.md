# ADR-004: 100% Test Coverage Requirement

**Status:** Accepted
**Date:** 2025-12-04
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Forge is a financial modeling tool used for budget planning, variance analysis, and investor-ready financial projections. Financial calculations must be accurate - errors can lead to incorrect business decisions, misrepresented metrics to investors, or compliance issues.

### The Problem

Financial software has **zero tolerance for calculation errors**:

1. **Investment decisions** - Incorrect NPV/IRR calculations can lead to bad investments
2. **Budget variances** - Wrong variance calculations misrepresent financial health
3. **Depreciation** - Errors in SLN/DDB/DB affect tax calculations and asset reporting
4. **Statistical analysis** - Incorrect STDEV/VAR can misrepresent risk profiles
5. **Cash flow projections** - PMT/FV/PV errors cascade through entire models
6. **Excel export for investors** - Exported spreadsheets MUST be accurate and auditable

### Critical Path: Excel Import/Export

**Excel integration is mission-critical.** Users will:
- Export financial models to Excel for stakeholders
- Share spreadsheets with investors for due diligence
- Use exported Excel files for real business modeling
- Import existing Excel models into Forge

A bug in Excel export means:
- Wrong formulas in investor spreadsheets
- Incorrect values in financial reports
- Failed audits due to calculation discrepancies
- Loss of trust from stakeholders

**Excel import/export requires 100% coverage with no exceptions.**

### Options Considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A: Best effort testing** | Write tests as needed | Fast development | Unknown coverage gaps |
| **B: 80% coverage target** | Industry standard minimum | Reasonable balance | 20% untested = risk |
| **C: 100% line coverage** | Every line executed | No blind spots | More test code |
| **D: 100% + edge cases** | Full coverage + boundary tests | Maximum confidence | Highest effort |

## Decision

**Option D: 100% line coverage with mandatory edge case tests for all financial functions.**

### Enforcement Mechanism

```
┌─────────────────────────────────────────────────────────────────┐
│                    COVERAGE ENFORCEMENT                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  make coverage          Run coverage, fail if < 100%            │
│  make coverage-report   Generate detailed HTML report           │
│  make coverage-ci       CI mode: strict 100% enforcement        │
│                                                                 │
│  CI/CD Pipeline:                                                │
│  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌─────────┐           │
│  │  Test   │──│  Lint   │──│ Coverage │──│ Release │           │
│  └─────────┘  └─────────┘  └──────────┘  └─────────┘           │
│                               │                                 │
│                               ▼                                 │
│                         < 100%? FAIL                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Coverage Requirements

| Category | Requirement | Rationale |
|----------|-------------|-----------|
| **Line coverage** | 100% | Every code path must be tested |
| **Branch coverage** | 100% | All conditional branches tested |
| **Function coverage** | 100% | No untested functions |
| **Edge cases** | 20% of tests | Boundary conditions for financial math |

### Mandatory Edge Case Tests

Every financial function MUST have tests for:

1. **Zero values** - Division by zero, zero principal, zero rate
2. **Negative values** - Negative cash flows, negative rates
3. **Boundary values** - MIN/MAX float, precision limits
4. **Single element** - Arrays with 1 element
5. **Large datasets** - Performance with 1000+ elements

## Rationale

### Why 100%?

1. **Financial accuracy is non-negotiable**
   - A single untested edge case in IRR calculation could produce wrong investment recommendations
   - Untested variance calculations could misrepresent budget health

2. **Regulatory compliance**
   - SOX compliance requires demonstrable controls
   - Auditors expect comprehensive testing for financial software

3. **Investor confidence**
   - "Investor Ready" means auditable quality
   - 100% coverage is a clear, measurable metric

4. **Technical debt prevention**
   - Untested code is unmaintainable code
   - 100% coverage forces good design (testable = modular)

### Why Not 80%?

The "80% is good enough" argument fails for financial software:

```
80% coverage = 20% untested
20% of 2000 lines = 400 untested lines
400 lines × 0.1% bug rate = 0.4 bugs
0.4 bugs in financial calculations = UNACCEPTABLE
```

Even one bug in:
- NPV calculation → Wrong investment decision
- PMT calculation → Incorrect loan amortization
- VARIANCE → Misrepresented budget health

### Why Edge Cases?

Standard tests verify "happy path". Edge cases catch:

| Edge Case | Without Test | Risk |
|-----------|--------------|------|
| `IRR([])` | Untested | Division by zero crash |
| `STDEV([x])` | Untested | NaN propagation |
| `PMT(0, n, pv)` | Untested | Infinity result |
| `NPV(r, [])` | Untested | Wrong return value |
| `MEDIAN([])` | Untested | Index out of bounds |

## Consequences

### Positive

1. **Zero calculation bugs in production**
   - Every line tested = every line works
   - Edge cases caught before release

2. **Confident refactoring**
   - Tests catch regressions immediately
   - Safe to optimize financial algorithms

3. **Auditable quality**
   - Coverage reports prove testing rigor
   - Investors see professional engineering

4. **Documentation by example**
   - Tests show expected behavior
   - Edge case tests document limitations

### Negative

1. **More test code**
   - ~1:1 ratio of test code to source code
   - Accepted tradeoff for financial accuracy

2. **Slower CI pipeline**
   - Coverage analysis adds ~30 seconds
   - Accepted tradeoff for quality gate

3. **Strict enforcement**
   - Can't merge with < 100% coverage
   - Forces upfront test writing (good practice)

## Implementation

### Tool: cargo-llvm-cov

```bash
# Install
cargo install cargo-llvm-cov

# Run with enforcement
cargo llvm-cov --fail-under 100

# Generate HTML report
cargo llvm-cov --html
```

### Makefile Targets

```makefile
coverage:
	@cargo llvm-cov --fail-under 100

coverage-report:
	@cargo llvm-cov --html --open

coverage-ci:
	@cargo llvm-cov --fail-under 100 --lcov --output-path lcov.info
```

### CI/CD Integration

```yaml
coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: taiki-e/install-action@cargo-llvm-cov
    - run: cargo llvm-cov --fail-under 100
```

### Test Count Targets

| Metric | Target | Current |
|--------|--------|---------|
| Total tests | 500+ | 542 |
| Edge case tests | 100+ (20%) | ~110 |
| Line coverage | 100% | 68% (work in progress) |
| Failures | 0 | 0 |
| Warnings | 0 | 0 |

### Current Coverage Status (v5.0.0)

| Module | Coverage | Notes |
|--------|----------|-------|
| error.rs | 100% ✅ | Complete |
| lsp/capabilities.rs | 100% ✅ | Complete |
| lsp/document.rs | 100% ✅ | Complete |
| excel/reverse_formula_translator.rs | 94% | Near complete |
| parser/mod.rs | 86% | Good coverage |
| types.rs | 87% | Good coverage |
| excel/formula_translator.rs | 86% | Good coverage |
| excel/importer.rs | 84% | Good coverage |
| core/array_calculator.rs | 72% | Large module, more tests needed |
| cli/commands.rs | 60% | Many IO-dependent paths |
| mcp/server.rs | 53% | Internal tests exist |
| api/handlers.rs | 57% | Handler tests exist |
| api/server.rs | 49% | Async server code |
| update.rs | 27% | Network-dependent |
| lsp/server.rs | 0% | Async LSP protocol |
| main.rs | 0% | CLI entry point |
| bin/* | 0% | Binary entry points |

### Path to 100%

Every line of code MUST be tested. Required changes:

1. **Entry Points (main.rs, bin/*)**: Extract ALL logic into testable library functions
2. **LSP Server**: Add mock LSP client for protocol testing
3. **API Server**: Add integration tests with actual HTTP requests
4. **update.rs**: Mock network calls with test fixtures
5. **array_calculator.rs**: Add tests for every formula branch

## Exceptions

**NO EXCEPTIONS. ZERO. NONE.**

If code cannot be tested, it must be refactored until it CAN be tested:
1. **Extract pure functions** - Move ALL logic out of I/O code
2. **Dependency injection** - Mock ALL external dependencies
3. **Thin entry points** - main() should only call library code

The build WILL FAIL if coverage is below 100%. This is non-negotiable.

## Monitoring

Coverage is tracked in CI and must never decrease:

```
PR Merge Requirements:
✓ All tests pass
✓ Zero warnings
✓ 100% line coverage
✓ 100% branch coverage
✓ Coverage report uploaded
```

---

## Related

- [ADR-001: HTTP REST Over gRPC](ADR-001-NO-GRPC.md)
- [ADR-002: Variance YAML Only](ADR-002-VARIANCE-YAML-ONLY.md)
- [ADR-003: Editor Extension Architecture](ADR-003-EDITOR-EXTENSIONS.md)
- [07-TESTING-ARCHITECTURE](07-TESTING-ARCHITECTURE.md)

---

**Previous:** [ADR-003](ADR-003-EDITOR-EXTENSIONS.md)
