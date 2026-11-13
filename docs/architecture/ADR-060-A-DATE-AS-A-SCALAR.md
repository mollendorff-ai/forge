# ADR-060: A Date as a Scalar

**Status:** Proposed
**Date:** 2026-11-13
**Decided by:** the maintainer (roadmap T2-SCALAR-DATES, from the FP&A audit of 2026-11-12; accepted by merging this ADR); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-054 (a date is a date) from table columns to scalars

---

## Context

A scalar cannot be a date:

```yaml
inputs:
  last_actual: 2026-10-31
```

```
'inputs.last_actual' must be a number, not the text "2026-10-31"
```

So `examples/rolling-forecast` writes `last_actual: {formula: "=DATE(2026, 10, 31)"}`, and `forge calculate`, `forge report` and the exported workbook show 46326, Excel's day number.

A scalar is a number everywhere outside the evaluator: `Variable.value` is an `Option<f64>`, and so are the scenarios, the analytics commands, the JSON output and the workbook's cell. A formula scalar's result is turned into a number when it is stored, whatever it was. This has two effects, measured on rc.20 (2026-11-13):

| Formula | A date column `d` (ADR-054) | A scalar `s`, today |
| --- | --- | --- |
| `=EOMONTH(…, 0)` | `2026-10-31` | 46326 |
| `="2026-10-31"` | `2026-10-31` | 46326 |
| `=LEFT(…, 4)` of it | `"2026"` | 4632: the text of the day number, then turned into a number again |
| `=ISNUMBER(…)` of it | FALSE | TRUE |
| `=DATE(2026, 10, 31)` | 46326 (a number in a column too) | 46326 |

So one model holds two kinds of date. A cell of a date column is its text where text is read, and a scalar is its day number everywhere. Also, `DATE` gives a number where `EDATE`, `EOMONTH`, `WORKDAY` and `TODAY` give a date. That is why the forecast's `=DATE(2026, 10, 31)` is shown as 46326 and an `=EOMONTH(…)` column as dates.

**What a date cell does in a workbook, measured.** A sheet with a date cell `B2` (2026-10-31, number format `yyyy-mm-dd`) and a column of 19 month starts, April 2026 to October 2027, was recalculated by LibreOffice 26.2 and Gnumeric 1.12 on 2026-11-13. The two agree on every value:

| Formula on the date cell | Both engines |
| --- | --- |
| `=YEAR(B2)` | 2026 |
| `=B2 + 1` | 46327 |
| `=LEFT(B2, 4)` | `"4632"` |
| `=LEFT(TEXT(YEAR(B2)*10000+MONTH(B2)*100+DAY(B2), "0000-00-00"), 4)` (ADR-054's text of a date) | `"2026"` |
| `=COUNTIF(months, ">" & B2)` | 12 |
| `=IF(B2 = DATE(2026, 10, 31), 1, 0)` | 1 |
| `=EOMONTH(B2, 12)` | 46691 |
| `=ISNUMBER(B2)` | TRUE |

They do not agree on **which results they show as dates**. LibreOffice shows `=B2 + 1` and `=B2` as dates and `=EOMONTH(B2, 12)` as a number. Gnumeric does the opposite for all three. Both show `=DATE(2026, 10, 31)` as a date. A spreadsheet guesses the format of a formula cell from the formula, and each guesses differently, so the workbook cannot be left to guess.

## Decision

1. **A scalar written as a date is a date.** `last_actual: 2026-10-31`, or `value: 2026-10-31` in the mapping form, quoted or not. It must be a full date from 1900-03-01 on, as a cell of a date column must be to be exported as a date (ADR-054, decision 4). A month (`2026-10`), an earlier date or an impossible one (`2026-02-30`) is refused, and the message says which form is wanted. A scenario may set a date scalar to another date, and must give it a date. A date scalar takes `notes:` and `source:` like any other. It takes no `unit:`, since a date has none.

2. **A date scalar is read the way a cell of a date column is read.** In the evaluator it is the date's text, so every rule of ADR-054 applies unchanged. It is its day number where a number is read (`YEAR`, `+`, `<`, a criterion, an aggregate) and its text where text is read (`LEFT`, `&`, `ISNUMBER`). So `=LEFT(inputs.last_actual, 4)` is `"2026"`, as `=LEFT(months.month, 4)` is. Outside the evaluator it keeps its day number and knows that it is a date. So the scenarios, Monte Carlo and the other commands that read a scalar's number read it, and nothing that holds an `f64` changes type.

3. **A formula scalar is a date when its value is a date**, by the rule a formula column already follows (ADR-054, decision 5). That is, its outermost value is a date: `EDATE`, `EOMONTH`, `WORKDAY`, `TODAY`, `DATE`, a date scalar or a cell of a date column, or one of these passed on by `IF`, `CHOOSE`, `INDEX`, `XLOOKUP`, `LET` and the like. A date before 1900-03-01 stays its day number. Arithmetic on a date gives a number, as in a column: `=last_actual + 30` is 46356. A model that wants a date writes a function that gives one: `=EDATE(last_actual, 1)`, `=EOMONTH(last_actual, 0)`, or `=DATE(YEAR(last_actual), MONTH(last_actual), DAY(last_actual) + 30)` for thirty days later. An aggregate of dates is also a number (`=MAX(months.month)`), as it is today.

4. **`DATE` gives a date**, in a scalar as in a column, as `EDATE` and `EOMONTH` do. Excel shows a `DATE` cell as a date, and so do both engines measured. It is also the function a model reaches for when it means a date. Keeping it a number would leave the case this ADR exists for, `=DATE(2026, 10, 31)` shown as 46326. A column `=DATE(YEAR(month), 12, 31)` becomes a column of dates: shown as dates, exported as date cells, and read as text by `LEFT`, `&` and `ISNUMBER`, as `EDATE`'s column already is. No example, test or validation case has such a column or scalar today, apart from the forecast's `last_actual`, which this ADR replaces with a date.

5. **Shown as a date everywhere a scalar's value is shown.**
   - `forge calculate` prints `inputs.last_actual = 2026-10-31`. When it writes the file back, it writes `last_actual: 2026-10-31`, and a formula scalar's cached `value:` is the date too.
   - `forge report` and Forge Studio's cards show the date.
   - JSON (`--format json`, MCP, forge-wasm) gives `"2026-10-31"`, the text, as it does for a cell of a date column. This changes the JSON of a formula scalar whose value is a date, which today is a number. None is in the corpus apart from the forecast. The changelog says so.
   - `forge compare` and `forge variance` show both dates, and the difference in days.
   - The commands that set an input to numbers of their own (`sensitivity`, `goal-seek`, `break-even`, `tornado`, and `simulate`'s distributions) refuse a date input with a message that names it. Varying a date is not decided here. A date scalar among their outputs is shown as a date.

6. **Export and import.** A date scalar is written to its sheet as its day number with the number format `yyyy-mm-dd`, and so is a formula scalar whose value is a date. Forge sets the format itself instead of leaving the engine to guess (see the measurement above). In the export a formula that reads a date scalar follows ADR-054's rules, as a reference to a date column does. So `=LEFT(inputs.last_actual, 4)` becomes `=LEFT(TEXT(YEAR($B$2)*10000+…, "0000-00-00"), 4)`, and `">" & inputs.last_actual` becomes `">" & $B$2`. `forge import` reads a scalar cell with a date format, no time of day and a date from 1900-03-01 on as a date scalar. It reads a formula scalar by its formula: whether it is a date is decided by decision 3, not by the cell's format, as for a column (ADR-054, decision 8). A workbook formula that Forge would not have written, such as `=LEFT(B2, 4)` on a date cell (`"4632"` there, `"2026"` in Forge), is imported as its value, with a warning, as it is for a column.

7. **Model format 5.4.0.** A model with a date scalar says `_forge_version: "5.4.0"`. Format 5.3.0 shipped in rc.20 today. That Forge would refuse a date scalar in a 5.3.0 file with "must be a number", and a version is how an older Forge is meant to refuse a file it cannot read (ADR-059, decision 1). In 5.3.0 and earlier, a date scalar stays the error it is now, and the message names 5.4.0. `DATE`'s change (decision 4) applies in every version, as ADR-054's changes did, because it is about what a formula gives and not about what a file may hold. If T2-NUMBER-FORMAT changes the format before 5.4.0 is in a release, it shares that version.

## Consequences

**What gets better**

- The rolling forecast writes `last_actual: 2026-10-31`, and every output shows that date.
- A date means one thing in a model, whether it is a cell of a column or a scalar.
- The workbook shows a date scalar and a `DATE` result as dates in every spreadsheet, because Forge sets the format.

**What it costs**

- A new format version: the schema, the parser's version check, `forge schema` and `forge upgrade`, the VS Code schema and its test, and the docs of the format, as for 5.3.0.
- A scalar carries whether it is a date. Every place that prints or writes a scalar's value checks that: the printers of `calculate`, `report`, `compare` and `variance`, the JSON, the write-back, the export and the import, and Studio's cards. The varying commands get a check each.
- **Four answers change, each in a model that works today:**
  - a text function of a formula scalar that gives a date (`=LEFT(s, 4)` is `"2026"`, was 4632);
  - `ISNUMBER` and `ISTEXT` of one (FALSE and TRUE, were TRUE and FALSE);
  - `&` with one (`"2026-10-31"`, was `"46326"`);
  - the same three on a column or scalar whose formula is a `DATE`.

  `DATE` inside a formula keeps its answers wherever a number is read: in a comparison (`d = DATE(2026, 1, 31)`), in a criterion (`COUNTIF(due.d, DATE(2026, 2, 28))`) and in arithmetic. These are the rules a date column follows already. None of the four changed answers occurs in the examples, the tests or the validation suites. Measured in the build PR over the whole corpus, and every output that changes is listed there.

**Acceptance**

- The rolling forecast writes `last_actual: 2026-10-31` in format 5.4.0. `forge calculate` and `forge report` print 2026-10-31, and its numbers do not change (`tests/demo_examples.rs`).
- In the exported workbook its cell is a date cell (46326, `yyyy-mm-dd`), and the workbook recalculates to the same values in Gnumeric and LibreOffice. `forge import` gives `last_actual: 2026-10-31` back.
- One Gnumeric case for each row of the measured table above, with a date scalar, plus one each for `DATE` in a scalar and in a column. Forge gives the engines' answer before export, after export and after import, or for a text reading, ADR-054's.
- The cases already in the Gnumeric suite pass unchanged, including the `DATE` comparisons and criteria of `edge_real_dates.yaml`.
- A model without a date scalar or a `DATE` result gives byte-identical output to rc.20 for `calculate`, `--format json` and `export` (apart from the time) on every file in `examples/` except the forecast.
- A 5.3.0 file with a date scalar is refused with a message that names 5.4.0. A month, a date before 1900-03-01 and `unit:` on a date are each refused with a test. So is a date input in each varying command.

## Alternatives considered

- **A date as a number with a format, as in Excel.** The scalar would stay its day number, and only the printing and the export would show a date. This would change no answer. It was rejected because it keeps two kinds of date in one model (`LEFT` gives `"2026"` on a column and 4632 on a scalar). ADR-054 settled that a date in a model is its text where text is read, because the YAML shows `2026-10-31`.
- **Keep `DATE` a number.** Fewer answers change, but `=DATE(2026, 10, 31)` stays 46326 on screen while `=EOMONTH(…)` beside it is a date.
- **Share format 5.3.0.** One fewer version, but a 5.3.0 file with a date scalar would be refused by rc.20 with a message about numbers, not versions.
- **Let the engine choose the format of a formula cell.** The two engines measured choose differently for three of the ten formulas probed.

## Not decided here

- Varying a date input in `sensitivity`, `goal-seek` and the like.
- Whether `MAX` and `MIN` of dates give a date. Today and under this ADR they give a number, in a scalar as in a column.
- A month (`2026-10`) as a scalar. A month is text in the workbook (ADR-054), and a scalar has no text.
- Text scalars in general (`="abc"` is "result is not a number" today).
