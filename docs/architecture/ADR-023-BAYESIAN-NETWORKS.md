# ADR-023: Bayesian Networks

## Status

**Implemented** - v9.0.0

## Context

Monte Carlo and Bootstrap model uncertainty in individual variables. But FP&A often involves:

- **Causal relationships**: Economic conditions → Industry health → Company revenue
- **Conditional reasoning**: "If we observe X, what's the probability of Y?"
- **Belief updating**: "Given this new evidence, what should we believe?"
- **Root cause analysis**: "What's driving this outcome?"

Bayesian Networks model probabilistic dependencies between variables as a directed graph.

## Decision

**Forge will implement Bayesian Networks for causal probabilistic modeling.**

### Core Concept

```
Directed Acyclic Graph (DAG):

[Economic Conditions]
       ↓
[Company Revenue] → [Default Probability]
       ↑
[Management Quality]
```

Each node has:
- **States**: Possible values (discrete or continuous)
- **Prior**: Probability distribution (for root nodes)
- **CPT**: Conditional Probability Table (for child nodes)

**Inference**: Calculate P(A | evidence) using belief propagation.

### YAML Syntax

```yaml
_forge_version: "9.0.0"

bayesian_network:
  name: "Credit Risk Model"

  nodes:
    economic_conditions:
      type: discrete
      states: [good, neutral, bad]
      prior: [0.3, 0.5, 0.2]

    company_revenue:
      type: discrete
      states: [high, medium, low]
      parents: [economic_conditions]
      cpt:
        good:    [0.6, 0.3, 0.1]  # P(revenue | economy=good)
        neutral: [0.3, 0.5, 0.2]  # P(revenue | economy=neutral)
        bad:     [0.1, 0.3, 0.6]  # P(revenue | economy=bad)

    default_probability:
      type: discrete
      states: [low, medium, high]
      parents: [company_revenue]
      cpt:
        high:   [0.8, 0.15, 0.05]  # P(default | revenue=high)
        medium: [0.4, 0.40, 0.20]  # P(default | revenue=medium)
        low:    [0.1, 0.30, 0.60]  # P(default | revenue=low)
```

### Node Types

| Type | Description | Example |
|------|-------------|---------|
| **discrete** | Finite states | good/neutral/bad |
| **continuous** | Gaussian distribution | Revenue amount |

### CLI Commands

```bash
# Query marginal probability
forge bayesian model.yaml --query default_probability

# Query with evidence
forge bayesian model.yaml --query default_probability \
  --evidence economic_conditions=bad

# Query all nodes
forge bayesian model.yaml --query-all

# Most likely explanation (MPE)
forge bayesian model.yaml --mpe

# Export network to DOT (Graphviz)
forge bayesian model.yaml --export-graph > network.dot
dot -Tpng network.dot -o network.png
```

### Output Format

#### Marginal Query

```yaml
query_result:
  name: default_probability
  states: [low, medium, high]
  probabilities: [0.49, 0.32, 0.19]
  most_likely: low
  max_probability: 0.49
```

#### Conditional Query (with Evidence)

```yaml
query_result:
  name: default_probability
  states: [low, medium, high]
  probabilities: [0.19, 0.33, 0.48]  # Given economy=bad
  most_likely: high
  max_probability: 0.48

evidence:
  economic_conditions: bad
```

#### Full Network Query

```yaml
bayesian_results:
  name: "Credit Risk Model"

  queries:
    economic_conditions:
      states: [good, neutral, bad]
      probabilities: [0.3, 0.5, 0.2]
      most_likely: neutral

    company_revenue:
      states: [high, medium, low]
      probabilities: [0.36, 0.39, 0.25]
      most_likely: medium

    default_probability:
      states: [low, medium, high]
      probabilities: [0.49, 0.32, 0.19]
      most_likely: low

  evidence: {}
```

## Rationale

### Why Bayesian Networks?

1. **Causal modeling**: Explicit dependencies between variables
2. **Bidirectional inference**: Predict effects OR diagnose causes
3. **Incremental evidence**: Update beliefs as data arrives
4. **Uncertainty propagation**: Track how uncertainty flows through system
5. **Transparent reasoning**: Graph shows assumptions clearly

### Use Cases in FP&A

#### Credit Risk Assessment

```
Economic Conditions → Industry Health → Company Revenue
                                     ↘
Debt Level → Default Probability ←──╯
```

Query: "If economy turns bad AND company revenue drops, what's default probability?"

#### Project Success Modeling

```
Market Size → Customer Adoption → Revenue
           ↘                    ↗
            Technology Risk ────╯
```

Query: "If tech risk is high, what's the probability revenue exceeds target?"

#### Supply Chain Risk

```
Supplier A Disruption ↘
                       → Production Delay → Revenue Impact
Supplier B Disruption ↗
```

Query: "If Supplier A fails, what's probability of material revenue impact?"

### Bayesian Networks vs Decision Trees

| Criterion | Bayesian Networks | Decision Trees |
|-----------|-------------------|----------------|
| **Structure** | Arbitrary DAG | Strict tree |
| **Decisions** | No | Yes |
| **Inference** | Bidirectional | Forward only |
| **Learning** | From data | Manual structure |
| **Use case** | Belief updating | Optimal policy |

**Best practice**: Use Decision Trees for sequential decisions, Bayesian Networks for causal reasoning.

## Consequences

### Positive
- Models causal relationships explicitly
- Supports bidirectional inference (predict AND diagnose)
- Updates beliefs with new evidence
- Transparent structure (graph is self-documenting)
- Standard technique in AI/ML

