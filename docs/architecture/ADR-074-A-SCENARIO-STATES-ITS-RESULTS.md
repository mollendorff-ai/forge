# ADR-074: A Scenario States Its Results, and a Bayesian Network Reads the Model

**Status:** Accepted (2026-11-22), all seven questions as recommended; question 7 as all three parts built together, with `=BAYES()` left out. Steps 1 to 4 built 2026-11-22 (the note under step 4); steps 5 and 6 built 2026-11-22 (the note under step 6)
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who asked of question 5 what export does with scenarios and stored values, and of question 7 what makes more sense for an LLM in an AI pipeline; the analysis is Claude's
**For:** roadmap T2-SCENARIO-CHECKS, and T2-BAYESIAN-LINK (more than its first step: see question 7)

---

## Context

kinship-protocol (github.com/mollendorff-ai/kinship-protocol) is a set of 24 Forge models about cooperation between AIs. Its docs quote figures that Forge works out: thresholds, equilibria, tournament ranks, posteriors. Its CI pins those figures with `scripts/check_models.py`, 647 lines and the repo's only Python. The maintainer wants the maths done by Forge alone. On 2026-11-22 they chose to change Forge to do it, not to commit golden output for CI to diff: a golden file is regenerated, not stated.

The script does what Forge cannot do today:

1. **`forge validate` checks the base case only.** It compares each stored `value:` with its formula (absolute tolerance 0.0001) and takes no `--scenario`. Most of kinship's figures are results of scenarios: a cost of conflict, a level of noise, a horizon, a number of generations.
2. **Most of the figures have no `value:` to compare.** None of kinship's 13 game models stores a `value:`, and most of what they pin are a game's own results, such as `long_run.match.ad.grim`, `population.equilibrium_3.me.grim` and `noisy.rank.tft`. Those names have no entry in the file that could hold one.
3. **`forge bayesian` reads literal numbers only.** A formula in a `prior` or a `cpt` is refused (`invalid type: string, expected f64`), and the command takes no `--scenario`. The script checks `mistake_or_defection.yaml` at nine settings of the prior and the noise by rewriting the network into a temporary file for each, then comparing Forge's posterior with Bayes' rule in exact fractions.

Two things were measured while writing this, with rc.33:

- **`forge calculate --scenario X` without `--dry-run` writes the scenario's results into the base case's `value:`s.** `twice: =w * 2` with `w: 0.6` stored 1.2. After `calculate --scenario short` (`w: 0.4`) it stored 0.8, and `forge validate` then failed with `Current: 0.8, Expected: 1.2`. The file then claims results that it does not give.
- **Two of kinship's models fail `forge validate` today:** `short_horizon.yaml` (18 values) and `analytics/five_ais_bootstrap.yaml` (1). They were typed to two decimals, and the script allows 0.006. This is for kinship to fix, by storing Forge's values. It needs no change in Forge.

### kinship's figures, before any decision

What `check_models.py` pins, counted from the script:

| What | Figures | Where it comes from | Where it would go |
| --- | --- | --- | --- |
| `GAME_RESULTS`: 10 game models, results in the base case and 43 scenarios | 252 | `forge calculate --dry-run [--scenario]` | `expect:` (questions 1 to 3) |
| `GAME_MODELS`: stag hunt, chicken, Axelrod, in the base case and 3 scenarios each | 32 | the same | `expect:` |
| `KEY_FIGURES`: base-case scalars of the old-format models | 6 | the same | `expect:`, or the `value:` already stored |
| `BAYESIAN`: 3 posteriors in the base case, then 3 posteriors and "how many defections it takes" at 9 settings | 39 | `forge bayesian` on rewritten files | question 7 |
| `GOAL_SEEK`: the w at which AlwaysDefect stops beating GrimTrigger, at 3 costs | 3 | `forge goal-seek` on rewritten files | `expect:`: `kinship.defector_edge` is 0 in a scenario at each w* |
| `ANALYTICS` and inspection's tornado: Monte Carlo, tornado, real options, bootstrap | 11 | each command's printed report | left out (see "What is left out") |
| Text the docs must still contain | 39 | the docs | left out: shell (`grep -F`) in kinship's workflow |

