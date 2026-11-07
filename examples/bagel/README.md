# Bagel Lune

A fictional wood-fired bagel shop in Mile End, Montreal, planned over its first twelve months. All numbers are made up. The model was built through the MCP tools on 2026-11-07 to see how Forge behaves on a model someone might actually have; it found the bugs fixed in PR 76.

- `bagel.yaml`: the plan. A monthly table (seasonality, growth, revenue, five cost lines, profit), a `year` group with totals, margin, NPV and payback, and four scenarios. Supplier and wage costs come from `costs.yaml` through `_includes`, so a formula reads them as `@costs.ingredients.flour_per_kg`. The `flour_shock` scenario changes a value in that included file.
- `costs.yaml`: ingredient prices, the loaded wage (a formula inside the included file) and the firewood.
- `bagel_simulate.yaml`: the same plan with uncertain opening demand (PERT) and staffing (triangular), for `forge simulate`. `forge calculate` refuses it and says why.

```bash
forge calculate examples/bagel/bagel.yaml --dry-run
forge scenarios examples/bagel/bagel.yaml
forge break-even examples/bagel/bagel.yaml --output year.profit --vary assumptions.price_per_dozen
forge goal-seek examples/bagel/bagel.yaml --target year.profit --value 60000 --vary assumptions.staff_hours_per_day
forge tornado examples/bagel/bagel.yaml
forge simulate examples/bagel/bagel_simulate.yaml
forge export examples/bagel/bagel.yaml bagel.xlsx
```

Expected: year profit 786.32 as planned; break-even price 15.97 a dozen; 24.27 staff hours a day for a 60,000 profit; the flour shock scenario gives -16,943.27. These were checked against a Python calculation written independently of Forge.
