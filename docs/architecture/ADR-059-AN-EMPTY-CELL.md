# ADR-059: An Empty Cell in a Number Column

**Status:** Proposed (2026-11-12); not built (roadmap T2-MISSING-VALUES)
**Date:** 2026-11-12
**Decided by:** the maintainer (an ADR before a number column may hold an empty cell, agreed after the FP&A audit of 2026-11-12); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-053 and ADR-054 (the model format's versions and what a column holds)

---

## Context

A number column cannot hold an empty cell:

```yaml
months:
  actual_units: [985, 1032, 1001, null, null]
```

```
Column 'actual_units' row 3: null values not allowed in numeric arrays. Use 0 or remove the row if the value is missing.
```

So `examples/rolling-forecast` holds 0 for the months that have no actual yet, and a month whose actual really is 0 looks the same. It needs `is_actual: "=month <= inputs.last_actual"`, where Excel would ask whether the cell is empty.

**`forge import` gives a wrong answer today.** An empty cell inside a column of numbers is imported as 0, without a warning. A workbook column of 10, empty, 30, empty, 0 becomes `[10, 0, 30, 0, 0]`, so `AVERAGE` is 8 in Forge and 13.33 in Excel (rc.19, checked 2026-11-12).

**What an empty cell does, measured.** A workbook with the column 10, empty, 30, empty, 0 and 50 formulas, recalculated by Gnumeric 1.12 and LibreOffice 26.2 on 2026-11-12. The two agree on every formula (to the 15th digit):

| Formula | Engines | Forge's evaluator today, with `Null` |
| --- | --- | --- |
| `=A3` (an empty cell) | 0 | `Null` |
| `=A3 + 1`, `=A3 * 5` | 1, 0 | error |
| `=A3 = 0`, `=A3 = ""` | TRUE, TRUE | FALSE, FALSE |
| `ISBLANK(A3)`, `ISNUMBER(A3)` | TRUE, FALSE | same |
| `IF(A3, 1, 2)`, `"[" & A3 & "]"`, `LEN(A3)` | 2, `[]`, 0 | same |
| `SUM`, `COUNT`, `COUNTA` | 40, 3, 3 | same |
| `AVERAGE`, `MEDIAN`, `STDEV`, `PERCENTILE(…, 0.5)` | 13.33, 10, 15.28, 10 | same |
| `MIN`, `MAX`, `PRODUCT`, `LARGE(…, 3)`, `SMALL(…, 1)`, `RANK(30, …)` | 0, 30, 0, 0, 0, 1 | same |
| `SUMPRODUCT(x, x)` | 1000 | same |
| `COUNTIF(x, "")`, `COUNTIF(x, "=")`, `COUNTIF(x, "<>")`, `COUNTIF(x, 0)` | 2, 2, 3, 1 | same |
| `SUMIF(x, ">=0")`, `AVERAGEIF(x, ">=0")`, `COUNTIFS(x, "")` | 40, 13.33, 2 | same |
| `NPV(0.1, x)` | 33.88: the empty cells are skipped, so 30 is discounted one year, not two | same |
| `COUNTBLANK(x)` | 2 | unknown function |
| `=A3 <> 0`, `=A3 <> ""` | FALSE, FALSE | (not measured) |
| `INDEX(x, 2)` | 0 | `Null` |
| a column of empty cells: `SUM`, `COUNT`, `AVERAGE`, `MAX`, `MIN` | 0, 0, #DIV/0!, 0, 0 | -0, 0, error, error, (not measured) |

The evaluator already has `Value::Null`, and the aggregates skip it as Excel does, because they collect only numbers. What the model format lacks is a way to write one, and the evaluator gets seven things wrong once it can be written.

The number column is `ColumnValue::Number(Vec<f64>)`, matched in 328 places in 40 files.

## Decision

1. **`null` (or `~`) in a number column is an empty cell.** This applies only to a column of data. The column's type comes from its first value that is not null, and a column of only nulls is a number column. A model that has one says `_forge_version: "5.3.0"`. In 5.1.0 and 5.2.0, null stays the error it is now, and the message names 5.3.0. So an older Forge refuses a file with an empty cell because of its version, instead of with a message about nulls. If T2-SCALAR-DATES or T2-NUMBER-FORMAT change the format before 5.3.0 is released, they share that version.

2. **An empty cell does what Excel does with it.** The table above is the specification. Its "Engines" column is what Forge gives, and each row is a case in the Gnumeric suite. The changes to the evaluator:
   - in arithmetic an empty cell is 0, in text `""`, and in a condition FALSE;
   - `= 0` and `= ""` are TRUE for it, and `<> 0` and `<> ""` are FALSE;
   - `COUNTBLANK` is added;
   - `MAX`, `MIN`, `SUM` and the like of a column of only empty cells are 0 (not an error, not -0), and `AVERAGE` of one stays an error (#DIV/0!).

   The aggregates already skip empty cells and do not change. Every other function in the list gets one case with an empty cell, and Forge must give the engines' answer.

3. **A formula never gives an empty cell.** A formula whose value is an empty cell (`=actual`, `=INDEX(actual, 2)`) gives 0, in a column as in a scalar, as in Excel. Empty cells exist only in data. So every calculated column stays `Number`, and so does everything downstream of a calculation: the analytics commands, Monte Carlo, the sensitivity tables.

4. **A column with an empty cell is a variant of its own: `ColumnValue::Blanks(Vec<Option<f64>>)`.** A number column without a null stays `Number`, so a model without null gives the same results and the same output, byte for byte. A new variant makes the compiler stop at every exhaustive `match`. Each of these decides what an empty cell is there; most only print it or hand it to the evaluator. The arms that catch everything else (`_ =>`) are listed in the pull request that builds this and checked one by one.

   Rejected:
   - **NaN for an empty cell.** `.nan` is already a value a column can hold, and it is an error in a result (`nan_input_tests`). An empty cell would be indistinguishable from it.
   - **A list of empty positions beside the numbers.** Code that does not know about the list would read the placeholder as a number, which is exactly the silent 0 import gives today.
   - **`Option<f64>` in every number column.** That changes all 328 places and every model pays for it.

5. **How an empty cell is shown, exported and imported.**
   - `forge calculate` prints `[985, 1032, null, null]`; `forge report` and Forge Studio's grid show an empty cell.
   - JSON output (`--format json`, MCP) gives `null`.
   - `forge calculate` writes the data back with the nulls where they were.
   - `forge export` writes an empty cell.
   - `forge import` reads an empty cell inside a column of numbers as null, and writes `_forge_version: "5.3.0"` when there is one. As now, the empty cells at the end of a shorter column are trimmed.

6. **Until this is built, `forge import` says what it does.** An empty cell imported as 0 gets a warning that names the cells and says that `AVERAGE`, `COUNT` and `MIN` will differ from Excel. This is a small change on its own, released before the rest. It is a fix of a silent wrong answer, so by docs/STABILITY.md it restarts the two weeks before a stable release.

7. **`NPV` and `IRR` follow Excel and skip an empty cell.** That shifts the later cash flows a period earlier, which is easy to miss. Both engines measured do this for `IRR` too (-100, empty, 60, 60 gives the IRR of -100, 60, 60: 13.07%). `forge functions` says so for `NPV` and `IRR`, and `forge audit` warns when one of them reads a column with an empty cell. `XNPV`, `XIRR` and `MIRR` with an empty cell are measured when this is built, and follow the engines where they agree. Following Excel matters more here than a rule of Forge's own, because the export has to recalculate to the same number.

## Consequences

**What gets better**

- The rolling forecast holds null for the months not yet closed, and `is_actual` can be `=ISNUMBER(actual_standard_units)`. A month with an actual of 0 is a 0.
- A workbook with empty cells imports to the model Excel computes: `AVERAGE` and `COUNT` give Excel's answers, not the ones a 0 would give.
- `ISBLANK` and `COUNTBLANK` mean something in Forge.

**What it costs**

- A new format version, 5.3.0: the schema, the parser's version check, the VS Code schema test and the docs of the format.
- Every exhaustive `match` on `ColumnValue` gets an arm, and every wildcard arm is checked. That is the largest part of the work, and it is mechanical.
- Seven evaluator changes (decision 2), each of which could change a result. None changes a model that works today, because an empty cell cannot be written today, except where a function is given the result of `INDEX` or a lookup that returns `Null`. The Gnumeric suite and the unit tests show whether any such result changes.

**Acceptance**

- The rolling forecast's four actual columns hold null for the open months, `is_actual` is `=ISNUMBER(actual_standard_units)`, and its numbers do not change (`tests/demo_examples.rs`).
- For each row of the table above, a case in `validation/gnumeric` with an empty cell, and Forge gives the engines' answer before export, after export and after import.
- A model without null gives byte-identical output to rc.19 for `calculate`, `--format json` and `export` (apart from the time), on every file in `examples/`.
- `forge import` of the measured workbook gives `[10, null, 30, null, 0]` and `AVERAGE` 13.33.

**Not decided here**

- An empty cell in a column of text, dates or booleans. Excel's rules differ again: a reference to an empty cell in a text column is 0, not `""`. Those columns keep refusing null.
- An empty scalar. `value: null` already means "not calculated yet".
- Whether a formula may return an empty cell on purpose. Excel cannot, and this ADR keeps it that way.
