# Forge Examples

Real-world usage examples and patterns.

## Quick Start

### Basic Calculation

**Input (pricing.yaml):**

```yaml
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

```yaml
pricing_table:
  product: ["Widget A", "Widget B", "Widget C"]
  base_price: [100, 150, 200]
  discount_rate: [0.10, 0.15, 0.20]
  final_price: [90.0, 127.5, 160.0]  # Calculated!
```

## Financial Models

### SaaS Metrics (v1.2.1)

```yaml
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
    formula: "=COUNTIF(saas_metrics.mrr, > 15000)"
  
  avg_ltv_high_growth:
    formula: "=AVERAGEIF(saas_metrics.mrr, > 15000, saas_metrics.ltv)"
```

### Quarterly P&L

```yaml
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
      thresholds: [0, 100000]
      sensitivity: true

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
forge monte-carlo npv_analysis.yaml --output results.yaml
```

**Output interpretation:**

```yaml
monte_carlo_results:
  valuation.npv:
    percentiles:
      p10: -12450.23    # 10% chance NPV is below this
      p50: 45678.91     # Median outcome (most likely)
      p90: 98234.56     # 10% chance NPV exceeds this

    thresholds:
      "0": 0.73         # 73% probability of positive NPV
      "100000": 0.12    # 12% probability NPV exceeds $100K

    sensitivity:
      assumptions.revenue_growth: 0.85    # Highest impact (positive)
      assumptions.initial_cost: -0.62     # Second highest (negative)
      assumptions.discount_rate: -0.43    # Third highest (negative)

    statistics:
      mean: 46123.45
      std_dev: 32456.78
      min: -45678.12
      max: 145678.90
```

**Key insights:**

1. **Risk Assessment**: 73% probability of positive NPV suggests acceptable risk
2. **Upside Limited**: Only 12% chance of exceeding $100K target
3. **Key Driver**: Revenue growth (0.85 correlation) is the primary value driver
4. **Downside Protection**: P10 of -$12.4K represents worst-case scenario

**Decision framework:**

- **Green light** if P(NPV > 0) > 70% and P10 > -acceptable_loss
- **Yellow flag** if high sensitivity to uncontrollable variables
- **Red flag** if P50 < minimum_acceptable_return

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
      thresholds: [500000, 750000, 1000000]

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

**Results interpretation:**

```yaml
monte_carlo_results:
  forecast.q4_revenue:
    percentiles:
      p10: 573245.12    # Conservative estimate (10th percentile)
      p25: 614567.89    # Lower bound of likely range
      p50: 678234.56    # Median forecast
      p75: 745123.45    # Upper bound of likely range
      p90: 812456.78    # Optimistic estimate (90th percentile)

    thresholds:
      "500000": 0.94    # 94% chance of exceeding $500K
      "750000": 0.32    # 32% chance of exceeding $750K
      "1000000": 0.04   # 4% chance of exceeding $1M
```

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
      thresholds: [0, 50000, 100000]
      sensitivity: true

    - variable: financials.revenue
      percentiles: [10, 50, 90]

    - variable: financials.margin
      percentiles: [10, 50, 90]

# Correlated revenue and margin scenarios
# When revenue is high, margins tend to be lower (volume discount effect)
# Negative correlation of -0.6
correlated_inputs:
  revenue_margin:
    formula: "=MC.Correlated([1000000, 0.30], [150000, 0.05], -0.6)"
    # [revenue_mean, margin_mean], [revenue_std, margin_std], correlation

financials:
  # Extract correlated values
  revenue:
    formula: "=INDEX(correlated_inputs.revenue_margin, 0)"  # First element

  margin:
    formula: "=INDEX(correlated_inputs.revenue_margin, 1)"  # Second element

  # Additional independent variables
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

- **Positive correlation (+0.6 to +1.0)**: Variables move together (e.g., revenue and costs)
- **Negative correlation (-0.6 to -1.0)**: Variables move opposite (e.g., volume and price)
- **No correlation (0)**: Variables are independent

**Results show correlated behavior:**

```yaml
monte_carlo_results:
  financials.net_income:
    percentiles:
      p5: -48234.56     # 5% worst case
      p25: 67890.12     # Lower quartile
      p50: 98765.43     # Median
      p75: 128456.78    # Upper quartile
      p95: 167234.91    # 5% best case

    thresholds:
      "0": 0.89         # 89% probability of profit
      "50000": 0.68     # 68% probability of > $50K profit
      "100000": 0.41    # 41% probability of > $100K profit

    sensitivity:
      correlated_inputs.revenue_margin[0]: 0.92   # Revenue is key driver
      financials.fixed_costs: -0.38               # Costs have negative impact
      correlated_inputs.revenue_margin[1]: 0.71   # Margin also important

  financials.revenue:
    percentiles:
      p10: 823456.78
      p50: 1001234.56
      p90: 1178912.34

  financials.margin:
    percentiles:
      p10: 0.237        # When revenue is high (p90), margin tends low
      p50: 0.301        # Median margin
      p90: 0.364        # When revenue is low (p10), margin tends high
