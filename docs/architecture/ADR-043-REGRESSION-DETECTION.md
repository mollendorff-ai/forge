# ADR-043: Regression Detection Strategy

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Sonnet 4.5 (Principal Autonomous AI)

---

## Context

Forge-e2e validates ~700+ tests across multiple categories. As the codebase evolves:

- **New features** may break existing calculations
- **Refactoring** can introduce subtle bugs
- **Dependency updates** (Gnumeric, R) may change behavior
- **Environment changes** affect test reliability

**Problem**: How do we catch regressions—tests that used to pass but now fail?

Without regression tracking, developers don't know if failures are:

- New bugs introduced by their changes
- Pre-existing failures
- Environmental issues

## Decision

**Implement baseline-driven regression detection with two scripts:**

1. `scripts/baseline_save.sh` - Captures test results as JSON baseline
2. `scripts/baseline_compare.sh` - Detects regressions against baseline

### Design Principles

1. **Baseline as Source of Truth**: Save passing state as baseline, compare future runs against it
2. **Machine-Readable Format**: JSON for easy parsing and automation
3. **Metadata Tracking**: Include forge version, Gnumeric version, timestamp for debugging
4. **Exit Code Contract**: Compare script returns 1 if regressions found (CI-friendly)
5. **Human-Readable Output**: Pretty-printed diff with colors for manual review

---

## Implementation

### 1. Baseline Structure

Baselines are stored in `baselines/YYYY-MM-DD.json`:

```json
{
  "metadata": {
    "timestamp": "2025-12-17T18:00:00Z",
    "forge_version": "forge 9.9.0",
    "gnumeric_version": "ssconvert version 1.12.57",
    "hostname": "ci-runner-01",
    "platform": "Darwin"
  },
  "results": [
    {
      "name": "assumptions.test_abs",
      "status": "pass",
      "actual": "42"
    },
    {
      "name": "assumptions.test_npv",
      "status": "fail",
      "formula": "=NPV(0.1, A1:A5)",
      "expected": "100.5",
      "actual": "100.4999",
      "error": "Value mismatch"
    },
    {
      "name": "assumptions.test_datedif",
      "status": "skip",
      "reason": "Gnumeric DATEDIF not implemented"
    }
  ]
}
```

### 2. Baseline Save Script

**Location**: `scripts/baseline_save.sh`

**Usage**:

```bash
# Save today's results as baseline
./scripts/baseline_save.sh

# Save to specific file
./scripts/baseline_save.sh --output baselines/stable-v9.8.0.json
```

**Implementation**:

- Runs `./run-e2e.sh --all` to execute all tests
- Parses text output to extract test results
- Captures metadata: forge version, Gnumeric version, timestamp
- Saves to `baselines/YYYY-MM-DD.json` by default
- Reports summary: passed, failed, skipped counts

**Output Parsing**:

- Pass: `✓ test_name = value`
- Fail: `✗ test_name` followed by formula, expected, actual, error
- Skip: `⊘ test_name (reason)`

### 3. Baseline Compare Script

**Location**: `scripts/baseline_compare.sh`

**Usage**:

```bash
# Compare current run against baseline
./scripts/baseline_compare.sh baselines/2025-12-17.json

# Compare two saved baselines
./scripts/baseline_compare.sh baselines/old.json --current baselines/new.json
```

**Exit Codes**:

- `0`: No regressions detected
- `1`: Regressions found (tests that passed now fail)
- `2`: Error (missing files, invalid JSON)

**Detection Logic**:

| Baseline Status | Current Status | Classification |
|----------------|----------------|----------------|
| Pass | Fail | **Regression** (CRITICAL) |
| Fail | Pass | **Fix** (Good) |
| Pass | Pass | Unchanged |
| Fail | Fail | Unchanged |
| N/A | Pass/Fail | New test |
| Pass/Fail | N/A | Removed test |

**Output Format**:

```
══════════════════════════════════════════════════════════════════════
  REGRESSION DETECTION REPORT
══════════════════════════════════════════════════════════════════════

Baseline: baselines/2025-12-17.json
  Timestamp: 2025-12-17T18:00:00Z

Current:  /tmp/current-results.json
  Timestamp: 2025-12-17T19:30:00Z

Summary:
  Regressions:      3 (tests that now fail)
  Fixes:            2 (tests that now pass)
  Unchanged (pass): 700
  Unchanged (fail): 10
  New tests:        5
  Removed tests:    1

┌─ REGRESSIONS (tests that now fail) ────────────────────────────────┐
  1. assumptions.test_npv
     Baseline: PASS (actual: 100.5)
     Current:  FAIL
     Formula:  =NPV(0.1, A1:A5)
     Expected: 100.5
     Actual:   100.4999
     Error:    Value mismatch
└─────────────────────────────────────────────────────────────────────┘
```

---

## When to Update Baseline

### Update When

1. **Intentional behavior change**: Fixed a bug, baseline should now match new behavior
2. **Major release**: Forge 9.8.0 → 9.9.0, save new baseline for new version
3. **All tests pass**: No regressions, current run becomes new baseline
4. **Dependency upgrade**: Gnumeric updated, re-baseline if behavior changes

