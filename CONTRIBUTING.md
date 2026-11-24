# Contributing to Forge

## How to Contribute

| Method | Description |
|--------|-------------|
| **Issues** | Report bugs, request features |
| **Discussions** | Ask questions, share ideas |
| **Pull Requests** | Bug fixes, new functions, documentation |
| **Forks** | Create your own version |

## Development

```bash
make check          # Format + lint + unit tests
make test           # Run all tests
make lint           # Clippy with pedantic lints
```

## What a change needs

- **An entry in CHANGELOG.md**, under `[Unreleased]`, saying in plain words what a user saw and what changed.
- **A test that fails without the change** for every bug fix: a wrong answer, an error, a crash or a wrong message. The test reproduces the bug, fails on the commit before the fix and passes after it.
- **An example with outside numbers** for a new or changed command: a model in `examples/` whose results come from a calculation made outside Forge (by hand, a spreadsheet, R, Python), asserted in a test. `src/cli/commands/examples.rs` (its `bagel_example` tests) shows how. Using the bagel models found more bugs in one evening than the unit tests had.
- **Green CI**, including the Validation workflow, which runs the suites in `validation/` (forge-e2e-gnumeric, forge-e2e-r, the generated cases of `validation/sweep` and the games of `validation/games`; ADR-057) against the build of the same commit. New cases go in the same pull request. A pull request that changes or removes an existing expected value there says `changes expected values` in its title and, in its description, where each new value comes from (Excel, Gnumeric, R; the version and the date); the Expected values check fails it otherwise.
- **Docs that stay true**: update what the change makes wrong; a page about a design that no longer exists gets a dated note at the top.
- **No skipping the rules** in CLAUDE.md: 100% coverage, pedantic lints and formatting, with no `allow`, no coverage-only code, no ignored test and no formatting skip; tests inline, in the file they test; no file over 1,500 lines. `validation/repo/src/no_skipping.rs` counts what is still left from before the rules, and fails if a change adds to it.
- **Measure coverage with Rust 1.100 or later**, the toolchain CI's stable is: `RUSTUP_TOOLCHAIN=1.100.0 make coverage` (`rustup toolchain install 1.100.0 --component llvm-tools-preview`). `scripts/coverage.sh` refuses an older `rustc`, which counts fewer regions inside macros and reads 100% where CI does not.

The pull request template has these as a checklist. `.asimov/project.yaml` lists the conventions; `validation/repo/src/conventions.rs` checks the ones a test can.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as MIT OR Apache-2.0,
without any additional terms or conditions.
