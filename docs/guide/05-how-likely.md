# 5. How likely

The plan of [lesson 2](02-a-first-model.md) makes 22,247.06 in its first year, if 55 dozen a day come in at opening and the shop needs 32 staff hours a day. Neither is known. This lesson makes them uncertain, a range with a likeliest value, and asks Forge how likely a good year is: `forge simulate` calculates the model thousands of times, each time with values drawn from those ranges, and reports what came out.

You need lesson 2's `bagel.yaml` and `costs.yaml`, as it left them.

<details>
<summary>Starting here? The two files</summary>

```yaml
# bagel.yaml, as examples/guide/02-bagel.yaml has it
_forge_version: "5.1.0"
# Bagel Lune's first year: the model of lesson 2 of the guide
_includes:
  - file: costs.yaml
    as: costs

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  monthly_growth: 0.03
  open_days_per_month: 30
  staff_hours_per_day: 32
  rent_per_month: 4800

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
  ingredients: "=dozens * @costs.ingredients.per_dozen"
  labour: "=shop.staff_hours_per_day * shop.open_days_per_month * @costs.labour.loaded_wage"
  profit: "=revenue - ingredients - labour - shop.rent_per_month"
  cash: "=PREV(cash, 0) + profit"

year:
  revenue: {formula: "=SUM(months.revenue)"}
  profit: {formula: "=SUM(months.profit)"}
  margin: {formula: "=profit / revenue"}
  worst_month: {formula: "=MIN(months.profit)"}

scenarios:
  base:
    probability: 0.6
    description: The plan as written
    scalars: {}
  rainy_year:
    probability: 0.3
    description: A slow start and a wet summer
    scalars:
      shop.dozens_per_day: 45
  flour_shock:
    probability: 0.1
    description: The price of flour up by half
    scalars:
      "@costs.ingredients.flour_per_dozen": 2.30
```

```yaml
# costs.yaml, as examples/guide/costs.yaml has it
_forge_version: "5.1.0"
# What the bagel shop pays its suppliers and its staff. When a price
# changes, this is the file that changes.
ingredients:
  flour_per_dozen: 1.53
  sesame_per_dozen: 0.68
  honey_water_per_dozen: 0.35
  per_dozen: {formula: "=flour_per_dozen + sesame_per_dozen + honey_water_per_dozen"}
labour:
  wage_per_hour: 18.50
  payroll_load: 0.15
  loaded_wage: {formula: "=wage_per_hour * (1 + payroll_load)"}
```

</details>

## An uncertain input

Opening demand could be as low as 40 dozen a day or as high as 70, and 55 is the likeliest. Staff hours are uncertain too, but not evenly: a day runs long more easily than short, so from 28 to 38, likeliest at 32. In the model each becomes a formula that draws from a distribution, and keeps its number as `value:`:

```yaml
shop:
  price_per_dozen: 16
  dozens_per_day:
    value: 55
    formula: "=MC.PERT(40, 55, 70)"
```

