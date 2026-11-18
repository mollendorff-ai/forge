# ADR-065: How Two Numbers Compare

**Status:** Accepted by merging (proposed and built in one pull request, as ADR-064 was); the Excel behaviour it follows was taken from Microsoft's documentation and Q&A answers, and was **checked in Excel for iPhone on 2026-11-18**: Excel gives Forge's answer to all 13 formulas below (roadmap T1-EXCEL-EQUALITY)
**Date:** 2026-11-17
**Decided by:** the maintainer asked for roadmap item T1-EQUAL-NEAR-ZERO ("go with T1-EQUAL-NEAR-ZERO", 2026-11-17), without Excel at hand; the design is Claude's, in a session configured for Opus 5.5
**Closes:** roadmap T1-EQUAL-NEAR-ZERO, found by the generated `HLOOKUP` cases of ADR-062

---

## Context

Before this ADR, Forge compared two numbers three different ways:

- **`=` and `<>`** (`values_equal`): equal within an absolute 1E-10. So `0.1 + 0.2 = 0.3` was TRUE, but so were `-1E-12 = 0`, `1E-12 = 2E-12` and `1E-300 = 0`. Meanwhile `1E15 + 0.2 = 1E15` was FALSE, though the two differ in their 16th digit.
- **`<`, `<=`, `>`, `>=`**: exactly. So `0.1 + 0.2 = 0.3` and `0.1 + 0.2 > 0.3` were both TRUE, and `0.1 + 0.2 <= 0.3` was FALSE: a number equal to another and greater than it.
- **The exact lookups** (`MATCH` 0, `XMATCH`, `XLOOKUP`, `VLOOKUP` and `HLOOKUP` with FALSE) used `values_equal`, so `MATCH(0.1 + 0.2, {0.3}, 0)` found 0.3. `RANK` checked that its number was in the list within 1E-10.
- **The criteria of `COUNTIF` and its family**: equal within `f64::EPSILON` (2.2E-16) absolute, `<` and `>` exactly. So `COUNTIF` counted 0.3 for `0.1 + 0.2` (a gap of 5.6E-17), but not 110 for `100 * 1.1` (a gap of 1.4E-14).

The other engines measured on 2026-11-17 (`validation/gnumeric/tests/edge/edge_equality.yaml`):

- **Gnumeric 1.12.56** compares exactly everywhere: `0.1 + 0.2 = 0.3` is FALSE.
- **LibreOffice 26.2** compares the six operators and the criteria to about 15 digits (a relative tolerance), all six alike. Its `MATCH` and `VLOOKUP` are exact. Its `XLOOKUP` is exact over an array constant but not over a range of cells.

**Excel**, as documented:

- Microsoft's article "Floating-point arithmetic may give inaccurate results in Excel" says that `IF(A1=C1)` is TRUE for two values that differ only past the 15th digit. A Microsoft Q&A answer says the `=` operator does not compare the internal binary values but the values rounded to 15 significant digits. A Microsoft Tech Community answer says the same for the comparison operators (`=`, `<` and the others) and for `COUNTIF[S]`.
- A Microsoft Q&A thread ("VLOOKUP, XLOOKUP, INDEX/MATCH Issue Finding Exact Match Among Calculated Values", 2025) reports exact lookups giving #N/A for calculated values off in digits Excel does not show. The fix it gives is `ROUND`. So the exact lookups compare the binary values.

## Decision

1. **The comparison operators compare two numbers rounded to 15 significant digits, all six alike.** This covers `=`, `<>`, `<`, `<=`, `>` and `>=`, a date against its day number, and `SWITCH` (which matches as `=` does, as in LibreOffice).
   - Each number is rounded to 15 digits relative to its own size, by `core::precision::to_15_digits`, the rounding Forge already uses for the numbers it writes. So there is no tolerance near zero: `-1E-12 = 0` is FALSE and `-1E-12 < 0` is TRUE.
   - The rule rounds each number. It does not measure the gap: two numbers one binary step apart on either side of a 15th digit's halfway point are unequal.
   - `core::precision::excel_order` is the one function, with a shortcut for numbers further apart than 1E-13 of the larger. A test checks that the shortcut gives what rounding both would.
