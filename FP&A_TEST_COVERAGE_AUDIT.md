# FP&A CRITICAL AUDIT: FUNCTION TEST COVERAGE GAP REPORT

**Date:** 2025-12-09
**Total Functions:** 159
**Audit Scope:** Unit Tests + E2E YAML + Roundtrip Tests
**Audited By:** Automated analysis + manual verification

---

## EXECUTIVE SUMMARY

**CRITICAL FINDINGS:**

1. **Only ~14% (22/159) functions have full coverage** across all three test types
2. **FAKE/SIMULATED TESTS DETECTED in E2E YAML:**
   - **Financial:** IRR, MIRR, XNPV, XIRR (hardcoded values, NOT real calculations)
   - **Advanced:** LAMBDA, LET (arithmetic simulations, NOT real function calls)
   - **TOTAL: 6 functions with FAKE tests** - UNACCEPTABLE for FP&A tool!

3. **76% (122/159) functions lack roundtrip validation** against LibreOffice/Gnumeric
4. **Array functions (5/5) have NO E2E tests** - explicitly stated as out of scope

---

## FUNCTIONS WITH FULL COVERAGE (unit + E2E + roundtrip): ~22/159

✅ **Math:** ABS, CEILING, EXP, FLOOR, LN, MOD, POWER, ROUND, SQRT
✅ **Aggregation:** AVERAGE, COUNT, COUNTA, MAX, MIN, PRODUCT, SUM
✅ **Date:** DAY, DAYS, MONTH, YEAR
✅ **Logical:** IF
✅ **Statistical:** MEDIAN, VAR.S

---

## CRITICAL GAPS - FAKE/SIMULATED TESTS (P0 PRIORITY)

**These functions claim E2E coverage but use HARDCODED VALUES:**

### Financial (4 functions)
- **IRR** - `formula: "=ROUND(0.088 * 100, 2)"` - Comment says "Simulated IRR"
- **MIRR** - `formula: "=ROUND(0.135 * 100, 2)"` - Comment says "Simulated MIRR"
- **XNPV** - Uses NPV instead (different function!)
- **XIRR** - No real test, uses placeholder arithmetic

### Advanced (2 functions)
- **LAMBDA** - `formula: "=5*5"` - Comment says "Simulates LAMBDA(x, x*x)(5)"
- **LET** - `formula: "=10*10+5"` - Comment says "Simulates LET(x,10,x*x+5)"

**IMPACT:** For a Financial Planning & Analysis tool, having fake tests for IRR/MIRR/XNPV/XIRR is a CRITICAL failure. These are core financial functions.

---

## FUNCTIONS MISSING E2E YAML TESTS

### Array (5/5 - 100% gap!)
**EXPLICITLY EXCLUDED from E2E scope:**
- FILTER
- RANDARRAY
- SEQUENCE
- SORT
- UNIQUE

*Comment in e2e_array_complete.yaml:* "YAML E2E tests focus on scalar-returning functions (COUNTUNIQUE)"

### Information (11/13 - 84% gap!)
- ISBLANK, ISERROR, ISEVEN, ISFORMULA, ISLOGICAL
- ISNA, ISNUMBER, ISODD, ISREF, ISTEXT
- NA

### Logical (6/9 - 66% gap!)
**Core operators missing:**
- AND
- OR
- NOT
- TRUE, FALSE
- XOR

### Lookup (7/12 - 58% gap!)
- ADDRESS, COLUMN, ROW, ROWS, COLUMNS
- HLOOKUP, INDIRECT, OFFSET, VLOOKUP

### Date (6/21 - 28% gap!)
- D, M, Y (shorthand functions)
- DATE, TODAY, NOW
- EDATE, EOMONTH, WORKDAY

### Aggregation (3/13 - 23% gap!)
- LARGE
- RANK.EQ
- SMALL

### Statistical (2/11 - 18% gap!)
- STDEVP
- VARP

### ForgeNative (3/8 - 37% gap!)
- MD, YD, YM (date period functions)

**VERIFIED COVERAGE (not gaps):**
- **Text:** All 15 functions DO have E2E (wrapped in LEN for verification) ✅
- **Financial:** PMT, PV, FV, NPV, RATE, NPER, SLN, DDB, DB have REAL E2E tests ✅
- **Trigonometric:** All 9 functions have E2E ✅
- **Math:** All 19 functions have E2E ✅

---

## FUNCTIONS MISSING ROUNDTRIP TESTS: 122/159

### Categories with 100% Roundtrip Gap:
- **Information:** 13/13 (100%)
- **Lookup:** 13/13 (100%) - Including INDEX, MATCH!
- **Conditional:** 8/8 (100%) - SUMIF, COUNTIF, etc.
- **Array:** 5/5 (100%)
- **Text:** 15/15 (100%)
- **ForgeNative:** 8/8 (100%)

### Categories with High Roundtrip Gap:
- **Trigonometric:** 9/11 (81%)
- **Date:** 14/21 (66%)
- **Aggregation:** 7/13 (53%)
- **Math:** 9/19 (47%)
- **Financial:** 7/20 (35%)

**IMPACT:** Roundtrip tests validate calculations against LibreOffice/Gnumeric. Missing these means no external validation.

---

## RECOMMENDATIONS

### IMMEDIATE (P0 - FP&A Mandate Violations)

1. **🚨 REPLACE FAKE TESTS WITH REAL TESTS:**
   ```
   IRR - Currently: =ROUND(0.088 * 100, 2)
         Required: =IRR(cash_flows.values)

   MIRR - Currently: =ROUND(0.135 * 100, 2)
          Required: =MIRR(cash_flows.values, 0.10, 0.12)

   XNPV - Currently: Uses NPV (wrong function!)
          Required: =XNPV(0.09, cash_flows.values, cash_flows.dates)

   XIRR - Currently: No real test
          Required: =XIRR(cash_flows.values, cash_flows.dates)

   LAMBDA - Currently: =5*5 (arithmetic)
            Required: =LAMBDA(x, x*x)(5)

   LET - Currently: =10*10+5 (arithmetic)
         Required: =LET(x, 10, x*x+5)
   ```

