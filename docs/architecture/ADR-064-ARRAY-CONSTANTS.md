# ADR-064: Array Constants in Formulas

**Status:** Accepted (proposed 2026-11-14, built in the same pull request and accepted by merging it); amended 2026-11-15 after a check in Excel (decision 2, last item)
**Date:** 2026-11-14
**Decided by:** the maintainer ("let's fix it", 2026-11-14, after the Excel-check workbook needed cell ranges where Excel users type `{5,1,3,3,8}`; ADR and code in one pull request, at his request); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-058 (an aggregate's range is a whole column) and ADR-059 (empty cells)

---

## Context

Excel lets a formula hold a list of values typed between braces: `{1,2,3}` is a row and `{1;2;3}` a column, and `{1,2;3,4}` has two rows of two. They are common in real workbooks, for example:
- the k-th values: `LARGE(x, {1,2,3})`;
- a short lookup: `MATCH(v, {"a","b","c"}, 0)`;
- a tax table typed into a formula: `INDEX({14.29;24.49;17.49}, year)`.

Before this ADR, Forge had no `{` at all:
- **In a model,** `=SUM({1,2,3})` was a syntax error ("Unexpected character: '{'").
- **In `forge import`,** a workbook formula with a constant was imported as its last calculated values, with a warning ("unsupported character '{'"), so the formula was lost.

This was found on 2026-11-14 while building the workbook of Excel checks. `LARGE({5,1,3,3,8},1.5)` had to be written over a cell range, because Forge could not read the constant.

What Excel allows inside the braces, from Microsoft's page "Name an array constant":
- numbers, whole, decimal or in scientific form;
- text in double quotes;
- logical values, `TRUE` and `FALSE`;
- error values such as `#N/A`.

It does not allow other arrays, formulas or functions (a range or a `SUM` inside is refused). Numbers cannot have `%`, `$`, `,` or parentheses. Microsoft's "Use array constants in array formulas" gives the separators: `,` between the items of a row, `;` between rows. Neither page says whether an item may be empty.

Forge has no two-dimensional values. A table is columns of one dimension each, and the evaluator's `Value::Array` is one list.

## Decision

1. **A constant is a row `{1,2,3}` or a column `{1;2;3}`.** Its items can be:
   - numbers, with a sign and an exponent if any (`-3.5E1`);
   - texts in double quotes;
   - `TRUE` and `FALSE`.

   The separators are Excel's own in an English locale, the same that Excel writes inside a workbook. A single item `{7}` is a row of one.

2. **What Forge cannot hold is refused at parse time, with the reason:**
   - **A constant with both `,` and `;`** has rows and columns. The error says Forge has no two-dimensional values and suggests a row, a column or a table.
   - **An error value** (`#N/A`, `#DIV/0!`) cannot be held, since Forge's arrays have no error items. The error says that `NA()` gives `#N/A`. `#` outside braces gets the same message.
   - **A constant where `COUNTIF` and its family read a range** (`SUMIF`, `AVERAGEIF`, `COUNTIFS`, `SUMIFS`, `AVERAGEIFS`, `MAXIFS`, `MINIFS`). Excel answers "There's a problem with this formula" (checked on 2026-11-15 for `COUNTIF` and `SUMIF`), so a workbook with one cannot be opened there. The error says to put the values in a table column. A constant is accepted as a criterion.
   - **Anything else in the braces is refused:** a name, an expression such as `2+3` and nested parentheses, as in Excel. An empty item (`{1,,2}`) is refused too, since Microsoft's pages do not say Excel takes one.

3. **A constant is a whole list, like a column given to an aggregate (ADR-058).**
   - A function that takes a range reads all of it: `SUM`, `LARGE`, `MATCH`, `INDEX`, `SUMPRODUCT`, `XLOOKUP`.
   - Arithmetic on a constant is applied to each item: `SUM({1,2,3} * 2)` is 12, as in Excel and LibreOffice.
   - In a row formula the constant is the same whole list in every row: `INDEX({10;20;30}, x)` reads row `x`'s item.
   - **A result with several values is refused,** whether in a scalar formula (`={1,2,3}`) or in a row formula (`=x * {1,2,3}`). The error says how many values there are and to aggregate them with `SUM` or `INDEX`. The row-formula error was "result is not a number" before. Forge does not spill results into other cells, so there is nowhere else to put them.

4. **The round trip keeps the constant.**
   - **`forge export`** writes it as Excel writes it, with no spaces: `{1,2,3}`, `{"a";"b"}`, `{-1,2.5}`. Excel, Gnumeric and LibreOffice read it from the workbook.
   - **Dates written as text:** a constant holding one is written as its value instead of a formula. Excel reads such an item as text, while Forge reads it as a date (ADR-060), so the two would differ.
   - **`forge import`** reads a constant back as Forge text (`{1, 2, 3}`). A constant with rows and columns, or with an error value inside, is still imported as values, with a warning that says which.

5. **The model format stays 5.4.0.** Array constants are formula syntax, like a new function, not a key of the model. A model that uses one needs a Forge that has them, and an older Forge says "Unexpected character: '{'".

## Consequences

**Checks:**
- **Against the engines:** `validation/gnumeric/tests/functions/array_constants.yaml`, 20 cases. Each is checked against Gnumeric 1.12 after `forge export` and against Forge after `forge import`. Three are judged by LibreOffice 26.2 instead, each with its reason:
  - Gnumeric reads `INDEX({10,20,30}, 2)` as row 2 of a one-row constant and gives #REF!. Microsoft's `INDEX` page says that with one row the number is the column, which gives 20.
  - Gnumeric takes one item of `{1,2,3} * 2` in a cell that is not an array formula, and gives 2 for the `SUM`.
  - Gnumeric has no `XLOOKUP`.
- **What the engines cannot check:** `tests/array_constants.rs` covers the refusals, the import of a workbook formula with a constant, and export then import giving the same answers. Unit tests cover the parser and the workbook text.

**The example:** `examples/capex-plan` holds the IRS's 7-year MACRS rates as a constant (Publication 946, table A-1).

**What is still missing:**
- **Two-dimensional constants:** Forge has no two-dimensional values.
- **Error values inside braces:** Forge's arrays have no error items.
- **`COUNTIF`, `SUMIF` and the like over a constant:** Excel takes only a range there. Checked in Excel on 2026-11-15, `=COUNTIF({1,2,3,2},2)` and `=SUMIF({1,2,3},">1")` are both refused ("There's a problem with this formula"), though Gnumeric, LibreOffice and Forge had accepted them. Forge now refuses a constant in a range argument of `COUNTIF`, `SUMIF`, `AVERAGEIF`, `COUNTIFS`, `SUMIFS`, `AVERAGEIFS`, `MAXIFS` and `MINIFS`, at parse time, with the reason, so that no workbook Excel would refuse is written. Excel was asked about the first two only; the others are in the same family, whose pages in Microsoft's documentation name a range. A constant is still accepted as a criterion, and in functions that take arrays (`SUMPRODUCT`, `MATCH`, `INDEX`).
