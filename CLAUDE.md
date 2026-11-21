# Working on Forge

## Rules that are not negotiable

No exceptions and no skipping, in every crate (the main crate, `forge-wasm`, `validation/gnumeric`, `validation/r`). Switching a rule off is not fixing the code.

- **100% test coverage.** Lines, functions and regions, measured by `cargo llvm-cov` over everything outside `tests/`. A test asserts a result; one that only runs the code does not count. No code compiled only for or only without the coverage run (`cfg(coverage)`, `cfg(not(coverage))`, `cfg_attr(coverage, ...)`), no `coverage(off)`, and no ignored test (`#[ignore]`, or `ignore` inside a `cfg_attr`): a test that cannot run in CI is fixed until it can. CI fails below the floor in `.github/workflows/ci.yml`; the floor only rises, to 100 (roadmap T1-COVERAGE-GATE), and `make coverage` already fails below 100.
- **Pedantic linting.** `cargo clippy --locked --all-targets -- -D warnings` passes on stable and on the oldest supported Rust (`rust-version` in Cargo.toml), with the `pedantic`, `nursery` and `cargo` groups on in every crate. No `#[allow(...)]` or `#[expect(...)]`, alone or inside a `cfg_attr`, and no lint set to `"allow"` in a `Cargo.toml`, `multiple_crate_versions` included: fix the code, or the dependency.
- **Formatting.** `cargo fmt --all -- --check` passes, with no `#[rustfmt::skip]` and no `ignore` in `rustfmt.toml`.
- **Tests are inline, all of them.** Every test is in a `#[cfg(test)] mod tests` inside the file it tests: no `tests/` folder in any crate, and no test module in a file of its own (`mod foo_tests;`). A test of the command line or the MCP server calls the same code in-process (the command's `run`, the tool's handler) rather than starting the binary.
- **No file over 1,500 lines, its tests included.** A longer file is split into modules by what it does; moving its tests out is not a split.

`validation/repo/src/no_skipping.rs` counts each way round these rules, the test files outside the code they test included. The counts may only fall, to zero (roadmap T1-NO-SKIPPING): the test fails when one rises, and when one falls without its budget being lowered in the same change. Never raise a budget or lower a coverage floor.

## Libraries before code of our own

Prefer a stable, well-maintained library to code of our own. Size alone is never the reason to replace one while Forge Studio's WebAssembly module is under the ceiling of ADR-070 (3,000,000 bytes gzipped); code of our own needs its own merits (more correct, simpler, or what the library cannot do). Forge's names are Unicode (`hypothèses.coût_unitaire`, `假设.单价`): whatever matches them must too. The rule against crates in two versions does not compete with keeping a core, stable library: `getrandom` and its `r-efi` are allowed twice for `jsonschema` (ADR-071).

## Proposing the next step

Whenever you propose what to do next, give each step the model and the effort level to run it with, and say why in a few words. For example:

| Step | Model | Effort | Why |
| --- | --- | --- | --- |
| Fix the parser's nesting limit | Sonnet 5.5 | medium | A contained change with a clear test |

Choose for the work, not the default: a wrong answer in the engine or a large gate deserves a stronger model or more effort than a mechanical change.
