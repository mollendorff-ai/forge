# Periods and the row above

Two small models over time (ADR-053). Both use a period column, which generates the dates of the periods, and `PREV`, which reads the row above. All figures are made up.

- `monthly_plan.yaml`: a shop's 2027, month by month. The trend of sales grows 1% a month (`=PREV(trend * (1 + inputs.monthly_growth), inputs.january_trend)`), each calendar month has a season factor (`INDEX(season.factor, MONTH(month))`), and cash carries over (`=PREV(cash, inputs.opening_cash) + profit`). Sales for the year are 229,045.23, profit 18,846.23, and cash ends at 30,846.23 after a low of 10,309.52 in February. Two months lose money.
- `dcf.yaml`: a five-year discounted cash flow. Revenue grows from last year's (`=PREV(revenue, assumptions.revenue_2026) * (1 + assumptions.growth)`), and `ROW()` is the number of the year in the discount factor. The five years are worth 1,866,331.18, the terminal value 5,315,953.60 today, and the equity 5,682,284.78. `valuation.check` compares the five years with `NPV` of the same cash flows.

```bash
forge calculate examples/periods/monthly_plan.yaml --dry-run
forge calculate examples/periods/dcf.yaml --dry-run
forge sensitivity examples/periods/dcf.yaml -v assumptions.wacc -r 0.08,0.11,0.005 -o valuation.equity_value
forge export examples/periods/monthly_plan.yaml plan.xlsx
```

`tests/demo_examples.rs` checks these numbers against the same calculations written in Python. In the exported workbooks each `PREV` cell reads the cell above it; Gnumeric and LibreOffice recalculate both to the same numbers.

The loan schedule, the third example of the ADR, is in [`examples/mortgage`](../mortgage/README.md).
