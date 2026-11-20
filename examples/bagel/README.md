# Bagel Lune

A fictional wood-fired bagel shop in Mile End, Montreal, planned over its first twelve months. All numbers are made up. The model was built through the MCP tools on 2026-11-07 to see how Forge behaves on a model someone might actually have; it found the bugs fixed in PR 76.

- `bagel.yaml`: the plan. A monthly table (seasonality, growth, revenue, five cost lines, profit), a `year` group with totals, margin, NPV and the first year's return on the fit-out, and four scenarios. Supplier and wage costs come from `costs.yaml` through `_includes`, so a formula reads them as `@costs.ingredients.flour_per_kg`. The `flour_shock` scenario changes a value in that included file.
- `format:` (ADR-061): the price is `$#,##0.00`, the season `0%`, the dozens `#,##0` and the year's margin `0.0%`. A format changes how a number is shown, never its value: the exported workbook has these in its cells, `forge report` and Forge Studio show `$16.00` and `0.2%`, and every calculated number is the same. That makes the file model format 5.4.0.
- `costs.yaml`: ingredient prices, the loaded wage (a formula inside the included file) and the firewood.
- `bagel_simulate.yaml`: the same plan with uncertain opening demand (PERT) and staffing (triangular), for `forge simulate`, each with the plan's figure as its `value:`, with three thresholds and the expected shortfall of the year's profit, each month's chance of a loss, and a `rent_hike` scenario. `forge calculate` refuses it and says why.
- `costs_risk.yaml`: `costs.yaml` with the flour price (triangular, up to the flour-shock price) and the wage (uniform) uncertain, each with `costs.yaml`'s figure as its `value:`; it reads the other costs from `costs.yaml`.
- `actual.yaml`: what happened in the first year (fewer customers, a higher price, more staff hours, higher rent), for `forge variance` against the plan.
- `second_shop.yaml`: open a second shop now, run a three-month pop-up stall first, or stay with one shop (`forge decision-tree`).
- `wait_option.yaml`: what the right to wait a year before signing the second lease is worth, and the right to sell the oven if it fails (`forge real-options`).
- `first_month.yaml`: dozens sold on each of the first 30 days; how sure can we be about the average day (`forge bootstrap`)?
- `slow_week.yaml`: a slow week, with two possible causes, rain and roadwork (`forge bayesian`; `sales` has two parents).
- `price_war.yaml`: a price war with the bagel shop across the street (`forge game`, model format 5.5.0). Each month both shops hold $16 a dozen or cut to $14; the payoffs are each shop's margin before rent and staff, worked out from `bagel.yaml` and `costs.yaml`, with four new inputs (the street's 110 dozen a day, the 70% a cheaper shop takes, the cut price and 5% more sales in a price war). The rival is assumed to have the same costs. `price_war` is the month played once; `rivalry` plays it month after month between five pricing habits, with a 0.95 chance that both shops are open the next month. Two scenarios: `loyal_regulars` (a cheaper shop takes only 55%) and `lease_ending` (a 0.6 chance). In Forge Studio, open it from the bagel set's Files tab: the games are on the Model view and move as an input is dragged or a scenario tapped.

```bash
forge calculate examples/bagel/bagel.yaml --dry-run
forge scenarios examples/bagel/bagel.yaml
forge break-even examples/bagel/bagel.yaml --output-var year.profit --vary assumptions.price_per_dozen
forge goal-seek examples/bagel/bagel.yaml --target year.profit --value 60000 --vary assumptions.staff_hours_per_day
forge tornado examples/bagel/bagel.yaml
forge simulate examples/bagel/bagel_simulate.yaml
forge simulate examples/bagel/bagel_simulate.yaml --scenario rent_hike
forge simulate examples/bagel/bagel_simulate.yaml --seek "P(year.profit > 0) = 0.8" --vary price_per_dozen --min 14 --max 24
forge export examples/bagel/bagel.yaml bagel.xlsx
forge variance examples/bagel/bagel.yaml examples/bagel/actual.yaml
forge compare examples/bagel/bagel.yaml --scenarios base,flour_shock
forge decision-tree examples/bagel/second_shop.yaml
forge real-options examples/bagel/wait_option.yaml
forge bootstrap examples/bagel/first_month.yaml
forge bayesian examples/bagel/slow_week.yaml -q roadwork -e sales=slow -e weather=rainy
forge game examples/bagel/price_war.yaml
forge game examples/bagel/price_war.yaml --scenario loyal_regulars
forge game examples/bagel/price_war.yaml --scenario lease_ending
```

Expected: year profit 786.32 as planned; break-even price 15.97 a dozen; 24.27 staff hours a day for a 60,000 profit; the flour shock scenario gives -16,943.27. Second shop: run the pop-up first (expected value 61,500, against 58,500 for opening now). Waiting a year is worth 64,604.68; the right to sell the oven, 279.72. Average day: 50.7 dozen, 95% interval about 46.8 to 54.7. A slow week makes roadwork 40% likely; if it also rained, 27%. Against the plan, rent and staff hours are unfavorable and the first-year NPV is 29% worse. Under uncertainty (5,000 runs), the year's mean is -15,925.94 against 786.32 at plan: the uncertain inputs lean above their plan values (staff hours average 32.67 against 32, flour 1.30 against 1.15, the wage 19.00 against 18.50), and the year is linear in each, so their means alone take about 16,703 off it; `forge simulate` prints the plan beside each mean. The year makes money only 35% of the time, and opening demand drives profit most (rank correlation +0.84), then staff hours (-0.42) and the wage (-0.29); the uncertain flour price barely matters (-0.09), although the flour shock alone costs 17,700. The rent hike takes exactly 9,600 off every run. In the worst 5% of years the shop loses 91,760 on average. Month by month, November, January and February lose money in every run, and only July and August (the table's rows 9 and 10) are reliably profitable. For the year to make money in 80% of runs, the price has to be 18.32 a dozen instead of 16.

The price war: a month in which both hold earns each shop 21,489; cutting alone earns 25,585 and leaves the other 12,894; both cutting, 19,189. Cutting pays more whatever the rival does, so the month played once ends with both at $14, each 2,300 a month worse off than if both had held: a prisoner's dilemma the plan's numbers make. Month after month, the shop that always cuts scores most (21,065 a month on average), but only by taking the shop that always holds; matching the rival's last price (tit for tat) or holding until the first cut (grim trigger) scores 20,966 and cannot be beaten by a discounter, which needs both shops to be likely to meet again: a chance above (T - R) / (T - P) = 0.640. With loyal regulars, holding pays whatever the rival does and the discounter comes last (19,457). With the lease ending (0.6), only the discounter is stable.

All of these were checked against Python written independently of Forge (a hand-written binomial tree, numpy resampling, enumeration of the joint distribution, numpy and scipy on the simulation's own samples, nashpy and the axelrod library for the price war) and are asserted in `src/cli/commands/examples/mod.rs` (its `bagel_example` tests).