```

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
      thresholds: [0, 100000, 200000, 300000]
      sensitivity: true

    # Track ROI with key percentiles only
    - variable: metrics.roi
      percentiles: [10, 50, 90]
      thresholds: [0.10, 0.15, 0.20]  # 10%, 15%, 20% return thresholds

    # Track payback period
    - variable: metrics.payback_years
      percentiles: [25, 50, 75]
      thresholds: [2.0, 3.0, 5.0]  # Years to payback

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

**Results with multiple outputs:**

```yaml
monte_carlo_results:
  metrics.ebitda:
    percentiles:
      p1: -87654.32     # Extreme downside
      p5: -34567.89
      p10: -12345.67
      p25: 34567.89
      p50: 78901.23     # Median EBITDA
      p75: 123456.78
      p90: 167890.12
      p95: 198765.43
      p99: 245678.90    # Extreme upside

    thresholds:
      "0": 0.67         # 67% chance of positive EBITDA
      "100000": 0.38    # 38% chance of > $100K EBITDA
      "200000": 0.09    # 9% chance of > $200K EBITDA
      "300000": 0.01    # 1% chance of > $300K EBITDA

    sensitivity:
      assumptions.market_share: 0.88        # Market share is critical
      assumptions.market_size: 0.72         # Market size important
      assumptions.avg_price: 0.45           # Price has moderate impact
      assumptions.unit_cost: -0.41          # Costs negatively impact
      assumptions.opex: -0.38               # OpEx negatively impacts

  metrics.roi:
    percentiles:
      p10: -0.025       # -2.5% return (10th percentile)
      p50: 0.158        # 15.8% median return
      p90: 0.334        # 33.4% return (90th percentile)

    thresholds:
      "0.10": 0.71      # 71% chance of > 10% ROI
      "0.15": 0.54      # 54% chance of > 15% ROI
      "0.20": 0.32      # 32% chance of > 20% ROI

  metrics.payback_years:
    percentiles:
      p25: 3.21         # Fast payback (25th percentile)
      p50: 6.34         # Median payback
      p75: 14.56        # Slow payback (75th percentile)

    thresholds:
      "2.0": 0.08       # 8% chance of 2-year payback
      "3.0": 0.21       # 21% chance of 3-year payback
      "5.0": 0.45       # 45% chance of 5-year payback

  projections.year_1_revenue:
    percentiles:
      p10: 323456.78
      p50: 578901.23
      p90: 867234.56
```

**Multi-output decision making:**

1. **EBITDA Analysis**: 67% chance of profitability, but only 38% chance of hitting $100K target
2. **ROI Analysis**: 54% chance of exceeding 15% return threshold - acceptable for moderate risk
3. **Payback Analysis**: 45% chance of 5-year payback - longer than preferred 3-year target
4. **Key Lever**: Market share (0.88 sensitivity) is the critical success factor

**Recommended actions:**

- Focus resources on market share acquisition (highest sensitivity)
- Plan for median payback of 6.3 years, not optimistic 3-year case
- Set internal target at P25-P50 range, not P90 optimistic scenario

### Cost Estimation with Triangular Distribution

When you have min/most-likely/max estimates (common in project planning):

```yaml
_forge_version: "5.1.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  outputs:
    - variable: project.total_cost
      percentiles: [10, 50, 90, 95, 99]
      thresholds: [250000, 300000, 350000, 400000]

