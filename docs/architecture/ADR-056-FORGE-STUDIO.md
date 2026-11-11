# ADR-056: Forge Studio, a Web Interface in This Repository

**Status:** Accepted, not yet built (roadmap `studio`)
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
