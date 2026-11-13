# A rolling forecast

`forecast.yaml` is the model an FP&A team updates every month. The fiscal year starts in April, and actuals are in up to October 2026. It runs 19 months, from April 2026 to October 2027, and from November 2026 on it is forecast, twelve months past the last actual. The budget covers the fiscal year 2027, April 2026 to March 2027. There are two products, Standard and Premium. All figures are made up.

Things that usually take a spreadsheet's worth of care are one formula each:

- **Actuals, then forecast.** The months not closed yet have an empty cell (`null`) in each actual column, and `is_actual: "=ISNUMBER(actual_standard_units)"` marks the months that have one (ADR-059). An actual of 0 is a 0, not an open month. Each driver is the actual where there is one, and otherwise the month before, grown: `=IF(is_actual, actual_standard_units, PREV(standard_units, 0) * (1 + inputs.standard_volume_growth))`. Next month, type in the new actuals over the nulls, change the date in `last_actual: {value: 2026-10-31, format: "d mmm yyyy"}` to the new month's last day (the next twelve months are counted from it; a date as a scalar is format 5.4.0, ADR-060), and add one to the period column's `count` (and a `null` to each actual column) to keep twelve months ahead; the forecast starts one month later by itself.
- **The fiscal calendar.** `fiscal_year` and `fiscal_quarter` come from the month and `fy_start_month`, so a year that starts in July is one number to change.
- **Formats.** `format:` (ADR-061) says how a number is shown: units, revenue and budget `#,##0`, prices `#,##0.00`, growth and shares `0.0%`, the variance `#,##0;(#,##0)` with a loss in brackets, and the last actual `d mmm yyyy`. The workbook `forge export` writes has them in its cells, `forge report` and Studio show them, and no number changes.
- **Year to date.** `fiscal_ytd` starts again each April: `=IF(fiscal_year = PREV(fiscal_year, 0), PREV(fiscal_ytd, 0), 0) + revenue`.
- **The budget, month by month.** The budget is a table of its own with 12 months. `XLOOKUP(month, budget.month, ...)` finds the budget of the same month, and gives 0 after the budget's year.
- **A table of quarters.** `quarters` has one row per quarter, rolled up from the months: `revenue: "=SUMIFS(months.revenue, months.fiscal_year, 2027, months.fiscal_quarter, quarter)"`. In a row formula an aggregate's range is the whole column and its criterion the row's value, as on a summary sheet in Excel (ADR-058). `share_of_year: "=revenue / SUM(revenue)"` is each quarter's share of the year.
- **The next twelve months.** `SUMIFS` over a window of dates after the last actual.
- **Price, volume and mix.** Why the fiscal year 2027 differs from budget. Volume counts all units, sold in the budget's mix at the budget's prices. Mix is each product's share, at the budget's prices, and price is each product's own price. `pvm.check` shows that the three add up to the variance.

The fiscal year 2027 comes to 987,621.93 against a budget of 960,600.00, which is 27,021.93 over. Of that, volume is 14,412.74, mix 11,875.97 (Premium sells more than budgeted) and price 733.22. The quarters are 229,100.50, 240,878.50, 253,830.27 and 263,812.66, against budgets of 225,300, 235,200, 245,100 and 255,000, and the actuals so far are 553,505.60. The twelve months after October 2026 come to 1,090,944.36.

```bash
forge calculate examples/rolling-forecast/forecast.yaml --dry-run
forge sensitivity examples/rolling-forecast/forecast.yaml -v inputs.premium_volume_growth -r 0,0.03,0.01 -o fy2027.revenue
forge export examples/rolling-forecast/forecast.yaml forecast.xlsx
```

`tests/demo_examples.rs` checks these numbers against the same calculation written in Python. In the exported workbook every result is a formula, and Gnumeric recalculates it to the same numbers.

## What Forge cannot do here yet

Building this model found three gaps, all closed now: a table of quarters (ADR-058), months without actuals (ADR-059) and the last actual as a date (ADR-060).

Two products are four columns and their formulas twice. A dimension such as product × month is the job of a planning tool (Anaplan, Pigment), and Forge does not try to be one.
