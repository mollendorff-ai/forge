# forge-wasm

Forge's tools in a browser (ADR-055). Each tool runs the same core function as the command line and the MCP server (`calculate_core`, `simulate_core`, ...) on a model held in memory. Nothing in this crate calculates.

It is part of this repository's workspace and is not published. [Forge Studio](../studio/README.md) (ADR-056) is built on it: the page calls `call(tool, request)` in a worker and calculates nothing itself, so what you see in the browser is what the command line would print. Try it at <https://forge.mollendorff.ai/>.

## Calling it

One function, `call(tool, request)`, takes and returns JSON text:

```js
const answer = JSON.parse(forge.call("goal_seek", JSON.stringify({
  files: { "bagel.yaml": bagelText, "costs.yaml": costsText },
  main: "bagel.yaml",
  target: "year.profit", value: 60000, vary: "assumptions.staff_hours_per_day",
})));
// { ok: { solution: 24.2687..., converged: true, ... } }  or  { error: "..." }
```

`files` holds the main model and every file it includes, by the path written in `_includes`. `main` names the model (default `model.yaml`). The other fields are the MCP tool's arguments without file paths. `forge.version()` is the Forge version it runs.

| Tool | Arguments |
| --- | --- |
| `validate`, `decision_tree`, `real_options` | none |
| `calculate` | `scenario` |
| `audit` | `variable` |
| `scenarios` | `scenario_filter` |
| `compare` | `scenarios` (list) |
| `goal_seek` | `target`, `value`, `vary`, `min`, `max`, `tolerance` (0.0001) |
| `break_even` | `output`, `vary`, `min`, `max` |
| `sensitivity` | `vary`, `range` (`"0.08,0.11,0.005"`), `vary2`, `range2`, `output` |
| `tornado` | `output_var` |
| `simulate` | `iterations`, `seed`, `sampling`, `scenario` |
| `probability_seek` | `goal` (`{output, threshold, probability}`), `input`, `min`, `max`, and those of `simulate` |
| `variance` | `budget`, `actual` (names in `files`), `threshold` (10) |
| `report` | `groups`, `keys`, `no_inputs` |
| `bootstrap` | `iterations`, `seed`, `confidence` (list) |
| `bayesian` | `query`, `evidence` (`["sales=slow"]`) |
| `functions`, `examples` (`name`), `schema` (`version`) | no `files` |
| `export` | none: the workbook `forge export` writes, as `excel_base64`, with `byte_count`, `table_count`, `scalar_count` and `warnings` (the MCP tool's in-memory answer) |
| `import` | `excel_base64`, `name` (`model.yaml`), no `files`: the model `forge import` writes from that workbook, as `files` (the main model and its included files, by name) and `main`, with `warnings` |
| `model` (not an MCP tool) | none: the model read and not calculated, as Forge Studio's cards show it: each scalar's input value or formula with its unit, each table's columns (data or formula) in the file's order, and each included file's scalars as `@namespace.group.name` |

`calculate` never writes results back: there is no file. The module is built without the engine's command line and servers (ADR-055) and runs inside the page's own tab: nothing in it sends a model anywhere. A workbook made by `export` is the one `forge export` writes from the same files, apart from the time it was made (docProps/core.xml, to the second); CI compares them part by part.

## How it is checked

- `cargo test -p forge-wasm --release`: every case of `tests/cases.json`, all on the repository's `examples/`, gives the same answer from files in memory as from the same files on disk; the headline numbers of the bagel and mortgage examples come out.
- CI (job `forge-wasm`, which also tests Forge Studio) builds the module, runs the same cases in Node, and compares each answer with the native one (`examples/expected.rs`, `tests/node/compare.mjs`). The module's size and each case's time are in the job's summary.

## Building it

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
cargo build -p forge-wasm --profile wasm --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/wasm/forge_wasm.wasm
```

The `wasm` profile (in the workspace `Cargo.toml`) builds for size: the module is about 1.86 MB gzipped, 20% less than with the release profile, and as fast. CI fails if it grows past 2.0 MB gzipped.

`--target web` gives an ES module for a page, which Forge Studio uses (`studio/README.md`); the Node comparison uses `--target nodejs`.
