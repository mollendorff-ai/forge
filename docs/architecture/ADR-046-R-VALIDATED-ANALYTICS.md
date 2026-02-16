# ADR-046: R-Validated Analytics E2E Tests

**Status:** Accepted
**Date:** 2026-01-01
**Author:** forge-e2e Team
**Version:** v0.9.0

## Context

While formula-based E2E tests (v0.8.4) achieve 1125 passed tests using `forge calculate`, the analytics features have **zero real tests**:

| Feature | Current State | Real Tests |
|---------|---------------|------------|
| Monte Carlo | Placeholder stubs (`=1+1`) | 0 |
| Bootstrap | Placeholder stubs | 0 |
| Tornado | **Missing entirely** | 0 |
| Sensitivity | Partial binary test | 0 |
| Scenarios | Placeholder stubs | 0 |
| Decision Trees | Placeholder stubs | 0 |
| Real Options | Placeholder stubs | 0 |
| Bayesian Networks | Placeholder stubs | 0 |
| Goal Seek | Partial binary test | 0 |

This is unacceptable for FP&A correctness requirements.
The analytics features require CLI commands (`forge simulate`, `forge tornado`, etc.), not `forge calculate`.

## Decision

We will implement **round-trip E2E testing** for all 9 analytics features:

1. **Invoke actual forge CLI commands** (not just `forge calculate`)
2. **Capture JSON output** from forge analytics commands
3. **Validate against R reference implementations** using established CRAN packages
4. **Use statistical tolerance** for stochastic outputs (Monte Carlo, Bootstrap)

### R Package Selection

After researching available R packages, we selected battle-proven CRAN packages:

| Feature | R Package | Key Function | Why Selected |
|---------|-----------|--------------|--------------|
| Monte Carlo | mc2d + stats | `rpert()`, `rtriang()`, `rnorm()` | PERT/Triangular in mc2d |
| Bootstrap | boot | `boot.ci()` | Gold standard, 5 CI methods |
| Tornado | tornado | `tornado()` | Direct tornado plot generation |
| Sensitivity | sensitivity | `sobol2007()`, `pcc()` | Comprehensive SA methods |
| Decision Trees | data.tree | `Aggregate()` | EMV via backward induction |
| Real Options | RQuantLib + derivmkts | `EuropeanOption()` | Industry-standard QuantLib |
| Bayesian | bnlearn | `cpquery()` | Belief propagation |
| Goal Seek | stats | `uniroot()`, `optimize()` | Base R optimization |

### Test Architecture

```
tests/
├── e2e/
│   ├── functions/          # Formula tests (existing, use forge calculate)
│   │   ├── math.yaml
│   │   ├── financial.yaml
│   │   └── ...
│   └── analytics/          # CLI tests (new, use forge simulate/tornado/etc.)
│       ├── monte_carlo.yaml
│       ├── bootstrap.yaml
│       ├── tornado.yaml
│       ├── sensitivity.yaml
│       ├── scenarios.yaml
│       ├── decision_trees.yaml
│       ├── real_options.yaml
│       ├── bayesian.yaml
│       └── goal_seek.yaml
├── fixtures/               # Shared test data files
│   ├── monte_carlo/
│   │   ├── normal_dist.yaml
│   │   └── pert_dist.yaml
│   └── decision_trees/
│       └── investment_decision.yaml
validators/
├── r/
│   ├── run_all.sh          # Orchestration script
│   ├── monte_carlo_validator.R    # Existing (19 tests)
│   ├── bootstrap_validator.R      # Existing (15 tests)
│   ├── tornado_validator.R        # NEW
│   ├── sensitivity_validator.R    # NEW
│   ├── scenario_validator.R       # NEW
│   ├── decision_tree_validator.R  # NEW
│   ├── real_options_validator.R   # NEW
│   ├── bayesian_validator.R       # NEW
│   └── goal_seek_validator.R      # NEW
src/
├── cli_runner.rs           # NEW: Run forge CLI commands
├── r_validator.rs          # NEW: Call R scripts, parse output
├── stats.rs                # NEW: Statistical comparison (KS test, tolerance)
└── analytics_tests.rs      # NEW: Analytics test runner
```

### Test YAML Schema for Analytics

```yaml
# tests/e2e/analytics/monte_carlo.yaml
schema_version: "6.0.0"
type: analytics
command: simulate

tests:
  - name: normal_distribution_10k
    fixture: fixtures/monte_carlo/normal_dist.yaml
    args: ["--iterations", "10000", "--seed", "42"]
    validation:
      method: statistical
      r_validator: monte_carlo_validator.R
      tolerance:
        mean: 0.01      # 1% tolerance
        std: 0.05       # 5% tolerance
        ks_test: 0.05   # KS test p-value threshold
    expected:
      distribution: normal
      mean: 100.0
      std: 15.0
```

