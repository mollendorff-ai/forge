# Les Trois Pentes: a season when the harvest and the price move together

Les Trois Pentes is a fictional vineyard of 24 hectares that sells its grapes to the wineries of its region. The vineyard and every figure are made up. It plans one season at a time, and its owner wants to know: **how likely is a season that loses money, and how bad is a bad one?**

The answer depends on something a plan of averages leaves out. Weather is regional, so when this vineyard has a big crop, so does every vineyard around it, and the wineries pay less for each tonne. A short crop sells dear. The crop and the price move against each other, and that changes the risk: by how much is what this example measures.

| File | What it is | Command |
| --- | --- | --- |
| `costs.yaml` | What a season costs to run, included by the other files as `@costs` | (included) |
| `vineyard.yaml` | The season in a normal year, three blocks of vines, four scenarios and a tornado | `calculate`, `scenarios`, `tornado`, `break-even` |
| `vineyard_risk.yaml` | The same season with the crop, the price and a spring frost uncertain, the crop and the price correlated | `simulate` |
| `vineyard_independent.yaml` | `vineyard_risk.yaml` with the correlation set to 0, and nothing else changed | `simulate` |

## The plan

Three blocks: 10 hectares of chardonnay giving 7.5 tonnes a hectare in a normal year, 8 of pinot noir giving 5.5, and 6 of riesling giving 8.0, 167 tonnes in all. The region's wineries pay 2,000 a tonne on average; pinot noir fetches 35% more and riesling 10% less. Growing costs 7,000 a hectare whatever the crop, picking and trucking 180 a tonne, and the vineyard pays 45,000 a year of overhead and 40,000 on its loan.

| | |
| --- | --- |
| Revenue | 355,200 |
| Growing, picking, overhead and the loan | 283,060 |
| **Profit** | **72,140** (a 20.3% margin) |
| Price a tonne at which the season breaks even (`forge break-even`) | 1,593.81 |

The scenarios show the correlation one season at a time. A glut across the region (a quarter more grapes, sold at 1,550) still makes 53,525, and a dry summer (30% fewer grapes, sold at 2,500) makes 36,758: in both, the price makes up for much of the crop. A May frost that kills 30% of this vineyard's buds loses 25,402, because the frost is this vineyard's alone and the region's price does not move to make up for it. Weighted 60/15/15/10, the season is worth 54,286.

The tornado moves each input on its own: the crop from half a normal year to 130% swings profit from -90,430 to 169,682, more than the price from 1,500 to 2,500 (-16,660 to 160,940) or a frost (to -25,402). Taken one at a time, the crop looks like the risk to watch. The simulation says otherwise.

## The season under uncertainty

`vineyard_risk.yaml` draws three inputs, each with the plan's figure as its `value:`:

- the crop against a normal year: PERT, from 50% to 130%, most likely 100%;
- the region's price a tonne: lognormal, mean 2,000, standard deviation 500 (never below zero);
- the share of the crop killed by a spring frost: `MC.Discrete({0, 0.3}, {0.9, 0.1})`, nothing in nine springs out of ten and 30% in the tenth.

The crop and the price have a correlation of -0.7 (`monte_carlo: correlations:`). Forge applies it as a Gaussian copula: each input keeps its own distribution, and their ranks move together with a correlation of (6/π) asin(-0.35) = -0.683. The frost is drawn on its own.

10,000 seasons each, the same seed:

| | Crop and price correlated (`vineyard_risk.yaml`) | Independent (`vineyard_independent.yaml`) |
| --- | --- | --- |
| Mean profit | 42,889.02 | 51,654.62 |
| Median | 39,680.67 | 39,374.67 |
| Standard deviation | 64,664.26 | 100,027.56 |
| 5th percentile | -58,926.62 | -88,919.76 |
| 95th percentile | 153,625.62 | 234,023.75 |
| **Chance of a loss** | **25.6%** | **33.4%** |
| Mean of the worst 5% of seasons | -79,829.45 | -111,208.60 |

The correlation cuts the spread by a third. The best seasons are not as good, since a big crop never sells at a high price, and the worst are not as bad, since a short one sells dear. A loss comes one season in four instead of one in three, and the worst 5% lose 31,000 less. The correlation also costs about 8,800 of the mean. The two effects come from the same fact: the vineyard never gets a big crop and a high price together. A simulation that leaves the correlation out overstates both what the vineyard can earn and what it can lose.

Both are below the plan's 72,140: the crop's PERT leans below a normal year (its mean is 97%), and a frost costs 30% of the crop one year in ten, which the plan, a normal year, leaves out.

The drivers say what the tornado could not. Under the correlation the price drives profit most (rank correlation +0.70), then the frost (-0.43), and the crop barely at all (-0.12): a bigger crop brings a lower price, so it adds little. The vineyard's risk is the price and the frost, not the size of the harvest.

Latin hypercube sampling (the default) and plain random sampling agree within sampling error: with `--sampling monte_carlo`, the mean is 43,734.17 and the chance of a loss 25.6%.

## Run it

```bash
forge calculate examples/vineyard/vineyard.yaml --dry-run
forge scenarios examples/vineyard/vineyard.yaml
forge tornado examples/vineyard/vineyard.yaml
forge break-even examples/vineyard/vineyard.yaml --output season.profit --vary harvest.price_per_tonne
forge simulate examples/vineyard/vineyard_risk.yaml
forge simulate examples/vineyard/vineyard_independent.yaml
forge simulate examples/vineyard/vineyard_risk.yaml --sampling monte_carlo
```

**Checked outside Forge:** the season was written again in Python from this description, not from the model's formulas. Its plan, scenarios, tornado and break-even agree with Forge's to 1e-6 (`src/cli/commands/prediction/scenarios.rs`, `tornado.rs` and `analysis/goal_seek.rs`, each in a `vineyard_*` module). For the simulation, numpy and scipy checked three things (`src/monte_carlo/correlation.rs`, `vineyard_example`, where the Python is quoted):

- **Forge's draws.** Their rank correlation is -0.690 against the copula's -0.683. Each input keeps its distribution: at five of scipy's quantiles of the PERT and the lognormal, the share of draws below is within the 5% Kolmogorov-Smirnov bound for 10,000 draws. One spring in ten has a frost. The same holds for plain sampling and for the independent file.
- **The results from Forge's own draws.** Python recalculates every season's profit to within 2.3e-10, and the mean, percentiles, chance of a loss and worst 5% above.
- **An independent simulation.** numpy drew 2,000,000 seasons with the same copula. Forge's mean, 5th percentile, chance of a loss and worst 5% are each within four standard errors of numpy's, with and without the correlation.

A correlation that no inputs can have is refused before anything is drawn. Correlating the frost with both a big crop and a high price (0.9 each), when the crop and the price move apart, gives "the correlations of harvest.frost_loss, harvest.price_per_tonne and harvest.yield_factor cannot all hold at once". A test checks that `vineyard_independent.yaml` differs from `vineyard_risk.yaml` only in the correlation.

**What the model leaves out:** one season, so no year carries over to the next and no vintage is stored to sell later. The crop is one factor for all three blocks. The region's price is the only market, with no contract that fixes it in advance, and no crop insurance pays out after a frost.
