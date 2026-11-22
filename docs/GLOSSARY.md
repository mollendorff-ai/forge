# Glossary

The words Forge, its documents and Forge Studio use, each with where to read more. [FEATURES.md](FEATURES.md) is the reference; [EXAMPLES.md](EXAMPLES.md#your-first-model) starts with a first model.

## A model

- **Model**: one YAML file, and the files it includes. Forge reads it, checks it and calculates it. Nothing outside it is needed. [The model file](FEATURES.md#the-model-file).
- **Format version** (`_forge_version`): the version of the model format the file is written in, from 1.0.0 to 5.5.0. It is not the version of the Forge program. Each format adds to the one before.
- **Scalar**: a single named value, as in `price_per_dozen: 16`.
  - **Input**: a scalar with a value and no formula. A change to an input is what makes everything else move.
  - **Result**: a scalar calculated by a formula (`profit: {formula: "=revenue - costs"}`).
- **Group**: a mapping of scalars under one name (`assumptions:`). A scalar in a group is read as `assumptions.price_per_dozen`.
- **Table**: a mapping of columns of the same length (`months:`), each a list of values or a formula worked out row by row. A table is told from a group by its lists.
- **Formula**: Excel's language, after `=`: operators, the functions in [FUNCTIONS.md](FUNCTIONS.md) and the model's names. [Formula evaluation](FEATURES.md#formula-evaluation).
- **Period, the row above**: a table whose rows follow one another (months, years). A formula reads the row above with `PREV`, which is how a balance carries over. [Periods and the row above](FEATURES.md#periods-and-the-row-above).
- **Unit** (`unit:`): what a number counts (CAD, %, dozens). Forge warns when units that cannot be added are added.
- **Format** (`format:`): how a number is shown (`$#,##0.00`), never its value. [How a number is shown](FEATURES.md#how-a-number-is-shown-format).
- **Include** (`_includes`): another file read into the model under a name, its values read as `@costs.ingredients.flour_per_kg`. [Values from other files](FEATURES.md#values-from-other-files-_includes).

## What if

- **Scenario**: a named set of changes to inputs (`flour_shock`). It is applied without changing the file. With probabilities that add up to 1, Forge also weighs the scenarios into expected values. [Scenario analysis](FEATURES.md#scenario-analysis).
- **Plan**: the model as written, before any scenario or simulation.
- **Goal seek**: the input value that makes a result reach a target. **Break-even**: the same with a target of 0. **Sensitivity**: a result over a range of one or two inputs. [Goal seek, break-even and sensitivity](FEATURES.md#goal-seek-break-even-and-sensitivity).
- **Tornado**: each input moved from low to high, one at a time, ranked by how far it moves a result. [Tornado diagrams](FEATURES.md#tornado-diagrams).
- **Budget against actual** (variance): a plan compared with what happened, name by name, each difference marked favourable or not. [Budget vs actual](FEATURES.md#budget-vs-actual-forge-variance).

## Uncertainty

- **Uncertain input, distribution**: an input written as a range of values with their chances, by a `MC.*` function such as `MC.PERT(40, 55, 70)` (lowest, likeliest, highest). [Distributions](FEATURES.md#distributions).
- **Simulation** (Monte Carlo): the model calculated many times, each time with every uncertain input drawn from its distribution. The result is a spread of answers, not one. [Monte Carlo simulation](FEATURES.md#monte-carlo-simulation-forge-simulate).
- **Run, or iteration**: one of those calculations.
- **Seed**: the number that fixes the draws, so the same seed gives the same answers.
- **Latin hypercube**: a way of drawing that covers each distribution evenly with fewer runs.
- **At plan**: a result with each uncertain input at its plan figure (`value:`), shown beside the simulated mean.
- **Percentile** (P5, P50, P95): the value below which that share of the runs fall. P50 is the median.
- **Threshold**: a condition on a result (`> 0`). Forge gives the share of runs that meet it.
- **Expected shortfall**: the average of the worst runs, the worst 5% for example.
- **Driver**: an uncertain input ranked by how closely a result follows it (rank correlation).
- **Bootstrap**: a confidence interval from data alone, by drawing from the data again and again. [Bootstrap resampling](FEATURES.md#bootstrap-resampling).

## Decisions

- **Decision tree**: choices and chances in sequence. Forge works backwards to the best choice and its expected value. [Decision trees](FEATURES.md#decision-trees).
- **Real option**: the right, not the duty, to wait, expand, contract or abandon, and what it is worth. [Real options](FEATURES.md#real-options).
- **Bayesian network**: variables whose chances depend on one another. Given what is seen (evidence), Forge gives the chances of the rest. [Bayesian networks](FEATURES.md#bayesian-networks).

## Games

- **Game**: players who each choose a strategy at the same time, each paid by what all of them chose. [Games](FEATURES.md#games-format-550-adr-068).
- **Player, strategy, payoff**: who chooses, what each can choose, and what each gets for each combination. A payoff is a number or a formula, so a game can be built from a model's numbers.
- **Symmetric game**: every player has the same strategies and is paid the same in the same place. Only the first player's payoffs are written.
- **Count form**: a symmetric game of many players written as one payoff per strategy, over how many of the *others* play each (`street.others.pay`).
- **Equilibrium**: a choice for every player that no player would change alone.
  - **Pure**: each player picks one strategy.
  - **Mixed**: players pick by chance, with given probabilities.
- **Dominated strategy**: one that pays a player less than another of its strategies whatever the others do.
- **Tournament**: a game played round after round between habits such as tit for tat or always defect, each against every other. [Tournaments](FEATURES.md#tournaments).
- **Continuation**: in a tournament, the chance that there is another round.
- **Collectively stable**: a habit that no other does better against than it does against itself.
- **Dynamics** (`dynamics:`): how the shares of each strategy in a population change, generation by generation, as strategies that pay more spread (the replicator equation). [Strategies that spread](FEATURES.md#strategies-that-spread-dynamics).

## Where Forge runs

- **forge**: the command line (`forge calculate`, `forge simulate`, `forge game`…). [cli/README.md](cli/README.md).
- **MCP server**: the same tools for an AI agent (`forge_calculate`…). [MCP.md](MCP.md).
- **Forge Studio**: the same engine in a web page, phone first, at <https://forge.mollendorff.ai/>. Nothing typed there leaves the device. [studio/README.md](../studio/README.md).
- **forge-wasm**: Forge built for the browser (WebAssembly), which Forge Studio runs.
