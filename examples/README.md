# Examples

Every file here runs as it is. Start with the first one, or open the same examples in your browser, with nothing to install, in [Forge Studio](https://mollendorff-ai.github.io/forge/).

```bash
forge calculate examples/starter.yaml
```

## Models with a story

Each has a README with the numbers it should print. All figures are made up.

| Example | What it is |
| --- | --- |
| [`starter.yaml`](starter.yaml) | A first model, commented line by line. Then `forge report examples/starter.yaml` shows formulas and values side by side. |
| [`bagel/`](bagel/) | A bagel shop's first year: seasonal months, costs in an included file, scenarios, a tornado, the year's actuals, and an uncertain version to simulate. |
| [`mortgage/`](mortgage/) | A 450,000 CAD mortgage and what 300 more a month does. Canadian fixed rates compound twice a year, and the model says how. |
| [`household/`](household/) | One household's October: the budget against what was spent. |
| [`periods/`](periods/) | Two small models over time: a monthly plan and a five-year DCF, using period columns and `PREV`. |
| [`rolling-forecast/`](rolling-forecast/) | The model an FP&A team updates every month: actuals up to October, forecast for the rest. |
| [`capex-plan/`](capex-plan/) | A machine bought on a loan: payment schedule, depreciation and a revenue trend. |
| [`options/`](options/compare-options.yaml) | Two rentals compared. |

Studio opens all of these except the one-command files below.

## One command each

Small files, each written for one analysis. The first comment of each file says how to run it.

| File | Command |
| --- | --- |
| [`breakeven.yaml`](breakeven.yaml) | `forge calculate` (break-even) |
| [`variance.yaml`](variance.yaml) | `forge calculate` (budget against actual) |
| [`scenarios.yaml`](scenarios.yaml) | scenarios on an acquisition (see the file) |
| [`tornado.yaml`](tornado.yaml) | `forge tornado` |
| [`monte-carlo.yaml`](monte-carlo.yaml) | `forge simulate` |
| [`bootstrap.yaml`](bootstrap.yaml) | `forge bootstrap` |
| [`bayesian.yaml`](bayesian.yaml) | `forge bayesian` |
| [`decision-tree.yaml`](decision-tree.yaml) | `forge decision-tree` |
| [`real-options.yaml`](real-options.yaml) | `forge real-options` |
| [`quota_forecast.yaml`](quota_forecast.yaml) | nested references between groups |

The long walkthrough is [docs/EXAMPLES.md](../docs/EXAMPLES.md).
