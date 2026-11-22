# ADR-075: Simulations and Games on Forge Studio's Cards

**Status:** Accepted (2026-11-22), not yet built
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who found that a calculation is edited on its cards while a simulation or a game is changed only in the YAML view, and who left the choice of how an uncertain model behaves on the Model view to Claude (decision 1); the analysis is Claude's
**For:** roadmap S-UNCERTAIN, S-GAMES-EDIT and S-TEMPLATES. It extends ADR-056 and ADR-068 and builds on ADR-074

---

## Context

As of 10.0.0-rc.33, at 390 px:

- **A calculation** is edited on its cards: an input with a slider and a field, a formula with Edit, Add and Delete on a group.
- **A simulation is shown as broken.** `examples/bagel/bagel_simulate.yaml` opens on the Model view with:
  - a red Problem pill;
  - a red banner sending you to Analyze;
  - "Not calculated" in large type on every result;
  - its two uncertain inputs listed as results, as their `=MC.PERT(40, 55, 70)` text.

  It runs only from Analyze, Simulate.
- **A game card shows its answers, not its question.** The payoff table, the equilibria, the tournament's ranking and the path of `dynamics:` are drawn and move with the inputs (ADR-068, ADR-073). What cannot be done there:
  - see that a cell is the formula `=market.R`;
  - change a cell, or rename a strategy;
  - add a player or a strategy;
  - find the inputs the game reads. In `price_war.yaml` they are about 8,000 px further down, under every group of the two included files.