Under the decisions below, 332 of the 382 checks move into the models, and `forge validate` checks them. The other 50 are 11 lines of a seeded report and 39 quotes in the docs, matched as plain text by a few lines of shell.

## The questions, and what was decided

### 1. Where a scenario's expected results are written

**Decided: an `expect:` mapping in the scenario, and a top-level `expect:` for the file as written.**

```yaml
_forge_version: "5.6.0"

expect:                                  # the file as written
  kinship.barrier: 0.5
  population.equilibria: 3
  population.equilibrium_3.stable: 0

scenarios:
  costly:
    description: A cost of conflict of 0.5
    scalars: {c: 0.5}
    expect:
      kinship.threshold: 0.375
      kinship.barrier: =2/11
      population.equilibrium_3.me.grim: =2/11
      kinship.spreads: 1
```

- **Why not a `value:` for each scenario on the formula.** It cannot hold kinship's figures: a game's results are not entries of the file (point 2 of the context). It would also put the same figure in two places, the formula's base `value:` and its scenario `value:`, which `forge calculate` would then have to tell apart.
- **`value:` and `expect:` say different things, and both stay.** `value:` is the result last stored by `forge calculate`, a cache, so validate's question is "is the file up to date". `expect:` is written by a person and never by Forge, so the question is "does the model still give what the author checked somewhere else". A figure pinned to nashpy, Gambit or a hand calculation belongs in `expect:`. If Forge rewrote it, an engine bug would become the expected value without anyone seeing it.
- **`expect` is a reserved key of a scenario.** Like `scalars`, it makes the scenario the full form, so the overrides go under `scalars:`. A scenario in the short form could never hold a mapping, so no file that works today reads differently.
- **The top-level `expect:` comes with model format 5.6.0.** Before 5.6.0, a group called `expect` is a group, as `games` was before 5.5.0 (ADR-068). `forge upgrade` to 5.6.0 refuses a file that has such a group and says to rename it. `expect:` in a scenario needs 5.6.0 too, so the feature has one version. At 5.5.0 it is refused with a message that names the version.

_Instead:_ a section of its own keyed by scenario (`expect: {base: …, costly: …}`). It keeps a scenario's inputs away from its results, but `base` is a common scenario name in Forge's own examples, so the file as written would need a name no scenario can have. _Or:_ the expected figures in a separate file beside the model. That is the golden file the maintainer set aside.

### 2. What an expectation may name, and what it may be

**Decided: any scalar the calculation gives, named as `forge calculate` prints it. The value is a number, or a formula over numbers only.**

- **Names:** inputs, formulas, and the results of a game (`<game>.…`) or of a network (question 7), each written in full as `calculate` prints it. A short name, or `@namespace.name` from an included file, is resolved as a scenario's overrides are (`apply_scenario`).
- **A name the calculation does not give fails.** It does not pass silently. `kinship.thresold` fails with "`expect` names 'kinship.thresold', which the model does not give". A game's result exists only once the game is calculated, so the check runs after the calculation. Without this rule, a misspelt name would pass forever.
- **A formula over numbers:** `=2/11`, `=-19/15`, `=113/225`. kinship's figures are fractions worked out by hand, and writing 0.181818 throws away what the author knew. The formula may read no name: an expectation is stated, not worked out by the model. A formula that names a scalar is refused with that reason. Otherwise `=(T - c - R) / (T - P)` would let the model check itself.
- **A table column is left out.** kinship pins none, and a whole column of expected values is a second table.

### 3. The tolerance

**Decided: |Forge − expected| ≤ 10⁻⁶ × max(1, |expected|). That is absolute below 1 and relative above it. One tolerance for every expectation, not written in the model.**