### Round-Trip Testing Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        E2E Test Runner                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Load test YAML (tests/e2e/analytics/monte_carlo.yaml)       │
│  2. Load fixture (fixtures/monte_carlo/normal_dist.yaml)        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Run forge CLI command:                                       │
│     forge simulate fixtures/monte_carlo/normal_dist.yaml \       │
│       --iterations 10000 --seed 42 --output json                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Parse JSON output from forge                                 │
│     {                                                            │
│       "iterations": 10000,                                       │
│       "mean": 99.87,                                             │
│       "std": 14.92,                                              │
│       "percentiles": { "5": 75.2, "50": 99.9, "95": 124.8 }     │
│     }                                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. Run R validator:                                             │
│     Rscript validators/r/monte_carlo_validator.R \               │
│       --dist normal --mean 100 --sd 15 --n 10000 --seed 42       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  6. Compare results with statistical tolerance:                  │
│     - |forge_mean - r_mean| / r_mean < 0.01  ✓                   │
│     - |forge_std - r_std| / r_std < 0.05     ✓                   │
│     - KS test p-value > 0.05                 ✓                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  7. Report: PASS / FAIL with details                             │
└─────────────────────────────────────────────────────────────────┘
```

### R Validator JSON Contract

All R validators MUST output JSON to stdout for Rust to parse:

```json
{
  "validator": "monte_carlo",
  "version": "1.0.0",
  "success": true,
  "results": {
    "mean": 100.02,
    "std": 14.98,
    "percentiles": { "5": 75.3, "50": 100.1, "95": 124.5 },
    "samples": [99.2, 101.5, ...]  // Optional: for KS test
  },
  "error": null
}
```

For errors:

```json
{
  "validator": "monte_carlo",
  "success": false,
  "results": null,
  "error": "Package 'mc2d' not installed"
}
```

### Rust Module API

**`src/cli_runner.rs`** - Forge CLI executor:

```rust
pub struct ForgeCommand {
    pub cmd: String,        // "simulate", "tornado", "goal-seek"
    pub args: Vec<String>,  // ["--iterations", "10000", "--seed", "42"]
    pub fixture: PathBuf,   // Path to YAML fixture
}

pub struct AnalyticsOutput {
    pub raw_json: Value,
    pub stats: Option<Stats>,  // Parsed summary statistics
}

pub fn run_forge_command(cmd: &ForgeCommand) -> Result<AnalyticsOutput>;
```

**`src/r_validator.rs`** - R script bridge:

```rust
pub struct RParams {
    pub distribution: String,  // "normal", "pert", "triangular"
    pub params: HashMap<String, f64>,  // {"mean": 100, "sd": 15}
    pub seed: u64,
    pub iterations: usize,
}

pub struct RResult {
    pub validator: String,
    pub success: bool,
    pub results: Option<Value>,
    pub error: Option<String>,
}

pub fn validate_with_r(validator: &str, params: &RParams) -> Result<RResult>;
pub fn compare_results(forge: &AnalyticsOutput, r: &RResult, tol: &Tolerance) -> ValidationResult;
```

**`src/stats.rs`** - Statistical comparison:

```rust
pub fn within_tolerance(actual: f64, expected: f64, pct: f64) -> bool;
pub fn ks_test_pvalue(sample1: &[f64], sample2: &[f64]) -> f64;
pub fn compare_distributions(forge: &Stats, r: &Stats, tol: &Tolerance) -> StatResult;
```

### Error Handling Strategy

| Scenario | Action |
|----------|--------|
| R not installed | Skip analytics tests with warning |
| R package missing | Fail test with install instructions |
| R script timeout (30s) | Fail test, log timeout |
| R script error | Fail test, capture stderr |
| Forge command fails | Fail test, capture stderr |
| JSON parse error | Fail test with parse error details |

### Parallel Execution

- Analytics tests run in parallel using `rayon`
- Each test spawns its own R process (no shared state)
- Seed ensures reproducibility despite parallelism
- Thread pool size: `num_cpus::get()` (default)

### Integration with run-e2e.sh

```bash
#!/bin/bash
# run-e2e.sh additions

# Existing modes
--forge        # Formula tests via forge calculate
--gnumeric     # Formula tests via Gnumeric ssconvert

# New modes for analytics
--analytics    # All analytics tests (Monte Carlo, Bootstrap, etc.)
--monte-carlo  # Monte Carlo tests only
--bootstrap    # Bootstrap tests only
--tornado      # Tornado tests only
--sensitivity  # Sensitivity tests only
--r-validate   # Run R validators after forge tests