Measured in the page (forge-wasm, Chromium, the container's CPU), on the bagel's simulation:

| Call | Time |
| --- | --- |
| `simulate`, 200 runs | 72 ms |
| `simulate`, 1,000 runs | 144 ms |
| `simulate`, 5,000 runs (the model's own setting) | 617 ms |
| `calculate` (`bagel.yaml`) | 10 ms |
| `game` (`price_war.yaml`) | 14 ms |

`simulate` already answers, for each output, the value at plan (`at_plan`), and for each uncertain input, its mean and its plan.

## Decision

### 1. An uncertain model on the Model view: the plan, with its range under it

An uncertain model is not an error.

- **The cards show the model at plan.** Every uncertain input takes its plan figure, the one `forge simulate` uses for `at_plan` (`value:` when it has one). Every result is calculated at once, as for any model. Sliders answer in milliseconds, and every card has a number.
- **The results the model asks about get their range.** Those listed in `monte_carlo.outputs` get a band under the plan figure:
  - the mean;
  - P5 to P95;
  - a mark at the plan;
  - one line per threshold ("> 0: 34.8%").

  The band comes from `simulate` run with the model's own settings: runs, seed and sampling. So the numbers are the ones `forge simulate` prints and Analyze shows, and the screen test can compare them with the native command, as it does every other number.
- **When it runs.** The band is worked out again a short pause after a change (400 ms, against 120 ms for a calculation).
  - While it runs, the previous band stays, dimmed, marked "running".
  - An answer that a newer change has overtaken is dropped (the page's generation counter).
  - The seed is fixed, so the band never moves by itself, only when something is changed.
- **Not "Problem".** The status pill and the banner say "uncertain: N inputs are ranges" and link to Analyze for other outputs and seeds. Problem stays for real errors.

Why this and not the other choices:

- **The median or the mean as the big number.** It would be honest about uncertainty. But it moves only after a run, and it differs from the plan figure the slider just set. The plan as the big number keeps the cards fast and the same as every other model. The band then shows how far the truth can be from it: in the bagel, a plan of +786 over a mean of −15,926.
- **A smaller, faster sample on the cards** (200 or 1,000 runs). It would make the cards disagree with Analyze and with `forge simulate` on the same model, which is worse than half a second's wait for a band.
- **Simulate only on a button.** That is what is there now: the Model view of an uncertain model is then useless.

Engine work: `calculate` (and forge-wasm's `calculate`) takes `plan: true` to calculate an uncertain model with each uncertain input at its plan figure, the rule `simulate` already applies for `at_plan`. Without it, `calculate` refuses as it does today.

### 2. An uncertain input is a distribution card

An input whose formula is a distribution is drawn as an input, not as a result.

- **A row of kinds**: fixed, and Forge's `MC.*` functions.
- **One field per parameter**, named as in Forge's function list (`=MC.PERT(min, mode, max)` gives min, mode and max). The page keeps no list of distributions or parameters of its own.
- **Its plan** (`value:`) and its mean from the last run.

A change writes the formula (`set_formula`, ADR-074), or `make_certain` when "fixed" is chosen. Every number on the card is Forge's. No shape is drawn in this step: drawing one from the parameters would be calculating in the page, and a histogram from the samples would need an engine answer it does not give yet.

### 3. Analyze's Simulate, small fixes

- The output list is in the model's order (months.profit row 1 to 12), not sorted as text.
- A scenarios card for a model whose scenarios have no probabilities says what Forge's message means ("choices to compare, not chances"), not the raw "Validation error".

### 4. Make an input uncertain

Every number input card gets "Make uncertain", which is ADR-074's `make_uncertain`.

- **The default is a triangle** from 80% to 120% of the value, with the value as both the mode and the plan. The person then edits it on its distribution card.
- **If the model has no `monte_carlo:` block,** one is added, with the model's results as its outputs (`iterations: 5000`, `seed: 7`, `sampling: latin_hypercube`, as the examples use).
- **"Make certain"** takes it back.

### 5. A game edited on its card

All of these are written by ADR-074's `edit`, and each is kept only when Forge accepts it.

- **Each payoff cell shows its formula under its value** when the card is in Edit mode. A tap opens the formula editor Studio already has, with its suggestions (`set_formula` on the cell).
- **Players and strategies are renamed** on the card, with every reference to them (`rename`). A strategy or a player is added or deleted from the card. A new cell starts at 0.
- **"What moves it":** the inputs the game's payoffs read, traced through formulas as `forge audit` traces a scalar, drawn as input cards inside the game card. Engine work: `game` answers each game's inputs. A game's results cannot be audited today ("not a scalar or a table column of the model").
- **A tournament:**
  - its players as chips, one per habit, from the nine habits the schema allows;
  - its continuation or rounds;
  - its noise.
- **`dynamics:`:** the starting shares and the number of generations. Each is an input when it is a number, or a formula like any other.
- **An included file's groups** are folded under one line, such as "from bagel.yaml: 4 groups, 1 table", when the model being shown is not that file. In `price_war.yaml` the game and its inputs then fit on two screens.

### 6. Add from a template, not a wizard

Add on the Model view offers, besides an input, a formula and a column:

- make inputs uncertain;
- a game of two players;
- a game of many players alike (the count form);
- play this game again and again (a tournament);
- watch it spread (`dynamics:`).

Each inserts a commented block (ADR-074's `insert`), which Forge checks. The person then edits the block on the cards, with decisions 2 and 5. The templates are served by Forge (a `templates` tool: a name, one line, and the YAML). Each is tested to be valid when inserted into every example, so the CLI and MCP have the same blocks.

Why not a wizard: on a phone, a wizard of several steps holds a second copy of what the schema already knows, is used once, and leaves the person in the YAML view afterwards. A template plus the cards is the same thing, and the cards stay the place to edit it.

## Consequences

- An uncertain model reads like a model, with its range shown where it is asked for.
- A game can be built on a phone without the YAML view.
- Two engine answers grow: `calculate` with `plan`, and `game` with its inputs. There is one new tool, `templates`.
- The page stays free of calculation. `studio/tests/no-engine.mjs` is unchanged, and the new numbers are held against Forge's report as before.

## What is built, in order

1. Analyze's fixes (decision 3).
2. `calculate` with `plan: true`. The Model view of an uncertain model: the plan, the bands, "uncertain" in place of "Problem". Held against `forge simulate` in `studio/tests/screen.mjs`'s way.
3. The distribution card (decision 2).
4. Make uncertain and make certain, after ADR-074 step 4.
5. Games on the card (decision 5): `game` with its inputs, and the folded included groups first; the edits after ADR-074 steps 3 and 5.
6. Templates (decision 6), after ADR-074 step 6.
