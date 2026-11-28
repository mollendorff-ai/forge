# ADR-082: Forge Says What Is Surprising in a Model, with the Command That Shows It

**Status:** Accepted (2026-11-28); nothing built yet
**Date:** 2026-11-28
**Decided by:** the maintainer (2026-11-28), who accepted the seven decisions as written, with the recommendation of each of the five questions. The design is Claude's, in a session configured for Opus 5.5
**For:** roadmap F-INSIGHTS. It builds on ADR-047 (one core function under every surface), ADR-068 (games), ADR-076 and its addendum (a plan and `--plan`) and ADR-081 (every tool over REST), and changes none of their decisions

---

## Context

A model answers the questions its reader thinks to ask. An agent asked to calculate reports the plan and stops. Each check an analyst would make already exists as a command: `sensitivity`, `break-even`, `simulate`, `scenarios`, `tornado`, `game`. Nothing runs them together and says which results deserve a look.

Three findings were waiting in the examples and in one model written from scratch, and each was found only because someone ran the right command by hand:

- **The bagel's plan is a coin toss.** `examples/bagel/bagel.yaml` makes 786.32 in its first year. Fifteen of its seventeen inputs reach break-even within 10% of their plan figure; only the oven's cost and the discount rate, which the year's profit does not read, do not. The price is closest: 15.97 against 16, 0.21% away (`forge break-even ... --vary assumptions.price_per_dozen`). Next come the opening demand (54.86 against 55, 0.25%), the wage and the staff hours (0.32% each). Nothing in `forge calculate` says so.
- **The mean is not the plan.** `bagel_simulate.yaml` has the same plan with its uncertain inputs at their plan figures. Its simulated mean profit is -15,925.94, against 786.32 at plan (5,000 runs, standard error 538). The real estate example is the same: a mean value created of 11,035.93 against 155,715.31 at plan. The README calls this the flaw of averages; `forge simulate` prints both numbers but never next to each other.
- **The plan sits near its worst point.** A floating sauna written from scratch with v10.0.0-rc.36 (roadmap F-INSIGHTS) makes 39,151 with no new members, about 14,500 at 10 a month and 17,207 at the plan's 12. Only two sensitivity tables, run out of curiosity, showed it.

A fourth is in a game. In `price_war.yaml` both shops cutting is the only equilibrium, at 19,188.94 each. Both holding pays 21,489.38 each, so both would rather leave the equilibrium together. `forge game` gives the equilibrium and, in its JSON, every cell's payoffs, but it never compares them.

The hard part is not running the checks; it is what to leave out. A command that reports noise teaches its reader to ignore it. Each decision below says what is reported and what is not.

## Decision

1. **A command, `forge insights <file or folder>`, and the same in every surface.** It runs checks on a model and lists findings. Each finding has its figures and the command that shows them, a command the reader can run as printed. A folder means each model in it that no other file of the folder includes: in `examples/bagel` that is every file except `costs.yaml` and `costs_risk.yaml`. One core function, `insights_core`, is under the command, its JSON (`-o insights.json`), the `forge_insights` MCP tool, its REST endpoint (which ADR-081 gives every tool), `insights` in forge-wasm and a card on Studio's Analyze view. Nothing is written to the model.

2. **What is checked, and on what.**
   - **Outputs** are what the model names: the tornado's `output`, each `monte_carlo.outputs` variable, and each game. A model that names none has as outputs its scalar results that no other formula reads (the ends of its dependency graph). Rows of a column output (`months.profit`) are not checked one by one.
   - **Inputs** are the scalars with a value and no formula, the model's own and its includes'. An uncertain input counts too, at its plan figure: every check but the simulation runs at plan, as `--plan` does (ADR-076, addendum).
   - **How far each input is swept:** first, the tornado's `low` and `high` when the model gives them. Otherwise, for an uncertain input, the P5 and P95 of its distribution. Otherwise half to one and a half times its plan figure. An input at 0 with neither a tornado entry nor a distribution is not swept, and the answer lists it under `not_checked`. Each sweep is 21 points, evenly spaced, plus the plan.

3. **Five checks, each reported only past a threshold.** Every threshold is a named constant, given in the answer.
   1. **Near break-even.** An output reaches 0 when an input moves by at most 10% of its plan figure. There is one finding per output, not one per input. It names how many inputs reach 0 that way, and lists them nearest first, each with its break-even figure from the same core as `forge break-even`, searched within that 10%. An output that never crosses 0 there (a revenue, a count) gives nothing.
   2. **Near a turning point.** The swept output is not monotonic in an input, its worst point is inside the range (not at an end), and that point is within a quarter of the swept width of the plan. "Worst" follows the output's direction, decided as `forge variance` decides it (a `better:` field, then the group, then the name; FEATURES.md, "Whether a difference is good"). When the direction is unknown, the turning point nearest the plan is reported, named only as the lowest or highest. When the direction is known and the turning point is the best, nothing is reported: a plan set at the best size is what the modeller meant (the microgrid's battery and solar).
   3. **The mean is not the plan.** For each simulated output, the mean is on the other side of 0 from the plan's figure, or the two differ by at least 10% of the plan's. The gap must also be at least three standard errors of the mean, so sampling alone cannot give the finding. The simulation runs with the model's own runs, seed and sampling, so `forge simulate` prints the same mean. The finding names the inputs whose distribution is lopsided about its plan figure (mean above or below it), because those inputs are why the mean moved.
   4. **A driver's sign is borrowed.** A simulated driver is in a `correlations:` entry, and its rank correlation with the output has the opposite sign from the output's own move across the input's sweep with everything else at plan. For example, a dearer cord of wood lowers profit, yet shows as a positive driver, because it rides on cold winters (F-DRIVERS-CORRELATED). A correlated driver whose signs agree gives nothing.
   5. **A game's equilibrium.** A game has more than one equilibrium: which one is played depends on what each side expects. Or a pure equilibrium is beaten for every player by another cell of the table: both would leave it together, as in the price war's 19,188.94 against 21,489.38. Where the model has scenarios, an equilibrium that changes in one of them is a finding too, with `forge game --scenario` as its command.