2. **Add E2E tests for core logical operators:**
   - AND, OR, NOT are fundamental boolean operations
   - Currently only tested within IF statements

3. **Add E2E tests for Array functions:**
   - Requires schema support for array-output formulas
   - Or test via scalar functions that consume arrays

4. **Add E2E tests for Information functions:**
   - IS* functions essential for data validation
   - 84% gap (11/13 missing)

### HIGH PRIORITY (P1)

5. **Add roundtrip tests for high-value categories:**
   ```
   Priority 1: Conditional (SUMIF, COUNTIF, AVERAGEIF, etc.)
   Priority 2: Text (all 15 functions)
   Priority 3: Lookup (INDEX, MATCH, VLOOKUP, etc.)
   Priority 4: Information (IS* functions)
   ```

6. **Complete roundtrip coverage for existing E2E tests:**
   - Many functions have E2E but no roundtrip validation
   - This is "low-hanging fruit" for coverage improvement

### MEDIUM PRIORITY (P2)

7. **Complete roundtrip coverage for:**
   - Trigonometric functions (81% gap)
   - Date functions (66% gap)
   - Math functions (47% gap)

8. **Document why Array functions excluded from E2E:**
   - Current comment says "focus on scalar-returning functions"
   - Need formal ADR for this decision

---

## COVERAGE METRICS

| Test Type | Coverage | Gap | Status |
|-----------|----------|-----|--------|
| **Unit Tests** | 159/159 (100%) | 0 | ✅ COMPLETE |
| **E2E YAML** | ~87/159 (54%) | 72 | ⚠️ NEEDS WORK |
| **E2E YAML (REAL)** | ~81/159 (51%) | 78 | ❌ 6 FAKE TESTS |
| **Roundtrip** | 37/159 (23%) | 122 | ❌ CRITICAL GAP |
| **Full Coverage** | ~22/159 (14%) | 137 | ❌ URGENT |

**FAKE/SIMULATED TESTS:** 6 functions (IRR, MIRR, XNPV, XIRR, LAMBDA, LET)

---

## DETAILED BREAKDOWN BY CATEGORY

| Category | Total | Unit | E2E | E2E (Real) | Roundtrip | Full Coverage |
|----------|-------|------|-----|------------|-----------|---------------|
| Math | 19 | 19 ✅ | 19 ✅ | 19 ✅ | 10 | 10 |
| Aggregation | 13 | 13 ✅ | 10 | 10 ✅ | 6 | 6 |
| Logical | 9 | 9 ✅ | 3 | 3 ✅ | 4 | 1 |
| Text | 15 | 15 ✅ | 15 ✅ | 15 ✅ | 0 ❌ | 0 ❌ |
| Date | 21 | 21 ✅ | 15 | 15 ✅ | 7 | 4 |
| Lookup | 12 | 12 ✅ | 5 | 5 ✅ | 0 ❌ | 0 ❌ |
| Financial | 20 | 20 ✅ | 13 | 9 ⚠️ | 13 | 9 ⚠️ |
| Statistical | 11 | 11 ✅ | 7 | 7 ✅ | 3 | 2 |
| Trigonometric | 11 | 11 ✅ | 11 ✅ | 11 ✅ | 2 | 2 |
| Information | 13 | 13 ✅ | 2 | 2 ✅ | 0 ❌ | 0 ❌ |
| Conditional | 8 | 8 ✅ | 8 ✅ | 8 ✅ | 0 ❌ | 0 ❌ |
| Array | 5 | 5 ✅ | 0 ❌ | 0 ❌ | 0 ❌ | 0 ❌ |
| Advanced | 3 | 3 ✅ | 3 | 1 ⚠️ | 0 ❌ | 0 ❌ |
| ForgeNative | 8 | 8 ✅ | 5 | 5 ✅ | 0 ❌ | 0 ❌ |

**Legend:**
✅ Complete coverage
⚠️ Has fake/simulated tests
❌ Critical gap

---

## CONCLUSION

**FP&A Compliance Status: FAILED**

While unit test coverage is 100%, the project has:
- **6 fake/simulated tests** masquerading as real E2E tests
- **76% of functions** lack external validation (roundtrip tests)
- **100% gaps** in critical categories (Array, Information, Lookup, Conditional, Text for roundtrip)

**For a Financial Planning & Analysis tool, this level of test coverage is UNACCEPTABLE.**

**Minimum acceptable standards:**
1. ✅ 100% real E2E tests (no simulations)
2. ❌ 80%+ roundtrip validation (currently 23%)
3. ❌ No fake tests (currently 6 fake tests)

**Estimated work to achieve compliance:**
- Replace 6 fake tests: 2-3 days
- Add 122 roundtrip tests: 10-15 days
- Add missing E2E tests: 5-7 days

**TOTAL: 3-4 weeks of focused testing work**

---

## FILES ANALYZED

- `/Users/rex/src/royalbit/forge/src/functions/registry.rs` - All 159 function definitions
- `/Users/rex/src/royalbit/forge/src/core/array_calculator/evaluator/*.rs` - Unit test files
- `/Users/rex/src/royalbit/forge/tests/e2e_*_complete.yaml` - 15 E2E YAML test files (3,247 total lines)
- `/Users/rex/src/royalbit/forge/tests/e2e_libreoffice_tests.rs` - Roundtrip tests (31 test functions)
