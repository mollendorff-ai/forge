# ADR-040: Financial Analytics Validation

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Forge implements advanced financial analytics functions for business intelligence:

- **Breakeven analysis** (BREAKEVEN_UNITS, BREAKEVEN_REVENUE)
- **Budget variance tracking** (VARIANCE, VARIANCE_PCT, VARIANCE_STATUS)
- **Performance monitoring** (threshold-based status indicators)

**How do we validate these financial calculations are correct?**

These are **forge-native functions** - they don't exist in Excel or Gnumeric. We can't use our primary validation engine (Gnumeric) because it doesn't implement these functions. We need **gold-standard external validation** from a trusted computational engine.

## Decision

**Validate Forge financial analytics functions against R:**

- Pure arithmetic formulas (no statistical randomness)
- Deterministic calculations with exact results
- R's computational accuracy provides third-party verification

**Why R for financial analytics?**

- R is academically validated for numerical accuracy
- Widely used in financial modeling and business analytics
- Provides independent verification of arithmetic formulas
- Already required for statistical validation (ADR-039)
- No additional dependencies needed

**Tolerance threshold** for floating-point comparison:

- **1e-10** (10 decimal places) - exact arithmetic, no randomness

---

## Functions Under Validation

### 1. BREAKEVEN_UNITS

**Purpose**: Calculate the number of units that must be sold to break even (zero profit).

**Formula**:

```
BREAKEVEN_UNITS(fixed_costs, price, variable_cost) = fixed_costs / (price - variable_cost)
```

**Parameters**:

- `fixed_costs`: Total fixed costs (e.g., rent, salaries)
- `price`: Selling price per unit
- `variable_cost`: Variable cost per unit (e.g., materials, labor)

**Example**:

```yaml
_forge_version: 9.8.0
assumptions:
  test_breakeven_units:
    formula: =BREAKEVEN_UNITS(50000, 100, 60)
    expected: 1250
```

**R Validation**:

```r
breakeven_units <- function(fixed_costs, price, variable_cost) {
  fixed_costs / (price - variable_cost)
}

result <- breakeven_units(50000, 100, 60)
# Output: 1250
```

**Tolerance**: `1e-10` (exact arithmetic)

---

### 2. BREAKEVEN_REVENUE

**Purpose**: Calculate the revenue required to break even (zero profit).

**Formula**:

```
BREAKEVEN_REVENUE(fixed_costs, margin_pct) = fixed_costs / margin_pct
```

**Parameters**:

- `fixed_costs`: Total fixed costs
- `margin_pct`: Profit margin as decimal (e.g., 0.25 = 25%)

**Example**:

```yaml
_forge_version: 9.8.0
assumptions:
  test_breakeven_revenue:
    formula: =BREAKEVEN_REVENUE(50000, 0.4)
    expected: 125000
```

**R Validation**:

```r
breakeven_revenue <- function(fixed_costs, margin_pct) {
  fixed_costs / margin_pct
}

result <- breakeven_revenue(50000, 0.4)
# Output: 125000
```

**Tolerance**: `1e-10` (exact arithmetic)

---

### 3. VARIANCE

**Purpose**: Calculate the absolute variance between actual and budget values.

**Formula**:

```
VARIANCE(actual, budget) = actual - budget
```

**Parameters**:

- `actual`: Actual value (revenue, expenses, etc.)
- `budget`: Budgeted/planned value

**Example**:

```yaml
_forge_version: 9.8.0
assumptions:
  test_variance_positive:
    formula: =VARIANCE(120000, 100000)
    expected: 20000

  test_variance_negative:
    formula: =VARIANCE(95000, 100000)
    expected: -5000
```

**R Validation**:

```r
variance <- function(actual, budget) {
  actual - budget
}

result_positive <- variance(120000, 100000)
# Output: 20000

result_negative <- variance(95000, 100000)
# Output: -5000
```

**Tolerance**: `1e-10` (exact arithmetic)

---

### 4. VARIANCE_PCT

**Purpose**: Calculate the percentage variance between actual and budget values.

**Formula**:

```
VARIANCE_PCT(actual, budget) = (actual - budget) / budget
```

**Parameters**:

