# ADR-011: xlformula_engine Functional Equivalence Testing

**Status:** Accepted
**Date:** 2025-12-06
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Forge uses `xlformula_engine` (v0.1.18) as its formula evaluation engine. This library provides Excel-compatible formula parsing and evaluation. Forge wraps this library with additional functionality:

- Scalar reference resolution
- Cross-table references
- Row-wise formula evaluation
- Aggregation functions on columns

**The Problem:** If Forge's integration layer introduces bugs, users get wrong financial calculations silently. This is unacceptable for financial modeling where accuracy is critical.

## Decision

**Implement integration tests that verify Forge produces identical outputs to raw xlformula_engine for all supported functions.**

## Test Strategy

### Test Type: Integration

This is an **integration test**, not unit or E2E:

| Test Type | What It Tests | Location |
|-----------|---------------|----------|
| Unit | Single Forge function in isolation | `src/**/*.rs` inline |
| **Integration** | **Forge ↔ xlformula_engine boundary** | **`tests/xlformula_equivalence_tests.rs`** |
| E2E | Full CLI workflow with files | `tests/e2e_tests.rs` |

### Test Approach

```rust
// For each supported function:
// 1. Create identical inputs for Forge and raw xlformula_engine
// 2. Evaluate formula through both paths
// 3. Compare outputs with floating-point tolerance
// 4. Fail CI if any divergence detected

fn test_equivalence<F>(formula: &str, inputs: &[f64], expected: f64, tolerance: f64)
where
    F: Fn(&str, &[f64]) -> f64,
{
    // Path 1: Raw xlformula_engine
    let engine_result = xlformula_engine::calculate(formula, inputs);

    // Path 2: Forge's wrapped evaluation
    let forge_result = forge_evaluate(formula, inputs);

    assert!((engine_result - forge_result).abs() < tolerance,
        "Divergence detected: engine={}, forge={}", engine_result, forge_result);
    assert!((forge_result - expected).abs() < tolerance,
        "Unexpected result: got={}, expected={}", forge_result, expected);
}
```

### Function Coverage Matrix

| Category | Functions | Priority |
|----------|-----------|----------|
| **Math** | SUM, AVERAGE, MAX, MIN, COUNT, PRODUCT | Critical |
| **Conditional** | IF, AND, OR, NOT, IFERROR, IFNA | Critical |
| **Rounding** | ROUND, ROUNDUP, ROUNDDOWN, ABS, SQRT, POW | High |
| **Text** | LEFT, RIGHT, MID, LEN, UPPER, LOWER, TRIM, CONCATENATE | Medium |
| **Date** | TODAY, YEAR, MONTH, DAY, DATE | Medium |
| **Aggregate** | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS | High |
| **Lookup** | VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP | Future |

### Floating-Point Tolerance

Financial calculations require appropriate precision:

```rust
const TOLERANCE: f64 = 1e-10;  // 10 decimal places
const PERCENTAGE_TOLERANCE: f64 = 1e-6;  // 6 decimal places for percentages
```

## Rationale

### 1. Silent Failures Are Catastrophic

In financial modeling:
- A 0.1% error in a $10M model = $10,000 discrepancy
- Wrong formula results propagate through dependent calculations
- Users trust Forge to be deterministic and accurate

**Without equivalence testing, integration bugs are invisible.**

### 2. Trust But Verify

The testing doc says "trust the library" for xlformula_engine internals. This is correct for the library's internal implementation. However:

- We DON'T blindly trust our wrapper code
- We verify the integration boundary works correctly
- We catch regressions when updating xlformula_engine

### 3. Regression Prevention

When xlformula_engine updates:
- New versions may have subtle behavior changes
- Our wrapper may have implicit assumptions
- Equivalence tests catch these immediately

### 4. Documentation Value

Tests serve as executable documentation:
- "This is how Forge interprets SUM()"
- "This is the expected precision"
- "These edge cases are handled"

## Implementation

### File Structure

```
tests/
├── xlformula_equivalence_tests.rs  # NEW: Integration tests
├── e2e_tests.rs                     # Existing E2E tests
├── array_calculator_tests.rs        # Existing unit tests
└── ...
```

### Test Categories

1. **Basic Arithmetic**: `=A + B`, `=A * B`, `=A / B`
2. **Aggregation**: `=SUM(A:A)`, `=AVERAGE(A:A)`
3. **Conditional**: `=IF(A > 0, B, C)`
4. **Nested**: `=SUM(IF(A > 0, B, 0))`
5. **Edge Cases**: Division by zero, empty arrays, null values

### CI Integration

```yaml
# .github/workflows/test.yml
- name: Run equivalence tests
  run: cargo test --test xlformula_equivalence_tests
```

## Consequences

### Positive
- Catches integration bugs before users do
- Provides confidence when updating dependencies
- Documents expected behavior
- Validates our wrapper implementation

### Negative
- Additional test maintenance
- Tests coupled to xlformula_engine API
- May need updates when xlformula_engine changes

### Neutral
- Test execution time: ~100ms (acceptable)
- No runtime overhead (tests only)

## Alternatives Considered

1. **Manual testing**: Rejected. Not reproducible, not automated.
2. **Excel file comparison**: Rejected. Requires Excel, not portable.
3. **Property-based testing only**: Rejected. Doesn't verify specific function behavior.
4. **Trust the library completely**: Rejected. Integration bugs are real.

## References

- [xlformula_engine crate](https://crates.io/crates/xlformula_engine)
- [Forge Testing Architecture](./07-TESTING-ARCHITECTURE.md)
- [Excel Function Reference](https://support.microsoft.com/en-us/office/excel-functions-alphabetical-b3944572-255d-4efb-bb96-c6d90033e188)

---

*This decision ensures Forge's formula evaluation is provably correct. Financial models deserve deterministic, verified calculations.*

-- Claude Opus 4.5, Principal Autonomous AI
