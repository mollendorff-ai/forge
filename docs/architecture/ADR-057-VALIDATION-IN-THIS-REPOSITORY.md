# ADR-057: The Validation Suites Live in This Repository

**Status:** Accepted (2026-11-12); built, the old repositories not yet archived (roadmap T1-VALIDATION-IN-REPO)
**Date:** 2026-11-12
**Decided by:** the maintainer (bring the two end-to-end suites into Forge and archive their repositories); the design is Claude's, in a session configured for Fable 5.2
**Supersedes:** the rest of ADR-027 (the suites in repositories of their own)

---

## Context

Forge's results are checked against engines Forge does not contain, by two suites that live in repositories of their own:

| Suite | Compares with | Size (tracked files) | Rust | Commits |
| --- | --- | --- | --- | --- |
| forge-e2e-gnumeric | Gnumeric, and LibreOffice for what Gnumeric cannot read | 71 files, 676 KB | 1,671 lines | 53 |
| forge-e2e-r | R (`boot`, `stats`, `data.tree`, `derivmkts`, `bnlearn`) | 47 files, 528 KB | 2,892 lines | 41 |

Each is a small Rust program that runs the built `forge` binary (`FORGE_BIN`) on models with expected values, and the other engine on the same inputs. Neither depends on the Forge crate. Forge's Validation workflow builds Forge, clones the two suites and runs them, weekly and on every pull request that changes `src/`.

ADR-027 moved the tests out in 2026 for a reason that no longer holds: Forge was 636,000 lines then, too large to read in one session. It is 73,000 lines of source today, the integration tests came back into `tests/` long ago, and what stayed outside is 4,563 lines of Rust and the spec files.

Living in three repositories costs something every time the engine and a suite change together:

- **Two pull requests for one change.** PR #161 (2026-11-12) changed how `forge sensitivity` prints. The R suite read that output, so the change needed forge-e2e-r#13 as well, written to work with both the old and the new Forge so that either could merge first. Most of that pull request exists only because the two could not merge together.
- **A convention to test them together.** The Validation workflow looks for a suite branch with the same name as the Forge branch. It works, and nobody would guess it.
- **`main` can break from outside.** The workflow clones each suite's `main`, so a merge there changes what Forge's CI does without a commit in Forge.
- **forge-e2e-r has no CI of its own.** A pull request there shows no checks; its only test is Forge's workflow, run from the other repository.
- **Access.** A session attached to Forge cannot push to the suites until it asks (2026-11-12, mid-task).

What the separation was thought to give is independence. The independence that matters is not where the files are. It is that the suites run the binary a user runs, and compare it with engines Forge shares no code with. Both stay true in one repository.

## Decision

1. **The two suites move into this repository**, as `validation/gnumeric/` and `validation/r/`, with their history (`git subtree add`, not squashed, so each suite's commits are in Forge's history and `git log` reaches them).

2. **They stay separate programs that run the binary.** Each is a workspace member (as `forge-wasm` is), not published, and left out of the `mollendorff-forge` package. Neither gains a dependency on the Forge crate: they keep running `forge` through `FORGE_BIN`, which now defaults to `target/release/forge` of the same checkout. The binaries keep their names, `forge-e2e-gnumeric` and `forge-e2e-r`, which is what the docs and the changelog call the suites.

3. **One lock file, one build directory, the workspace's lint.** The suites are linted as `forge-wasm` is (`cargo clippy -p <suite> --all-targets -- -D warnings`) and their own unit tests (20 today) run in CI. They are not part of `cargo test` at the root or of the pre-commit hook, which need neither Gnumeric nor R.

4. **The Validation workflow runs the suites of the commit it builds.** No clone, no branch of the same name. It runs on pull requests that change `src/`, `Cargo.toml`, the workflow or `validation/`, on demand, and weekly: the weekly run is what notices a new Gnumeric, LibreOffice or R.

5. **An expected value does not change quietly.** One pull request can now change the engine and the value it is checked against, which three repositories made harder by accident. So:
   - a pull request that changes or removes an existing expected value under `validation/` says `changes expected values` in its title, and its description says where each new value comes from (Excel, Gnumeric, R; the version and the date);
   - a CI check fails the pull request when its diff changes or removes such a line and the title does not say so. Adding cases needs nothing.

6. **The old repositories are archived, not deleted**, after the first Validation run on `main` passes from inside this repository. 35 files here link to them, as do the changelog and earlier ADRs, and an archived repository keeps those links alive. Each gets a first line in its README that says where the suite went, then is made read-only.

