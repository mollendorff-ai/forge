# Forge Studio

Forge in your browser, phone first (ADR-056): **<https://forge.mollendorff.ai/>**

The first visit opens with a short welcome card: what the page is, and for an example of several models, the questions they answer (a tap opens one), each marked with its kind (a calculation, a simulation, a game, a decision tree…); a strip at the top of the Model view opens any of them later. Open one of the examples, a YAML file from your device, or an Excel workbook, and the model shows as cards: inputs to change, results with their formulas, tables, scenarios. Change a number and Forge calculates the model again. It is the same engine as the command line, running in the page as WebAssembly.

## What it does

- **Model:** inputs as sliders and fields, results with their formulas, tables, scenarios to apply, and a model's games (ADR-068): a payoff table with its equilibria marked, a tournament's ranking and matches, all worked out again when an input or a scenario changes, and under each game the results a formula can read, by name (`stag_hunt.even_mix.me.stag`), which the formula suggestions offer too. A formula can be changed on its card, with suggestions for names and functions; it is kept only when Forge accepts it, and otherwise Forge's message says why. A group's Add button adds an input, a formula or a table column, and a card's Delete removes it; each change is kept only when Forge reads back exactly that change.
- **An uncertain model** (ADR-076): every card shows the plan, each uncertain input at its plan figure (Forge's `calculate` with `plan: true`); each result the simulation asks about has its range under it, P5 to P95 with the mean, a mark at the plan and the chance of each threshold, from `forge simulate` with the model's own runs and seed, worked out again a moment after each change; an input that is a distribution is a card with Forge's kinds and a field for each parameter, named as Forge's function list names them. The pill says how many inputs are ranges.
- **The other analyses:** a file that is a decision tree, real options, a bootstrap or a Bayesian network opens on a card drawn from its tool (ADR-077): the best choice and the plan, the value of each option, the estimate and its intervals, and each node's chances, with what you have seen set by a tap.
- **Grid:** the model as a spreadsheet reads it, read-only. A tap shows a cell's formula and leads to its card.
- **YAML:** the model's text, with Forge's formula colours, and suggestions for the keys of the model format and for names and functions inside a formula. On a phone the edit is applied with a button; above 900 px the editor sits beside the cards and a number typed in one changes the other.
- **Analyze:** goal seek, break-even, scenarios, a tornado, a simulation and budget against actual. **Tools:** the MCP server's tools, run on the open model.
- **Help:** a "?" on each heading opens a few sentences, the terms the section uses, an example to open and the manual on GitHub at the release that is running; while a formula or a key is typed, the line under the suggestions gives Forge's syntax and description for it (ADR-077).
- **Files:** the examples, opening and saving. A link can open one model of an example set: `?example=bagel` opens the bagel's plan, and `?example=bagel&model=price_war.yaml` its price war, as Files' Open button would (a file the set does not have opens the set's own model). Save opens the share sheet on an iPhone or iPad (Save to Files, iCloud Drive) for the model's YAML or its Excel workbook.

## Your model stays on your device

There is no server doing the work: the page runs `forge-wasm` (ADR-055) in a background worker, and nothing you type is sent anywhere. There are no accounts, no cookies and no analytics. Your files are kept in the browser's storage on your device until you save them. GitHub Pages serves the page, so GitHub sees that it was downloaded, as with any website.

It also works offline. Once Forge is running, the page keeps its own files on the device (`sw.js`), so Studio opens and calculates without a connection, and on an iPhone it can be added to the Home Screen (Share, Add to Home Screen) to open like an app. A new release replaces the files the next time Studio is opened after being closed, never half of each, so close it fully and open it again to get one.

## How it is built

Every number is Forge's. The page reads no YAML itself: a change on a card is written into that line of the YAML, comments and all, and Forge calculates the files again.

| File | What it holds |
| --- | --- |
| `index.html`, `studio.css` | The page |
| `app.js` | State, the six views, events; the parts below are its modules |
| `forge.js`, `worker.js` | The only way to a number: `call(tool, request)` to forge-wasm, in a worker |
| `text.js` | Text work on YAML: changing one input's or one formula's line, colouring the editor with the VS Code grammar's rules (`formula-rules.json`, written by `build.mjs` from `editors/vscode/syntaxes/`) |
| `help.js`, `help.json` | The "?" on each heading and its sheet (the text is `help.json`; the manual is linked on GitHub at the running release's tag), and the line under the suggestions with Forge's own syntax and description of a function or a key (ADR-077) |
| `tools.js` | The Tools view: the MCP server's tools, the request each is shown with, and an answer shown whole |
| `games.js` | The game cards: payoff tables, equilibria, tournaments and dynamics, drawn from `game`'s answer (ADR-068, ADR-073) |
| `analyses.js` | A decision tree, real options, a bootstrap and a Bayesian network drawn on the Model view from their tools' answers, each number with the place in the answer it comes from (ADR-077) |
| `uncertain.js` | An uncertain model (ADR-076): the ranges under the outputs from `simulate`, each number with the place in the answer it comes from, when they are worked out again, the distribution card from Forge's function list, and Analyze's scenarios reason |
| `sets.js` | A set's models: its questions on the welcome card, the strip, and each model's kind from Forge's `model` answer (ADR-077) |
| `suggest.js` | What to offer while a formula or a key is typed: names from the model and the games' results from Forge's `calculate`, functions from Forge's own list, keys from Forge's schema (the page keeps no list of its own) |
| `format.js` | How numbers look on screen: one rule (`isPercent`) for when a number is a percentage, by the model's unit first |
| `build.mjs` | Puts the page, forge-wasm and the example sets in one folder |
| `sw.js`, `manifest.webmanifest` | Offline and Add to Home Screen: the page keeps its own files on the device |
| `screenshots.mjs` | Makes the screenshots in the main README from the page itself |

## Building it

```bash
cargo build -p forge-wasm --profile wasm --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir forge-wasm/pkg-web target/wasm32-unknown-unknown/wasm/forge_wasm.wasm
node studio/build.mjs forge-wasm/pkg-web studio-site
```

`studio-site` is a static site. Serve it over HTTP (a worker and `fetch` do not run from `file://`), for example `python3 -m http.server -d studio-site`.

## Where it is published

At <https://forge.mollendorff.ai/>, on GitHub Pages (the old `mollendorff-ai.github.io/forge/` address redirects there), after each release: the Release workflow starts `.github/workflows/pages.yml` with its tag. The page's status pill shows the Forge version it runs. The workflow builds at the release's tag, deploys, and then opens the published address in a headless browser (`tests/live.mjs`): the version must be the release's and the bagel example's `year.profit` must be 786.32.

Pages is switched on once, by an administrator: Settings, Pages, Source: GitHub Actions. To publish by hand, run the workflow from the Actions tab with a tag or a branch; it defaults to `main`. Releases up to v10.0.0-rc.13 predate Studio and cannot be published. The page uses relative paths only, so a custom domain needs no change to the files.

## How it is checked

All of these but `live.mjs` (run by `pages.yml` after a release) run in CI, in the `forge-wasm` job, which keeps the built site as the `forge-studio` artifact. Most of them compare what the page shows with what Forge returns for every example, so they check the page against the engine, not against a number someone typed.

| Test | What it checks |
| --- | --- |
| `no-engine.mjs` | No file of the page loads a YAML library or implements what Forge calculates |
| `screen.mjs` | Every scalar on screen equals `forge calculate` run natively for that model, and the same with a scenario applied (`expected.json` comes from `cargo run -p forge-wasm --release --example expected`) |
| `games.mjs` | The bagel's price war: every payoff cell, the equilibria marked, the tournament's ranking, scores, stable players and matches are what `forge game` answers, in the plan and both scenarios; an input typed on a card moves them; Tools runs `forge_game` and shows its whole answer, the tool's JSON, with Copy and Download. The Games set: every game of its ten files is Forge's, each equilibrium with its chances and stability, and the stag hunt's scenario and a typed cost move its mixed equilibrium. A game of five players alike is drawn by count, each number every player's payoff in Forge's cells there, its pure equilibria by split (6 at r = 5 for the public goods game's 32); the dynamics end at Forge's shares (hawks at 2/3, the stag hunt all stag, or all hare in `few_stags`), and the chart's pointer shows a generation of Forge's path. The acquisition's sealed bids (`deal_game.yaml`): both pure equilibria marked, and only both at 30% in its scenario. A link with `model=` opens the price war on its games, and a file the set does not have opens the set's own model |
| `scenarios.mjs` | The acquisition's Analyze table has the weighted scenarios Forge returns and names the ones without a probability; a model whose only scenario has none gets Forge's reason (ADR-067, ADR-076) |
| `expect.mjs` | On the stag hunt, each result with an `expect:` shows it as Forge's `calculate` reports it, in the plan and under a scenario, marked when they differ; an input typed that moves a result away from it is kept (ADR-074) |
| `grid.mjs` | Every cell of every table and every scalar in the Grid tab is what Forge returns, with its formula; a tap leads to the card |
| `formats.mjs` | A `format:` on screen: the bagel's price, margin, season and dozens, and the rolling forecast's date, percentages and counts, are the text Forge gives on the cards and in the grid, with the values underneath unchanged |
| `text.mjs`, `formulas.mjs` | Changing an input or a formula changes its one line and nothing else, byte for byte, for each way YAML writes it; an invalid formula is refused with Forge's message and the file stays as it was |
| `elements.mjs` | An input, a formula and a column added to every group and table of every example, then deleted: only the new lines, and the file back byte for byte; refusals leave it as it was |
| `suggest.mjs`, `yamlformula.mjs`, `keys.mjs` | The first letters of every name, function and key in every example are offered at their place (a few more for a name a game's results crowd out of the 16 offered) (on a card, in the Add form and in the YAML editor), only Forge's functions and the schema's keys are offered, and nothing inside a text, a number, a comment or after a colon |
| `excel.mjs`, `save.mjs` | The workbook Studio downloads is the one `forge export` writes, part by part; a workbook `forge export` wrote opens in Studio and calculates to the same numbers; Save shares the model's files and the workbook within the tap Safari requires |
| `offline.mjs` | Every file is kept on the device, the page opens and calculates with the connection cut, and a later release replaces the cache once Studio has been closed |
| `layout.mjs` | No view scrolls sideways at 320 px; at 1440 x 900 the editor and the cards are side by side and follow each other, the divider moves, either side hides |
| `highlight.mjs`, `format.mjs` | The formula colours are the VS Code extension's rules, unchanged; the percent rule and how typed numbers are read |
| `welcome.mjs` | A first visit shows the welcome card (a set's questions, or a single model's three tips), the model still recalculates under it, Got it keeps it away after a reload, and the Files tab brings it back |
| `analyses.mjs` | Every example file that is a decision tree, real options, a bootstrap or a Bayesian network opens on a card whose every number is its tool's, at the place in the answer the card names; on a network a state tapped is evidence, as the tool answers it, and tapped again is not; no card is wider than 390 px |
| `help.mjs` | Every entry of `help.json` links a document of this repository and a heading in it, by GitHub's rule for anchors, and is opened by a "?" on some heading; a sheet links the manual at the running release's tag and opens its example; under a function or a key being typed, Forge's syntax and description |
| `uncertain.mjs` | Every uncertain model of the example sets: every scalar on screen is `forge calculate --plan`'s natively and the pill says how many inputs are ranges; every number of every range, and each distribution's mean, is `forge simulate`'s natively with the model's own settings; each distribution card has Forge's kinds and parameter names. On the bagel, a typed input dims the range and the new one is simulate's, only the last of two quick changes is kept, a distribution's parameter and kind are written, and Analyze lists the outputs in the model's order (`expected.json` gives the native answers) |
| `gameedit.mjs` | A game edited on its card (inspection game, price war): the names "change this game" offers are Forge's; a strategy added, a player renamed, a strategy deleted and a tournament's player added give the cards of `game` for the files `edit` wrote; a change Forge refuses shows its message |
| `sets.mjs` | A set of several models lists them as questions on the welcome card, each with the kind of model Forge's `model` says its file is; a question, a chip of the strip at the top of the Model view and Files open each model of every such set (ADR-077) |
| `live.mjs` | After each release, the published page runs the release's Forge and the bagel example's `year.profit` is 786.32 |

Run one with `node studio/tests/<name>.mjs studio-site` (after `npm install` here); `no-engine.mjs`, `text.mjs`, `format.mjs` and `highlight.mjs` need no built site. `screen.mjs` and `uncertain.mjs` also take `forge-wasm/expected.json`, `excel.mjs` and `save.mjs` the folder of workbooks `forge export` wrote (`forge-wasm/xlsx`), and `layout.mjs` a folder for its screenshots, as in `.github/workflows/ci.yml`.