2. **The number criteria of `COUNTIF`, `SUMIF`, `AVERAGEIF`, the `...IFS` functions, `MAXIFS` and `MINIFS` compare by the same rule.** This applies to `=`, `<>`, `<`, `<=`, `>` and `>=` in a criterion, and to a criterion that is a number.
3. **The exact lookups compare numbers exactly** (IEEE equality, `core::precision::same`). This covers `MATCH` 0, `XMATCH` and `XLOOKUP` in their exact modes, and `VLOOKUP` and `HLOOKUP` with FALSE, a date against its day number included. Text compares as before, ignoring case, with wildcards. The approximate searches (`MATCH` 1 and -1, the next smaller or larger) order numbers exactly, as before.
4. **`RANK` finds its number in the list exactly**, as an exact lookup does, as LibreOffice does. Counting the values above it was already exact.
5. **Arithmetic does not change.** Excel sets a final subtraction that is very close to zero to zero (`=0.1+0.2-0.3` is 0 in LibreOffice too); Forge gives 5.55E-17. That is a rule about arithmetic, not comparison, and is left for its own item.

## Checked in Excel

The rule came from documentation. These are the cases where the engines differ, or the documentation leaves the answer open. They were checked in the Excel app for iPhone on 2026-11-18, in a workbook that calculated each formula and compared it with Forge's answer, read from a screenshot of the sheet (`0.3` was in A1:A2 of a second sheet). Excel gives Forge's answer to all 13.

| Formula | Forge | Excel (iPhone, 2026-11-18) | LibreOffice 26.2 | Gnumeric 1.12 |
| --- | --- | --- | --- | --- |
| `=0.1+0.2=0.3` | TRUE | TRUE | TRUE | FALSE |
| `=0.1+0.2<=0.3` | TRUE | TRUE | TRUE | FALSE |
| `=0.1+0.2>0.3` | FALSE | FALSE | FALSE | TRUE |
| `=1+1E-15=1` | TRUE | TRUE | TRUE | FALSE |
| `=1+1E-14=1` | FALSE | FALSE | FALSE | FALSE |
| `=1E15+1=1E15` | TRUE | TRUE | FALSE | FALSE |
| `=1E15+0.2=1E15` | TRUE | TRUE | TRUE | FALSE |
| `=-1E-12=0` | FALSE | FALSE | FALSE | FALSE |
| `=MATCH(0.1+0.2,{0.3},0)` | #N/A | #N/A | #N/A | #N/A |
| `=XLOOKUP(0.1+0.2,A1:A2,A1:A2)` with 0.3 in A1:A2 | #N/A | #N/A | 0.3 | (none) |
| `=COUNTIF(A1:A2,0.1+0.2)` with 0.3 in A1:A2 | 2 | 2 | 2 | 0 |
| `=SWITCH(0.1+0.2,0.3,1,0)` | 1 | 1 | 1 | 0 |
| `=RANK(0.1+0.2,A1:A2)` with 0.3 in A1:A2 | #N/A | #N/A | #N/A | 1 |

`1E15+1=1E15` is the one where Forge follows the documented rule against both engines, and Excel agrees with Forge: `1000000000000001` has 16 significant digits, and to 15 it is `1E15`. `XLOOKUP` over a range of cells gives #N/A in Excel, as in Forge, where LibreOffice finds 0.3.

The same sheet had the two subtractions of point 5: Excel gives `=0.1+0.2-0.3` as 0 (Forge 5.55E-17) and `=1E15+0.2-1E15` as 0 (Forge 0.25), so Excel sets a final subtraction near zero to zero. Forge does not; that is a difference from Excel in arithmetic, not in comparison, and roadmap T1-EXCEL-SUBTRACTION decides it.

## Consequences

- One rule for the six operators, so a number is never both equal to another and greater than it.
- No answer near zero comes from a tolerance: a number is zero only if it is zero.
- `MATCH(x, range, 0)` on calculated values can now give #N/A where it found a value before, as in Excel. A model that relied on the old tolerance needs `ROUND`, as an Excel user does.
- Every result of the examples and of the 2,138 cases of `validation/gnumeric` is unchanged, as are the generated cases of ADR-062 (27,000 of the weekly seed: 0 failed). The `equal-near-zero` rule in `validation/sweep/known.py` is removed: the exact lookups now agree with both engines.
- The new cases are `validation/gnumeric/tests/edge/edge_equality.yaml`. Gnumeric compares exactly, so those that depend on the 15 digits are judged by LibreOffice, or checked against Forge only where neither engine gives Excel's answer.
