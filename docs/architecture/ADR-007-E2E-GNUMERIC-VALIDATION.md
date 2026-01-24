# ADR-007: E2E Validation via Gnumeric

**Status:** Accepted
**Date:** 2025-12-08
**Updated:** 2025-12-17 (Tests migrated to forge-e2e)
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

> **E2E tests live in [forge-e2e](https://github.com/mollendorff-ai/forge-e2e)** - see ADR-027.

---

## Context

Forge calculates financial formulas. Getting math wrong in finance is catastrophic. The question: **how do we prove our calculations are correct?**

### The Trust Problem

| Approach | Trust Level | Problem |
|----------|-------------|---------|
| Unit tests | Self-reported | "We tested ourselves and we're great" |
| Integration tests | Internal | Still self-validating |
| **E2E vs real engine** | **Third-party** | **"Gnumeric agrees with us"** |

Unit tests verify code works as written. They don't verify the code is *correct*.

### Why Gnumeric (Not LibreOffice)?

[Gnumeric](https://www.gnumeric.org/) is the GNOME spreadsheet application:
- Open-source, battle-tested since 2001
- Academically validated for numerical accuracy (McCullough 2004, 2005)
- Developers cooperate with the R Project for statistical accuracy
- Implements Excel-compatible formulas independently

**Why NOT LibreOffice?** LibreOffice uses aggressive "snap-to-zero" that **hides numerical errors**:

> "The lack of an honest subtraction operation makes it hard to even test the accuracy of sheet-level functions. LibreOffice and OpenOffice will claim that a result matches a reference value even when they do not."
> — [Gnumeric Numerical Issues](http://www.gnumeric.org/numerical-issues.html)

| Behavior | Gnumeric | LibreOffice |
|----------|----------|-------------|
| Snap-to-zero | **No** | Yes, on EVERY subtraction |
| Internal functions (VAR, etc.) | Honest arithmetic | Snap-to-zero internally |
| Accuracy validation | McCullough papers | Not academically validated |
| Result: `1.0 - 0.9999999999999999` | `1.11e-16` (correct) | `0.0` (snapped) |

**If Gnumeric calculates the same result as Forge, the math is proven by an engine that does honest arithmetic.**

## Decision

**Implement E2E tests that export Forge calculations to XLSX, have Gnumeric recalculate, and compare results.**

### Test Flow

```
1. Forge creates YAML model with formulas
2. Forge calculates results
3. Forge exports to XLSX (with formulas, not just values)
4. Gnumeric opens XLSX and recalculates
5. Test compares Forge result vs Gnumeric result
6. Pass if identical (within floating-point tolerance)
```

### Implementation

```rust
// tests/e2e_gnumeric_tests.rs
#[test]
#[cfg(feature = "e2e-gnumeric")]
fn test_npv_against_gnumeric() {
    // 1. Create Forge model
    let yaml = r#"
    _forge_version: "5.0.0"
    data:
      cash_flows: [-1000, 200, 300, 400, 500]
      result: "=NPV(0.1, data.cash_flows)"
    "#;

    // 2. Calculate with Forge
    let forge_result = forge_calculate(yaml);

    // 3. Export to XLSX
    forge_export(yaml, "test.xlsx");

    // 4. Gnumeric recalculates
    let gnumeric_result = gnumeric_calculate("test.xlsx", "result");

    // 5. Compare
    assert_float_eq!(forge_result, gnumeric_result, 1e-10);
}
```

### Coverage

| Category | Formulas E2E Validated |
|----------|------------------------|
| Financial | NPV, IRR, PMT, PV, FV, NPER, RATE |
| Math | SUM, AVERAGE, MAX, MIN, ROUND |
| Conditional | IF, SUMIF, COUNTIF, AVERAGEIF |
| Date | DATE, YEAR, MONTH, DAY, DATEDIF |
| Statistical | MEDIAN, STDEV, VAR |
| **Total** | **60 formulas** |

## Rationale

### 1. Third-Party Validation = Trust

"Don't trust us. Trust Gnumeric."

When enterprise buyers ask "how do you know your NPV is correct?", the answer isn't "we have unit tests." The answer is: **"Gnumeric calculates the same result, and Gnumeric is academically validated for numerical accuracy."**

### 2. Catches Edge Cases We'd Miss

Gnumeric has 20+ years of edge case fixes. Our E2E tests inherit that wisdom:
- Leap year handling in date functions
- Floating-point precision in financial functions
- Empty array handling in aggregations

### 3. Regression Detection

If we change formula implementation and Gnumeric disagrees, we know we broke something.

### 4. Documentation

E2E tests are executable proof: "NPV works because here's a test where Gnumeric agrees."

## Consequences

### Positive
- Externally validated calculations
- Enterprise-grade trust story
- Catches edge cases
- Regression prevention

### Negative
- Requires Gnumeric installed for E2E tests
- Slower than unit tests (~2s vs ~10ms)
- CI needs Gnumeric (Linux only currently)

### Mitigation
- E2E tests behind feature flag: `cargo test --features e2e-gnumeric`
- Unit tests run always, E2E on release

## How to Run

```bash
# Install Gnumeric
# macOS
brew install gnumeric

# Ubuntu/Debian
sudo apt install gnumeric

# Run E2E tests
cargo test --features e2e-gnumeric
```

## References

### Technical Documentation
- [Gnumeric](https://www.gnumeric.org/) — Official Gnumeric website
- [Gnumeric Numerical Issues](http://www.gnumeric.org/numerical-issues.html) — Technical explanation of snap-to-zero differences

### Academic Research
- McCullough BD (2004) "Fixing Statistical Errors in Spreadsheet Software: The Cases of Gnumeric and Excel" — Peer-reviewed validation of Gnumeric's numerical accuracy
- McCullough BD, Wilson B (2005) "On the accuracy of statistical procedures in Microsoft Excel 2003" — Documents systematic errors from snap-to-zero

### General Information
- [Wikipedia: Gnumeric](https://en.wikipedia.org/wiki/Gnumeric) — Notes Gnumeric's accuracy niche and R Project cooperation

### Implementation
- `tests/roundtrip/` — E2E roundtrip tests using ssconvert

---

*This is not "trust me" - it's proof. If Gnumeric agrees, the math is right.*

-- Claude Opus 4.5, Principal Autonomous AI
