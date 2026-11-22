# ADR-075: Forge Writes the Model

**Status:** Accepted (2026-11-22); built: the spike (decision 5), step 2 (`set_value`, `set_formula`, `add`, `delete`; forge-wasm's `edit`, `forge_edit`, `forge edit`), step 3 (`rename` of a scalar, a group, a table and a column; a game, a player and a strategy are left for step 5), and step 4 (`make_uncertain`, `make_certain`). The maintainer allowed `hashbrown` twice for yaml-edit (2026-11-22)
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who agreed that changes of more than one line to a model are written by Forge rather than by Forge Studio's page; the analysis is Claude's
**For:** ADR-076 (simulations and games edited on Studio's cards). It extends ADR-056 and changes none of its decisions

---

## Context

Forge Studio (ADR-056) never reads a model's meaning: Forge does, in forge-wasm. A change made on a card is written by `studio/text.js`, which finds one line of the YAML and rewrites it, comments and layout kept. It handles three kinds of change: an input's value (`setInputText`), a formula (`setFormulaText`), and adding or deleting one entry (`addEntryText`, `deleteEntryText`, roadmap S9). Each is kept only when Forge reads back exactly that change. That works because each is one line, or one entry, at a place found by its key.

What the next features need is not one line:

- **Renaming** a game's strategy or player, or any name, changes every formula that reads it (`price_war.dominated.lune.hold`, `street.others.pay`). Finding them means reading formulas. The page is not allowed to do that (`studio/tests/no-engine.mjs`), and it should not: Forge's own formula reader is the only one that is right.
- **Adding a strategy** to a game adds a row and a column to `payoffs:`, which is a mapping of mappings, often written flow-style (`hold: {hold: =market.R, cut: =market.S}`).
- **Making an input uncertain** turns `price: 16` into `price: {value: 16, formula: "=MC.Triangular(…)"}`, and adds a `monte_carlo:` block if there is none.
- **Adding from a template** (a game, a tournament, `dynamics:`) inserts a block at the right place in a file whose layout the page does not know.

Each of these could be written in `text.js` as more line searches. Each would then be a second, partial reader of Forge's format in JavaScript: the thing ADR-056 set out not to have. And the command line and the MCP server, through which an agent edits a model, would not get them.

## Decision

1. **Forge writes a model's text.** A new operation, `edit`, takes a model's files and a list of changes, and answers the new text of each file it changed, or a refusal with Forge's message. It runs in the engine (`src/edit/`), so forge-wasm, the MCP server and the command line share it, as every other tool does (ADR-047).
2. **What it can change.** Each change is named and typed, not free text:
   - `set_value` (an input), `set_formula` (a scalar, a column or a payoff cell), `add` and `delete` (an input, a formula, a column): what `text.js` does today.
   - `rename` (a scalar, a group, a column, a table, a game, a player, a strategy): the name and every reference to it, in every file of the model, found by Forge's formula reader.
   - `make_uncertain` (an input becomes a distribution with its value kept as `value:`) and `make_certain` (back again).
   - `add_strategy`, `delete_strategy`, `add_player`, `delete_player` on a game.
   - `insert` of a named template (ADR-076, step 6).
3. **Kept only when Forge agrees.** After writing, `edit` reads the model again. It answers the new text only if the model is valid and the only change is the one asked for: the same rule S9 applies in the page today, moved into the engine. Otherwise the files are unchanged and the message says why.
4. **Comments and layout are kept.** Every byte outside the change stays as it was. The tests that hold `text.js` to this today (`studio/tests/text.mjs`, `formulas.mjs`, `elements.mjs`) are run again against `edit` for the same changes, byte for byte.
5. **A library first** (CLAUDE.md, ADR-070). The text is edited through a lossless YAML syntax tree, not by searching lines. The first candidate is `yaml-edit` (0.3.6, Apache-2.0, built on `rowan`: <https://github.com/jelmer/yaml-edit>). Before it is adopted, a spike checks:
   - Every example file goes in and comes out byte for byte.
   - It finds Forge's keys written in each way YAML allows: plain, quoted, flow and block.
   - Its cost in the WebAssembly module, gzipped. Over 100 KB is said in the change and the CHANGELOG, as ADR-070 asks.
   - Its versions do not duplicate a crate already in the tree. If one does, that is the maintainer's decision, per CLAUDE.md.

   If it fails, the next candidates are `yaml-edit-rp` (a fork with fixes) and then `yaml-splice`. Only if all of them fail is the editor written in Forge, and the spike's findings go here.
6. **Studio moves to it change by change.** New changes (rename, games, uncertainty, templates) use `edit` from the start. `text.js` keeps the one-line changes until `edit` gives the same bytes for them; then they move too, and `text.js` keeps only the editor's colours. The step that empties `text.js` deletes its tests along with it, with no test left uncounted.
7. **The command line and MCP.** The command line gets `forge edit <file> --set name=value --rename old=new …`. MCP gets a `forge_edit` tool with the same changes as JSON. Both refuse as `edit` refuses. Neither writes values back (ADR-051 is unchanged: only `forge calculate` writes `value:`).

## What the spike found (decision 5, 2026-11-22)

`yaml-edit` 0.3.6 with `default-features = false` (its only default feature, `binary`, brings `base64` 0.22 beside Forge's 0.23, and Forge reads no binary scalars), in a throwaway crate outside the workspace, against the files of this repository at commit `ce8a90ef`.

| Check | Result |
| --- | --- |
| Every YAML file in the repository parsed and written back | 174 of 174 byte for byte, comments, quoting, flow and block kept. One reports a parse error, `test-data/test_malformed.yaml`, which is malformed on purpose, and still comes back byte for byte |
| Every formula and every number in every example, each changed in place on a fresh parse | 771 formulas (to `=(…)`, in the style the file uses) and 1,326 numbers in 62 examples: each time only the bytes of that value changed, and the file parses again |
| `studio/tests/text.mjs`'s cases of `set_value` and `set_formula` | 18 of 18 byte for byte: block, flow (first and last key), plain, single- and double-quoted, a quote or `#` that needs quoting, a backslash, `1e-5`, `-3`, `0x10`, `value:` under the name |
| Its cases of `add` and `delete` | 21 of 21 byte for byte: an input, one with a unit, a formula, a column after a block sequence, a group indented by 4, the top level after a scalar and after a formula; each added and deleted again to the same file; a deletion keeps the comment above it and takes the lines under it |
| Keys in each way YAML allows | plain, `"double"`, `'single'` and inside a flow mapping are found; a key added to or removed from a flow mapping keeps it on its line (`{value: 0.03, unit: ratio, better: lower}`) |
| Rename | `rename_key` changes the key and nothing else, in block and flow mappings |
| Unicode names | `hypothèses.coût_unitaire` and `假设.单价` found, set, renamed, and a key added next to them |
| Windows line endings | kept on every line, including new ones |
| Size in forge_wasm_bg.wasm (`wasm-bindgen --target web`, `gzip -9`, as CI measures it) | 2,011,484 bytes gzipped without it, 2,167,171 with it linked and used (parse, find, set, remove, rename, insert): **+155,687 bytes gzipped** (+429,359 raw). Under ADR-070's ceiling of 3,000,000, and over 100 KB, so the change that adds it says so, and so does the CHANGELOG |
| Rust 1.97 | yaml-edit needs 1.70, rowan 1.77 |
| A crate in two versions | **one: `hashbrown` 0.14.5**, from `rowan` 0.16.1 (yaml-edit's syntax tree), beside 0.17.1 from `indexmap`. Both are in the shipped WebAssembly module. No newer `rowan` drops it (0.18.0-alpha.2 still takes 0.14), so there is no fix on our side |

How the editor works with it: yaml-edit finds the node, and Forge writes its text. `Scalar::set_value` replaces a value's text exactly as given, so the rule of `text.js`'s `encodeFormula` (keep the file's quoting, quote only when YAML needs it) stays Forge's, and a new entry is written from a snippet in Forge's own style, not from the library's serializer. yaml-edit recovers from errors; Forge refuses any text it reports an error in, and reads the result back with its own reader (decision 3), so the library never decides what a model means.

The other candidates were looked at, not run: `yaml-edit-rp` (0.3.1) is an older fork of the same code on the same `rowan`, so it brings the same `hashbrown`; `yaml-splice` (0.4.1) edits only a mapping's top-level keys, and Forge's changes are inside groups, tables, games and flow mappings.

**The maintainer's decision.** Clippy does not report this duplicate today, only because `hashbrown` 0.15.5 (from `petgraph`, a dev-dependency) is in the workspace too, and the lint names a crate only when every version of it is a normal dependency. The rule of CLAUDE.md is about the duplicate, not the lint: adopting yaml-edit means allowing `hashbrown` in `clippy.toml` with this reason and counting it in `validation/repo/src/no_skipping.rs`, in the change that adds it. That is the exception CLAUDE.md gives the maintainer (a core, stable library with no fix on our side), and only the maintainer can take it. If not taken, the editor is written in Forge (decision 5's last resort), on the findings above. The build (step 2) waits on this decision. **Decided 2026-11-22: the maintainer allowed it**, and `hashbrown` is in `clippy.toml`'s list, counted in `validation/repo/src/no_skipping.rs`, in the change that adds yaml-edit.

**What step 2 found.** Two things the tree does not do as `text.js` did, and Forge does instead. A snippet set with `Mapping::set` is indented by yaml-edit's own step of 2, under a group indented by 4 too, where the file's style is 4 more: so the tree gives the place (the end of the group's last entry, whose span runs to the end of its last line, comment included) and Forge writes the lines, as `text.js` did. And a bare `value:` gets its space. Each change is checked by reading the model back as Forge Studio did (S9): the model's JSON, and each included file's, before and after, equal once the changed entry is set aside, and the entry as asked. An uncertain model is checked at plan, as `forge calculate --plan` calculates it. A distribution is read only as a whole formula (`=MC.Normal(0, 1)`, not `=(MC.Normal(0, 1))`), and `edit` refuses the second as Forge does.

**What step 3 found.** A rename is the name's key, and every name that reads it, in every file of the model. What a name in a formula reads is found by the calculator's own reader (`src/core/array_calculator/references.rs`), at the place the formula is written: Forge's tokenizer gives each name and where it is (`tokenizer::names`, the identifiers that are not a function's), and the calculator's name resolution says what it reads (a column's bare name reads its own table's columns first, a scalar's short name its group first, then the top level, then the only scalar of that last name; `@alias.name` reads the included file). A name outside formulas (a scenario's override and expectation, the file's `expect:`, a simulation's outputs and correlated inputs, a tornado's output and inputs) is read as those commands read it: a full name, or the last part of only one scalar. Each is written again as it was, full or short; a short name that would read something else once renamed (`units` in a table that now has a column `units`; `price` once there are two) is written in full, and one that cannot be is refused. The file's bytes are changed only at those names: yaml-edit's tree gives each key and scalar with its span, and the new text is spliced in, in the quotes the file uses; a formula folded over several lines is refused. A text under `notes:`, `description:` and the other keys of text is never read as a formula, even when it starts with `=`. The check is the model read back: every formula, override and expectation of every file, each name in it replaced by what it reads, equals the same before with the rename applied, and so do the analyses' names, read from the file; the model calculates. Two things this refuses rather than guesses: a key read under two names (`g: *base`, a YAML alias, renames both), and a name of `LET` that would read the renamed scalar. Every scalar, group, table and column of the 62 example models, and each scalar of a file one includes, is renamed and back to its files byte for byte (`every_name_of_every_example_is_renamed_and_back`). A game's payoffs and a Bayesian network's priors and tables (ADR-074) are formulas over the model's scalars: they are found by the same reader, and the read-back compares them too (a first version compared only scalars, tables, scenarios and games, so it refused renaming `outlook.p_bad` in `expected_loss.yaml`, whose network reads it; the every-example test found it once `main` had that example). A game, a player and a strategy are named in a game's keys, its results and its tournaments, and are renamed with the game changes (step 5).

**What step 4 found.** An input is made uncertain at the one entry, in the style the file uses for it: `price: 16   # dollars` becomes `price:` with `value: 16   # dollars` and `formula:` under it (the comment stays on the value, the new lines are indented as `add` indents them), a block gets a `formula:` line at its end, and a flow mapping gets `formula:` before its closing brace. `make_certain` takes the `formula:` out and, when what is left is `value:` alone, writes it short again, so every input of every example goes through both and comes back byte for byte (479 inputs, in the main files and the files they include). With no distribution given, the default is the triangle of ADR-076 (80% to 120% of the value, the value its mode); a value of 0 needs one. The `monte_carlo:` block is added to the main file's end when it has none, with its results (the scalars with a formula that no formula, table column or game reads) as `outputs`; an included file's input is changed there and the block goes in the main file. `make_certain` of a distribution with no `value:` writes the plan figure `forge simulate` uses (`plan_of`: the mode, the mean, the midpoint of a uniform). The read-back is the model at plan: only that scalar's formula and value, and the `analyses` list when the block is added, may differ. One limit of the engine shows: an input of a file that another included file includes too (the bagel's `costs.yaml` under `price_war.yaml`, which includes `bagel.yaml`, which includes it) cannot be planned, because the middle file calculates its own includes without the plan figures. Such a model does not simulate either, so `make_uncertain` refuses it with Forge's message (20 of the 499 inputs of the examples); making the engine plan through nested includes is a separate change.

## Alternatives considered

- **More line work in `text.js`.** Rejected, for the reasons in Context. A rename cannot be done without reading formulas, and the CLI and MCP would not get any of it.
- **Rewrite the file from the parsed model (serialize).** Rejected. Comments and layout would be lost, and Forge's files are read by people: the bagel's comments are half of what it teaches.

## Consequences

- One writer of Forge's format, in the engine, tested once for every caller.
- Renaming becomes safe, in Studio and for an agent.
- forge-wasm grows by the editor's size; it is measured in step 1.
- Changes of one line keep working throughout the move; nothing in Studio breaks while `edit` grows.

## What is built, in order

1. The spike in decision 5, with its findings added to this ADR (done, 2026-11-22).
2. `src/edit/` with `set_value`, `set_formula`, `add` and `delete`, held to the bytes `text.js` writes. The forge-wasm `edit` tool, the `forge_edit` MCP tool, and `forge edit` (done, 2026-11-22).
3. `rename` of a scalar, a group, a table and a column (done, 2026-11-22); of a game, a player and a strategy with step 5.
4. `make_uncertain` and `make_certain` (done, 2026-11-22). Studio's "fixed" button on the distribution card is ADR-076 step 4.
5. The game changes (ADR-076, step 5), and `rename` of a game, a player and a strategy.
6. `insert` of a template (ADR-076, step 6).
7. Studio's one-line changes moved to `edit`; `text.js` keeps only the colours.