### Negative
- Requires understanding of probability theory
- DAG structure can be hard to elicit from experts
- Discrete nodes can oversimplify continuous variables
- Inference complexity grows with network size
- Conditional probability tables can be large

### Mitigations
- Provide example templates for common FP&A patterns
- Validate DAG structure (no cycles)
- Support continuous nodes (Gaussian)
- Document inference algorithm (belief propagation)
- Warn on large CPTs (>1000 entries)

## Alternatives Considered

### Influence Diagrams

Extends Bayesian Networks with decision and utility nodes.

**Accepted for future**: Will implement as extension in v9.1+.

### Markov Networks (Undirected)

Undirected graphical models.

**Rejected**: Less intuitive for causal modeling, harder to specify CPTs.

### External Tools Only

Use BayesiaLab, GeNIe, or external libraries.

**Rejected**: Breaks YAML-native workflow, vendor lock-in.

## Implementation Notes

### Algorithm: Variable Elimination

1. **Build factors**: Convert CPTs to factor representation
2. **Elimination order**: Topological sort (or min-fill heuristic)
3. **For each variable to eliminate**:
   - Multiply all factors containing that variable
   - Sum out (marginalize) the variable
4. **Final result**: Multiply remaining factors, normalize

### Engine Architecture

```rust
pub struct BayesianEngine {
    config: BayesianConfig,
    bp: BeliefPropagation,
}

impl BayesianEngine {
    pub fn query(&self, target: &str) -> Result<VariableResult, String> {
        // Variable elimination inference
    }

    pub fn query_with_evidence(
        &self,
        target: &str,
        evidence: &HashMap<String, &str>,
    ) -> Result<VariableResult, String> {
        // Inference with observed values
    }
}
```

### Validation Requirements

1. **DAG check**: No cycles (DFS traversal)
2. **Probability sums**: All CPT rows sum to 1.0 (within tolerance)
3. **Parent references**: All parents exist in network
4. **State consistency**: CPT keys match parent states

### Continuous Nodes (Future)

For Gaussian nodes:
```yaml
revenue:
  type: continuous
  mean: 1000000
  std: 200000
  parents: [market_size]
  # Linear Gaussian CPD: mean = β₀ + β₁ × parent
```

## Roundtrip Validation

> **E2E tests live in [forge-e2e](https://github.com/mollendorff-ai/forge-e2e)** - see ADR-027.

Bayesian Network results are validated against **R's bnlearn package** (gold standard for Bayesian networks).

### Validation Tool

```bash
# R validation script (requires: brew install r && R -e 'install.packages("bnlearn")')
R --quiet -e '
  library(bnlearn)

  # Define network structure: economy -> revenue -> default
  dag <- model2network("[economic_conditions][company_revenue|economic_conditions][default_probability|company_revenue]")

  # Create sample data matching CPTs
  # economic_conditions: good=0.3, neutral=0.5, bad=0.2
  # company_revenue | economic_conditions (conditional)
  # default_probability | company_revenue (conditional)

  # For validation, calculate marginal P(default_probability)
  # Using law of total probability:
  # P(default=low) = sum over all paths
  p_econ <- c(good=0.3, neutral=0.5, bad=0.2)
  p_rev_given_econ <- matrix(
    c(0.6, 0.3, 0.1,   # P(rev|good)
      0.3, 0.5, 0.2,   # P(rev|neutral)
      0.1, 0.3, 0.6),  # P(rev|bad)
    nrow=3, byrow=TRUE
  )
  p_def_given_rev <- matrix(
    c(0.8, 0.15, 0.05,  # P(def|high_rev)
      0.4, 0.4, 0.2,    # P(def|med_rev)
      0.1, 0.3, 0.6),   # P(def|low_rev)
    nrow=3, byrow=TRUE
  )

  # P(revenue) = sum_econ P(rev|econ) * P(econ)
  p_rev <- colSums(p_rev_given_econ * p_econ)
  cat("P(revenue) [high, med, low]:", round(p_rev, 3), "\n")

  # P(default) = sum_rev P(def|rev) * P(rev)
  p_def <- colSums(t(p_def_given_rev) * p_rev)
  cat("P(default) [low, med, high]:", round(p_def, 3), "\n")
  # Expected: ~[0.49, 0.32, 0.19]
'
```

### Expected Output

```
P(revenue) [high, med, low]: 0.35 0.39 0.26
P(default) [low, med, high]: 0.49 0.32 0.19

P(default | economy=bad):
[low, med, high]: 0.19 0.33 0.48
```

### Test Coverage

| Test | Validation |
|------|------------|
| Marginal probabilities | R bnlearn |
| Conditional probabilities | R with evidence |
| DAG validation | bnlearn model check |
| CPT normalization | Unit test |
| Topological ordering | E2E test |

### Known Differences

Forge and R may differ slightly due to:
- **Floating-point precision**: Acceptable within 0.01%
- **Elimination order**: Different heuristics (same result)
- **Factor representation**: Internal only, results match

## References

- Pearl, J. (1988). *Probabilistic Reasoning in Intelligent Systems*. Morgan Kaufmann.
- Koller, D. & Friedman, N. (2009). *Probabilistic Graphical Models*. MIT Press.
- Jensen, F. V. & Nielsen, T. D. (2007). *Bayesian Networks and Decision Graphs*. Springer.
- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- ADR-019: Decision Trees
- R bnlearn: https://www.bnlearn.com/
