# ADR-058: Aggregates in a Row Formula

**Status:** Accepted (2026-11-12); built
**Date:** 2026-11-12
**Decided by:** the maintainer (an ADR before a table column may aggregate another table, agreed after the FP&A audit of 2026-11-12; accepted by merging it, #174); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-053 (periods and the row above), for the rule of what a name means in a row formula

---

## Context

A table column whose formula aggregates stops with an error:

```yaml
quarters:
  quarter: [1, 2, 3, 4]
  revenue: "=SUMIFS(months.revenue, months.fiscal_quarter, quarter)"
```

```
Table 'quarters': Column 'revenue' uses aggregation formula - aggregations should be in scalars section
```

So `examples/rolling-forecast` writes its four quarters as four scalars by hand, one `SUMIFS` each. Excel users do this on an ordinary summary sheet: `=SUMIFS(Months!M:M, Months!H:H, A2)`, filled down. A share of the total (`=C2/SUM(C:C)`) is the same thing within one table, and Forge refuses that too.

The rule exists because of how a name is read in a row formula. While a row formula is evaluated, `months.revenue` and `revenue` are **the value in the current row**. An aggregate given that one value would aggregate one number: `SUM(revenue)` would be `revenue`, silently. Refusing every aggregate was the safe answer when the rule was written.

Forge already breaks this rule for some arguments. In a row formula, `INDEX`'s array, `MATCH`'s and `XMATCH`'s lookup array, `ROWS`'s argument, `XLOOKUP`'s lookup and return arrays and `FORECAST`'s known x and y values are read as **whole columns**, while their other arguments are read as the row's value. That is why the rolling forecast's `XLOOKUP(month, budget.month, budget.standard_units, 0)` works in a table of 19 months over a budget of 12. The export knows the same positions (`WHOLE_COLUMN_ARGS` in `src/excel/workbook_refs.rs`) and writes those arguments as absolute ranges (`'budget'!$A$2:$A$13`), and the import reads them back as `budget.month`. So the rule is not new. It is a list that covers lookups and not aggregates.

The list lives in three places that must agree: the evaluator (each lookup function clears the row by itself), the aggregation check (`expr_has_aggregation` in `src/core/array_calculator/mod.rs`) and the exporter's `WHOLE_COLUMN_ARGS`.

The evaluator clears the row by **copying the whole context**, every scalar and every table, on each call. A table of 4,000 rows with one `XLOOKUP` into a table of 4,000 rows takes 0.79 s, and 1,000 rows take 0.06 s (release build, 2026-11-12). The cost grows with the square of the size because of the copies, not the lookups: 16 million comparisons take a few hundredths of a second.

## Decision

1. **An argument that Excel reads as a range is a whole column in a row formula.** Every other argument is the row's value, as now. Which arguments are ranges is fixed per function and position, by the table below. A name in a range position means the whole column whether it names another table (`months.revenue`) or the row's own table (`revenue`).

   | Functions | Whole column | Row's value |
   | --- | --- | --- |
   | `SUM`, `AVERAGE`, `AVG`, `MAX`, `MIN`, `COUNT`, `MEDIAN`, `MODE`, `MODE.SNGL`, `GEOMEAN` with one argument | the argument | |
   | `SUMIF`, `COUNTIF`, `AVERAGEIF` | 1st (range), 3rd (sum or average range) | 2nd (criterion) |
   | `SUMIFS`, `AVERAGEIFS`, `MAXIFS`, `MINIFS` | 1st, and each criteria range (2nd, 4th, …) | each criterion (3rd, 5th, …) |
   | `COUNTIFS` | each criteria range (1st, 3rd, …) | each criterion (2nd, 4th, …) |
   | `SUMPRODUCT`, `VAR`, `VAR.S`, `VAR.P`, `STDEV`, `STDEV.S`, `STDEV.P` | every argument | |
   | `PERCENTILE`, `PERCENTILE.INC`, `PERCENTILE.EXC`, `QUARTILE`, `QUARTILE.INC`, `QUARTILE.EXC` | 1st | 2nd (k or the quartile) |
   | `CORREL`, `SLOPE`, `INTERCEPT`, `RSQ`, `COVARIANCE.S`, `COVARIANCE.P` | both | |
   | `FORECAST`, `FORECAST.LINEAR` | 2nd, 3rd (as today) | 1st (x) |
   | `INDEX`, `MATCH`, `XMATCH`, `ROWS`, `XLOOKUP` | as today | as today |

   So a table of quarters is `=SUMIFS(months.revenue, months.fiscal_quarter, quarter)`, a share is `=revenue / SUM(revenue)`, and a table of percentiles is `=PERCENTILE.INC(results.npv, p)`, with a different `p` in each row.

2. **With several arguments, `SUM`, `AVERAGE`, `MAX`, `MIN`, `COUNT`, `MEDIAN`, `MODE` and `GEOMEAN` stay row by row.** `=MAX(0, ebit)` is the maximum of two numbers in each row, as documented, and that does not change. This is where Forge and Excel differ, and the difference is old: in Excel `=MAX(0, C:C)` is the maximum of the whole column. A model that wants the whole column writes one argument. The error for `MAX(0, months.revenue)` in a table of another length (a row-count mismatch) stays, and its message gains a hint: "to aggregate the whole column, give the aggregate one argument".

3. **The "aggregations should be in scalars section" error goes away** for every function in the table. A formula that aggregates its own column (`revenue: "=SUM(revenue)"`) is a cycle and fails as one, naming the column. An aggregate of a column of the same `PREV` group (ADR-053) fails with the message that the recursive evaluator already gives ("needs a whole column first").

4. **One list, read by the evaluator, the dependency order and the exporter.** The table above becomes one function in the calculator: given a function name, the number of arguments and a position, it says whether that argument is a whole column. `expr_has_aggregation` stops being a gate. The exporter's `WHOLE_COLUMN_ARGS` is replaced by the same function. A unit test runs every row of the table through all three.

5. **A whole column is read without copying the context.** The evaluator gets a way to evaluate one argument with no current row that borrows the context, such as a field on the context that hides the row for the length of one call. It does not build a new context. The lookups move to it in the same change, so the 4,000-row `XLOOKUP` gets faster too. Target: a table of 5,000 rows with a `SUMIFS` over a table of 5,000 rows in under 1 s in a release build, with a test.

6. **Export and import.** An argument in a range position is written as the absolute range of its column, `'months'!$M$2:$M$20`, as the scalars and the lookups already are. The import reads an absolute range that is exactly a table sheet's data column back as `table.column`, as it does for `XLOOKUP`, and a row reference (`M2`) back as the row's name. A share of the total exports as `=M2 / SUM('months'!$M$2:$M$20)`.

7. **No model changes its answer.** Every formula this ADR makes work is refused today. Functions not in the table keep what they do now. In particular, `NPV`, `IRR` and the other functions that take a range of cash flows still read the row's value in a row formula. Making them read a whole column would change the result of a model that works today, so it needs a decision of its own. It is not part of this ADR.

## Consequences

**What gets better**

- A table of quarters or years is a table, not one scalar per period by hand. The rolling forecast's four quarter scalars become a table of four rows, and the same table can roll up next year without new formulas.
- A share of the total, a running rank (`=COUNTIF(revenue, ">" & revenue) + 1`) and a percentile table each become one row formula.
- One list instead of three that must agree. The export of a lookup and the export of an aggregate cannot drift apart.
- Row lookups no longer copy the model on every call.

**What it costs**

- A row formula's meaning depends on the position of a name, not only on the name. `SUMIFS(months.revenue, months.fiscal_quarter, quarter)` reads `months.revenue` whole and `quarter` in the row. That is what Excel does with a range and a cell, but a reader of the YAML has to know the table. The docs (03-FORMULA-EVALUATION, FEATURES, the `forge functions` help for each function in the table) say so, with the quarters example.
- `MAX(0, months.revenue)` and `MAX(months.revenue)` mean different things in a row formula, as they do today in a scalar. Decision 2 keeps the old rule instead of following Excel, because following Excel would change every model with `=MAX(0, ebit)`.
- An aggregate in each row of an N-row table over an M-row table costs N × M. The 5,000 × 5,000 target is checked by a test. Beyond it, there is no index on criteria columns, and this ADR does not add one.

**Acceptance** (the roadmap item's, made precise)

- `examples/rolling-forecast` has a table `quarters` with `revenue` and `budget` made with `SUMIFS` over `months`. Its values equal the four quarter scalars, which are then removed. `tests/demo_examples.rs` checks it against the Python already there.
- The exported workbook recalculates to the same values in Gnumeric and LibreOffice: new cases in `validation/gnumeric`, one per row of the table in decision 1, including an own-table share and a per-row percentile.
- `forge import` of that workbook gives the `SUMIFS` formulas back as written.
- Each error in decisions 2 and 3 has a test with its message.
- The 5,000 × 5,000 test passes in under 1 s, and the 4,000-row `XLOOKUP` takes less than half of today's 0.79 s.

**Not decided here**

- Whether `NPV`, `IRR`, `XNPV`, `XIRR` and `MIRR` read a whole column in a row formula (decision 7).
- A criteria index for large tables.
- Dynamic arrays (a formula that returns a column), which `TREND` and `GROWTH` would need.
