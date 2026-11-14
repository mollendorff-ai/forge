# Sweeps against Gnumeric and LibreOffice

Generated cases, many at a time, to find answers where Forge disagrees with
both spreadsheet engines. They are not part of CI: each run takes minutes and
reports candidates for a person to judge, and a confirmed case goes into
`validation/gnumeric/tests` with its expected value.

```bash
cargo build --release
cd validation/sweep
FORGE=../../target/release/forge python3 -I dates.py   # 5,201 date cases
FORGE=../../target/release/forge python3 -I fin.py     # 2,758 financial cases
FORGE=../../target/release/forge python3 -I vdb.py     # 1,368 VDB cases
python3 -I summarize.py work/dates_result.json         # counts by function
python3 -I samples.py work/dates_result.json 5         # examples of each kind
```

Needs `ssconvert` (Gnumeric) and `soffice` (LibreOffice) on the path, as the
Gnumeric suite does.

The first run, on 2026-11-13, found the wrong answers fixed then (holidays in
`NETWORKDAYS` and `WORKDAY`, `WEEKDAY` types 11 to 17, `DATEDIF` "YM", `DATE`
with a year below 1900, `TIME` past a day, `EFFECT` and `NOMINAL` with a
fractional `npery`, `IPMT`/`PPMT` periods out of range, `DDB` after the life),
whose cases are in `validation/gnumeric/tests/edge/edge_date_finance_sweep.yaml`.
What it left open is in `.asimov/roadmap.yaml` (T1-EXCEL-CHECKS).