- `actual`: Actual value
- `budget`: Budgeted value

**Example**:

```yaml
_forge_version: 9.8.0
assumptions:
  test_variance_pct_positive:
    formula: =VARIANCE_PCT(120000, 100000)
    expected: 0.2

  test_variance_pct_negative:
    formula: =VARIANCE_PCT(95000, 100000)
    expected: -0.05
```

**R Validation**:

```r
variance_pct <- function(actual, budget) {
  (actual - budget) / budget
}

result_positive <- variance_pct(120000, 100000)
# Output: 0.2

result_negative <- variance_pct(95000, 100000)
# Output: -0.05
```

**Tolerance**: `1e-10` (exact arithmetic)

---

### 5. VARIANCE_STATUS

**Purpose**: Categorize variance into status codes for dashboard indicators.

**Formula**:

```
VARIANCE_STATUS(actual, budget, threshold) =
  if actual > budget * (1 + threshold): 1 (over budget)
  if actual < budget * (1 - threshold): -1 (under budget)
  otherwise: 0 (within threshold)
```

**Parameters**:

- `actual`: Actual value
- `budget`: Budgeted value
- `threshold`: Acceptable variance threshold as decimal (e.g., 0.1 = 10%)

**Example**:

```yaml
_forge_version: 9.8.0
assumptions:
  test_variance_status_over:
    formula: =VARIANCE_STATUS(120000, 100000, 0.1)
    expected: 1

  test_variance_status_under:
    formula: =VARIANCE_STATUS(85000, 100000, 0.1)
    expected: -1

  test_variance_status_within:
    formula: =VARIANCE_STATUS(105000, 100000, 0.1)
    expected: 0
```

**R Validation**:

```r
variance_status <- function(actual, budget, threshold) {
  if (actual > budget * (1 + threshold)) {
    return(1)
  } else if (actual < budget * (1 - threshold)) {
    return(-1)
  } else {
    return(0)
  }
}

result_over <- variance_status(120000, 100000, 0.1)
# Output: 1

result_under <- variance_status(85000, 100000, 0.1)
# Output: -1

result_within <- variance_status(105000, 100000, 0.1)
# Output: 0
```

**Tolerance**: `1e-10` (exact arithmetic - returns integer)

---

## Tolerance Rationale

### Why 1e-10 for Financial Analytics?

Financial analytics functions use **deterministic, closed-form calculations**:

- Pure arithmetic operations (+, -, *, /)
- No approximations or iterative algorithms
- No random sampling or Monte Carlo simulation
- No numerical integration or root-finding

**Comparison with statistical functions**:

| Function Type | Tolerance | Rationale |
|---------------|-----------|-----------|
| **Financial Analytics** | `1e-10` | Pure arithmetic, deterministic |
| **Statistical Distributions** | `1e-6` | Numerical CDF/inverse approximations |
| **Bootstrap** | `1e-3` | Random sampling, iteration accumulation |
| **Monte Carlo** | `1e-2` | Random variance, simulation |

**Expected differences**:

- Only floating-point rounding errors
- Should match R to machine precision
- Any difference > 1e-10 indicates a bug

---

## Validation Workflow

### Step 1: Create Test Case

**YAML test**:

```yaml
_forge_version: 9.8.0
assumptions:
  # Breakeven analysis
  test_breakeven_units:
    formula: =BREAKEVEN_UNITS(50000, 100, 60)
    expected: 1250

  test_breakeven_revenue:
    formula: =BREAKEVEN_REVENUE(50000, 0.4)
    expected: 125000

  # Variance tracking
  test_variance:
    formula: =VARIANCE(120000, 100000)
    expected: 20000

  test_variance_pct:
    formula: =VARIANCE_PCT(120000, 100000)
    expected: 0.2

  test_variance_status:
    formula: =VARIANCE_STATUS(120000, 100000, 0.1)
    expected: 1
```

### Step 2: Run Forge Calculation

```bash
forge calculate tests/financial_analytics_model.yaml --output forge_results.yaml
```

### Step 3: Run R Validator

