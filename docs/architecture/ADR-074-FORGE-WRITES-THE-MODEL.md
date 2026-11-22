# ADR-074: Forge Writes the Model

**Status:** Accepted (2026-11-22), not yet built
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who agreed that changes of more than one line to a model are written by Forge rather than by Forge Studio's page; the analysis is Claude's
**For:** ADR-075 (simulations and games edited on Studio's cards). It extends ADR-056 and changes none of its decisions

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
   - `insert` of a named template (ADR-075, step 6).
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

## Alternatives considered

- **More line work in `text.js`.** Rejected, for the reasons in Context. A rename cannot be done without reading formulas, and the CLI and MCP would not get any of it.
- **Rewrite the file from the parsed model (serialize).** Rejected. Comments and layout would be lost, and Forge's files are read by people: the bagel's comments are half of what it teaches.

## Consequences

- One writer of Forge's format, in the engine, tested once for every caller.
- Renaming becomes safe, in Studio and for an agent.
- forge-wasm grows by the editor's size; it is measured in step 1.
- Changes of one line keep working throughout the move; nothing in Studio breaks while `edit` grows.

## What is built, in order

1. The spike in decision 5, with its findings added to this ADR.
2. `src/edit/` with `set_value`, `set_formula`, `add` and `delete`, held to the bytes `text.js` writes. The forge-wasm `edit` tool, the `forge_edit` MCP tool, and `forge edit`.
3. `rename`.
4. `make_uncertain` and `make_certain` (ADR-075, step 4).
5. The game changes (ADR-075, step 5).
6. `insert` of a template (ADR-075, step 6).
7. Studio's one-line changes moved to `edit`; `text.js` keeps only the colours.