- `MC.PERT(40, 55, 70)` is a distribution: the lowest, the likeliest and the highest value, with most draws near the likeliest. `MC.Triangular(28, 32, 38)` is a triangle with the same three. [Distributions](../FEATURES.md#distributions) lists the others: `MC.Normal`, `MC.Uniform`, `MC.Lognormal` and more.
- `value: 55` is the **plan**, the input as it was. `forge simulate` calculates the model once at the plan, beside the thousands of draws.

## What to ask

A `monte_carlo:` section at the end of the file sets the simulation and names the results it reports. Here is the whole model, lesson 2's with the two inputs uncertain and the section added; write it over `bagel.yaml`:

```yaml
# bagel.yaml, as examples/guide/05-bagel-uncertain.yaml has it
_forge_version: "5.1.0"
# Bagel Lune's first year, its demand and its staff hours uncertain: the
# model of lesson 5 of the guide
_includes:
  - file: costs.yaml
    as: costs

shop:
  price_per_dozen: 16
  dozens_per_day:
    value: 55
    formula: "=MC.PERT(40, 55, 70)"
  monthly_growth: 0.03
  open_days_per_month: 30
  staff_hours_per_day:
    value: 32
    formula: "=MC.Triangular(28, 32, 38)"
  rent_per_month: 4800

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
  ingredients: "=dozens * @costs.ingredients.per_dozen"
  labour: "=shop.staff_hours_per_day * shop.open_days_per_month * @costs.labour.loaded_wage"
  profit: "=revenue - ingredients - labour - shop.rent_per_month"
  cash: "=PREV(cash, 0) + profit"

year:
  revenue: {formula: "=SUM(months.revenue)"}
  profit: {formula: "=SUM(months.profit)"}
  margin: {formula: "=profit / revenue"}
  worst_month: {formula: "=MIN(months.profit)"}

scenarios:
  base:
    probability: 0.6
    description: The plan as written
    scalars: {}
  rainy_year:
    probability: 0.3
    description: A slow start and a wet summer
    scalars:
      shop.dozens_per_day: 45
  flour_shock:
    probability: 0.1
    description: The price of flour up by half
    scalars:
      "@costs.ingredients.flour_per_dozen": 2.30

monte_carlo:
  iterations: 5000
  seed: 7
  sampling: latin_hypercube
  outputs:
    - variable: year.profit
      percentiles: [10, 50, 90]
      thresholds: ["> 0", "> 20000"]
      expected_shortfall: [5]
    - variable: months.profit
      thresholds: ["< 0"]
```

- `iterations: 5000` years are drawn; `seed: 7` makes the draws the same each run, so the numbers below are the ones you get; `latin_hypercube` spreads the draws evenly over each range.
- `percentiles: [10, 50, 90]`: the profit that 10%, 50% and 90% of the years fall below.
- `thresholds`: the chance of each, here of making money at all and of making 20,000.
- `expected_shortfall: [5]`: the average of the worst 5% of years.
- `months.profit` with `"< 0"`: each month's chance of a loss.

`forge calculate` cannot draw from a distribution, and says so:

```text
$ forge calculate bagel.yaml --dry-run
Error: Validation error: shop.dozens_per_day, shop.staff_hours_per_day use MC.* distributions, which calculate and validate cannot evaluate. Run `forge simulate` on this file instead, or `forge calculate --plan` for its plan.
```

The what-if commands of [lesson 4](04-what-if.md) (`scenarios`, `compare`, `goal-seek`, `break-even`, `sensitivity`, `tornado` and `audit`) refuse an uncertain model in the same way, and take the same `--plan`: each uncertain input is then at its plan figure, the `value:` beside its formula, so one file holds the plan and its uncertainty. A scenario that sets an uncertain input still fixes it at its own number. The break-even price at plan is the one lesson 2's file gives, with plain numbers for both inputs:

```text
$ forge break-even bagel.yaml --output-var year.profit --vary shop.price_per_dozen --plan
📋 AT PLAN - each uncertain input at its plan figure
   shop.price_per_dozen = 15.08 (15.0798147287831) → year.profit = 0.000014
```

Releases after 10.0.0-rc.34 also write a distribution for you, `forge edit bagel.yaml --uncertain "shop.dozens_per_day==MC.PERT(40, 55, 70)"`, and add a `monte_carlo:` section if there is none.

## Reading forge simulate

```text
$ forge simulate bagel.yaml
   Input Distributions:
      shop.dozens_per_day mean=55.00 plan=55.00 (value) min=40.26 max=69.52
      shop.staff_hours_per_day mean=32.67 plan=32.00 (value) min=28.03 max=37.91
      year.profit:
         Mean:      17,140.57
         At plan:   22,247.06  (each uncertain input at its plan value)
         Percentiles:
            P10: -31,510.02
            P50: 17,328.10
            P90: 65,677.32
         P(year.profit > 0) = 66.82%
         P(year.profit > 20000) = 47.52%
         Mean of the worst 5% (expected shortfall): -57,351.53
         Drivers (rank correlation with the output):
            +0.907  shop.dozens_per_day
            -0.404  shop.staff_hours_per_day
```

- **The chance of a good year.** The shop makes money in 66.82% of the 5,000 years, and 20,000 or more in 47.52%. One year in ten loses 31,510.02 or more, and the worst one in twenty loses 57,351.53 on average.
- **The range.** Half the years fall between the 25th and 75th percentiles; the 10th to the 90th, from a loss of 31,510.02 to a profit of 65,677.32, is a wide range for a plan that says 22,247.06.
- **The drivers.** The rank correlation of each input with the profit: demand (+0.907) moves it most, and up with it; staff hours (−0.404) less, and down. The tornado of [lesson 4](04-what-if.md#the-tornado-which-input-matters-most) says the same, one input at a time.

## Why the mean is below the plan

The plan makes 22,247.06; the mean of the simulated years is 17,140.57, and `forge simulate` says why:

```text
$ forge simulate bagel.yaml
         The mean is 5,106.49 below the value at plan: the uncertain inputs' means are not their plan values (see Input Distributions), or the model is not linear in them.
```

The plan is each input at its likeliest value, and the likeliest value is not the average. The PERT from 40 to 70 is even about 55, so its mean is 55. The triangle from 28 to 38 is not: it reaches further above 32 than below, and its mean is (28 + 32 + 38) / 3 = 32.67 hours, the mean `Input Distributions` shows. The average year pays for more staff hours than the plan, so it makes less than the plan.

That is the most common thing a simulation tells a plan: an input written as its likeliest value, where the risk is lopsided, makes the plan better than the average year. [What price makes the bagel year pay?](../posts/bagel-price.md) shows it on the fuller shop of [`examples/bagel/bagel_simulate.yaml`](../../examples/bagel/bagel_simulate.yaml), where the flour price and the wage are uncertain too.

## Month by month

```text
$ forge simulate bagel.yaml
          row           mean        at plan            P10            P50            P90     P(< 0)
            1      -6,799.92      -6,374.40      -9,881.52      -6,780.50      -3,764.22    100.00%
            2       2,902.06       3,327.60      -1,355.64       2,927.24       7,135.73     19.66%
            3      -6,828.31      -6,402.79      -9,907.22      -6,807.14      -3,795.15    100.00%
           10      10,518.75      10,944.31       5,323.13      10,527.34      15,767.89      0.04%
```

November, the first month, and January, the third, lose money in every one of the 5,000 years; August, the tenth, almost never does. The cash the shop needs to get through its first winter is in the plan whatever the demand.

## Check it yourself

The gap between the plan and the mean can be worked out without a simulation. In this model the profit moves in a straight line with the staff hours: each hour a day costs 30 days × 12 months × 21.275 = 7,659 a year at the loaded wage. The triangle's mean is 0.67 hours above the plan's 32 (two thirds of an hour), so the average year should make 2/3 × 7,659 = 5,106 less than the plan. Forge's 5,106.49 differs only by the luck of 5,000 draws; with more `iterations` it comes closer. The demand does not add to the gap, because its mean is its plan.

## Your turn

How high must the price be for the year to make money nine times in ten? `forge simulate` can seek it: `--seek` names the goal, and `--vary` the input it moves between `--min` and `--max`.

<details>
<summary>The answer</summary>

```text
$ forge simulate bagel.yaml --seek "P(year.profit > 0) = 0.9" --vary shop.price_per_dozen --min 14 --max 20
   shop.price_per_dozen = 17.50
   P(year.profit > 0) there: 90.00%  (18.24% at 14, 99.76% at 20)
```

`17.50` a dozen. Forge runs the simulation for each price it tries, with the same draws each time, so the chance moves only with the price.

</details>

Next: [6. What will they do](06-what-will-they-do.md).
