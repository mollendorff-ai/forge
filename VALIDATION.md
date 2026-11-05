# Validation

How Forge's results are checked, what the checks cover, and how to run each one yourself. Numbers are from 2026-11-05 on Forge main after PR #28.

Forge is a beta and an R&D prototype. These checks reduce the chance of a wrong answer; they do not rule it out. Check results that matter independently.

## Summary

| Check | Compares Forge with | Result |
| --- | --- | --- |
| Unit tests (this repo) | Expected values written in the tests | 1,457 unit tests and 2 doctests pass |
| [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) | Gnumeric, a separate spreadsheet engine | 1,300 formula cases pass, 1 skipped |
| [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r), formulas | R, with Excel rules where R differs | 119 math, date and financial checks pass |
| forge-e2e-r, analytics | R packages `boot`, `stats` and base R | 30 of 77 specs pass; 10 more pass against expected values only; 37 skipped |

Only the unit tests run on every push. The two validation suites run in the weekly **Validation** workflow (Actions tab, also runnable on demand) and can be run by hand with the commands below.

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

Needs `ssconvert` (package `gnumeric`). Result: 1,300 passed, 0 failed, 1 skipped. Of the passed cases, 34 are checked against Forge only, each with a written reason (Gnumeric 1.12 cannot read the formula or has no such function, for example `XLOOKUP`).

## 3. Math, date and financial functions against R

The R validators send each formula through `forge calculate` and compare the answer with R. Where R and Excel differ, Excel is the reference (`ROUND` rounds half away from zero; `NETWORKDAYS` counts both end dates).

```bash
git clone https://github.com/mollendorff-ai/forge-e2e-r
cd forge-e2e-r
FORGE_BIN=/path/to/forge bash validators/r/run_all.sh
```

Needs R with `jsonlite`. Result: 119 checks pass.

## 4. Analytics against R

Monte Carlo, bootstrap, tornado, sensitivity and decision trees: the runner builds a Forge model from each spec, runs the Forge command and compares the output with the matching R validator.

```bash
cd forge-e2e-r
cargo run --release -- --all --binary /path/to/forge
```

| Category | Specs | Pass | Not compared, and why |
| --- | --- | --- | --- |
| Monte Carlo | 12 | 10 | 2 use the exponential distribution, which Forge does not have |
| Bootstrap | 12 | 7 | 5 use BCa or basic intervals; Forge computes percentile intervals only |
| Tornado | 8 | 7 | 1 has an input whose low equals its high, which Forge rejects |
| Sensitivity | 10 | 6 | 3 are elasticity (no Forge command); 1 is a single point |
| Decision trees | 10 | 10 | Passes only against the spec's expected values (`--expected-only`); the R side needs the `data.tree` package and has not been run |
| Real options | 15 | 0 | No runner yet |
| Bayesian networks | 10 | 0 | No runner yet |

Running without `--expected-only` on a machine without `data.tree` reports the decision tree specs as errors, not passes. Bootstrap, tornado and sensitivity need a Forge build with machine-readable `--output` (included from PR #27).

## Known limits

- The R side of decision trees, real options and Bayesian networks has not been run by the maintainers; the R packages `data.tree`, `derivmkts` and `bnlearn` were not available in the environment used.
- The Gnumeric and R suites run weekly, not on every push.
- The expected values in a spec were written by the author or by an AI assistant. Where R or Gnumeric disagreed with them, the disagreement was investigated, but a value on which both Forge and its expected value are wrong in the same way would pass.
- Bayesian networks support one parent per node. Real-options switch and compound valuations are rough approximations.
- `unsafe-libyaml`, a dependency of the YAML parser, is unmaintained. `cargo audit` reports 0 vulnerabilities and this one warning.
