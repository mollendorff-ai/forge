# ADR-021: Tornado Diagrams (Sensitivity Analysis)

## Status

**Implemented** - v8.6.0

## Context

Monte Carlo simulation shows the full distribution of outcomes, but FP&A analysts need to answer:

- "Which inputs drive the most variance in our results?"
- "Where should we focus our due diligence?"
- "Is the model behaving as expected?"
- "Which assumptions matter most to refine?"

Tornado diagrams (one-at-a-time sensitivity analysis) answer these questions visually.

## Decision

**Forge will implement Tornado Diagrams for identifying key value drivers.**

### YAML Syntax

```yaml
_forge_version: "8.6.0"

# Base model scalars
scalars:
  revenue:
    value: 1000000
  revenue_growth:
    value: 0.05
  discount_rate:
    value: 0.10
  operating_margin:
    value: 0.20
  tax_rate:
    value: 0.25

  projected_revenue:
    formula: "=revenue * (1 + revenue_growth)"
  operating_profit:
    formula: "=projected_revenue * operating_margin"
  net_profit:
    formula: "=operating_profit * (1 - tax_rate)"
  npv:
    formula: "=net_profit / discount_rate"

# Tornado analysis configuration
tornado:
  output: npv
  inputs:
    - name: revenue_growth
      low: 0.02
      high: 0.08
      base: 0.05

    - name: discount_rate
      low: 0.08
      high: 0.12
      base: 0.10

    - name: operating_margin
      low: 0.15
      high: 0.25
      base: 0.20

    - name: tax_rate
      low: 0.20
      high: 0.30
      base: 0.25

  steps: 2  # Number of steps between low/high (default: 2)
```

### CLI Commands

```bash
# Run tornado analysis
forge tornado model.yaml

# Export to YAML
forge tornado model.yaml --output results.yaml

# Export to JSON
forge tornado model.yaml --format json

# Show top N drivers only
forge tornado model.yaml --top 5

# Export to CSV for Excel
forge tornado model.yaml --format csv > tornado.csv
```

### Output Format

#### ASCII Visualization

```
npv Sensitivity (Base: $1,500,000)

revenue_growth       |████████████████████| +/- $450,000
discount_rate        |██████████████      | +/- $320,000
operating_margin     |██████████          | +/- $180,000
tax_rate             |████                | +/- $75,000
```

#### YAML Output

```yaml
output: npv
base_value: 1500000
bars:
  - input_name: revenue_growth
    output_at_low: 1200000    # npv when revenue_growth = 0.02
    output_at_high: 1650000   # npv when revenue_growth = 0.08
    swing: 450000
    abs_swing: 450000
    input_low: 0.02
    input_high: 0.08

  - input_name: discount_rate
    output_at_low: 1660000    # npv when discount_rate = 0.08
    output_at_high: 1340000   # npv when discount_rate = 0.12
    swing: -320000
    abs_swing: 320000
    input_low: 0.08
    input_high: 0.12

  # ... more bars sorted by abs_swing

total_variance: 1025000
```

### Analysis Workflow

1. **Define baseline model** with default values
2. **Specify input ranges** (low/high for each variable)
3. **Run one-at-a-time**: Vary each input while holding others constant
4. **Calculate swing**: Difference in output between low and high input
5. **Sort by impact**: Display bars from largest to smallest swing
6. **Identify drivers**: Top 2-3 inputs typically explain 70-80% of variance

## Rationale

### Why Tornado Diagrams?

1. **Visual clarity**: Executives instantly see what matters
2. **Model validation**: Unexpected sensitivities indicate errors
3. **Prioritize effort**: Focus refinement on high-impact inputs
4. **Communication**: Simpler than full Monte Carlo for board presentations
5. **Due diligence**: Guides where to spend time gathering data

### Why This Syntax?

1. **YAML-native**: Consistent with Forge philosophy
2. **Declarative**: Specify what to analyze, not how
3. **Composable**: Works with existing scalar models
4. **Git-friendly**: Sensitivity configs are diffable
5. **Flexible ranges**: Support expert estimates or quantiles

