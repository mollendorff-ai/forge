# forge-e2e-r

R-validated E2E testing for [forge](https://github.com/mollendorff-ai/forge) analytics.

## Philosophy

**Forge is NEVER the authority.**

All tests validate forge against R, the gold-standard for statistical computing.
If forge disagrees with R, forge has a bug.

This is a child repository of [forge-e2e](https://github.com/mollendorff-ai/forge-e2e), the documentation hub that coordinates validation of forge against external reference implementations.

## What It Tests

| Category | Tests | R Packages |
|----------|-------|------------|
| Monte Carlo | 12 distribution tests | stats, mc2d |
| Bootstrap | Confidence intervals | boot |
| Bayesian Networks | Inference, belief propagation | bnlearn |
| Decision Trees | Expected monetary value | data.tree |
| Real Options | Black-Scholes, binomial | derivmkts |
| Tornado | Sensitivity analysis | base R |
| Scenarios | Probability-weighted | base R |
| Variance | Budget vs actual | base R |
| Breakeven | Unit economics | base R |

### What actually runs

`cargo run --release -- --all` runs the Monte Carlo specs (10) and the bootstrap specs (13 of 17) against R: 23 of the 77 specs in `tests/analytics`. Four bootstrap specs use BCa or basic intervals, which Forge does not compute, so they are skipped. The other 54 specs (tornado, sensitivity, decision trees, real options, Bayesian, scenarios, variance, breakeven) are reported as skipped; their runners are still to come. Bootstrap needs a Forge build that includes `--output` in JSON (forge PR #27).

`validators/r/math_validator.R`, `date_validator.R` and `financial_validator.R` run each formula through `forge calculate` and compare the result with R. They need `FORGE_BIN` set (or `forge` on the PATH) and stop with exit code 2 if Forge cannot be found. Where R and Excel differ, the reference is Excel: `ROUND` rounds half away from zero and `NETWORKDAYS` counts both end dates.

## Requirements

### Forge Binary

```bash
# Option 1: Set environment variable
export FORGE_BIN=/path/to/forge

# Option 2: Build forge in sibling directory (auto-detected)
cd ../forge && cargo build --release
```

### R Installation

```bash
# macOS
brew install r

# Ubuntu/Debian
apt install r-base

# Fedora/RHEL
dnf install R
```

### R Packages

```bash
# Core packages (required)
Rscript -e 'install.packages(c("jsonlite", "boot", "mc2d"))'

# Optional packages (for full test coverage)
Rscript -e 'install.packages(c("bnlearn", "data.tree", "derivmkts"))'

# Or use the installer script
Rscript validators/r/install_deps.R
```

## Usage

```bash
# Run all tests
FORGE_BIN=/path/to/forge cargo run --release -- --all

# Run specific test file
cargo run --release -- --tests tests/analytics/monte_carlo.yaml

# Run specific test directory
cargo run --release -- --tests tests/analytics --all

# With verbose output
cargo run --release -- --all --verbose
```

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                         Test Execution                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   1. Load YAML test spec                                        │
│      └── distribution, params, seed, iterations                 │
│                                                                 │
│   2. Run forge command                                          │
│      └── forge simulate → JSON output                           │
│                                                                 │
│   3. Run R validator                                            │
│      └── Rscript validator.R → R reference results              │
│                                                                 │
│   4. Compare statistics                                         │
│      ├── Mean: 1% tolerance                                     │
│      ├── Std: 5% tolerance                                      │
│      ├── Percentiles: 2% tolerance                              │
│      └── KS test: p > 0.05                                      │
│                                                                 │
│   5. Report Pass/Fail                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
forge-e2e-r/
├── src/
│   ├── main.rs           # CLI test runner
│   ├── lib.rs            # Library exports
│   ├── cli_runner.rs     # Forge binary orchestration
│   ├── r_validator.rs    # R process management
│   ├── types.rs          # Test specifications
│   └── stats.rs          # Statistical comparison
│
├── tests/analytics/      # YAML test specifications
│   ├── monte_carlo.yaml  # 12 distribution tests
│   ├── bootstrap.yaml    # Confidence intervals
│   ├── bayesian.yaml     # Bayesian networks
│   ├── decision_trees.yaml
│   ├── real_options.yaml
│   ├── tornado.yaml
│   ├── sensitivity.yaml
│   ├── scenarios.yaml
│   ├── variance.yaml
│   └── breakeven.yaml
│
└── validators/r/         # R validation scripts
    ├── monte_carlo_validator.R
    ├── bootstrap_validator.R
    ├── bayesian_validator.R
    ├── decision_tree_validator.R
    ├── real_options_validator.R
    ├── tornado_validator.R
    ├── sensitivity_validator.R
    ├── distribution_validator.R
    ├── financial_validator.R
    ├── math_validator.R
    ├── date_validator.R
    └── install_deps.R
```

## Test YAML Format

```yaml
_forge_version: "1.0.0"
_r_validator: "monte_carlo_validator.R"

tests:
  normal_standard:
    distribution: normal
    params:
      mean: 100
      sd: 15
    seed: 42
    iterations: 10000
    r_expected:
      mean: 99.6126336002
      std: 15.0378185524
      percentiles:
        "5": 75.3192593056
        "50": 99.8029873351
        "95": 123.0023063276
    tolerance:
      mean: 0.01
      std: 0.05
      percentiles: 0.02
```

## Tolerance Rationale

| Statistic | Tolerance | Rationale |
|-----------|-----------|-----------|
| Mean | 1% | Central tendency converges quickly |
| Std | 5% | Variance estimates have higher variance |
| Percentiles | 2% | Tail estimates need more samples |
| KS test | p > 0.05 | Standard significance level |

These tolerances account for:
- Random sampling variation (even with same seed, implementation may differ)
- Floating-point precision differences
- Algorithm variations between implementations

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## Related Projects

- [forge](https://github.com/mollendorff-ai/forge) - Deterministic YAML financial modeling engine
- [forge-e2e](https://github.com/mollendorff-ai/forge-e2e) - Documentation hub for E2E validation
- [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric) - Excel formula validation

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
