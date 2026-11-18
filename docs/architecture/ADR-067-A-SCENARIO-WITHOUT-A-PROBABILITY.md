# ADR-067: A Scenario Without a Probability Is Compared, Not Weighted

**Status:** Proposed; not yet built
**Date:** 2026-11-18
**Decided by:** the maintainer, when this is merged; the analysis and the recommendation are Claude's
**Closes:** roadmap T1-UNWEIGHTED-SCENARIOS

---

## Context

A model's `scenarios:` holds two kinds of thing. Some are **chances**: a slow integration, the revenue synergies never coming, rates going up. Only one of them will happen, and each has a probability. Others are **choices**: paying for a company all in cash, all in shares, or 60/40. Nobody weighs a choice by its chance. The board picks one, and wants to see them side by side first.

`examples/acquisition/deal.yaml` needs both. Its four weighted scenarios (`base` 45%, `slow_integration` 25%, `no_revenue_synergies` 20%, `rates_up` 10%) give `forge scenarios` an expected value created of 139.49. The payment mix is a choice, so it has no probability, and putting it in the file stops `forge scenarios` from running at all. The example put the payment mix in a goal seek and the sensitivity table instead (T1-MA-EXAMPLE, 2026-11-18).

What the code does today, read from it and run on a scratch copy of the example with `all_cash` (`financing.cash_share: 1`) and `all_shares` (`financing.cash_share: 0`) added:

| Where | A scenario with no `probability:` |
| --- | --- |
| The parser (`parser::model::parse_scenarios`) | Accepted, in the flat form (`earnout: {offer.premium: 0.20}`) or the full form with `scalars:` and no `probability:`. It goes into `ParsedModel.scenarios` like any other. The parser never reads `probability`. |
| The JSON schema, 5.0.0 to 5.4.0 | Accepted: `ScenarioDefinition` needs `probability` or `scalars`, and the flat form is `ScenarioOverrides`. |
| `forge compare`, `forge calculate --scenario`, `forge simulate --scenario` | Used. `forge compare --help` says so: "compare does not weigh the scenarios, so it needs no probability". `deal_risk.yaml`'s `earnout` is one. |
| `forge scenarios` (`scenarios_core`) | The whole file is refused: "scenario 'all_cash' has no probability. forge scenarios weighs the scenarios by it, and the probabilities of all scenarios must sum to 1 (forge compare does not need it)". |
| Studio | The chips list every scenario and show a percentage where `forge scenarios` gave one. When `scenarios` fails, Studio swallows the error, and the Analyze tab says "This model has no scenarios section". It says that today for `deal_risk.yaml`, which has one. |
| The Excel export | No scenario is exported. The export warns and names them all: "scenarios: not exported (all_cash, all_shares, base, …)". |
| The MCP tools | `forge_compare` takes the names and works. `forge_scenarios` fails as the command does. |

`forge compare deal.yaml --scenarios base,all_cash,all_shares` already gives the board its table. These are Forge's own figures on the scratch copy, not yet checked outside Forge:

| | base (60/40) | all_cash | all_shares |
| --- | --- | --- | --- |
| New Valdane shares | 15.6 | 0.0 | 39.0 |
| Net debt over EBITDA at closing | 2.04x | 2.60x | 1.20x |
| Value created | 225 | 225 | 225 |
| Value created per Valdane share | 1.04 | 1.12 | 0.94 |
| Earnings per share, year 3 | -1.1% | +1.3% | -4.2% |

The value created is the same in all three, because the model counts everything Valdane pays, however it is paid. The mix changes who carries the value: leverage against dilution. That is a choice, and there is nothing to weigh.

So unweighted scenarios already exist in the format. One command refuses them. The roadmap item gives two ways out.

## The two options

### Option 1: weigh only the scenarios that have a probability, and name the rest

`scenarios:` stays one list. `forge scenarios` weighs the scenarios that have a `probability:`, leaves out those that have none, and says which it left out.

| Where | What changes |
| --- | --- |
| The parser | Nothing. `parse_model` already reads both forms. `scenarios_core` (`src/cli/commands/prediction/scenarios.rs`) skips a scenario without `probability` instead of refusing the file, and keeps its name. `ScenarioEngine` is unchanged: it is given only the weighted ones. |
| `forge scenarios` | One more line, after the weighted scenarios: `Not weighted (no probability): all_cash, all_shares`, with the `forge compare` command that shows them. The errors are in the decision below. |
| The results (`ScenarioResults`, `-o file`) | A new field, `unweighted`: the names left out, in the file's order, and an empty list when there are none, so a program can rely on it. |
| The JSON schema | Nothing. No new format version: no model that works today reads differently. |
| Studio | The Analyze table already shows the scenarios `forge scenarios` returns. A note below it names the unweighted ones. The chips already show no percentage where there is none. The false "This model has no scenarios section" goes away: `scenarios` no longer fails on `deal_risk.yaml` or on a mixed file. It still fails on a file where no scenario has a probability, and that case needs its own message ("No scenario here has a probability"), not the false one. `forge-wasm` passes the new field through without a change. |
| The Excel export | Nothing. The warning already names every scenario. |
| The MCP tools | `forge_scenarios` returns `unweighted` in its JSON (via `scenarios_json`). Its description and `docs/MCP.md` say that it weighs the scenarios with a probability and names the rest. `forge_compare` is unchanged. |
| Help and docs | `forge scenarios --help`, `forge compare --help`, `docs/cli/README.md`, `docs/FEATURES.md` ("must sum to 1.0"), ADR-018's note. |

