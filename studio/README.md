# Forge Studio

Forge in your browser, phone first (ADR-056): **<https://mollendorff-ai.github.io/forge/>**

Open one of the examples, a YAML file from your device, or an Excel workbook, and the model shows as cards: inputs to change, results with their formulas, tables, scenarios. Change a number and Forge calculates the model again. It is the same engine as the command line, running in the page as WebAssembly.

## What it does

- **Model:** inputs as sliders and fields, results with their formulas, tables, scenarios to apply. A formula can be changed on its card, with suggestions for names and functions; it is kept only when Forge accepts it, and otherwise Forge's message says why. A group's Add button adds an input, a formula or a table column, and a card's Delete removes it; each change is kept only when Forge reads back exactly that change.
- **Grid:** the model as a spreadsheet reads it, read-only. A tap shows a cell's formula and leads to its card.
- **YAML:** the model's text, with Forge's formula colours, and suggestions for the keys of the model format and for names and functions inside a formula. On a phone the edit is applied with a button; above 900 px the editor sits beside the cards and a number typed in one changes the other.
- **Analyze:** goal seek, break-even, scenarios, a tornado, a simulation and budget against actual. **Tools:** the MCP server's 20 tools, run on the open model.
- **Files:** the examples, opening and saving. Save opens the share sheet on an iPhone or iPad (Save to Files, iCloud Drive) for the model's YAML or its Excel workbook.

## Your model stays on your device

There is no server doing the work: the page runs `forge-wasm` (ADR-055) in a background worker, and nothing you type is sent anywhere. There are no accounts, no cookies and no analytics. Your files are kept in the browser's storage on your device until you save them. GitHub Pages serves the page, so GitHub sees that it was downloaded, as with any website.

It also works offline. Once Forge is running, the page keeps its own files on the device (`sw.js`), so Studio opens and calculates without a connection, and on an iPhone it can be added to the Home Screen (Share, Add to Home Screen) to open like an app. A new release replaces the files the next time Studio is opened after being closed, never half of each, so close it fully and open it again to get one.

## How it is built

Every number is Forge's. The page reads no YAML itself: a change on a card is written into that line of the YAML, comments and all, and Forge calculates the files again.

| File | What it holds |
| --- | --- |
| `index.html`, `studio.css` | The page |
| `app.js` | State, the six views, events |
| `forge.js`, `worker.js` | The only way to a number: `call(tool, request)` to forge-wasm, in a worker |
| `text.js` | Text work on YAML: changing one input's or one formula's line, colouring the editor with the VS Code grammar's rules (`formula-rules.json`, written by `build.mjs` from `editors/vscode/syntaxes/`) |
| `suggest.js` | What to offer while a formula or a key is typed: names from the model, functions from Forge's own list, keys from Forge's schema (the page keeps no list of its own) |
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

On GitHub Pages, at `https://mollendorff-ai.github.io/forge/`, after each release: the Release workflow starts `.github/workflows/pages.yml` with its tag. The page's status pill shows the Forge version it runs. The workflow builds at the release's tag, deploys, and then opens the published address in a headless browser (`tests/live.mjs`): the version must be the release's and the bagel example's `year.profit` must be 786.32.

Pages is switched on once, by an administrator: Settings, Pages, Source: GitHub Actions. To publish by hand, run the workflow from the Actions tab with a tag or a branch; it defaults to `main`. Releases up to v10.0.0-rc.13 predate Studio and cannot be published. The page uses relative paths only, so a custom domain needs no change to the files.

## How it is checked

All of these run in CI, in the `forge-wasm` job, which keeps the built site as the `forge-studio` artifact. Most of them compare what the page shows with what Forge returns for every example, so they check the page against the engine, not against a number someone typed.

| Test | What it checks |
| --- | --- |
| `no-engine.mjs` | No file of the page loads a YAML library or implements what Forge calculates |
| `screen.mjs` | Every scalar on screen equals `forge calculate` run natively for that model, and the same with a scenario applied (`expected.json` comes from `cargo run -p forge-wasm --release --example expected`) |
| `grid.mjs` | Every cell of every table and every scalar in the Grid tab is what Forge returns, with its formula; a tap leads to the card |
| `text.mjs`, `formulas.mjs` | Changing an input or a formula changes its one line and nothing else, byte for byte, for each way YAML writes it; an invalid formula is refused with Forge's message and the file stays as it was |
| `elements.mjs` | An input, a formula and a column added to every group and table of every example, then deleted: only the new lines, and the file back byte for byte; refusals leave it as it was |
| `suggest.mjs`, `yamlformula.mjs`, `keys.mjs` | The first letters of every name, function and key in every example are offered at their place (on a card, in the Add form and in the YAML editor), only Forge's functions and the schema's keys are offered, and nothing inside a text, a number, a comment or after a colon |
| `excel.mjs`, `save.mjs` | The workbook Studio downloads is the one `forge export` writes, part by part; a workbook `forge export` wrote opens in Studio and calculates to the same numbers; Save shares the model's files and the workbook within the tap Safari requires |
| `offline.mjs` | Every file is kept on the device, the page opens and calculates with the connection cut, and a later release replaces the cache once Studio has been closed |
| `layout.mjs` | No view scrolls sideways at 320 px; at 1440 x 900 the editor and the cards are side by side and follow each other, the divider moves, either side hides |
| `highlight.mjs`, `format.mjs` | The formula colours are the VS Code extension's rules, unchanged; the percent rule and how typed numbers are read |
| `live.mjs` | After each release, the published page runs the release's Forge and the bagel example's `year.profit` is 786.32 |

Run one with `node studio/tests/<name>.mjs studio-site` (after `npm install` here); `no-engine.mjs`, `text.mjs`, `format.mjs` and `highlight.mjs` need no built site.
