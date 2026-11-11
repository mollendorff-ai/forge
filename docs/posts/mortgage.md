# What do 300 extra dollars a month do to a mortgage?

A 450,000 CAD mortgage at 4.79% over 25 years. The model is [`examples/mortgage/mortgage.yaml`](../../examples/mortgage/mortgage.yaml); the figures are made up. Numbers below are what Forge printed (10.0.0-rc.15).

## One detail that matters

Canadian fixed rates compound twice a year, but payments are monthly. The monthly rate is therefore `(1 + 4.79%/2)^(1/6) - 1`, which is 0.003952, and not 4.79%/12. Using the second gives a payment of 2,575.89, which is 12.20 a month too much. The model has the first.

## As agreed

```
forge calculate examples/mortgage/mortgage.yaml --dry-run
      terms.payment = 2563.68762956532
      as_agreed.total_interest = 319106.288869595
```

The payment is 2,563.69 a month and the interest over 25 years is 319,106.29, 71% of the amount borrowed.

## With 300 more each month

```
      with_extra.payment = 2863.68762956532
      with_extra.total_interest = 254509.405647968
      saved.interest = 64596.8832216271
      saved.months = 53
      paid_off.payments = 247
      paid_off.last_payment = 42.2487749021881
      paid_off.year = 2047
      paid_off.month_of_year = 6
```

The loan is paid off in 247 payments instead of 300. The last one is 42.25, in June 2047. The extra payments save 64,596.88 of interest and 53 months, which is 4.4 years. In the first year, 21,056.17 of the payments is interest and 13,308.08 is principal.

## The other way round

You may not want to pay 300 for no reason. Ask for a goal instead: how much more must be paid to save 80,000?

```
forge goal-seek examples/mortgage/mortgage.yaml --target saved.interest --value 80000 --vary loan.extra_per_month
   loan.extra_per_month = 397.04 → saved.interest = 80,000.00
```

397.04 a month, which is 97.04 more than the 300: those 97.04 buy another 15,403.12 of saving.

## Two ways to say the same thing

The model says this twice. The scalar groups use closed formulas (`PMT`, `NPER`). The `schedule` table is the loan month by month for 300 months, each month opening with what the month before closed with (`PREV`). The `check` group is the difference between the two, and is all zeros:

```
      check.interest = 0
      check.last_payment = 0
      check.payments = 0
```

If you build a model like this, build both and let one check the other.
