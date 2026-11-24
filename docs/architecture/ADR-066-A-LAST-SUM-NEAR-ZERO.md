# ADR-066: A Last Sum Near Zero Is Zero

**Status:** Accepted by merging; the rule is Excel's, as the Excel app for iPhone gives it on 69 formulas (2026-11-18)
**Date:** 2026-11-18
**Decided by:** the maintainer, who took the advice to follow Excel once ADR-065's Excel round found the difference, and ran the three sheets that pinned the rule down in Excel (2026-11-18); the design is Claude's, in a session configured for Opus 5.5
**Closes:** roadmap T1-EXCEL-SUBTRACTION

---

## Context

Binary arithmetic leaves noise: `0.1 + 0.2` is `0.30000000000000004`, so `0.1 + 0.2 - 0.3` is `5.551115123125783E-17`. ADR-065 made comparisons ignore noise past the 15th significant digit, but left the arithmetic alone (its point 5). Its Excel round found that Excel gives `=0.1+0.2-0.3` as 0 and `=1E15+0.2-1E15` as 0, where Forge gave 5.55E-17 and 0.25.

The difference matters most where a model checks itself. A balance check written as a cell of its own, `difference: =assets - liabilities - equity`, read by `=IF(difference = 0, "OK", "CHECK")`, says OK in Excel and CHECK in Forge when the sides agree but carry noise. So does a `SUM` of debits and credits that cancel. The comparison cannot hide it: 5.55E-17 is not 0 to 15 significant digits, and ADR-065 deliberately gives no tolerance near zero.

Microsoft's article "Floating-point arithmetic may give inaccurate results in Excel" says Excel corrects a final addition or subtraction near zero, but not when or how near. LibreOffice does something similar everywhere, below 16 units in the last place (`rtl::math::approxSub`). Neither is enough to write the rule from, so it was measured.

## The measurement

Three workbooks, each formula calculated by Excel for iPhone beside Forge's answer, read from screenshots on 2026-11-18. 69 formulas in all; 68 of them, with Excel's answers, are the tests of `final_sum` in `src/core/array_calculator/mod.rs` (the 69th is the typed number of point 4), and 13 are cases of `validation/gnumeric/tests/edge/edge_subtraction.yaml`.

| What | Excel |
| --- | --- |
| `=0.1+0.2-0.3`, `=0.5-0.4-0.1`, `=-0.3+0.1+0.2`, `=0.1+0.2+(-0.3)`, `=(0.1+0.2)-0.3`, `=0.1+(0.2-0.3)`, `=0.3-(0.1+0.2)`, the same with cells | 0 |
| `=(0.1+0.2-0.3)`, `=1*(0.1+0.2-0.3)`, `=(0.1+0.2-0.3)*1`, `=-(0.1+0.2-0.3)`, `=ABS(...)`, `=IF(TRUE, 0.1+0.2-0.3, 1)` | 5.55E-17 |
| `=0.1+0.2-0.3+0` | 5.55E-17: the last `+` adds 0 to 5.55E-17 and cancels nothing |
| `=SUM(0.1,0.2,-0.3)`, `=SUM(range)`, `=SUM(-0.3,0.1,0.2)`, `=1*SUM(0.1,0.2,-0.3)` | 0 |
| `=SUM(0.1,0.2,-0.3,0)`, `=SUMPRODUCT(...)`, `=SUMIF(...)` | 5.55E-17 |
| `=0.1+0.2-0.3=0`, `=IF(0.1+0.2-0.3=0,1,0)` | FALSE, 0: the subtraction is not the last step |
| A cell `=0.1+0.2-0.3`, then `=cell=0`, `=cell*1E20` | TRUE, 0: the cell holds 0 |
| `=1+5*2^-52-1`, `6*`, `7*`; the same near 512 and in `SUM` | 0 |
| `=1+8*2^-52-1`; the same near 512 and in `SUM` | 1.78E-15, kept |
| `=1E15+0.2-1E15`, `=1E15+0.5-1E15`, `=2^53+2-2^53` | 0 |
| `=1E15+1-1E15`, `=1E15+1.5-1E15`, `=1000+1E-12-1000`, `=1E-10+1E-25-1E-10` | 1, 1.5, 1.02E-12, 1.03E-25: kept |
| `=2-(2-14*2^-52)`: 7 units of 2, 14 of 1.99... | 0 |

A threshold relative to the operands cannot fit: `1+1E-15-1` (1.1E-15 relative) is 0 and `1E15+1-1E15` (1.0E-15 relative) is kept. Counted in units in the last place (ulps), every row fits: 1, 2, 4, 5, 6 and 7 are 0; 8, 9, 12 and more are kept, at every size.

## Decision

1. **The rule.** The result `r` of an addition or subtraction of `a` and `b` is 0 when `|r| < 8 × ulp(max(|a|, |b|))`, that is `|r| < 2^(e-49)` for `2^e ≤ max(|a|, |b|) < 2^(e+1)` (`precision::final_sum`). Only numbers that nearly cancel can fall below it; adding 0, or numbers of one sign, never does. A subnormal operand has no such unit, and nothing is changed.
2. **Where.** At the last operation of a cell's formula, when it is a `+` or `-` and the formula is not wrapped in one pair of parentheses (`evaluator::evaluate_cell`): a scalar's formula, each row of a row formula, and each row of a column that reads the row above (`PREV`). An array result is zeroed element by element. Not inside parentheses, another operation or a function argument.
3. **`SUM`.** `SUM` adds left to right, as before, and its last addition follows the rule wherever `SUM` is, inside another formula too. `SUMPRODUCT`, `SUMIF`, `SUMIFS` and the other aggregates are unchanged, as in Excel.
4. **Not changed.** ADR-065's comparisons, every other operator and function, and a formula's value in full precision between cells. A number typed in a formula keeps all its digits: Excel cuts `1000000000000000.5` to 15 significant digits when the formula is entered, and Forge does not (written down, not changed here). *Note, 2026-11-24: Forge now reads a number typed in a formula, and text turned into a number, to 15 significant digits as Excel does, the rest dropped (roadmap T1-EXCEL-LITERAL-DIGITS, VALIDATION.md section 5); this ADR's rule is unchanged.*

## Consequences

- A difference or a `SUM` that cancels is 0, as in Excel, and so is everything calculated from it. This is a wrong answer fixed: by docs/STABILITY.md the two weeks before a stable release start again from the release that carries it.
- A formula that wants the noise writes it in parentheses, as in Excel: `=((a + b) - 418.33)` keeps 5.68E-14. The test of the in-place writer that showed the noise does that now.
- Every other result of the library's tests, of the examples, and of the 1,975 cases of `validation/gnumeric` that Gnumeric judges is unchanged (2026-11-18). Gnumeric 1.12 never sets a result to 0, so the new cases where Excel does are judged by LibreOffice; the LibreOffice cases run in CI, where LibreOffice is 24.8 or later.
- LibreOffice sets a result to 0 below 16 units, everywhere, inside parentheses too; Excel below 8, at the top level. Forge follows Excel. The generated cases of ADR-062 compare with both engines to a tolerance, so they cannot see the difference.
