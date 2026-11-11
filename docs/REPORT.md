# forge report

`forge report` shows a calculated model's scalars side by side: one row per group, one column per key. It answers "how do these options compare?" without a script.

```bash
forge calculate options.yaml
forge report options.yaml
```

```
group       rent  utilities  year1_monthly
oak_suite  1,450         60          1,510
elm_loft   1,390         90          1,480
```

## What it reads

The values that `forge calculate` wrote into the file. `report` does not calculate and never writes the file, so the YAML stays the source of truth. If you changed an input since the last `forge calculate`, the formulas still show the old results: calculate first.

A formula with no value yet is shown as `—`, and one note on standard error names it. The command still succeeds (exit 0):

```
Note: 2 formulas have no value yet and are shown as —; run `forge calculate options.yaml` first (oak_suite.year1_monthly, elm_loft.year1_monthly)
```

Only scalars are shown. A table (columns of values) is listed in a note; `forge export` writes tables to Excel.

## How the rows and columns are chosen

- A **group** is a mapping of scalars at the top of the file (`oak_suite:` above). Scalars at the top level of the file, outside any group, are one row named `(top level)`.
- Groups that have **the same keys** share one table, whatever order the keys are written in. Groups with other keys get their own table. Tables, and the rows in each, come in file order.
- **Inputs** (values written with no formula) come before formulas, each in file order.

| Option | What it does |
| --- | --- |
| `--group a,b` | Only these groups, in file order. An unknown name is an error that lists the groups. |
| `--keys x,y` | Only these keys, in this order, as one table, for every group that has at least one of them. A group without a key has an empty cell there. An unknown key is an error that lists the keys. |
| `--no-inputs` | Leave out inputs. Not with `--keys`, which already chooses the columns (`--help` of both says so). |
| `--format table` | Aligned columns for the terminal (the default). |
| `--format markdown` | One pipe table per set of keys, to paste into a document. |
| `--format csv` | One block per set of keys with its own header, separated by a blank line. `--keys` gives a single block. |
| `--format json` | `{"sections": [{"keys": [...], "rows": [{"group": ..., "values": {...}}]}], "uncalculated": [...], "tables_not_shown": [...]}`. A formula without a value is `null`; a key the group does not have is left out. |

Notes go to standard error, so the output of `--format csv` or `json` can be piped as it is.

## Numbers

In `table` and `markdown`, a number has two decimals and thousands separators (`1,450.00`). A number under 1 keeps four significant digits, so a rate of `0.0479` is not shown as `0.05`. The digits are rounded half away from zero from the number's first 15 significant digits, as a spreadsheet shows them (`2.675` is `2.68`). A number under 1 shows at most six decimals, so a value closer to zero than 0.0000005 is `0.00`, without a sign.

In `csv` and `json`, numbers have 15 significant digits, as Excel shows them, with no separators, for another program to read.

A column whose values are all whole numbers has no decimals (`1,700`, `3`, `0`, `2,047`). The rule is per column, not per cell, so a column never mixes `3` and `3.50`: one fractional value gives every cell of that column its decimals. The file does not say which numbers are amounts, so a column of whole amounts (`1,700`) reads the same as a column of counts.

An input is shown as written where the rules above would round it: `0.14975` is not `0.1498`. A formula's result is never shown more precisely than the rules say; use `--format csv` or `json` for all 15 significant digits.

## A wide table

A model with many keys makes a table wider than a phone or a narrow terminal. `report` does not wrap or cut it. `--keys` narrows it to the columns you choose, in your order:

```bash
forge report options.yaml --keys rent,year1_monthly
```

## Example

[`examples/options/compare-options.yaml`](../examples/options/compare-options.yaml) compares five invented rental units, each a group with the same keys, against a take-home pay. The examples are kept without results, so calculate a copy first:

```bash
cp examples/options/compare-options.yaml /tmp/options.yaml
forge calculate /tmp/options.yaml
forge report /tmp/options.yaml --no-inputs --format markdown
```

| group | monthly | first_year | second_year_monthly | left_each_month | share_of_pay | room_under_cap |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| maple_studio | 1,582 | 19,884 | 1,622.50 | 3,218 | 0.3296 | 98 |
| harbour_one_bed | 1,650 | 21,000 | 1,689.88 | 3,150 | 0.3438 | 30 |
| park_loft | 1,752 | 21,674 | 1,811.20 | 3,048 | 0.365 | -72 |
| river_studio | 1,542 | 20,004 | 1,587.15 | 3,258 | 0.3213 | 138 |
| garden_one_bed | 1,660 | 20,320 | 1,736.25 | 3,140 | 0.3458 | 20 |

| group | lowest_monthly | lowest_first_year |
| --- | ---: | ---: |
| summary | 1,542 | 19,884 |

The `budget` group holds only inputs, so `--no-inputs` leaves it out. Its numbers, and the five options', are checked against the same calculation in Python in `tests/demo_examples.rs`. `--keys monthly,first_year` gives a narrower table, for a phone.
