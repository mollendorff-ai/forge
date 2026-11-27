# Working on Forge in a session

How an agent's session works on Forge: where it starts, how it commits, what CI will check, how a pull request lands, and what it leaves the next session. The rules are in [AGENTS.md](../AGENTS.md), and which lane does what is there too. This page is what the handoffs between sessions used to repeat. A lesson that holds for every session goes here, in the pull request of the session that learned it, and not in a handoff.

## Starting

1. Your lane is the first line of your handoff. Read `AGENTS.md`, [docs/README.md](README.md) (the documentation's index) and your lane's roadmap, `.asimov/roadmap/<lane>.yaml`.
2. Start your branch from the newest `main`, which other lanes merge into several times an hour: `git fetch origin main && git checkout -B <your branch> origin/main`.
3. Read `git log origin/main --oneline -15` and your roadmap on `main` before you start an item: an item another session finished has moved to `done:`.

## Committing

- The pre-commit hook (`.cargo-husky/hooks/pre-commit`) runs `cargo fmt --all -- --check`, `make lint`, `cargo test --all-features`, builds `forge` and runs `validation/repo` against it: 4 to 8 minutes. Run `git commit` in the background with its output in a log, and wait with `until ! pgrep -f hooks/pre-commit >/dev/null; do sleep 10; done`.
- In a fresh clone the hook may not be installed yet: cargo-husky writes `.git/hooks/pre-commit` when the main crate's tests are first built. Check `ls .git/hooks/pre-commit` before your first commit; without it `git commit` checks nothing, and a lint such as clippy's `too_many_lines` reaches CI. Run `cargo test` once, or the hook's steps yourself.
- Then check that the commit is there (`git log -1`). A failing hook leaves the change staged, and the shell that ran it may still have exited 0. The hook's output is short: `cargo test -p mollendorff-forge-repo` shows which check failed.
- A merge with no conflict runs no hook. Run `bash .git/hooks/pre-commit` yourself after it.
- Never run two cargo jobs at once (the hook and a coverage run): the disk fills. When it is full, delete `target/llvm-cov-target`, `target/release` and `target/wasm32-unknown-unknown`, or all of `target/`.
- The hook does not run the Gnumeric suite. After changing a function, run `cargo test -q -p mollendorff-forge-e2e-gnumeric --lib` before you push.
- `cargo test` takes one filter before `--`: `cargo test -p mollendorff-forge-repo -- a b c`.
- Use `git grep`, not `grep -r`, which reads `target/`.

## What CI checks, and how to run it first

- **Clippy, four ways**, all with `-D warnings`: `cargo clippy --locked --all-targets`; the same in `forge-wasm/`; `cargo +1.100.0 clippy --locked --all-targets -p mollendorff-forge` (with `CARGO_TARGET_DIR=target/clippy100`); and `cargo clippy --lib --no-default-features`. The newest Rust has lints the oldest supported one (`rust-version` in `Cargo.toml`) does not, such as `suboptimal_flops` and `too_long_first_doc_paragraph`, and the other way round. Pedantic wants `to_bits()` to compare floats, a short first paragraph in a doc comment, and `map_or_else` for `unwrap_or(f())`.
- **Coverage, as CI measures it**: `RUSTUP_TOOLCHAIN=1.100.0 scripts/coverage.sh --json --output-path <scratch>/cov.json`, then `python3 scripts/coverage_misses.py <scratch>/cov.json`, which prints nothing at 100%. It takes about 6 minutes; run it in the background. The script refuses an older Rust. A fresh cloud container may lack both tools: `rustup toolchain install 1.100.0 --component llvm-tools-preview --profile minimal`, then `cargo install --locked cargo-llvm-cov` (about 5 minutes, once).
- **Missed regions** are usually one of these. A `?` that can never fail is a region nothing reaches: write it so it cannot fail, or test the failing path. Each closure is a region, so `unwrap_or_else(|e| panic!(...))` in a test is a miss: `assert!(r.is_ok())`, then `unwrap_or_default()`. An `if let` with no `else` in a fixture leaves its else uncovered. Generic code is counted once for each type it is used with.
- **Markdown**: `make validate-docs` lints the Markdown your branch changed since it left `origin/main`, every line of each file it touches. It needs `markdownlint-cli2` (`npm install -g markdownlint-cli2@0.24.2`).
- **Pages that are run**: `validation/repo` runs every `$ forge` block of `docs/guide/` in a copy of `examples/`, and a number in a `<details>` code span must be printed by a command on the same page. Only "double quotes" join words into one argument. A comment in `examples/*.yaml` does not say `examples/`.
- **Counts**: the number of Gnumeric cases written in the docs must be exact. The docs give no count of unit tests (roadmap E-DOC-COUNTS).
- **`docs/cli/README.md` is `forge --help`**: rebuild it with `cp target/debug/forge target/release/forge && make docs-cli`.
- **Forge Studio**: `cargo build -p forge-wasm --profile wasm --target wasm32-unknown-unknown`, `wasm-bindgen` (its version is the one in `Cargo.lock`) with `--target nodejs` and `--target web`, `node studio/build.mjs forge-wasm/pkg-web studio-site`, `npm ci` in `studio/`, then each `node tests/*.mjs` line of `ci.yml`. `studio/app.js` is at the 1,500-line limit: new interface code goes in a module of its own.

## Pull requests

- Open one, and merge one, only when the maintainer asks.
- Bring `main` in with a merge (`git fetch origin main && git merge origin/main`), never a rebase, and never force-push.
- Two pull requests that each move items to `done:` in the same roadmap conflict once the first merges, and other lanes append to your backlog meanwhile. Resolve by keeping every item from both sides, each in the list it now belongs to, then check that the roadmap parses and `cargo test -p mollendorff-forge-repo -- lanes` passes.
- `main` moves while CI runs: check `git rev-list --count HEAD..origin/main` before merging, and merge with `expectedHeadSha` and the merge method.
- A completed check suite is not a green one: read each check run's conclusion. Two skipped jobs on a pull request are normal. The maintainer may merge before CI ends: check `main`'s own run on the merge commit.
- Subscribe to a pull request's activity once `create_pull_request` has returned its number, then end the turn rather than polling.
- A Dependabot bump of a crate that `Cargo.toml` also names in its wasm32-only section leaves that entry behind, which gives two versions of the crate: bump both.

## Tools and sources

- Gnumeric (`ssconvert`) and LibreOffice are installed in the cloud image; Excel answers come from the maintainer's Excel for iPhone, as screenshots. To run one Gnumeric spec, copy it to a folder of its own and run `FORGE_BIN=$PWD/target/debug/forge cargo run -q -p mollendorff-forge-e2e-gnumeric -- --tests <folder>`.
- A fresh-agent trial gets only the file under test, an empty folder and its own `HOME`.
- GitHub works through its MCP tools; there is no `gh`. `raw.githubusercontent.com` and a `git clone` of a public repository work where some sites are blocked.
- Blocked from the cloud container: `api.osv.dev` and scorecard.dev's API (read an advisory in a `git clone --depth 1` of `rustsec/advisory-db`; ask the maintainer for what the Scorecard viewer shows), and Sigstore's TUF CDN, so `cosign verify-blob` needs `--trusted-root` with `trusted_root.json` from `sigstore/root-signing`'s `targets/` on `raw.githubusercontent.com`. The crates.io API works (`/api/v1/crates/<name>/<version>/dependencies`), with a `User-Agent`.
- Which crates come in two versions: `cargo tree -d --target all`. Clippy's `multiple_crate_versions` reads every target, so a duplicate that only `wasm32` brings counts; `cargo tree -d` alone shows the host's. The wasm32 target is not installed in a fresh container: `rustup target add wasm32-unknown-unknown`.

## Handing off

When a session grows long, the `handoff` skill writes the prompt for the next one in the same lane. The prompt starts with the lane and carries only what is not on `main`: the branch, the open pull request and its CI, work half done, and what was found but not yet written down. What is next is the lane's roadmap; how to work is this page.
