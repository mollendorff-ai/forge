# How sure can we be about an average day?

After thirty days open, Bagel Lune counted the dozens it sold each day: 36, 50, 56, 42 and so on, up to 73 on the best day and down to 31 on the worst. The average is 50.7. The plan expected 1,402.5 dozens in November, which is 46.75 a day (55 a day at the start, times 0.85 for the slow season, over 30 days). Is the shop ahead of the plan, or is it just a good month? The data is in [`examples/bagel/first_month.yaml`](../../examples/bagel/first_month.yaml); the figures are made up. Numbers below are what Forge printed (10.0.0-rc.15).

## Resampling the month

A bootstrap answers by replaying the month. It draws thirty days from the thirty days (with repeats), takes the average, and does that 20,000 times. The spread of the averages shows how much a month of thirty days can move.

```
forge bootstrap examples/bagel/first_month.yaml

   Original statistic: 50.7000
   Bootstrap mean: 50.6995
   Bootstrap std error: 2.0163
   Bias: -0.0005

   Confidence Intervals:
      90% CI: [47.4333, 54.0667]
      95% CI: [46.8000, 54.7000]
```

## Reading it

The average day was 50.7, and with 95% confidence the real average is between 46.8 and 54.7. The plan's 46.75 is just below that interval, and the 90% interval (47.4 to 54.1) leaves it out too.

So the first month is, if anything, a little above the plan. It is not far above: the plan sits right at the edge of what thirty days can show, and the standard error of 2.02 dozen is the size of the doubt. With only thirty days it will not shrink fast; it falls with the square root of the number of days.

## What to take from it

You do not need a formula or a normal distribution for this: the thirty numbers are enough. The seed is fixed (3), so you get the same interval every time you run it. The first month may not look like the other eleven, and the interval says nothing about that.
