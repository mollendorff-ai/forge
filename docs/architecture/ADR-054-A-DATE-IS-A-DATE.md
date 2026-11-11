# ADR-054: A Date Is a Date

**Status:** Accepted
**Date:** 2026-11-10
**Decided by:** the maintainer asked for date columns as real dates in Excel (roadmap T3-EXPORT-DATES) and left the design to Claude; the design is Claude Fable 5.2's

---

## Context

In a Forge model a date is the text `2026-01-31`. `forge export` wrote it to the workbook as that text. The roadmap item asked for real dates, for charts, filters and pivots, and warned about one thing: `LEFT(month, 4)` is `2026` in Forge and would be `4605` on Excel's day number.

Measuring before changing anything showed that text was not a safe place to stay. With dates as text in the workbook, these formulas gave one answer in Forge and another in both Gnumeric and LibreOffice, with no warning (`d` is a column of four dates, `v` of numbers):

| Formula | Forge | Excel, dates as text |
| --- | --- | --- |
| `=IF(d > DATE(2026, 6, 1), 1, 0)` | 0, 0, 1, 0 | 1, 1, 1, 1 (text is "greater" than any number) |
| `=IF(EDATE(d, 0) = d, 1, 0)` | 1, 1, 1, 1 | 0, 0, 0, 0 |
| `=MIN(t.d)`, `=MAX(t.d)`, `=SUM(t.d)` | the day numbers | 0 |
| `=COUNT(t.d)` | 4 | 0 |
| `=YEAR(MAX(t.d))` | 2026 | 1899 |
| `=SUMPRODUCT((t.d > DATE(2026, 2, 1)) * t.v)` | 90 | 100 |

A column calculated with `EDATE(d, 1)` was the text `2026-02-28` in Forge and the number 46081 in the workbook, shown as a number.

Forge itself was of two minds. A date was a number in `MAX(t.d)` over a column and ignored in `MAX(d, DATE(2026, 6, 1))` in a row; `COUNT(d, e, n)` counted only the number; `COUNTIF(t.d, ">2026-02-01")` counted nothing; `d = DATE(2026, 1, 31)` was FALSE for that very date; `TEXT(d, "yyyy")` was `"46053"`; and `d + 1` was `"2026-01-311"`, because `+` joined text whenever one side was text (`"5" + 1` was `"51"`, where Excel gives 6).

## Decision

1. **A date is a date.** In a model it is written `2026-01-31`, and that is its text. Wherever a formula reads a number, it is its day number, the days since 1899-12-30, which is Excel's number for every date from 1900-03-01 on. This was already so in arithmetic, in `<` and `>`, in the date functions and in aggregates over a column. It is now so everywhere:
   - `=` and `<>`: a date equals its day number (`d = DATE(2026, 1, 31)`). Other text is never equal to a number, as in Excel (`"5" = 5` is FALSE).
   - A single date in an aggregate counts as a number: `MAX(d, DATE(2026, 6, 1))`, `MIN`, `SUM`, `AVERAGE`, `COUNT(d, e, n)`.
   - A criterion compares dates: `COUNTIF(t.d, ">2026-02-01")`, `SUMIFS(t.v, t.d, ">=2026-02-01", t.d, "<=2026-02-28")`.

2. **`+` adds and `&` joins, as in Excel.** `"5" + 1` is 6, a date plus 1 is the next day's number, and text that is neither a number nor a date is `#VALUE!` with a message that names `&` and `CONCAT`. `&` is new in Forge formulas; it binds less than `+` and more than a comparison.

3. **`TEXT(value, format)` reads Excel's format codes**, a defined part of them: digits `0` and `#`, a point, `,` between digits, `%`, `E+00`, literals (also between digits, `0000-00-00`), and dates (`yyyy`, `mmm`, `dddd`...). Any other code (sections with `;`, times, fractions) is an error that says so. A format Forge does not know must not come out as a wrong string, as `"#,##0.00"` did (`1234.50`).

