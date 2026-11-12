# ADR-053: Periods and the Row Above

**Status:** Accepted
**Date:** 2026-11-09
**Decided by:** the maintainer asked for periods as a first-class axis (roadmap T2-TIME) and left the design to Claude; the design is Claude Fable 5.2's

---

## Context

Every financial model has time, and a Forge table had no way to say "the row before". A row formula read its own row only, so a loan schedule, a running cash balance or revenue that grows from last month's had to be rewritten as a closed form on a row number:

```yaml
first_year:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  opening: "=loan.principal * (1 + terms.monthly_rate) ^ (month - 1) - with_extra.payment * ((1 + terms.monthly_rate) ^ (month - 1) - 1) / terms.monthly_rate"
```

That is what a spreadsheet user writes as `=G2`. A closed form exists for few things; a schedule with a rounded payment, a changing rate or a floor has none. And the rows themselves had to be typed: `month: [1, 2, ..., 300]` for a 25-year loan.

Three things were missing: a way to read the row above, a way to say "300 months from December 2026" without typing 300 values, and the number of the period.

## Decision

1. **`PREV(value, first, [back])` reads the row above.** In a row formula, `PREV(closing, loan.principal)` is `closing` as it is one row up, and `loan.principal` in the first row, which has no row above. `value` is any expression (`PREV(sales * 1.01, 100)`), read as a whole in the earlier row. `back` (1 if left out) is how many rows up; the first `back` rows give `first`. `first` and `back` are read in the formula's own row.

   `first` is required. A first row always needs a value, and a default (0, or an error) would be either a silent wrong answer or a rule to remember.

2. **Columns that read each other through `PREV` are calculated together, row by row.** `opening` reads `closing` in the row above and `closing` reads `opening` in its own row. Forge calculated a column at a time, so this was a circular dependency. Now a dependency is *lagged* when a formula reads a column of its own table only inside the first argument of `PREV`:

   - Without the lagged dependencies the cells must still have an order. A cycle there is a circular dependency, as before.
   - With them, cells that reach each other form a group. A group is calculated row by row, its columns in dependency order within each row.
   - A group must be columns of one table. A cycle that passes through a scalar (`=SUM(t.a)`) or another table needs a whole column before its first row exists, and is an error that says so.

   A model without `PREV` has no group and is ordered exactly as before.

3. **A period column generates the dates.** A table column may be written as a mapping:

   ```yaml
   schedule:
     month: {start: 2026-12, count: 300, unit: month}
   ```

   It is the column of the dates on which `count` periods of one `unit` (`day`, `week`, `month`, `quarter`, `year`) start, the first at `start` (`2026-12` is the first of the month, `2026-12-15` a day, `2026` the first of the year). It is an ordinary column of dates (`2026-12-01`, `2027-01-01`, ...): `YEAR(month)`, `MONTH(month)` and `INDEX(schedule.month, n)` work on it, and it gives the table its number of rows. Each date is counted from the start, so a start on the 31st gives the last day of shorter months, as `EDATE` does. The author names the column; there is no reserved name. A mapping that holds a period column is a table (this adds one case to "How a group is read" in ADR-051).

4. **`ROW()` is the number of the period.** It was already the 1-based number of the table's row. `forge export` wrote it as a value, because Excel's `ROW()` is the sheet's row; it is now written as `(ROW() - 1)`.

5. **Model format 5.2.0.** The period column is a new shape in the file, so it is a new minor format, by the rule of ADR-051: 5.1.0 and 5.0.0 files stay valid and unchanged, and an older Forge reading a 5.2.0 file fails with "Unsupported _forge_version" instead of misreading it. 5.2.0 is 5.1.0 plus the period column and nothing else. A file needs it only if a table has a period column; `PREV` is a function and works in every format that has tables. `forge upgrade --to 5.2.0` changes the version line.

## Three worked examples

Each is a file in `examples/`, and each number below is checked in `tests/demo_examples.rs` against the same calculation written in Python.

**A loan schedule** (`examples/mortgage/mortgage.yaml`): 450,000 at 4.79% compounded twice a year, 2,863.69 a month.

```yaml
schedule:
  month: {start: 2026-12, count: 300, unit: month}
  number: "=ROW()"
  opening: "=PREV(closing, loan.principal)"
  interest: "=opening * terms.monthly_rate"
  payment: "=MIN(with_extra.payment, opening + interest)"
  principal_paid: "=payment - interest"
  closing: "=opening + interest - payment"
```

The loan is paid off with payment 247 (42.25, in June 2047) and the rows after it are zeros. The total interest is 254,509.41, which is what the closed forms of the same example give; the example checks that. A month-by-month loop in Python gives the same opening balance, interest, payment and closing balance in months 1, 12, 60, 120, 246 and 247, to a millionth.

**A monthly plan** (`examples/periods/monthly_plan.yaml`): sales that grow 1% a month, with a season, and cash that carries over.

```yaml
months:
  month: {start: 2027-01, count: 12, unit: month}
  trend: "=PREV(trend * (1 + inputs.monthly_growth), inputs.january_trend)"
  sales: "=trend * INDEX(season.factor, MONTH(month))"
  gross_margin: "=sales * (1 - inputs.cost_of_goods)"
  profit: "=gross_margin - inputs.fixed_costs"
  cash: "=PREV(cash, inputs.opening_cash) + profit"
  change: "=sales / PREV(sales, sales) - 1"
```

