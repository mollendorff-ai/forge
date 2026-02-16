# ADR-045: Function-Level Coverage Tracking

**Status:** Accepted
**Date:** 2025-12-17
**Author:** forge-e2e Team
**Version:** v0.12.0

## Context

forge enterprise supports 173 functions across 15 categories. As the E2E test suite has grown to 2100+ test cases across 62 test files, we needed a systematic way to track which functions have test coverage and their validation status.

Without function-level tracking, we faced:

- **Unknown Coverage Gaps:** No visibility into which of the 173 functions lack tests
- **Validation Ambiguity:** Unclear which functions are validated by Gnumeric vs R vs forge-direct
- **Prioritization Difficulty:** No data-driven approach to prioritize test development
- **Coverage Regression:** Risk of losing coverage when refactoring tests

## Decision

We will implement function-level coverage tracking using automated scripts that:

1. **Extract Function Definitions** (`scripts/extract_functions.sh`)
   - Parse `../forge/src/functions/definitions.rs` as the single source of truth
   - Extract all 173 function names, categories, and metadata
   - Support multiple output formats (JSON, CSV, Markdown)

2. **Generate Coverage Reports** (`scripts/coverage_report.sh`)
   - Parse all test YAML files to extract function names from formulas
   - Cross-reference against the 173 functions from forge
   - Track validation status (Gnumeric, R, or untested)
   - Generate coverage matrix showing:
     - Function name
     - Has test? (yes/no)
     - Gnumeric validated? (yes/no)
     - R validated? (yes/no)
     - Test file references

3. **Maintain Coverage Matrix** (`coverage/function_matrix.md`)
   - Auto-generated markdown table showing coverage status
   - Updated via `scripts/coverage_report.sh --output coverage/function_matrix.md`
   - Tracked in git to monitor coverage trends over time

4. **Document Methodology** (this ADR)
   - Define what "covered" means
   - Define what "validated" means
   - Establish gap prioritization strategy

## Coverage Methodology

### Definition: "Covered"

A function is considered **covered** if:

- At least one test YAML file contains a formula using that function
- The test includes expected output (not just a syntax check)

A function is **not covered** if:

- No test files contain formulas using that function
- Only referenced in comments or documentation

### Definition: "Validated"

A function is **validated** based on which external validation engine verifies its correctness:

1. **Gnumeric Validated**
   - Test file is listed in `test_manifest.yaml` under `gnumeric_compatible:` section
   - Formula is evaluated by Gnumeric (`ssconvert`)
   - Results match expected output
   - Applicable to: Excel-compatible functions (SUM, IF, VLOOKUP, DATE, etc.)

2. **R Validated**
   - Test file is listed in `test_manifest.yaml` under `forge_native:` section
   - Formula is evaluated by R validator scripts (`validators/r/`)
   - Results match R's statistical packages (stats, boot)
   - Applicable to: Statistical distributions, bootstrap, forge-native functions

3. **Not Validated (Yet)**
   - Function has test coverage but not yet validated by external tool
   - Test files in `table_tests:` or `cli_only:` sections
   - Requires forge-direct evaluation mode

### Validation Tiers (from ADR-037)

```
Tier 1: Gnumeric (~120 functions)
  - Excel-compatible functions
  - Validated via ssconvert
  - Examples: SUM, IF, VLOOKUP, DATE, PMT

Tier 2: R (~50 functions)
  - Statistical distributions
  - Forge-native functions
  - Validated via R scripts
  - Examples: VARIANCE, BREAKEVEN, MC.Normal, PERCENTILE.BOOT

Tier 3: Forge-Direct
  - Table references (products.price, sales.amount)
  - Advanced array functions
  - Requires forge schema context
  - Examples: Tests using table syntax
```

## Gap Prioritization Strategy

When prioritizing which untested functions to cover first:

### Priority 1: Demo Functions

- Functions with `demo: true` in definitions.rs
- These are customer-facing functions in forge-demo
- Examples: ABS, SUM, IF, DATE, CONCAT
- Target: 100% coverage of all 48 demo functions

### Priority 2: Excel-Compatible Functions

- Functions that can be validated by Gnumeric
- Ensures compatibility with standard spreadsheet behavior
- Categories: Math, Aggregation, Logical, Text, Date, Lookup
- Target: 95%+ coverage

### Priority 3: Financial Functions

- Critical for business use cases
- Examples: PMT, NPV, IRR, PV, FV
- Target: 90%+ coverage

### Priority 4: Statistical Functions

- Requires R validation
- Examples: MEDIAN, STDEV, VAR, PERCENTILE, QUARTILE
- Target: 85%+ coverage

### Priority 5: Advanced/Enterprise Functions

- Array functions, LET, LAMBDA, SCENARIO
- Monte Carlo distributions
- Target: 80%+ coverage

### Coverage Quality Metrics

Beyond simple "has a test" tracking, we track:

1. **Test Depth**
   - Number of test cases per function
   - Edge cases covered (zero, negative, overflow, error conditions)

