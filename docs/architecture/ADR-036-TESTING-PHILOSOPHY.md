# ADR-036: Testing Philosophy

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

> **Note (2026-01-02):** The forge-e2e test suite has been split into dedicated repositories:
>
> - [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) - Excel formula validation
> - [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r) - Monte Carlo & analytics validation
>
> This ADR documents the philosophy; see child repos for implementation.

---

## Context

Forge is a financial calculation engine where **correctness is non-negotiable**. A bug in NPV calculation could cost millions. The question: **what testing strategy proves Forge calculates correctly?**

### The Testing Pyramid

```
                    ┌─────────────────────────┐
                    │   E2E Tests             │
                    │             │ ← Third-party validation
                    │   ~100 tests            │
                    ├─────────────────────────┤
                    │   Integration Tests     │
                    │   (forge/tests/)        │ ← Internal validation
                    │   ~500 tests            │
                    ├─────────────────────────┤
                    │   Unit Tests            │
                    │   (forge/src/*/tests/)  │ ← Function isolation
                    │   ~2,700 tests          │
                    └─────────────────────────┘
```

**Problem**: These all test **within** Forge. None prove Forge calculates the **right answer**.

## Decision

**Forge testing is split into two projects with distinct responsibilities:**

### 1. Unit Tests (forge/src/*/tests/)

**Purpose**: Test individual functions in isolation

**Location**: Embedded in source files via `#[cfg(test)]`

**What They Test**:

- Function logic correctness
- Edge cases (empty inputs, zero values, negative numbers)
- Error handling (division by zero, invalid dates)
- Type conversions (string to number, boolean coercion)

**Example**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npv_basic() {
        let cash_flows = vec![100.0, 200.0, 300.0];
        let result = npv(0.1, &cash_flows);
        assert_float_eq!(result, 481.5934, 1e-4);
    }

    #[test]
    fn test_npv_empty() {
        let result = npv(0.1, &[]);
        assert!(result.is_nan());
    }
}
```

**Coverage**: 100% line coverage (see ADR-004-100-PERCENT-TEST-COVERAGE.md)

**Trust Level**: Self-reported. These verify Forge works as **designed**, not that the design is **correct**.

### 2. E2E Tests

**Purpose**: Validate Forge calculations against external engines

**Location**: Separate project at `/Users/rex/src/mollendorff/forge-e2e/`

**What They Test**:

- **Full formula evaluation** against Gnumeric
- **Statistical functions** against R and Python
- **Roundtrip accuracy** (YAML → XLSX → Gnumeric → CSV)
- **Edge case discovery** through systematic boundary testing

**Test Types**:

#### A. Roundtrip Tests

Validate YAML → XLSX → Gnumeric → CSV pipeline:

```yaml
# tests/e2e/functions/financial.yaml
_forge_version: 1.0.0
assumptions:
  npv_basic:
    value: 481.5934
    formula: =NPV(0.1, {100, 200, 300})
    expected: 481.5934
```

**Flow**:

1. Forge loads YAML and calculates result
2. Forge exports to XLSX with formulas
3. Gnumeric opens XLSX and recalculates
4. Test compares Forge vs Gnumeric results
5. Pass if identical within tolerance (1e-10)

**Trust Level**: Third-party validation. If Gnumeric agrees, math is proven.

#### B. Edge Case Tests

Systematic boundary testing to discover bugs:

```yaml
# tests/e2e/edge/edge_arithmetic.yaml
_forge_version: 1.0.0
assumptions:
  test_zero_power_zero:
    value: 1.0
    formula: =0^0
    expected: 1  # Excel convention: 0^0 = 1

  test_mod_negative_negative:
    value: -2.0
    formula: =MOD(-5, -3)
    expected: -2
```

**Discovery Process**: These tests revealed BUG-010 to BUG-013 (see ADR-038-EDGE-CASE-DISCOVERY.md)

**Trust Level**: Gnumeric validates expected behavior, catches deviations.

#### C. Statistical Validation Tests

Validate statistical functions against R and Python:

```yaml
# tests/e2e/enterprise/bootstrap.yaml
_forge_version: 8.7.0
bootstrap:
  iterations: 10000
  data: [0.05, -0.02, 0.08, 0.03, -0.05]
  statistic: mean
  seed: 12345
