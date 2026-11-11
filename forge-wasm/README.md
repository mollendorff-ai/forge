# forge-wasm

Forge's tools in a browser (ADR-055). Each tool runs the same core function as the command line and the MCP server (`calculate_core`, `simulate_core`, ...) on a model held in memory. Nothing in this crate calculates.

It is part of this repository's workspace and is not published. Forge Studio (ADR-056) will call it.

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
| `model` (not an MCP tool) | none: the model read and not calculated, as Forge Studio's cards show it: each scalar's input value or formula with its unit, each table's columns (data or formula) in the file's order, and each included file's scalars as `@namespace.group.name` |

`calculate` never writes results back: there is no file. Excel export and import are not here yet (roadmap S8).

## How it is checked

- `cargo test -p forge-wasm --release`: every case of `tests/cases.json`, all on the repository's `examples/`, gives the same answer from files in memory as from the same files on disk; the headline numbers of the bagel and mortgage examples come out.
- CI (job `forge-wasm`, which also tests Forge Studio) builds the module, runs the same cases in Node, and compares each answer with the native one (`examples/expected.rs`, `tests/node/compare.mjs`). The module's size and each case's time are in the job's summary.

## Building it

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
cargo build -p forge-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/forge_wasm.wasm
```

`--target web` gives an ES module for a page, which Forge Studio uses (`studio/README.md`); the Node comparison uses `--target nodejs`.
