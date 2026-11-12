# R Statistical Validators

R validators for forge-e2e statistical functions. Part of v0.4.0 release.

## Overview

These validators compare forge's statistical functions against R's gold-standard implementations:
- **R** is the de facto standard for statistical computing
- Created by statisticians for statisticians
- Peer-reviewed packages vetted by CRAN
- Used by regulatory agencies (FDA, EMA)

## Validators

### 1. bootstrap_validator.R

Validates forge's `PERCENTILE.BOOT` function against R's `boot` package.

**Tests:**
- Bootstrap mean with 10,000 iterations
- Bootstrap median
- Bootstrap with larger datasets (N=100)
- Bootstrap standard deviation
- Confidence intervals (90%, 95%, 99%)

**Tolerance:** `1e-3` (0.1% - acceptable due to random sampling differences)

**Example:**
```bash
Rscript validators/r/bootstrap_validator.R
```

### 2. distribution_validator.R

Validates statistical distribution functions against R's `stats` package.

**Distributions:**
- **Normal:** `pnorm` (NORM.DIST), `qnorm` (NORM.INV)
- **t-distribution:** `pt` (T.DIST), `qt` (T.INV)
- **Chi-square:** `pchisq` (CHISQ.DIST), `qchisq` (CHISQ.INV)

**Tolerance:** `1e-6` (6 decimal places - standard for statistical functions)

**Example:**
```bash
Rscript validators/r/distribution_validator.R
```

### 3. monte_carlo_validator.R

Validates Monte Carlo distribution sampling against R's `stats` package.

**Distributions:**
1. Normal: `rnorm()`
2. Uniform: `runif()`
3. Lognormal: `rlnorm()`
4. Triangular: custom implementation
5. PERT: Beta-based implementation
6. Exponential: `rexp()`

**Validation Approach:**
- Generate large sample (N=100,000)
- Compare summary statistics (mean, sd, percentiles)
- Tolerance: `1e-3` (0.1% acceptable due to random sampling)

**Example:**
```bash
Rscript validators/r/monte_carlo_validator.R
```

## Usage

### Run All Validators

```bash
./validators/r/run_all.sh
```

**Output:**
```
========================================
  forge-e2e R Validators v0.4.0
========================================

Running R Validators...

================================================
Running: Bootstrap Validator
================================================
[PASS] Bootstrap Validator completed successfully

================================================
Running: Distribution Validator
================================================
[PASS] Distribution Validator completed successfully

================================================
Running: Monte Carlo Validator
================================================
[PASS] Monte Carlo Validator completed successfully

========================================
  Validation Summary
========================================

Total Validators:  3
Passed:            3
Failed:            0

✓ All validators passed!
```

### Run Individual Validators

```bash
# Bootstrap validation
Rscript validators/r/bootstrap_validator.R

# Distribution validation
Rscript validators/r/distribution_validator.R

# Monte Carlo validation
Rscript validators/r/monte_carlo_validator.R
```

## Installation

### macOS

```bash
# Install R
brew install r

# Install boot package
R -e 'install.packages("boot", repos="https://cloud.r-project.org/")'
```

### Ubuntu/Debian

```bash
# Install R
sudo apt-get update
sudo apt-get install -y r-base

# Install boot package
R -e 'install.packages("boot", repos="https://cloud.r-project.org/")'
```

### Fedora/RHEL

```bash
# Install R
sudo dnf install R

# Install boot package
R -e 'install.packages("boot", repos="https://cloud.r-project.org/")'
```

## Tolerance Rationale

Different function types require different precision tolerances:

| Function Type | Tolerance | Rationale |
|---------------|-----------|-----------|
| **Exact** (SUM, MEAN) | `1e-10` | Deterministic, no randomness |
| **Statistical** (NORM.DIST) | `1e-6` | Numerical approximations in CDF |
| **Bootstrap** (resampling) | `1e-3` | Random sampling, iteration accumulation |
| **Monte Carlo** (simulation) | `1e-3` | Random sampling, different RNG algorithms |

### Why Different RNG Results Are Acceptable

**Different Random Number Generators:**
- R uses the Mersenne Twister algorithm
- Rust uses different RNG implementations
- Even with the same seed, different algorithms produce different samples

**Statistical Validity:**
- If summary statistics match within tolerance, the distribution is correct
- Exact sample matching is neither expected nor required
- Focus on statistical properties, not individual samples

## CI/CD Integration

These validators run automatically in GitHub Actions:

```yaml
# .github/workflows/e2e.yml
jobs:
  r-validators:
    name: R Statistical Validators
    runs-on: ubuntu-latest
    steps:
      - name: Install R
        uses: r-lib/actions/setup-r@v2

      - name: Install R boot package
        run: R -e 'install.packages("boot")'

      - name: Run R validators
        run: ./validators/r/run_all.sh
```

## References

### R Documentation
- [R Project](https://www.r-project.org/)
- [boot package](https://cran.r-project.org/package=boot)
- [stats package](https://stat.ethz.ch/R-manual/R-devel/library/stats/html/00Index.html)

### forge-e2e ADRs
- [ADR-002: External Validation Engines](../../docs/architecture/ADR-002-EXTERNAL-VALIDATION-ENGINES.md)
- [ADR-004: Statistical Validation](../../docs/architecture/ADR-004-STATISTICAL-VALIDATION.md)

### Academic References
- Efron, B. (1979) "Bootstrap Methods: Another Look at the Jackknife"
- Davison, A. C. & Hinkley, D. V. (1997) *Bootstrap Methods and Their Application*

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.
