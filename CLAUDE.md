# Working on Forge

## Rules that are not negotiable

- **100% test coverage.** CI is to fail below it (roadmap T1-COVERAGE-GATE, ADR-004). A test asserts a result; one that only runs the code does not count.
- **Pedantic linting.** `cargo clippy --locked --all-targets -- -D warnings` passes, on stable and on the oldest supported Rust (`rust-version` in Cargo.toml). Fix the code; an `#[allow(...)]` needs a reason in a comment next to it.
- **Formatting.** `cargo fmt --all -- --check` passes.

## Proposing the next step

Whenever you propose what to do next, give each step the model and the effort level to run it with, and say why in a few words. For example:

| Step | Model | Effort | Why |
| --- | --- | --- | --- |
| Fix the parser's nesting limit | Sonnet 5.5 | medium | A contained change with a clear test |

Choose for the work, not the default: a wrong answer in the engine or a large gate deserves a stronger model or more effort than a mechanical change.