```

**Validation**: Compare against R's `boot` package (see ADR-039-STATISTICAL-VALIDATION.md)

**Trust Level**: Gold standard libraries (R boot, scipy) validate complex statistics.

## Rationale

### Why Split Testing Across Two Projects?

| Criterion | forge (unit tests) | forge-e2e (E2E tests) |
|-----------|-------------------|----------------------|
| **What** | Individual functions | Full formula evaluation |
| **How** | Rust `#[test]` | YAML → XLSX → Gnumeric |
| **Trust** | Self-validated | Third-party validated |
| **Speed** | Fast (~1-2ms/test) | Slow (~100-200ms/test) |
| **Count** | ~2,700 tests | ~100 tests |
| **When** | Every commit | Release + CI |
| **Coverage** | 100% of code | 173 functions |

**Answer to "How do you know NPV is correct?"**:

- ❌ "We have unit tests" ← Self-reported
- ✅ "Gnumeric calculates the same result, and Gnumeric is academically validated" ← Third-party proof

### Why forge-e2e is a Separate Project

**Problem**: Forge codebase is 636K LOC, exceeding Claude's effective context window.

**Solution**: Dedicated E2E project keeps tests focused and maintainable.

**Benefits**:

- Independent development (parallel agents)
- Smaller context window per project
- Clear separation of concerns
- Easier CI/CD (unit tests run fast, E2E tests optional)

## Test Coverage Matrix

| Category | Unit Tests | E2E Tests |
|----------|-------------------|----------------------|
| **Math** | ✅ SUM, AVERAGE, MAX, MIN, etc. | ✅ Gnumeric validation |
| **Financial** | ✅ NPV, IRR, PMT, PV, FV | ✅ Gnumeric validation |
| **Date** | ✅ DATE, DATEDIF, YEAR | ✅ Gnumeric validation |
| **Statistical** | ✅ STDEV, VAR, MEDIAN | ✅ Gnumeric validation |
| **Text** | ✅ CONCATENATE, TRIM, LEN | ✅ Gnumeric validation |
| **Logical** | ✅ IF, AND, OR, NOT | ✅ Gnumeric validation |
| **Bootstrap** | ✅ Algorithm correctness | ✅ R boot package |
| **Monte Carlo** | ✅ RNG, distributions | ✅ scipy.stats |
| **Edge Cases** | ✅ Basic boundaries | ✅ Systematic discovery |

## Consequences

### Positive

- **Unit tests**: Fast feedback, 100% coverage, test-driven development
- **E2E tests**: Third-party validation, enterprise trust story, edge case discovery
- **Separation**: Parallel development, smaller context windows, clear responsibilities
- **Documentation**: Tests as executable proof of correctness

### Negative

- **Complexity**: Two test suites to maintain
- **Dependencies**: E2E tests require Gnumeric, R, Python
- **Speed**: E2E tests slower (~100x) than unit tests
- **CI**: Must manage two test environments

### Mitigations

- Unit tests run on **every commit** (fast, comprehensive)
- E2E tests run on **release** and **nightly CI** (slow, validation)
- Feature flags: `cargo test --features e2e-gnumeric`
- Documentation: Clear instructions in README.md

## Testing Principles

1. **Unit tests verify code works as written**
   → Test function logic, edge cases, error handling

2. **E2E tests verify code is correct**
   → Validate against external engines (Gnumeric, R, Python)

3. **Roundtrip tests prove pipeline integrity**
   → YAML → XLSX → Gnumeric → CSV must match

4. **Edge case tests discover bugs**
   → Systematic boundary testing reveals assumptions

5. **Trust comes from third parties**
   → "Gnumeric agrees" > "We tested ourselves"

## References

- ADR-004: 100% Test Coverage
- ADR-007: E2E Validation via Gnumeric
- ADR-037: External Validation Engines
- ADR-038: Edge Case Discovery
- ADR-039: Statistical Validation
- forge/src/*/tests/ — Unit tests
- forge-e2e/tests/e2e/ — E2E tests

---

*Unit tests verify you built the thing right. E2E tests verify you built the right thing.*

— Claude Opus 4.5, Principal Autonomous AI