4. **A column of dates is written to the workbook as dates**: the day number, with the number format `yyyy-mm-dd`. A column is one of dates when every value is a full date from 1900-03-01 on, whether typed in or calculated (`EDATE(d, 1)`, a period column). A column of months (`2026-01`), of earlier dates, or of dates mixed with other text stays text, as before.

5. **A formula reads each date the way Forge reads it there:**

   | Where | Forge | Workbook |
   | --- | --- | --- |
   | a number is read | `=YEAR(d)`, `=e - d`, `=d + 1`, `=IF(e > d, 1, 0)` | the cell: `=YEAR(A2)` |
   | text is read | `=LEFT(d, 4)`, `=name & "-" & d`, `=ISNUMBER(d)` | `=LEFT(TEXT(YEAR(A2)*10000+MONTH(A2)*100+DAY(A2), "0000-00-00"), 4)` |
   | a date is written in the formula | `=IF(d > "2026-06-01", 1, 0)` | `=IF(A2 > DATE(2026, 6, 1), 1, 0)` |
   | a criterion | `=COUNTIF(t.d, ">2026-02-01")` | `=COUNTIF(range, ">" & DATE(2026, 2, 1))` |
   | a value is passed on | `=IF(v > 15, d, "2000-01-01")` | what the reader of the result reads: `=IF(C2 > 15, A2, DATE(2000, 1, 1))` |

   The text of a date is written with `TEXT(..., "0000-00-00")` over the year, month and day, not with `TEXT(A2, "yyyy-mm-dd")`: Excel reads date codes in the language it runs in (`aaaa-mm-jj` in French), and a workbook must give the same answer in Montreal as in Toronto. For the same reason a date in a criterion is its day number, not text for Excel to parse.

   A formula column that holds dates in Forge must give dates in the workbook, so its formula's outermost value must be a date (a date column, `EDATE`, `IF` of dates, `PREV`...). `=UPPER(d)` holds date text made by a text function: it is written as values.

6. **Text that holds a date is not read as a number in the workbook.** A column that is `n/a` in some rows and a date in others, or holds a date before 1900-03-01, stays text. Forge reads each date in it as its day number where a number is read (decision 1); Excel reads text there: greater than any number, skipped by `MAX` and `SUM`, equal to no number. So a formula that reads a number from such a column (`YEAR(maturity)`, `maturity > DATE(2026, 6, 1)`, `maturity = DATE(2026, 1, 31)`, `MAX(loans.maturity)`, a criterion with a date or a comparison) is written as its values, with a warning. One that reads its text is the same in both and stays a formula: `maturity = "n/a"`, `maturity = "2026-01-31"`, `LEFT(maturity, 4)`, `COUNTIF(loans.maturity, "n/a")`, `MATCH("2026-12-01", loans.maturity, 0)`.

   **Amended 2026-11-10 (roadmap T1-TEXT-NUMBERS): the same holds for any text with a digit in it.** A text column of numbers (`"5"`, `"1,200"`), of months (`2026-01`) or of codes is read as a number its own way by each: in a column Forge reads `"5"` as 5 where Excel reads text (`SUM` 7 and 0 over `["5", "apple", "2", "x"]`, `COUNTIF(">3")` 1 and 0), and Excel turns into a number text Forge does not (`"5%"`, `"2026-01"`). A column with a digit in some row is treated as above: a formula that reads a number from it is written as values, one that reads its text stays live. Two more rules close the rest:
   - **Ordering text is not written live**: `<`, `>`, `<=` or `>=` with text on either side. Excel puts any text above every number and orders text by its letters; Forge stops with an error, so under `IFERROR` the two give different answers (`IFERROR(IF(s > 3, 1, 0), -1)` is -1 for `apple` in Forge and 1 in Excel).
   - **Text made in a formula is not read as a number** (`LEFT(name, 2) * 1`): what it holds is known only once calculated. `VALUE(LEFT(name, 2))` says what is meant and stays a formula.

   A column of words (no digit in any row) reads the same in both: as a number it is nothing in `SUM` and `SUMPRODUCT`, and an error in arithmetic. Those formulas stay live. Forge's own calculation is not changed by this: the decision is about the workbook, which must never compute something else without saying so. Whether Forge itself should read `"5"` in a column as Excel does is a separate question, not decided here.

