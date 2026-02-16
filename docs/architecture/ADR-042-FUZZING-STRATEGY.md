# ADR-042: Property-Based Fuzzing Strategy

**Status:** Accepted
**Date:** 2025-12-17
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Forge implements 173 Excel-compatible functions with complex edge cases. Traditional unit tests are limited:

- **Test bias**: We test what we think might break, not what actually breaks
- **Limited coverage**: Edge cases are infinite; we test a finite set
- **Implementation coupling**: Tests written by developers who wrote the code

**Question:** How do we find bugs we didn't anticipate?

## Decision

**Implement property-based fuzzing using Gnumeric as an oracle.**

### What is Property-Based Fuzzing?

Instead of writing specific test cases, we:

1. **Generate random inputs** (numbers, strings, dates, edge cases)
2. **Run through both engines** (forge and Gnumeric)
3. **Compare results** (differential testing)
4. **Report mismatches** as potential bugs

**Key insight:** We don't need to know the "correct" answer. If forge and Gnumeric disagree, one of them is wrong.

### Why Gnumeric as Oracle?

From ADR-037, Gnumeric is:

- Academically validated for numerical accuracy (McCullough 2004, 2005)
- Battle-tested since 2001 (20+ years of edge case fixes)
- Implements Excel formulas independently from forge
- No snap-to-zero errors (unlike LibreOffice)

**Gnumeric is our "ground truth" for Excel-compatible functions.**

---

## Implementation

### Architecture

```
┌─────────────────────────────────────────────────────┐
│ Fuzzer (scripts/fuzz_oracle.sh)                     │
│                                                      │
│ 1. Generate random inputs                           │
│    - Numeric: integers, floats, negative, zero      │
│    - Edge cases: NaN, Infinity, large numbers       │
│    - Special values: empty, null                    │
│                                                      │
│ 2. Build formula: =FUNCTION(input1, input2, ...)    │
│                                                      │
│ 3. Create YAML test file                            │
└────────────────┬─────────────────────────────────────┘
                 │
         ┌───────┴──────────┐
         │                  │
         ▼                  ▼
┌─────────────────┐  ┌──────────────────┐
│ Forge           │  │ Gnumeric         │
│                 │  │                  │
│ forge calculate │  │ forge export →   │
│ → result        │  │ ssconvert →      │
│                 │  │ → result         │
└────────┬────────┘  └──────┬───────────┘
         │                  │
         └────────┬─────────┘
                  │
                  ▼
         ┌────────────────┐
         │ Compare        │
         │                │
         │ If different:  │
         │ → Log bug      │
         │ → Record seed  │
         │ → Save formula │
         └────────────────┘
```

### Tools

#### 1. scripts/fuzz_oracle.sh

Main fuzzing script that generates random inputs and compares results.

**Usage:**

```bash
# Fuzz specific function
./scripts/fuzz_oracle.sh --function ABS --iterations 1000

# Fuzz random functions with seed (reproducible)
./scripts/fuzz_oracle.sh --seed 12345 --iterations 500

# Verbose output
./scripts/fuzz_oracle.sh --function POWER --verbose
```

**Options:**

- `--function FUNC`: Function to fuzz (default: random)
- `--iterations N`: Number of test iterations (default: 100)
- `--seed N`: Random seed for reproducibility
- `--output DIR`: Output directory for results
- `--tolerance N`: Floating point tolerance (default: 1e-10)
- `--verbose`: Show detailed output

**Output:**

- `fuzz_results/fuzz_TIMESTAMP.log`: All test results
- `fuzz_results/bugs_TIMESTAMP.txt`: Bug details (if bugs found)

#### 2. scripts/fuzz_report.sh

Analyzes fuzzing results and generates bug reports.

**Usage:**

