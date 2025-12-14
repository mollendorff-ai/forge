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
    value: null
    formula: "=SUM(saas_metrics.arr)"
  
  high_growth_months:
    value: null
    formula: "=COUNTIF(saas_metrics.mrr, > 15000)"
  
  avg_ltv_high_growth:
    value: null
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
    value: null
    formula: "=SUM(pl_2025_q1.revenue)"
  
  avg_gross_margin:
    value: null
    formula: "=AVERAGE(pl_2025_q1.gross_margin)"
```

### Monte Carlo Simulation (v8.0.0 - Enterprise)

**NPV Uncertainty Analysis:**

```yaml
_forge_version: "5.0.0"

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
    value: null
    formula: "=MC.Normal(0.15, 0.05)"  # 15% ± 5% annual growth

  initial_cost:
    value: null
    formula: "=MC.Triangular(80000, 100000, 150000)"  # min/mode/max

  discount_rate:
    value: null
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
    value: null
    formula: "=NPV(assumptions.discount_rate, cash_flows.net_cash_flow)"

  irr:
    value: null
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
_forge_version: "5.0.0"

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
    value: null
    formula: "=MC.Normal(600000, 50000)"  # Mean $600K, std dev $50K

  growth_rate:
    value: null
    formula: "=MC.Triangular(0.05, 0.10, 0.20)"  # min 5%, mode 10%, max 20%

  seasonality_factor:
    value: null
    formula: "=MC.Uniform(0.95, 1.15)"  # -5% to +15% seasonal variation

forecast:
  q4_revenue:
    value: null
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
_forge_version: "5.0.0"

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
    value: null
    formula: "=MC.Correlated([1000000, 0.30], [150000, 0.05], -0.6)"
    # [revenue_mean, margin_mean], [revenue_std, margin_std], correlation

financials:
  # Extract correlated values
  revenue:
    value: null
    formula: "=INDEX(correlated_inputs.revenue_margin, 0)"  # First element

  margin:
    value: null
    formula: "=INDEX(correlated_inputs.revenue_margin, 1)"  # Second element

  # Additional independent variables
  fixed_costs:
    value: null
    formula: "=MC.Normal(200000, 15000)"

  # Calculated outcomes
  gross_profit:
    value: null
    formula: "=revenue * margin"

  net_income:
    value: null
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
_forge_version: "5.0.0"

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
    value: null
    formula: "=MC.Normal(5000000, 800000)"

  market_share:
    value: null
    formula: "=MC.Triangular(0.05, 0.10, 0.18)"  # 5-18% share, mode 10%

  avg_price:
    value: null
    formula: "=MC.Normal(25, 3)"

  unit_cost:
    value: null
    formula: "=MC.Triangular(12, 15, 18)"

  opex:
    value: null
    formula: "=MC.Normal(150000, 20000)"

  initial_investment:
    value: 500000
    formula: null

projections:
  year_1_revenue:
    value: null
    formula: "=assumptions.market_size * assumptions.market_share"

  units_sold:
    value: null
    formula: "=year_1_revenue / assumptions.avg_price"

  cogs:
    value: null
    formula: "=units_sold * assumptions.unit_cost"

  gross_profit:
    value: null
    formula: "=year_1_revenue - cogs"

metrics:
  ebitda:
    value: null
    formula: "=projections.gross_profit - assumptions.opex"

  roi:
    value: null
    formula: "=ebitda / assumptions.initial_investment"

  payback_years:
    value: null
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
_forge_version: "5.0.0"

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
    value: null
    formula: "=MC.Uniform(1.0, 1.25)"  # 0-25% cost overrun

  resource_availability:
    value: null
    formula: "=MC.Triangular(1.0, 1.05, 1.30)"  # Usually 5%, up to 30% delay

project:
  base_cost:
    value: null
    formula: "=SUM(phases.design_cost) + SUM(phases.dev_cost) + SUM(phases.test_cost) + SUM(phases.deploy_cost)"

  total_cost:
    value: null
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
  tax_rate:
    value: 0.25
    formula: null

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
diff model.yaml model_roundtrip.yaml  # Should be identical!
```

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
    value: null
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

For more examples, see test-data/v1.0/ in the repository.