- `validate`'s 0.0001 for `value:` is too loose for a stated figure. In ecology's `careful_long`, Pavlov's share is 0.000125454, and at 0.0001 absolute an expectation of 0 would pass.
- 10⁻⁶ is the script's own `GAME_TOL`. Every figure it pins passes at 10⁻⁶ when written as Forge prints it (six decimals, or all fifteen). A fraction passes when written as a formula. The Axelrod crossover the script pinned to ±0.0005 is exactly 0.108.
- Relative above 1, because a hand-worked 3.2 × 10¹⁰ and Forge's floating-point sum can differ by more than 10⁻⁶ while agreeing to 15 digits.
- `value:` keeps its 0.0001. Changing it would fail files that pass today.

_Left out:_ a tolerance for each figure (`{value: 0.71, within: 0.01}`). No figure of kinship's needs one once it is written with its digits, and a seeded Monte Carlo is not checked through `expect:` (see "What is left out").

### 4. What `forge validate` does, and what it says

**Decided: it calculates the file as written and each scenario that has an `expect:`, and lists every difference with the scenario, the name and both numbers. It fails if there is one.**

```text
✅ All scalar values match their formulas!
❌ 2 of 290 expected results differ

   scenario costly
      kinship.barrier
         Expected: 0.181818 (=2/11)
         Forge:    0.2
   the file as written
      population.equilibria
         Expected: 3
         Forge:    2

💡 The model no longer gives what its expect: says. Change the model, or the expectation if the model is right
```

- When they match: `✅ 290 expected results match (the file as written and 52 scenarios)`. A file with no `expect:` prints what it prints today.
- **A scenario that fails to calculate fails validate, with the scenario named**, as a base case that fails to calculate does today.
- **A scenario without an `expect:` is not calculated.** Calculating every scenario would be a stronger check, but a scenario whose override is an `MC.*` distribution cannot be calculated by `validate` (`reject_distributions`). The cost would also land on files that asked for nothing. That is left out, with this reason.
- **A scenario with an `expect:` and an `MC.*` override** is refused, with the scenario named: run `forge simulate` instead.
- **The base `value:` check runs first and does not change.** The two results are reported separately, because they say different things (question 1).
- **One function.** Today the command (`validate_single_file`) and `validate_core` (MCP, REST, Forge Studio) are two copies of the same loop with the same tolerance. The command prints what `validate_core` returns, so the command, the MCP server, the REST API and Studio cannot disagree.

### 5. Whether `forge calculate --scenario` writes anything

**Decided: `calculate` never writes `expect:`. Under `--scenario` it writes nothing: it prints the results and says why it wrote none.**

