# Project Kittiwake: a takeover, from the board's model to year one

Valdane Instruments, a listed maker of industrial measuring instruments, bids for Quillon Sensors, a smaller listed maker of sensors for factories and grids. A private-equity fund, Thornvale Capital, is circling too. All three companies and every figure are made up. Millions, except per share.

Valdane offers **31.20 a share, a 30% premium** to Quillon's undisturbed price of 24.00: 1,560 for the shares, 1,660 with the debt that comes with the company, 10.4 times Quillon's EBITDA and 20 times its earnings. It pays 60% in cash, borrowed at 6.5%, and 40% in new Valdane shares at 40.00. The board wants to know what every board asks: **do the synergies pay for the premium, and does earnings per share go up or down?**

| File | What it is | Command |
| --- | --- | --- |
| `buyer.yaml`, `target.yaml` | Each company as the market sees it before the deal, included by the deal files as `@buyer` and `@target` | (included) |
| `deal.yaml` | The deal year by year over five years: the offer and how it is paid, synergies ramping in, the integration bill, the new interest and the amortisation of acquired intangibles, earnings per share with and without the deal, and the value the deal creates | `calculate`, `scenarios`, `tornado`, `break-even`, `goal-seek`, `sensitivity`, `compare`, `export` |
| `deal_risk.yaml` | The same deal with the five numbers a deal team can least promise left uncertain, and an earn-out to switch on | `simulate`, `simulate --scenario earnout`, `simulate --seek` |
| `deal_bid.yaml` | Thornvale may counter: open at 30%, or knock it out at 40%? | `decision-tree` |
| `deal_game.yaml` | Best and final offers, sealed: what each bidder does when the other's choice is worked out from its own numbers, not typed in | `game` |
| `halo_option.yaml` | Quillon's next platform, which the DCF values at less than nothing | `real-options` |
| `premiums.yaml` | The premiums paid in 24 comparable takeovers | `bootstrap` |
| `diligence.yaml` | A spike in the last quarter's sales: a big contract, or channel stuffing? | `bayesian` |
| `actual.yaml` | The deal underwritten again a year after closing | `variance` |

## The board's case

Valdane's deal team underwrites its own case for Quillon: 7% growth a year for five years (Quillon's management plans 9%), 2.5% after that, and an 8% return asked by Valdane's investors. Synergies are 40 a year of cost (two plants into one, purchasing, a head office), in full by year 3, and 60 a year of extra sales of Quillon's sensors through Valdane's channels at a 30% margin, from year 2 and in full by year 5. Integration costs 100, half of it in the first year. Fees are 2% of the price. Acquired customer lists and technology of 400 are written off over ten years.

| | |
| --- | --- |
| Quillon on its own (five years of cash, then growth for ever) | 1,312.56 |
| The synergies, net of the integration bill | 603.15: 1.68 times the 360 premium |
| Everything Valdane pays (shares, debt taken on, fees) | 1,691.20 |
| **Value created for Valdane's shareholders** | **224.51** (1.04 a Valdane share) |
| Return on the deal, before debt | 10.89%, against 8% asked |
| Net debt over EBITDA at closing | 2.04 (the agencies are comfortable to 2.5) |
| Quillon's shareholders end up owning | 7.2% of Valdane |

**The synergy value is a terminal value.** 91% of the synergies' worth comes after year 5, in the growth-for-ever line. The board is buying a promise about the 2030s.

**Earnings per share fall before they rise.** Reported earnings per share are 12.0% lower in year 1 than Valdane alone would earn, 1.1% lower in year 3, and higher only from year 4 (+2.8% in year 5). The press release will count it differently: before amortisation and integration costs, the deal is neutral in year 1 (+0.1%). Paying Quillon 20 times earnings with shares that trade at 15 dilutes; the amortisation of the intangibles and the integration bill do the rest.

