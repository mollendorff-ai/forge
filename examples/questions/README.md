# Five questions people ask

Five small models, each the answer to a question people type into a search box, and each one where a single number is the wrong answer: the answer is a range, a chance, what a rival does, or a choice made in turn. They are the models of a trial of five pages on mollendorff.ai (roadmap M-QUESTION-PAGES-TRIAL), each page opening its model in [Forge Studio](https://forge.mollendorff.ai/). All figures are made up, and none of this is financial or legal advice.

| Model | The question | What Forge says |
| --- | --- | --- |
| [`job_offer.yaml`](job_offer.yaml) | Higher salary or stock options: which job offer pays more? | By the recruiter's story (a sale at 300 million) the options are worth 388,750 against 100,000 of salary given up. Simulated, they are worth 116,519 on average and nothing in 85 futures out of 100. |
| [`mortgage_or_tfsa.yaml`](mortgage_or_tfsa.yaml) | Pay down the mortgage or put the money in a TFSA? | 300 a month for ten years: at plan the TFSA ends 2,792.60 ahead. It is ahead in 59 futures out of 100, and more than 10,665 behind in one out of ten. |
| [`food_truck.yaml`](food_truck.yaml) | How many meals a day does a food truck need to break even? | 55.19 a day if the season goes to plan. But a truck that sells 55 makes money in only 28 seasons out of 100; 68.52 a day make nine seasons in ten profitable. |
| [`price_cut.yaml`](price_cut.yaml), [`price_cut_unsure.yaml`](price_cut_unsure.yaml) | A competitor cut prices: should I match? | Match when more than a quarter of the customers would leave (the cut over the margin, 2 / 8). Above a third a price war is certain, and costs each shop 2,000 a month. With 28 in 100 leaving, matching earns 240 a month more than holding, and the street has two equilibria. With the share unknown (the second file), matching pays in 66 futures out of 100. |
| [`settlement.yaml`](settlement.yaml) | Accept the settlement offer or go to court? | Take the 40,000: refusing is worth 33,800, though the trial alone is worth 48,000. Refusing is better only when the claim wins more than 65.4 times in 100. |

```bash
forge calculate examples/questions/job_offer.yaml --plan
forge simulate examples/questions/job_offer.yaml
forge simulate examples/questions/mortgage_or_tfsa.yaml
forge break-even examples/questions/food_truck.yaml --plan --output season.profit --vary meals_per_day --min 20 --max 150
forge simulate examples/questions/food_truck.yaml --seek "P(season.profit > 0) = 0.9" --vary meals_per_day --min 20 --max 150
forge calculate examples/questions/price_cut.yaml --dry-run
forge game examples/questions/price_cut.yaml
forge game examples/questions/price_cut.yaml --scenario fickle_customers
forge simulate examples/questions/price_cut_unsure.yaml
forge decision-tree examples/questions/settlement.yaml -o settlement-result.yaml
```

Each uncertain model has a plan, the figure a spreadsheet would hold, and `forge simulate` gives the range around it. The price cut is two files, the same street with the share of customers who leave as a number and as a range: `forge game`, and the game's card in Forge Studio, do not yet read an uncertain input at its plan figure (roadmap F-GAME-PLAN), so the first file is the one to open and the second the one to simulate. `forge decision-tree` prints the choice at each decision; the value of the branch not taken (33,800) is in the file `-o` writes, and on the tree's card in Studio.

Every model but `price_cut_unsure.yaml` opens in Studio from a link with the model after the `#`, and shows the figures above (checked 2026-11-28 in a Studio built from this commit; the job offer's link is 1,260 characters).

## How the figures are checked

Each model's figures are asserted in the tests of the command that runs it, against a calculation written outside Forge (the Python is in each test module's comment):

- **The job offer and the TFSA** (`src/cli/commands/simulate/iteration.rs`): every one of the 10,000 runs is worked out again from Forge's own draws. The options have any value in exactly 15 runs in 100 (the two larger sales, 0.12 + 0.03), and their exact mean is 116,812.50. The chance that the fund beats the mortgage is that of a normal return above the mortgage's yearly cost of 4.847%: 0.5912.
- **The food truck** (`src/cli/commands/probability_seek.rs`): the season's profit is linear in the meals a day, so each run has a break-even of its own; the figure for nine seasons in ten lies between the 4,500th and the 4,501st lowest of the 5,000.
- **The price cut** (`src/cli/commands/game.rs`): the equilibria of every run are counted again from the share who leave, and the three chances are those of the PERT's beta distribution (SciPy): 0.6630, 0.2844 and 0.3787.
- **The settlement** (`src/cli/commands/prediction/decision_tree.rs`): the tree is rolled back by hand, and the choice changes between a chance of winning of 0.653 and 0.654 (85 / 130 is 0.6538).
