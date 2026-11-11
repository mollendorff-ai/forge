# Validation

How Forge's results are checked, what the checks cover, and how to run each one yourself. Numbers are from 2026-11-06 on Forge main.

Forge is a beta and an R&D prototype. These checks reduce the chance of a wrong answer; they do not rule it out. Check results that matter independently.

## Summary

| Check | Compares Forge with | Result |
| --- | --- | --- |
| Unit tests (this repo) | Expected values written in the tests | 1,695 tests and 2 doctests pass |
| [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) | Gnumeric, a separate spreadsheet engine, and LibreOffice for 70 cases Gnumeric cannot read | 1,445 formula cases pass, 1 skipped |
| [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r), formulas | R, with Excel rules where R differs | 119 math, date and financial checks pass |
| forge-e2e-r, analytics | R packages `boot`, `stats`, `data.tree`, `derivmkts`, `bnlearn` and base R | 70 of 77 specs pass against R in the Validation workflow; 7 skipped |
| Real Excel (by hand, once) | Microsoft Excel for iPhone, recalculating a workbook written by `forge export` | 18 of 18 financial and date results match (2026-11-06) |

The unit tests run on every push. The two validation suites run in the **Validation** workflow, weekly and on every pull request that changes `src/` (Actions tab, also runnable on demand) and can be run by hand with the commands below.

## 1. Unit tests

```bash
cargo test --release
```

Includes Black-Scholes and binomial option prices checked against QuantLib reference values. CI runs the tests on stable and beta Rust and strict clippy (`-D warnings`) on every push; a weekly job also runs `cargo audit` and the tests on the latest stable Rust.

## 2. Formulas against Gnumeric

Each case is a Forge model with an expected value. A case passes only if both agree with it: `forge calculate`, and Gnumeric's recalculation of the file written by `forge export`. A case whose `libreoffice` key says why Gnumeric cannot read it (Gnumeric 1.12 has no `LET`, `UNIQUE` or `XLOOKUP`, gives #VALUE! for `DATEDIF` read from an xlsx file and for `ISFORMULA` of a literal, and differs from Excel on a comparison at 15 digits and on two financial results) is recalculated by LibreOffice instead, run with a profile that recalculates every formula on load: the workbook caches Forge's results, which LibreOffice would otherwise show.

```bash
git clone https://github.com/mollendorff-ai/forge-e2e-gnumeric
cd forge-e2e-gnumeric
FORGE_BIN=/path/to/forge cargo run --release
```

Needs `ssconvert` (package `gnumeric`) and LibreOffice 24.8 or later (`soffice`; on Ubuntu 24.04 from noble-backports). Result: 1,445 passed, 0 failed, 1 skipped; 70 of them recalculated by LibreOffice. Every case is also checked after `forge export` and `forge import`, so a formula that does not survive the round trip fails. Four cases are checked against Forge only, each with a written reason: `PRICEDISC` and `ACCRINT` results where Gnumeric 1.12 and LibreOffice 26.2 both differ from Excel, which was checked by hand (section 5). With every cached scalar result written 1,000 too high, the 144 cases of two spec files (27 of them in LibreOffice) still all passed, so both engines recalculate rather than read the cache.

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

## 5. Real Excel, by hand

Done once, on 2026-11-06, with Forge `fix/day-count-and-export` (after v10.0.0-beta.11), in the Excel app for iPhone. The workbook was written by `forge export` with the saved results removed, so a viewer that does not calculate would show empty cells; Excel's XIRR differs from Forge's in the ninth decimal, which shows Excel calculated it. A summary sheet compared each result with Forge's (relative tolerance 1e-7).

All 18 matched: XIRR, XNPV, two RATE cases, IRR with a negative rate, PMT, FV and IPMT with payments at the start of the period (including period 1, where Excel gives 0), NPER, PRICEDISC with bases 0, 1 and 4, YIELDDISC with basis 1, ACCRINT with bases 0 and 1, and YEARFRAC with bases 0 and 1. Six of these are cases where Gnumeric gives a different answer; Excel agreed with Forge on all six, and the Gnumeric suite now records them with that reason.

This covers one workbook and is not repeated automatically. An earlier check of the README example, on 2026-11-06, was made in a file viewer, not in Excel, and does not count.
