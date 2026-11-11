# Forge Studio

A web page for Forge's YAML models, phone first (ADR-056). It opens the models of this repository's `examples/`, or YAML files from the device, and shows them as cards: inputs to change, results with their formulas, tables, scenarios. Analyze runs goal seek, break-even, scenarios, a tornado, a simulation and budget against actual; Tools runs the MCP server's 20 tools on the open model; YAML is the model's text, with Forge's formula colours.

Every number is Forge's. The page runs `forge-wasm` (ADR-055) in a background worker and reads no YAML itself: a change on a card is written into that line of the YAML, comments and all, and Forge calculates the files again. Nothing leaves the device; the files stay in the browser's storage.

| File | What it holds |
| --- | --- |
| `index.html`, `studio.css` | The page |
| `app.js` | State, the five views, events |
| `forge.js`, `worker.js` | The only way to a number: `call(tool, request)` to forge-wasm, in a worker |
| `text.js` | Text work on YAML: changing one input's line, colouring the editor |
| `format.js` | How numbers look on screen |
| `build.mjs` | Puts the page, forge-wasm and the example sets in one folder |

## Building it

```bash
cargo build -p forge-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir forge-wasm/pkg-web target/wasm32-unknown-unknown/release/forge_wasm.wasm
node studio/build.mjs forge-wasm/pkg-web studio-site
```

`studio-site` is a static site. Serve it over HTTP (a worker and `fetch` do not run from `file://`), for example `python3 -m http.server -d studio-site`.

## How it is checked

- `node studio/tests/no-engine.mjs`: no file of the page loads a YAML library or implements what Forge calculates.
- `node studio/tests/screen.mjs studio-site forge-wasm/expected.json` (after `npm install` here): a headless browser opens each example set and checks that every scalar on screen equals what `forge calculate` gives natively for that model, and the same with a scenario applied. `expected.json` comes from `cargo run -p forge-wasm --release --example expected`.

Both run in CI, in the `forge-wasm` job, which keeps the built site as the `forge-studio` artifact.
