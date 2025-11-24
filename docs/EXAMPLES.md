# 📖 Forge Examples

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
  final_price: [90.0, 127.5, 160.0]  # ✅ Calculated!
```

## Financial Models

### SaaS Metrics (v1.1.0)

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
  # v1.1.0 conditional aggregations
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

### Multi-Criteria Filtering (v1.1.0)

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

### Precision Control (v1.1.0)

```yaml
calculations:
  raw_values: [123.456, 789.123, 456.789]
  rounded: "=ROUND(raw_values, 2)"
  rounded_up: "=ROUNDUP(raw_values, 1)"
  rounded_down: "=ROUNDDOWN(raw_values, 0)"
```

For more examples, see test-data/v1.0/ in the repository.