```bash
# Auto-detect latest bug file
./scripts/fuzz_report.sh

# Specific bug file
./scripts/fuzz_report.sh fuzz_results/bugs_20231217_123456.txt

# Generate markdown report
./scripts/fuzz_report.sh --format markdown --output bug_report.md

# Summary only
./scripts/fuzz_report.sh --summary
```

**Formats:**

- `text`: Human-readable text report (default)
- `markdown`: GitHub-compatible markdown
- `json`: Machine-readable JSON

---

## Input Generation Strategy

### 1. Numeric Inputs

For math functions (ABS, SQRT, POWER, etc.):

```bash
# Integers: -1000 to 1000
random_int -1000 1000

# Floats: -1000.0 to 1000.0 with 6 decimal places
random_float -1000 1000

# Edge cases (future):
- Zero: 0, 0.0, -0.0
- Large: 1e10, 1e20, Number.MAX_VALUE
- Small: 1e-10, 1e-20, Number.MIN_VALUE
- Special: NaN, Infinity, -Infinity
```

**Strategy:**

- Most values in "normal" range (-1000 to 1000)
- Some edge cases (zero, negative, large)
- Avoid domain errors for specific functions (e.g., SQRT only positive)

### 2. String Inputs

For text functions (CONCATENATE, LEFT, RIGHT, etc.):

```bash
# Random alphanumeric strings
random_string 10  # Length 10

# Edge cases (future):
- Empty string: ""
- Single char: "a"
- Special chars: "!@#$%^&*()"
- Unicode: "日本語", "emoji 🔥"
- Whitespace: "   ", "\t", "\n"
```

### 3. Date Inputs

For date functions (DATE, DATEDIF, EDATE, etc.):

```bash
# Random dates
random_date 1900-01-01 2100-12-31

# Edge cases (future):
- Leap years: 2000-02-29, 2024-02-29
- Boundaries: 1900-01-01 (Excel epoch), 9999-12-31
- Invalid: 2023-02-30, 2001-02-29 (non-leap year)
- Excel quirk: 1900-02-29 (doesn't exist, but Excel allows it)
```

### 4. Boolean Inputs

For logical functions (AND, OR, IF, etc.):

```bash
# Random boolean
random_bool  # TRUE or FALSE

# Edge cases (future):
- Numeric: 0 (FALSE), 1 (TRUE), non-zero (TRUE)
- String: "TRUE", "FALSE", "true", "false"
- Empty: null, undefined, ""
```

---

## Functions to Fuzz

### Tier 1: Basic Math (Highest Priority)

| Function | Inputs | Edge Cases |
|----------|--------|------------|
| ABS | 1 numeric | Negative, zero, large |
| SQRT | 1 positive | Zero, very small, very large |
| POWER | 2 numeric | Base=0, exponent=0, negative base |
| MOD | 2 numeric | Divisor=0, negative numbers |
| ROUND | 1-2 numeric | Large decimals, negative precision |
| FLOOR | 1 numeric | Negative numbers, zero |
| CEILING | 1 numeric | Negative numbers, zero |

**Why start here?**

- Simple functions with clear semantics
- Easy to verify results manually
- High impact (used in many models)

### Tier 2: Aggregation

| Function | Inputs | Edge Cases |
|----------|--------|------------|
| SUM | Array of numeric | Empty array, single value, all zeros |
| AVERAGE | Array of numeric | Empty array, single value, outliers |
| MIN | Array of numeric | Single value, all equal, negative |
| MAX | Array of numeric | Single value, all equal, negative |

### Tier 3: Logical

| Function | Inputs | Edge Cases |
|----------|--------|------------|
| IF | Condition, true, false | Always true/false, same values |
| AND | Array of boolean | Empty, single value, all true/false |
| OR | Array of boolean | Empty, single value, all true/false |

### Tier 4: Advanced (Future)

- **Financial**: NPV, IRR, PMT (complex domain constraints)
- **Text**: CONCATENATE, LEFT, RIGHT (string edge cases)
- **Date**: DATE, DATEDIF, EDATE (leap years, boundaries)
- **Lookup**: VLOOKUP, INDEX, MATCH (array handling)

