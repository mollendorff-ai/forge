# Bagel Lune

A fictional wood-fired bagel shop in Mile End, Montreal, planned over its first twelve months. All numbers are made up. The model was built through the MCP tools on 2026-11-07 to see how Forge behaves on a model someone might actually have; it found the bugs fixed in PR 76.

- `bagel.yaml`: the plan. A monthly table (seasonality, growth, revenue, five cost lines, profit), a `year` group with totals, margin, NPV and payback, and four scenarios. Supplier and wage costs come from `costs.yaml` through `_includes`, so a formula reads them as `@costs.ingredients.flour_per_kg`. The `flour_shock` scenario changes a value in that included file.
- `costs.yaml`: ingredient prices, the loaded wage (a formula inside the included file) and the firewood.
- `bagel_simulate.yaml`: the same plan with uncertain opening demand (PERT) and staffing (triangular), for `forge simulate`, with three thresholds on the year's profit and a `rent_hike` scenario. `forge calculate` refuses it and says why.
- `costs_risk.yaml`: `costs.yaml` with the flour price (triangular, up to the flour-shock price) and the wage (uniform) uncertain; it reads the other costs from `costs.yaml`.
- `actual.yaml`: what happened in the first year (fewer customers, a higher price, more staff hours, higher rent), for `forge variance` against the plan.
- `second_shop.yaml`: open a second shop now, run a three-month pop-up stall first, or stay with one shop (`forge decision-tree`).
- `wait_option.yaml`: what the right to wait a year before signing the second lease is worth, and the right to sell the oven if it fails (`forge real-options`).
- `first_month.yaml`: dozens sold on each of the first 30 days; how sure can we be about the average day (`forge bootstrap`)?
- `slow_week.yaml`: a slow week, with two possible causes, rain and roadwork (`forge bayesian`; `sales` has two parents).

```bash
forge calculate examples/bagel/bagel.yaml --dry-run
forge scenarios examples/bagel/bagel.yaml
forge break-even examples/bagel/bagel.yaml --output year.profit --vary assumptions.price_per_dozen
forge goal-seek examples/bagel/bagel.yaml --target year.profit --value 60000 --vary assumptions.staff_hours_per_day
forge tornado examples/bagel/bagel.yaml
forge simulate examples/bagel/bagel_simulate.yaml
forge simulate examples/bagel/bagel_simulate.yaml --scenario rent_hike
forge export examples/bagel/bagel.yaml bagel.xlsx
forge variance examples/bagel/bagel.yaml examples/bagel/actual.yaml
forge compare examples/bagel/bagel.yaml --scenarios base,flour_shock
forge decision-tree examples/bagel/second_shop.yaml
forge real-options examples/bagel/wait_option.yaml
forge bootstrap examples/bagel/first_month.yaml
forge bayesian examples/bagel/slow_week.yaml -q roadwork -e sales=slow -e weather=rainy
```

Expected: year profit 786.32 as planned; break-even price 15.97 a dozen; 24.27 staff hours a day for a 60,000 profit; the flour shock scenario gives -16,943.27. Second shop: run the pop-up first (expected value 61,500, against 58,500 for opening now). Waiting a year is worth 64,604.68; the right to sell the oven, 279.72. Average day: 50.7 dozen, 95% interval about 46.8 to 54.7. A slow week makes roadwork 40% likely; if it also rained, 27%. Against the plan, rent and staff hours are unfavorable and the first-year NPV is 29% worse. Under uncertainty (5,000 runs), the year makes money only 35% of the time, and opening demand drives profit most (rank correlation +0.84), then staff hours (-0.42) and the wage (-0.29); the uncertain flour price barely matters (-0.09), although the flour shock alone costs 17,700. The rent hike takes exactly 9,600 off every run.

All of these were checked against Python written independently of Forge (a hand-written binomial tree, numpy resampling, enumeration of the joint distribution, numpy and scipy on the simulation's own samples) and are asserted in `tests/bagel_example.rs`.