4. **What is never reported.**
   - A plain slope: an output that moves one way across a sweep, however steep. The tornado already ranks those.
   - A turning point that is the best one, or far from the plan.
   - A break-even more than 10% away.
   - A mean within 10% of the plan, or within three standard errors of it.
   - A game with one equilibrium that no cell beats for everyone.
   - A finding that is a copy of another: a check repeated on a second output that only restates the first (two outputs equal by formula) is merged into one.

   A linear model with no break-even near its plan gives no finding, and the answer says what was checked: so many inputs swept, so many outputs, so many runs.

5. **How findings are ranked.** First by check, in the order above, which is roughly how much each changes a decision. Then, within a check, by the size of the gap relative to the plan figure: the nearest break-even, the largest share of the mean's gap. The command prints the first 10 and says how many more there are. `--all` prints every one. The JSON always has every finding.

6. **The answer.** Per file:

   ```json
   {
     "file": "...",
     "findings": [
       {
         "check": "near_break_even",
         "output": "...",
         "input": "...",
         "plan": 0,
         "figures": {},
         "says": "...",
         "command": "forge break-even ..."
       }
     ],
     "checked": {"inputs": 0, "outputs": 0, "calculations": 0, "runs": 0},
     "not_checked": [{"name": "...", "why": "..."}],
     "thresholds": {}
   }
   ```

   `says` is one sentence in plain words, with the numbers. `command` carries `--plan` when the file is uncertain, and its range or bounds when the check used its own. The same model gives the same answer every time: the sweeps are fixed points and the simulation uses the model's seed.

7. **Cost.** A sweep costs a calculation per point and per input: about 360 for the bagel's 17. The simulation is the model's own. The command stops at 5,000 calculations per file, and anything left unchecked is listed under `not_checked`, so a large model is slow at worst, never wrong. The MCP tool's description is kept short, for the `tools/list` ceiling (T3-MCP-TOKENS).

## Questions for the maintainer

These are the numbers and choices most likely to be argued with. The maintainer accepted the recommendation of each (2026-11-28):

1. The 10% band for a break-even, and whether it is 10% of the input's plan figure (recommended) or of its swept width.
2. A quarter of the swept width for "near" a turning point. For the sauna (members swept 6 to 18 around 12, worst at about 10) it is 2 against 3.
3. Half to one and a half times the plan figure, as the sweep of an input with no tornado entry and no distribution.
4. Whether a best turning point far from the plan ("a higher price earns more") should be reported. Recommended not: a plan is a choice made for reasons the model does not hold (capacity, a market), and the tornado shows the slope.
5. Ten findings printed by default.

## Alternatives

- **Run every analysis and print it all.** That is a report, not a finding. The reader is back to reading every table, and an agent copies the lot into its answer.
- **Rank by a single score across checks** (a weight per check times a size): one number looks objective and is not, since no gap of a mean is comparable with a gap of a break-even. A fixed order by check is plainer and can be argued with.
- **A finding per input for break-even:** the bagel would give fifteen, saying fifteen times what one finding says. One per output, listing the inputs, says it once.
- **Sweeping every input over a fixed ±20%:** simpler, but it ignores the ranges the model already states, and a tornado's range is the modeller's own view of what is plausible.
- **Asking a language model which results are interesting:** not reproducible, not testable against a fixture, and not available in forge-wasm. The checks here are arithmetic, and each is run again by its printed command.

## Consequences

- Nothing in the model format changes. `better:` already gives a direction, and the tornado and `monte_carlo:` already give ranges.
- Each check is a core function that exists, called with bounds: break-even, sensitivity, simulate, game. The finding's command is that function's command, so running it shows the finding, and a test can run each printed command and compare.
- A finding can be wrong only if a check's own core is wrong, or a threshold is a poor choice. The thresholds are in the answer and in this ADR, so a reader can see why a finding was or was not reported.
- Studio's Analyze view gets a card, drawn from its own module (`studio/app.js` is at its 1,500-line limit).
- Decision trees and real options are not checked yet: a decision tree takes no scenario (`forge decision-tree` has no `--scenario`), so "a scenario that changes the best choice" applies to games only until it does. That gap is listed in the roadmap item, not here.

## What is built, in order

1. `insights_core` with checks 1, 3 and 5, the command, its JSON and `--all`. Tests: `examples/bagel` gives the price within 10% of break-even (15.97 against 16), the simulated mean below the plan (-15,925.94 against 786.32) and the price war's equilibrium that both shops would leave (19,188.94 against 21,489.38), each with its command, and each command, run, shows its finding. A linear fixture with no break-even near its plan gives no finding.
2. Checks 2 and 4, with a fixture of the sauna whose members give the turning point near the plan, and a correlated pair whose driver's sign is borrowed.
3. The `forge_insights` MCP tool and its REST endpoint, which come with it (ADR-081); `insights` in forge-wasm and its case; `forge insights --help`.
4. The Analyze card in Studio, and its test against the tool's answer.
5. The guide's page and the architecture's line, filed for the manual and architecture lanes.
