# Validation

How Forge's results are checked, what the checks cover, and how to run each one yourself. Numbers are from 2026-11-06 on Forge main.

Forge is a beta and an R&D prototype. These checks reduce the chance of a wrong answer; they do not rule it out. Check results that matter independently.

## Summary

| Check | Compares Forge with | Result |
| --- | --- | --- |
| Unit tests (this repo) | Expected values written in the tests | 1,471 unit tests and 2 doctests pass |
| [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) | Gnumeric, a separate spreadsheet engine | 1,312 formula cases pass, 1 skipped |
| [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r), formulas | R, with Excel rules where R differs | 119 math, date and financial checks pass |
| forge-e2e-r, analytics | R packages `boot`, `stats`, `data.tree`, `derivmkts`, `bnlearn` and base R | 70 of 77 specs pass against R in the Validation workflow; 7 skipped |

The unit tests run on every push. The two validation suites run in the **Validation** workflow, weekly and on every pull request that changes `src/` (Actions tab, also runnable on demand) and can be run by hand with the commands below.

## 1. Unit tests

```bash
cargo test --release
```

Includes Black-Scholes and binomial option prices checked against QuantLib reference values. CI runs the tests on stable and beta Rust and strict clippy (`-D warnings`) on every push; a weekly job also runs `cargo audit` and the tests on the latest stable Rust.

## 2. Formulas against Gnumeric

Each case is a Forge model with an expected value. A case passes only if both agree with it: `forge calculate`, and Gnumeric's recalculation of the file written by `forge export`.

```bash
git clone https://github.com/mollendorff-ai/forge-e2e-gnumeric
cd forge-e2e-gnumeric
FORGE_BIN=/path/to/forge cargo run --release
```

Needs `ssconvert` (package `gnumeric`). Result: 1,312 passed, 0 failed, 1 skipped. Of the passed cases, 34 are checked against Forge only, each with a written reason (Gnumeric 1.12 cannot read the formula or has no such function, for example `XLOOKUP`).

## 3. Math, date and financial functions against R

The R validators send each formula through `forge calculate` and compare the answer with R. Where R and Excel differ, Excel is the reference (`ROUND` rounds half away from zero; `NETWORKDAYS` counts both end dates).

```bash
git clone https://github.com/mollendorff-ai/forge-e2e-r
cd forge-e2e-r
FORGE_BIN=/path/to/forge bash validators/r/run_all.sh
```

Needs R with `jsonlite`. Result: 119 checks pass.

## 4. Analytics against R

Monte Carlo, bootstrap, tornado, sensitivity, decision trees, real options and Bayesian networks: the runner builds a Forge model from each spec, runs the Forge command and compares the output with the matching R validator.

```bash
cd forge-e2e-r
cargo run --release -- --all --binary /path/to/forge
```

| Category | Specs | Pass | Not compared, and why |
| --- | --- | --- | --- |
| Monte Carlo | 12 | 10 | 2 use the exponential distribution, which Forge does not have |
| Bootstrap | 12 | 12 | Percentile, basic and BCa intervals |
| Tornado | 8 | 7 | 1 has an input whose low equals its high, which Forge rejects |
| Sensitivity | 10 | 6 | 3 are elasticity (no Forge command); 1 is a single point |
| Decision trees | 10 | 10 | |
| Real options | 15 | 15 | Price only; Forge's command does not print Greeks |
| Bayesian networks | 10 | 10 | |

Running without `--expected-only` on a machine without `data.tree`, `derivmkts` or `bnlearn` reports those specs as errors, not passes (`derivmkts` is optional). `--expected-only` compares decision trees, real options and Bayesian networks with the expected values in the spec and does not run R. Bootstrap, tornado and sensitivity need a Forge build with machine-readable `--output` (included from PR #27).

## Known limits

- The Bayesian comparison uses `bnlearn` 4.9.4 (the newest release that installs on R 4.3, which GitHub's Ubuntu 24.04 runner has) and sampling with a fixed seed, so it agrees with Forge within 2 to 5 percentage points, not exactly.
- When the real options and Bayesian runners were added, 11 expected prices and 2 expected posteriors in the specs turned out to be wrong and were recomputed; Forge matched an independent calculation in every case.
- The Gnumeric and R suites run weekly and on pull requests that change `src/`, not on every push to a branch.
- The expected values in a spec were written by the author or by an AI assistant. Where R or Gnumeric disagreed with them, the disagreement was investigated, but a value on which both Forge and its expected value are wrong in the same way would pass.
- Real-options switch and compound valuations are rough approximations.
- `cargo audit` reports no vulnerabilities and no warnings.