---

## Interpreting Results

### Types of Findings

#### 1. Genuine Bugs

**Example:**

```
Formula:   =POWER(2, 10)
Forge:     1023
Gnumeric:  1024
Diff:      -1
```

**Action:**

- File GitHub issue in forge repository
- Include formula, inputs, expected vs actual
- Reference fuzzing seed for reproduction
- Investigate root cause in forge code

#### 2. Floating Point Precision

**Example:**

```
Formula:   =SQRT(2) * SQRT(2)
Forge:     2.0000000000000004
Gnumeric:  2.0
Diff:      4.440892098500626e-16
```

**Action:**

- Compare against tolerance (default: 1e-10)
- If within tolerance: PASS (expected rounding)
- If exceeds tolerance: Investigate further
- Document in ADR if acceptable

#### 3. Implementation Differences

**Example:**

```
Formula:   =ROUND(2.5, 0)
Forge:     3 (round half up)
Gnumeric:  2 (round half even)
```

**Action:**

- Research Excel behavior (which is correct?)
- Document intentional differences
- Update tolerance or fuzzer logic if needed
- Consider if this affects users

#### 4. Domain Errors

**Example:**

```
Formula:   =SQRT(-1)
Forge:     #NUM!
Gnumeric:  #NUM!
```

**Action:**

- Both error: PASS (correct behavior)
- Different errors: Investigate error codes
- One errors, one doesn't: BUG

---

## Fuzzing Workflow

### 1. Run Initial Fuzzing

```bash
# Start with basic math functions
./scripts/fuzz_oracle.sh --function ABS --iterations 1000
./scripts/fuzz_oracle.sh --function SQRT --iterations 1000
./scripts/fuzz_oracle.sh --function POWER --iterations 1000

# Or fuzz all basic functions (random)
for i in {1..10}; do
    ./scripts/fuzz_oracle.sh --iterations 100
done
```

### 2. Analyze Results

```bash
# Generate summary
./scripts/fuzz_report.sh --summary

# Detailed report
./scripts/fuzz_report.sh --format markdown --output bug_report.md
```

### 3. Triage Bugs

For each bug:

1. **Reproduce:** Run fuzzer with same seed
2. **Investigate:** Check formula, inputs, expected result
3. **Classify:** Genuine bug, precision issue, or implementation difference
4. **Document:** File issue or update ADR

### 4. Fix and Verify

```bash
# After fixing bug in forge
# Re-run fuzzer with same seed to verify fix
./scripts/fuzz_oracle.sh --seed 12345 --iterations 1000
```

### 5. Continuous Fuzzing

```bash
# Add to CI pipeline (future)
# Run nightly fuzzing jobs
# Alert on new bugs found
```

---

## Advantages

### 1. Finds Unexpected Bugs

**Traditional testing:**

```rust
#[test]
fn test_abs() {
    assert_eq!(abs(-42), 42);  // Tests what we think might break
}
```

**Fuzzing:**

```bash
# Tests random inputs we never considered
=ABS(-0.0000001)
=ABS(999999999999)
=ABS(-Infinity)  # Didn't even know this was possible!
```

### 2. Reproducible

Every bug is logged with:

- Random seed (can reproduce exact sequence)
- Formula and inputs
- Expected vs actual results
- Timestamp

**Reproduce any bug:**

```bash
./scripts/fuzz_oracle.sh --seed 12345 --function ABS
```

### 3. Differential Testing

We don't need to know the "correct" answer:

- If forge = Gnumeric → Probably correct
- If forge ≠ Gnumeric → Someone is wrong

**Then:** Manually verify against Excel or mathematical definition.

### 4. Complements Existing Tests

- **Unit tests**: Test specific known cases
- **E2E tests**: Test real-world scenarios
- **Fuzzing**: Find edge cases we didn't anticipate

