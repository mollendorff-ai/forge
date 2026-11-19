# An island's microgrid

Ilha Corva, a fictional island of 4,200 MWh a year, runs on diesel brought by boat. Should it build 3.4 MWp of solar and a 10 MWh battery, and keep its generators for what they cannot cover? `microgrid.yaml` is the island year by year for 20 years. All figures are made up.

How much solar the grid can take depends on the battery: about 30% of demand with no storage, 90 points more for each day's demand the battery holds, and no more than 90%. The battery fades 2.5% a year and is replaced in year 11. The project costs 8,140,000, 70% of it borrowed at 6.5% over 10 years.

| | |
|---|---|
| Net present value at 8% | 13,161,304.86 |
| Return on the project / on the equity | 23.0% / 41.1% |
| Paid back | year 5 |
| Lowest debt cover (cash ÷ loan payment) | 1.96, in year 1 |
| Cost of a MWh over 20 years, diesel / microgrid | 575.86 / 310.06 |
| Share of demand met by solar, year 1 / year 20 | 90% / 69.0% |
| Solar made but not used | 12.0% |
| CO2 avoided | 67,755 t |

Diesel would have to fall from 1.45 a litre to 0.58 for the project to be worth nothing (`forge break-even`). Of the four scenarios, cheap diesel hurts most: 5,402,750.37, with a lowest cover of 1.49. A fuel shock gives 24,675,150.91, a cloudy year with a hotel boom 12,797,727.12, and the weighted value is 13,263,955.32. The tornado puts the diesel price first, from 7,841,813.09 at 1.10 a litre to 20,760,578.82 at 1.95.

The plan's sizes are the best ones. A battery of 8 or 12 MWh is worth 11,934,790.42 or 12,708,614.06; 3.0 or 3.8 MWp of solar, 12,813,461.92 or 12,650,614.20. More solar pays only with more battery: without it, the grid cannot take the extra.

The loan's 10 years are the other choice. A 12-year loan still has a payment to make in year 11, when the battery is replaced, and in a cheap-diesel year that leaves almost nothing to make it with.

- `microgrid_risk.yaml`: the same plan with five inputs uncertain: the diesel price (triangular, 1.05 to 2.10) and its growth (normal, 3% ± 1.2), demand growth (uniform, 0 to 5%), sunshine (normal, 1,550 ± 80 MWh a MWp) and the battery price (PERT, 300,000 to 520,000 a MWh). In 10,000 futures the project is worth 13,705,188.09 at the median and 7,703,707.13 at the 5th percentile, and more than 5,000,000 in 99.6% of them. The loan's cover never falls below 1.2; its 5th percentile is 1.58.
- `microgrid_decision.yaml`: build the plan now, a smaller system now (2.8 MWp, 9 MWh), or wait a year in case batteries get 30% cheaper. Building the plan now is worth 13,792,484.13 in expected value; waiting, 13,697,669.33; the smaller system, 12,826,402.48. Waiting would win only if a fall in battery prices were more likely than 57%. Each of the tree's 36 numbers is `microgrid.yaml` calculated for that branch.

```bash
forge calculate examples/microgrid/microgrid.yaml --dry-run
forge scenarios examples/microgrid/microgrid.yaml
forge tornado examples/microgrid/microgrid.yaml
forge break-even examples/microgrid/microgrid.yaml --output verdict.project_npv --vary fuel.diesel_price
forge sensitivity examples/microgrid/microgrid.yaml --vary system.battery_mwh -r 4,16,2 --output-var verdict.project_npv
forge simulate examples/microgrid/microgrid_risk.yaml
forge decision-tree examples/microgrid/microgrid_decision.yaml
```

**Checked outside Forge:** the island was written again in Python from this description, not from the model's formulas, and its plan, scenarios, tornado, break-even and sizes agree with Forge's to 1e-6; the tree, whose numbers are rounded to the unit, within 1. The Python is in `src/cli/commands/prediction/scenarios.rs`, and the tests are in a `microgrid_*` module beside each command. A test calculates each of the tree's 36 numbers again from `microgrid.yaml`, so the tree cannot fall behind the model, and three of the simulated futures again with the plain model. The simulation's percentiles are Forge's own.

**What the model leaves out:** it counts by the year, so it has no hours, seasons or outages, and the share of solar the grid can take is a rule of thumb, not a study of the grid. Taxes, insurance, land, building delays and the inflation of anything but diesel are not in it.