| Year | 2027 | 2028 | 2029 | 2030 | 2031 |
| --- | --- | --- | --- | --- | --- |
| Reported EPS, Valdane alone | 2.60 | 2.73 | 2.87 | 3.01 | 3.16 |
| Reported EPS, with Quillon | 2.29 | 2.55 | 2.83 | 3.06 | 3.25 |
| Accretion | -12.0% | -6.7% | -1.1% | +1.8% | +2.8% |
| Adjusted accretion | +0.1% | +2.9% | +5.5% | +6.4% | +7.2% |

## What could change it

**The walk-away price is 35.60 a share** (`forge break-even`, a 48.3% premium): above it the deal destroys value even if everything goes to plan.

**Quillon's free cash flow can fall to 51.40 a year from 62, 17% below plan, before the deal creates nothing** (`forge break-even --vary @target.company.free_cash_flow_next_year`: an input of the included `target.yaml` is named as a scenario names it).

**For earnings per share not to fall in year 3,** Valdane needs cost synergies of 49.4 a year instead of 40, or to pay 79.8% of the price in cash instead of 60% (`forge goal-seek`). More cash is cheaper than Valdane's shares: debt costs 4.9% after tax, and Valdane's shares earn their buyers 6.5%. It is also more debt.

**How Valdane pays is a choice, not a chance** (`forge compare --scenarios base,all_cash,all_shares`). `all_cash` and `all_shares` sit beside the four weighted scenarios with no probability, so `forge scenarios` leaves them out and names them.

| | 60/40 (the plan) | All cash | All shares |
| --- | --- | --- | --- |
| New Valdane shares | 15.6 | 0 | 39.0 |
| Quillon's shareholders end up owning | 7.2% | 0% | 16.3% |
| Net debt over EBITDA at closing | 2.04 | 2.60 | 1.20 |
| Earnings per share in year 3 | -1.1% | +1.3% | -4.2% |
| Value created a Valdane share | 1.04 | 1.12 | 0.94 |

The deal creates 224.51 however it is paid: the mix decides who carries it. All cash makes year 3 accretive and takes leverage past the 2.5 the agencies are comfortable with; all shares keeps the balance sheet light and dilutes most.

**Four scenarios** (`forge scenarios`): as the board was shown (45%), 224.51; a slow integration, five years to merge the plants and a bill 50% over (25%), 166.93; Valdane's sales teams never selling Quillon's sensors (20%), 28.40; rates up 1.5 points before the debt is fixed and investors asking 9% (10%), -89.55. Weighted, 139.49. Without the revenue synergies, the deal barely pays for itself. The output names the two payment structures it leaves out, which have no probability.

**The tornado** (`forge tornado`) puts the deal team's valuation assumptions above every synergy: the discount rate swings the value from 428.94 (7.5%) to -89.55 (9%), and terminal growth from -32.02 (1.5%) to 391.25 (3%). Then cost synergies, the premium, revenue synergies and Quillon's growth. The integration bill, the number the integration team will be judged on, matters least: 237.74 at 80, 178.20 at 170.

**The banker's table** (`forge sensitivity`): value created at each premium and each run rate of cost synergies.

| Premium \ cost synergies | 20 | 30 | 40 | 50 | 60 |
| --- | --- | --- | --- | --- | --- |
| 20% | 110.31 | 228.61 | 346.91 | 465.21 | 583.51 |
| 30% | -12.09 | 106.21 | **224.51** | 342.81 | 461.11 |
| 40% | -134.49 | -16.19 | 102.11 | 220.41 | 338.71 |
| 50% | -256.89 | -138.59 | -20.29 | 98.01 | 216.31 |

## Under uncertainty

`deal_risk.yaml` leaves five numbers to chance, each with the board's figure as its `value:`. Cost synergies follow a PERT from 20 to 50, most likely 40; revenue synergies a triangle from 0 to 80, most likely 60; the integration bill a triangle from 90 to 170, most likely 100. They take 2 to 5 years to arrive, most likely 3, and Quillon grows 7% ± 2.5%. Every synergy leans against Valdane, as synergy plans do.

