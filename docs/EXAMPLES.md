# Forge Examples

Real-world usage examples and patterns. For eight questions answered start to finish with the examples in `examples/`, see [docs/posts](posts/README.md).

## Your first model

Three rules catch most first models:

1. **`_forge_version` is the version of the model format, not of Forge.** Write `"5.1.0"`, or `"5.2.0"` only if a table has a period column. `"10.0.0"` is Forge's own version and is refused.
2. **In a group of scalars, a formula is written `{formula: "=..."}`**, because `forge calculate` writes its result next to it (`value: 4800`). A bare `"=..."` string is a column formula and belongs in a table.
3. **A table column works row by row; an aggregate's range is the whole column.** In a column, `revenue` is the row's revenue, but inside `SUM(revenue)` or as the range of `SUMIFS` it is the whole column, so `share: "=revenue / SUM(revenue)"` is each row's share of the total, and a table of quarters can sum a table of months: `revenue: "=SUMIFS(months.revenue, months.quarter, quarter)"` (the criterion, `quarter`, is the row's). With several arguments `SUM`, `MAX` and the like work row by row: `=MAX(0, ebit)`. See ADR-058.

`forge examples starter > model.yaml` gives this model, which calculates as it is:

```yaml
# A first Forge model. Calculate it with: forge calculate starter.yaml
# Then see the results side by side with: forge report starter.yaml

# The model FORMAT version, not the Forge version: "5.1.0" for a new model.
_forge_version: "5.1.0"

# A group of scalars: one value per name.
# An input is a plain number. A formula is written {formula: "=..."}, because
# forge calculate writes its result next to it, as value: ...
# The full name (plan.price) always works; a key of the same group (price) too.
plan:
  price: 40
  units_per_month: 120
  monthly_costs: 3000
  revenue: {formula: "=plan.price * plan.units_per_month"}
  profit: {formula: "=plan.revenue - plan.monthly_costs"}

# A table: columns of the same length, one row per item.
# A column formula is a bare "=..." string, calculated row by row from the
# table's own columns. It cannot reduce a column (SUM, MIN, MAX, AVERAGE of a
# whole column): that is a scalar, below, and a column may read it.
products:
  name: ["Basic", "Plus", "Pro"]
  price: [20, 40, 80]
  units: [200, 120, 30]
  revenue: "=price * units"
  share_of_revenue: "=revenue / totals.revenue"

# A scalar can reduce a column of a table, which it names with the table.
totals:
  revenue: {formula: "=SUM(products.revenue)"}
  best_seller_units: {formula: "=MAX(products.units)"}
```

`forge calculate model.yaml` writes the results into the file:

```
      plan.revenue = 4800
      plan.profit = 1800
      totals.revenue = 11200
      totals.best_seller_units = 200
```

and `forge report model.yaml` shows them side by side (see [REPORT.md](REPORT.md)):

```text
group  price  units_per_month  monthly_costs   revenue    profit
plan   40.00           120.00       3,000.00  4,800.00  1,800.00

group     revenue  best_seller_units
totals  11,200.00             200.00
```

## Quick Start

### Basic Calculation

**Input (pricing.yaml):**

```yaml
_forge_version: "5.1.0"

pricing_table:
  product: ["Widget A", "Widget B", "Widget C"]
  base_price: [100, 150, 200]
  discount_rate: [0.10, 0.15, 0.20]
  final_price: "=base_price * (1 - discount_rate)"
```

**Run:**

```bash
forge calculate pricing.yaml
```

**Output:**

```
   📊 Table: pricing_table
      product = ["Widget A", "Widget B", "Widget C"]
      base_price = [100, 150, 200]
      discount_rate = [0.1, 0.15, 0.2]
      final_price = [90, 127.5, 160]
```

The results are printed. A table formula stays a formula in the file (`final_price` keeps `=base_price * (1 - discount_rate)`); a scalar formula also gets its result stored next to it (see FEATURES.md, The Model File). `--dry-run` prints without writing anything.

## Example Models

