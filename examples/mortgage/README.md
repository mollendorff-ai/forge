# A mortgage with extra payments

A 450,000 CAD mortgage at 4.79% over 25 years, and what paying 300 more each month does. Canadian fixed rates compound twice a year while payments are monthly, so the monthly rate is `(1 + 4.79%/2)^(1/6) - 1`, not 4.79%/12. All figures are made up. The model was built through the MCP tools on 2026-11-09.

The payment is 2,563.69 a month and the interest over 25 years 319,106.29. With 300 more a month the loan is paid off in 247 payments (the last one 42.25), the interest is 254,509.41, and the extra payments save 64,596.88 and 53 months. The first year pays 21,056.17 of interest and 13,308.08 of principal (the `first_year` table).

`tests/demo_examples.rs` checks these numbers against a month-by-month amortisation loop written in Python, and the exported workbook, recalculated by Gnumeric, gives the same values. To save 80,000 of interest, the goal seek says to pay 397.04 more a month; the same loop, solved with SciPy, says 397.044208.

```bash
forge calculate examples/mortgage/mortgage.yaml --dry-run
forge goal-seek examples/mortgage/mortgage.yaml --target saved.interest --value 80000 --vary loan.extra_per_month
forge export examples/mortgage/mortgage.yaml mortgage.xlsx
```

Forge has no "previous row" yet (roadmap T2-TIME), so the `first_year` table computes each month's opening balance with the closed form instead of from the row before.