### DON'T Update When

1. **Regressions detected**: Fix the code, don't accept failures
2. **Random failures**: Flaky tests should be fixed, not accepted
3. **Work in progress**: Mid-development, incomplete changes

### Best Practice

- Save baseline **after** successful release/deploy
- Name baseline files semantically: `baselines/v9.8.0-stable.json`
- Keep multiple baselines for different environments (macOS, Linux)
- Commit baseline files to git for team consistency

---

## CI Integration Pattern

### GitHub Actions Workflow

```yaml
name: Regression Detection

on: [pull_request]

jobs:
  regression-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get install -y gnumeric
          brew install r  # or install R another way

      - name: Build forge
        run: cd ../forge && cargo build --release

      - name: Run regression detection
        run: |
          ./scripts/baseline_compare.sh baselines/stable.json
        env:
          FORGE_BIN: ../forge/target/release/forge

      - name: Upload results
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: regression-report
          path: /tmp/regression-*.json
```

### Local Development

```bash
# Before starting work, save current state
./scripts/baseline_save.sh --output baselines/pre-feature-x.json

# After changes, check for regressions
./scripts/baseline_compare.sh baselines/pre-feature-x.json

# If all clear, update main baseline
./scripts/baseline_save.sh --output baselines/stable.json
```

---

## Alternatives Considered

### 1. Git-Based Comparison (Rejected)

**Idea**: Compare test output between git commits

**Rejected because**:

- No metadata (what version of Gnumeric?)
- Requires git history (doesn't work on fresh clones)
- Hard to correlate failures with specific changes
- No semantic understanding of test results

### 2. Database Storage (Rejected)

**Idea**: Store test results in SQLite/Postgres

**Rejected because**:

- Over-engineered for simple use case
- Requires database setup (complexity)
- JSON files are human-readable and git-friendly
- No query complexity needed (linear scan is fine)

### 3. JUnit XML Format (Rejected)

**Idea**: Use standard JUnit XML for test results

**Rejected because**:

- Verbose, harder to read manually
- Designed for different use case (test suites)
- JSON is more natural for Rust serde integration
- Custom format is simpler and more flexible

### 4. Inline Metadata in Test Files (Rejected)

**Idea**: Store expected results in YAML test files

**Rejected because**:

- Already have `expected:` field in tests
- Baseline tracks **actual execution**, not spec
- Captures environment-specific behavior
- Metadata (versions) doesn't belong in test specs

---

## Future Enhancements

### 1. JSON Output Mode in Rust Tool

**Current**: Scripts parse text output (fragile)
**Future**: Add `--json` flag to `forge-e2e` for native JSON output

```rust
// In src/main.rs
if cli.json {
    println!("{}", serde_json::to_string_pretty(&results)?);
}
```

**Benefits**:

- Eliminates regex parsing
- Faster, more reliable
- Native data structure (TestResult already implements Serialize)

### 2. Baseline Diff Visualization

**Current**: Text output
**Future**: HTML report with visual diff

```bash
./scripts/baseline_compare.sh baseline.json --html report.html
```

### 3. Automatic Baseline Update on Release

**Current**: Manual
**Future**: CI automatically saves baseline on tag

```yaml
- name: Save release baseline
  if: startsWith(github.ref, 'refs/tags/v')
  run: |
    ./scripts/baseline_save.sh --output baselines/release-${{ github.ref_name }}.json
    git add baselines/
    git commit -m "Baseline for ${{ github.ref_name }}"
```

### 4. Per-Category Baselines

**Current**: Single baseline for all tests
**Future**: Separate baselines for financial, statistical, text functions

**Benefits**:

- Catch domain-specific regressions
- Different update cadence per category
- Clearer ownership (financial team owns financial baseline)

---

## Consequences

### Positive

- **Early detection**: Catch regressions before merging
- **CI integration**: Automated checks on every PR
- **Historical tracking**: Baselines provide audit trail
- **Confidence**: Developers know if they broke something

### Negative

- **Maintenance overhead**: Baselines need updating
- **False positives**: Environmental differences may trigger alerts
- **Parsing fragility**: Text parsing is brittle (mitigated by future JSON output)
- **Storage cost**: JSON files in git (minimal, ~100KB each)

### Neutral

- **Learning curve**: Team needs to understand baseline workflow
- **Multiple baselines**: May need per-environment/per-version baselines
- **Manual review**: Regressions still require human judgment

---

## Related Documents

- [ADR-036: Testing Philosophy](ADR-036-TESTING-PHILOSOPHY.md) - Why we test
- [ADR-037: External Validation Engines](ADR-037-EXTERNAL-VALIDATION-ENGINES.md) - Gnumeric & R
- [ADR-039: Statistical Validation](ADR-039-STATISTICAL-VALIDATION.md) - R validators
- [TEST-STRUCTURE.md](../TEST-STRUCTURE.md) - Test organization

---

## References

- Martin Fowler - [Continuous Integration](https://martinfowler.com/articles/continuousIntegration.html)
- Google Testing Blog - [Test Flakiness](https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html)
- Mozilla - [Baseline Testing](https://firefox-source-docs.mozilla.org/testing/perfdocs/baseline.html)
