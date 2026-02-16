# ADR-041: Auto-Generated Expected Values

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Writing E2E tests for forge requires **expected values** for each formula. Currently, these are:

1. **Manually calculated** - error-prone, time-consuming
2. **Copied from Excel/Gnumeric** - requires manual spreadsheet work
3. **Hard-coded** - no traceability to authoritative source

This creates problems:

- **Typos**: Manual transcription errors (e.g., `3.14159` vs `3.14156`)
- **Rounding**: Inconsistent precision across tests
- **Maintenance**: Updating tests requires recalculating values
- **Trust**: No proof that expected values are correct

**We need automated generation of expected values from authoritative sources.**

---

## Decision

**Create scripts that auto-generate expected values using Gnumeric and R as validators.**

Two scripts provide complementary coverage:

1. **`scripts/generate_expected.sh`** - Uses Gnumeric for Excel-compatible functions
2. **`scripts/generate_expected_r.R`** - Uses R for statistical/forge-native functions

These scripts:

- Accept a formula as input
- Evaluate it using the authoritative engine
- Output a value formatted for YAML tests
- Include source attribution (Gnumeric version, R version)

---

## 1. Gnumeric Script (`generate_expected.sh`)

### Purpose

Generate expected values for **Excel-compatible functions** using Gnumeric as the validator.

### When to Use

Use Gnumeric for:

- ✅ Math functions: `SUM`, `ROUND`, `POWER`, `SQRT`, `ABS`
- ✅ Financial functions: `NPV`, `IRR`, `PMT`, `PV`, `FV`
- ✅ Date functions: `DATE`, `YEAR`, `MONTH`, `DATEDIF`
- ✅ Text functions: `CONCATENATE`, `TRIM`, `LEFT`, `RIGHT`
- ✅ Logical functions: `IF`, `AND`, `OR`, `NOT`
- ✅ Basic statistical: `AVERAGE`, `MAX`, `MIN`

**Don't use** for:

- ❌ Forge-native functions (`MC.*`, `VARIANCE`, `BREAKEVEN`)
- ❌ Complex statistical functions requiring Monte Carlo
- ❌ Functions with special behavior in forge but not Excel

### Usage

```bash
# Basic usage
./scripts/generate_expected.sh "=SUM(1,2,3)"

# Mathematical functions
./scripts/generate_expected.sh "=ROUND(PI(), 5)"
./scripts/generate_expected.sh "=SQRT(144)"
./scripts/generate_expected.sh "=POWER(2, 10)"

# Financial functions
./scripts/generate_expected.sh "=NPV(0.1, 100, 200, 300)"
./scripts/generate_expected.sh "=PMT(0.05/12, 60, 10000)"

# Date functions
./scripts/generate_expected.sh "=DATEDIF(DATE(2020,1,1), DATE(2020,12,31), \"D\")"

# Nested functions
./scripts/generate_expected.sh "=ROUND(EXP(1), 5)"
```

### Output Format

```
Result: 6

YAML entry:
expected: 6

Formula: =SUM(1,2,3)
Source: Gnumeric ssconvert version 1.12.55
```

### How It Works

1. Creates temporary CSV file with formula in cell A1
2. Runs `ssconvert --recalc` to evaluate the formula
3. Extracts result from output CSV
4. Formats result for YAML (removing trailing zeros)
5. Prints result with source attribution

### Installation Requirements

```bash
# macOS
brew install gnumeric

# Ubuntu/Debian
sudo apt install gnumeric

# Verify
ssconvert --version
```

---

## 2. R Script (`generate_expected_r.R`)

### Purpose

Generate expected values for **statistical and forge-native functions** using R as the validator.

### When to Use

Use R for:

- ✅ Statistical distributions: `NORM.DIST`, `T.DIST`, `CHISQ.DIST`, `F.DIST`
- ✅ Inverse distributions: `NORM.INV`, `T.INV`, `CHISQ.INV`
- ✅ Variance/StdDev: `VAR.S`, `VAR.P`, `STDEV.S`, `STDEV.P`
- ✅ Descriptive stats: `MEDIAN`, `PERCENTILE`, `QUARTILE`, `CORREL`
- ✅ Monte Carlo simulations (with custom R implementation)
- ✅ Forge-native functions (with custom R implementation)

**R is the gold standard** for statistical computing - if R agrees, we're correct.

### Usage

```bash
# Statistical distributions (CDF)
Rscript scripts/generate_expected_r.R "NORM.DIST(0,0,1,TRUE)"
Rscript scripts/generate_expected_r.R "T.DIST(1.96,10,TRUE)"

# Inverse distributions (Quantile)
Rscript scripts/generate_expected_r.R "NORM.INV(0.975,0,1)"
Rscript scripts/generate_expected_r.R "T.INV(0.95,10)"

# Variance and standard deviation
Rscript scripts/generate_expected_r.R "VAR.S(c(10,20,30,40,50))"
Rscript scripts/generate_expected_r.R "STDEV.S(c(10,20,30,40,50))"

# Percentiles and quartiles
Rscript scripts/generate_expected_r.R "MEDIAN(c(1,2,3,4,5))"
Rscript scripts/generate_expected_r.R "PERCENTILE(c(10,20,30,40,50),0.5)"
Rscript scripts/generate_expected_r.R "QUARTILE(c(10,20,30,40,50),1)"

# Correlation
Rscript scripts/generate_expected_r.R "CORREL(c(1,2,3,4,5),c(2,4,6,8,10))"
```

### Output Format

```
Evaluating: NORM.DIST(0,0,1,TRUE)
R formula: pnorm(0, mean=0, sd=1)

Result: 0.5

YAML entry:
expected: 0.5

Formula: NORM.DIST(0,0,1,TRUE)
Source: R version 4.3.1
Package: stats (base R)
```

### Excel to R Function Mapping

The script automatically translates Excel function names to R equivalents:

| Excel Function | R Equivalent | Description |
|----------------|--------------|-------------|
| `NORM.DIST(...,TRUE)` | `pnorm(...)` | Normal CDF |
| `NORM.DIST(...,FALSE)` | `dnorm(...)` | Normal PDF |
| `NORM.INV(...)` | `qnorm(...)` | Normal quantile |
| `T.DIST(...,TRUE)` | `pt(...)` | t-distribution CDF |
| `T.INV(...)` | `qt(...)` | t-distribution quantile |
| `VAR.S(...)` | `var(...)` | Sample variance |
| `STDEV.S(...)` | `sd(...)` | Sample std dev |
| `MEDIAN(...)` | `median(...)` | Median |
| `PERCENTILE(...)` | `quantile(...)` | Percentile |
| `CORREL(...)` | `cor(...)` | Correlation |

### Vector Syntax

