# ADR-037: External Validation Engines

**Status:** Accepted
**Date:** 2025-12-17
**Updated:** 2025-12-17 (Two-tier validation strategy documented)
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

> **Note (2026-01-02):** Implementation split into dedicated repositories:
>
> - [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) - Gnumeric validation
> - [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r) - R validation

---

## Context

Forge calculates financial and statistical formulas. **How do we prove the calculations are correct?**

Unit tests are self-validating: "We tested ourselves and we're great." Enterprise buyers need **third-party proof**.

## Decision

**Forge-e2e validates calculations against two external engines:**

1. **Gnumeric (ssconvert)**: Primary validation for Excel functions
2. **R**: Statistical function validation (boot package, stats) - the gold standard

Each engine provides independent verification that Forge calculates correctly.

### Why Not Python?

Python (scipy/numpy) was considered but **removed** because:

- R is the gold standard for statistical computing
- scipy implements the same algorithms as R (often ported from R)
- Adding Python provides no additional authority - it's redundant
- Fewer dependencies = simpler maintenance

**If R agrees with Forge, we're done.**

---

## 1. Gnumeric (Primary Validation Engine)

### What is Gnumeric?

[Gnumeric](https://www.gnumeric.org/) is the GNOME spreadsheet application:

- Open-source, battle-tested since 2001
- Academically validated for numerical accuracy (McCullough 2004, 2005)
- Developers cooperate with the R Project for statistical accuracy
- Implements Excel-compatible formulas independently

### Why Gnumeric?

| Feature | Gnumeric | Excel | LibreOffice |
|---------|----------|-------|-------------|
| **Numerical accuracy** | ✅ Academically validated | ✅ Industry standard | ❌ Snap-to-zero errors |
| **Honest arithmetic** | ✅ No snap-to-zero | ✅ Accurate | ❌ Aggressive rounding |
| **Open source** | ✅ Yes | ❌ No | ✅ Yes |
| **Programmatic access** | ✅ ssconvert CLI | ❌ Requires automation | ⚠️ Complex API |
| **Cross-platform** | ✅ Linux, macOS, Windows | ⚠️ Windows/macOS only | ✅ All platforms |

### ssconvert Tool

Gnumeric provides `ssconvert` for batch conversions:

```bash
# Convert XLSX to CSV
ssconvert input.xlsx output.csv

# Extract specific sheet
ssconvert -S input.xlsx output.csv

# Recalculate formulas
ssconvert --recalc input.xlsx output.xlsx
```

**E2E Test Flow**:

```
1. Forge creates YAML model with formulas
2. Forge calculates and exports to XLSX
3. ssconvert recalculates XLSX → CSV
4. Test compares Forge result vs CSV result
5. Pass if identical within tolerance
```

### Installation

```bash
# macOS
brew install gnumeric

# Ubuntu/Debian
sudo apt install gnumeric

# Verify
ssconvert --version
```

### Coverage

Gnumeric validates **173 Excel functions**:

| Category | Functions |
|----------|-----------|
| **Financial** | NPV, IRR, PMT, PV, FV, NPER, RATE, XNPV, XIRR |
| **Math** | SUM, AVERAGE, MAX, MIN, ROUND, POWER, MOD, ABS |
| **Date** | DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH |
| **Statistical** | STDEV, VAR, MEDIAN, PERCENTILE, QUARTILE |
| **Text** | CONCATENATE, TRIM, LEN, LEFT, RIGHT, MID |
| **Logical** | IF, AND, OR, NOT, IFERROR |
| **Total** | **173 functions** |

---

## 2. R (Statistical Validation - Gold Standard)

### Why R?

R is the **gold standard** for statistical computing:

- Created by statisticians for statisticians
- Used by academics and researchers worldwide
- Peer-reviewed packages vetted by CRAN
- Regulatory acceptance (FDA/EMA accept R for clinical trials)
- The `boot` package was written by the authors of the bootstrap textbook

### Key Packages

#### boot Package (Bootstrap Validation)

Validates Forge's Bootstrap implementation:

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

# Confidence intervals
ci <- boot.ci(results, type="perc", conf=0.95)

# Output
cat("Original mean:", mean(data), "\n")
cat("Bootstrap mean:", mean(results$t), "\n")
cat("Std error:", sd(results$t), "\n")
cat("95% CI:", ci$percent[4], "to", ci$percent[5], "\n")
```

**Validation**: Compare R's `boot()` output with Forge's `PERCENTILE.BOOT()` function.

#### stats Package (Statistical Distributions)

Validates statistical functions:

```r
# Normal distribution
pnorm(1.96, mean=0, sd=1)  # CDF
qnorm(0.975, mean=0, sd=1)  # Quantile

# t-distribution
pt(2.5, df=10)
qt(0.95, df=10)

# Chi-square
pchisq(5.99, df=2)
qchisq(0.95, df=2)
```

### Installation

```bash
# Install R
brew install r  # macOS
sudo apt install r-base  # Ubuntu

# Install packages
R -e 'install.packages("boot")'
R -e 'install.packages("MASS")'
```

### Coverage

| Forge Feature | R Package | Functions Validated |
|---------------|-----------|---------------------|
| Bootstrap resampling | boot | PERCENTILE.BOOT |
| Statistical distributions | stats | NORM.DIST, T.DIST, CHISQ.DIST |
| Hypothesis tests | stats | T.TEST, CHISQ.TEST |
| Monte Carlo | stats | Distribution sampling |

---

## Why NOT LibreOffice?

### The Snap-to-Zero Problem

LibreOffice uses **aggressive "snap-to-zero"** that **hides numerical errors**:

> "The lack of an honest subtraction operation makes it hard to even test the accuracy of sheet-level functions. LibreOffice and OpenOffice will claim that a result matches a reference value even when they do not."
>
> — [Gnumeric Numerical Issues](http://www.gnumeric.org/numerical-issues.html)

### Comparison

| Behavior | Gnumeric | LibreOffice |
|----------|----------|-------------|
| **Snap-to-zero** | No | Yes, on EVERY subtraction |
| **Internal functions (VAR, etc.)** | Honest arithmetic | Snap-to-zero internally |
| **Accuracy validation** | McCullough papers | Not academically validated |
| **Result: `1.0 - 0.9999999999999999`** | `1.11e-16` (correct) | `0.0` (snapped) |

**Decision**: Use Gnumeric for honest arithmetic and accurate validation.

---

## Two-Tier Validation Strategy

Forge-e2e validates **173 functions** using a two-tier approach:

- **Tier 1 (Gnumeric)**: ~120 Excel-compatible functions
- **Tier 2 (R)**: ~50 forge-native statistical and financial functions

Each tier uses the most appropriate validator for the function category.

### Tier 1: Gnumeric Validation (~120 Functions)

**Target**: Excel-compatible functions that have direct Gnumeric equivalents

**Function Categories**:

| Category | Example Functions | Count |
|----------|-------------------|-------|
| **Math** | ABS, SQRT, ROUND, FLOOR, CEILING, POWER, MOD, EXP, LN, LOG | ~25 |
| **Text** | CONCAT, LEFT, RIGHT, MID, UPPER, LOWER, TRIM, LEN, FIND | ~15 |
| **Date** | DATE, YEAR, MONTH, DAY, TODAY, DATEDIF, EDATE, EOMONTH | ~15 |
| **Logical** | IF, AND, OR, NOT, IFERROR, IFNA | ~8 |
| **Lookup** | VLOOKUP, HLOOKUP, INDEX, MATCH, OFFSET | ~8 |
| **Aggregation** | SUM, AVERAGE, COUNT, MIN, MAX, MEDIAN, STDEV, VAR | ~20 |
| **Financial (Basic)** | NPV, IRR, PMT, PV, FV, NPER, RATE, XNPV, XIRR | ~15 |
| **Array/Reference** | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS | ~14 |
| **Total** | | **~120** |

**Validation Method**:

```
Forge YAML → Forge XLSX → ssconvert recalc → CSV → Compare
```

**Why Gnumeric for Tier 1?**

- Excel-compatible function implementations
- Academically validated numerical accuracy (McCullough 2004, 2005)
- 20+ years of edge case fixes
- No snap-to-zero errors (unlike LibreOffice)

### Tier 2: R Validation (~50 Functions)

**Target**: Forge-native statistical and financial functions that have no Excel equivalent or require specialized statistical validation

**Function Categories**:

| Category | Example Functions | R Package | Count |
|----------|-------------------|-----------|-------|
| **Statistical Distributions** | NORM.DIST, T.DIST, CHISQ.DIST, F.DIST, BETA.DIST | stats | ~15 |
| **Bootstrap Methods** | PERCENTILE.BOOT, MEAN.BOOT, STDEV.BOOT | boot | ~5 |
| **Monte Carlo** | MC.Normal, MC.Uniform, MC.Triangular, MC.Beta | stats | ~10 |
| **Financial Analytics** | BREAKEVEN_UNITS, CONTRIBUTION_MARGIN, VARIANCE_ANALYSIS | custom | ~12 |
| **Hypothesis Tests** | T.TEST, CHISQ.TEST, ANOVA | stats | ~8 |
| **Total** | | | **~50** |

**Validation Method**:

```
Forge calculates → R script calculates → Compare numerical results
```

**Why R for Tier 2?**

- Gold standard for statistical computing
- Peer-reviewed packages (CRAN)
- Regulatory acceptance (FDA/EMA)
- Bootstrap package by textbook authors
- No Excel equivalent for comparison

### Decision Matrix: Which Validator for Which Function?

| Function Type | Validator | Rationale |
|---------------|-----------|-----------|
| **Excel-compatible** (SUM, IF, VLOOKUP) | Gnumeric | Direct Excel equivalents, ssconvert handles XLSX |
| **Statistical distributions** (NORM.DIST, T.DIST) | R | Gold standard, peer-reviewed stats package |
| **Bootstrap resampling** (PERCENTILE.BOOT) | R | boot package by bootstrap textbook authors |
| **Monte Carlo** (MC.Normal, MC.Uniform) | R | Best random number generation and sampling |
| **Forge-native financial** (BREAKEVEN_UNITS) | R | Custom calculations, R for numerical accuracy |
| **Complex edge cases** (DATE boundaries) | Gnumeric | 20+ years of edge case fixes |

**Rule of Thumb**:

- If Excel has it → Use Gnumeric (Tier 1)
- If it's statistical/Monte Carlo → Use R (Tier 2)
- If forge-native financial → Use R (Tier 2)

### Validation Flow

```
┌─────────────────────────────────────────────────┐
│ Forge Calculation (173 functions)               │
│ (YAML model → formula evaluation)               │
└────────────────┬────────────────────────────────┘
                 │
         ┌───────┴───────────┐
         │                   │
         ▼                   ▼
┌──────────────────┐  ┌──────────────────────┐
│ TIER 1: Gnumeric │  │ TIER 2: R            │
│ (ssconvert)      │  │ (gold standard)      │
│                  │  │                      │
│ ~120 functions:  │  │ ~50 functions:       │
│ • Excel-compat   │  │ • Statistical        │
│ • Math/Text/Date │  │ • Bootstrap          │
│ • Lookup/Logical │  │ • Monte Carlo        │
│ • Basic Finance  │  │ • Forge Financial    │
└────────┬─────────┘  └──────────┬───────────┘
         │                       │
         └──────┬────────────────┘
                │
                ▼
         ┌────────────┐
         │ Compare    │
         │ Results    │
         │            │
         │ Pass if    │
         │ identical  │
         │ (±1e-10)   │
         └────────────┘
```

### Cross-References

This two-tier strategy is implemented in detail by:

- **ADR-039: Statistical Validation** - R validation for distributions, bootstrap, Monte Carlo
- **ADR-040: Financial Analytics Validation** - R validation for forge-native financial functions
- **ADR-036: Testing Philosophy** - Overall e2e testing approach

## Consequences

### Positive

- **Third-party validation**: "Gnumeric and R agree" > "We tested ourselves"
- **Academic rigor**: R is peer-reviewed and the gold standard
- **Edge case coverage**: Gnumeric has 20+ years of fixes
- **Trust story**: Enterprise buyers trust external validation
- **Simplicity**: Two validators, not three

### Negative

- **Dependencies**: Requires Gnumeric and R installed
- **Platform constraints**: Gnumeric best on Linux/macOS
- **Speed**: External validation slower than unit tests

### Mitigations

- Feature flags: `cargo test --features e2e-gnumeric`
- CI matrix: Test on Linux, macOS
- Docker images: Pre-configured environments
- Documentation: Clear installation instructions

## References

### Gnumeric

- [Gnumeric Official Site](https://www.gnumeric.org/)
- [Gnumeric Numerical Issues](http://www.gnumeric.org/numerical-issues.html)
- McCullough BD (2004) "Fixing Statistical Errors in Spreadsheet Software"
- McCullough BD, Wilson B (2005) "On the accuracy of statistical procedures in Microsoft Excel 2003"

### R

- [R Project](https://www.r-project.org/)
- [boot package](https://cran.r-project.org/package=boot)
- Efron, B. (1979) "Bootstrap Methods: Another Look at the Jackknife"
- Davison, A. C. & Hinkley, D. V. (1997) *Bootstrap Methods and Their Application*

### Related ADRs

- **ADR-036: Testing Philosophy** - Overall e2e testing approach
- **ADR-039: Statistical Validation** - Tier 2 R validation for distributions, bootstrap, Monte Carlo
- **ADR-040: Financial Analytics Validation** - Tier 2 R validation for forge-native financial functions
- ADR-007: E2E Validation via Gnumeric - Original Gnumeric validation ADR
- ADR-022: Bootstrap Resampling - Bootstrap implementation details

---

*Don't trust us. Trust Gnumeric and R.*

— Claude Opus 4.5, Principal Autonomous AI
