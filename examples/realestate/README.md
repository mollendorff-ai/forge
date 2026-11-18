# A building of 24 flats, valued under uncertainty

Linden Court, a fictional building of 24 flats, is offered at 4,600,000 (191,667 a flat). What is it worth to a buyer who holds it ten years and sells, and what does that buyer earn on 35% equity with a 30-year loan for the rest? `realestate.yaml` is the building year by year. All figures are made up.

Each flat rents for 1,450 a month and brings 60 more in parking, storage and laundry; 6% of the rent is lost to empty flats and 1% is never paid. Running costs are 5,200 a flat a year, management takes 4% of the income, and 350 a flat a year is set aside for repairs. Rents grow 3% a year and costs 2.5%. In year 10 the building is sold at year 11's net operating income divided by a 5.75% cap rate, less 2% to sell. The buyer pays 1.5% on top of the price to close and asks 8% a year before debt. The loan, 65% of the price at 6%, is paid once a year.

| | |
|---|---|
| Value at 8% (ten years of cash and the sale) | 4,824,715.31 |
| Value less the price and closing costs | 155,715.31 |
| Going-in cap rate | 5.75% |
| Return before debt / on the equity | 8.44% / 11.37% |
| Equity multiple | 2.68 |
| Lowest debt cover (income ÷ loan payment) | 1.22, in year 1 |
| Sale, net of costs | 6,196,943.26 (59.5% of the value) |

The building is worth what it costs at a price of 4,753,414.10 (`forge break-even`). Of the four scenarios, a soft market (rents grow 1%, 10% vacancy, a 6.75% exit cap) gives -1,350,622.81 and an equity return of -1.8%; rates up two points gives -925,263.64 and a debt cover of 0.996 in year 1; a strong market gives 1,487,629.95. Weighted, the purchase is worth -129,179.92: worth buying in the plan, not once the scenarios are weighed. The tornado puts rent growth first, from -803,372.99 at 1% to 1,278,481.66 at 5%, then the exit cap rate and the buyer's discount rate.

- `realestate_risk.yaml`: the same building with five inputs uncertain: rent growth (normal, 3% ± 1.2), vacancy (triangular, 3% to 12%, most likely 6%), running costs (PERT, 4,600 to 6,200 a flat) and their growth (normal, 2.5% ± 1), and the exit cap rate (triangular, 5% to 7%, most likely 5.75%). In 10,000 futures the purchase is worth -28,741.40 at the median and more than nothing in 48.2% of them; the worst 5% lose 1,279,267.07 on average. Three of the five lean against the buyer, so the mean falls below the plan: this is the flaw of averages, and why one pro forma overstates the building. The bank's usual covenant, a cover of 1.20, is broken in about 55% of futures.

- `realestate_flex.yaml`: the flexibility to sell early, after Geltner and de Neufville's *Flexibility and Real Estate Valuation under Uncertainty* (2018). The building is bought for cash, and each future is a path: every year from the second, the market's rent takes a shock of its own (normal, 6%, one draw per row of the column `years.shock`) on top of the 3% trend, and 30% of the gap from the trend closes each year, so the market runs in cycles. The exit cap rate is uncertain as above. In each future the building is owned two ways: held ten years, as the pro forma says, or sold at the end of the first year from year 3 in which it would fetch, net, 135% of its price. The right to sell early is worth the difference.

  | 10,000 futures | Held 10 years | Sold by the rule |
  |---|---|---|
  | Mean value less cost | 87,995.39 | 188,577.55 |
  | Median | 76,292.98 | 211,522.98 |
  | 5th percentile | -631,629.60 | -627,939.59 |
  | Worth more than it costs | 56.8% | 62.8% |

  The rule sells early in 44.2% of futures, and the right to do so is worth 100,582.17 on average, two thirds of the plan's margin over the price. It never costs anything at plan, where the market follows its trend and the rule never fires. A lower bar, 125% (`--scenario sell_above_125`), sells in 69% of futures and is worth 112,484.05; a higher one, 150%, sells in 15% and is worth 42,272.80.

  The cycles are what make it pay. In a random walk (`--scenario random_walk`, where a gap from the trend never closes), the same rule is worth -54,006.87: selling after a rise gives up growth as likely to last as to reverse. Whether flexibility is worth anything is a question about the market, not the building.

```bash
forge calculate examples/realestate/realestate.yaml --dry-run
forge scenarios examples/realestate/realestate.yaml
forge tornado examples/realestate/realestate.yaml
forge break-even examples/realestate/realestate.yaml --output value.npv --vary deal.price --min 4000000 --max 5500000
forge simulate examples/realestate/realestate_risk.yaml
forge simulate examples/realestate/realestate_flex.yaml
forge simulate examples/realestate/realestate_flex.yaml --scenario random_walk
```

`forge break-even` needs `--min` and `--max` here: far from the price, the equity return has no answer and the search stops on it.

**Checked outside Forge:** the building was written again in Python from this description, not from the model's formulas, and its plan, scenarios, tornado and break-even agree with Forge's to 1e-6. The Python is in `src/cli/commands/prediction/scenarios.rs`, and the tests are in a `realestate_*` module beside each command. For the flexibility, three market paths (the plan, a bust then a boom, and an early boom that lasts) were written again in Python, and Forge's value held, value sold by the rule and year of sale agree with them; a test also checks that the right to sell early is worth more than nothing with cycles and less in a random walk (`src/cli/commands/simulate/mod.rs`, `realestate_flexibility`). Tests calculate three of each simulation's futures again with the plain model. The simulations' percentiles are Forge's own.

**What the model leaves out:** in `realestate_risk.yaml` each future draws one rent growth for all ten years, so the paths spread less than a real market's; `realestate_flex.yaml` moves the market year by year, but its buyers pay the same cap rate at a peak as in a trough, and its only flexibility is when to sell (not to expand, phase or refinance). Taxes, depreciation, refinancing and a monthly loan are not in it either.