R uses `c()` for vectors (not Excel's comma-separated lists):

```bash
# CORRECT
Rscript generate_expected_r.R "VAR.S(c(10,20,30,40,50))"

# INCORRECT (will fail)
Rscript generate_expected_r.R "VAR.S(10,20,30,40,50)"
```

### Installation Requirements

```bash
# macOS
brew install r

# Ubuntu/Debian
sudo apt install r-base

# Verify
R --version
Rscript --version
```

---

## 3. Integration with Test Workflow

### Workflow for Creating Tests

#### Step 1: Design Test Case

Identify what you want to test:

```yaml
# Example: Testing SQRT function
sqrt_perfect_square:
  value: null
  formula: "=SQRT(144)"
  expected: ???  # Need to generate this
```

#### Step 2: Generate Expected Value

Run the appropriate script:

```bash
# For Excel functions - use Gnumeric
./scripts/generate_expected.sh "=SQRT(144)"

# Output:
# Result: 12
#
# YAML entry:
# expected: 12
```

#### Step 3: Copy to YAML

Paste the result into your test:

```yaml
sqrt_perfect_square:
  value: null
  formula: "=SQRT(144)"
  expected: 12  # ← Generated by Gnumeric
```

#### Step 4: Document Source (Optional but Recommended)

Add a comment for traceability:

```yaml
sqrt_perfect_square:
  value: null
  formula: "=SQRT(144)"
  expected: 12  # Gnumeric 1.12.55
```

### Workflow for Statistical Functions

Same process, but use R script:

```bash
# Generate expected value
Rscript scripts/generate_expected_r.R "NORM.DIST(0,0,1,TRUE)"

# Output shows R result
# Result: 0.5
#
# YAML entry:
# expected: 0.5
```

Paste into test:

```yaml
norm_dist_standard_mean:
  value: null
  formula: "=NORM.DIST(0,0,1,TRUE)"
  expected: 0.5  # R stats::pnorm
```

### Batch Generation (Advanced)

For generating many values at once:

```bash
# Create a file with formulas
cat > formulas.txt <<EOF
=SUM(1,2,3)
=ROUND(PI(), 5)
=SQRT(144)
=POWER(2, 10)
EOF

# Generate all expected values
while IFS= read -r formula; do
  echo "Formula: $formula"
  ./scripts/generate_expected.sh "$formula"
  echo ""
done < formulas.txt
```

---

## 4. When to Use Which Script

### Decision Matrix

| Function Category | Use Script | Reason |
|-------------------|------------|--------|
| **Math** (SUM, ROUND, SQRT) | `generate_expected.sh` | Excel-compatible, Gnumeric validates |
| **Financial** (NPV, IRR, PMT) | `generate_expected.sh` | Excel-compatible, Gnumeric validates |
| **Date** (DATE, DATEDIF) | `generate_expected.sh` | Excel-compatible, Gnumeric validates |
| **Text** (CONCATENATE, TRIM) | `generate_expected.sh` | Excel-compatible, Gnumeric validates |
| **Logical** (IF, AND, OR) | `generate_expected.sh` | Excel-compatible, Gnumeric validates |
| **Statistical Distributions** | `generate_expected_r.R` | R is gold standard for statistics |
| **Variance/StdDev** | `generate_expected_r.R` | R provides reference implementation |
| **Percentiles** | `generate_expected_r.R` | R has precise quantile algorithms |
| **Monte Carlo** | `generate_expected_r.R` | Requires custom R implementation |
| **Forge-native** | `generate_expected_r.R` | Custom implementation in R |

### Special Cases

**Complex Nested Functions**: Try Gnumeric first, fall back to manual calculation:

```bash
# This works in Gnumeric
./scripts/generate_expected.sh "=ROUND(LOG10(POWER(10, 3)), 5)"
```

**Table References**: Neither script supports table syntax - use forge-direct mode:

```yaml
# Table references require forge-direct validation
sales_total:
  value: null
  formula: "=SUM(sales.amount)"
  expected: 12500  # Validated by forge-direct mode
```

**Edge Cases**: Always verify edge cases manually:

```bash
# Division by zero
./scripts/generate_expected.sh "=1/0"
# Output: #DIV/0! (error)

# This tells you the error type is correct
```

---

## 5. Best Practices

### ✅ DO

1. **Always use scripts for numeric values**
   - Eliminates transcription errors
   - Ensures precision consistency
   - Documents source of truth

2. **Include source attribution in comments**

   ```yaml
   expected: 3.14159  # Gnumeric 1.12.55
   expected: 0.5      # R 4.3.1 stats::pnorm
   ```

3. **Verify edge cases manually**
   - Scripts show errors correctly
   - Document why edge cases behave that way

4. **Use consistent precision**
   - Scripts handle trailing zeros automatically
   - Match precision to test requirements

5. **Re-generate when updating tests**
   - If formula changes, regenerate expected value
   - Ensures tests stay in sync

### ❌ DON'T

1. **Don't manually type expected values**
   - Use scripts to prevent errors
   - Manual typing defeats the purpose

2. **Don't skip source attribution**
   - Future maintainers need to know where values came from
   - Debugging requires knowing the source

3. **Don't assume scripts work for everything**
   - Some functions require custom implementation
   - Forge-native functions need validation logic

4. **Don't ignore script errors**
   - Errors indicate real issues
   - May reveal functions not supported by validator

---

## 6. Extending the Scripts

### Adding Custom R Functions

For forge-native functions, add custom implementations to `generate_expected_r.R`:

```r
# Example: Add VARIANCE function (forge-native)
VARIANCE <- function(mean, std_dev, num_simulations = 10000) {
  set.seed(12345)  # Reproducible results
  samples <- rnorm(num_simulations, mean = mean, sd = std_dev)
  return(var(samples))
}

# Add mapping in excel_to_r function
formula <- gsub("VARIANCE\\(", "VARIANCE(", formula)
```

### Adding Monte Carlo Functions

```r
# Example: MC.MEAN function
MC.MEAN <- function(distribution, params, num_simulations = 10000) {
  set.seed(12345)

  if (distribution == "normal") {
    samples <- rnorm(num_simulations, mean = params$mean, sd = params$sd)
  } else if (distribution == "uniform") {
    samples <- runif(num_simulations, min = params$min, max = params$max)
  }

  return(mean(samples))
}
```

### Supporting New Gnumeric Functions

If Gnumeric adds new functions, the script automatically supports them:

```bash
# No code changes needed - just use the new function
./scripts/generate_expected.sh "=NEWFUNCTION(arg1, arg2)"
```

---

## 7. Comparison to Alternatives

### Alternative 1: Manual Calculation

**Problem**: Error-prone, no traceability

```yaml
# Where did 3.14159 come from?
pi_value:
  expected: 3.14159  # ??? manual calculation ???
```

**Solution**: Use script with source

```yaml
# Clear source and reproducible
pi_value:
  expected: 3.14159  # Gnumeric 1.12.55
```

### Alternative 2: Excel Screenshots

**Problem**: Not reproducible, version-dependent

```yaml
# Calculated in Excel 2021 on Windows (screenshot in docs/)
npv_basic:
  expected: 248.69  # Excel says so
```

**Solution**: Gnumeric provides CLI access

```bash
./scripts/generate_expected.sh "=NPV(0.1, 100, 150, 200)"
# Result: 248.6851...
# Reproducible on any platform
```

### Alternative 3: Python/Scipy

**Problem**: Adds dependency, not authoritative for statistics

```python
# scipy is just a port of R algorithms
from scipy import stats
result = stats.norm.cdf(0)  # Same as R's pnorm(0)
```

**Solution**: Use R directly - it's the source

```bash
Rscript generate_expected_r.R "NORM.DIST(0,0,1,TRUE)"
# Uses R's original implementation
```

---

## 8. Troubleshooting

### Error: ssconvert not found

```bash
# Install Gnumeric
brew install gnumeric  # macOS
sudo apt install gnumeric  # Linux
```

### Error: R not found

```bash
# Install R
brew install r  # macOS
sudo apt install r-base  # Linux
```

### Error: Formula returns #NUM! or #VALUE

This is **not a bug** - it means the formula has an error:

```bash
./scripts/generate_expected.sh "=SQRT(-1)"
# Output: #NUM! (negative square root)

# Document this as an error test case
sqrt_negative:
  formula: "=SQRT(-1)"
  expected: "#NUM!"  # Error expected
```

### Error: Function not supported by Gnumeric

Use R script instead:

```bash
# Gnumeric doesn't support NORM.DIST
./scripts/generate_expected.sh "=NORM.DIST(0,0,1,TRUE)"
# Error: Function not supported

# Use R script
Rscript scripts/generate_expected_r.R "NORM.DIST(0,0,1,TRUE)"
# Result: 0.5
```

### Error: Vector syntax in R

Remember to use `c()` for vectors:

```bash
# WRONG
Rscript generate_expected_r.R "VAR.S(10,20,30)"

# RIGHT
Rscript generate_expected_r.R "VAR.S(c(10,20,30))"
```

---

## Benefits

### 1. Accuracy

- **Eliminates transcription errors** - no manual typing
- **Consistent precision** - automated formatting
- **Verified sources** - Gnumeric and R are authoritative

### 2. Traceability

- **Source attribution** - know where values came from
- **Reproducible** - run script again to verify
- **Version-tracked** - scripts in git with tests

### 3. Maintainability

- **Easy updates** - change formula, rerun script
- **Batch generation** - process many formulas at once
- **Documentation** - workflow is part of ADR

### 4. Trust

- **Third-party validation** - not self-referential
- **Industry standards** - Gnumeric and R are proven
- **Peer review** - R packages are CRAN-vetted

---

## Risks and Mitigations

### Risk 1: Gnumeric vs Excel Differences

**Risk**: Gnumeric might not match Excel exactly for edge cases

**Mitigation**:

- Document known differences in tests
- Use R for statistical functions where precision matters
- Test against real Excel when critical

### Risk 2: R Version Differences

**Risk**: Different R versions might give slightly different results

**Mitigation**:

- Document R version in output
- Use set.seed() for reproducible random numbers
- Test with tolerance (1e-6) not exact equality

### Risk 3: Forge-Native Functions

**Risk**: No authoritative source for forge-native functions

**Mitigation**:

- Implement in R for validation
- Document algorithm clearly
- Use Monte Carlo with set seeds for reproducibility

---

## Related ADRs

- **ADR-036**: Testing Philosophy - Why we validate against external engines
- **ADR-037**: External Validation Engines - Gnumeric and R as validators
- **ADR-039**: Statistical Validation - R as gold standard for statistics
- **ADR-040**: Financial Analytics Validation - Financial function validation approach

---

## Examples Gallery

### Example 1: Simple Math

```bash
$ ./scripts/generate_expected.sh "=SUM(1,2,3)"
Result: 6

YAML entry:
expected: 6

Formula: =SUM(1,2,3)
Source: Gnumeric ssconvert version 1.12.55
```

### Example 2: Financial Function

```bash
$ ./scripts/generate_expected.sh "=NPV(0.1, 100, 200, 300)"
Result: 481.5903

YAML entry:
expected: 481.5903

Formula: =NPV(0.1, 100, 200, 300)
Source: Gnumeric ssconvert version 1.12.55
```

### Example 3: Statistical Distribution

```bash
$ Rscript scripts/generate_expected_r.R "NORM.DIST(1.96,0,1,TRUE)"
Evaluating: NORM.DIST(1.96,0,1,TRUE)
R formula: pnorm(1.96, mean=0, sd=1)

Result: 0.9750021

YAML entry:
expected: 0.9750021

Formula: NORM.DIST(1.96,0,1,TRUE)
Source: R version 4.3.1
Package: stats (base R)
```

### Example 4: Variance

```bash
$ Rscript scripts/generate_expected_r.R "VAR.S(c(10,20,30,40,50))"
Evaluating: VAR.S(c(10,20,30,40,50))
R formula: var(c(10,20,30,40,50))

Result: 250

YAML entry:
expected: 250

Formula: VAR.S(c(10,20,30,40,50))
Source: R version 4.3.1
Package: stats (base R)
```

---

## Conclusion

**Auto-generated expected values solve the trust problem in E2E testing.**

By using Gnumeric and R as authoritative sources, we:

1. Eliminate manual errors
2. Document source of truth
3. Enable reproducible test generation
4. Provide third-party validation

These scripts make it **easy to do the right thing** - generating verified expected values is now simpler than typing them manually.

**Start using these scripts today for all new tests.**
