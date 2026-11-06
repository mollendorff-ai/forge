# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Runners for real options (15 specs, price only) and Bayesian networks (7 of 10 specs; Forge supports one parent per node). `--expected-only` covers them too. The Gnumeric and R CI install `derivmkts` and `bnlearn` for them.

### Fixed (specs and validators)

- 11 expected prices in `real_options.yaml` and 2 expected posteriors in `bayesian.yaml` were wrong; Forge matched an independent calculation. They are recomputed.
- `bayesian_validator.R` read probability tables column by column, but the specs list them row by row, so `bnlearn` would have rejected them. It now reads them row by row. The `credit_risk` table was listed the other way round and is corrected. Sampling uses a fixed seed.
- `real_options_validator.R` no longer fails to start when `derivmkts` is missing.

### Fixed

- **The math, date and financial validators never ran Forge.** Each test compared a hard-coded number (`forge_result <- 3`) with R, so a pass said nothing about Forge. They now run every formula through `forge calculate` (`validators/r/forge_helper.R`; set `FORGE_BIN`) and exit 2 if Forge cannot be found. 119 tests.
- The leap-year tests passed whenever R reported an invalid date (`|| r_result == TRUE`). They now check Forge's `DATE` for Feb 29.
- Two R references were not Excel's: `round()` rounds half to even (Excel rounds half away from zero), and the `NETWORKDAYS` reference excluded the start date. Both now follow Excel.
- Found by the change: Forge returned 0 for `NETWORKDAYS` with a start after the end (Excel: negative count). Fixed in forge#26; the reverse-range date test needs a Forge build with that fix.

### Fixed

- **A run where tests errored exited 0.** Tests that could not run (a missing R package, a Forge crash, unparseable output) were reported as errors but not counted, so `--all` printed `PASS 0 passed` and exited 0. Errors now count against the run, and a run that passed nothing exits 1.
- `validators/r/run_all.sh` stopped after the first passing validator: `((PASSED_VALIDATORS++))` returns status 1 when the counter is 0, and `set -e` exited. The counters now use `$((n + 1))`.

### Changed

- The summary says how many tests were skipped and not validated. The Rust runner only executes Monte Carlo tests; the other types in `tests/analytics` are skipped.

## [1.0.1] - 2026-01-24

### Changed

- **Rebranding: RoyalBit to Möllendorff AI**
  - Updated crate name from `royalbit-forge-e2e-r` to `mollendorff-forge-e2e-r`
  - **Why rebrand?** The "RoyalBit" name (company founded 2006) was hijacked by unrelated cryptocurrency scammers:
    - UK FCA issued official warning (Oct 2024) about "Royalbit Miners" - unauthorized firm
    - Multiple fraudulent domains: royalbit.ltd (trust score 38/100), royalbit.top, royal-bit.club
    - Classic HYIP Ponzi schemes offering impossible returns (155-580% in days)
    - Sources: [FCA Warning](https://www.fca.org.uk/news/warnings/royalbit-miners), [Scam Detector](https://www.scam-detector.com/validator/royalbit-ltd-review/)

## [1.0.0] - 2024-12-XX

Core R validation framework for forge analytics.

### Added

- Rust CLI test runner with colored output
- R validator integration via Rscript
- Monte Carlo validation (6 distributions: Normal, Uniform, Lognormal, Triangular, PERT, Exponential)
- Bootstrap confidence interval validation
- Bayesian network inference validation
- Decision tree valuation validation
- Real options pricing validation
- Tornado sensitivity analysis validation
- Scenario analysis validation
- Variance (budget vs actual) validation
- Breakeven analysis validation
- Statistical tolerance framework (mean 1%, std 5%, percentiles 2%)
- 12 R validator scripts
- 10 YAML test specification files

### Philosophy

**Forge is NEVER the authority.**
All tests validate forge against R, the gold-standard for statistical computing.
If forge disagrees with R, forge has a bug.