All three together = comprehensive coverage.

---

## Limitations

### 1. Gnumeric is Not Always Right

Gnumeric can have bugs too. If fuzzing finds a discrepancy:

- Don't assume forge is wrong
- Verify against Excel (the "real" standard)
- Verify against mathematical definition
- Consider filing bug with Gnumeric if needed

### 2. Tolerance Tuning

Floating point comparison is hard:

- Too strict (1e-15): Many false positives
- Too loose (1e-5): Might miss real bugs
- Default (1e-10): Good balance, may need tuning

### 3. Domain Constraints

Some functions have input constraints:

- `SQRT` only positive numbers
- `MOD` divisor cannot be zero
- `DATE` must be valid dates

Fuzzer must respect these to avoid domain errors.

### 4. Complex Functions

Some functions are hard to fuzz:

- **VLOOKUP**: Requires array structure
- **NPV**: Requires specific cash flow format
- **DATEDIF**: Complex date arithmetic

Start simple (math), expand to complex later.

---

## Future Enhancements

### 1. Smarter Input Generation

Current: Pure random within range
Future:

- **Boundary values**: 0, 1, -1, max, min
- **Special values**: NaN, Infinity, -0.0
- **Domain-specific**: Leap years for dates, common rates for financial

### 2. Multi-Function Fuzzing

Current: Single function at a time
Future: Nested formulas

```
=ABS(SQRT(POWER(2, 10)))
=IF(SUM(A1:A10) > 0, AVERAGE(A1:A10), 0)
```

### 3. Array Fuzzing

Current: Scalar inputs
Future: Array inputs

```
=SUM(A1:A100)  # Fuzz entire array
=VLOOKUP(key, range, column)  # Fuzz lookup tables
```

### 4. Mutation-Based Fuzzing

Current: Pure random generation
Future: Mutate existing formulas

- Take working formula
- Tweak inputs slightly
- See if it breaks

### 5. Coverage-Guided Fuzzing

Track code coverage in forge:

- If input reaches new code path → Save it
- Mutate saved inputs to explore more paths
- Similar to AFL, libFuzzer

### 6. Performance Fuzzing

Current: Functional correctness only
Future: Performance regression detection

- Track execution time
- Alert if function becomes slow
- Find O(n²) bugs

---

## References

### Fuzzing

- Zalewski, M. (2014) *American Fuzzy Lop (AFL)* - Coverage-guided fuzzing
- Godefroid, P. et al. (2008) "Automated Whitebox Fuzz Testing" - SAGE fuzzer
- Claessen, K. & Hughes, J. (2000) "QuickCheck: A Lightweight Tool for Random Testing"

### Differential Testing

- McKeeman, W. M. (1998) "Differential Testing for Software"
- Yang, X. et al. (2011) "Finding and Understanding Bugs in C Compilers" - Csmith

### Property-Based Testing

- Haskell QuickCheck: Random property testing
- Hypothesis (Python): Smart input generation
- PropTest (Rust): Property testing for Rust

### Related ADRs

- **ADR-036: Testing Philosophy** - Overall e2e testing approach
- **ADR-037: External Validation Engines** - Why Gnumeric as oracle
- **ADR-038: Edge Case Discovery** - Edge case testing strategy

---

## Success Criteria

Fuzzing is successful if:

1. **Finds real bugs**: At least one genuine bug in forge per 10,000 iterations
2. **No false positives**: <5% false positive rate (after tolerance tuning)
3. **Reproducible**: Can reproduce any bug with seed
4. **Actionable**: Bug reports contain enough info to fix
5. **Fast**: Can run 1000 iterations in <5 minutes
6. **Automated**: Can run in CI without manual intervention

---

*"Testing shows the presence, not the absence of bugs." - Edsger Dijkstra*

*Fuzzing finds the bugs you didn't know existed.*

— Claude Opus 4.5, Principal Autonomous AI
