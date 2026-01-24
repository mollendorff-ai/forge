# ADR-020: Real Options Analysis

## Status

**Implemented** - v8.5.0

## Context

Traditional NPV assumes:
1. Invest now or never
2. Once committed, you're locked in
3. No learning or adaptation

Reality is different. Managers have **flexibility**:
- Wait for more information before investing
- Expand if things go well
- Abandon if things go poorly
- Switch between alternatives

This flexibility has value. Real Options Analysis quantifies it.

## Decision

**Forge will implement Real Options Analysis for valuing managerial flexibility.**

### Core Concept

```
Traditional NPV: Project value assuming fixed plan
Real Options:    Project value including flexibility value

Real Options Value = Traditional NPV + Option Value
```

If NPV = -$1M but Option Value = +$1.5M, the project is worth $0.5M.

### YAML Syntax

```yaml
_forge_version: "9.0.0"

real_options:
  name: "Phased Factory Investment"
  method: binomial  # or black_scholes, monte_carlo

  underlying:
    current_value: 10000000  # Present value of project cash flows
    volatility: 0.30         # Annual volatility of value
    risk_free_rate: 0.05     # Annual risk-free rate
    time_horizon: 3          # Years

  options:
    - type: defer
      name: "Wait up to 2 years"
      max_deferral: 2
      exercise_cost: 8000000

    - type: expand
      name: "Build Phase 2"
      trigger: "phase1_success"
      expansion_factor: 1.5
      exercise_cost: 4000000

    - type: abandon
      name: "Sell assets"
      salvage_value: 3000000

# Base project for comparison
scalars:
  base_npv:
    value: -500000  # Traditional NPV is negative
```

### Option Types

| Option | Description | Value Driver |
|--------|-------------|--------------|
| **defer** | Wait before investing | Uncertainty resolution |
| **expand** | Scale up if successful | Upside capture |
| **contract** | Scale down if weak | Cost reduction |
| **abandon** | Exit and recover salvage | Loss limitation |
| **switch** | Change inputs/outputs | Flexibility |
| **compound** | Option on an option | Staged investment |

### CLI Commands

```bash
# Value all options in model
forge real-options model.yaml

# Value specific option
forge real-options model.yaml --option defer

# Sensitivity to volatility
forge real-options model.yaml --sensitivity volatility

# Compare with/without options
forge real-options model.yaml --compare-npv

# Export option value surface
forge real-options model.yaml --export-surface
```

### Output Format

```yaml
real_options_results:
  underlying:
    current_value: 10000000
    volatility: 0.30

  traditional_npv: -500000

  options:
    defer:
      value: 1200000
      optimal_trigger: "value > 9500000"
      expected_wait: 1.3  # years

    expand:
      value: 800000
      probability_exercise: 0.45

    abandon:
      value: 400000
      probability_exercise: 0.20

  total_option_value: 2400000
  project_value_with_options: 1900000  # -500K + 2.4M

  decision: "ACCEPT (with options)"
  recommendation: "Defer investment, exercise when value exceeds $9.5M"
```

### Valuation Methods

| Method | Best For | Complexity |
|--------|----------|------------|
| **Black-Scholes** | Simple European options | Low |
| **Binomial Tree** | American options, path-dependent | Medium |
| **Monte Carlo** | Complex/exotic options | High |

```yaml
real_options:
  method: binomial
  binomial_steps: 100  # More steps = more accuracy

  # Or for complex options:
  method: monte_carlo
  monte_carlo:
    iterations: 50000
    sampling: latin_hypercube
```

## Rationale

### Why Real Options?

1. **Captures flexibility value**: NPV misses option value
2. **Better capital allocation**: Don't reject valuable projects
3. **Strategic insight**: Quantifies "wait and see"
4. **Risk management**: Values downside protection

### Key Insight

Traditional NPV often says "no" to projects that intuition says "maybe."

Real Options explains why: The option to learn and adapt has value.

```
R&D Investment:
  NPV = -$2M (reject?)

  But you have options:
  - Abandon if Phase 1 fails (limits loss to $1M)
  - Expand 3x if breakthrough (captures upside)

  Option Value = +$3M
  Project Value = $1M (accept!)
```

