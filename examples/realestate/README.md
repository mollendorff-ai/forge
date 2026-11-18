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

```bash
forge calculate examples/realestate/realestate.yaml --dry-run
forge scenarios examples/realestate/realestate.yaml
forge tornado examples/realestate/realestate.yaml
forge break-even examples/realestate/realestate.yaml --output value.npv --vary deal.price --min 4000000 --max 5500000
forge simulate examples/realestate/realestate_risk.yaml
```

`forge break-even` needs `--min` and `--max` here: far from the price, the equity return has no answer and the search stops on it.

**Checked outside Forge:** the building was written again in Python from this description, not from the model's formulas, and its plan, scenarios, tornado and break-even agree with Forge's to 1e-6. The Python is in `src/cli/commands/prediction/scenarios.rs`, and the tests are in a `realestate_*` module beside each command. A test calculates three of the simulated futures again with the plain model. The simulation's percentiles are Forge's own.

**What the model leaves out:** the flexibility of Geltner and de Neufville's *Flexibility and Real Estate Valuation under Uncertainty*: a market that moves year by year, and decision rules such as selling early, inside the simulation. Here each future draws one rent growth for all ten years, so the paths spread less than a real market's. Taxes, depreciation, refinancing and a monthly loan are not in it either.