Sales for the year 229,045.23, profit 18,846.23, cash 30,846.23 at the end and 10,309.52 at its lowest.

**A five-year DCF** (`examples/periods/dcf.yaml`):

```yaml
forecast:
  year: {start: 2027, count: 5, unit: year}
  revenue: "=PREV(revenue, assumptions.revenue_2026) * (1 + assumptions.growth)"
  after_tax_profit: "=revenue * assumptions.ebit_margin * (1 - assumptions.tax_rate)"
  free_cash_flow: "=after_tax_profit * (1 - assumptions.reinvestment)"
  discount_factor: "=1 / (1 + assumptions.wacc) ^ ROW()"
  present_value: "=free_cash_flow * discount_factor"
```

The five years are worth 1,866,331.18 (the same as `NPV` of the cash flows), the terminal value 5,315,953.60 today, and the equity 5,682,284.78.

## In Excel

A row formula is written once per row, so `PREV` needs nothing new in Excel: the cell holds `first` in the first row and reads the cell above in every other row, which is how a schedule is built by hand.

| Forge | Row 2 | Row 3 |
| --- | --- | --- |
| `=PREV(closing, loan.principal)` | `=('loan'!$B$2)` | `=(G2)` |
| `=PREV(running, 0) + x` | `=(0) + A2` | `=(B2) + A3` |
| `=ROW()` | `=(ROW() - 1)` | `=(ROW() - 1)` |

Gnumeric recalculates the exported loan schedule and the two other examples to Forge's numbers, and so does LibreOffice. forge-e2e-gnumeric has 14 cases on `PREV`, `ROW()` and a period column (`tests/edge/edge_previous_row.yaml`), with expected values worked out by hand.

`forge import` reads a column whose first rows differ from the rest back as the `PREV` that was written. The later rows are read with a marker for each cell from a row above (`PREV(closing, 0)`, or `PREV(x, 0, 2)` two rows up), lined up with the first row, and each part that reads only the rows above becomes `PREV(part, the first row's part, back)`; the result is kept only if it gives every first row as the workbook holds it (T3-IMPORT-PREV, 2026-11-12). A running balance built by hand in Excel (`=A2`, then `=B2+A3`) has `amount` in its first row where a `PREV` would have `0 + amount`, so it comes in as `=IF(ROW() = 1, amount, PREV(balance, 0) + amount)`: the same calculation, written differently. Until 2026-11-12 every such column came in that way, and one that read two rows up came in as values.

## Alternatives considered

- **`revenue[t-1]`, an index in brackets.** It reads like the notation of a planning tool, but it needs a `t`, a meaning for `t+1` (the row below, which cannot be calculated row by row), and a rule for the first row that the notation has no place for. `PREV(value, first)` has a place for the first row and cannot name a later one.
- **A default for the first row** (0, or the value itself). An opening balance of 0 that nobody wrote is a silent wrong answer. Rejected: `first` is required.
- **`_period:` as a reserved key of the table**, generating a column named `period`. One more reserved name and a column that appears from nowhere. A column the author names (`month`, `year`, `week`) reads better in formulas and in Excel.
- **A function that makes the dates** (`=PERIODS("2026-12", 300, "month")`). A row formula cannot give a table its number of rows: the rows must be known before any formula runs.
- **`count` as a formula** (`count: terms.months`). The number of rows would then depend on a calculation, and every check that runs before calculating (lengths, the schema) would have to wait for it. A schedule longer than needed, ending in zeros, does the same job.
- **A new function for the period number.** `ROW()` already is that number; it only had to be exported correctly.
- **Putting the period column in format 5.1.0.** No file would break, but an older Forge would read `_forge_version: "5.1.0"` and then fail on a shape it was told it knows. ADR-051 set the rule: a new shape is a new minor format.
- **Writing date columns as real Excel dates.** `forge export` writes dates as text (`2026-12-01`), a period column included. Gnumeric and LibreOffice read `YEAR` and `MONTH` of that text as Forge does. A real date would be better for charts and filters in Excel, but a text function on the column (`LEFT(month, 4)`) would then give another result in Excel than in Forge, and `forge import` would have to read dates back. It is its own decision (roadmap T3-EXPORT-DATES), not part of this one. (Decided since: [ADR-054](ADR-054-A-DATE-IS-A-DATE.md) writes them as dates.)

## Consequences

- A column calculated row by row is known only up to the row being calculated. Reading a later row of it (`INDEX(closing, 300)` in a row formula of the same group) is an error that says so.
- `PREV` of a column of another table reads that table's row above; the other table is calculated first, as a whole.
- `PREV` with a number of rows back that is calculated, not written as a number, is exported as values.
- `forge import` no longer takes the formula of the first data row for the whole column. It did, without a word, so a running balance of 100, 150, 175 came in as `=amount` (100, 50, 25). A column whose rows hold different formulas that fit neither one formula nor the first-row-and-the-rest shape is imported as its values, with a warning. Excel's `ROW()` is read as `(ROW() + 1)`, and a formula that uses a function Forge defines differently from Excel (`COLUMN`, `VLOOKUP`, `OFFSET`, `INDIRECT`, `NOW`, spilling functions) is imported as its value.
- The examples that use a period column are format 5.2.0; the others stay 5.1.0. Editors get one schema, `schema/forge.schema.json`, which checks a file against the schema of the format it names.
- Not done here: reading the row below, a period column whose length depends on a calculation, and periods of unequal length (a 4-4-5 calendar).