2. **Validation Quality**
   - External validation vs forge-only
   - Cross-validation (Gnumeric + R when applicable)

3. **Test Maintenance**
   - Last updated timestamp
   - Known issues or expected failures

## Usage

### Generate Coverage Report

```bash
# Generate markdown coverage matrix
./scripts/coverage_report.sh

# Generate CSV for analysis
./scripts/coverage_report.sh --format csv --output coverage/functions.csv

# Generate JSON for programmatic access
./scripts/coverage_report.sh --format json --output coverage/functions.json

# Verbose mode (show progress)
./scripts/coverage_report.sh --verbose
```

### Extract Function Definitions

```bash
# Generate markdown reference
./scripts/extract_functions.sh --format markdown

# Generate JSON for tooling
./scripts/extract_functions.sh --format json > functions.json

# Generate CSV for spreadsheet analysis
./scripts/extract_functions.sh --format csv > functions.csv
```

### Monitor Coverage Trends

```bash
# Generate coverage report and commit
./scripts/coverage_report.sh
git add coverage/function_matrix.md
git commit -m "docs: Update function coverage matrix"

# Compare coverage over time
git log -p coverage/function_matrix.md | grep "Coverage:"
```

## Implementation

### Scripts

1. **`scripts/extract_functions.sh`**
   - Parses `../forge/src/functions/definitions.rs`
   - Extracts 173 function definitions
   - Outputs in JSON, CSV, or Markdown format
   - Groups by category

2. **`scripts/coverage_report.sh`**
   - Scans all test YAML files in `tests/`
   - Extracts function names from `formula:` lines
   - Cross-references with `test_manifest.yaml` for validation tier
   - Generates coverage matrix showing test and validation status
   - Calculates coverage percentage by category

### Generated Artifacts

1. **`coverage/function_matrix.md`**
   - Markdown table showing all 173 functions
   - Coverage status: tested vs untested
   - Validation status: Gnumeric, R, or none
   - Test file references
   - Gap analysis (untested functions by category)

### Automation

Coverage reports should be regenerated:

- Before each release to track progress
- After adding new test files
- When onboarding new functions to forge

Consider adding to CI/CD:

```bash
# In .github/workflows/coverage.yml
- name: Generate Coverage Report
  run: ./scripts/coverage_report.sh
- name: Check Coverage Threshold
  run: |
    coverage=$(grep "Coverage:" coverage/function_matrix.md | awk '{print $2}' | tr -d '%')
    if (( $(echo "$coverage < 80.0" | bc -l) )); then
      echo "Coverage $coverage% is below 80% threshold"
      exit 1
    fi
```

## Alternatives Considered

### Alternative 1: Manual Tracking in Spreadsheet

- **Pros:** Simple, familiar tool
- **Cons:** Manual updates, prone to drift, no automation
- **Rejected:** Not sustainable at 173 functions

### Alternative 2: Code Coverage Tools (cargo-tarpaulin)

- **Pros:** Standard Rust tooling, automated
- **Cons:** Measures code coverage, not function coverage; doesn't track validation status
- **Rejected:** Doesn't meet our needs for function-level tracking

### Alternative 3: Custom Database

- **Pros:** Structured storage, query capabilities
- **Cons:** Adds complexity, requires DB setup, harder to review
- **Rejected:** Overkill for current scale

### Alternative 4: Parse Test Files Only

- **Pros:** No dependency on forge source
- **Cons:** No single source of truth, prone to missing new functions
- **Rejected:** forge/definitions.rs is the authoritative function list

## Consequences

### Positive

- **Visibility:** Clear view of which functions have test coverage
- **Data-Driven Prioritization:** Can prioritize gaps based on function importance
- **Quality Metrics:** Track coverage trends over time
- **Onboarding:** New contributors can see what needs testing
- **Release Confidence:** Coverage reports inform release readiness

### Negative

- **Maintenance Overhead:** Scripts must be kept in sync with definitions.rs format
- **False Positives:** Simple regex parsing may miss complex function usage
- **Manual Updates:** Coverage matrix must be regenerated manually (not yet in CI)

### Neutral

- **Single Source of Truth:** Depends on forge/definitions.rs structure
- **Test File Parsing:** Assumes YAML format and formula: field naming
- **Cross-Repo Dependency:** Requires ../forge to be checked out

## Related ADRs

- [ADR-037: External Validation Engines](ADR-037-EXTERNAL-VALIDATION-ENGINES.md) - Defines validation tiers
- [ADR-040: Test Organization](ADR-040-FINANCIAL-ANALYTICS-VALIDATION.md) - Test file structure
- [ADR-043: Test Manifest Structure](ADR-043-REGRESSION-DETECTION.md) - Test categorization

## References

- forge function definitions: `../forge/src/functions/definitions.rs`
- Test manifest: `tests/test_manifest.yaml`
- Existing coverage dashboard: `scripts/coverage_dashboard.sh`

## Review History

- **2025-12-17:** Initial version (v0.12.0)