In 10,000 futures the deal creates **147.13 on average, against 224.51 at plan**. The synergies alone are worth 522.09 on average against the plan's 603.15: the plan is not the average. The deal creates value in 85.1% of futures; in the worst 5% it destroys 136.24 on average. Earnings per share are higher in year 3 in only 1.4% of them. Quillon's own growth drives the value most (rank correlation +0.77), then cost and revenue synergies; the integration bill barely shows (-0.08).

**To be 75% sure of creating value, pay at most a 34.2% premium, 32.22 a share** (`forge simulate --seek`), not the 35.60 the plan allows.

**An earn-out moves Quillon's growth risk to its sellers.** With `--scenario earnout`, Valdane pays a 20% premium now (28.80) and 4.80 a share more at the end of year 2 if Quillon's revenue that year reaches its own management's plan. The earn-out is paid in 21.2% of futures. The deal then creates value in 97.2% of them, and the worst 5% lose 20.31 on average instead of 136.24. The headline is 33.60 a share; on average Quillon's shareholders get about 29.82. Whether they accept depends on how much they believe their own plan.

## The contest, and the extras

**Thornvale** (`forge decision-tree`): if it counters a 30% offer (60% likely, a judgement), Valdane raises to 45%, and Thornvale gives up 65% of the time. If Thornvale wins, Quillon pays Valdane a break fee of 30, less 12 spent, 18. Opening at 30% is worth 109.54; a 40% knockout bid that Thornvale counters only 15% of the time, 91.73. The answer turns on a number nobody knows. Opening low is better as long as Thornvale counters it less than 69% of the time. The deal values in the tree are `deal.yaml`'s at each price.

**Sealed bids** (`forge game`): the tree's 60% is a judgement. In `deal_game.yaml` Quillon's board asks for best and final offers, sealed, and each bidder picks 30% or 40% not knowing the other's. Each payoff is the bidder's value created from `deal.yaml`: a point of premium costs both 12.24, so Valdane's 224.51 at 30% is 102.11 at 40%. Thornvale, a fund that owns a rival sensor maker, shares Valdane's view of Quillon but has cost cuts and not Valdane's sales channels: synergies worth 530 against 603.15, so 151.36 at 30% and 28.96 at 40%. The higher bid wins, a tie is won half the time, and the loser has spent 12. **Both bidding 30% is an equilibrium, and so is both bidding 40%.** Whoever expects the other to go to 40% goes there too, and both then expect 61.2 less (Valdane 45.05 against 106.25, Thornvale 8.48 against 69.68): the knockout bid is a trap both can fall into, not a way out. In the third equilibrium Thornvale bids 40% only 6.8% of the time, and Valdane 66.5%. If Thornvale's synergies are only 400 (`--scenario thornvale_cost_only`), 40% loses Thornvale money whatever Valdane does (−101.04 if it wins), so it bids 30%, and so does Valdane: 106.25 for Valdane, 4.68 for Thornvale.

**Comparable premiums** (`forge bootstrap`): the median of 24 comparable takeovers is 33.5%, with a 90% interval of 30.0% to 37.5%. Eight of the 24 paid less than 30%. Valdane's offer is at the bottom of the range, which is why Thornvale is a risk.

**Halo, Quillon's next platform** (`forge real-options`): launching it costs 260 for cash worth 220, so the DCF says -40 and leaves it out. Valdane can wait up to two years for the grid market to decide, so the platform is worth 37.62 (launch when it is worth more than about 286). The right to sell its patents for 90 if it flops adds 3.07. A small reason to pay above the DCF, not a large one.

**Diligence** (`forge bayesian`): one deal in twelve hides stuffed sales. Quillon's last quarter shows a spike: 23% likely stuffed. Its receivables jumped too: 62%. Then the grid contract turns out to be real: 39%, because the contract explains the spike away but not the receivables. Ask for the quality-of-earnings review before signing.

## One year after closing

