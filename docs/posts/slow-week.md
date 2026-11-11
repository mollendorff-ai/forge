# Was the slow week the rain or the roadwork?

One week at Bagel Lune, sales were slow. Two things could have done it: rain, or the roadwork on the street. The model is [`examples/bagel/slow_week.yaml`](../../examples/bagel/slow_week.yaml): a Bayesian network where rain (30% of weeks) and roadwork (20%) both make slow sales more likely. The probabilities are made up. Numbers below are what Forge printed (10.0.0-rc.15).

## Before the week

```
forge bayesian examples/bagel/slow_week.yaml -q roadwork
      yes             |██████| 20.00%
      no              |████████████████████████| 80.00%

forge bayesian examples/bagel/slow_week.yaml -q sales
      slow            |███████████| 38.00%
      normal          |██████████████████| 62.00%
```

Roadwork is on the street one week in five, and 38% of weeks are slow.

## Sales were slow

```
forge bayesian examples/bagel/slow_week.yaml -q roadwork -e sales=slow
      yes             |████████████| 40.00%
      no              |██████████████████| 60.00%
```

A slow week doubles the chance that roadwork is the cause, from 20% to 40%. It is still more likely there was none.

## And it also rained

```
forge bayesian examples/bagel/slow_week.yaml -q roadwork -e sales=slow -e weather=rainy
      yes             |████████| 27.27%
      no              |█████████████████████| 72.73%
```

Rain already explains a slow week, so roadwork becomes less likely: 27.27%. The reverse works as well. Given a slow week, rain is 52.11% likely; if we also know there was roadwork, rain drops to 35.53% (`-q weather -e sales=slow -e roadwork=yes`).

## What to take from it

Two causes compete for the same effect. When one is confirmed, the other loses weight. This is easy to get wrong in your head, because both causes feel guilty at once. The network keeps the arithmetic straight: it is the same as checking all the combinations of weather, roadwork and sales and counting.

The result is only as good as the table of probabilities in the file. Those numbers should come from your own weeks, not from a guess.
