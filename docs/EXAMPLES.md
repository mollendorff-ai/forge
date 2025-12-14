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
