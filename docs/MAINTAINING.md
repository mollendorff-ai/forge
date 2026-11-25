# Maintaining Forge

For the person who makes releases and merges pull requests. A contributor reads [CONTRIBUTING.md](../CONTRIBUTING.md); the design is in [docs/architecture/](architecture/README.md). This page says what the workflows do when a release is made, so that each step can be checked against them.

Every claim below is read from `.github/workflows/ci.yml` or `.github/workflows/release.yml`, and `validation/repo/src/roles.rs` fails when a job, a script, a secret or a file this page names is not there.

## What makes a release

A release is a version in `Cargo.toml` on `main` that has no tag yet. Nothing else starts one.

1. A pull request that changes the version is merged to `main`.
2. CI runs on that push. Its `Auto Release` job (`auto-release` in `ci.yml`) runs after the `test`, `lint`, `build` and `repo-checks` jobs pass. It reads `version` from `Cargo.toml` and creates the tag `v<version>` (an annotated tag, through the API if `git push` of the tag is refused). It then starts the Release workflow with `gh workflow run release.yml --ref v<version>`, because a tag pushed with `GITHUB_TOKEN` does not start workflows. If the tag exists but its GitHub release does not, and no Release run is queued or running, the next push to `main` starts it again.
3. The Release workflow (`release.yml`) runs the jobs below.

### The jobs of the Release workflow

| Job | What it does |
| --- | --- |
| `test` | The gate: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --verbose` and `cargo build --release`, on stable |
| `build` | Builds the binary for five targets (Linux x86_64 and ARM64 musl, macOS ARM and Intel, Windows x86_64) with `FORGE_RELEASE=1`, so the build carries no commit stamp, and uploads an archive for each: `.tar.gz`, `.zip` on Windows |
| `release` | Writes `checksums.txt` with `scripts/release-checksums.sh`, checks it with `sha256sum -c`, signs a build attestation for each archive (`actions/attest-build-provenance`), creates the GitHub release with generated notes, then downloads every asset and checks them again |
| `studio` | Starts the `pages.yml` workflow for this tag, which publishes Forge Studio at <https://forge.mollendorff.ai/> |
| `publish` | `cargo publish` to crates.io with the `CRATES_IO_TOKEN` secret; it does not fail the run when the version is already published (`continue-on-error`) |
| `homebrew` | Downloads the release's `checksums.txt`, writes `Formula/forge.rb` with `scripts/homebrew-formula.sh` and commits it to `main` |
| `install` | Calls `install.yml`: `brew install` and `cargo binstall` as a user runs them, each of which must print the released version |

`build`, `release`, `studio`, `publish` and `homebrew` each need the job before them (`test`, then `build`, then `release`; `studio`, `publish` and `homebrew` after `release`; `install` after `homebrew` and `publish`). A red `test` stops everything, so nothing is built from a commit that does not pass the gate.

## Making a release

Do these in one pull request, as the release commits before it did (`git log --grep "Release 10.0.0"`):

1. **Choose the version.** Forge is a release candidate (`10.0.0-rc.N`); the next is `rc.N+1`. [STABILITY.md](STABILITY.md) says what the stable release waits for: a candidate published for two weeks with no open report of a wrong answer. A wrong answer fixed in a release starts the two weeks again from that candidate.
2. **Change the version in the places that write it:** `version` in `Cargo.toml`, the `mollendorff-forge` entry of `Cargo.lock` (run `cargo build` and commit what it changes), the status line of `README.md` ("Forge is a release candidate (v10.0.0-rc.N)"), the first paragraph of `docs/STABILITY.md`, and the `Cargo.toml` example of `docs/EMBEDDING.md`. `Formula/forge.rb` is written by the `homebrew` job after the release; leave it.
3. **Date the changelog.** Under `## [Unreleased]` of `CHANGELOG.md`, add `## [10.0.0-rc.N] - YYYY-MM-DD` with a short paragraph, and leave `## [Unreleased]` empty above it.
4. **Merge only when CI is green on the pull request and on `main` after the merge**, including the Validation workflow (the Gnumeric, R and games suites). A release made from a red `main` publishes what the checks refused.
5. **Watch the run.** Open the Actions tab: `CI` on `main` (its `Auto Release` job), then `Release`, then `Pages` and `Install`. The release page must have five archives and `checksums.txt`.

## After a release

- `brew install mollendorff-ai/forge/forge` and `cargo binstall mollendorff-forge` print the new version (the `Install` workflow checks both).
- `gh attestation verify <archive> --repo mollendorff-ai/forge` verifies where an archive was built.
- The `crates.io` page for `mollendorff-forge` shows the version. `docs.rs` builds the crate's documentation from `src/lib.rs`.
- <https://forge.mollendorff.ai/> shows the release in its help links, which point at the tag.

## When a step fails

- **A job fails downloading an artifact ("No artifacts found").** This is the runner, not the change: re-run the failed jobs once, after the whole run has finished.
- **The tag exists and the release does not.** Push to `main` (or run `gh workflow run release.yml --ref v<version>`): `Auto Release` starts the Release workflow again when none is queued or running.
- **`publish` is red.** `crates.io` refuses a version published twice and the job tolerates that; any other failure is the token or the crate, and `cargo publish` can be run by hand with the token.
- **`homebrew` cannot push.** It retries three times against `main`'s newest commit; if `main` moved a lot, run the job again.

## Weekly and on demand

`.github/workflows/weekly.yml` runs on Mondays (and when `Cargo.lock` changes): `cargo audit` on the dependencies, and the tests, clippy and property tests (with the week's seed) on the toolchains `weekly.yml` lists, so a new Rust release or advisory that breaks the build shows without a code change. `.github/workflows/validation.yml` runs the suites against Gnumeric, LibreOffice, R and the games' reference implementations on Mondays, on a push or pull request that changes `src/`, `validation/` or `Cargo.toml`, and on demand (`workflow_dispatch`, with a seed). A failure there is a wrong answer or a changed engine, and [VALIDATION.md](../VALIDATION.md) says what each suite compares.

## Dependencies

Dependabot opens the pull requests (`.github/dependabot.yml`). A dependency that brings a second version of a crate is not merged by anyone but the maintainer: the duplicate is allowed in `clippy.toml` with its reason and counted in `validation/repo/src/no_skipping.rs` (AGENTS.md, "Libraries before code of our own"). The WebAssembly module of Forge Studio must stay under 3,000,000 bytes gzipped (ADR-070); the `forge-wasm` job of `ci.yml` prints its size.
