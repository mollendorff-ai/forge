# tests/

Every test is going inline, into a `#[cfg(test)] mod tests` in the file it tests (CLAUDE.md, roadmap T1-TESTS-INLINE); `tests/no_skipping.rs` counts the files still here, and the count only falls. What is left:

- the files that start the `forge` binary. They move into the modules they test and call the command line in-process: `cli::run_from(args, console)` runs `forge` from its arguments to its exit code, and in a test `cli::console::capture` gives it buffers and returns what it wrote (see the tests in `src/cli/run.rs`);
- the checks of the repository's own text (`conventions.rs`, `no_skipping.rs`, `doc_counts.rs`, `doc_examples.rs` and the others), which move to a validation program of their own.

What only the built program can show is run by `scripts/coverage.sh`, which runs it after the tests and adds it to the coverage (`main`, and MCP over stdin and stdout), and by the MCP Protocol Test job in CI.

The end-to-end suites are in `validation/` (ADR-057). They run the release binary from `.github/workflows/validation.yml` on pull requests and weekly, and are not part of `cargo test`, which needs neither Gnumeric nor R:

- [forge-e2e-gnumeric](../validation/gnumeric): formulas against Gnumeric, and the Excel export and import round trip;
- [forge-e2e-r](../validation/r): statistics and analytics against R.

```bash
cargo test               # every test in this crate
scripts/coverage.sh      # the tests and the built program, with their coverage
```
