# A machine bought on a loan

A 120,000 machine paid for with a five-year loan at 6.5%, written down over eight years, with five years of revenue from its line. It uses the schedule and regression functions `CUMIPMT`, `CUMPRINC`, `SYD`, `VDB`, `RRI`, `SLOPE`, `FORECAST.LINEAR` and `RSQ`, and a tax table typed into a formula as an array constant (`{14.29; 24.49; …}`, ADR-064).

```
$ forge calculate examples/capex-plan/capex.yaml
```

| | |
|---|---|
| Monthly payment | 2,347.94 |
| Year 2: interest / principal | 5,775.93 / 22,399.33 |
| Year 3 write-down: straight line / sum of the years' digits / double declining | 13,500 / 18,000 / 16,875 |
| Year 3 tax depreciation, 7-year MACRS (17.49%) | 20,988 |
| Revenue growth a year, first to last year | 12.64% |
| Revenue in year 6 on the straight line through the five years | 72,300 (R² 0.993) |

`bank.check` is 0: year 2's interest plus principal is exactly twelve payments.

**Checked outside Forge:** `src/cli/commands/examples/mod.rs` (its `demo_examples` tests) repeats the loan month by month and fits the line with NumPy; the numbers agree to 1e-9. The functions themselves are checked against Gnumeric and LibreOffice in `validation/gnumeric`.

`forge calculate` writes the results back into the file; run it on a copy.
