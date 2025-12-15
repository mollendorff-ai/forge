# ADR-018: Scenario Analysis

## Status

**Planned** - Target v8.4.0

## Context

Monte Carlo simulation handles continuous uncertainty well, but FP&A often requires discrete scenario modeling:

- "What if we win vs lose the contract?"
- "Base case / Bull case / Bear case"
- "Recession vs growth vs stagnation"

These are mutually exclusive outcomes with assigned probabilities, not continuous distributions.

## Decision

**Forge will implement Scenario Analysis as a complement to Monte Carlo.**

### YAML Syntax

```yaml
_forge_version: "8.4.0"

scenarios:
  base_case:
    probability: 0.50
    description: "Market grows 5%, we maintain share"
    scalars:
      revenue_growth: 0.05
      market_share: 0.15

  bull_case:
    probability: 0.30
    description: "Competitor exits, we capture share"
    scalars:
      revenue_growth: 0.15
      market_share: 0.25

  bear_case:
    probability: 0.20
    description: "Recession hits, market contracts"
    scalars:
      revenue_growth: -0.10
      market_share: 0.12

# Base model uses scenario values
scalars:
  base_revenue:
    value: 10000000
  projected_revenue:
    formula: "=base_revenue * (1 + revenue_growth)"
```

### Combined with Monte Carlo

Scenarios can contain Monte Carlo distributions for continuous uncertainty within discrete outcomes:

```yaml
scenarios:
  win_contract:
    probability: 0.60
    scalars:
      contract_revenue:
        formula: "=MC.Normal(5000000, 500000)"

  lose_contract:
    probability: 0.40
    scalars:
      contract_revenue:
        formula: "=MC.Normal(1000000, 200000)"
```

### CLI Commands

```bash
# Run all scenarios
forge scenarios model.yaml

# Run specific scenario
forge scenarios model.yaml --scenario bull_case

# Compare scenarios
forge scenarios model.yaml --compare

# Expected value across scenarios
forge scenarios model.yaml --expected-value
```

### Output Format

```yaml
scenario_results:
  base_case:
    probability: 0.50
    npv: 1250000
    irr: 0.142

  bull_case:
    probability: 0.30
    npv: 2100000
    irr: 0.198

  bear_case:
    probability: 0.20
    npv: -450000
    irr: 0.032

  expected_value:
    npv: 1015000  # 0.5*1.25M + 0.3*2.1M + 0.2*(-0.45M)
    probability_positive_npv: 0.80
```

## Rationale

### Why Add Scenarios?

1. **Board communication**: Executives think in scenarios, not distributions
2. **Strategic planning**: Discrete futures require discrete analysis
3. **Stress testing**: Regulatory requirements often specify scenarios
4. **Complements Monte Carlo**: Different tool for different uncertainty

### Why This Syntax?

1. **Probability-weighted**: Matches how analysts think
2. **Composable**: Can use Monte Carlo within scenarios
3. **YAML-native**: Consistent with Forge philosophy
4. **Git-friendly**: Scenarios are diffable text

## Consequences

### Positive
- Covers discrete uncertainty gap in current tooling
- Board-friendly output format
- Natural integration with Monte Carlo
- Enables regulatory stress testing

### Negative
- Scenarios must be mutually exclusive (limitation)
- Probabilities must sum to 1.0 (validation needed)
- More complex model files

## Alternatives Considered

### Scenario as Separate Files

```bash
forge calculate base_case.yaml
forge calculate bull_case.yaml
forge calculate bear_case.yaml
```

**Rejected**: No expected value calculation, harder to maintain consistency.

### Scenario as CLI Override

```bash
forge calculate model.yaml --set revenue_growth=0.05
```

**Rejected**: Doesn't capture probability weighting, scattered logic.

## Implementation Notes

1. Validate probabilities sum to 1.0 (within tolerance)
2. Run each scenario independently
3. For MC+Scenarios: Run full Monte Carlo within each scenario
4. Report per-scenario and expected-value results
5. Excel export: One sheet per scenario + summary sheet

## Roundtrip Validation

Scenario Analysis results are validated against **R** (the gold standard for statistical computing).

### Validation Tool

```bash
# Setup (one-time)
./tests/validators/setup.sh

# R validation script
R --quiet -e '
  # Forge scenario results
  scenarios <- c("base", "bull", "bear")
  probabilities <- c(0.50, 0.30, 0.20)
  npv_values <- c(1250000, 2100000, -450000)

  # Expected value calculation
  ev <- weighted.mean(npv_values, probabilities)
  cat("Expected NPV:", ev, "\n")
  # Expected: 1015000
'
```

### Test Coverage

| Test | Validation |
|------|------------|
| Probability sum = 1.0 | Unit test |
| Expected value calculation | R `weighted.mean()` |
| Scenario isolation | E2E test |
| MC within scenarios | E2E + MC validator |

## References

- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- ADR-017: Monte Carlo Sequential Execution
- R Project: https://www.r-project.org/
