# Contributing to Forge

## How to Contribute

| Method | Description |
|--------|-------------|
| **Issues** | Report bugs, request features |
| **Discussions** | Ask questions, share ideas |
| **Pull Requests** | Bug fixes, new functions, documentation |
| **Forks** | Create your own version |

An AI agent working in this repository reads [AGENTS.md](AGENTS.md) first (`CLAUDE.md` imports it): the rules that are not negotiable. [docs/architecture/](docs/architecture/README.md) says how Forge is built, and [10 - Extending Forge](docs/architecture/10-EXTENDING-FORGE.md) has a checklist for each common change. A maintainer making a release reads [docs/MAINTAINING.md](docs/MAINTAINING.md).

## Setting up

`make check` needs Rust (1.97 or later), Node and `markdownlint-cli2`, the first three rows below. The other tools are for the suites that compare Forge with other engines, and CI runs those if you do not.

| What | Which | Why |
| --- | --- | --- |
| Rust | Stable, with `rustfmt` and `clippy` (`rustup component add rustfmt clippy`). CI's stable is 1.100.0; the oldest Rust Forge builds with is 1.97 (`rust-version` in `Cargo.toml`, which `clippy.toml` repeats) | Building, `make lint`, `make format-check` |
| Node 22 | `node` and `npm` | Forge Studio's tests and the diagram check run on Node 22; `markdownlint-cli2` is an npm package |
| `markdownlint-cli2` | `npm install -g markdownlint-cli2` | `make validate-docs`, the last step of `make check` (and `make validate-docs-all`) |
| Rust 1.100.0 with `llvm-tools-preview`, and `cargo-llvm-cov` | `rustup toolchain install 1.100.0 --component llvm-tools-preview`, `cargo install cargo-llvm-cov` | `make coverage` |
| The `wasm32-unknown-unknown` target and `wasm-bindgen-cli` | `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version <the version> --locked`, the `version` under `name = "wasm-bindgen"` in `Cargo.lock` | `forge-wasm` and Forge Studio |
| Gnumeric | `sudo apt-get install -y gnumeric` (`brew install gnumeric` on macOS): `ssconvert` must be in `PATH` | `validation/gnumeric` |
| LibreOffice 24.8 or later | `sudo apt-get install -y -t noble-backports libreoffice-calc-nogui` on Ubuntu 24.04, whose own LibreOffice is 24.2 and lacks `LET`; `soffice` in `PATH` | The 175 cases of `validation/gnumeric` that Gnumeric cannot judge |
| R | `r-base-core`, `r-base-dev` and the packages [validation/r/README.md](validation/r/README.md) lists | `validation/r` |
| Python 3.12 | `pip install -r validation/games/requirements.txt` | `validation/games`; `validation/sweep` needs Python alone |

### The first build and the checks

```bash
git clone https://github.com/mollendorff-ai/forge
cd forge
cargo build --bin forge
make check                # format, pedantic lint, unit tests, Markdown lint
```

`make check` is the quick check while you work. Its last step, `make validate-docs`, runs `markdownlint-cli2` on the Markdown files your branch changed since it left `origin/main` (run `git fetch` first; not CHANGELOG.md, which every change touches and which has hundreds of old errors), so a page you write or change must lint clean, and a clone with no changes passes. `make validate-docs-all` lints every page; it fails on pages that were written before the lint existed, and CI runs neither.

```bash
make test                       # every test of the main crate, and validation/repo
make test-repo                  # only validation/repo: the docs, counts and rules against the binary
RUSTUP_TOOLCHAIN=1.100.0 make coverage   # 100%, measured with CI's toolchain
```

`make test-repo` fails when a page, a count or a rule no longer matches the code. The other crates each have their own tests and set-up: [validation/gnumeric](validation/gnumeric/README.md), [validation/r](validation/r/README.md), [validation/games](validation/games/README.md), [validation/sweep](validation/sweep/README.md), [validation/mermaid](validation/mermaid/README.md), [forge-wasm](forge-wasm/README.md) and [studio](studio/README.md).

## Development

```bash
make check          # Format + lint + unit tests, then Markdown lint of the pages you changed
make test           # Run all tests
make lint           # Clippy with pedantic lints
make format         # Format the code
```

## What a change needs

- **A changelog entry**, as a new file in `changelog.d/` ([how](changelog.d/README.md)), saying in plain words what a user saw and what changed. `CHANGELOG.md` itself changes only when a release is made.
- **A test that fails without the change** for every bug fix: a wrong answer, an error, a crash or a wrong message. The test reproduces the bug, fails on the commit before the fix and passes after it.
- **An example with outside numbers** for a new or changed command: a model in `examples/` whose results come from a calculation made outside Forge (by hand, a spreadsheet, R, Python), asserted in a test. `src/cli/commands/examples/` (its `bagel_example` tests) shows how. Using the bagel models found more bugs in one evening than the unit tests had.
- **Green CI**, including the Validation workflow, which runs the suites in `validation/` (forge-e2e-gnumeric, forge-e2e-r, the generated cases of `validation/sweep` and the games of `validation/games`; ADR-057) against the build of the same commit. New cases go in the same pull request. A pull request that changes or removes an existing expected value there says `changes expected values` in its title and, in its description, where each new value comes from (Excel, Gnumeric, R; the version and the date); the Expected values check fails it otherwise.
- **Docs that stay true**: update what the change makes wrong; a page about a design that no longer exists gets a dated note at the top.
- **No skipping the rules** in AGENTS.md: 100% coverage, pedantic lints and formatting, with no `allow`, no coverage-only code, no ignored test and no formatting skip; tests inline, in the file they test; no file over 1,500 lines. `validation/repo/src/no_skipping.rs` counts what is still left from before the rules, and fails if a change adds to it.
- **Measure coverage with Rust 1.100 or later**, the toolchain CI's stable is: `RUSTUP_TOOLCHAIN=1.100.0 make coverage`. `scripts/coverage.sh` refuses an older `rustc`, which counts fewer regions inside macros and reads 100% where CI does not.

The pull request template has these as a checklist. `.asimov/project.yaml` lists the conventions; `validation/repo/src/conventions.rs` checks the ones a test can.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as MIT OR Apache-2.0,
without any additional terms or conditions.