7. **A place with no rule is written as values, with a warning.** The rules cover what both engines confirm (the list is in `src/excel/workbook_refs/dates.rs`). A date read any other way (`SUMPRODUCT(t.d, t.v)`, a date looked up in text, a date inside a `LAMBDA`, a formula column that gives dates in some rows and numbers in others) is not guessed.

8. **`forge import` reads date cells as dates** (a cell with a date format and no time of day, whatever the format), and reads a formula over dates only if it is the same calculation in Forge: if Forge would write that very formula for some Forge formula, that Forge formula is what comes in. So `=LEFT(TEXT(YEAR(A2)*10000+..., "0000-00-00"), 4)` comes back as `=LEFT(d, 4)`, and `=A2 > DATE(2026, 6, 1)` as `=d > "2026-06-01"`. A workbook's own `=LEFT(A2, 4)` on a date cell is `4605` in Excel and would be `2026` in Forge: it is imported as its values, with a warning. `=A2 + 30` shown as a date is a number in Forge, the day number the cell holds. Decision 6 holds on the way in too: a workbook with `2026-01-31` in text cells (every Forge before this one wrote them so) has `=IF(A2 > DATE(2026, 6, 1), 1, 0)` at 1 in every row, and Forge would compare dates. It is imported as its values, with a warning; `=LEFT(A2, 4)` there is the same in both and comes in as a formula.

## Evidence

- **forge-e2e-gnumeric, `tests/edge/edge_real_dates.yaml`**: 68 cases over a date column, a period column and a text column that holds dates in some rows, with expected values from the same calculation in Python (`datetime`). Gnumeric recalculates 66 of them and LibreOffice 2 (`XLOOKUP`, `UNIQUE`). The 1,367 cases that were there pass unchanged.
- **Every rule was written against a measurement**: ten probe models with 469 formula cells, and the three shipped examples with a period column with 1,932 more. Each cell's recalculated value in LibreOffice, and whether it is text or a number, was compared with what Forge cached; Gnumeric recalculated the same workbooks. One cell differs in LibreOffice, and not because of dates: `YEARFRAC` from the last day of February to a 31st (roadmap T1-YEARFRAC-FEB). Five more differ in Gnumeric, and they are Gnumeric 1.12's own: `DATEDIF(..., "D")` (three cells) and `XIRR` on date cells give `#VALUE!` there and Forge's number in LibreOffice, and so does the formula that `COUNTUNIQUE` of a text column is written as (`MMULT` over `EXACT`). On the six of those models that the build before this change calculates (the others use `&`), the same check finds 57 different cells of 165.
- **Text that holds a date** (`n/a`, `2026-01-31`, `2026-12-01`, `none`), with its formulas live as rc.11 wrote them: `IF(maturity > DATE(2026, 6, 1), 1, 0)` is 1 in every row in both engines, and in Forge 0 for January, 1 for December and an error for the other two; `MAX` of the column is 0 in both and December's day number in Forge; `maturity = DATE(2026, 1, 31)` is 0 in every row in both and 1 for January in Forge, now that a date equals its day number.
- **`TEXT`**: 51 formats compared. Forge gives what both engines give in 44, and what one of them gives in 6 more, where the two differ and Forge follows Excel's rule (the digits shown are rounded from 15 significant digits). One is not confirmed by either: `TEXT(-0.001, "0.00")`, where Forge gives `-0.00` and both engines `0.00`.
- **Round trip**: export, import and calculate again gives every value of nine of the probe models and of the three examples back, and the original formulas for all but the ones written as values. The tenth has a formula column that gives a date in some rows and 0 in others, under a first row that is a date: the import says that Forge will not read that column, and Forge does not.
- **A model without dates is not touched**: of the 43 workbooks exported from the examples and the test data, 40 are part for part what the build before this change wrote; the other three are the examples with a period column. The 64 calculation outputs of the comparison corpus are byte for byte what they were.
- **Not checked in Excel.** Gnumeric and LibreOffice stand in for it. Three things would be worth one look in Excel, and are easy on a phone: `=TEXT(-0.001, "0.00")`, `=TEXT(20260131, "0000-00-00")` and `=COUNTIF(A1:A4, ">" & DATE(2026, 2, 1))` over four date cells.