Three complete models, each with a README and tests against an outside calculation: [`examples/household`](../examples/household/README.md) (budget against actual), [`examples/mortgage`](../examples/mortgage/README.md) (a loan with extra payments) and [`examples/bagel`](../examples/bagel/README.md) (a small business's first year, with an included costs file).

## Financial Models

### SaaS Metrics (v1.2.1)

```yaml
_forge_version: "5.1.0"

saas_metrics:
  month: ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
  mrr: [10000, 12000, 15000, 18000, 22000, 26000]
  arr: "=mrr * 12"
  new_customers: [10, 15, 20, 25, 30, 35]
  cac: [500, 480, 450, 420, 400, 380]
  ltv: [5000, 5200, 5400, 5600, 5800, 6000]
  ltv_cac_ratio: "=ltv / cac"
  payback_months: "=cac / (mrr * 0.70)"

summary:
  # v1.2.1 conditional aggregations
  total_arr:
    formula: "=SUM(saas_metrics.arr)"
  
  high_growth_months:
    formula: "=COUNTIF(saas_metrics.mrr, \">15000\")"
  
  avg_ltv_high_growth:
    formula: "=AVERAGEIF(saas_metrics.mrr, \">15000\", saas_metrics.ltv)"
```

`forge calculate` gives `total_arr` 1,236,000 (12 × the six months' MRR, 103,000), `high_growth_months` 3 (April to June are above 15,000) and `avg_ltv_high_growth` 5,800 (the average LTV of those three months). A criterion is a string, as in Excel: `">15000"`.

### Quarterly P&L

```yaml
_forge_version: "5.1.0"

pl_2025_q1:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100000, 120000, 150000]
  cogs: [40000, 48000, 60000]
  gross_profit: "=revenue - cogs"
  gross_margin: "=gross_profit / revenue"
  
  opex: [30000, 32000, 35000]
  ebitda: "=gross_profit - opex"
  ebitda_margin: "=ebitda / revenue"

summary:
  total_revenue:
    formula: "=SUM(pl_2025_q1.revenue)"
  
  avg_gross_margin:
    formula: "=AVERAGE(pl_2025_q1.gross_margin)"
```

`forge calculate` gives `total_revenue` 370,000 and `avg_gross_margin` 0.6; `ebitda` is 30,000, 40,000 and 55,000 for the three months.

### Monte Carlo Simulation (v8.0.0)

**NPV Uncertainty Analysis:**

```yaml
_forge_version: "5.1.0"

# Monte Carlo configuration
monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 12345
  outputs:
    - variable: valuation.npv
      percentiles: [10, 50, 90]
      threshold: "> 0"

# Probabilistic assumptions
assumptions:
  revenue_growth:
    formula: "=MC.Normal(0.15, 0.05)"  # 15% ± 5% annual growth

  initial_cost:
    formula: "=MC.Triangular(80000, 100000, 150000)"  # min/mode/max

  discount_rate:
    formula: "=MC.Uniform(0.08, 0.12)"  # 8-12% WACC range

# Cash flow projections
cash_flows:
  year: [0, 1, 2, 3, 4, 5]
  base_revenue: [0, 100000, 110000, 120000, 130000, 140000]

  # Apply stochastic growth
  actual_revenue: "=IF(year = 0, 0, base_revenue * (1 + assumptions.revenue_growth))"

  # Initial cost + ongoing costs
  costs: "=IF(year = 0, assumptions.initial_cost, actual_revenue * 0.6)"

  net_cash_flow: "=actual_revenue - costs"

# Valuation with uncertain inputs
valuation:
  npv:
    formula: "=NPV(assumptions.discount_rate, cash_flows.net_cash_flow)"

  irr:
    formula: "=IRR(cash_flows.net_cash_flow)"
```

**Run simulation:**

```bash
forge simulate npv_analysis.yaml
```

**Output** (`forge simulate npv_analysis.yaml`, trimmed):

```
      valuation.npv:
         Mean:      87,437.48
         Median:    88,265.50
         Std Dev:   17,132.30
         Min:       34,406.02
         Max:       146,902.33
            P10: 64,080.81
            P50: 88,265.50
            P90: 109,079.80
         P(valuation.npv > 0) = 100.00%
         Drivers (rank correlation with the output):
            -0.772  assumptions.initial_cost
            +0.466  assumptions.revenue_growth
            -0.392  assumptions.discount_rate
```

Check by hand: at the inputs' averages (growth 15%, initial cost 110,000, rate 10%) the NPV is 87,218; the average of 10,000 runs is 87,437 (close, not equal, because NPV is not linear in the rate). `NPV()` discounts every cash flow, year 0 included, as Excel's does.

**Key insights:**

1. **Risk**: none of the 10,000 runs loses money; even the 10th percentile is about 64,000.
2. **Spread**: 80% of outcomes fall between about 64,000 and 109,000, so the decision does not hinge on the exact inputs.
3. **What drives it**: the initial cost moves the NPV most (rank correlation -0.77), then growth, then the rate. The drivers rank how consistently each input moves the result while the others vary too; `forge tornado` shows how far each one moves it alone.

**Decision framework:**

- **Green light** if P(NPV > 0) is high and P10 is above the loss you can accept
- **Yellow flag** if the result is very sensitive to inputs you cannot control
- **Red flag** if P50 is below the minimum acceptable return

### Basic Revenue Forecasting

Simple Monte Carlo simulation for quarterly revenue forecasting with uncertainty:

```yaml
_forge_version: "5.1.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  seed: 42
  outputs:
    - variable: forecast.q4_revenue
      percentiles: [10, 25, 50, 75, 90]
      threshold: "> 750000"

assumptions:
  base_revenue:
    formula: "=MC.Normal(600000, 50000)"  # Mean $600K, std dev $50K

  growth_rate:
    formula: "=MC.Triangular(0.05, 0.10, 0.20)"  # min 5%, mode 10%, max 20%

  seasonality_factor:
    formula: "=MC.Uniform(0.95, 1.15)"  # -5% to +15% seasonal variation

forecast:
  q4_revenue:
    formula: "=assumptions.base_revenue * (1 + assumptions.growth_rate) * assumptions.seasonality_factor"
```

**Run:**

```bash
forge simulate revenue_forecast.yaml --output forecast_results.yaml
```

**Output** (trimmed):

```
      forecast.q4_revenue:
         Mean:      703,473.44
            P10: 610,255.85
            P25: 651,287.82
            P50: 701,644.43
            P75: 751,521.98
            P90: 799,584.71
         P(forecast.q4_revenue > 750000) = 25.62%
         Drivers (rank correlation with the output):
            +0.784  assumptions.base_revenue
            +0.531  assumptions.seasonality_factor
            +0.251  assumptions.growth_rate
```

Check by hand: 600,000 × (1 + 0.1167, the triangular mean) × 1.05 (the uniform mean) = 703,500; the simulated mean is 703,473. One quarter in four beats 750,000; plan on about 650,000 to 750,000 (P25 to P75).

### Correlated Variables

Real-world variables often move together. Use `MC.Correlated` to model relationships like revenue and margin correlation:

```yaml
_forge_version: "5.1.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 99
  outputs:
    - variable: financials.net_income
      percentiles: [5, 25, 50, 75, 95]
      threshold: "> 0"

    - variable: financials.revenue
      percentiles: [10, 50, 90]

    - variable: financials.margin
      percentiles: [10, 50, 90]

  # Revenue and margin move against each other (volume discounts):
  # a correlation of -0.6 between the two sampled inputs
  correlations:
    - variables: [financials.revenue, financials.margin]
      coefficient: -0.6

financials:
  revenue:
    formula: "=MC.Normal(1000000, 150000)"

  margin:
    formula: "=MC.Normal(0.30, 0.05)"

  fixed_costs:
    formula: "=MC.Normal(200000, 15000)"

  # Calculated outcomes
  gross_profit:
    formula: "=revenue * margin"

  net_income:
    formula: "=gross_profit - fixed_costs"
```

**Run:**

```bash
forge simulate correlated_model.yaml --output corr_results.yaml
```

**Understanding correlation:**

- **Positive correlation (+0.6 to +1.0)**: the variables move together (revenue and costs)
- **Negative correlation (-0.6 to -1.0)**: they move in opposite directions (volume and price)
- **No correlation (0)**: they are independent

**Output** (trimmed):

```
      financials.net_income:
         Mean:      95,563.16
            P5: 21,981.23
            P25: 63,962.42
            P50: 94,779.01
            P75: 125,748.63
            P95: 173,228.60
         P(financials.net_income > 0) = 98.43%
         Drivers (rank correlation with the output):
            +0.485  financials.margin
            +0.314  financials.revenue
            -0.310  financials.fixed_costs
      financials.revenue:
            P10: 808,766.34
            P50: 1,000,048.26
            P90: 1,192,681.48
      financials.margin:
            P10: 0.2359
            P50: 0.30
            P90: 0.3641
```

The correlation shows in the mean, not in each input's own percentiles: E[revenue × margin] = 1,000,000 × 0.30 + (−0.6 × 150,000 × 0.05) = 295,500, so the expected net income is 295,500 − 200,000 = 95,500 (the simulation gives 95,563). Without the correlation it would be 100,000: high revenue coming with lower margins costs about 4,500 on average.

### Multiple Output Tracking

Track multiple metrics simultaneously with different analysis settings:

```yaml
_forge_version: "5.1.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 2025
  outputs:
    # Track profitability with detailed percentiles
    - variable: metrics.ebitda
      percentiles: [1, 5, 10, 25, 50, 75, 90, 95, 99]
      threshold: "> 0"

    # Track ROI with key percentiles only
    - variable: metrics.roi
      percentiles: [10, 50, 90]
      threshold: "> 0.15"  # chance of a return above 15%

    # Track payback period
    - variable: metrics.payback_years
      percentiles: [25, 50, 75]
      threshold: "< 3"  # chance of paying back within 3 years

    # Track revenue (no sensitivity needed)
    - variable: projections.year_1_revenue
      percentiles: [10, 50, 90]

assumptions:
  market_size:
    formula: "=MC.Normal(5000000, 800000)"

  market_share:
    formula: "=MC.Triangular(0.05, 0.10, 0.18)"  # 5-18% share, mode 10%

  avg_price:
    formula: "=MC.Normal(25, 3)"

  unit_cost:
    formula: "=MC.Triangular(12, 15, 18)"

  opex:
    formula: "=MC.Normal(150000, 20000)"

  initial_investment: 500000

projections:
  year_1_revenue:
    formula: "=assumptions.market_size * assumptions.market_share"

  units_sold:
    formula: "=year_1_revenue / assumptions.avg_price"

  cogs:
    formula: "=units_sold * assumptions.unit_cost"

  gross_profit:
    formula: "=year_1_revenue - cogs"

metrics:
  ebitda:
    formula: "=projections.gross_profit - assumptions.opex"

  roi:
    formula: "=ebitda / assumptions.initial_investment"

  payback_years:
    formula: "=IF(ebitda > 0, assumptions.initial_investment / ebitda, 999)"
```

**Run:**

```bash
forge simulate multi_output.yaml --output multi_results.yaml
```

**Output** (trimmed):

```
      metrics.ebitda:
         Mean:      64,708.01
            P1: -100,507.21
            P5: -58,395.81
            P25: 4,950.99
            P50: 56,764.40
            P75: 116,586.85
            P95: 213,680.42
            P99: 289,681.67
         P(metrics.ebitda > 0) = 77.20%
         Drivers (rank correlation with the output):
            +0.606  assumptions.market_share
            +0.476  assumptions.avg_price
            +0.387  assumptions.market_size
            -0.310  assumptions.unit_cost
            -0.242  assumptions.opex
      metrics.roi:
            P10: -0.0698
            P50: 0.1135
            P90: 0.3518
         P(metrics.roi > 0.15) = 41.62%
      metrics.payback_years:
         Median:    8.81
            P25: 4.29
            P75: 100.99
         P(metrics.payback_years < 3) = 11.86%
      projections.year_1_revenue:
         Mean:      549,468.92
            P10: 354,324.19
            P50: 533,585.49
            P90: 766,406.24
```

Check by hand: revenue averages 5,000,000 × 0.11 (the triangular mean of the share) = 550,000; the simulation gives 549,469.

**Multi-output decision making:**

1. **EBITDA**: positive in 77% of runs, but the median (about 57,000) is below the mean (about 65,000), so most outcomes are modest and a few are large.
2. **ROI**: 42% chance of beating 15%; the median return is 11%.
3. **Payback**: only 12% chance of paying back within 3 years; the median is about 9 years. When EBITDA is zero or negative the model sets payback to 999, which is why the mean payback (248 years) is meaningless here: use the median and the probability.
4. **Next step**: `forge tornado` shows which input moves EBITDA most.

### Cost Estimation with Triangular Distribution

When you have min/most-likely/max estimates (common in project planning):

```yaml
_forge_version: "5.1.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  seed: 7
  outputs:
    - variable: project.total_cost
      percentiles: [10, 50, 90, 95, 99]
      threshold: "> 350000"

# Triangular distributions based on expert estimates: (minimum, most likely, maximum)
phases:
  design_cost:
    formula: "=MC.Triangular(40000, 50000, 75000)"
  dev_cost:
    formula: "=MC.Triangular(120000, 150000, 220000)"
  test_cost:
    formula: "=MC.Triangular(30000, 40000, 60000)"
  deploy_cost:
    formula: "=MC.Triangular(15000, 20000, 35000)"

# Risk factors
risks:
  scope_creep:
    formula: "=MC.Uniform(1.0, 1.25)"  # 0-25% cost overrun

  resource_availability:
    formula: "=MC.Triangular(1.0, 1.05, 1.30)"  # Usually 5%, up to 30% delay

project:
  base_cost:
    formula: "=phases.design_cost + phases.dev_cost + phases.test_cost + phases.deploy_cost"

  total_cost:
    formula: "=base_cost * risks.scope_creep * risks.resource_availability"
```

**Run:**

```bash
forge simulate project_cost.yaml --output cost_results.yaml
```

**Output** (`forge simulate project_cost.yaml`, trimmed):

```
      project.total_cost:
         Mean:      358,064.47
            P10: 304,332.61
            P50: 355,350.55
            P90: 416,021.19
            P95: 435,343.43
            P99: 470,931.18
         P(project.total_cost > 350000) = 55.26%
         Drivers (rank correlation with the output):
            +0.610  phases.dev_cost
            +0.535  risks.scope_creep
            +0.468  risks.resource_availability
            +0.199  phases.design_cost
            +0.182  phases.test_cost
            +0.124  phases.deploy_cost
```

Check by hand: the phases average 55,000 + 163,333 + 43,333 + 23,333 = 285,000 (each triangular mean is (min + mode + max) / 3), and the two risk factors average 1.125 and 1.117, so the expected cost is 285,000 × 1.125 × 1.117 = 358,031; the simulation gives 358,064.

**Budgeting recommendations:**

- **A 350,000 budget is a coin flip**: 55% of runs exceed it.
- **Budget at P90 (about 416,000)** for 90% confidence, or hold a contingency of about 60,000 on top of the 355,000 median.
- **The most likely phase costs add up to only 260,000**: the risk factors and the long right tails are why the expected cost is almost 100,000 more.

## Advanced Features

### Cross-Table References

```yaml
_forge_version: "5.1.0"

assumptions:
  tax_rate: 0.25

revenue:
  product: ["A", "B", "C"]
  sales: [100000, 150000, 200000]
  tax: "=sales * assumptions.tax_rate"  # Cross-table ref
```

### Excel Integration

**Export to Excel:**

```bash
forge export model.yaml model.xlsx
```

**Import from Excel:**

```bash
forge import model.xlsx model.yaml
```

**Round-trip:**

```bash
forge export model.yaml temp.xlsx
forge import temp.xlsx model_roundtrip.yaml
forge calculate model_roundtrip.yaml  # Same numbers as model.yaml
```

The round-tripped file calculates to the same values and keeps the model's order, but it is not byte-identical: comments are not in the workbook, and formulas are respaced. Formulas that Excel or Forge cannot express come back as values, and both commands list them.

## Common Patterns

### Conditional Logic

```yaml
_forge_version: "5.1.0"

pricing:
  volume: [10, 50, 100, 500]
  base_price: [100, 95, 90, 85]
  discount: "=IF(volume > 100, 0.20, IF(volume > 50, 0.10, 0))"
  final_price: "=base_price * (1 - discount)"
```

### Multi-Criteria Filtering (v1.2.1)

```yaml
_forge_version: "5.1.0"

sales:
  region: ["North", "South", "North", "West", "East"]
  category: ["Tech", "Tech", "Furniture", "Tech", "Furniture"]
  revenue: [100000, 150000, 120000, 80000, 95000]

analysis:
  north_tech_revenue:
    formula: "=SUMIFS(sales.revenue, sales.region, 'North', sales.category, 'Tech')"
```

### Precision Control (v1.2.1)

```yaml
_forge_version: "5.1.0"

calculations:
  raw_values: [123.456, 789.123, 456.789]
  rounded: "=ROUND(raw_values, 2)"
  rounded_up: "=ROUNDUP(raw_values, 1)"
  rounded_down: "=ROUNDDOWN(raw_values, 0)"
```

## Advanced Prediction Methods

### Scenario Analysis (v8.3.0)

Model discrete strategic futures with assigned probabilities - perfect for board presentations and strategic planning.

**Use case**: Evaluate a new product launch under three distinct market scenarios. Each scenario has specific assumptions about market adoption and pricing power.

**Input (scenario_npv.yaml):**

```yaml
_forge_version: "5.1.0"

# Scenario configuration
scenarios:
  base_case:
    probability: 0.50
    description: "Market grows 5%, we maintain share"
    scalars:
      revenue_growth: 0.05
      market_share: 0.10
      avg_price: 25.0
      unit_cost: 15.0

  bull_case:
    probability: 0.30
    description: "Competitor exits, we gain share"
    scalars:
      revenue_growth: 0.15
      market_share: 0.18
      avg_price: 28.0
      unit_cost: 14.0

  bear_case:
    probability: 0.20
    description: "Recession, market contracts"
    scalars:
      revenue_growth: -0.10
      market_share: 0.08
      avg_price: 22.0
      unit_cost: 16.0

# Assumptions (the base case; each scenario overrides some of them)
assumptions:
  market_size: 5000000
  initial_investment: 500000
  discount_rate: 0.10
  revenue_growth: 0.05
  market_share: 0.10
  avg_price: 25.0
  unit_cost: 15.0

# Calculations (applied within each scenario)
financials:
  year_1_revenue:
    formula: "=assumptions.market_size * (1 + assumptions.revenue_growth) * assumptions.market_share * assumptions.avg_price"

  year_1_cogs:
    formula: "=(year_1_revenue / assumptions.avg_price) * assumptions.unit_cost"

  year_1_profit:
    formula: "=year_1_revenue - year_1_cogs"

  npv:
    formula: "=(year_1_profit / (1 + assumptions.discount_rate)) - assumptions.initial_investment"
```

**Run:**

```bash
forge scenarios scenario_npv.yaml --output scenario_results.yaml
```

**Output** (`forge scenarios scenario_npv.yaml`, trimmed):

```
   base_case (p=50%): Market grows 5%, we maintain share
      financials.npv: 4,272,727.27
      financials.year_1_revenue: 13,125,000.00

   bear_case (p=20%): Recession, market contracts
      financials.npv: 1,463,636.36
      financials.year_1_revenue: 7,920,000.00

   bull_case (p=30%): Competitor exits, we gain share
      financials.npv: 12,672,727.27
      financials.year_1_revenue: 28,980,000.00

   Expected Values (probability-weighted):
      financials.npv: 6,230,909.09   (range 1,463,636.36 to 12,672,727.27, P(>0) = 100%)
      financials.year_1_revenue: 16,840,500.00   (range 7,920,000.00 to 28,980,000.00, P(>0) = 100%)
```

Check by hand, base case: revenue 5,000,000 × 1.05 × 0.10 × 25 = 13,125,000; COGS 13,125,000 / 25 × 15 = 7,875,000; NPV 5,250,000 / 1.1 − 500,000 = 4,272,727. Expected NPV 0.5 × 4,272,727 + 0.3 × 12,672,727 + 0.2 × 1,463,636 = 6,230,909.

**Use case explanation:**

Scenario analysis is ideal when:
- Management thinks in discrete "cases" not probability distributions
- You need clear storylines for board presentations
- Strategic futures are mutually exclusive (only one will occur)
- Combining with Monte Carlo for continuous uncertainty within scenarios

**Key insight**: every case is positive here, so the question is not whether to invest but how much the outcome depends on the bull case: it is 30% of the probability and 61% of the expected NPV.

---

### Decision Trees (v8.4.0)

Model sequential decisions and chance events to find optimal decision paths using expected value rollback.

**Use case**: Should we invest $2M in R&D? If technology succeeds (60% chance), should we license the IP or manufacture ourselves?

**Input (rnd_decision.yaml):**

```yaml
_forge_version: "5.1.0"

# Decision tree configuration
decision_tree:
  name: "R&D Investment Decision"

  # Root decision node
  root:
    type: decision
    name: "Invest in R&D?"
    branches:
      invest:
        cost: 2000000
        next: "tech_outcome"

      dont_invest:
        cost: 0
        value: 0  # Terminal: walk away with $0

  # Additional nodes
  nodes:
    tech_outcome:
      type: chance
      name: "Technology works?"
      branches:
        success:
          probability: 0.60
          next: "commercialize"

        failure:
          probability: 0.40
          value: 0  # Terminal: nothing more; the $2M is already on the invest branch

    commercialize:
      type: decision
      name: "How to commercialize?"
      branches:
        license:
          cost: 0
          value: 5000000  # Terminal: license revenue

        manufacture:
          cost: 3000000
          value: 8000000  # Terminal: manufacturing revenue
```

**Run:**

```bash
forge decision-tree rnd_decision.yaml
```

**Output** (trimmed):

```
   Optimal Path:
      → Invest in R&D? → invest
      → Technology works? → (await outcome)
      →   if success →
      →     How to commercialize? → license

   Expected Value: $1,000,000.00

   Decision Policy:
      At "commercialize": choose "license"
         ⚠️ equally good: manufacture (the expected values are the same; the choice is not made by the numbers)
      At "root": choose "invest"

   Risk Profile:
      Best case:  $3,000,000.00
      Worst case: -$2,000,000.00
      P(value > 0): 60.0%
```

Check by hand: licensing is worth 5,000,000 and manufacturing 8,000,000 − 3,000,000 = 5,000,000, a tie; so success is worth 5,000,000, and investing is worth 0.6 × 5,000,000 + 0.4 × 0 − 2,000,000 = 1,000,000. Best case 5,000,000 − 2,000,000 = 3,000,000; worst case −2,000,000.

**Use case explanation:**

Decision trees excel when:
- Decisions are sequential (decide, learn, decide again)
- You have discrete choice points with known probabilities
- You need to find the optimal strategy before taking action
- Explaining decision logic to stakeholders

**Key insight**: invest. The expected value is 1,000,000 against 0 for walking away, with a 40% chance of losing the 2,000,000 R&D cost. Licensing and manufacturing tie, and Forge says so instead of picking one silently: the numbers do not decide between them, so the choice comes down to what the tree leaves out, such as the extra 3,000,000 of capital and risk that manufacturing needs.

---

### Real Options (v8.5.0)

Value managerial flexibility to defer, expand, contract, or abandon projects. Recognizes that you can adapt as uncertainty resolves.

**Use case**: Factory investment with option to wait 2 years (defer), scale up if successful (expand), or sell assets if failing (abandon).

**Input:** `examples/real-options.yaml` (a $12M project that costs $10M to build, 30% volatility, 3 years, with `traditional_npv: 2000000`).

**Run:**

```bash
forge real-options examples/real-options.yaml
forge real-options examples/real-options.yaml -o options_results.yaml
```

**Output:**

| Option | Option value | Project value with it |
|--------|--------------|-----------------------|
| Wait up to 2 years before building (defer) | $3.25M | $3.25M (waiting already includes building now) |
| Double capacity if demand high (expand) | $7.12M | $9.12M ($2M NPV + $7.12M) |
| Sell assets for salvage value (abandon) | $0.002M | $2.00M ($2M NPV + $0.002M) |

How the project value is worked out:

- **Defer** already includes investing today as one choice, so the project value with it is the option value. Adding the NPV would count it twice.
- **Expand, abandon, contract, switch, compound** are held on top of a project you build now, so the project value is `traditional_npv` + option value.
- The options overlap (you cannot both wait and expand a factory you have built), so Forge compares them and reports the best one. It never adds them up.
- Without `traditional_npv` in `real_options`, Forge still values every option but leaves the decision open.

**Use case explanation:**

Real options are crucial when:
- Traditional NPV says "no" but intuition says "maybe"
- You have managerial flexibility (not locked in)
- Uncertainty resolves over time (you'll learn more)
- Investments are staged, reversible, or scalable

**Key insight**: building now and keeping the right to double capacity is worth $9.12M, against $2M for building with no flexibility. Waiting is worth $3.25M, which is $1.25M more than building now with no options, but less than building with the expansion option.

---

### Tornado Diagrams (v8.6.0)

Visualize which input variables drive the most output variance. Essential for focusing analysis effort on what matters.

**Use case**: Sensitivity analysis for NPV calculation. Which assumptions should we refine: revenue growth, discount rate, margin, or tax rate?

**Input (npv_sensitivity.yaml):**

```yaml
_forge_version: "5.1.0"

# Tornado configuration
tornado:
  output: "valuation.npv"  # Variable to analyze

  inputs:
    - name: "assumptions.revenue_growth"
      low: 0.02  # Pessimistic: 2% growth
      high: 0.08  # Optimistic: 8% growth
      base: 0.05  # Base case: 5% growth

    - name: "assumptions.discount_rate"
      low: 0.08  # Low discount rate
      high: 0.12  # High discount rate
      base: 0.10  # Base: 10% WACC

    - name: "assumptions.operating_margin"
      low: 0.15  # Pessimistic margin
      high: 0.25  # Optimistic margin
      base: 0.20  # Base margin

    - name: "assumptions.tax_rate"
      low: 0.20  # Low tax scenario
      high: 0.30  # High tax scenario
      base: 0.25  # Current rate

    - name: "assumptions.working_capital"
      low: 0.05  # Efficient WC management
      high: 0.15  # Higher WC needs
      base: 0.10  # Base assumption

# Model (simplified NPV calculation)
assumptions:
  revenue_growth: 0.05

  discount_rate: 0.10

  operating_margin: 0.20

  tax_rate: 0.25

  working_capital: 0.10

cash_flows:
  year: [0, 1, 2, 3, 4, 5]
  base_revenue: [0, 500000, 525000, 551250, 578813, 607753]

  revenue: "=IF(year = 0, 0, base_revenue * (1 + assumptions.revenue_growth))"
  operating_income: "=revenue * assumptions.operating_margin"
  taxes: "=operating_income * assumptions.tax_rate"
  nopat: "=operating_income - taxes"
  wc_investment: "=IF(year = 0, 0, (revenue - base_revenue) * assumptions.working_capital)"
  free_cash_flow: "=nopat - wc_investment"

valuation:
  npv:
    formula: "=NPV(assumptions.discount_rate, cash_flows.free_cash_flow)"
```

**Run:**

```bash
forge tornado npv_sensitivity.yaml
```

**Output:**

```
   Base value: 287711.48
   assumptions.operating_margin |██████████████████████████████| swing 148572.32  (output 213425.32 at low, 361997.64 at high)
   assumptions.discount_rate    |████████                      | swing 40958.89  (output 309199.00 at low, 268240.11 at high)
   assumptions.tax_rate         |███████                       | swing 39619.29  (output 307521.12 at low, 267901.83 at high)
   assumptions.working_capital  |█                             | swing 9433.16  (output 292428.06 at low, 282994.90 at high)
   assumptions.revenue_growth   |█                             | swing 5659.90  (output 284881.53 at low, 290541.43 at high)
```

Every bar matches a separate calculation of the same cash flows in Python (base NPV 287,711.48; margin at 15% and 25% gives 213,425.32 and 361,997.64).

**Use case explanation:**

Tornado diagrams are essential for:
- Prioritizing which assumptions to refine
- Explaining "key value drivers" to executives
- Validating models (unexpected sensitivities indicate errors)
- Deciding where to spend due diligence effort

**Key insight**: operating margin is 61% of the total swing; get it right before refining anything else. Revenue growth barely matters in this model, because it is a one-off uplift on each year's revenue rather than compounding growth, and part of the gain is eaten by the extra working capital. A tornado diagram can show a structural fact like this that intuition would get wrong.

---

### Bootstrap Resampling (v8.7.0)

Non-parametric confidence intervals from historical data. Let the data speak without assuming distributions.

**Use case**: Calculate confidence intervals for expected returns from 36 months of historical return data.

**Input (returns_bootstrap.yaml):**

```yaml
_forge_version: "5.1.0"

# Bootstrap configuration
bootstrap:
  iterations: 10000
  confidence_levels: [0.90, 0.95, 0.99]
  seed: 12345
  statistic: mean  # mean, median, std, var, percentile, min, max

  # Historical monthly returns (36 months)
  data: [
    0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01,
    0.06, 0.04, 0.02, -0.03, 0.07, 0.04, -0.02, 0.09,
    0.00, -0.04, 0.05, 0.03, 0.08, -0.01, 0.06, 0.02,
    0.04, 0.07, -0.02, 0.05, 0.03, -0.03, 0.06, 0.04,
    0.02, 0.05, -0.01, 0.08
  ]

# The same returns as a table, for formulas (bootstrap.data is a setting,
# not a column, so formulas cannot read it)
history:
  monthly_return: [
    0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01,
    0.06, 0.04, 0.02, -0.03, 0.07, 0.04, -0.02, 0.09,
    0.00, -0.04, 0.05, 0.03, 0.08, -0.01, 0.06, 0.02,
    0.04, 0.07, -0.02, 0.05, 0.03, -0.03, 0.06, 0.04,
    0.02, 0.05, -0.01, 0.08
  ]

# Reference calculation for comparison
analysis:
  sample_mean:
    formula: "=AVERAGE(history.monthly_return)"

  sample_std:
    formula: "=STDEV(history.monthly_return)"

  annualized_return:
    formula: "=sample_mean * 12"

  annualized_volatility:
    formula: "=sample_std * SQRT(12)"
```

**Run:**

```bash
forge bootstrap returns_bootstrap.yaml --output bootstrap_results.yaml
```

**Output** (`forge bootstrap returns_bootstrap.yaml`):

```
   Original statistic: 0.0292
   Bootstrap mean: 0.0292
   Bootstrap std error: 0.0068
   Bias: 0.0000

   Confidence Intervals:
      90% CI: [0.0181, 0.0403]
      95% CI: [0.0158, 0.0422]
      99% CI: [0.0119, 0.0464]
```

The 36 months average 2.92% a month. With 95% confidence the true average is between 1.58% and 4.22%. For comparison, assuming normality gives 2.92% ± 1.96 × 0.69% = [1.57%, 4.26%], almost the same, because these returns are roughly symmetric; bootstrap matters more for skewed data, fat tails or small samples.

**Use case explanation:**

Bootstrap resampling is powerful when:
- You have historical data but don't know the distribution
- Distribution is complex (multimodal, skewed, fat tails)
- You want to avoid parametric assumptions
- Validating results from parametric models

**Key insight**: with 95% confidence the average monthly return is between 1.58% and 4.22% (about 19% to 51% a year, simply × 12). Bootstrap gets this from the data alone, without assuming a normal distribution.

---

### Bayesian Networks

`examples/bayesian.yaml` models credit default risk as a chain, where each node has one parent (a node can have several; `examples/bagel/slow_week.yaml` has one with two):

```
[Economic Conditions] → [Company Revenue] → [Default Probability]
```

Run it:

```bash
forge bayesian examples/bayesian.yaml                                    # every node
forge bayesian examples/bayesian.yaml -q default_probability -e economic_conditions=bad
forge bayesian examples/bayesian.yaml -q economic_conditions -e default_probability=high
```

| Query | Result |
|-------|--------|
| `default_probability`, no evidence | low 48.25%, medium 29.70%, high 22.05% |
| `default_probability` given `economic_conditions=bad` | low 26.50%, medium 32.70%, high 40.80% |
| `economic_conditions` given `default_probability=high` | good 16.73%, neutral 46.26%, bad 37.01% |

**Key insight**: a bad economy alone raises the chance of a high default risk from 22% to 41%. Run backwards, a high default risk raises the chance the economy is bad from 20% to 37%. The numbers match a hand enumeration of the joint distribution.

A node can also have several parents. Each CPT row then names one state of each parent, in the order of `parents`:

```yaml
    grass:
      states: [dry, wet]
      parents: [rain, sprinkler]
      cpt:
        no, off:  [1.0, 0.0]
        yes, off: [0.2, 0.8]
        no, on:   [0.1, 0.9]
        yes, on:  [0.01, 0.99]
```

With rain at 20% and the sprinkler on 40% of the time, `forge bayesian model.yaml -q rain -e grass=wet` gives rain 37.82%, as a hand calculation does (0.1752 / 0.4632).

---

For more examples, see test-data/v1.0/ in the repository.
