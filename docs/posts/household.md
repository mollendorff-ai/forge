# Where did October's money go?

One household's October: a budget ([`budget.yaml`](../../examples/household/budget.yaml)) and what actually came in and went out ([`actual.yaml`](../../examples/household/actual.yaml)). All figures are made up. Numbers below are what Forge printed (10.0.0-rc.15).

## The month in two numbers

```
forge calculate examples/household/budget.yaml --dry-run
      expenses.total = 3100
      result.left_over = 2500
      result.savings_rate = 0.446428571428571

forge calculate examples/household/actual.yaml --dry-run
      expenses.total = 3305.85
      result.left_over = 2044.15
      result.savings_rate = 0.382084112149533
```

The household planned to keep 2,500 and kept 2,044.15. The savings rate fell from 44.6% to 38.2%.

## Which lines moved

```
forge variance examples/household/budget.yaml examples/household/actual.yaml --threshold 10

Variable                      Budget       Actual     Variance      Var %   Status
income.side_work              400.00       150.00      -250.00     -62.5%     ⚠️ ❌
expenses.groceries            700.00       812.43       112.43      16.1%     ⚠️ ❌
expenses.transport            180.00        96.50       -83.50     -46.4%     ⚠️ ✅
expenses.utilities            140.00       163.20        23.20      16.6%     ⚠️ ❌
expenses.eating_out           250.00       388.75       138.75      55.5%     ⚠️ ❌
expenses.subscriptions         60.00        74.97        14.97      24.9%     ⚠️ ❌
result.left_over            2,500.00     2,044.15      -455.85     -18.2%     ⚠️ ❌
result.savings_rate           0.4464       0.3821     -0.06434     -14.4%     ⚠️ ❌

   ✅ Favorable: 1  ❌ Unfavorable: 9  · No direction: 0  = On target: 3  ⚠️ Alerts (>10%): 8
```

(The rows that stayed under 10%, rent, salary, phone and the totals, are left out here.)

With a 10% threshold, eight lines are flagged. Seven are unfavourable: the side work brought in 250 less than planned, groceries, utilities, eating out and subscriptions cost more, and so the two results (money left over and the savings rate) fell. The eighth, transport at 83.50 under budget, is the only good one.

## Why a variance can be good or bad

A 138.75 variance on eating out is bad, and a 138.75 variance on income would be good. Forge decides from the file: anything under `income` is better when higher, anything under `expenses` better when lower, and `result` says `better: higher` on its own lines. That is why the report counts "favorable" and "unfavorable" and not "plus" and "minus".

## What to take from it

The overspend is 205.85 on expenses (6.6%), under the threshold. The bigger part of the 455.85 shortfall is the side work that did not come in: 250. Eating out, up 55.5%, is the largest overspend. The savings rate is the number worth watching: it fell 6.4 points, although rent and salary did not move.
