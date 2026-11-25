# 4. What if

A plan gives one answer for one set of inputs. Most questions about a plan are about other inputs: a wet year, a dearer flour, a lower price; what price breaks even; which input matters most. This lesson asks them of the bagel shop of [lesson 1](01-first-ten-minutes.md), [`examples/bagel/bagel.yaml`](../../examples/bagel/bagel.yaml), and nothing in it changes the file.

## Scenarios: other guesses, with their chances

`bagel.yaml` ends with four scenarios, written as you wrote yours in [lesson 2](02-a-first-model.md#scenarios):

```yaml
# examples/bagel/bagel.yaml, part of it
scenarios:
  base:
    probability: 0.5
    description: Plan as written
    scalars: {}
  tourist_summer:
    probability: 0.2
    description: Festival season and a blog write-up lift demand
    scalars:
      opening_dozens_per_day: 65
      monthly_growth: 0.04
  rainy_year:
    probability: 0.2
    description: Slow start, wet summer
    scalars:
      opening_dozens_per_day: 45
      monthly_growth: 0.02
  flour_shock:
    probability: 0.1
    description: Wheat price spike, flour up almost 50 percent
    scalars:
      "@costs.ingredients.flour_per_kg": 1.70
```

A scenario names the inputs it changes, by their key or their full name; `flour_shock` changes a value of the included `costs.yaml`, flour at 1.70 a kilo instead of 1.15. The probabilities add up to 1.

`forge scenarios` calculates the model once under each, and weighs them by their chances:

```text
$ forge scenarios examples/bagel/bagel.yaml
   base (p=50%): Plan as written
      year.profit: 786.32
   flour_shock (p=10%): Wheat price spike, flour up almost 50 percent
      year.profit: -16,943.27
   rainy_year (p=20%): Slow start, wet summer
      year.profit: -71,197.20
   tourist_summer (p=20%): Festival season and a blog write-up lift demand
      year.profit: 80,811.24
   Expected Values (probability-weighted):
      year.profit: 621.64   (range -71,197.20 to 80,811.24, P(>0) = 70%)
```

The year makes money in 70% of the weight (the base and the tourist summer), and loses 71,197.20 in a wet one. To see two scenarios side by side, input by input and result by result:

```text
$ forge compare examples/bagel/bagel.yaml --scenarios base,flour_shock
variable                                  base  flour_shock
year.profit                             786.32   -16,943.27
year.food_cost_share                      0.16       0.2058
```

`forge calculate examples/bagel/bagel.yaml --scenario rainy_year` prints the whole model under one scenario, and writes nothing. [Scenario analysis](../FEATURES.md#scenario-analysis) has the rest, among it `expect:`, which states what a scenario should give so that `forge validate` checks it.

## Break-even: the input that makes a result zero

At 16 a dozen the shop barely makes money. What price makes the year's profit exactly zero?

```text
$ forge break-even examples/bagel/bagel.yaml --output-var year.profit --vary assumptions.price_per_dozen
   assumptions.price_per_dozen = 15.97 (15.966607933396) → year.profit = -0.000063
   ✅ Within tolerance
```

Forge calculates the model again and again with other prices, narrowing in until the profit is zero to within 0.0001, and leaves the file as it was. 15.97 is the price to two decimals, and in brackets every digit Forge keeps: the profit is zero at 15.966607933396, and 79.88 at 15.97, so put the long one in the model to check it.

## Goal seek: the input that gives a result you name

Break-even is goal seek with a goal of zero. Name another goal: how few staff hours a day would make the year 60,000?

```text
$ forge goal-seek examples/bagel/bagel.yaml --target year.profit --value 60000 --vary assumptions.staff_hours_per_day
   assumptions.staff_hours_per_day = 24.27 (24.2687453046069) → year.profit = 60,000.00
```

From 32 hours a day to 24.27: a quarter fewer, which says more about the plan's staffing than about the target.

## A table of two inputs

`forge sensitivity` calculates a result for every pair of two inputs, in steps you choose. Price against the opening demand:

```text
$ forge sensitivity examples/bagel/bagel.yaml --vary assumptions.price_per_dozen --range 15,18,1 --vary2 assumptions.opening_dozens_per_day --range2 45,65,10 --output-var year.profit
assumptions.price_per_dozen      45.0000      55.0000      65.0000
                    15.0000   -75,730.20   -22,761.80    30,206.60
                    16.0000   -56,463.56       786.32    58,036.20
                    17.0000   -37,196.91    24,334.44    85,865.79
                    18.0000   -17,930.27    47,882.56   113,695.39
```

The plan is in the middle, 786.32. At 45 dozen a day not even 18 a dozen saves the year; at 65, 15 a dozen still makes 30,206.60.

## The tornado: which input matters most

Each input has a range it could be in. The `tornado:` section of `bagel.yaml` names five, each with a low and a high; `forge tornado` moves one at a time from its low to its high, the others at plan, and sorts them by how far the profit moves:

```text
$ forge tornado examples/bagel/bagel.yaml
   Base value: 786.32
   assumptions.opening_dozens_per_day |██████████████████████████████| swing 114,499.75  (output -56,463.56 at low, 58,036.20 at high)
   assumptions.price_per_dozen        |████████████████████████      | swing 94,192.48  (output -46,309.92 at low, 47,882.56 at high)
   assumptions.staff_hours_per_day    |████████████████████          | swing 76,590.00  (output 46,740.32 at low, -29,849.68 at high)
   assumptions.rent_per_month         |██████                        | swing 24,000.00  (output 10,386.32 at low, -13,613.68 at high)
   assumptions.card_fee_rate          |█                             | swing 4,641.93  (output 3,880.94 at low, -760.99 at high)
```

How many people come in moves the year most: 114,499.75 between 45 and 65 dozen a day. The card fee, which the owner might have spent a week negotiating, moves it by 4,641.93. [Tornado diagrams](../FEATURES.md#tornado-diagrams) says how to write the section.

A tornado moves one input at a time. Inputs that move together, and the chance of each value, are [lesson 5](05-how-likely.md)'s.

## Check it yourself

The tornado's price bar is a line you can draw by hand. Each dollar on the price of a dozen adds a dollar for each of the 24,176.72 dozen the year sells (lesson 1), less the card company's 2.6%: 24,176.72 × 0.974 = 23,548.12 of profit a dollar. The tornado moves the price from 14 to 18, four dollars: 4 × 23,548.12 = 94,192.48, the swing it printed.

The same line gives the break-even price: the profit at 16 is 786.32, so it reaches zero 786.32 / 23,548.12 = 0.0334 dollars lower, at 15.9666, which is 15.97.

## Your turn

The plan opens at 55 dozen a day. How many does the shop need on its first day for the year to break even?

<details>
<summary>The answer</summary>

```text
$ forge break-even examples/bagel/bagel.yaml --output-var year.profit --vary assumptions.opening_dozens_per_day
   assumptions.opening_dozens_per_day = 54.86 (54.8626511689632) → year.profit = 0.000001
```

`54.86` dozen a day: the plan is a seventh of a dozen above breaking even. A plan that close to zero is one to ask "how likely?" of, which is the next lesson.

</details>

Next: [5. How likely](05-how-likely.md). The posts answer more questions of the same shop: [what price makes the year pay](../posts/bagel-price.md) and [which months lose money](../posts/bagel-seasons.md).