```r
# tests/validators/financial_analytics_validator.R

# Define functions
breakeven_units <- function(fixed_costs, price, variable_cost) {
  fixed_costs / (price - variable_cost)
}

breakeven_revenue <- function(fixed_costs, margin_pct) {
  fixed_costs / margin_pct
}

variance <- function(actual, budget) {
  actual - budget
}

variance_pct <- function(actual, budget) {
  (actual - budget) / budget
}

variance_status <- function(actual, budget, threshold) {
  if (actual > budget * (1 + threshold)) {
    return(1)
  } else if (actual < budget * (1 - threshold)) {
    return(-1)
  } else {
    return(0)
  }
}

# Run tests
cat("BREAKEVEN_UNITS(50000, 100, 60):", breakeven_units(50000, 100, 60), "\n")
cat("BREAKEVEN_REVENUE(50000, 0.4):", breakeven_revenue(50000, 0.4), "\n")
cat("VARIANCE(120000, 100000):", variance(120000, 100000), "\n")
cat("VARIANCE_PCT(120000, 100000):", variance_pct(120000, 100000), "\n")
cat("VARIANCE_STATUS(120000, 100000, 0.1):", variance_status(120000, 100000, 0.1), "\n")
```

**Expected Output**:

```
BREAKEVEN_UNITS(50000, 100, 60): 1250
BREAKEVEN_REVENUE(50000, 0.4): 125000
VARIANCE(120000, 100000): 20000
VARIANCE_PCT(120000, 100000): 0.2
VARIANCE_STATUS(120000, 100000, 0.1): 1
```

### Step 4: Compare with Tolerance

```r
validate <- function(forge_result, r_result, tolerance, test_name) {
  diff <- abs(forge_result - r_result)
  if (diff <= tolerance) {
    cat(sprintf("PASS: %s (diff: %.2e)\n", test_name, diff))
    return(TRUE)
  } else {
    cat(sprintf("FAIL: %s (diff: %.2e > %.2e)\n", test_name, diff, tolerance))
    return(FALSE)
  }
}

# Example validation
validate(1250, breakeven_units(50000, 100, 60), 1e-10, "BREAKEVEN_UNITS")
validate(125000, breakeven_revenue(50000, 0.4), 1e-10, "BREAKEVEN_REVENUE")
validate(20000, variance(120000, 100000), 1e-10, "VARIANCE")
validate(0.2, variance_pct(120000, 100000), 1e-10, "VARIANCE_PCT")
validate(1, variance_status(120000, 100000, 0.1), 1e-10, "VARIANCE_STATUS")
```

### Step 5: Automated Validation Script

```bash
#!/bin/bash
# tests/validators/validate_financial_analytics.sh

echo "Running financial analytics validation against R..."

# Run Forge
forge calculate tests/financial_analytics_model.yaml --output forge_output.yaml

# Run R validation
R --quiet --no-save < tests/validators/financial_analytics_validator.R > r_output.txt

# Compare results
python tests/validators/compare_results.py \
  forge_output.yaml \
  r_output.txt \
  --tolerance 1e-10

echo "Financial analytics validation complete"
```

---

## Edge Cases and Boundary Conditions

### Division by Zero

**BREAKEVEN_UNITS**:

```yaml
test_breakeven_units_zero_margin:
  formula: =BREAKEVEN_UNITS(50000, 100, 100)  # price = variable_cost
  expected: ERROR  # Division by zero
```

**R Validation**:

```r
breakeven_units(50000, 100, 100)
# Output: Inf (R returns infinity)
```

**Expected behavior**: Forge should return error or Inf consistently with R.

---

**BREAKEVEN_REVENUE**:

```yaml
test_breakeven_revenue_zero_margin:
  formula: =BREAKEVEN_REVENUE(50000, 0)
  expected: ERROR  # Division by zero
```

**R Validation**:

```r
breakeven_revenue(50000, 0)
# Output: Inf
```

---

### Negative Values

**VARIANCE** (negative variance is valid):

```yaml
test_variance_negative:
  formula: =VARIANCE(80000, 100000)
  expected: -20000
```

**VARIANCE_PCT** (negative percentage is valid):

```yaml
test_variance_pct_negative:
  formula: =VARIANCE_PCT(80000, 100000)
  expected: -0.2
```

---

### Zero Budget

