# Corvane Seed Fund I: an investor's cash flows on their dates

Corvane Seed Fund I is a fictional venture fund of 40,000,000, seen by its investors (its limited partners). The fund, its eight companies and every date and figure are made up. Money moves on the day it moves: the fund calls capital when it invests and once a year for its fee, and pays it back when a company is sold, years later and never on a schedule. A return worked out by row or by year would be wrong by the months in between. This example works on the dates themselves, with `XIRR` and `XNPV`.

`fund.yaml` is the whole model: `forge calculate`, `scenarios`, `tornado`, `goal-seek` and `export`.

## The fund

- **The investments.** From its first close on 2 March 2020, the fund invests 30,000,000 in eight companies between April 2020 and March 2023, 3,000,000 to 4,500,000 each (`deals`). Each is sold a set number of months after it was bought (`=EDATE(invested, hold_months)`), from 36 to 84, for a multiple of its cost. As in most venture funds, the multiples are uneven: two companies return nothing, one returns half, and one, brisk, returns 8 times its cost and a little under half of everything.
- **The fee.** It is called each 31 March (`=EOMONTH(first_close, 12 * (n - 1))`): 2% of the commitment for five years, then 1.75%, 1.5%, 1.25%, 1% and 1%, looked up by year with `XLOOKUP`. That is 6,600,000 over the ten years.
- **The carry.** The managers take 20% of what comes back beyond what the investors paid in (36,600,000 with the fees), out of the last exit. There is no hurdle.
- **The ledger.** The investors' account is the table `ledger`: one row a cash flow, in the order of the plan's dates. Each row finds its date and amount with `XLOOKUP` by the company or the fee year, so a scenario that moves the exits moves the dates.
- **The term.** The fund runs ten years (`=DATE(YEAR(first_close) + 10, ...)`, to 2 March 2030), and `fund.exits_after_term` counts the sales after that.

| | |
| --- | --- |
| Paid in (investments and fees) | 36,600,000 |
| Exits, before carry | 68,850,000: 2.30 times the cost |
| Carry | 6,450,000 |
| **Return on the companies** (`XIRR` of the gross flows) | **15.68%** |
| **The investors' return** (after fees and carry) | **10.83%** |
| Back to the investors for each dollar paid in (TVPI, with `LET`) | 1.70 |
| The investors' flows at 12%, what they could earn elsewhere (`XNPV`) | -1,599,342.07 |

A portfolio that earns 15.7% gives its investors 10.8%: fees and carry take almost five points. At the 12% an investor could earn elsewhere for this risk, the fund is worth 1.6 million less than it costs them.

## Timing and the exit market

| Scenario | Return on the companies | The investors' return | At 12% | Exits after the term |
| --- | --- | --- | --- | --- |
| As planned | 15.68% | 10.83% | -1,599,342.07 | 0 |
| Every exit a year late, for the same price | 13.16% | 9.01% | -4,633,065.97 | 1 |
| A cold exit market: every sale a quarter lower | 9.93% | 5.55% | -8,040,127.25 | 0 |
| Both | 8.39% | 4.65% | -10,382,636.16 | 1 |

A year's delay leaves every multiple and the carry as they were, and still costs the investors 1.8 points of return and 3.0 million of value at 12%. Late, the last company (lumen) sells in September 2030, after the fund's ten years, so the investors would have to agree to extend it. Weighted 40/25/20/15, the fund is worth -4,963,424.20 at 12%.

The tornado moves each input alone. The exit market moves the investors' return most, from 4.37% (every sale at 70% of plan) to 16.11% (130%). Timing comes next: 13.56% if every exit comes a year early, 9.01% a year late. Carry matters least, from 11.65% at 10% to 9.94% at 30%. For the investors to earn 15% after fees and carry, every company would have to sell for 1.2324 times its planned multiple (`forge goal-seek`).

```bash
forge calculate examples/fund/fund.yaml --dry-run
forge scenarios examples/fund/fund.yaml
forge tornado examples/fund/fund.yaml
forge goal-seek examples/fund/fund.yaml --target fund.net_irr --value 0.15 --vary market.multiple_factor
forge export examples/fund/fund.yaml fund.xlsx
```

**Checked outside Forge:**

- **Python.** The fund was written again in Python from this description, not from the model's formulas: Excel's `EDATE` rule for the exit dates, and `XIRR` and `XNPV` on actual days over 365 from the first date, as Microsoft's documentation describes them, solved with scipy's `brentq`. Forge agrees on every return to 1e-9 and every value to the cent, in the plan, the four scenarios, the tornado and the goal seek. The Python is in `src/cli/commands/prediction/scenarios.rs` (`fund_example`), and the tests are in a `fund_*` module beside each command.
- **LibreOffice.** The workbook `forge export` writes was recalculated by LibreOffice 26.2, with a profile that recalculates every formula on load. It gives Forge's numbers to every digit it prints: 15.675607686942%, 10.8261445763612% and -1,599,342.07163067. With the exit market and the delay changed in the workbook itself (so no value Forge wrote is left to show), it gives the "both" scenario's 4.64737043725587% and -10,382,636.1608154.
- **Gnumeric.** Gnumeric 1.12 has no `XLOOKUP` or `LET`, so it cannot calculate the ledger. The ledger's dated flows are typed into `validation/gnumeric/tests/edge/edge_fund_cash_flows.yaml`, and Gnumeric and Forge agree on its `XIRR` and `XNPV`, including the late case, whose dates are out of order. That case runs in the Validation workflow.

**What the model leaves out:** the investors' 8% hurdle and the managers' catch-up, which most funds have (the carry here is 20% of all profit, from the last exit); management fees on invested capital after the investment period; recycling, fund expenses and the managers' own commitment; partial exits and follow-on investments. Each company is sold once, whole.
