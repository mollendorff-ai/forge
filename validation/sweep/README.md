# Sweeps against Gnumeric and LibreOffice

Cases made by a program, many at a time, to find answers where Forge disagrees
with both spreadsheet engines. A confirmed case goes into
`validation/gnumeric/tests` with its expected value, by hand.

## Generated cases, in CI (ADR-062)

`generate.py` draws each function's arguments from a seed, weighted toward the
edges (month ends, Feb 29, zero, halves, 1E±12, multi-byte text, wildcards,
columns with empty cells), over a catalogue of 135 functions, and judges each
case against both engines (LibreOffice alone where `JUDGES` says why). Forge
alone against the judges, or all three different, fails unless a rule in
`known.py` covers it: one class of input each, with its reason and where it is
tracked.

```bash
cargo build --release
cd validation/sweep
FORGE=../../target/release/forge python3 -I generate.py --seed 1 --per-function 20 --fail   # as a pull request runs it (3 s)
FORGE=../../target/release/forge python3 -I generate.py --seed 202646 --per-function 200    # a week's run (22 s)
FORGE=../../target/release/forge python3 -I generate.py --seed 202646 --per-function 200 --only DATEDIF   # replay one function
```

The `Formulas vs Gnumeric` job in `.github/workflows/validation.yml` runs seed
1 on every pull request and push, and the week's seed (ISO year and week) on
Mondays; Run workflow takes a seed and a count to replay one. The same seed,
`--per-function` and commit give the same cases.

## Grids, by hand

The grids below run by hand and report candidates for a person to judge.

```bash
cargo build --release
cd validation/sweep
FORGE=../../target/release/forge python3 -I dates.py   # 5,201 date cases
FORGE=../../target/release/forge python3 -I fin.py     # 2,758 financial cases
FORGE=../../target/release/forge python3 -I vdb.py     # 1,368 VDB cases
FORGE=../../target/release/forge python3 -I stats.py   # 1,234 statistical and conditional cases
FORGE=../../target/release/forge python3 -I mathf.py   # 1,009 math cases
FORGE=../../target/release/forge python3 -I lookup.py  # 846 lookup cases
FORGE=../../target/release/forge python3 -I text.py    # 2,823 text cases
FORGE=../../target/release/forge python3 -I numtext.py # 71 cases of a number read as text
python3 -I summarize.py work/dates_result.json         # counts by function
python3 -I samples.py work/dates_result.json 5         # examples of each kind
```

Needs `ssconvert` (Gnumeric) and `soffice` (LibreOffice) on the path, as the
Gnumeric suite does. A Forge scalar is a number, so a formula that gives text
runs as a column of a one-row table instead (`report_text` in `harness.py`),
and its text is compared as text, or as a number when it reads as one: compare
lengths with `LEN` where the digits matter (`numtext.py`).

The first run, on 2026-11-13, found the wrong answers fixed then (holidays in
`NETWORKDAYS` and `WORKDAY`, `WEEKDAY` types 11 to 17, `DATEDIF` "YM", `DATE`
with a year below 1900, `TIME` past a day, `EFFECT` and `NOMINAL` with a
fractional `npery`, `IPMT`/`PPMT` periods out of range, `DDB` after the life),
whose cases are in `validation/gnumeric/tests/edge/edge_date_finance_sweep.yaml`.
What it left open is in `.asimov/roadmap.yaml` (T1-EXCEL-CHECKS).

The second run, on 2026-11-14, over the statistical, lookup, math and text
functions, found the wrong answers fixed then (wildcards in criteria, `MATCH`,
`SEARCH`, `XMATCH` and `XLOOKUP`; `XLOOKUP`'s search mode; `FIND` and `SEARCH`
counting bytes; two crashes, in `REPLACE` and `FIND`; text arguments out of
range; `ROUND` and the like on binary values; `FLOOR` and `CEILING` with a
negative or zero significance; a number as text with 17 digits; `CONCAT` of a
column; `VALUE`), whose cases are in
`validation/gnumeric/tests/edge/edge_text_lookup_sweep.yaml`. Left as they are:
`TEXT` with a time or fraction format (refused with a message: Forge has dates
only), and an approximate `MATCH` or a binary `XMATCH` on values that are not
sorted the way they need (Excel's answer there comes from its binary search).