### When Real Options Add Most Value

1. **High uncertainty**: More volatility = more option value
2. **Long time horizon**: More time to learn and adapt
3. **Managerial flexibility**: Must be able to exercise options
4. **Staged investments**: Natural decision points

### When Real Options Add Little Value

1. **Now or never**: No deferral possible
2. **Low uncertainty**: Little to learn
3. **Sunk costs**: No abandonment value
4. **Contractual lock-in**: No flexibility

## Consequences

### Positive
- Quantifies flexibility value (major gap in NPV)
- Better investment decisions
- Aligns with how managers actually think
- Rigorous theoretical foundation

### Negative
- Requires volatility estimation (often difficult)
- Complex to explain to non-finance audiences
- Can be used to justify bad projects ("there's option value!")
- More parameters to estimate

### Mitigations
- Provide sensitivity analysis on volatility
- Compare with/without options clearly
- Document assumptions prominently
- Require traditional NPV alongside option value

## Alternatives Considered

### External Tools Only

Use specialized software (Real Options Valuation, Crystal Ball Real Options).

**Rejected**: Breaks YAML-native workflow, requires separate tools.

### Decision Trees Only

Model flexibility as decision tree branches.

**Rejected**: Doesn't capture continuous exercise decisions, less theoretically rigorous for financial options.

### Monte Carlo with Embedded Decisions

Simulate paths with if/then decision rules.

**Accepted as option**: Will support this as `method: monte_carlo` for complex cases.

## Implementation Notes

1. **Binomial tree**: Build recombining tree, roll back with max(exercise, hold)
2. **Black-Scholes**: Implement closed-form for European options
3. **Monte Carlo**: Least-squares Monte Carlo (Longstaff-Schwartz) for American options
4. **Volatility**: Allow direct input or calculate from historical data
5. **Output**: Show traditional NPV vs option-adjusted value
6. **Validation**: Compare simple cases with analytical solutions

## Roundtrip Validation

> **E2E tests live in [forge-e2e](https://github.com/mollendorff-ai/forge-e2e)** - see ADR-027.

Real Options results are validated using **RustQuant** (Rust-native options pricing).

### Validation Tool

```rust
// Cargo.toml: rustquant = "0.3"
use rustquant::instruments::options::*;

// European call option (Black-Scholes)
let spot = 100.0;
let strike = 100.0;
let rate = 0.05;
let volatility = 0.20;
let maturity = 1.0;  // years

let bs = BlackScholes::new(spot, strike, rate, volatility, maturity);
let call_price = bs.call_price();
// Expected: ~$10.4506 (Black-Scholes closed-form)
```

### R Alternative

```bash
# R validation (requires: brew install r)
R --quiet -e '
  # Black-Scholes European call
  spot <- 100
  strike <- 100
  rate <- 0.05
  vol <- 0.20
  T <- 1.0

  d1 <- (log(spot/strike) + (rate + vol^2/2) * T) / (vol * sqrt(T))
  d2 <- d1 - vol * sqrt(T)

  call_price <- spot * pnorm(d1) - strike * exp(-rate * T) * pnorm(d2)
  cat("Call Option Value:", round(call_price, 4), "\n")
  # Expected: ~10.4506
'
```

### Test Coverage

| Test | Validation |
|------|------------|
| Black-Scholes formula | RustQuant / R |
| Binomial tree convergence | Unit test |
| American option pricing | Unit test (binomial) |
| Put-call parity | Unit test |

## References

- Dixit, A. & Pindyck, R. (1994). *Investment Under Uncertainty*. Princeton.
- Trigeorgis, L. (1996). *Real Options: Managerial Flexibility and Strategy*. MIT Press.
- Longstaff, F. & Schwartz, E. (2001). "Valuing American Options by Simulation." *Review of Financial Studies*.
- docs/FPA-PREDICTION-METHODS.md - Method comparison guide
- ADR-016: Monte Carlo Architecture
- ADR-019: Decision Trees
- QuantLib: https://www.quantlib.org/
- RustQuant: https://github.com/avhz/RustQuant