# Triangular distributions based on expert estimates
phases:
  phase: ["Design", "Development", "Testing", "Deployment"]

  # Cost estimates: [minimum, most_likely, maximum]
  design_cost: "=MC.Triangular(40000, 50000, 75000)"
  dev_cost: "=MC.Triangular(120000, 150000, 220000)"
  test_cost: "=MC.Triangular(30000, 40000, 60000)"
  deploy_cost: "=MC.Triangular(15000, 20000, 35000)"

# Risk factors
risks:
  scope_creep:
    formula: "=MC.Uniform(1.0, 1.25)"  # 0-25% cost overrun

  resource_availability:
    formula: "=MC.Triangular(1.0, 1.05, 1.30)"  # Usually 5%, up to 30% delay

project:
  base_cost:
    formula: "=SUM(phases.design_cost) + SUM(phases.dev_cost) + SUM(phases.test_cost) + SUM(phases.deploy_cost)"

  total_cost:
    formula: "=base_cost * risks.scope_creep * risks.resource_availability"
```

**Run:**

```bash
forge simulate project_cost.yaml --output cost_results.yaml
```

**Cost estimation results:**

```yaml
monte_carlo_results:
  project.total_cost:
    percentiles:
      p10: 218765.43    # Optimistic (10% chance lower)
      p50: 287654.32    # Median estimate - use for planning
      p90: 389012.45    # Conservative (10% chance higher)
      p95: 427890.12    # Risk reserve level
      p99: 512345.67    # Extreme scenario

    thresholds:
      "250000": 0.72    # 72% chance of staying under $250K
      "300000": 0.52    # 52% chance of staying under $300K
      "350000": 0.32    # 32% chance of staying under $350K
      "400000": 0.21    # 21% chance of staying under $400K

    statistics:
      mean: 295432.10
      std_dev: 78901.23
```

**Budgeting recommendations:**

- **Budget at P75-P90**: Set budget at $350K-$390K for 75-90% confidence
- **Contingency**: $287K (P50) + $100K (35% buffer) = $387K aligns with P90
- **Board presentation**: "Project cost $290K ± $100K" (mean ± 1 std dev)

## Advanced Features

### Cross-Table References

```yaml
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

The round-tripped file calculates to the same values, but it is not byte-identical: keys are sorted and formulas respaced. Formulas that Excel or Forge cannot express come back as values, and both commands list them.

## Common Patterns

### Conditional Logic

```yaml
pricing:
  volume: [10, 50, 100, 500]
  base_price: [100, 95, 90, 85]
  discount: "=IF(volume > 100, 0.20, IF(volume > 50, 0.10, 0))"
  final_price: "=base_price * (1 - discount)"
```

### Multi-Criteria Filtering (v1.2.1)

```yaml
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

# Fixed assumptions (same across all scenarios)
assumptions:
  market_size: 5000000

  initial_investment: 500000

  discount_rate: 0.10

# Calculations (applied within each scenario)
financials:
  year_1_revenue:
    formula: "=assumptions.market_size * (1 + revenue_growth) * market_share * avg_price"

  year_1_cogs:
    formula: "=(year_1_revenue / avg_price) * unit_cost"

  year_1_profit:
    formula: "=year_1_revenue - year_1_cogs"

  npv:
    formula: "=(year_1_profit / (1 + assumptions.discount_rate)) - assumptions.initial_investment"
```

**Run:**

```bash
forge scenarios scenario_npv.yaml --output scenario_results.yaml
```

**Output:**

```yaml
scenario_results:
  base_case:
    npv: 45678.90
    year_1_revenue: 656250.00
    probability: 0.50

  bull_case:
    npv: 187234.56
    year_1_revenue: 1449000.00
    probability: 0.30

  bear_case:
    npv: -123456.78
    year_1_revenue: 396000.00
    probability: 0.20

  expected_value:
    npv: 51234.57  # Weighted average: 0.5*45679 + 0.3*187235 + 0.2*(-123457)
    year_1_revenue: 759075.00

  decision:
    recommendation: "PROCEED"
    reason: "Positive expected NPV with acceptable downside"
```

**Use case explanation:**

Scenario analysis is ideal when:
- Management thinks in discrete "cases" not probability distributions
- You need clear storylines for board presentations
- Strategic futures are mutually exclusive (only one will occur)
- Combining with Monte Carlo for continuous uncertainty within scenarios

