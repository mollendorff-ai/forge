# R validators

Six R scripts, run by `run_all.sh`. Three send formulas through `forge calculate` and compare each result with R; three check R's own output and do not call Forge. The Forge side of the bootstrap and the distributions is compared with R by the analytics runner (`cargo run --release -- --all`, [../../README.md](../../README.md)).

| Script | Calls Forge | What it checks |
| --- | --- | --- |
| `math_validator.R` | yes | `ROUND`, `FLOOR` and `CEILING` against R's base math, to 1e-10 |
| `date_validator.R` | yes | `DATE`, `YEAR`, `MONTH`, `DAY`, `WORKDAY` and `NETWORKDAYS` against R's base dates, exactly |
| `financial_validator.R` | yes | Forge's FP&A functions (`VARIANCE`, `VARIANCE_PCT`, `VARIANCE_STATUS`, `BREAKEVEN_UNITS`, `BREAKEVEN_REVENUE`) against the same arithmetic in R, to 1e-10 |
| `bootstrap_validator.R` | no | R's `boot` package: bootstrap means, medians and intervals |
| `distribution_validator.R` | no | R's `stats` package: values of the normal, t and chi-square distributions, for comparison by hand |
| `monte_carlo_validator.R` | no | R's samplers (normal, uniform, lognormal, triangular, PERT, exponential): summary statistics of 100,000 draws |

Where R and Excel differ, the reference is Excel: `ROUND` rounds half away from zero and `NETWORKDAYS` counts both end dates.

## Running them

From `validation/r`, with a built Forge:

```bash
FORGE_BIN=../../target/release/forge bash validators/r/run_all.sh   # all six
FORGE_BIN=../../target/release/forge Rscript validators/r/math_validator.R
```

The scripts that call Forge stop with exit code 2 when `FORGE_BIN` is not set and `forge` is not on the PATH. They need R with `jsonlite` and `boot`; `install_deps.R` installs the packages the validators use.

## In CI

The `r` job of `.github/workflows/validation.yml` ("Functions and analytics vs R") installs R from Ubuntu's packages (`r-base-core`, `r-cran-jsonlite`, `r-cran-boot`, `r-cran-mvtnorm`), builds Forge, and runs `run_all.sh` in its "Math, date and financial functions" step, then the analytics specs. VALIDATION.md reports the checks that call Forge.

Why R is a reference, and how the tolerances were chosen: [ADR-037](../../../../docs/architecture/ADR-037-EXTERNAL-VALIDATION-ENGINES.md), [ADR-039](../../../../docs/architecture/ADR-039-STATISTICAL-VALIDATION.md), [ADR-046](../../../../docs/architecture/ADR-046-R-VALIDATED-ANALYTICS.md).

## License

Dual-licensed under [MIT](../../../../LICENSE-MIT) or [Apache-2.0](../../../../LICENSE-APACHE), at your option.