# Examples:
./run-e2e.sh --forge --all                    # All formula tests
./run-e2e.sh --analytics --all                # All analytics tests
./run-e2e.sh --monte-carlo --r-validate       # MC tests with R validation
./run-e2e.sh --forge --analytics --all        # Everything
```

### Statistical Tolerance Strategy

For stochastic outputs (Monte Carlo, Bootstrap):

| Metric | Tolerance | Rationale |
|--------|-----------|-----------|
| Mean | 1% | Central tendency should be stable |
| Std Dev | 5% | More variance in variance estimates |
| Percentiles | 2% | Tail behavior is noisier |
| KS Test | p > 0.05 | Distribution shape match |
| CI Bounds | 2% | Bootstrap intervals |

For deterministic outputs (Tornado, Sensitivity, Goal Seek):

| Metric | Tolerance | Rationale |
|--------|-----------|-----------|
| Rankings | Exact match | Variable order must match |
| Swing values | 0.1% | Numerical precision |
| Optimal value | 1e-8 | Goal seek convergence |
| EMV | 0.01% | Decision tree calculations |

## Implementation Plan

### Phase 1: Framework (Week 1)

- [ ] Add `src/cli_runner.rs` - Run forge CLI commands
- [ ] Add `src/r_validator.rs` - Call R scripts
- [ ] Add `src/stats.rs` - Statistical comparisons
- [ ] Update `run-e2e.sh` with `--analytics` mode
- [ ] Rename `tests/e2e/enterprise/` → `tests/e2e/analytics/`

### Phase 2: Monte Carlo & Bootstrap (Week 2)

- [ ] Install R packages: `mc2d`, `boot`
- [ ] Write real `monte_carlo.yaml` tests (6 distributions)
- [ ] Write real `bootstrap.yaml` tests (BCa, percentile, basic)
- [ ] Integrate existing R validators with E2E runner

### Phase 3: Tornado & Sensitivity (Week 3)

- [ ] Install R packages: `tornado`, `sensitivity`
- [ ] Create `validators/r/tornado_validator.R`
- [ ] Create `validators/r/sensitivity_validator.R`
- [ ] Write `tornado.yaml` and `sensitivity.yaml` tests

### Phase 4: Decision Trees & Real Options (Week 4)

- [ ] Install R packages: `data.tree`, `RQuantLib`, `derivmkts`
- [ ] Create `validators/r/decision_tree_validator.R`
- [ ] Create `validators/r/real_options_validator.R`
- [ ] Write `decision_trees.yaml` and `real_options.yaml` tests

### Phase 5: Bayesian & Goal Seek (Week 5)

- [ ] Install R packages: `bnlearn`
- [ ] Create `validators/r/bayesian_validator.R`
- [ ] Create `validators/r/goal_seek_validator.R`
- [ ] Write `bayesian.yaml` and `goal_seek.yaml` tests

### Phase 6: Scenarios & Cleanup

- [ ] Write `scenarios.yaml` tests (manual R calculations)
- [ ] Delete all placeholder stubs
- [ ] CI integration for R validators
- [ ] Documentation update

## Alternatives Considered

### Alternative 1: Python Validation

- **Pros:** NumPy/SciPy are excellent, Python is common
- **Cons:** R has more domain-specific packages (bnlearn, data.tree)
- **Rejected:** R packages better match forge's analytics features

### Alternative 2: Implement Validators in Rust

- **Pros:** No external dependency, faster execution
- **Cons:** Reinventing statistical wheels, not authoritative
- **Rejected:** R packages are battle-proven references

### Alternative 3: Unit Tests Only

- **Pros:** Simpler, no external tools
- **Cons:** Not true E2E validation, no external authority
- **Rejected:** Doesn't meet FP&A correctness requirements

### Alternative 4: Excel Validation

- **Pros:** Excel is the industry standard
- **Cons:** Hard to automate, no Monte Carlo in Excel
- **Rejected:** Gnumeric covers Excel functions; R needed for analytics

## Consequences

### Positive

- **Authoritative Validation:** R packages are industry standards
- **Complete Coverage:** All 9 analytics features will have real tests
- **Statistical Rigor:** Proper tolerance for stochastic outputs
- **Reproducibility:** Seeded random tests for deterministic CI

### Negative

- **R Dependency:** CI must have R installed with packages
- **Complexity:** More moving parts than formula tests
- **Slower Tests:** R invocation adds latency

### Neutral

- **Hybrid Testing:** Formula tests remain Rust-only; analytics adds R

## References

- `.asimov/references.yaml` - R package documentation
- `.asimov/roadmap.yaml` - Implementation roadmap
- [mc2d CRAN](https://cran.r-project.org/web/packages/mc2d/)
- [boot CRAN](https://cran.r-project.org/web/packages/boot/)
- [bnlearn CRAN](https://cran.r-project.org/web/packages/bnlearn/)
- [data.tree CRAN](https://cran.r-project.org/web/packages/data.tree/)
- [RQuantLib CRAN](https://cran.r-project.org/web/packages/RQuantLib/)

## Review History

- **2026-01-01:** Initial version (v0.9.0)
