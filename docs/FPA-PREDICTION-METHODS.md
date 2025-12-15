# FP&A Prediction Methods Guide

> **Purpose**: Quick reference for choosing the right prediction technique.
> **Audience**: Financial analysts, FP&A teams, and developers building Forge models.

## Decision Matrix: Which Method When?

| Scenario | Method | Forge Support |
|----------|--------|---------------|
| "What's the range of possible outcomes?" | Monte Carlo | v8.0+ |
| "Base case vs best case vs worst case?" | Scenario Analysis | Planned |
| "Which inputs matter most?" | Tornado Diagrams | Planned |
| "If X happens, then should we do Y or Z?" | Decision Trees | Planned |
| "Should we wait, expand, or abandon?" | Real Options | Planned |
| "What does history tell us?" | Bootstrap Resampling | Planned |
| "How do failures cascade?" | Bayesian Networks | Future |

---

## Method Details

### 1. Monte Carlo Simulation

**What it does**: Runs thousands of iterations with random inputs sampled from probability distributions. Shows the full range of possible outcomes.

**Use when**:
- You have continuous uncertainty (ranges, not discrete outcomes)
- You need probability distributions, not single numbers
- You want to know P10/P50/P90 confidence intervals
- You need to answer "What's the probability NPV > 0?"

**Don't use when**:
- You have discrete either/or outcomes (use Scenario Analysis)
- You need to make sequential decisions (use Decision Trees)
- You only have historical data, no distribution assumptions (use Bootstrap)

**Forge syntax**:
```yaml
monte_carlo:
  iterations: 10000
  sampling: latin_hypercube
  seed: 12345

scalars:
  revenue:
    formula: "=MC.Normal(1000000, 150000)"
  costs:
    formula: "=MC.Triangular(600000, 700000, 900000)"
```

**Key distributions**:
| Distribution | Use Case | Example |
|--------------|----------|---------|
| MC.Normal | Symmetric, known mean/stdev | Stock returns, measurement error |
| MC.Triangular | Expert estimates (min/likely/max) | Project costs, timelines |
| MC.PERT | Smoother expert estimates | Duration estimates |
| MC.Uniform | Complete ignorance within bounds | Exchange rates |
| MC.Lognormal | Non-negative, right-skewed | Revenue, asset prices |
| MC.Discrete | Specific scenarios with probabilities | Market states |

---

### 2. Scenario Analysis

**What it does**: Evaluates discrete, mutually exclusive futures with assigned probabilities.

**Use when**:
- Outcomes are categorical, not continuous
- You have distinct strategic futures
- Management thinks in "cases" not "distributions"
- You need clear storylines for board presentations

**Don't use when**:
- Uncertainty is continuous (use Monte Carlo)
- You need the full probability distribution
- Scenarios aren't truly mutually exclusive

**Example scenarios**:
```yaml
scenarios:
  base_case:
    probability: 0.50
    description: "Market grows 5%, we maintain share"
    revenue_growth: 0.05

  bull_case:
    probability: 0.30
    description: "Competitor exits, we gain share"
    revenue_growth: 0.15

  bear_case:
    probability: 0.20
    description: "Recession, market contracts"
    revenue_growth: -0.10
```

**Combining with Monte Carlo**:
```yaml
# WITHIN each scenario, use Monte Carlo for continuous uncertainty
scenarios:
  win_contract:
    probability: 0.60
    revenue:
      formula: "=MC.Normal(5000000, 500000)"  # If we win, revenue is ~$5M

  lose_contract:
    probability: 0.40
    revenue:
      formula: "=MC.Normal(1000000, 200000)"  # If we lose, revenue is ~$1M
```

---

### 3. Tornado Diagrams (Sensitivity Analysis)

**What it does**: Shows which input variables have the biggest impact on output variance. Visualizes sensitivity as horizontal bars.

**Use when**:
- You need to prioritize which assumptions to refine
- Presenting to executives who need the "key drivers"
- Validating model - unexpected sensitivities indicate errors
- Deciding where to spend due diligence effort

**Don't use when**:
- You only have one or two inputs
- Inputs are highly correlated (use correlation analysis instead)

