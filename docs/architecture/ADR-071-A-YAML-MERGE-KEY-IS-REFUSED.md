# ADR-071: A YAML Merge Key Is Refused, With One Message

**Status:** Accepted (option 1); built
**Date:** 2026-11-21
**Decided by:** the maintainer, who chose option 1 (2026-11-21); the analysis and the recommendation are Claude's
**Closes:** roadmap T1-YAML-MERGE-KEYS

---

## Context

YAML 1.1 has a merge key: `<<: *base` in a mapping means "the keys of `base`, then the ones written here". YAML 1.2 (2009) dropped it. Anchors and aliases stayed, and Forge reads them: `players: &roster [a, b]` then `players: *roster` reuses a whole block.

`yaml_serde` 0.10.7, the YAML library Forge uses, reads `<<` as an ordinary key. It can merge on request (`Value::apply_merge`) but does not do so by itself. No part of Forge asked it to, so each part met a key named `<<` in its own way. Measured on caf9097 (2026-11-21), each through the command that reads it:

| Where | What happened |
| --- | --- |
| A tournament's players (`forge game`) | `'games.wide.tournament.players.<<' is not any of the forms allowed here (a mapping)`, and the same line for the anchored player |
| A payoff row (`forge game`) | `'games.dilemma.payoffs.defect' has "<<", which is not a strategy of you (cooperate, defect)` |
| A scenario (`forge scenarios`) | `'scenarios.more.<<' must be a number, not a mapping` |
| An included file's group (`forge calculate`) | `'lower.<<.price': there is no key 'price' here` |
| A `decision_tree:` branch (`forge decision-tree`, which reads its section itself) | `decision_tree config error: unknown field '<<'` |
| A scalar written as `y: {<<: *x}` | **Accepted without a word.** `y` became a group with a member named `<<`, so no scalar `y` existed: a formula that read `y` failed with "Unknown variable: y", and with none the value was lost silently |

The model text became YAML in seventeen places: the parser (the model, an included file, each document of a multi-document file), `forge upgrade`, the MCP server's inline models, and the commands that read their own section (`simulate`, `scenarios`, `tornado`, `bayesian`, `bootstrap`, `real-options`, `decision-tree`). Each called `yaml_serde::from_str` itself.

## The two choices

### Option 1: refuse a merge key, with one message, wherever it is

Every model text becomes YAML through one function, `parser::read_yaml`, which refuses a mapping key `<<` anywhere in the document with its place: `'scenarios.more.<<' is a YAML merge key (<<), which Forge does not read: YAML 1.2 dropped merge keys. Write the keys out (an alias such as `players: *roster` still reuses a whole block)`.

### Option 2: resolve a merge key before anything reads the model

The same one function, calling `Value::apply_merge` after parsing, so `<<` means what it meant in YAML 1.1.

## Decision

**Option 1.** The reasons, in order of weight:

1. **Refusing can be undone; supporting cannot.** No file that works today stops working if merge keys are supported later. Once models use `<<`, it can never be taken away. While the choice is open, the one that keeps it open is safer.
2. **A model should mean the same in every reader.** Merge keys are a YAML 1.1 feature. A reader in YAML 1.2 mode (an editor's schema check, `yq`, a script) reads `<<` as a key, and would see a different model from Forge's.
3. **What merge keys are for, Forge mostly has.** An alias reuses a whole block. "This, with a few values changed" is what a scenario is.
4. **`apply_merge` adds the merged keys after the ones written out.** In a payoff table, key order is the order of the strategies, so a merge would have changed the order of a game's strategies.

## Consequences

- One message, with the key's place, in every place a model is read: the command line, the MCP server, the REST API and the browser (`forge-wasm`) all read a model through the parser. The scalar that was accepted silently is refused.
- `parser::read_yaml` fails with a `yaml_serde::Error`, so each caller keeps its own wording around it ("YAML parse error: …", "Failed to parse document 2: …").
- A key written `"<<"` in quotes is refused too: `yaml_serde` does not keep the quotes, and no name in a model is `<<`.
- The writer (`src/writer/in_place.rs`) still parses with `yaml_serde` directly: it reads text that has already been read through `read_yaml`, to edit it.
- **To support merge keys later**, `read_yaml` would call `apply_merge` instead of refusing, with a test that a payoff table keeps its strategy order (point 4). That would be a new decision recorded here, and every file that works today would still work.
