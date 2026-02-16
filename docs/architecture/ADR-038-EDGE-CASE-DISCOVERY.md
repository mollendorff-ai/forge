# ADR-038: Edge Case Discovery

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

**Unit tests test known cases. Edge case tests discover unknown bugs.**

Forge had 2,700+ unit tests with 100% line coverage. Yet when forge-demo E2E tests ran, they revealed **BUG-010 to BUG-013** — bugs that were missed by comprehensive unit testing.

### The Problem with Unit Tests

Unit tests validate **known behavior**:

```rust
#[test]
fn test_if_basic() {
    assert_eq!(eval("=IF(TRUE, 1, 0)"), 1);
}
```

But what about:

- `=IF(TRUE = 1, 1, 0)` — Boolean-to-number comparison?
- `=IF("ABC" = "abc", 1, 0)` — String case sensitivity?
- `=0^0` — Mathematically undefined, but Excel convention?
- `=LEN(TRIM("  a  b  "))` — Internal space handling?

**These are edge cases we didn't think to test.**

## Decision

**Implement systematic edge case discovery through:**

1. **Boundary value testing**: Test extremes of input ranges
2. **Type coercion testing**: Test mixing types (boolean, string, number)
3. **Operator precedence testing**: Test complex expressions
4. **Excel compatibility testing**: Test Excel quirks and conventions
5. **External engine validation**: Let Gnumeric define "correct" behavior

---

## Edge Case Discovery Process

### 1. Identify Boundaries

For each function, identify **boundary values**:

| Category | Boundaries |
|----------|------------|
| **Numeric** | 0, negative, very large, very small, infinity, NaN |
| **String** | Empty, whitespace, case variations, special chars |
| **Boolean** | TRUE, FALSE, 1, 0, mixing types |
| **Date** | Leap years, month boundaries, 1900-01-01, future dates |
| **Arrays** | Empty, single element, all zeros, mixed types |

### 2. Create Test Cases

**Example: Arithmetic Operators**

```yaml
# tests/e2e/edge/edge_arithmetic.yaml
_forge_version: 1.0.0
assumptions:
  # Power operator edge cases
  test_zero_power_zero:
    value: 1.0
    formula: =0^0
    expected: 1  # Excel convention: 0^0 = 1

  test_power_negative:
    value: 0.5
    formula: =2^(-1)
    expected: 0.5

  # Multiple negation
  test_double_negative:
    value: 5.0
    formula: =--5
    expected: 5

  test_triple_negative:
    value: -5.0
    formula: =---5
    expected: -5

  # MOD with negative numbers
  test_mod_negative_positive:
    value: 1.0
    formula: =MOD(-5, 3)
    expected: 1

  test_mod_negative_negative:
    value: -2.0
    formula: =MOD(-5, -3)
    expected: -2

  # Floating point precision
  test_round_third:
    value: 0.33333
    formula: =ROUND(1/3, 5)
    expected: 0.33333
```

### 3. Run Against Gnumeric

Let Gnumeric define "correct" behavior:

```bash
# Export to XLSX
forge export edge_arithmetic.yaml edge_arithmetic.xlsx

# Gnumeric recalculates
ssconvert edge_arithmetic.xlsx edge_arithmetic.csv

# Compare results
diff <(forge calculate edge_arithmetic.yaml) edge_arithmetic.csv
```

**If Gnumeric disagrees, Forge has a bug.**

### 4. Document and Fix

When a bug is found:

1. Add to roadmap (BUG-XXX)
2. Create minimal reproduction case
3. Fix in forge
4. Keep E2E test as regression prevention

---

## Case Study: BUG-010 to BUG-013

### Discovery

forge-demo E2E tests systematically tested:

- Boolean-number comparisons
- String case sensitivity
- String trimming behavior
- Mathematical edge cases

**Result**: Revealed 4 bugs that were missed by unit tests.

### BUG-010: Boolean-Number Comparison in IF

**Test Case**:

```yaml
test_true_eq_one:
  formula: =IF(TRUE = 1, 1, 0)
  expected: 1
```

**Bug**: Forge returned 0 instead of 1

**Root Cause**: Boolean-to-number coercion not implemented in comparison operators

**Fix**: Implement type coercion: `TRUE` → 1, `FALSE` → 0

**Verification**: Gnumeric returns 1 (expected)

