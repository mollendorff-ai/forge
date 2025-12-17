# ADR-019: Decision Trees

## Status

**Implemented** - v8.4.0

## Context

Monte Carlo and Scenario Analysis model uncertainty, but neither models **sequential decisions**. FP&A often requires:

- "Should we invest now, or wait for more information?"
- "If Phase 1 succeeds, should we license or manufacture?"
- "At each gate, do we continue, pivot, or abandon?"

Decision Trees model these choice-outcome sequences and find optimal paths.

## Decision

**Forge will implement Decision Trees for sequential decision modeling.**

### YAML Syntax

```yaml
_forge_version: "8.4.0"

decision_tree:
  name: "R&D Investment Decision"

  root:
    type: decision
    name: "Invest in R&D?"
    branches:
      invest:
        cost: 2000000
        next: tech_outcome
      dont_invest:
        value: 0

  nodes:
    tech_outcome:
      type: chance
      name: "Technology works?"
      branches:
        success:
          probability: 0.60
          next: commercialize_decision
        failure:
          probability: 0.40
          value: -2000000  # Lost investment

    commercialize_decision:
      type: decision
      name: "How to commercialize?"
      branches:
        license:
          value: 5000000
        manufacture:
          cost: 3000000
          value: 8000000
```

### Node Types

| Type | Symbol | Description |
|------|--------|-------------|
| `decision` | Square | Choice point (we control) |
| `chance` | Circle | Uncertainty (we don't control) |
| `terminal` | Triangle | End state with value |

### CLI Commands

```bash
# Analyze tree, find optimal path
forge decision-tree model.yaml

# Show full tree with values
forge decision-tree model.yaml --verbose

# Export to visualization format
forge decision-tree model.yaml --export dot > tree.dot

# Sensitivity on probabilities
forge decision-tree model.yaml --sensitivity tech_outcome.success
```

### Output Format

```yaml
decision_tree_results:
  optimal_path:
    - decision: "Invest in R&D?" → "invest"
    - chance: "Technology works?" → (await outcome)
    - decision: "How to commercialize?" → "license"

  expected_value: 200000

  node_values:
    root: 200000
    tech_outcome: 2200000
    commercialize_decision: 5000000

  decision_policy:
    "Invest in R&D?": "invest"
    "How to commercialize?": "license"  # If success

  risk_profile:
    best_case: 5000000   # Invest → Success → License
    worst_case: -2000000 # Invest → Failure
    probability_positive: 0.60
```

### Rollback Calculation

Decision trees solve by **backward induction**:

1. Start at terminal nodes (known values)
2. At chance nodes: Expected value = Σ(probability × child_value)
3. At decision nodes: Optimal value = max(child_values)
4. Roll back to root

```
commercialize_decision: max($5M, $8M-$3M) = $5M (license)
tech_outcome: 0.6 × $5M + 0.4 × (-$2M) = $2.2M
root: max($2.2M - $2M, $0) = $0.2M (invest)
```

## Rationale

### Why Decision Trees?

1. **Sequential decisions**: Model real-world staged investments
2. **Optimal policy**: Tells you what to do at each point
3. **Clear communication**: Visual tree intuitive for executives
4. **Value of information**: Can calculate worth of waiting

### Why YAML Format?

1. **Git-diffable**: Track decision model changes
2. **Composable**: Nodes can reference Monte Carlo distributions
3. **Readable**: Self-documenting structure
4. **Validates**: Schema ensures tree is well-formed

### Integration with Monte Carlo

Terminal values can be Monte Carlo simulations:

```yaml
nodes:
  market_entry:
    type: terminal
    monte_carlo:
      iterations: 10000
      outputs:
        - variable: npv
          percentiles: [10, 50, 90]
      scalars:
        revenue:
          formula: "=MC.Normal(5000000, 800000)"
        costs:
          formula: "=MC.Triangular(2000000, 2500000, 3500000)"
        npv:
          formula: "=revenue - costs"
```

## Consequences

### Positive
- Models sequential decisions (gap in current tooling)
- Finds optimal decision policy automatically
- Integrates with Monte Carlo for terminal values
- Standard FP&A technique with clear theory

### Negative
- Combinatorial explosion with many branches
- Assumes rational decision-maker
- Probabilities must be estimated (often subjective)
- More complex than simple NPV

### Mitigations
- Limit tree depth in validation
- Allow probability sensitivity analysis
- Document that trees are decision aids, not oracles

## Alternatives Considered

### Influence Diagrams

More compact than decision trees, but:
- Harder to visualize
- Less intuitive for non-experts
- Overkill for most FP&A use cases

**Rejected**: Decision trees sufficient for target use cases.

### External Tree Tools

Use TreeAge, PrecisionTree, etc. and import results.

**Rejected**: Breaks YAML-native philosophy, vendor lock-in.

## Implementation Notes

1. Parse YAML into tree structure
2. Validate: all branches sum to 1.0 at chance nodes
3. Validate: no cycles (DAG only)
4. Rollback algorithm for expected values
5. Track optimal decisions at each decision node
6. Optional: Monte Carlo at terminal nodes
7. Export: DOT format for Graphviz visualization

## Roundtrip Validation

> **E2E tests live in [forge-e2e](https://github.com/royalbit/forge-e2e)** - see ADR-027.

Decision Tree results are validated against **R** (gold standard for statistical computing).

### Validation Tool

```bash
# R validation script (requires: brew install r)
R --quiet -e '
  # Decision tree structure (R&D Investment example)
  # Node values from backward induction

  # Terminal values
  license_value <- 5000000
  manufacture_value <- 8000000 - 3000000  # net of cost
  failure_value <- -2000000
  no_invest_value <- 0

  # Commercialize decision (choose max)
  commercialize_ev <- max(license_value, manufacture_value)
  cat("Commercialize EV:", commercialize_ev, "\n")  # 5000000

  # Tech outcome (chance node)
  p_success <- 0.60
  p_failure <- 0.40
  tech_ev <- p_success * commercialize_ev + p_failure * failure_value
  cat("Tech Outcome EV:", tech_ev, "\n")  # 2200000

  # Invest decision (choose max, subtract investment cost)
  invest_cost <- 2000000
  invest_ev <- tech_ev - invest_cost
  cat("Invest EV:", invest_ev, "\n")  # 200000

  # Root decision
  root_ev <- max(invest_ev, no_invest_value)
  cat("Optimal Decision:", ifelse(invest_ev > no_invest_value, "Invest", "Dont Invest"), "\n")
  cat("Root EV:", root_ev, "\n")  # 200000
'
```

### Test Coverage

| Test | Validation |
|------|------------|
| DAG structure (no cycles) | Unit test |
| Backward induction | R |
| Optimal path identification | E2E test |
| MC at terminal nodes | E2E + MC validator |

## References

- Raiffa, H. (1968). *Decision Analysis*. Addison-Wesley.
- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- ADR-018: Scenario Analysis
- R Project: https://www.r-project.org/