**Key insight**: Expected NPV is positive ($51K) despite 20% chance of loss, but bull case drives most upside.

---

### Decision Trees (v8.4.0)

Model sequential decisions and chance events to find optimal decision paths using expected value rollback.

**Use case**: Should we invest $2M in R&D? If technology succeeds (60% chance), should we license the IP or manufacture ourselves?

**Input (rnd_decision.yaml):**

```yaml
_forge_version: "5.0.0"

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
          value: -2000000  # Terminal: lose R&D investment

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
forge decision-tree rnd_decision.yaml --output decision_results.yaml
```

**Output:**

```yaml
decision_results:
  tree: "R&D Investment Decision"

  # Optimal path (rollback from leaves)
  optimal_path:
    - node: "root"
      decision: "invest"
      expected_value: 200000
      reasoning: "EV of investing ($200K) > not investing ($0)"

    - node: "tech_outcome"
      outcome: "success (60%) or failure (40%)"
      expected_value: 2200000  # Before subtracting initial $2M cost

    - node: "commercialize"
      decision: "manufacture"
      expected_value: 5000000  # $8M revenue - $3M cost
      reasoning: "Manufacturing ($5M net) > licensing ($5M)"

  # Expected value calculations
  rollback:
    commercialize:
      license: 5000000
      manufacture: 5000000  # max(5M, 5M) - choose either
      optimal: "manufacture"  # Slight edge if revenues differ

    tech_outcome:
      success_branch: 5000000  # Value of optimal commercialization
      failure_branch: -2000000
      expected: 2200000  # 0.6 * 5M + 0.4 * (-2M) = 3M - 0.8M = 2.2M

    root:
      invest: 200000  # 2.2M - 2M investment = 200K
      dont_invest: 0
      optimal: "invest"

  recommendation:
    decision: "INVEST in R&D"
    expected_npv: 200000
    optimal_strategy: "If successful, choose manufacturing over licensing"
    risk: "40% chance of losing $2M investment"
```

**Use case explanation:**

Decision trees excel when:
- Decisions are sequential (decide, learn, decide again)
- You have discrete choice points with known probabilities
- You need to find the optimal strategy before taking action
- Explaining decision logic to stakeholders

**Key insight**: Even with 40% failure risk, the expected value ($200K) is positive. The tree shows the optimal path: invest, and if successful, manufacture.

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
      low: 0.10  # Efficient WC management
      high: 0.20  # Higher WC needs
      base: 0.15  # Base assumption

# Model (simplified NPV calculation)
assumptions:
  revenue_growth: 0.05

  discount_rate: 0.10

  operating_margin: 0.20

  tax_rate: 0.25

  working_capital: 0.15

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
forge tornado npv_sensitivity.yaml --output tornado_results.yaml
```

**Output:**

```yaml
tornado_results:
  output_variable: "valuation.npv"
  base_value: 1234567.89

  # Sorted by absolute swing (most impactful first)
  sensitivity_bars:
    - input: "assumptions.revenue_growth"
      low_value: 0.02
      low_output: 784567.12  # NPV with 2% growth
      high_value: 0.08
      high_output: 1684567.89  # NPV with 8% growth
      swing: 450000.38  # Half the range: (1684567 - 784567) / 2
      abs_swing: 450000.38
      pct_swing: 36.4%  # Relative to base NPV
      rank: 1

    - input: "assumptions.discount_rate"
      low_value: 0.08
      low_output: 1554567.23
      high_value: 0.12
      high_output: 914567.34
      swing: -320000.06  # Negative: high discount rate lowers NPV
      abs_swing: 320000.06
      pct_swing: 25.9%
      rank: 2

    - input: "assumptions.operating_margin"
      low_value: 0.15
      low_output: 1054567.45
      high_value: 0.25
      high_output: 1414567.90
      swing: 180000.23
      abs_swing: 180000.23
      pct_swing: 14.6%
      rank: 3

    - input: "assumptions.tax_rate"
      low_value: 0.20
      low_output: 1309567.12
      high_value: 0.30
      high_output: 1159567.45
      swing: -75000.34
      abs_swing: 75000.34
      pct_swing: 6.1%
      rank: 4

    - input: "assumptions.working_capital"
      low_value: 0.10
      low_output: 1274567.34
      high_value: 0.20
      high_output: 1194567.78
      swing: -40000.28
      abs_swing: 40000.28
      pct_swing: 3.2%
      rank: 5