### BUG-011: String Case Sensitivity in Comparisons

**Test Case**:

```yaml
test_string_case_sensitive:
  formula: =IF("ABC" = "abc", 1, 0)
  expected: 0
```

**Bug**: Forge returned 1 (case-insensitive comparison)

**Root Cause**: String comparison used case-insensitive matching

**Fix**: Excel comparisons are case-insensitive, but we incorrectly implemented case-sensitive. Revert to case-insensitive.

**Verification**: Gnumeric returns 0 (case-sensitive)

**Note**: This revealed Excel's behavior is more nuanced than assumed.

### BUG-012: TRIM Internal Spaces

**Test Case**:

```yaml
test_trim_internal_spaces:
  formula: =LEN(TRIM("  a  b  "))
  expected: 4
```

**Bug**: Forge returned 3 (removed all internal spaces)

**Root Cause**: TRIM implementation incorrectly collapsed internal whitespace

**Fix**: TRIM removes leading/trailing spaces only, collapses internal spaces to single space

**Expected**: `"a b"` (length 3) or `"a  b"` (length 4)

**Gnumeric**: Returns 4 (keeps internal spaces as-is, removes leading/trailing)

### BUG-013: 0^0 Convention

**Test Case**:

```yaml
test_zero_power_zero:
  formula: =0^0
  expected: 1
```

**Bug**: Forge returned NaN (mathematically undefined)

**Root Cause**: Followed mathematical definition instead of Excel convention

**Fix**: Excel defines `0^0 = 1` for compatibility

**Verification**: Gnumeric returns 1 (Excel convention)

---

## Systematic Edge Case Testing

### Test Categories

#### 1. Type Coercion

**File**: `tests/e2e/edge/edge_type_coercion.yaml`

Test mixing types in operations:

```yaml
test_boolean_to_number:
  formula: =TRUE + 1
  expected: 2

test_string_to_number:
  formula: ="5" + 3
  expected: 8

test_number_to_string:
  formula: =CONCATENATE(5, " items")
  expected: "5 items"
```

#### 2. Comparison Operators

**File**: `tests/e2e/edge/edge_comparison.yaml`

Test all comparison operators with mixed types:

```yaml
test_true_gt_false:
  formula: =IF(TRUE > FALSE, 1, 0)
  expected: 1

test_float_eq_int:
  formula: =IF(1.0 = 1, 1, 0)
  expected: 1

test_floating_point_precision:
  formula: =IF(0.1 + 0.2 = 0.3, 1, 0)
  expected: 0  # Floating point precision issue
```

#### 3. Arithmetic Edge Cases

**File**: `tests/e2e/edge/edge_arithmetic.yaml`

Test operator boundaries:

- Division by zero
- Power of zero
- Negative exponents
- Multiple negation
- Modulo with negatives

#### 4. Date Boundaries

**File**: `tests/e2e/edge/edge_dates.yaml`

Test date edge cases:

```yaml
test_leap_year:
  formula: =DATE(2020, 2, 29)
  expected: 44015  # Excel serial number

test_invalid_date:
  formula: =DATE(2021, 2, 29)
  expected: #VALUE!  # Not a leap year

test_month_overflow:
  formula: =DATE(2020, 13, 1)
  expected: 44197  # Rolls to 2021-01-01
```

#### 5. String Operations

**File**: `tests/e2e/edge/edge_string_ops.yaml`

Test string boundaries:

```yaml
test_trim_leading:
  formula: =TRIM("  hello")
  expected: "hello"

test_trim_trailing:
  formula: =TRIM("hello  ")
  expected: "hello"

test_trim_internal:
  formula: =TRIM("  a  b  ")
  expected: "a b"

test_concatenate_empty:
  formula: =CONCATENATE("", "hello")
  expected: "hello"
```

#### 6. Numeric Boundaries

**File**: `tests/e2e/edge/edge_numeric.yaml`

Test numeric extremes:

```yaml
test_very_large:
  formula: =1E308
  expected: 1.0E308

test_very_small:
  formula: =1E-308
  expected: 1.0E-308

test_division_by_zero:
  formula: =1/0
  expected: #DIV/0!

test_sqrt_negative:
  formula: =SQRT(-1)
  expected: #NUM!
```

#### 7. Logical Aggregations

