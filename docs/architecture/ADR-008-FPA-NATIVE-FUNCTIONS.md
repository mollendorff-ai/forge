# ADR-008: FP&A-Native Functions

**Status:** Accepted
**Date:** 2025-12-08
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Excel has 400+ functions. But ask any FP&A analyst what they do daily:

1. **Budget vs Actual variance** - "Did we beat or miss?"
2. **Break-even analysis** - "How many units to profitability?"

Excel has NPV, IRR, PMT. Excel does NOT have:
- `VARIANCE_STATUS(actual, budget)` → "BEAT" / "MISS" / "ON_TARGET"
- `BREAKEVEN_UNITS(fixed_costs, price, variable_cost)` → units needed

**Every analyst builds these manually. Every time. In every model.**

## Decision

**Implement 6 FP&A-native functions that Excel should have had.**

| Function | Purpose | Excel Equivalent |
|----------|---------|------------------|
| `VARIANCE(actual, budget)` | Raw variance | `=actual - budget` |
| `VARIANCE_PCT(actual, budget)` | % variance | `=(actual - budget) / budget` |
| `VARIANCE_STATUS(actual, budget, [type])` | BEAT/MISS/ON_TARGET | 3+ nested IF statements |
| `BREAKEVEN_UNITS(fixed, price, var_cost)` | Units to break even | Manual formula |
| `BREAKEVEN_REVENUE(fixed, margin_pct)` | Revenue to break even | Manual formula |
| `SCENARIO(name, variable)` | Get scenario value | No equivalent |

### Usage

```yaml
# What takes nested IFs in Excel:
status: "=VARIANCE_STATUS(actual.revenue, budget.revenue)"
# Returns: "BEAT", "MISS", or "ON_TARGET"

# Cost variance (under budget = good):
cost_status: "=VARIANCE_STATUS(actual.costs, budget.costs, \"cost\")"
# Returns: "BEAT" if actual < budget

# Break-even in one function:
units_needed: "=BREAKEVEN_UNITS(500000, 150, 60)"
# Returns: 5556 (500000 / (150 - 60))
```

## Rationale

### 1. What Analysts Actually Do

FP&A workflow:
1. Build budget model
2. Month closes, actuals come in
3. Compare budget vs actual
4. Flag variances
5. Calculate break-even for new products

Steps 3-5 are done manually in Excel. Every. Single. Time.

### 2. Semantic Clarity

Excel:
```excel
=IF(actual>budget,"BEAT",IF(actual<budget,"MISS","ON_TARGET"))
```

Forge:
```yaml
status: "=VARIANCE_STATUS(actual, budget)"
```

Which one does AI understand better? Which one is self-documenting?

### 3. Type-Aware Variance

Revenue over budget = good. Costs over budget = bad.

```yaml
# Revenue: actual > budget = BEAT
revenue_status: "=VARIANCE_STATUS(actual_rev, budget_rev)"

# Costs: actual < budget = BEAT (inverted logic)
cost_status: "=VARIANCE_STATUS(actual_cost, budget_cost, \"cost\")"
```

The `type` parameter handles FP&A semantics that analysts implement manually.

### 4. Excel Export Compatibility

Forge-native functions translate to Excel formulas on export:

| Forge | Excel Export |
|-------|--------------|
| `VARIANCE(a, b)` | `=a-b` |
| `VARIANCE_PCT(a, b)` | `=(a-b)/b` |
| `VARIANCE_STATUS(a, b)` | `=IF(a>b,"BEAT",IF(a<b,"MISS","ON_TARGET"))` |
| `BREAKEVEN_UNITS(f, p, v)` | `=f/(p-v)` |

CFO still gets working Excel. Analysts get cleaner YAML.

## Consequences

### Positive
- Analysts write less boilerplate
- Self-documenting formulas
- Type-aware variance (revenue vs cost)
- AI-friendly semantics
- Excel export works

### Negative
- Functions not in Excel (learning curve)
- Must document clearly
- Export translation adds complexity

### Neutral
- 6 new functions to maintain
- Tests required for each

## Implementation

```rust
// src/core/array_calculator/evaluator/forge.rs

pub fn variance(actual: f64, budget: f64) -> f64 {
    actual - budget
}

pub fn variance_pct(actual: f64, budget: f64) -> f64 {
    if budget == 0.0 { return 0.0; }
    (actual - budget) / budget
}

pub fn variance_status(actual: f64, budget: f64, var_type: &str) -> &'static str {
    let is_cost = var_type == "cost";
    let diff = actual - budget;

    match (diff > 0.0, diff < 0.0, is_cost) {
        (true, _, false) => "BEAT",   // Revenue over = good
        (_, true, false) => "MISS",   // Revenue under = bad
        (true, _, true) => "MISS",    // Cost over = bad
        (_, true, true) => "BEAT",    // Cost under = good
        _ => "ON_TARGET",
    }
}

pub fn breakeven_units(fixed: f64, price: f64, var_cost: f64) -> f64 {
    let margin = price - var_cost;
    if margin <= 0.0 { return f64::INFINITY; }
    fixed / margin
}
```

## References

- FP&A workflow: Budget vs Actual analysis
- Break-even analysis fundamentals
- `src/core/array_calculator/evaluator/forge.rs`

---

*Excel has NPV. Excel forgot VARIANCE_STATUS. We didn't.*

-- Claude Opus 4.5, Principal Autonomous AI
