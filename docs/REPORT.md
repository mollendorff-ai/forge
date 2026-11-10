# forge report

`forge report` shows a calculated model's scalars side by side: one row per group, one column per key. It answers "how do these options compare?" without a script.

```bash
forge calculate options.yaml
forge report options.yaml
```

```
group          rent  utilities  year1_monthly
oak_suite  1,450.00      60.00       1,510.00
elm_loft   1,390.00      90.00       1,480.00
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
| `--no-inputs` | Leave out inputs. Cannot be combined with `--keys`, which already chooses the columns. |
| `--format table` | Aligned columns for the terminal (the default). |
| `--format markdown` | One pipe table per set of keys, to paste into a document. |
| `--format csv` | One block per set of keys with its own header, separated by a blank line. `--keys` gives a single block. |
| `--format json` | `{"sections": [{"keys": [...], "rows": [{"group": ..., "values": {...}}]}], "uncalculated": [...], "tables_not_shown": [...]}`. A formula without a value is `null`; a key the group does not have is left out. |

Notes go to standard error, so the output of `--format csv` or `json` can be piped as it is.

## Numbers

In `table` and `markdown`, a number has two decimals and thousands separators (`1,450.00`). A number under 1 keeps four significant digits, so a rate of `0.0479` is not shown as `0.05`. The digits are rounded half away from zero from the number's first 15 significant digits, as a spreadsheet shows them (`2.675` is `2.68`). A number under 1 shows at most six decimals, so a value closer to zero than 0.0000005 is `0.00`, without a sign.

In `csv` and `json`, numbers are at full precision with no separators, for another program to read.

A year or a count is formatted like any other number (`2,047.00`, `247.00`): the file does not say which numbers are amounts.

## Example

The examples in `examples/` work with `report` once calculated (they are kept without results):

```bash
cp examples/mortgage/mortgage.yaml /tmp/mortgage.yaml
forge calculate /tmp/mortgage.yaml
forge report /tmp/mortgage.yaml --group with_extra,saved --format markdown
```
