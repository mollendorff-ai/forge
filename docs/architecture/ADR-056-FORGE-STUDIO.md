# ADR-056: Forge Studio, a Web Interface in This Repository

**Status:** Accepted; Studio is in `studio/` and calculates with forge-wasm (S3); published by `pages.yml` once Pages is switched on (S4); addenda on adding and deleting elements (S9) and on the grid view (S11), 2026-11-11
**Date:** 2026-11-10
**Decided by:** the maintainer (a public demo page, phone first, with a split view on wide screens, built in this repository and published on GitHub Pages); the design is Claude's, in a session configured for Opus 5.5

---

## Context

Forge is used through a terminal or, by an AI agent, through MCP. A person who wants to look at a model, change an input and see what moves has no way in without installing it.

A Forge model is a YAML file, and YAML is vertical: groups of named values, one per line. A spreadsheet is wide. A phone held upright is narrow. A model shown as a column of cards (inputs with a slider, results with their formula under them, tables as a sparkline that opens into its values) fits a phone better than a grid does.

A mockup was built outside this repository between 2026-11-07 and 2026-11-10 and tried on an iPhone. Five views: **Model** (the cards, scenario chips that apply their overrides, a side-by-side table of groups with the same keys as `forge report` shows them), **YAML** (the files, editable, with formulas highlighted by the rules of `editors/vscode`), **Analyze** (goal seek and break-even, scenarios with their expected value, simulation, tornado, budget against actual), **MCP** (the call an agent would send for each of the 20 tools, with this model) and **Files** (examples, import from the device, export as YAML or CSV). Changing an input on a card rewrote one value in the YAML text and kept its comments. It calculated with its own engine, which ADR-055 replaces.

## Decision

1. **Forge Studio lives in this repository**, in `studio/`: static files (HTML, CSS, JavaScript modules) with no framework and no build step of its own. It calculates with the WebAssembly build of ADR-055, so YAML is parsed by Forge, not by a JavaScript library.

2. **The YAML is the model.** Every change made on a card is a change to the text, which keeps comments, quotes and key order, as `forge calculate` keeps them (ADR-051). There is no state that the YAML does not hold, apart from what the screen shows (open cards, the chosen scenario).

3. **Phone first, split on wide screens.** Under about 900 pixels wide: one column of cards and five tabs at the bottom, as in the mockup. Wider: the YAML editor on the left and the cards on the right, side by side, with a divider that can be dragged and a control to hide either side; Analyze, MCP and Files open in the right pane. Typing in the editor updates the cards beside it. The page never scrolls sideways, down to 320 pixels.

4. **Highlighting has one source.** The colours of a formula come from the TextMate rules in `editors/vscode/syntaxes`. Studio uses the same patterns in the same order, and a test fails when the two disagree.

5. **Everything runs on the device.** No server, no account, no upload: a model opened from the device stays in the browser, kept in its local storage between visits. The page says so.

6. **Published on GitHub Pages** by a workflow on each release: the Studio files and the WebAssembly build of that release, at the repository's Pages address. The page shows the Forge version it runs. Pull requests build it and keep the result as a workflow artifact, without publishing.

7. **The examples are the repository's.** Studio opens `examples/` as they are in the release (bagel, mortgage, periods, household, options, starter), so the demo and the tests use the same models and numbers.

8. **What it is not:** a spreadsheet with a grid, a hosted service, or a native app. A native app can follow if the page is used.

## Alternatives considered

