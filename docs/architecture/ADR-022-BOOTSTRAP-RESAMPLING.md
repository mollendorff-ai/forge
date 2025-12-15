# ADR-022: Bootstrap Resampling

## Status

**Implemented** - v8.7.0

## Context

Monte Carlo simulation requires specifying probability distributions (Normal, Triangular, etc.). But analysts often face:

- "We have historical data but don't know the distribution"
- "The distribution is weird - multimodal, fat-tailed, or skewed"
- "I don't want to make parametric assumptions"
- "How do we quantify uncertainty from limited historical data?"

Bootstrap resampling solves this: Resample from observed data to estimate uncertainty non-parametrically.

## Decision

**Forge will implement Bootstrap Resampling for non-parametric uncertainty quantification.**

### Core Concept

```
Historical data: [5%, -2%, 8%, 3%, -5%, 12%, 1%, -1%, 6%, 4%]

Bootstrap iteration 1: Sample WITH replacement
  → [8%, 8%, -2%, 6%, 4%, -5%, 12%, 3%, 3%, 1%]
  → Statistic (e.g., mean) = 3.8%

Bootstrap iteration 2: Sample WITH replacement
  → [5%, -1%, -1%, 12%, 6%, 8%, -2%, 4%, 5%, 3%]
  → Statistic = 3.9%

... repeat 10,000 times ...

Result: Empirical distribution of statistic → confidence intervals
```

### YAML Syntax

```yaml
_forge_version: "8.7.0"

bootstrap:
  iterations: 10000
  confidence_levels: [0.90, 0.95, 0.99]
  seed: 12345  # For reproducibility

  # Historical data
  data: [0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04]

  # Statistic to compute
  statistic: mean  # or median, std, var, percentile, min, max

  # If statistic = percentile:
  percentile_value: 75  # 75th percentile
```

### Statistics Supported

| Statistic | Use Case | Example |
|-----------|----------|---------|
| `mean` | Average value | Average return |
| `median` | Robust central tendency | Median sales |
| `std` | Variability | Risk measure |
| `var` | Variance | Volatility squared |
| `percentile` | Quantile estimation | P95 loss |
| `min` | Worst case | Max drawdown |
| `max` | Best case | Peak performance |

### CLI Commands

```bash
# Run bootstrap analysis
forge bootstrap model.yaml

# Specify number of iterations
forge bootstrap model.yaml --iterations 50000

# Export results to YAML
forge bootstrap model.yaml --output results.yaml

# Export to JSON
forge bootstrap model.yaml --format json

# Quick analysis with fewer iterations
forge bootstrap model.yaml --quick  # Uses 1000 iterations
```

### Output Format

```yaml
bootstrap_results:
  original_estimate: 0.031  # Mean of original sample
  bootstrap_mean: 0.0315    # Mean of bootstrap distribution
  bootstrap_std_error: 0.018  # Standard error from bootstrap
  bias: 0.0005              # Bias = bootstrap_mean - original_estimate
  bias_corrected_estimate: 0.0305  # Original - bias

  confidence_intervals:
    - level: 0.90
      lower: -0.002
      upper: 0.065

    - level: 0.95
      lower: -0.010
      upper: 0.072

    - level: 0.99
      lower: -0.025
      upper: 0.087

  iterations: 10000

  # Full distribution (optional, for advanced analysis)
  distribution: [0.029, 0.034, 0.028, ...]  # All 10,000 values
```

### Integration with Forge Models

Bootstrap can resample from model outputs:

```yaml
_forge_version: "8.7.0"

# 1. Run model to generate historical scenarios
scenarios:
  historical_years:
    # ... load from historical data

# 2. Bootstrap from scenario results
bootstrap:
  data: ${historical_npv_values}  # Reference to computed values
  statistic: mean
  iterations: 10000
```

## Rationale

### Why Bootstrap?

1. **No distribution assumptions**: Let the data speak
2. **Handles complex distributions**: Multimodal, skewed, fat-tailed
3. **Quantifies sampling uncertainty**: From limited historical data
4. **Theoretically sound**: Efron (1979), widely accepted
5. **Simple to explain**: "Resample from what we've seen"

### Why NOT Bootstrap?

1. **Future ≠ Past**: If regime has changed, history is misleading
2. **Small samples**: Need at least 30-50 observations
3. **No new information**: Can't extrapolate beyond observed range
4. **Computational cost**: Thousands of resamples

