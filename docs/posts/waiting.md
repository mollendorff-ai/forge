# What is it worth to wait a year before signing the lease?

Bagel Lune has an offer on the Verdun lease, open for one year. Signing today means 180,000 to open a shop worth 240,000 today: a net present value of 60,000. But nobody has to sign today. The model is [`examples/bagel/wait_option.yaml`](../../examples/bagel/wait_option.yaml); the figures are made up. Numbers below are what Forge printed (10.0.0-rc.15).

The shop's value is uncertain (35% a year), the risk-free rate is 4%, and waiting costs the 6% a year of profit that the shop would have earned. Two rights are valued:

- **Wait up to a year** before paying the 180,000.
- **Sell the oven and fit-out** for 110,000 if the shop fails.

```
forge real-options examples/bagel/wait_option.yaml

   Traditional NPV (invest now, no flexibility): $60000.00

   Options (each valued on its own):
      Wait up to a year (Defer): option value $64604.68
         Project value with this option: $64604.68
         Rule of thumb: exercise when value > $198000
      Sell the oven and fit-out if it fails (Abandon): option value $279.72
         Project value with this option: $60279.72
         Rule of thumb: exercise when value < $132000

   Project value with the best option (Wait up to a year): $64604.68
   Option values overlap, so they are compared, not added.

   Decision: ACCEPT (with Wait up to a year)
```

## Reading it

The right to wait is worth 64,604.68, against 60,000 for signing now: 4,604.68 more. The reason is that waiting lets you skip the bad outcomes. If the shop turns out worth less than 180,000 you do not sign; if it is worth more, you sign anyway and have lost only a year of its profit.

The right to sell the oven is worth 279.72. It matters only in the bad cases, when the shop's value falls below 132,000. The two values overlap, so Forge compares them and does not add them.

The rules of thumb are the prices at which each right pays: sign only once the shop's value is above 198,000, and sell the oven if its value falls below 132,000.

## What to take from it

The answer says "wait", but by 4,605 on a 60,000 project, and it rests on the 35% volatility, which is a guess. The pop-up in [the second shop post](second-shop.md) is another way of buying information before committing; this model does not compare the two.
