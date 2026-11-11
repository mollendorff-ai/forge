# Forge Studio

A web page for Forge's YAML models, phone first (ADR-056). It opens the models of this repository's `examples/`, YAML files from the device, or an Excel workbook (imported by Forge), and shows them as cards: inputs to change, results with their formulas, tables, scenarios. Analyze runs goal seek, break-even, scenarios, a tornado, a simulation and budget against actual; Tools runs the MCP server's 20 tools on the open model; YAML is the model's text, with Forge's formula colours.

On a phone it shows one view at a time with tabs at the bottom. Above 900 px the editor is on the left and the other views on the right: a number typed in the editor changes its card, a number changed on a card shows in the editor, the divider between them can be dragged, and either side can be hidden.

A formula can be changed on its card: it is kept only when Forge accepts it, and otherwise Forge's message says why.

Every number is Forge's. The page runs `forge-wasm` (ADR-055) in a background worker and reads no YAML itself: a change on a card is written into that line of the YAML, comments and all, and Forge calculates the files again. Nothing leaves the device; the files stay in the browser's storage.

| File | What it holds |
| --- | --- |
| `index.html`, `studio.css` | The page |
| `app.js` | State, the five views, events |
| `forge.js`, `worker.js` | The only way to a number: `call(tool, request)` to forge-wasm, in a worker |
| `text.js` | Text work on YAML: changing one input's or one formula's line, colouring the editor with the VS Code grammar's rules (`formula-rules.json`, written by `build.mjs` from `editors/vscode/syntaxes/`) |
| `format.js` | How numbers look on screen: one rule (`isPercent`) for when a number is a percentage, by the model's unit first |
| `build.mjs` | Puts the page, forge-wasm and the example sets in one folder |

## Building it

```bash
cargo build -p forge-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir forge-wasm/pkg-web target/wasm32-unknown-unknown/release/forge_wasm.wasm
node studio/build.mjs forge-wasm/pkg-web studio-site
```

`studio-site` is a static site. Serve it over HTTP (a worker and `fetch` do not run from `file://`), for example `python3 -m http.server -d studio-site`.

## Where it is published

On GitHub Pages, at `https://mollendorff-ai.github.io/forge/`, after each release (`.github/workflows/pages.yml`). The page's status pill shows the Forge version it runs. The workflow builds at the release's tag, deploys, and then opens the published address in a headless browser (`tests/live.mjs`): the version must be the release's and the bagel example's `year.profit` must be 786.32.

Pages is switched on once, by an administrator: Settings, Pages, Source: GitHub Actions. To publish by hand, run the workflow from the Actions tab with a tag or a branch; it defaults to `main`. Releases up to v10.0.0-rc.13 predate Studio and cannot be published. The page uses relative paths only, so a custom domain needs no change to the files.

## How it is checked

- `node studio/tests/no-engine.mjs`: no file of the page loads a YAML library or implements what Forge calculates.
- `node studio/tests/screen.mjs studio-site forge-wasm/expected.json` (after `npm install` here): a headless browser opens each example set and checks that every scalar on screen equals what `forge calculate` gives natively for that model, and the same with a scenario applied. `expected.json` comes from `cargo run -p forge-wasm --release --example expected`.

- `node studio/tests/layout.mjs studio-site [screenshots dir]`: no view scrolls sideways at 320 px; at 1440 x 900 the editor and the cards follow each other, the divider moves and either side hides; screenshots at 390 x 844 and 1440 x 900, light and dark.

- `node studio/tests/highlight.mjs [studio-site]`: the colours of formulas are the VS Code grammar's (patterns, order, scopes), and highlighting leaves every YAML file of `examples/` unchanged once its tags are removed.

- `node studio/tests/text.mjs`: changing an input or a formula changes its one value and nothing else, for each way YAML writes it.
- `node studio/tests/formulas.mjs studio-site`: every formula of every example, changed on its card, changes its one line and Forge reads it back; invalid formulas are refused with Forge's message and no change.

- `node studio/tests/excel.mjs studio-site <workbooks>`: the workbook Studio downloads for bagel and mortgage is the one `forge export` writes, part by part, apart from when it was made; a workbook `forge export` wrote, opened in Studio, calculates to the same numbers.
- `node studio/tests/format.mjs`: the percent rule and how typed numbers are read.

All eight run in CI, in the `forge-wasm` job, which keeps the built site as the `forge-studio` artifact.