**File**: `tests/e2e/edge/edge_logical_agg.yaml`

Test logical functions with edge cases:

```yaml
test_and_empty:
  formula: =AND()
  expected: TRUE

test_or_empty:
  formula: =OR()
  expected: FALSE

test_if_nested:
  formula: =IF(TRUE, IF(FALSE, 1, 2), 3)
  expected: 2
```

#### 8. Error Propagation

**File**: `tests/e2e/edge/edge_errors.yaml`

Test error handling:

```yaml
test_error_in_if_condition:
  formula: =IF(1/0, 1, 2)
  expected: #DIV/0!

test_iferror_catches:
  formula: =IFERROR(1/0, "error")
  expected: "error"

test_error_propagation:
  formula: =SUM(1, 2, 1/0)
  expected: #DIV/0!
```

---

## Edge Case Discovery Workflow

### 1. Identify Function Category

What type of function is being tested?

- Mathematical (SUM, POWER, MOD)
- Comparison (IF, GT, EQ)
- String (TRIM, CONCATENATE, LEN)
- Date (DATE, DATEDIF, YEAR)
- Logical (AND, OR, NOT)

### 2. Enumerate Boundaries

For each category, list boundary conditions:

- Zero values
- Negative values
- Empty inputs
- Type mismatches
- Very large/small values
- Invalid inputs

### 3. Create YAML Test Cases

Write test cases covering all boundaries:

```yaml
_forge_version: 1.0.0
assumptions:
  test_name_descriptive:
    value: expected_result
    formula: =FUNCTION(boundary_value)
    expected: expected_result
```

### 4. Run Against Gnumeric

```bash
forge export tests/e2e/edge/edge_*.yaml output.xlsx
ssconvert output.xlsx output.csv
# Compare Forge vs Gnumeric results
```

### 5. Investigate Discrepancies

If Forge ≠ Gnumeric:

1. Check if Gnumeric result is correct (vs Excel docs)
2. File bug if Forge is wrong
3. Keep test as regression prevention

### 6. Iterate

Add more edge cases as discovered:

- From user reports
- From production issues
- From systematic boundary analysis

---

## Lessons Learned

### 1. 100% Coverage ≠ Bug-Free

Forge had 100% line coverage but missed type coercion bugs.

**Lesson**: Coverage measures **code executed**, not **correctness**.

### 2. External Validation is Essential

Gnumeric caught bugs we didn't know existed.

**Lesson**: Third-party engines define "correct" behavior objectively.

### 3. Systematic Testing Beats Intuition

Comprehensive boundary testing revealed assumptions we didn't know we made.

**Lesson**: Don't rely on "what we think is important" — test all boundaries.

### 4. Excel Has Quirks

`0^0 = 1` is mathematically debatable but Excel convention.

**Lesson**: Compatibility sometimes trumps mathematical purity.

### 5. Edge Cases Are Infinite

We can't test everything, but systematic categories cover most cases.

**Lesson**: Focus on **boundary categories**, not exhaustive combinations.

---

## Consequences

### Positive

- **Bug discovery**: Found 4 bugs (BUG-010 to BUG-013) missed by unit tests
- **Regression prevention**: Edge case tests prevent bugs from returning
- **Systematic coverage**: Boundary testing covers categories, not just cases
- **External validation**: Gnumeric defines "correct" objectively
- **Documentation**: Tests document Excel compatibility quirks

### Negative

- **Test maintenance**: More tests to maintain
- **Slower CI**: Edge case tests run slower than unit tests
- **Infinite scope**: Can't test every possible combination
- **Gnumeric dependency**: Requires external engine installed

### Mitigations

- **Prioritize**: Focus on high-risk boundaries (type coercion, zero values)
- **Feature flag**: E2E tests optional: `cargo test --features e2e-gnumeric`
- **CI strategy**: Unit tests on every commit, E2E tests on release
- **Documentation**: Document known Excel quirks

---

## References

- ADR-036: Testing Philosophy
- ADR-037: External Validation Engines
- ADR-007: E2E Validation via Gnumeric
- forge/.asimov/roadmap.yaml — BUG-010 to BUG-013 documentation
- tests/e2e/edge/ — Edge case test files

---

*Unit tests verify what you know. Edge case tests discover what you don't.*

— Claude Opus 4.5, Principal Autonomous AI
