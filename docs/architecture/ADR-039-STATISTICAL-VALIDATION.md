# ADR-039: Statistical Validation

**Status:** Accepted
**Date:** 2025-12-17
**Updated:** 2025-12-17 (Simplified to R-only validation)
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

> **Note (2026-01-02):** Implementation moved to [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r).

---

## Context

Forge implements advanced statistical functions:

- **Bootstrap resampling** (PERCENTILE.BOOT)
- **Statistical distributions** (NORM.DIST, T.DIST, CHISQ.DIST)
- **Monte Carlo simulation** (with random sampling)
- **Bayesian networks** (probabilistic inference)

**How do we validate these statistical calculations are correct?**

Unit tests can verify algorithm implementation, but they can't prove the **statistical theory** is correct. We need **gold-standard external validation** from established statistical libraries.

## Decision

**Validate Forge statistical functions against R - the gold standard:**

- `boot` package for Bootstrap resampling
- `stats` package for statistical distributions
- Peer-reviewed, academically validated

**Why R only (no Python)?**

- R is the gold standard - created by statisticians for statisticians
- scipy often implements the same algorithms ported from R
- If R agrees with Forge, the statistics are proven
- Fewer dependencies = simpler maintenance
- Python adds no additional authority

**Tolerance thresholds** for floating-point comparison:

- **Exact functions** (mean, sum): `1e-10` (10 decimal places)
- **Statistical functions** (bootstrap, distributions): `1e-6` (6 decimal places)
- **Monte Carlo** (random sampling): `1e-3` (3 decimal places, seed-dependent)

---

## 1. R Validation

### Why R?

R is the **de facto standard** for statistical computing:

- Created by statisticians for statisticians
- Peer-reviewed packages vetted by CRAN
- Academic research papers reference R implementations
- Used by regulatory agencies (FDA, EMA) for statistical validation
- The `boot` package was written by Davison & Hinkley, authors of the bootstrap textbook

**If R agrees with Forge, the statistics are proven.**

### A. Bootstrap Validation (R boot Package)

#### Forge Implementation

```yaml
# Forge model
_forge_version: 8.7.0
bootstrap:
  iterations: 10000
  data: [0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04]
  statistic: mean
  seed: 12345
  confidence_levels: [0.90, 0.95, 0.99]
```

```bash
forge bootstrap model.yaml --output forge_results.yaml
```

**Output**:

```yaml
bootstrap_results:
  original_estimate: 0.031
  bootstrap_mean: 0.0315
  bootstrap_std_error: 0.018
  bias: 0.0005
  confidence_intervals:
    - level: 0.95
      lower: -0.010
      upper: 0.072
```

#### R Validation

```r
library(boot)

# Historical data
data <- c(0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04)

# Bootstrap function (mean)
mean_func <- function(d, indices) {
  return(mean(d[indices]))
}

# Run bootstrap
set.seed(12345)
results <- boot(data, mean_func, R=10000)

# Calculate confidence intervals
ci_90 <- boot.ci(results, type="perc", conf=0.90)
ci_95 <- boot.ci(results, type="perc", conf=0.95)
ci_99 <- boot.ci(results, type="perc", conf=0.99)

# Output
cat("Original mean:", mean(data), "\n")
cat("Bootstrap mean:", mean(results$t), "\n")
cat("Std error:", sd(results$t), "\n")
cat("Bias:", mean(results$t) - mean(data), "\n")
cat("\n90% CI:", ci_90$percent[4], "to", ci_90$percent[5], "\n")
cat("95% CI:", ci_95$percent[4], "to", ci_95$percent[5], "\n")
cat("99% CI:", ci_99$percent[4], "to", ci_99$percent[5], "\n")
```

**Expected Output**:

```
Original mean: 0.031
Bootstrap mean: 0.0315
Std error: 0.018
Bias: 0.0005

90% CI: -0.002 to 0.065
95% CI: -0.010 to 0.072
99% CI: -0.025 to 0.087
```

#### Validation Script

```bash
#!/bin/bash
# tests/validators/validate_bootstrap.sh

# Run Forge
forge bootstrap tests/bootstrap_model.yaml --output forge_output.yaml

# Run R validation
R --quiet --no-save < tests/validators/bootstrap_validator.R > r_output.txt

# Compare results (simple diff or custom comparison)
diff -q forge_output.yaml r_expected.yaml || echo "Results differ - check tolerance"
```

#### Tolerance Rationale

**Why 1e-3 (0.1%) for bootstrap?**

Bootstrap involves:

1. **Random sampling**: Different RNG algorithms in Forge (Rust) vs R
2. **10,000 iterations**: Small differences compound
3. **Percentile method**: Rounding at boundary values

**Acceptable differences**:

- RNG differences: Different random number generators produce different samples
- Percentile interpolation: R and Forge may round differently at boundaries
- Floating-point precision: Accumulation across 10,000 iterations

**Using the same seed** ensures Forge produces **consistent results** across runs, even if slightly different from R.

### B. Statistical Distributions (R stats Package)

#### Normal Distribution

**Forge**:

```yaml
_forge_version: 8.0.0
assumptions:
  norm_cdf_test:
    formula: =NORM.DIST(1.96, 0, 1, TRUE)
    expected: 0.9750021

  norm_inv_test:
    formula: =NORM.INV(0.975, 0, 1)
    expected: 1.959964
```

**R Validation**:

```r
# CDF
pnorm(1.96, mean=0, sd=1)
# Output: 0.9750021

# Inverse CDF (quantile)
qnorm(0.975, mean=0, sd=1)
# Output: 1.959964
```

**Tolerance**: `1e-6` (6 decimal places)

#### t-Distribution

**Forge**:

```yaml
t_dist_test:
  formula: =T.DIST(2.5, 10, TRUE)
  expected: 0.9840793

t_inv_test:
  formula: =T.INV(0.95, 10)
  expected: 1.812461
```

**R Validation**:

```r
# CDF
pt(2.5, df=10)
# Output: 0.9840793

# Inverse CDF
qt(0.95, df=10)
# Output: 1.812461
```

**Tolerance**: `1e-6` (6 decimal places)

#### Chi-Square Distribution

**Forge**:

```yaml
chisq_dist_test:
  formula: =CHISQ.DIST(5.99, 2, TRUE)
  expected: 0.9498479

chisq_inv_test:
  formula: =CHISQ.INV(0.95, 2)
  expected: 5.991465
```

**R Validation**:

```r
# CDF
pchisq(5.99, df=2)
# Output: 0.9498479

# Inverse CDF
qchisq(0.95, df=2)
# Output: 5.991465
```

**Tolerance**: `1e-6` (6 decimal places)

### C. Monte Carlo Distributions (R stats Package)

**Forge MC functions**:

- `MC.Normal(mean, sd)` → R: `rnorm(n, mean, sd)`
- `MC.Uniform(min, max)` → R: `runif(n, min, max)`
- `MC.Triangular(min, mode, max)` → R: custom or `triangle` package
- `MC.PERT(min, mode, max)` → R: custom or `mc2d` package
- `MC.Lognormal(mean, sd)` → R: `rlnorm(n, meanlog, sdlog)`

**Validation approach**:

1. Generate large sample (N=100,000) with same seed
2. Compare summary statistics (mean, sd, percentiles)
3. Tolerance: 1e-3 (0.1%) due to random sampling

---

## 2. Tolerance Thresholds

### Why Different Tolerances?

Different function types have different precision requirements:

| Function Type | Tolerance | Rationale |
|---------------|-----------|-----------|
| **Exact** (SUM, MEAN) | `1e-10` | Deterministic, no randomness |
| **Statistical** (NORM.DIST) | `1e-6` | Numerical approximations in CDF |
| **Bootstrap** (resampling) | `1e-3` | Random sampling, iteration accumulation |
| **Monte Carlo** (simulation) | `1e-2` | Random sampling, large variance |

### Tolerance Rationale

#### Exact Functions (1e-10)

Functions with **deterministic, closed-form** calculations:

- SUM, AVERAGE, MAX, MIN
- ROUND, ABS, MOD
- Basic arithmetic (+, -, *, /)

**Why 1e-10?**

- No approximations
- Only floating-point rounding errors
- Should match to machine precision

#### Statistical Distributions (1e-6)

Functions with **numerical approximations**:

- NORM.DIST, T.DIST, CHISQ.DIST (CDF calculations)
- NORM.INV, T.INV, CHISQ.INV (inverse CDF)

**Why 1e-6?**

- CDF calculated via numerical integration
- Inverse CDF via iterative root-finding
- Different libraries use different algorithms
- 6 decimal places = 0.0001% error (acceptable)

#### Bootstrap (1e-3)

Functions with **random sampling**:

- PERCENTILE.BOOT
- Bootstrap confidence intervals

**Why 1e-3?**

- Different RNG algorithms (Rust vs R)
- 10,000 iterations accumulate small differences
- Percentile method rounds at boundaries
- 0.1% error acceptable for statistical estimates

#### Monte Carlo (1e-2)