- An expectation written by Forge would be the regenerated golden file the maintainer set aside.
- Writing a scenario's results into the base `value:`s is the trap measured above: the file then claims results it does not give, and `validate` fails. Under a scenario, `calculate` will say: `📋 Under a scenario, nothing is written: the stored values are the file's own. To state what this scenario gives, list it under scenarios.short.expect`.
- The MCP tool `forge_calculate` takes a `scenario` and writes unless `dry_run`. It follows the same rule.
- **Export needs no written values.** `forge export` reads no stored `value:`: it calculates the model when it runs. Measured on rc.33: with `value: 99` stored for `=w * 2` and `w: 0.6`, the workbook has the formula and 1.2. It leaves scenarios out, with a warning ("scenarios: not exported (short); Excel has no equivalent"), and writes a game's results as numbers as of the export. So a scenario's results written into `value:` would serve no reader, export included.

_Instead:_ refuse `--scenario` without `--dry-run`. That is clearer, but it breaks a call that works today in order to stop it writing, when writing nothing gives the same safety.

### 6. MCP, REST and Forge Studio

**Decided: the same report everywhere. Studio's edit check does not wait on expectations.**

- **`ValidationResult`** gains `expectations_checked`, `expectations_met` and `expectation_differences`: a list of `{scenario, name, expected, got, diff}`, where `scenario` is `null` for the file as written. `scalars_valid` and `mismatches` keep their meaning. The MCP tool `forge_validate` and `POST /api/v1/validate` return it, so a difference is data, as a value mismatch is today, not a tool error.
- **Studio's edit gate stays as it is.** Studio calls `validate` before it keeps an edit and refuses only when the call fails (`studio/app.js`, `check`). An edit that moves a figure away from its expectation is how a model is explored, so it is kept. The gate asks `validate` not to check expectations. That is the one option of `validate_core`, because ecology's ten scenarios take about 2 seconds natively, and longer in the browser.
- **Studio shows them.** On the Model view, under the scenario that is applied (or the plan), a scalar with an expectation shows it beside its value and is marked when they differ. The numbers come from `validate`, and nothing is worked out in the page (`studio/tests/no-engine.mjs`).

### 7. A Bayesian network that reads the model, and `--scenario`

**Decided: all three parts, (a), (b) and (c), built together as one change. Parts (a) and (b) were the roadmap's first step; (c) is needed if the nine settings are to be checked by Forge alone, and it is what an agent in an AI pipeline needs (below).**

**(a) A `prior` or `cpt` entry is a number or a formula over the model's scalars.** It is worked out after the scenario's overrides, through the calculator, as a game's payoff is (ADR-068). Each row must still add up to 1, checked after the formulas are worked out, with the node and the scenario named. A string was refused before, so no file reads differently, and the network needs no new format version.

**(b) `forge bayesian --scenario <name>`,** and `scenario` on the MCP tool `forge_bayesian` and Studio's `bayesian` call, as `simulate` and `game` have it.

**(c) Named queries, whose results are scalars of the model.**

```yaml
_forge_version: "5.6.0"
p: 0.1        # the share of defectors
noise: 0.05   # the chance a move is flipped by mistake

bayesian_network:
  name: "Mistake or defection?"
  nodes:
    partner: {type: discrete, states: [cooperator, defector], prior: ["=1 - p", "=p"]}
    move_1:
      type: discrete
      states: [cooperate, defect]
      parents: [partner]
      cpt: {cooperator: ["=1 - noise", "=noise"], defector: ["=noise", "=1 - noise"]}
    # move_2, move_3 the same
  queries:
    one:   {node: partner, evidence: {move_1: defect}}
    two:   {node: partner, evidence: {move_1: defect, move_2: defect}}
    three: {node: partner, evidence: {move_1: defect, move_2: defect, move_3: defect}}

by_hand:
  one: {formula: "=p * (1 - noise) / (p * (1 - noise) + (1 - p) * noise)"}
  # two, three: the same with (1 - noise)^k and noise^k
  takes: {formula: "=IF(bayesian_network.one.defector > 0.5 + 1e-9, 1, IF(bayesian_network.two.defector > 0.5 + 1e-9, 2, 3))"}

scenarios:
  rare_clean:
    scalars: {p: 0.01, noise: 0.01}
    expect: {bayesian_network.one.defector: =1/2, by_hand.one: =1/2, by_hand.takes: 2}
