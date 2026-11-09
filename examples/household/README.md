# A household budget against what was spent

One household's October: the budget (`budget.yaml`) and what actually came in and went out (`actual.yaml`). All figures are made up. The models were built through the MCP tools on 2026-11-09.

The side work brought in 150 instead of 400, and groceries, utilities, eating out and subscriptions went over while transport came in under. Expenses were 3,305.85 against 3,100 budgeted, so 2,044.15 was left over instead of 2,500, and the savings rate fell from 44.6% to 38.2%. With a 10% threshold, `forge variance` flags eight lines; transport is the only favourable one.

No line carries a `better:` of its own: anything under `income` is better higher and anything under `expenses` better lower, including a salary, which a business would count as a cost. Only `result` says `better: higher`, since nothing in those names says which way is good. `tests/demo_examples.rs` checks the totals, directions and alerts.

```bash
forge calculate examples/household/budget.yaml --dry-run
forge variance examples/household/budget.yaml examples/household/actual.yaml --threshold 10
forge variance examples/household/budget.yaml examples/household/actual.yaml -o variance.xlsx
```
