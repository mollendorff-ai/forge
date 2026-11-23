# Examples

Every file here runs as it is. Start with the first one, or open the same examples in your browser, with nothing to install, in [Forge Studio](https://forge.mollendorff.ai/).

```bash
forge calculate examples/starter.yaml
```

Installed with `install.sh --examples`, the same files are in `forge-examples/`: write that where this page says `examples/`.

## Models with a story

Each folder but `options/` and `games/` has a README with the numbers it should print; the games state theirs in `expect:`. All figures are made up.

| Example | What it is |
| --- | --- |
| [`starter.yaml`](starter.yaml) | A first model, commented line by line. Then `forge report examples/starter.yaml` shows formulas and values side by side. |
| [`bagel/`](bagel/) | A bagel shop's first year: seasonal months, costs in an included file, scenarios, a tornado, the year's actuals, and an uncertain version to simulate. |
| [`guide/`](guide/) | The model [lesson 2 of the guide](../docs/guide/02-a-first-model.md) writes from an empty file: the bagel shop's year in `02-bagel.yaml`, and the suppliers' and staff's costs it includes from `costs.yaml`. |
| [`mortgage/`](mortgage/) | A 450,000 CAD mortgage and what 300 more a month does. Canadian fixed rates compound twice a year, and the model says how. |
| [`household/`](household/) | One household's October: the budget against what was spent. |
| [`periods/`](periods/) | Two small models over time: a monthly plan and a five-year DCF, using period columns and `PREV`. |
| [`rolling-forecast/`](rolling-forecast/) | The model an FP&A team updates every month: actuals up to October, forecast for the rest. |
| [`capex-plan/`](capex-plan/) | A machine bought on a loan: payment schedule, depreciation and a revenue trend. |
| [`microgrid/`](microgrid/) | An island trading diesel for solar and a battery over 20 years: the battery's replacement, the loan, scenarios, a simulation and a decision tree. |
| [`realestate/`](realestate/) | A building of 24 flats valued under uncertainty: ten years of rent and costs, a sale at an exit cap rate, a loan, scenarios, a tornado, 10,000 simulated futures, and what the right to sell early is worth in a market that moves in cycles. |
| [`acquisition/`](acquisition/) | Project Kittiwake: a listed company bids 30% over the market for a smaller rival, part cash, part shares. Value created against the premium, earnings per share year by year (dilutive, then accretive), the walk-away price, an earn-out that moves the risk to the sellers, a rival bidder (a decision tree, and sealed bids as a game), comparable premiums, a diligence red flag, and the deal a year after closing. |
| [`vineyard/`](vineyard/) | A vineyard's season, where a big harvest sells cheap: the crop and the price correlated, a spring frost one year in ten, scenarios, a tornado, and the same season simulated with and without the correlation. The correlation cuts the chance of a loss from one season in three to one in four, and the mean with it. |
| [`fund/`](fund/) | A venture fund seen by its investors: calls, fees and exits on their own dates, the return by `XIRR` and the value by `XNPV`, fees and carry taking 15.7% down to 10.8%, and what a year's delay or a cold exit market costs. |
| [`inherited-budget/`](inherited-budget/) | A coffee roaster's budget handed over as a workbook someone else built: titles, merged headings, a total row, a named range and one month's staff cost typed in. `forge import` reads it to the workbook's own numbers and names what it could not keep, `forge audit` finds the typed number, a pay rise is made in the YAML, and `forge export` goes back to a workbook LibreOffice recalculates to the same numbers. |
| [`options/`](options/compare-options.yaml) | Two rentals compared. |
| [`games/`](games/) | Games and tournaments, run with `forge game`: kinship-protocol's stag hunt, chicken and Axelrod tournament with the flags it wrote by hand now read from the games; the inspection game, where a harsher penalty lowers the audit rate and not the cheating (`forge tornado`); a tournament where noise moves a forgiving strategy ahead of TitForTat; the battle of the sexes; the continuation at which cooperation holds (`forge sensitivity`); and three games of many players: the public goods game, the volunteer's dilemma, and hawks and doves generation by generation (`dynamics:`). |

Studio opens all of these, `games/` included, but not `inherited-budget/` (it starts from a workbook) or the one-command files below.

## One command each

Small files, each written for one analysis. The first comment of each file says how to run it.

| File | Command |
| --- | --- |
| [`breakeven.yaml`](breakeven.yaml) | `forge calculate` (break-even) |
| [`variance.yaml`](variance.yaml) | `forge calculate` (budget against actual) |
| [`scenarios.yaml`](scenarios.yaml) | `forge scenarios` |
| [`tornado.yaml`](tornado.yaml) | `forge tornado` |
| [`monte-carlo.yaml`](monte-carlo.yaml) | `forge simulate` |
| [`bootstrap.yaml`](bootstrap.yaml) | `forge bootstrap` |
| [`bayesian.yaml`](bayesian.yaml) | `forge bayesian` |
| [`decision-tree.yaml`](decision-tree.yaml) | `forge decision-tree` |
| [`real-options.yaml`](real-options.yaml) | `forge real-options` |
| [`quota_forecast.yaml`](quota_forecast.yaml) | `forge calculate` (nested references between groups) |

The long walkthrough is [docs/EXAMPLES.md](../docs/EXAMPLES.md).
