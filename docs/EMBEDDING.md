# Embedding Forge

For a developer who wants Forge's engine inside something else: a Rust program, a service that calls it over HTTP, a web page, or an AI agent's host. Pick the door that fits; all four run the same core functions, so a model gives the same numbers through each (CI compares forge-wasm's answers with the native ones).

| You have | Use | Start at |
| --- | --- | --- |
| A Rust program | The library, `mollendorff-forge` | [The library](#the-library) |
| Any language, and a server | `forge serve`, the REST API | [The REST API](#the-rest-api) |
| A web page | `forge-wasm`, the engine in WebAssembly | [In a browser](#in-a-browser) |
| An AI agent or its host | `forge mcp`, the MCP server | [MCP.md](MCP.md) |

## The library

The crate is [`mollendorff-forge`](https://crates.io/crates/mollendorff-forge) (the library is `mollendorff_forge`), and its documentation is on [docs.rs](https://docs.rs/mollendorff-forge). In `Cargo.toml`, with the version from crates.io:

```toml
[dependencies]
mollendorff-forge = { version = "10.0.0-rc.37", default-features = false }
```

`default-features = false` leaves out the command line, the MCP server and the REST API (feature `cli`), and with them most of the dependencies: what is left is the parser, the calculator, the analyses and each command's `*_core` function, which build for `wasm32-unknown-unknown`. Add `features = ["excel"]` for the workbook export and import. The default features are the whole `forge` program.

A model is read from text and calculated, with nothing read from or written to the file system:

```rust
use std::collections::HashMap;

use mollendorff_forge::core::ArrayCalculator;
use mollendorff_forge::parser::parse_model_text;

let text = r#"
_forge_version: "5.1.0"
plan:
  price: 40
  units: 120
  revenue: {formula: "=plan.price * plan.units"}
sales:
  units: [10, 20, 30]
  revenue: "=units * plan.price"
"#;

let model = parse_model_text("plan.yaml", text, &HashMap::new())?;
let calculated = ArrayCalculator::new(model).calculate_all()?;

assert_eq!(calculated.scalars["plan.revenue"].value, Some(4800.0));
# Ok::<(), mollendorff_forge::ForgeError>(())
```

This is the example of the crate's own documentation, which `cargo test --doc` runs on every change. In the other direction:

- `parse_model(path)` reads a file, and its `_includes` from the disk. `parse_model_text` takes the text of each included file in the map, by the path the model writes.
- `ParsedModel` holds `scalars` and `tables`; after `calculate_all` each `Variable` has its `value`. A table's columns are in `tables`.
- Each command's core function (`calculate_core`, `simulate_core`, ...) in `mollendorff_forge::cli` takes a path or text and returns a result that is `Serialize`, and prints nothing. Use these, not the printing functions, to get what a command prints as data. [10 - Extending Forge](architecture/10-EXTENDING-FORGE.md#add-a-command) explains why each command has one.
- [STABILITY.md](STABILITY.md) promises the model format and the commands, not the library's Rust API: pin the version.

## The REST API

`forge serve` serves every MCP tool over HTTP, as JSON: `validate`, `calculate`, `audit`, `export` and `import` with requests of their own, and each other tool (`simulate`, `goal-seek`, `scenarios`, `edit`, ...) at `POST /api/v1/<name>` with the arguments its MCP tool takes. `GET /api/v1/openapi.json` is an OpenAPI 3.1 document of all of them, for a client generator. [FEATURES.md](FEATURES.md#http-rest-api-server) has each endpoint's request and answer; this is the shortest use, a model sent as text:

```bash
forge serve --port 8080
```

```bash
curl -X POST http://127.0.0.1:8080/api/v1/calculate \
  -H "Content-Type: application/json" \
  -d '{"content": "_forge_version: \"5.1.0\"\nplan:\n  price: 40\n  units: 120\n  revenue: {formula: \"=plan.price * plan.units\"}\n", "dry_run": true}'
```

The answer, here without its `request_id`:

```json
{"success": true, "data": {"calculated": true, "dry_run": true, "result": {"scalars": {"plan.price": 40.0, "plan.revenue": 4800.0, "plan.units": 120.0}}}}
```

Every answer of the five has HTTP status 200 and `success: true` when the request was handled, also when the model could not be calculated: `data.calculated` says whether it was, and `data.message` why not. A tool endpoint answers `200` with the tool's JSON in `data`, `422` with `success: false` and `error` when the tool fails or its arguments are not its request, and `400` when the body is not JSON. By default the server listens only on `127.0.0.1` and rejects other `Host` headers; to listen elsewhere set `FORGE_API_TOKEN` and send `Authorization: Bearer <token>`; `--root DIR` keeps file paths inside one folder, and `content` needs no file at all. [FEATURES.md](FEATURES.md#http-rest-api-server) has the security rules.

## In a browser

`forge-wasm` is the engine built for `wasm32-unknown-unknown`, with one function, `call(tool, request)`, that takes and returns JSON text; it runs inside the page's tab and sends nothing anywhere. [forge-wasm/README.md](../forge-wasm/README.md) lists every tool with its arguments and says how to build it:

```bash
cargo build -p forge-wasm --profile wasm --target wasm32-unknown-unknown
```

[Forge Studio](../studio/README.md) is a page built on it, at <https://forge.mollendorff.ai/>. The module is not published as a package: you build it from a clone of this repository, with the set-up of [CONTRIBUTING.md](../CONTRIBUTING.md#setting-up) (the target and `wasm-bindgen-cli`), and it must stay under 3,000,000 bytes gzipped (ADR-070).

## An AI agent

`forge mcp` speaks the Model Context Protocol over standard input and output and offers every tool of the [MCP reference](MCP.md); [the guide's AI page](guide/ai.md) shows it in each client. To build an agent that calls Forge from code, start `forge mcp --root DIR` as a child process, so the agent can reach only the models in `DIR`, and speak MCP to it with a client library.