The risk is a scenario left out by mistake: the author forgot its probability, and the expected value silently covers less than they meant. The sum check already catches that. Take a positive probability out of a set that sums to 1, and the rest no longer sum to 1, so the file is refused, and the message names the scenarios that had none. A scenario can be left out without an error only if the probability it was meant to have was 0, and then it adds nothing to the expected value. The output names it either way. Nothing is weighted wrong without a word (docs/STABILITY.md, point 1).

### Option 2: a second, unweighted list for compare

A new top-level section, say `alternatives:` (the name is for this comparison only), holds the scenarios that are choices. `scenarios:` keeps only the weighted ones, and every one there needs a probability.

| Where | What changes |
| --- | --- |
| The parser | `parse_model` reads a new reserved top-level key. A model that already has a group or table of that name would read differently, the problem `scenarios` already has (the table-or-scenarios guess at `parser/model.rs:54-83`). The key also goes into the lists in `parser/schema.rs` (the dates in scenarios, the reserved keys) and `cli/commands/upgrade.rs`. Its names must not clash with a name in `scenarios:`, because `compare`, `calculate --scenario` and `simulate --scenario` take one name. That is a new error. The two lists either go into one `ParsedModel.scenarios` or into a new field that each of those three reads. |
| `forge scenarios` | Still refuses a scenario without a probability in `scenarios:`, now pointing to the new section. |
| The JSON schema | A new format, 5.5.0 (ADR-014), because a new reserved key changes how an existing file reads. Changes: `forge-v5.5.0.schema.json`; `forge.schema.json` dispatching to it; the copies in `editors/vscode/schemas`; `forge schema v5.5`; `schema/README.md`. |
| The flat form already in `scenarios:` | Two choices, both bad. If the flat form stays, a choice can be written two ways, and one of them still breaks `forge scenarios` (`deal_risk.yaml`'s `earnout`, the examples in `forge compare --help`). If it is forbidden, models that work today break, which docs/STABILITY.md (point 2) allows only in a new major version. |
| Studio | The `model` call in `forge-wasm` lists both sections, so the chips show both. The false "no scenarios section" stays for every flat-form scenario until it is fixed on its own. |
| The Excel export | The warning has to name the new section's scenarios too. Otherwise they are dropped without a word. |
| The MCP tools | `forge_compare` is unchanged if the lists are merged. The schema and examples tools, and `docs/MCP.md`, describe the new section. |

What option 2 gives that option 1 does not: the author says which scenarios are choices, so a missing probability in `scenarios:` is always an error. Option 1's sum check already catches every case where a missing probability would change the answer. That leaves option 2 with a format version, a second name space and two ways to write the same thing, to catch a case that cannot change the answer.

## Decision (proposed)

**Option 1.** The format already has unweighted scenarios, and every other command already uses them. Only `forge scenarios` refuses them, and that is where the fix goes.

1. **Which are weighted.** A scenario with a `probability:` is weighted. A scenario without one is not: the flat form, or the full form with only `description:` and `scalars:`. A probability of 0 is weighted. It says the scenario cannot happen, which is not the same as having no probability.
2. **The checks, over the weighted ones only.** Each probability is between 0 and 1, and together they sum to 1 within 0.001, as now. The error names the scenarios that were left out, so a forgotten probability shows itself: `The probabilities of the 4 scenarios that have one sum to 0.90, not 1 (all_cash and all_shares have none and are not weighted)`.
3. **No scenario has a probability.** An error that says there is nothing to weigh and shows the `forge compare` command that uses them.
4. **`--scenario` names an unweighted one.** An error that points to `forge calculate --scenario` and `forge compare`. It does not run the scenario on its own with no weight.
5. **What is reported.** The expected value, the range and P(>0) cover the weighted scenarios only. The terminal output, the `-o` file, the MCP tool and Studio all name the ones left out (`unweighted`).
6. **The example.** `examples/acquisition/deal.yaml` gets `all_cash` and `all_shares` next to its four weighted scenarios. Its README shows `forge compare --scenarios base,all_cash,all_shares`, and says that `forge scenarios` still weighs four, gives 139.49, and names the two it left out. The Python check is extended to the two new columns.

## What is left out

- **The expected value under each choice.** This decision does not say what the four chances are worth if Valdane pays all in cash. That would be 4 weighted scenarios under each of 3 choices: an unweighted scenario applied first, then the weighted ones on top of it, as `forge simulate --scenario` already does for a simulation. A later roadmap item if a model needs it; not decided here.
- **Scenarios in the Excel export.** ADR-018's "one sheet per scenario" was never built, and the export still warns. Unchanged.
- **Scenarios that are not mutually exclusive.** The weighted ones are still read as one set of outcomes, only one of which happens (ADR-018).

## Consequences

- A model can keep its choices and its chances in one `scenarios:` section, and `forge scenarios` runs on it.
- No model that runs today gives a different answer. A file `forge scenarios` refused now runs and reports which scenarios it left out. The expected value of a file it ran before is unchanged.
- `ScenarioResults` gains a field, `unweighted`, in the JSON and YAML that `forge scenarios -o`, `forge_scenarios` and `forge-wasm` return. It is a new field; none is removed or renamed.
- Studio stops saying a model with scenarios has none.
- `forge compare --help` changes from "forge scenarios does [need a probability]: the probabilities must sum to 1" to: forge scenarios weighs the scenarios that have one, and names the rest.