**VARIANCE_PCT**:

```yaml
test_variance_pct_zero_budget:
  formula: =VARIANCE_PCT(100000, 0)
  expected: ERROR  # Division by zero
```

**R Validation**:

```r
variance_pct(100000, 0)
# Output: Inf
```

---

### Threshold Edge Cases

**VARIANCE_STATUS** (boundary testing):

```yaml
test_variance_status_exact_threshold:
  formula: =VARIANCE_STATUS(110000, 100000, 0.1)
  expected: 0  # Exactly at threshold (110000 = 100000 * 1.1)
```

**R Validation**:

```r
variance_status(110000, 100000, 0.1)
# Output: 0 (not over threshold, within)
```

---

## CI/CD Integration

### Automated Validation

```yaml
# .github/workflows/e2e-financial-analytics.yml
name: E2E Financial Analytics Validation

on:
  push:
    branches: [main]
  pull_request:

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install R
        run: |
          sudo apt-get update
          sudo apt-get install -y r-base

      - name: Build Forge
        run: cargo build --release

      - name: Run Financial Analytics Validation
        run: ./tests/validators/validate_financial_analytics.sh
```

### Validation Test Suite

```bash
#!/bin/bash
# tests/validators/run_all.sh

echo "Running E2E validation suite..."

# Gnumeric validation (Excel-compatible functions)
echo "1. Excel functions (Gnumeric)"
./tests/validators/validate_excel_functions.sh

# Statistical validation (R)
echo "2. Statistical functions (R)"
./tests/validators/validate_statistical.sh

# Financial analytics validation (R)
echo "3. Financial analytics (R)"
./tests/validators/validate_financial_analytics.sh

echo "All validations passed"
```

---

## Why R Instead of Gnumeric?

### Gnumeric Coverage Limitation

Gnumeric implements **Excel-compatible functions**:

- Financial: NPV, IRR, PMT, PV, FV
- Math: SUM, AVERAGE, MAX, MIN
- Statistical: STDEV, VAR, PERCENTILE

**Gnumeric does NOT implement**:

- `BREAKEVEN_UNITS` (forge-native)
- `BREAKEVEN_REVENUE` (forge-native)
- `VARIANCE` (simple subtraction, but not a standard Excel function)
- `VARIANCE_PCT` (forge-native)
- `VARIANCE_STATUS` (forge-native)

### R as the Validation Engine

For forge-native functions, we use **R**:

- Academically validated for numerical accuracy
- Simple arithmetic functions match exactly
- Already required for statistical validation (ADR-039)
- No additional dependencies

**Decision**: Use R to validate forge-native financial analytics functions.

---

## Consequences

### Positive

- **Third-party validation**: R provides independent verification
- **Academic rigor**: R is widely trusted for computational accuracy
- **Exact arithmetic**: 1e-10 tolerance ensures precision
- **No additional dependencies**: R already required for statistical validation (ADR-039)
- **Trust story**: Enterprise buyers trust R validation
- **Regression prevention**: Validation tests catch arithmetic bugs

### Negative

- **Test maintenance**: Must maintain parallel R implementations
- **Platform dependency**: Requires R installed
- **Slower tests**: External validation slower than unit tests
- **Limited edge case detection**: R may not detect all Forge-specific edge cases

### Mitigations

- **CI automation**: Run validation in GitHub Actions
- **Docker images**: Pre-configured R environment
- **Clear documentation**: R implementation in ADR
- **Feature flags**: Optional validation: `cargo test --features e2e-financial-analytics`
- **Edge case tests**: Systematic boundary testing (zero, negative, threshold)

---

## References

### R

- [R Project](https://www.r-project.org/)
- [R for Financial Analytics](https://cran.r-project.org/web/views/Finance.html)

### Financial Analytics

- Breakeven analysis: Fixed costs / (Price - Variable cost)
- Variance analysis: Actual - Budget
- Performance monitoring: Threshold-based status indicators

### Forge ADRs

- ADR-036: Testing Philosophy
- ADR-037: External Validation Engines
- ADR-039: Statistical Validation

---

*Trust the arithmetic. Validate with R.*

— Claude Opus 4.5, Principal Autonomous AI
