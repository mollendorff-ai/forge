# ADR-061: A Number Format, Next to the Unit

**Status:** Accepted (2026-11-13, by merging #192); built (2026-11-13)
**Date:** 2026-11-13
**Decided by:** the maintainer (roadmap T2-NUMBER-FORMAT, the maintainer's and his brother's idea of 2026-11-12: the workbook is what an audience sees; accepted by merging this ADR); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-054 (`TEXT` and Excel's format codes) and ADR-060 (model format 5.4.0)

---

## Context

`forge export` writes every number in Excel's default format, `General`. A margin is `0.1234` and a price `786.32036`, so the workbook has to be formatted by hand before anyone is shown it, and formatted again after every export. `unit:` says what a number is (`CAD`, `%`), not how to show it.

Forge already knows Excel's format codes in one place: `TEXT(value, format)` reads a defined part of them (ADR-054, decision 3; `evaluator/text/number_format.rs`). That part has `0` and `#`, one `.`, `,` between digits, `%`, `E+00`, literals and quoted text, and the date codes. It refuses the rest with an error that names it.

**What the engines do with the codes a finance model uses, measured.** 25 codes × 5 values (1234.567, -1234.567, 0, 0.1234, 46326), as `TEXT(value, code)` in LibreOffice 26.2 and Gnumeric 1.12, and in Forge, on 2026-11-13. A cell's own format is not readable from either engine's CSV output, which writes the value. `TEXT` is how each engine writes a value in a format, and is what Excel shows in the cell, colour and column width aside.

- **The two engines agree on all 125 values**, except for the date codes on a day number before 1900-03-01, which neither Forge nor Excel holds as a date (ADR-054).
- **Where Forge reads the code, it gives the engines' text in every case:** `General`, `0`, `0.00`, `#,##0`, `#,##0.00`, `0%`, `0.0%`, `$#,##0.00`, `0.00E+00`, `"CAD "#,##0`, `#,##0 "units"`, `0.0"x"`, `#,##0.0" months"`, `mmm yyyy`, `yyyy-mm-dd`, `d mmm yy`.
- **Forge refuses the rest, and for each of them the engines agree with each other:**

| Code | 1234.567 | -1234.567 | 0 |
| --- | --- | --- | --- |
| `#,##0;(#,##0)` | `1,235` | `(1,235)` | `0` |
| `#,##0.00;[Red](#,##0.00);"-"` | `1,234.57` | `(1,234.57)` | `-` |
| `#,##0;-#,##0;"–"` | `1,235` | `-1,235` | `–` |
| `#,##0_);(#,##0)` | `1,235 ` | `(1,235)` | `0 ` |
| `[Red]0.00` | `1234.57` | `-1234.57` | `0.00` |
| `0.0,,"M"` | `0.0M` | `0.0M` | `0.0M` |
| `#,##0,"k"` | `1k` | `-1k` | `0k` |
| `_($* #,##0.00_)` | ` $1,234.57 ` | `- $1,234.57 ` | ` $0.00 ` |

The rules these show:
- **Sections:** the second section is for negatives and has no minus of its own; the third is for zero.
- **Colour:** `[Red]` sets a colour and adds no text.
- **`_x`:** a space the width of `x`.
- **`*x`:** fills the cell with `x`, and adds nothing in text.
- **A trailing `,`:** each one, before the point or at the end of the digits, divides by 1,000.

The model files keep what a number is in `unit:`, and `Metadata` (`src/types.rs`) holds it for scalars and data columns. A formula column written in the long form, `{formula: "=…", unit: …}`, has its metadata parsed and then dropped (a `TODO` in `parser/variables.rs`). Most of what a model would format, its revenue and its margins, is formula columns.

**Reading a format back.** calamine 0.38, which `forge import` uses, gives each cell's format code (`Xlsx::worksheet_style`), the built-in ones as their code (`#,##0` for id 3).

## Decision

1. **`format:` holds an Excel number-format code**, beside `unit:`. It can be on a scalar in the mapping form, on a data column in its long form (`{value: [...], format: "#,##0"}`), and on a formula column in its long form (`{formula: "=...", format: "0.0%"}`). It is Excel's own code, so nothing is translated between Forge and the workbook, and `forge import` can read it back. A format uses Excel's invariant symbols, `,` for thousands and `.` for the decimal point, as the code inside every `.xlsx` file does. An Excel set to French shows `#,##0.00` as `1 234,57` from the same code.

2. **Forge reads every code it accepts, and accepts only those.** A format must be one that Forge's renderer writes as the engines do. That is the part `TEXT` reads today, extended by what the measurement shows a finance model needs:
   - **sections:** two (positive and zero; negative without its minus) or three (positive; negative; zero);
   - **colours** (`[Red]`, `[Blue]` and the other six Excel names): no text;
   - `_x`: one space;
   - `*x`: nothing;
   - **a trailing `,`:** each one divides by 1,000.

   Anything else is refused when the file is read, with a message that names the part Forge does not read: a fourth (text) section, a condition (`[>1000]`), a locale or currency tag (`[$€-407]`), fractions, times. A model never holds a format that Forge would show differently from Excel. `TEXT` reads the same extended part, so `TEXT(x, "#,##0;(#,##0)")` gives `(1,235)` where it was an error.

3. **A number code goes on a number and a date code on a date** (ADR-054, ADR-060). `format: "mmm yyyy"` on a date column or a date scalar shows `Oct 2026`, and a number code on a date, or a date code on a number, is refused. Without `format:`, a date is exported as `yyyy-mm-dd`, as now.

4. **A format changes how a number is shown, never its value.**
   - **Unchanged:** `forge calculate`'s output and the file it writes back, formulas, `--output`, the analysis commands, and every value in JSON.
   - **Shown with the format:**
     - `forge report`'s table and Markdown (its CSV and JSON stay values);
     - Forge Studio's cards and grid, which show the text Forge gives them. Studio adds no formatting rule of its own: its percentage-by-unit rule applies only where there is no `format:`.
   - **JSON** gains, beside the values, a `formatted` map of the text for each scalar and column that has a format. Nothing else in it changes.

   Forge writes thousands and the decimal point as `,` and `.` wherever it shows a format.

5. **Export and import.**
   - **Export:** `forge export` writes each cell of a scalar or column that has `format:` with that code, data and formula cells alike. A file without `format:` exports byte for byte what it exports today, apart from the time.
   - **Import, which formats come back:** `forge import` reads a column's code when every data cell of it has the same one, and a scalar's code from its cell. It writes `format:` when that code is one Forge reads (decision 2) and is not `General`. Nor is it, on a date, the `yyyy-mm-dd` that Forge writes for dates without a format.
   - **Import, which do not:** a code Forge does not read, or a column whose cells differ, is left out, with a warning that names the code and the cells.
   - **Import, the form written:** a scalar with a format is written in its mapping form, `{value: 16, format: "$#,##0.00"}`.

6. **A formula column keeps its metadata.** The long form `{formula: "=...", unit: ..., notes: ..., format: ...}` stores `unit:`, `notes:` and the rest instead of dropping them. Whatever reads a column's metadata then reads a formula column's too: the export's notes, `forge report`, Studio. That is a change of its own, made in the same pull request because `format:` needs it, and it is listed there.

7. **Model format.** `format:` is new, so a model with one names a format that has it. If this is built before a release carries 5.4.0 (ADR-060, not released as this is written), 5.4.0 has it and nothing else changes. Otherwise it is 5.5.0, with its own schema. Either way an earlier format refuses `format:` and names the one that has it. *(Built in 5.4.0, before any release carried it.)*

**Built this way, two notes.**
- **A negative number that rounds to zero keeps its minus in Forge.** For example, `0.0,,"M"` of -1234.567 is `-0.0M`, where both engines write `0.0M`. That is the rule `TEXT` already had (`TEXT(-0.001, "0.00")` is `-0.00`, ADR-054), which Excel is believed to follow and which neither engine confirms. The case is left out of the Gnumeric suite and named there.
- **LibreOffice formats some cells itself when it recalculates a workbook.** It gave `revenue = dozens * price` the price's currency format. `forge import` reads what a workbook holds, so a workbook saved by LibreOffice can come back with more `format:` than Forge exported.

## Consequences

**What gets better**
- An exported workbook can go to an audience as it is: `$1,234.57`, `12.3%`, `(1,235)`, `Oct 2026`.
- `forge report` and Studio show the numbers as the workbook does.
- `TEXT` reads the formats finance uses, with the engines' answers.
- A formula column's `unit:` and `notes:` stop disappearing.

**What it costs**
- The renderer gets sections, colours, `_`, `*` and scaling commas, and the parser checks every `format:` with it. Each rule gets cases in the Gnumeric suite, as `TEXT` formulas recalculated by the engine.
- `Metadata` gains `format`. Formula columns gain metadata, and the table keeps it beside `row_formulas`.
- **One answer changes in a model that works today.** A `TEXT` with one of the newly read codes was an error, and gives the engines' text now. That changes a model only where an `IFERROR` caught the error. It is the answer Excel gives.

**Acceptance**
- The bagel example gives `format:` to a price (`$#,##0.00`), a percentage (`0.0%`) and a count (`#,##0`), on a scalar, a data column and a formula column. Exported, opened in LibreOffice and recalculated, those cells have those codes. Their values are the same, with 0 mismatches.
- `forge import` of that workbook gives the three `format:` back.
- A file without `format:` exports the workbook it exports today, byte for byte apart from the time, on every file in `examples/`.
- For each code in the measured table above, and the codes Forge already read, one `TEXT` case in `validation/gnumeric` gives the engines' text.
- A code Forge does not read, a number code on a date and a date code on a number are each refused with a test. An import of a cell with such a code warns and leaves it out.
- `forge report` and Studio show a formatted scalar with its format. JSON's values are unchanged, and `formatted` holds the text.

## Alternatives considered

- **A short list of Forge's own formats** (`currency`, `percent:1`, `thousands`). Easier to type, but every name has to be translated to a code. Import would have to guess a name from a code, and a code with no name would be lost. Excel's codes are what the people who write models already type.
- **Pass any code through to the workbook unread.** The workbook would show it, but `forge report` and Studio could not, and a misspelt code would reach the audience. Forge accepts what it can show the same way.
- **Derive the format from `unit:`** (`%` shows as a percentage, `CAD` as currency). Units do not say decimals, negatives in parentheses or thousands, and the same unit is shown differently in different places.
- **Read each cell's format on import, cell by cell.** A Forge column has one format. A column whose cells differ is left without one, with a warning, rather than given the first cell's.

## Not decided here

- Cell styles other than the number format: bold, colours of a cell, borders, column widths.
- Locale-specific output from Forge (`1 234,57`). Forge shows `,` and `.`; the workbook shows the reader's locale.
- A format for text or boolean columns.
- A fourth (text) section, conditions such as `[>1000]` and currency tags such as `[$€-407]`, which Forge refuses until a model needs them and they are measured.