- **A separate repository.** Its own releases would fall behind the engine and the format. Rejected.
- **A framework** (React, Svelte, Vue). A build step and a larger page for an interface of five views. Rejected for now; worth revisiting if the interface grows.
- **A code editor library.** Monaco (VS Code's editor) is poor on phones. CodeMirror 6 is good on phones and could replace the mockup's editor (a text box over a highlighted copy) if that proves fragile on iOS. Not needed to start.
- **Tabs only, on every screen.** Simpler, but on a laptop the editor and the results would never be visible together, and seeing cause and effect side by side is the reason for a wide layout. Rejected.
- **A server-side page** calling `forge serve`. See ADR-055: a server to run, and the user's models leave the device. Rejected.

## Consequences

- **A public page under the Möllendorff AI name**, linked from the README once it is published. It is described as what it is: a demo of a release candidate, with the README's known limitations.
- **JavaScript in a Rust repository.** CI needs Node for Studio's tests (the highlighting rules, the YAML write-back, a headless browser that opens each example and reads its numbers).
- **GitHub Pages must be switched on once** in the repository's settings (Settings, Pages, Source: GitHub Actions) by an administrator.
- **Studio follows the engine.** A format change (as ADR-051, ADR-053, ADR-054) needs Studio's cards to show it; the headless test catches a model that no longer opens.
- **Editing formulas on a card, and Excel export and import in the browser,** are later steps (roadmap S7, S8).

## Addendum, 2026-11-11: adding and deleting elements (roadmap S9)

**Decided by:** the maintainer asked for a visual way to add elements to a model, since typing YAML on a phone is the hardest part of Studio; the design is Claude's, in a session configured for Opus 5.5.

Decision 2 holds: the YAML is the model, and a card writes text. S7 let a card change a formula; S9 lets a group's card add an element and any card delete one.

1. **What can be added.** To a group of scalars: an input (a number, with an optional unit) or a formula (with an optional unit). To a table: a column calculated by a formula. At the top level, a scalar goes after the last top-level scalar. Data columns (a list of values per row), groups, tables, scenarios and renames are not added from cards: a data column needs a value per row, and a rename has to rewrite every formula that uses the name. They stay on the YAML tab.

2. **How it is written.** The new lines go at the end of the group, in the indentation its keys already use, the way format 5.1.0 writes each kind (ADR-051): `name: 16`, or `value:` and `unit:` under `name:`; a formula as `formula: "=..."` (and `unit:`) under `name:`; a column as `name: "=..."`. A formula is always in double quotes. Names are restricted to letters, digits and `_`, starting with a letter or `_`, so the key is always plain YAML; Forge decides whether the name is valid in the model. A group written as a flow mapping, or a nested group, is not changed: Studio says so and points to the YAML tab.

3. **How it is deleted.** The element's key line and every line under it, including a block sequence written at the key's own indentation. A comment on the lines above it stays, since it may be about what follows. The last element of a group is not deleted from a card: the group would be left empty.

4. **What is kept.** As for a formula (S7): the new text is kept only when the model still validates (for a model with distributions, a ten-run simulation evaluates every formula), and the model Forge reads back is exactly the one before, with that one element added (with the value, formula and unit typed) or gone. Anything else (a name already in the model, a formula Forge refuses, deleting an input other formulas use) shows Forge's message, and the file is as it was. Adding and then deleting an element gives back the file byte for byte.

5. **What it costs.** Text functions in `studio/text.js` that find lines and insert or remove them; no YAML library, and the page still calculates nothing (`no-engine.mjs`). `studio/tests/text.mjs` covers the ways a file is written; `studio/tests/elements.mjs` adds an input and a formula to every group, and a column to every table, of every example, deletes each again, and checks the file byte for byte.

## Addendum, 2026-11-11: the grid view (roadmap S11)

**Decided by:** the maintainer asked for a spreadsheet visualizer and, after the options were set out, agreed on a read-only grid tab; the design is Claude's, in a session configured for Opus 5.5.

Decision 8 says what Studio is not: "a spreadsheet with a grid". That stands for what the word meant there: a grid as the way to edit a model, with a formula per cell. The **Grid** tab is a different thing, a way to read.

1. **What it shows.** Each table as rows and columns, with the column's one formula under its name in the heading (Forge has one formula per column, where Excel has one per cell), and the scalars as a list with their formulas. The numbers are the ones Forge calculated, in the scenario that is applied, formatted by the same rules as the cards. Included files stay on the Model tab.
2. **What it does not do.** It writes nothing: there is no input in it, and editing stays on the cards (S7, S9). A tap on a cell or a heading shows its value and formula and offers "Show its card".
3. **What it is not.** A preview of the exported workbook. iOS opens the `.xlsx` from Files in Quick Look or Numbers, with the real sheets and formulas, and showing it in the page would need either a spreadsheet library there (the page reads nothing itself, decision 2) or a new engine tool that mostly rebuilds a viewer the device has.
4. **On a phone** the first column (the row) and the heading stay in view while the table scrolls, and the page never scrolls sideways (the table scrolls inside its card).
5. **Tested.** `studio/tests/grid.mjs` compares every cell of every table of every example, and every scalar, with what Forge returns, with and without a scenario; it also checks the formula a tap shows and that the card is brought into view.
