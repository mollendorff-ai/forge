# ADR-055: Forge in the Browser (forge-wasm)

**Status:** Accepted; built (roadmap `studio`, S1 and S2)
**Date:** 2026-11-10
**Decided by:** the maintainer (a web interface that runs Forge's own engine, built and published from this repository); the design is Claude's, in a session configured for Opus 5.5

---

## Context

Forge has two kinds of user in mind: an AI agent, which reaches it through the MCP server, and a person, who until now needs a terminal. A web interface for people, phone first, is planned in ADR-056 (Forge Studio). It needs Forge's results in a browser.

A mockup of that interface was built outside this repository between 2026-11-07 and 2026-11-10. It calculates with its own engine, a JavaScript rewrite of part of Forge. Against `forge calculate` of rc.11 on nine example models it matched all 187 scalars, the goal seeks on the mortgage and bagel examples (397.044208 and 24.268745), and every line of `forge variance` on the household and bagel examples. It also showed why a second engine is the wrong foundation:

- **It drifts.** The model format changed twice in three days (5.1.0, ADR-051; 5.2.0 with `PREV` and period columns, ADR-053), and the mockup's engine had to be rewritten to follow. ADR-054 changed what a date is the next day; the mockup does not follow it yet.
- **It is partial.** About 50 of Forge's 175 functions, no decision trees, real options, bootstrap or Bayesian networks, and plain random sampling where Forge uses Latin hypercube, so a simulation is close and never equal (35.9% against 34.76% for the chance of a profit in `examples/bagel/bagel_simulate.yaml`).
- **It is a second place a wrong answer can live**, with none of Forge's validation behind it.

WebAssembly runs compiled Rust in a browser. Two things were checked before deciding:

- **The hosts allow it.** A test page loaded a 41-byte module six ways (compile in the page, compile without blocking, load a separate `.wasm` file, stream-compile that file, run it in a background worker). All six passed in the Claude app on an iPhone on 2026-11-10. GitHub Pages serves `.wasm` files as `application/wasm`, which streaming compilation needs.
- **The core is nearly free of the operating system.** Outside tests, the parser and calculator read files in two places: the model (`parser/mod.rs`, `parse_model`) and its includes (`parser/includes.rs`). What a browser cannot run belongs to the command line and the servers: `tokio`, `axum`, `tower`, `rmcp`, `notify`, `tempfile`, `clap`. `rand` needs its browser source of randomness, and `jsonschema` must not fetch over HTTP.

Not checked: that the crate compiles to `wasm32-unknown-unknown` (the session that wrote this could not download the target), its size, and its speed in a phone's browser.

## Decision

1. **A crate `forge-wasm`, in this repository**, a workspace member that depends on `mollendorff-forge` without its default features. It is a thin layer: every function takes the model as text and its includes as a map of name to text, and returns JSON. No calculation lives in it.

2. **What it exposes, first:** `validate`, `calculate` (with a scenario), `audit`, `scenarios`, `goal_seek`, `break_even`, `sensitivity`, `tornado`, `simulate` (with a seed), `variance` (two models), `report`, `decision_tree`, `real_options`, `bootstrap`, `bayesian`, `functions`, `examples`, `schema`. These are the MCP tools (ADR-047, ADR-049) with the same inputs, minus file paths. Excel `export` and `import` come later, if `rust_xlsxwriter` and `calamine` build for the browser (roadmap S8).

3. **The command line and the servers move behind a default Cargo feature.** `cargo install mollendorff-forge`, `cargo build` and the release binaries do not change. `--no-default-features` builds the parser, the calculator and the analysis engines alone.

4. **The parser takes a model from memory.** A model source is the main text and a map of include texts. Reading files becomes one way of filling it. The MCP server's inline content (ADR-049), which writes temporary files today, can use the same path later; that is not part of this decision.

5. **Randomness.** A simulation with a seed gives the same numbers in the browser as on the command line. `rand` uses the browser's random source only for a run without a seed.

6. **One engine.** The mockup's JavaScript engine is not carried into the repository as a fallback. A browser that cannot run WebAssembly gets a message that says so. The numbers a person sees in Forge Studio are Forge's.

7. **Built and checked by CI.** A workflow builds the crate with `wasm-bindgen` (the tool `wasm-pack` runs), runs the example models through the WebAssembly build, and compares them with the numbers already asserted in `tests/demo_examples.rs` and `tests/bagel_example.rs`. Its size is measured and written in the workflow's summary; no size is promised here.

## Alternatives considered

- **Keep the JavaScript engine.** It works today and loads instantly. It is a second engine that drifts and has no validation behind it. Rejected, see Context.
- **Keep it as a fallback.** Two engines that give different numbers for the same model, with the user not told which one answered. Rejected.
- **A hosted API** (`forge serve` behind a web address). It needs a server to run and pay for, and every model would leave the user's device. Rejected for Forge Studio; the REST API stays for those who host it (T3-API).
- **A separate repository for the browser build.** It would release on its own clock and fall behind the engine. Rejected.

## Consequences

- **Code changes outside the browser crate:** `Cargo.toml` (features, a workspace) and the parser's file reading. These touch the engine; they are made in their own pull request, merged before other engine work starts on the same files.
- **CI gains a Rust target and `wasm-pack`.**
- **`forge_examples` and the docs can say that Forge runs in a browser** only after the CI check of decision 7 passes.
- **Until it is built, the mockup stays outside the repository** and is described as a mockup wherever it is shown.