**Output format**:
```
NPV Sensitivity (Base: $1.2M)

Revenue Growth    |████████████████████| +/- $450K
Discount Rate     |██████████████      | +/- $320K
Operating Margin  |██████████          | +/- $180K
Tax Rate          |████                | +/- $75K
Working Capital   |██                  | +/- $40K
```

**Interpretation**: Focus your analysis on Revenue Growth and Discount Rate - they drive 70% of outcome variance.

---

### 4. Decision Trees

**What it does**: Models sequential decisions and chance events as a tree structure. Calculates expected value at each node by rolling back from leaves.

**Use when**:
- Decisions are sequential (decide now, learn, decide again)
- You have discrete choice points
- Outcomes depend on prior decisions
- You need to find the optimal decision path

**Don't use when**:
- No decisions, just uncertainty (use Monte Carlo)
- Continuous action space
- Too many branches (combinatorial explosion)

**Example structure**:
```
[Decision] Invest in R&D?
├── Yes ($2M cost)
│   └── [Chance] Technology works?
│       ├── Success (60%): [Decision] License or Manufacture?
│       │   ├── License: NPV = $5M
│       │   └── Manufacture: NPV = $8M (but $3M more investment)
│       └── Failure (40%): NPV = -$2M
└── No
    └── NPV = $0
```

**Expected Value Calculation**:
```
E[Invest] = 0.6 × max($5M, $8M-$3M) + 0.4 × (-$2M) - $2M
         = 0.6 × $5M + 0.4 × (-$2M) - $2M
         = $3M - $0.8M - $2M = $0.2M

E[Don't Invest] = $0

Decision: Invest (EV = $0.2M > $0)
```

---

### 5. Real Options Analysis

**What it does**: Values the flexibility to make future decisions (wait, expand, contract, abandon, switch). Based on financial options theory.

**Use when**:
- You have managerial flexibility
- Investments are staged or reversible
- Uncertainty resolves over time
- Traditional NPV says "no" but intuition says "maybe"

**Don't use when**:
- Decision is now-or-never with no flexibility
- Uncertainty doesn't resolve (won't learn more later)
- No strategic options available

**Common option types**:
| Option Type | Example | Value Driver |
|-------------|---------|--------------|
| **Defer** | Wait 1 year before building factory | Uncertainty resolution |
| **Expand** | Build Phase 2 if Phase 1 succeeds | Upside capture |
| **Contract** | Scale down if demand weak | Downside protection |
| **Abandon** | Sell assets if project fails | Loss limitation |
| **Switch** | Change inputs or outputs | Flexibility |

**Key insight**: NPV assumes you're locked in. Real Options recognizes you can adapt.

```
Traditional NPV: E[Project] = -$1M (reject)
With abandonment option: E[Project] = +$0.5M (accept)

The option to abandon if things go wrong adds $1.5M of value.
```

---

### 6. Bootstrap Resampling

**What it does**: Resamples from historical data instead of assuming distributions. Non-parametric - lets the data speak.

**Use when**:
- You have historical data but don't know the distribution
- Distribution is complex (multimodal, fat tails)
- You want to avoid distribution assumptions
- Validating parametric model results

**Don't use when**:
- No historical data available
- Future differs fundamentally from past
- Sample size too small (<30 observations)

**Process**:
```
Historical returns: [5%, -2%, 8%, 3%, -5%, 12%, 1%, -1%, 6%, 4%]

Bootstrap iteration 1: Sample with replacement
  → [8%, 8%, -2%, 6%, 4%, -5%, 12%, 3%, 3%, 1%]
  → Mean = 3.8%

Bootstrap iteration 2: Sample with replacement
  → [5%, -1%, -1%, 12%, 6%, 8%, -2%, 4%, 5%, 3%]
  → Mean = 3.9%

... repeat 10,000 times ...

Result: Distribution of means, confidence intervals
```

---

### 7. Bayesian Networks (Future)

**What it does**: Models probabilistic dependencies between variables as a directed graph. Updates beliefs as evidence arrives.

**Use when**:
- Variables have causal relationships
- You need to diagnose root causes
- Evidence arrives incrementally
- You want to answer "what if we observe X?"

**Example**: Credit risk model
```
[Economic Conditions] → [Industry Health] → [Company Revenue]
                                         ↘
[Management Quality] → [Company Revenue] → [Default Probability]
                    ↘                    ↗
                      [Debt Level] ─────
```