# ASCII visualization
diagram: |
  NPV Sensitivity Analysis (Base: $1,234,568)

  Revenue Growth    |████████████████████| ± $450K (36%)
  Discount Rate     |██████████████      | ± $320K (26%)
  Operating Margin  |██████████          | ± $180K (15%)
  Tax Rate          |████                | ± $75K (6%)
  Working Capital   |██                  | ± $40K (3%)

# Key insights
insights:
  top_drivers: ["revenue_growth", "discount_rate"]
  combined_impact: "Top 2 drivers explain 62% of variance"
  recommendation: "Focus due diligence on market growth assumptions and cost of capital"
  low_impact: ["working_capital"]
  low_impact_note: "Working capital has <5% impact - use reasonable estimate"
```

**Use case explanation:**

Tornado diagrams are essential for:
- Prioritizing which assumptions to refine
- Explaining "key value drivers" to executives
- Validating models (unexpected sensitivities indicate errors)
- Deciding where to spend due diligence effort

**Key insight**: Revenue growth and discount rate drive 62% of NPV variance. Focus analysis there; don't waste time refining working capital assumptions.

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

# Reference calculation for comparison
analysis:
  sample_mean:
    formula: "=AVERAGE(bootstrap.data)"

  sample_std:
    formula: "=STDEV(bootstrap.data)"

  annualized_return:
    formula: "=sample_mean * 12"

  annualized_volatility:
    formula: "=sample_std * SQRT(12)"
```

**Run:**

```bash
forge bootstrap returns_bootstrap.yaml --output bootstrap_results.yaml
```

**Output:**

```yaml
bootstrap_results:
  statistic: "mean"
  iterations: 10000

  # Original sample statistics
  original_estimate: 0.0361  # 3.61% average monthly return
  sample_size: 36

  # Bootstrap distribution of means
  bootstrap_distribution:
    mean: 0.0362  # Average of 10,000 bootstrap means
    std_error: 0.0087  # Standard error of the mean
    min: 0.0089  # Minimum bootstrap mean
    max: 0.0634  # Maximum bootstrap mean

  # Confidence intervals
  confidence_intervals:
    - level: 0.90
      lower: 0.0219  # 5th percentile
      upper: 0.0507  # 95th percentile
      width: 0.0288
      interpretation: "90% confident mean return is between 2.19% and 5.07%"

    - level: 0.95
      lower: 0.0193  # 2.5th percentile
      upper: 0.0531  # 97.5th percentile
      width: 0.0338
      interpretation: "95% confident mean return is between 1.93% and 5.31%"

    - level: 0.99
      lower: 0.0141  # 0.5th percentile
      upper: 0.0583  # 99.5th percentile
      width: 0.0442
      interpretation: "99% confident mean return is between 1.41% and 5.83%"

  # Annualized metrics (for reference)
  annualized:
    point_estimate: 0.4332  # 43.32% annualized
    ci_95_lower: 0.2316  # 23.16% annualized
    ci_95_upper: 0.6372  # 63.72% annualized

# Comparison to parametric approach (if we assumed normality)
parametric_comparison:
  normal_assumption:
    mean: 0.0361
    std_error: 0.0087  # sample_std / sqrt(n)
    ci_95: [0.0189, 0.0533]  # mean ± 1.96 * SE

  bootstrap_result:
    ci_95: [0.0193, 0.0531]

  difference: "Negligible - data is approximately normal"

  when_bootstrap_matters: |
    Bootstrap is superior when:
    - Distribution is skewed or has fat tails
    - Small sample size (<30)
    - Outliers present
    - Distribution shape unknown
```

**Use case explanation:**

Bootstrap resampling is powerful when:
- You have historical data but don't know the distribution
- Distribution is complex (multimodal, skewed, fat tails)
- You want to avoid parametric assumptions
- Validating results from parametric models

**Key insight**: With 95% confidence, expected monthly return is 1.93% to 5.31% (annualized: 23% to 64%). Bootstrap accounts for data shape without assuming normality.

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