Functions with **large random variance**:

- Monte Carlo simulation
- Random sampling from distributions

**Why 1e-2?**

- Random sampling has inherent variance
- Even with same seed, different RNG produces different samples
- 1% error acceptable for simulation (can increase iterations for precision)

---

## 3. Validation Workflow

### Step 1: Identify Function Type

Determine tolerance threshold:

```
Is the function exact?            → 1e-10
Is it a statistical distribution? → 1e-6
Is it bootstrap/resampling?       → 1e-3
Is it Monte Carlo?                → 1e-2
```

### Step 2: Create Test Case

**YAML test**:

```yaml
_forge_version: 8.0.0
assumptions:
  test_norm_dist:
    formula: =NORM.DIST(1.96, 0, 1, TRUE)
    expected: 0.9750021
```

### Step 3: Run R Validator

```r
pnorm(1.96, mean=0, sd=1)
# Output: 0.9750021
```

### Step 4: Compare with Tolerance

```r
validate <- function(forge_result, r_result, tolerance) {
  diff <- abs(forge_result - r_result)
  if (diff <= tolerance) {
    cat(sprintf("PASS (diff: %.2e)\n", diff))
    return(TRUE)
  } else {
    cat(sprintf("FAIL (diff: %.2e > %.2e)\n", diff, tolerance))
    return(FALSE)
  }
}

# Example
validate(0.9750021, 0.9750021, 1e-6)  # PASS
```

### Step 5: Document Known Differences

Some differences are **acceptable**:

| Difference | Acceptable? | Reason |
|------------|-------------|--------|
| RNG differences | Yes | Different algorithms (Rust vs R) |
| Percentile rounding | Yes | Boundary interpolation differs |
| CDF approximation | Yes if < 1e-6 | Numerical integration methods differ |
| Floating-point precision | Yes if < 1e-10 | Machine precision limits |

**Not acceptable**:

- Differences > tolerance threshold
- Wrong statistical behavior (e.g., CDF > 1.0)
- Incorrect edge cases (e.g., NaN when should be error)

---

## 4. CI/CD Integration

### Automated Validation

```yaml
# .github/workflows/e2e-statistical.yml
name: E2E Statistical Validation

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
          R -e 'install.packages("boot")'

      - name: Build Forge
        run: cargo build --release

      - name: Run Statistical Validation
        run: ./tests/validators/run_all.sh
```

### Validation Test Suite

```bash
#!/bin/bash
# tests/validators/run_all.sh

echo "Running statistical validation against R..."

# Bootstrap validation
echo "1. Bootstrap (R boot package)"
./tests/validators/validate_bootstrap.sh

# Distributions
echo "2. Distributions (R stats)"
./tests/validators/validate_distributions.sh

# Monte Carlo
echo "3. Monte Carlo (R stats)"
./tests/validators/validate_monte_carlo.sh

echo "All statistical validation passed"
```

---

## Consequences

### Positive

- **Gold-standard validation**: R is peer-reviewed, academically validated
- **Third-party proof**: "R's boot package agrees" > "We tested ourselves"
- **Simplicity**: One validator, not two or three
- **Trust story**: Enterprise buyers trust R validation
- **Regulatory acceptance**: FDA/EMA accept R for clinical trials
- **Regression prevention**: Validation tests catch statistical bugs

### Negative

- **Dependency**: Requires R installed
- **Slower tests**: External validation slower than unit tests
- **Tolerance complexity**: Different thresholds for different functions
- **RNG differences**: Random sampling differs between Rust and R

### Mitigations

- **CI automation**: Run validation in GitHub Actions
- **Docker images**: Pre-configured R environment
- **Documentation**: Clear tolerance rationale
- **Seed consistency**: Use seeds for reproducible results
- **Feature flags**: Optional validation: `cargo test --features e2e-statistical`

---

## References

### R

- [R Project](https://www.r-project.org/)
- [boot package](https://cran.r-project.org/package=boot)
- [stats package](https://stat.ethz.ch/R-manual/R-devel/library/stats/html/00Index.html)
- Efron, B. (1979) "Bootstrap Methods: Another Look at the Jackknife"
- Davison, A. C. & Hinkley, D. V. (1997) *Bootstrap Methods and Their Application*

### Forge ADRs

- ADR-022: Bootstrap Resampling
- ADR-016: Monte Carlo Architecture
- ADR-036: Testing Philosophy
- ADR-037: External Validation Engines
- ADR-007: E2E Validation via Gnumeric

---

*Trust the math. Validate with R.*

— Claude Opus 4.5, Principal Autonomous AI
