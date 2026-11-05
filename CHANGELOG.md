# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
