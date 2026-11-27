# ADR-076: Simulations and Games on Forge Studio's Cards

**Status:** Accepted (2026-11-22). Steps 1 to 4 built (2026-11-22, roadmap S-UNCERTAIN). Addendum (2026-11-26): the what-if commands take `--plan` (roadmap F-WHAT-IF-PLAN), decided by the maintainer
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who found that a calculation is edited on its cards while a simulation or a game is changed only in the YAML view, and who left the choice of how an uncertain model behaves on the Model view to Claude (decision 1); the analysis is Claude's
**For:** roadmap S-UNCERTAIN, S-GAMES-EDIT and S-TEMPLATES. It extends ADR-056 and ADR-068 and builds on ADR-075

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

A change writes the formula (`set_formula`, ADR-075), or `make_certain` when "fixed" is chosen. Every number on the card is Forge's. No shape is drawn in this step: drawing one from the parameters would be calculating in the page, and a histogram from the samples would need an engine answer it does not give yet.

### 3. Analyze's Simulate, small fixes

- The output list is in the model's order (months.profit row 1 to 12), not sorted as text.
- A scenarios card for a model whose scenarios have no probabilities says what Forge's message means ("choices to compare, not chances"), not the raw "Validation error".

### 4. Make an input uncertain

Every number input card gets "Make uncertain", which is ADR-075's `make_uncertain`.

- **The default is a triangle** from 80% to 120% of the value, with the value as both the mode and the plan. The person then edits it on its distribution card.
- **If the model has no `monte_carlo:` block,** one is added, with the model's results as its outputs (`iterations: 5000`, `seed: 7`, `sampling: latin_hypercube`, as the examples use).
- **"Make certain"** takes it back.

### 5. A game edited on its card

All of these are written by ADR-075's `edit`, and each is kept only when Forge accepts it.

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

Each inserts a commented block (ADR-075's `insert`), which Forge checks. The person then edits the block on the cards, with decisions 2 and 5. The templates are served by Forge (a `templates` tool: a name, one line, and the YAML). Each is tested to be valid when inserted into every example, so the CLI and MCP have the same blocks.

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
4. Make uncertain and make certain, after ADR-075 step 4 (done, 2026-11-22: "Make uncertain" on a number input's card and "fixed" on a distribution's call forge-wasm's `edit`, which writes the file and keeps the change only when Forge reads the model back as asked; a refusal, such as an input of 0 or one two includes deep, is shown as Forge words it).
5. Games on the card (decision 5): `game` with its inputs, and the folded included groups first; the edits after ADR-075 steps 3 and 5 (the engine's part done, 2026-11-23: `rename`, `add_strategy`, `delete_strategy`, `add_player` and `delete_player` in forge-wasm's `edit`). 2026-11-23: "change this game" on each game card (`studio/gameedit.js`) renames, adds and deletes strategies and players, and gives a tournament's new player its habit; Forge's refusal is shown in the card as it words it. 2026-11-23: a payoff cell's formula under its value while the card is open for changes, changed with `set_formula` (or `set_value` for a number) on `game.payoffs.<strategies>` (a list's player last); `game` answers each cell's `written` (its name for `edit` and its formula; `studio/tests/gameedit.mjs`). 2026-11-23: a tournament's length, noise and players' chances, and a game's dynamics (start, generations, baseline), are changed the same way (`rivalry.tournament.noise`, `price_war.dynamics.generations`), each with a button in the card while it is open for changes; `game` answers them as the report's `settings`; a `noise` or `baseline` the file leaves at its default is added. 2026-11-24: a tournament's player's habit is changed with `set_habit` (a select on its row rather than nine chips, which crowd a card 390 px wide); a game of players alike has a button for each strategy's payoff; `game` answers each game's `inputs`, every scalar without a formula or with a distribution that its numbers read, through formulas, a tournament's round, the network and into included files (`@plan.assumptions.price_per_dozen`; `src/core/array_calculator/inputs.rs`), drawn as input cards under "what moves it" on the game card; an included file's groups are folded under one line, `from bagel.yaml: 4 groups, 1 table`. Step 5 is built.
6. Templates (decision 6), after ADR-075 step 6 (done, 2026-11-23): four templates, `game_of_two`, `game_of_many`, `tournament` and `dynamics`, served by Forge's `templates` tool and added by `insert`; "add a game" on the Model view (`studio/templates.js`) lists them with Forge's line and adds one under a name, Forge's refusal shown in the card. "Make inputs uncertain" is not a template: it is `make_uncertain` on each input's card, which needs no block.

## Addendum (2026-11-26): the what-if commands take `--plan` (roadmap F-WHAT-IF-PLAN)

Decision 1 gave `calculate` a `plan`, and left the what-if commands refusing an uncertain model. A model written from scratch with 10.0.0-rc.36 showed the cost: a plan whose inputs are uncertain needs a second copy with plain numbers for its scenarios, break-even and tornado. `examples/bagel` has two, `bagel.yaml` and `bagel_simulate.yaml`, which drift unless someone keeps them equal.

**Decision.** `forge scenarios`, `compare`, `goal-seek`, `break-even`, `sensitivity`, `tornado` and `audit` take `--plan` (`plan: true` in their MCP tools, over REST and in forge-wasm), which sets each `MC.*` input at its plan figure, as `forge calculate --plan` does, and then runs the command as on any model. Without it they refuse such a model as before. The refusal now says to run the same command with `--plan` (or `plan: true`), and no longer points to `forge calculate --plan`, which is not what the person asked for. One rule sets the plan in every command (`at_plan_or_refused`, `src/cli/commands/plan.rs`), so the figures are the ones `forge simulate` gives for `at_plan`. A scenario that sets an uncertain input still sets it to its own number, as `calculate --plan --scenario` does, and an uncertain input is an input at plan. It can be varied, or be a tornado's bar, and it is not reported as an output of `forge scenarios`. The answers keep their shape. `forge calculate --plan` lists the figures the plan uses.

**Why a flag, and not the plan by default.** The plan as the default would have been one word less to type. It was rejected for three reasons:

- `calculate` already refuses without `plan` (decision 1), and a `forge scenarios` that answered the file `forge calculate` refuses would treat one model two ways.
- A break-even or a tornado that quietly ran at plan would print one confident number for a model whose author wrote that its inputs are ranges: the bagel's plan is +786 over a simulated mean of −15,926. With the flag, the person asked for the plan figure and knows that is what they read.
- No answer changes. Today these commands cannot run such a file at all. A flag can still become the default later; a default is a breaking change to take back.

**What is left for later.** Forge Studio's Analyze view still shows Forge's refusal for an uncertain model's scenarios, goal seek and tornado. It can now pass `plan: true` as its Model view does (roadmap F-STUDIO-WHAT-IF-PLAN). Merging the examples' pairs where they are the same plan is the manual lane's (M-WHAT-IF-PLAN-EXAMPLES).

**Acceptance.** On `examples/bagel/bagel_simulate.yaml` at plan, `forge break-even --vary assumptions.price_per_dozen` gives 15.97, as on `bagel.yaml`. A sensitivity at the planned price of 16 gives the year's 786.32. Its one scenario, `rent_hike`, has no probability, so `forge scenarios` has nothing to weigh with or without `--plan`. `forge compare --scenarios rent_hike --plan` gives what `forge calculate --plan --scenario rent_hike` gives. Each command's `--help` says what `--plan` does. A launch whose price is uncertain (`LAUNCH` in `plan.rs`) checks every command by hand, through the command line, MCP and REST, and forge-wasm's cases check the bagel's.
