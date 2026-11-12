# What price makes the bagel year pay?

Bagel Lune is a made-up wood-fired bagel shop in Mile End, Montreal. Its plan for the first twelve months sells a dozen bagels for 16. The plan is in [`examples/bagel/bagel.yaml`](../../examples/bagel/bagel.yaml). All the figures here are invented, and each number below is what Forge printed (10.0.0-rc.15).

## The plan says it just about works

```
forge calculate examples/bagel/bagel.yaml --dry-run
...
      year.revenue = 386827.449537828
      year.profit = 786.320361712362
```

A year profit of 786.32 on 386,827.45 of revenue is a margin of 0.2%. So how far is the price from the point where the shop stops losing money?

```
forge break-even examples/bagel/bagel.yaml --output year.profit --vary assumptions.price_per_dozen
...
   assumptions.price_per_dozen = 15.97 → year.profit = -0.000063
```

At 15.97 the plan earns nothing. The planned 16 is 3 cents above it.

## But the plan is one guess

Opening demand is not exactly 55 dozen a day, and the staff hours and the prices of flour and wages move too. `bagel_simulate.yaml` is the same plan with those four made uncertain. `forge simulate` runs it 5,000 times:

```
forge simulate examples/bagel/bagel_simulate.yaml
...
         P(year.profit > 0) = 34.76%
```

At a price of 16 the year makes money in about one run in three. A price that breaks even on the plan is not a price you can count on.

## Asking the question the other way round

Instead of trying prices by hand, ask for the price that makes the year pay in 80% of the runs:

```
forge simulate examples/bagel/bagel_simulate.yaml --seek "P(year.profit > 0) = 0.8" --vary price_per_dozen --min 14 --max 24

   assumptions.price_per_dozen = 18.32
   P(year.profit > 0) there: 80.00%  (2.92% at 14, 100.00% at 24)
   22 simulations of 5000 iterations, the same draws each time
```

The answer is 18.32 a dozen, 2.32 more than planned. At 14 the year pays in 2.92% of the runs; at 24, in all of them.

## What to take from it

Break-even on the plan (15.97) says the shop is nearly safe. The simulation says it is a bet you lose two times in three. The distance between 15.97 and 18.32 is the price of uncertainty.

One limit to keep in mind: in this model the number of bagels sold does not depend on the price. A higher price in real life loses customers, and Forge will not do that for you unless you write it into the formulas.