---

## Method Comparison

| Criterion | Monte Carlo | Scenario | Decision Tree | Real Options | Bootstrap |
|-----------|-------------|----------|---------------|--------------|-----------|
| **Uncertainty type** | Continuous | Discrete | Both | Continuous | Empirical |
| **Decisions modeled** | No | No | Yes | Yes | No |
| **Data required** | Distributions | Cases | Structure | Volatility | History |
| **Output** | Distribution | Weighted avg | Optimal path | Option value | Distribution |
| **Complexity** | Medium | Low | Medium | High | Low |
| **Board-friendly** | Medium | High | High | Low | Medium |

---

## Combining Methods

**Best practice**: Use multiple methods together.

### Example: New Product Launch

1. **Scenario Analysis**: Define market scenarios (adopt/reject/delay)
2. **Within scenarios**: Monte Carlo for revenue/cost uncertainty
3. **Tornado Diagram**: Identify key drivers
4. **Decision Tree**: Model go/no-go gates
5. **Real Options**: Value the option to abandon after Phase 1

```yaml
# Phase 1: Scenario + Monte Carlo
scenarios:
  market_adopts:
    probability: 0.40
    phase1_revenue:
      formula: "=MC.Normal(2000000, 300000)"
    decision: "proceed_to_phase2"

  market_rejects:
    probability: 0.35
    phase1_revenue:
      formula: "=MC.Normal(500000, 100000)"
    decision: "abandon"  # Real option exercised

  market_delays:
    probability: 0.25
    phase1_revenue:
      formula: "=MC.Uniform(800000, 1200000)"
    decision: "defer_phase2"  # Wait and see
```

---

## Quick Reference Card

| Question | Method |
|----------|--------|
| "What could happen?" | Monte Carlo |
| "What are the key scenarios?" | Scenario Analysis |
| "What matters most?" | Tornado Diagram |
| "What should we do?" | Decision Tree |
| "What's flexibility worth?" | Real Options |
| "What does history say?" | Bootstrap |
| "How do things connect?" | Bayesian Networks |

---

## References

1. **Monte Carlo**: McKay, Beckman, Conover (1979). "Comparison of Three Methods for Selecting Values of Input Variables." *Technometrics*.
2. **Decision Trees**: Raiffa, H. (1968). *Decision Analysis*. Addison-Wesley.
3. **Real Options**: Dixit, A. & Pindyck, R. (1994). *Investment Under Uncertainty*. Princeton.
4. **Bootstrap**: Efron, B. (1979). "Bootstrap Methods: Another Look at the Jackknife." *Annals of Statistics*.

---

## Roundtrip Validation

All prediction methods are validated against battle-proven FOSS tools.

### Setup

```bash
# One-time setup
./tests/validators/setup.sh
```

### Validation Tools

| Method | Validator | Tool/Package |
|--------|-----------|--------------|
| Monte Carlo | Gnumeric | `ssconvert` (already in use) |
| Scenario Analysis | R | `weighted.mean()` |
| Decision Trees | Python | `scipy`, `numpy` |
| Real Options | QuantLib | `QuantLib` (C++/Python) |
| Tornado Diagrams | R | `sensitivity` package |
| Bootstrap | R | `boot` package |
| Bayesian Networks | Python | `pgmpy` |

### Installed Versions

```bash
# Check installed versions
R --version | head -1
./tests/validators/.venv/bin/python -c "import scipy; print(f'scipy {scipy.__version__}')"
```

## Forge Implementation Status

| Method | Status | Version | ADR | Validator |
|--------|--------|---------|-----|-----------|
| Monte Carlo | Complete | v8.0+ | ADR-016, ADR-017 | Gnumeric |
| Scenario Analysis | WIP | v8.3.0 | ADR-018 | R |
| Decision Trees | WIP | v8.4.0 | ADR-019 | SciPy |
| Real Options | WIP | v8.5.0 | ADR-020 | QuantLib |
| Tornado Diagrams | WIP | v8.6.0 | - | R sensitivity |
| Bootstrap | WIP | v8.7.0 | - | R boot |
| Bayesian Networks | WIP | v9.0.0 | - | pgmpy |