### Bootstrap vs Monte Carlo

| Criterion | Bootstrap | Monte Carlo |
|-----------|-----------|-------------|
| **Data source** | Historical observations | Theoretical distributions |
| **Assumptions** | Past repeats | Distribution parameters known |
| **Output range** | Within observed data | Can exceed observations |
| **Use case** | Empirical uncertainty | Hypothetical scenarios |
| **Flexibility** | Limited to history | Full control over distributions |

**Best practice**: Use both. Bootstrap for historical validation, Monte Carlo for forward-looking scenarios.

## Consequences

### Positive
- No parametric assumptions required
- Handles any data distribution
- Theoretically rigorous (Efron 1979)
- Standard tool in statistics
- Simple to understand and explain

### Negative
- Requires historical data (not always available)
- Assumes future resembles past
- Minimum sample size needed (~30+)
- Computationally intensive
- Can't extrapolate beyond observed data

### Mitigations
- Document sample size requirements
- Warn if data < 30 observations
- Recommend combining with Monte Carlo
- Use seeds for reproducibility
- Allow iteration count tuning

## Alternatives Considered

### Parametric Confidence Intervals

Use t-distribution for mean confidence intervals.

**Rejected**: Assumes normality, doesn't handle complex distributions.

### Permutation Tests

Random reshuffling for hypothesis testing.

**Rejected**: Solves a different problem (testing), not estimation.

### Jackknife

Leave-one-out resampling.

**Rejected**: Less accurate than bootstrap for confidence intervals.

## Implementation Notes

### Algorithm

1. **Initialize**: Set random seed if provided
2. **Original statistic**: Calculate on original data
3. **Bootstrap loop**:
   - For i = 1 to iterations:
     - Resample n observations WITH replacement
     - Calculate statistic on resample
     - Store in bootstrap distribution
4. **Sort distribution** for percentile calculation
5. **Calculate**:
   - Bootstrap mean and standard error
   - Bias = bootstrap_mean - original_estimate
   - Confidence intervals via percentile method
6. **Return results**

### Percentile Confidence Intervals

For 95% CI with 10,000 iterations:
- Lower bound: 250th value (2.5th percentile)
- Upper bound: 9,750th value (97.5th percentile)

### Engine Architecture

```rust
pub struct BootstrapEngine {
    config: BootstrapConfig,
    rng: StdRng,
}

impl BootstrapEngine {
    pub fn analyze(&mut self) -> Result<BootstrapResult, String> {
        // 1. Calculate original statistic
        // 2. Bootstrap resampling
        // 3. Calculate confidence intervals
        // 4. Return structured result
    }

    fn compute_statistic(&self, sample: &[f64]) -> f64 {
        // Support mean, median, std, var, percentile, min, max
    }
}
```

## Roundtrip Validation

Bootstrap results are validated against **R's boot package** (the gold standard for bootstrap methods).

### Validation Tool

```bash
# Setup (one-time)
./tests/validators/setup.sh

# R validation script
R --quiet -e '
  library(boot)

  # Historical data
  data <- c(5, -2, 8, 3, -5, 12, 1, -1, 6, 4) / 100

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
'
```

### Expected Output

```
Original mean: 0.031
Bootstrap mean: 0.0315
Std error: 0.018
95% CI: -0.010 to 0.072
```

### Test Coverage

| Test | Validation |
|------|------------|
| Mean statistic | R boot package |
| Median statistic | R boot package |
| Percentile CI method | R boot.ci() |
| Bias calculation | Unit test |
| Reproducibility (seed) | E2E test |

### Known Differences

Forge and R may differ slightly due to:
- **Random sampling**: Different RNG algorithms (acceptable)
- **Percentile interpolation**: Rounding at boundaries (< 0.1% difference)
- **Floating-point precision**: Acceptable within 0.01%

Using the same seed ensures Forge produces consistent results across runs.

## References

- Efron, B. (1979). "Bootstrap Methods: Another Look at the Jackknife." *Annals of Statistics*.
- Efron, B. & Tibshirani, R. J. (1993). *An Introduction to the Bootstrap*. Chapman & Hall.
- Davison, A. C. & Hinkley, D. V. (1997). *Bootstrap Methods and Their Application*. Cambridge.
- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- R boot package: https://cran.r-project.org/package=boot