`actual.yaml` is the deal underwritten again with what Valdane knows a year later. The plants merged faster and took out 46 a year instead of 40. But integration cost 125, the sales teams sell 35 a year of Quillon's sensors instead of 60, Quillon grows 5% and not 7%, and the floating part of the debt reset to 6.85%. **Value created halves, from 224.51 to 112.06** (`forge variance`). The synergies lost 27.27: the cost overdelivery almost pays for the revenue shortfall. **Quillon on its own lost 85.18.** The synergy-tracking report the board reads each quarter does not show the larger loss.

```bash
forge calculate examples/acquisition/deal.yaml --dry-run
forge scenarios examples/acquisition/deal.yaml
forge tornado examples/acquisition/deal.yaml
forge break-even examples/acquisition/deal.yaml --output valuation.value_created --vary offer.premium
forge break-even examples/acquisition/deal.yaml --output valuation.value_created --vary @target.company.free_cash_flow_next_year
forge goal-seek examples/acquisition/deal.yaml --target eps.year_3_accretion --value 0 --vary synergies.cost_run_rate --min 0 --max 200
forge goal-seek examples/acquisition/deal.yaml --target eps.year_3_accretion --value 0 --vary financing.cash_share --min 0 --max 1
forge sensitivity examples/acquisition/deal.yaml --vary offer.premium --vary2 synergies.cost_run_rate --range 0.20,0.50,0.10 --range2 20,60,10 --output-var valuation.value_created
forge compare examples/acquisition/deal.yaml --scenarios base,slow_integration,no_revenue_synergies,rates_up
forge compare examples/acquisition/deal.yaml --scenarios base,all_cash,all_shares
forge export examples/acquisition/deal.yaml kittiwake.xlsx
forge simulate examples/acquisition/deal_risk.yaml
forge simulate examples/acquisition/deal_risk.yaml --scenario earnout
forge simulate examples/acquisition/deal_risk.yaml --seek "P(valuation.value_created > 0) = 0.75" --vary offer.premium --min 0 --max 0.6
forge decision-tree examples/acquisition/deal_bid.yaml
forge game examples/acquisition/deal_game.yaml
forge game examples/acquisition/deal_game.yaml --scenario thornvale_cost_only
forge bootstrap examples/acquisition/premiums.yaml
forge real-options examples/acquisition/halo_option.yaml
forge bayesian examples/acquisition/diligence.yaml -q stuffing -e q4_spike=yes -e receivables=jumped
forge variance examples/acquisition/deal.yaml examples/acquisition/actual.yaml
```

**Checked outside Forge:** the deal was written again in Python from this description, not from the model's formulas. Its plan, scenarios, payment structures, tornado, walk-away price, sensitivity table, year-one variance and bidding tree agree with Forge's to 1e-9, and the two goal seeks to their tolerance. The option was priced on a hand-written binomial tree, and the diligence network by enumerating its 16 combinations. The comparable premiums were resampled 200,000 times in numpy. The Python is in `src/cli/commands/prediction/scenarios.rs`, and the tests are in an `acquisition_example` module beside each command. A test calculates each deal value in the bidding tree again from `deal.yaml`, so the tree cannot fall behind the model. The sealed bids' three equilibria, and the one with Thornvale's cost cuts only, were worked out by hand and with nashpy 0.0.43, and the games suite checks the file against Gambit (`validation/games`). The share of futures that pay the earn-out matches its closed form (21.2%). Three of the simulation's futures are calculated again with the plain model. The simulation's percentiles are Forge's own.

**What the model leaves out:** the new debt is never repaid, so the interest and the leverage stay where they start. The intangibles are a round number, not a purchase-price allocation, and there is no tax on them. Valdane's share price does not move when the deal is announced. The earn-out is in the value but not in earnings per share. In the bidding tree, Thornvale's chances of countering are typed in; the sealed bids work Thornvale's choice out instead, but only between two prices, sealed, with Thornvale's synergies a round number and both bidders sharing Valdane's view of Quillon.