7. **What does not come along:** the suites' own `.asimov/`, `.claude/`, licence and security files (Forge has its own), their version numbers, and their changelogs, which stop where they are in the archived repositories. A change to a suite is an entry in Forge's changelog from then on.

8. **The living documents say where the suites are** (README, VALIDATION.md, CONTRIBUTING, FEATURES, `tests/README.md`, `.asimov/project.yaml`, comments in the source). The ADRs and changelog entries that describe the past keep their text; ADR-007, ADR-027 and ADR-046 get one line that points here.

## Consequences

**What gets better**

- A change that touches the engine and a suite is one pull request, tested as it will be merged.
- Nothing outside Forge can change what Forge's CI does.
- forge-e2e-r gets CI for the first time: lint and its unit tests on every pull request.
- The same session, the same access and the same conventions (changelog, ADRs, lint) cover everything.
- One fewer place for a contributor to find: `validation/` is next to what it validates.

**What it costs**

- The repository grows by about 1.2 MB and 118 files, plus 94 commits of history. The published crate does not grow.
- The workspace has two more members, so a full `cargo build --workspace` builds them. They share their dependencies with Forge (`clap`, `serde`, `calamine`, `tempfile`, `anyhow`); only `csv` is new to the lock file.
- CI time does not change in a way that matters: Gnumeric, LibreOffice and R are already installed on every pull request that changes `src/`. The clone goes away; the lint of two small crates is added.
- Rule 5 is a new thing to know. It replaces a protection that was never designed, only a side effect.

**What does not change**

- What the suites check, their cases, and their results. Moving them must give the same counts before and after: 1,445 passed and 1 skipped against Gnumeric; 136 function checks and 70 of 77 analytics specs against R in CI.
- The binary, the package, the release, Homebrew, Studio.

**Not known when this was written**

- Whether both suites pass `clippy -D warnings` as workspace members without changes. If not, they are fixed in the move, not exempted.
- Whether either suite was ever published on crates.io under its package name. If one was, that version stays there and is not yanked; nothing depends on it.
- Whether the session that does the move can archive a repository through the API. If it cannot, archiving is the one step left to the maintainer.

**Found in the move (2026-11-12)**

- Both suites pass `cargo clippy --all-targets -- -D warnings` as workspace members, with their own pedantic lint settings, unchanged. They were reformatted to Forge's `rustfmt.toml` (11 files, layout only).
- Neither was published: crates.io answers 404 for `mollendorff-forge-e2e-gnumeric` and `mollendorff-forge-e2e-r` (and 200 for `mollendorff-forge`).
- The Validation workflow on `main` ran the suites from `validation/` on 2026-11-12 (after #165, which also runs it on merges to `main`), green: 1,445 passed and 1 skipped against Gnumeric, 136 function checks and 70 of 77 analytics specs against R.
- A session cannot archive a repository: GitHub refuses repository settings writes, and file writes through the API, through the session's proxy. The README notes are pull requests in the old repositories; archiving is the maintainer's.
- forge-e2e-r had `cargo-husky` as a dev-dependency, which installs the git hooks in its `.cargo-husky/` when its tests are built. Forge has a pre-commit hook of its own, so both are removed. The suites' release profiles are dropped too, since a profile counts only at the workspace root.

## Alternatives considered

- **Leave it as it is.** It works, and #161 showed the price: a second pull request and compatibility code for one change of output. The reason for the split is gone.
- **Git submodules.** The suites stay in their repositories and Forge pins a commit of each. This fixes "main can break from outside" and nothing else: a change still needs two pull requests, plus a third step to move the pin.
- **Copy the files without their history.** Simpler to do, and it throws away why each expected value is what it is: 11 real-options prices and 2 posteriors were corrected in forge-e2e-r with the reasons in the commits. The history is 94 commits; keeping it is cheap.
- **Directories outside the workspace, each with its own lock file.** They would build exactly as today and never slow a workspace build. Rejected: two more lock files to keep current, a second and third build directory in CI, and a lint standard that is nobody's.
- **Fold the cases into `tests/` and call Gnumeric and R from `cargo test`.** Then `cargo test` needs both installed, on every machine and in the pre-commit hook. The suites are slow and need outside programs; they are a different kind of test and stay a different command.

## How it is built

One pull request for the move (subtree, workspace, workflow, rule 5's check, documents), which must show the same counts as the last run from the separate repositories. Then, once `main` is green: the line in each old README, and the two repositories archived.