```

- `bayesian_network.<query>.<state>` is the posterior of each state of the queried node. `bayesian_network` was never a group (the parser skips it), so the names cannot collide with one. The network is calculated like a game: after the scalars its formulas read, and before the formulas that read its results. A network that reads its own results is a cycle and is refused, as a game's is.
- **Without (c), Forge does not check the network at the nine settings.** `forge bayesian --scenario` would print each posterior, and `by_hand.one` could be pinned by `expect:`. But nothing in Forge would compare the two. kinship would have to match 27 printed posteriors in shell, so the figures would sit in its workflow, not beside the model. With (c), `expect:` pins the network's posterior and the formula of Bayes' rule in the same scenario, and `validate` checks both. That is two different pieces of code (variable elimination, and arithmetic) agreeing on a figure stated once.
- **(c) is most of T2-BAYESIAN-LINK, not only its first step.** A posterior then feeds a result of the model, which is that item's acceptance. `forge simulate` and `forge tornado` calculate the model, so they would move a prior that reads an `MC.*` input or a scalar. That follows from the network being calculated like a game, and step 6 below tests it.
- **Found with kinship's figures in front of it:** two of the nine settings tie exactly at 1/2 after the first defection (prior = noise: 1/100 and 1/100, 1/10 and 1/10). The script decides "more likely than not" in exact fractions. A formula comparing a floating-point posterior with 0.5 could tip either way, so kinship's `takes` must compare with a margin (`> 0.5 + 1e-9`, as above). This needs no change in Forge, but the worked example should show it.

**Why (c), seen from an agent in an AI pipeline.** The maintainer asked what makes more sense for an LLM that builds and checks models through the MCP server:

- **One call gives every result.** An agent calls `forge_calculate` or `forge_validate` and reads one JSON. With named queries the posteriors are in it, beside every other result. Without them, the agent calls `forge_bayesian` once for each set of evidence and joins the answers itself, which is where its own arithmetic, and its mistakes, come back.
- **A mapping is easier to get right than text in a formula.** `evidence: {move_1: defect}` is YAML that `forge_schema` describes and the parser checks before anything runs: "query 'one': move_1 has no state 'defects' (cooperate, defect)". In `=BAYES("partner", "defector", "move_1=defect")` the evidence is text inside a formula inside YAML: the quoting goes wrong, and a typo shows only when the formula is evaluated.
- **It is the pattern Forge already has.** A game is a section whose results are named scalars (`<game>.equilibrium_1…`). An agent that has learnt that reads `bayesian_network.<query>.<state>` with nothing new to learn. A function would be a second pattern, and one Excel does not have either.
- **The results can be followed and pinned.** `forge audit` follows `bayesian_network.one.defector`, `expect:` pins it, and a formula of Bayes' rule sits beside it. That is kinship's check, and the one a pipeline needs.
- **The FP&A use still works.** For the probability of default given the economy, feeding an expected loss, the economy's prior comes from a scalar (part a), and a query with no evidence gives the probability of default, which scenarios and `MC.*` inputs move. What named queries cannot do is change the evidence by scenario. An agent writes one query for each state and picks one with `IF`, which is plain and checkable.
- **(b) costs almost nothing once (a) exists.** The network is worked out under a scenario anyway, and `forge bayesian --scenario` is for evidence that is explored, not yet written as a query.

_Instead of (c):_ `=BAYES(node, state, evidence…)`, the function the roadmap named. It is more flexible, because the evidence is in the formula, but it needs the network inside the formula evaluator, the evidence as text, and a function Excel does not have. **Left out, not kept for later:** named queries say the same in the section that already holds the network, for the reasons above.

## What is left out

- **Expectations of the analyses' reports:** `simulate`, `tornado`, `real-options`, `bootstrap`, `decision-tree`, `sensitivity` and `goal-seek` print reports, not scalars of the model. kinship's 11 such figures (a seeded run, so the same every time) are matched by a few lines of shell. If a second user needs it, `expect:` inside those sections would be a later decision. The 3 goal-seek figures need none: a scenario at each w* pins `kinship.defector_edge` at 0.
- **`goal-seek --scenario`**, for the same reason.
- **`forge export --scenario`**, a workbook of the model calculated under one scenario. It would be the clean way to give Excel a scenario (question 5), and it is small, but no one has asked for it.
- **Checking prose.** Forge does not read the docs that quote its figures. kinship's 39 quotes stay as `grep -F` in its workflow.
- **Calculating every scenario in `validate`** (question 4), **a tolerance for each figure** (question 3), **expectations of table columns** (question 2), and **`calculate` writing `expect:`** (question 5), each with its reason above.
- **Beyond (c):** `=BAYES()` (question 7), soft evidence, evidence that changes by scenario, evidence from a formula, learning a network from data and continuous nodes. T2-BAYESIAN-LINK's own "why" says these are scope creep.

## What is built, in order

1. **`expect:`** in the parser and the 5.6.0 schema, `forge upgrade` to 5.6.0, and the formula over numbers. Tests: a scenario's and the file's expectations are read; a misspelt name, a formula that names a scalar, `expect:` at 5.5.0 and a group called `expect` on upgrade are each refused with their place.
2. **`validate_core` checks them, and the command prints what it returns** (questions 3 and 4). Tests: a broken expectation fails with the scenario, the name and both numbers; the tolerance either side of 1; a scenario with an `MC.*` override and an `expect:` is refused; the MCP tool and the REST endpoint return the same differences.
3. **`calculate --scenario` writes nothing** (question 5), on the command line and in `forge_calculate`. A test that fails before the change: the measured trap above.
4. **Studio** (question 6): the gate without expectations, and the expected figure on the Model view. `studio/tests` checks each number against `validate`.
   - **Note, 2026-11-22: built, with the numbers from `calculate`.** Studio shows the expectations of the run on screen, so it reads them from the `calculate` call it already makes for that run: `forge_calculate` reports each expectation of the file or the scenario applied (`expectations`), checked by the same function as `validate`. An extra `validate` on every edit would calculate every scenario again, about 2 seconds for kinship's ecology. `studio/tests/expect.mjs` checks each card against Forge's report, in the plan, under a scenario and after an edit. The command, MCP, REST and Studio share `src/cli/commands/validate.rs`; `forge watch` keeps its own check of stored values, which has no expectations.
5. **A network reads the model, and `--scenario`** (question 7, a and b): the command, `forge_bayesian`, forge-wasm. Steps 5 and 6 are one change, released together.
6. **Named queries** (question 7, c), checked when the model is read (each node, state and piece of evidence named, with the query's place), calculated in the order of a game, with the cycle refused. A worked example in `examples/`, where a posterior feeds a result of the model (the probability of default given the economy, feeding an expected loss), checked against Bayes' rule by hand, and a test that `simulate` moves a prior that reads an `MC.*` input.
   - **Note, 2026-11-22: steps 5 and 6 built, together.** The network is read into the model in every format (`src/bayesian/written.rs`), and is one cell of the calculation (`Cell::Network`, `src/core/array_calculator/network.rs`), ordered as a game is. Four things were settled while building it:
     - **Every probability is checked once worked out:** each from 0 to 1 as well as each row adding up to 1. A literal file with a negative entry was accepted before; it is refused now, with the node and the state.
     - **Evidence that cannot happen is refused.** Under a scenario where it has probability 0 (no defectors, and a partner who never errs, defects), inference gave every state a posterior of 0. The query now fails, named, instead of giving a number that is no probability.
     - **`forge bayesian` reads the file as a model.** A probability may read a scalar, so the command calculates the model first. A failed formula the network does not read does not fail it, as with `forge game`. The file now needs `_forge_version`, as every other command's does. Before, a bare `bayesian_network:` file was read on its own.
     - **One network a file, in any of its documents.** An included file's network gives its results by the include's alias (`@alias.bayesian_network.<query>.<state>`), as its other scalars do.
   - Studio's Bayesian card does not pass the scenario yet (roadmap T2-BAYESIAN-LINK, `studio`): none of Studio's example sets has a network with scenarios to test it on.
7. **Docs, each step:** docs/FEATURES.md, docs/cli, the CHANGELOG, the roadmap, docs/STABILITY.md for the new names.

Then a release, and then kinship's v0.8.0: its figures move into `expect:`, the nine settings become scenarios of one file, the two files typed to two decimals store Forge's values, and `scripts/check_models.py` and its `pip install` are deleted.

## Consequences

- A model says what each scenario should give, beside the scenario, and `forge validate` checks it. CI of any Forge model needs no script of its own to pin its figures.
- New names, fixed once released (docs/STABILITY.md): `expect` (a top-level section from 5.6.0, and a key of a scenario), `queries`, `node` and `evidence` in a network, and `bayesian_network.<query>.<state>`.
- One change of behaviour: `forge calculate --scenario` without `--dry-run` no longer writes. Before, it wrote values the file does not give.
- No new dependency.
