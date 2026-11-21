# Kestrel Coffee Roasters: a budget you inherit as a workbook

Kestrel Coffee Roasters is a fictional roaster that sells coffee by the kilo to cafés. Its FY2027 budget is a workbook someone else built, and you have just been handed it. Every name and figure is made up. This example starts where most models start, from a spreadsheet: `forge import`, `forge audit`, one change, and `forge export` back to a workbook that recalculates outside Forge.

`budget.xlsx` is the workbook as it was handed over. It was not written by Forge: `make_workbook.py` writes it with openpyxl, and LibreOffice then calculated it and stored its values, as a spreadsheet would have. `budget.yaml` is the model after the steps below.

## The workbook

Three sheets, laid out the way people lay them out, not the way `forge export` does:

- **Assumptions**: a title merged across the top, a line of instructions, then a table of drivers: `Driver`, `Value`, `Unit`, `Comment`. Opening volume 4,200 kg a month growing 1.5% a month, a price of $38.50 a kilo rising 4% from July, green coffee at $14.20 and roasting and packing at $3.10 a kilo, rent of 6,500 and staff of 21,000 a month, tax at 25%. The price is also a named range, `list_price`.
- **Monthly**: a title, headings merged over the columns (`Sales`, `Costs`), then January to June, a blank row, July to December, and a `Total` row of `SUM`s. Volume grows from the month above (reading over the blank row in July), the price reads `list_price`, and one cell, October's staff cost, is a number typed in where every other month reads the assumption.
- **Summary**: a title and labels with formulas beside them: revenue, gross margin and its share, EBITDA, tax, net income, and H2's revenue over H1's (`=SUM(Monthly!D11:D16)/SUM(Monthly!D4:D9)-1`).

## Import

```bash
forge import examples/inherited-budget/budget.xlsx budget.yaml
```

The titles, the merged headings and the blank row are left out of the tables, and every name is made from its label (`Opening volume` is `assumptions.opening_volume`, `Volume (kg)` is `monthly.volume_kg`). The `Total` row comes in as scalars beside the table, `monthly_total.revenue: =SUM(monthly.revenue)`, so `=Monthly!D17` on the summary is `monthly_total.revenue`. `list_price` is the cell it names, `assumptions.price`. Units and comments are kept as each assumption's `unit:` and `notes:`. Volume, which reads the month above, comes in as `=IF(ROW() = 1, assumptions.opening_volume, PREV(volume_kg, 0) * (1 + assumptions.volume_growth))`.

The import names three things it did not find as Forge holds them:

```text
- monthly.price: its rows hold different formulas or numbers typed in (rows 4 to 9: assumptions.price; rows 11 to 16: assumptions.price * (1 + assumptions.price_rise_from_july)), so it is imported as one formula that picks by row: =IF(ROW() <= 6, assumptions.price, assumptions.price * (1 + assumptions.price_rise_from_july))
- monthly.staff: its rows hold different formulas or numbers typed in (row 14 holds 21000, the others assumptions.staff), so it is imported as one formula that picks by row: =IF(ROW() = 10, 21000, assumptions.staff)
- summary.h2_revenue_over_h1: imported as a value (a range over part of a column: rows 11 to 16 of a table in rows 4 to 16): =SUM(Monthly!D11:D16)/SUM(Monthly!D4:D9)-1
```

The imported model calculates to the workbook's own values, every one of them (asserted in `src/cli/commands/excel_io/mod.rs`, `inherited_budget`):

| | FY2027 |
| --- | --- |
| Volume | 54,773.088 kg |
| Revenue | 2,152,821.71 |
| Gross margin | 1,205,247.29 (56.0%) |
| EBITDA | 875,247.29 |
| Net income | 656,435.46 |
| H2 revenue over H1 | 13.7% |

## Audit

```bash
forge audit budget.yaml summary.net_income
```

The tree runs from net income through `monthly_total.ebitda` and the month table down to the assumptions, and shows where October's typed number goes:

```text
            └─ monthly.staff (Column[monthly]) = 12 rows: 21000 … 21000 =IF(ROW() = 10, 21000, assumptions.staff)
               └─ assumptions.staff (Scalar) = 21000
```

Two things the import named would go wrong at the next change. October's staff cost does not move with the assumption. And H2 over H1 is a number now, 13.7%, that no change would move.

## The change

Staff gets a pay rise, to 22,500 a month. Before changing it, `budget.yaml` puts back the two things above:

- `monthly.staff: =assumptions.staff`, with October's 21,000 gone.
- `summary.h2_revenue_over_h1: =SUMIFS(monthly.revenue, monthly.month, ">=2027-07-01") / SUMIFS(monthly.revenue, monthly.month, "<2027-07-01") - 1`, which gives the workbook's 13.7% (0.137180994500346) as a formula.

Then `assumptions.staff: 22500`, and:

```bash
forge calculate examples/inherited-budget/budget.yaml --dry-run
```

| | Net income |
| --- | --- |
| The workbook | 656,435.46 |
| The pay rise, October still typed in | 644,060.46 |
| **The pay rise (`budget.yaml`)** | **642,935.46** |

Left as it was, October would have understated the year's staff cost by 1,500, and overstated net income by 1,125 after tax.

## Back to a workbook

```bash
forge export examples/inherited-budget/budget.yaml budget_2027.xlsx
python3 -I examples/inherited-budget/check_export.py forge
```

`check_export.py` exports the model, has LibreOffice recalculate every formula of the export on load (with a profile that does so, the same as `validation/gnumeric`'s, since LibreOffice otherwise shows the results Forge stored), and compares every number with Forge's. With LibreOffice 24.2: `120 numbers, 103 formulas recalculated by LibreOffice, 0 different`. A Forge result changed by hand in the file before the check comes back as LibreOffice calculates it, so the check is LibreOffice's arithmetic, not Forge's.

## What import does not read

- **A range over part of a column**, such as `SUM(Monthly!D11:D16)` for the second half-year, is imported as its value, with a warning. Forge reads a table's column whole; `SUMIFS` on a column of dates or labels picks the rows.
- **A column whose formulas change more than three times**, or that holds a number typed into more than three rows, or a different formula in every row, is imported as its values, with a warning.
- **A sheet needs a header or a title in its first row.** The first row is read as the header unless it is a title, so a sheet whose labels start in the first row loses the first label and its value; the import says so.
- **Only the last row of a table can be a total**, and only when one of its cells reads a range of its own column. A subtotal in the middle, or an `Average` row under the `Total`, is a row of the table with formulas that differ.
- **Formats are kept where a column's cells share one**, and the total row's are not kept. The merged cells, fonts and column widths are not.