## Consequences

### Positive
- Identifies key value drivers quickly
- Validates model assumptions
- Guides data collection priorities
- Board-friendly visualization
- Complements Monte Carlo analysis

### Negative
- One-at-a-time (doesn't capture interactions)
- Assumes linear or monotonic relationships
- Requires range estimates for inputs
- Not probabilistic (use Monte Carlo for distributions)

### Mitigations
- Document that tornado shows univariate sensitivity only
- Combine with Monte Carlo for interaction effects
- Use correlation analysis for dependent inputs
- Validate against domain expertise

## Alternatives Considered

### Multi-way Sensitivity

Test all combinations of inputs simultaneously.

**Rejected**: Combinatorial explosion (10 inputs × 3 values each = 59,049 runs). One-at-a-time sufficient for identifying drivers.

### Global Sensitivity Analysis

Sobol indices, FAST method, etc.

**Rejected**: Overkill for most FP&A use cases. Tornado diagrams provide 80% of value with 20% of complexity.

### External Tools Only

Use Excel Data Tables or @RISK sensitivity.

**Rejected**: Breaks YAML-native workflow, not git-friendly.

## Implementation Notes

1. **Parse config**: Extract output variable and input ranges
2. **Validate**: Ensure output exists, inputs are valid scalars
3. **Calculate baseline**: Run model with base values
4. **For each input**:
   - Set input to low value, calculate output
   - Set input to high value, calculate output
   - Calculate swing = high_output - low_output
5. **Sort bars**: Order by absolute swing (largest first)
6. **Generate output**: ASCII, YAML, JSON, or CSV format
7. **Optional**: Calculate variance explained percentages

### Engine Architecture

```rust
pub struct TornadoEngine {
    config: TornadoConfig,
    base_model: ParsedModel,
}

impl TornadoEngine {
    pub fn analyze(&self) -> Result<TornadoResult, String> {
        // 1. Calculate base case
        // 2. For each input, calculate sensitivity
        // 3. Sort by impact
        // 4. Return structured result
    }
}
```

## Roundtrip Validation

Tornado Diagram results are validated against **R's sensitivity package** (standard for sensitivity analysis).

### Validation Tool

```bash
# Setup (one-time)
./tests/validators/setup.sh

# R validation script
R --quiet -e '
  library(sensitivity)

  # Define model
  model <- function(X) {
    revenue <- X[1]
    cost_rate <- X[2]
    tax_rate <- X[3]
    profit <- revenue * (1 - cost_rate) * (1 - tax_rate)
    return(profit)
  }

  # One-at-a-time sensitivity
  # Revenue: [800K, 1200K]
  # Cost rate: [0.50, 0.70]
  # Tax rate: [0.20, 0.30]

  base <- c(1000000, 0.60, 0.25)
  base_profit <- model(base)

  # Sensitivity: Revenue
  low_revenue <- model(c(800000, 0.60, 0.25))
  high_revenue <- model(c(1200000, 0.60, 0.25))
  revenue_swing <- high_revenue - low_revenue

  cat("Base profit:", base_profit, "\n")
  cat("Revenue swing:", revenue_swing, "\n")
  # Expected: Base = $300,000, Swing = $120,000
'
```

### Test Coverage

| Test | Validation |
|------|------------|
| One-at-a-time calculation | R manual calculation |
| Sorting by impact | Unit test |
| Variance explained | E2E test |
| Multi-input analysis | R sensitivity package |

### Known Differences

Forge and R may differ slightly due to:
- **Floating-point precision**: Acceptable within 0.01%
- **Sorting ties**: Consistent within Forge, may differ from R

These differences are documented and acceptable for FP&A use.

## References

- Saltelli, A. et al. (2008). *Global Sensitivity Analysis: The Primer*. Wiley.
- Hamby, D. M. (1994). "A Review of Techniques for Parameter Sensitivity Analysis." *Environmental Monitoring*.
- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- R sensitivity package: https://cran.r-project.org/package=sensitivity
