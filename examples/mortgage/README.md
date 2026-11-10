# A mortgage with extra payments

A 450,000 CAD mortgage at 4.79% over 25 years, and what paying 300 more each month does. Canadian fixed rates compound twice a year while payments are monthly, so the monthly rate is `(1 + 4.79%/2)^(1/6) - 1`, not 4.79%/12. All figures are made up. The model was built through the MCP tools on 2026-11-09; the schedule was added with `PREV` and a period column (ADR-053, format 5.2.0).

The payment is 2,563.69 a month and the interest over 25 years 319,106.29. With 300 more a month the loan is paid off in 247 payments (the last one 42.25, in June 2047), the interest is 254,509.41, and the extra payments save 64,596.88 and 53 months. The first year pays 21,056.17 of interest and 13,308.08 of principal.

The model says this twice. The scalar groups use closed forms (`PMT`, `NPER`). The `schedule` table is the loan month by month for 300 months: each month opens with what the month before closed with (`opening: "=PREV(closing, loan.principal)"`), the last payment is whatever is left, and the months after it are zeros. The `check` group is the difference between the two, and is all zeros.

`tests/demo_examples.rs` checks these numbers against a month-by-month amortisation loop written in Python, and the schedule's balances against the loop's in months 1, 12, 60, 120, 246 and 247. The exported workbook, recalculated by Gnumeric and by LibreOffice, gives the same values; in it each opening balance reads the cell of the closing balance above. To save 80,000 of interest, the goal seek says to pay 397.04 more a month; the same loop, solved with SciPy, says 397.044208.

```bash
forge calculate examples/mortgage/mortgage.yaml --dry-run
forge goal-seek examples/mortgage/mortgage.yaml --target saved.interest --value 80000 --vary loan.extra_per_month
forge export examples/mortgage/mortgage.yaml mortgage.xlsx
```