## Alternatives considered

- **Leave dates as text.** It keeps `LEFT(month, 4)` simple and keeps the eight wrong answers above. Rejected.
- **`TEXT(cell, "yyyy-mm-dd")` for the text of a date.** Shorter, and wrong in every Excel that is not English.
- **A date type in Forge**, apart from text. A YAML file has no date type that every editor and diff shows the same way, and every existing model writes dates as text. The text stays; what it means is settled.
- **Excel's meaning for text functions** (`LEFT(d, 4)` as `4605`). It would make the YAML lie: the file shows `2026-01-31`.
- **Keep `+` joining text.** It is not Excel's `+`, the export wrote it as `+`, and a date plus one day is the most common date formula there is. `&` is what Excel users type.
- **Write every unknown place live and hope.** The earlier export did, and that is where the wrong answers came from. A place is live when it has a rule and a test in two engines.
- **Read a workbook's dates by pattern on import** (undo `TEXT(...)`, turn `DATE(...)` into text where it looks right). The import instead asks one question, "would Forge have written this?", which has no heuristics to get wrong.

## Consequences

- **A model that used `+` to join text now stops** with `#VALUE!: + adds numbers; use & or CONCAT to join text`. No test, example or validation case did.
- **`d = DATE(...)` is now TRUE for the same day**, a single date now counts in `MAX`, `MIN`, `SUM`, `AVERAGE` and `COUNT`, and a criterion with `>` or `<` and a date now compares. Each was a wrong answer.
- **`TEXT` with a format outside the list is an error** where it was a string, right or wrong.
- **An exported workbook with dates is not the workbook it was**: date cells, and other formulas over them. Its formulas over text-read dates are longer.
- **Excel shows a formula's date as a date only where Forge's column is one of dates.** `=d + 1` is a number in Forge and in the workbook (46054).
- **Still text in the workbook**: months (`2026-01`), dates before 1900-03-01, and a column that mixes dates with other text. A formula that reads a number from the last two is in the workbook as its values (decision 6). `forge import` cannot give back a mixed column that starts with a date: a YAML column that starts with a date is a column of dates. The import says so.
- **A workbook with dates as text loses formulas on import** that it kept before: `=YEAR(A2)` on the text `2026-01-31` gives 2026 in Excel as in Forge, and is imported as its values all the same, because no rule here says which readings of text Excel shares. A workbook exported by rc.11 or earlier is such a workbook; exporting the model again gives date cells.
- **Times are not read.** A cell with a time of day is imported as Excel's number, and `TEXT` has no `h:mm`. Forge has dates only.
- **A workbook in the 1904 date system** (old Mac files) has its date cells read as the right dates; day numbers calculated in it are 1,462 lower than Forge's.
- Found on the way and not part of this decision (roadmap): `YEARFRAC` from the last day of February to a 31st with basis 0 gives 300/360 where both engines give 301/360; and `forge functions` lists `Y`, `M`, `D`, `YD`, `YM` and `MD`, which are units of `DATEDIF`, not functions.
