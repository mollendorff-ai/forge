# 2. A first model

In this lesson you write a model of the bagel shop from an empty file, one idea at a time: a group of inputs, a formula, a table of twelve months, the year's totals, a second file the model reads, and scenarios. Each step shows the whole file as it is at that point, so you can type the new lines or paste the whole file.

You need Forge installed ([lesson 1](01-first-ten-minutes.md#install-forge)) and a text editor. Work in the folder where you ran lesson 1, the one that holds `examples/`. If you would rather not type, the finished files are [`examples/guide/02-bagel.yaml`](../../examples/guide/02-bagel.yaml) and [`examples/guide/costs.yaml`](../../examples/guide/costs.yaml).

## A group of inputs and a formula

Make a file called `bagel.yaml` with this in it:

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  open_days_per_month: 30
  revenue_per_month: {formula: "=price_per_dozen * dozens_per_day * open_days_per_month"}
```

- `_forge_version` is the version of the model format, not of Forge. `5.1.0` is enough for everything in this lesson; [lesson 3](03-keeping-a-model.md#a-newer-format-forge-upgrade) raises it.
- `shop` is a **group of scalars**: one value for each name. A plain number, such as `price_per_dozen: 16`, is an **input**.
- `revenue_per_month` is a **formula**. In a group of scalars it is written `{formula: "=..."}`, because `forge calculate` writes its result next to it, as `value:`. Inside its own group a formula can name the others by their key (`price_per_dozen`); from anywhere it can use the full name (`shop.price_per_dozen`).

Calculate it:

```text
$ forge calculate bagel.yaml --dry-run
   📐 Scalars:
      shop.price_per_dozen = 16
      shop.dozens_per_day = 55
      shop.open_days_per_month = 30
      shop.revenue_per_month = 26400
📋 Dry run complete - no changes written
```

`--dry-run` shows the results and writes nothing, so the file stays as you typed it. [Lesson 3](03-keeping-a-model.md) is about what `forge calculate` writes without it.

### When Forge says no

Forge refuses a model it cannot read, and says why. Write the formula as a bare string, as a table's columns are written (you will meet them next):

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  open_days_per_month: 30
  revenue_per_month: "=price_per_dozen * dozens_per_day * open_days_per_month"
```

```text
$ forge calculate bagel.yaml --dry-run
Error: Validation error: 'shop.revenue_per_month' is a formula in a group of scalars; write it as `revenue_per_month: {formula: "=price_per_dozen * dozens_per_day * open_days_per_month"}` so its result has a place in the file (a bare formula string is a column, and belongs in a table)
```

Or misspell a name:

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  open_days_per_month: 30
  revenue_per_month: {formula: "=price_per_dozen * dozen_per_day * open_days_per_month"}
```

```text
$ forge calculate bagel.yaml --dry-run
Error: Formula evaluation error: shop.revenue_per_month: Unknown variable: dozen_per_day. Did you mean 'dozens_per_day'? (formula: =price_per_dozen * dozen_per_day * open_days_per_month)
```

Forge names the close name it knows. A name that is close to none gets no suggestion, and then the typo is yours to find. Nothing is calculated and nothing is written until the model reads. Put the formula back as it was before you go on.

## A table of twelve months

A shop does not sell the same every month. The bagel shop opens in November, and summer tourists and December lift its sales. A **table** holds one row per month, and its columns are lists of the same length:

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  monthly_growth: 0.03
  open_days_per_month: 30

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
```

- `month` and `season` are columns of data: twelve numbers each.
- `dozens` and `revenue` are **column formulas**, written as a bare `"=..."` string. A column formula is calculated row by row: in each row, `month` and `season` are that row's numbers. It can read the table's own columns by their names and a scalar by its full name, `shop.dozens_per_day`.
- `revenue_per_month` is gone: the table does that work now, month by month. The shop grows 3% a month, and `(1 + shop.monthly_growth) ^ (month - 1)` is that growth in month `month`.

```text
$ forge calculate bagel.yaml --dry-run
   📊 Table: months
      dozens = [1402.5, 2124.375, 1400.388, 1442.39964, 1671.38058285, 1817.16211146525, 2068.69560373649, 2333.68565964369, 2612.71329286196, 2691.09469164782, 2328.33512721369, 2283.98588669534]
      revenue = [22440, 33990, 22406.208, 23078.39424, 26742.0893256, 29074.593783444, 33099.1296597839, 37338.9705542991, 41803.4126857913, 43057.5150663651, 37253.3620354191, 36543.7741871254]
```

A table's columns must have the same length. Leave out December's `1.00`:

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  monthly_growth: 0.03
  open_days_per_month: 30

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
```

```text
$ forge calculate bagel.yaml --dry-run
Error: Formula evaluation error: Table 'months': Column 'season' has 11 rows, expected 12 rows
```

Put it back before you go on.

## The year, a formula over both

Costs, profit, and a group that adds up the year. The costs are columns too, and `profit` reads them:

```yaml
# bagel.yaml
_forge_version: "5.1.0"

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  monthly_growth: 0.03
  open_days_per_month: 30
  ingredients_per_dozen: 2.56
  staff_hours_per_day: 32
  wage_per_hour: 21.275
  rent_per_month: 4800

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
  ingredients: "=dozens * shop.ingredients_per_dozen"
  labour: "=shop.staff_hours_per_day * shop.open_days_per_month * shop.wage_per_hour"
  profit: "=revenue - ingredients - labour - shop.rent_per_month"
  cash: "=PREV(cash, 0) + profit"

year:
  revenue: {formula: "=SUM(months.revenue)"}
  profit: {formula: "=SUM(months.profit)"}
  margin: {formula: "=profit / revenue"}
  worst_month: {formula: "=MIN(months.profit)"}
```

- `labour` reads only scalars, so it is the same in every row; `profit` reads the row's own `revenue`, `ingredients` and `labour`, and the rent.
- `cash` is the money the shop has made since it opened. `PREV(cash, 0)` is `cash` in the row above, and 0 in the first row, so each month adds its profit to the month before. [The row above](../FEATURES.md#periods-and-the-row-above) says more.
- `year` is a group of scalars whose formulas read a whole column: inside `SUM`, `MIN`, `MAX` or `AVERAGE`, `months.profit` is the twelve months of it. That is the formula over both: a scalar from a table's column, which a column can read in turn.

```text
$ forge calculate bagel.yaml --dry-run
      cash = [-6374.4, -3046.8, -9449.58528, -15287.7341184, -18048.379084896, -18849.720306803, -16270.4513925846, -10129.7161269734, -238.849470908634, 10705.463184838, 16774.28729459, 22247.0576117754]
      year.revenue = 386827.449537828
      year.profit = 22247.0576117754
      year.margin = 0.0575115794868115
      year.worst_month = -6402.78528
```

The year makes about 22,247 on 386,827 of sales, a margin of 5.8%; the third month is the worst, and the cash the shop has made stays below zero until the tenth month (August, since month 1 is November).

## A second file: `_includes`

The ingredient price and the wage are not the shop's decisions: they are what its suppliers and staff cost, and they change on their own. Put them in a file of their own, so that a new price from a supplier is a change to that file only. Make `costs.yaml` next to `bagel.yaml`:

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

The loaded wage is the wage and the 15% the shop pays on top of it, 21.275 an hour. Then name the file in `bagel.yaml`'s `_includes`, and read its scalars with `@costs.` before their names:

```yaml
# bagel.yaml
_forge_version: "5.1.0"
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
```

```text
$ forge calculate bagel.yaml --dry-run
      year.profit = 22247.0576117754
```

The same year as before, to the last digit: the ingredients are 1.53 + 0.68 + 0.35 = 2.56 a dozen and the loaded wage 21.275 an hour, as the step before had them, but they come from `costs.yaml` now, and `shop` holds only what the shop decides. [Values from other files](../FEATURES.md#values-from-other-files-_includes) says more about `_includes`.

## Scenarios

A plan is one guess. A **scenario** is another: a name, a probability, and the inputs it changes. Add them at the end of `bagel.yaml`. This is the finished model:

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

`flour_shock` changes a value of the included file, by the name a formula uses for it. The probabilities add up to 1.

```text
$ forge scenarios bagel.yaml
   base (p=60%): The plan as written
      year.profit: 22,247.06
   flour_shock (p=10%): The price of flour up by half
      year.profit: 3,630.99
   rainy_year (p=30%): A slow start and a wet summer
      year.profit: -36,832.04
   Expected Values (probability-weighted):
      year.profit: 2,661.72   (range -36,832.04 to 22,247.06, P(>0) = 70%)
```

A wet year loses 36,832, and weighted by their chances the three come to 2,661.72: much less than the plan's 22,247. `forge calculate bagel.yaml --scenario rainy_year` calculates the whole model under one of them, and writes nothing. [Scenario analysis](../FEATURES.md#scenario-analysis) has the rest.

## What you wrote

| Part | What it is | Written |
| --- | --- | --- |
| `_forge_version` | The model format | `"5.1.0"` |
| A group of scalars | One value for each name | `shop:`, `year:` |
| An input | A number you type | `price_per_dozen: 16` |
| A scalar's formula | Calculated, its result written next to it | `{formula: "=..."}` |
| A table | Columns of the same length | `months:` |
| A column formula | Calculated row by row | `"=dozens * shop.price_per_dozen"` |
| The row above | A column's value one row up | `PREV(cash, 0)` |
| `_includes` | Another file's scalars | `@costs.labour.loaded_wage` |
| Scenarios | Other guesses, with their chances | `scenarios:` |

[`examples/bagel/bagel.yaml`](../../examples/bagel/bagel.yaml), the model of lesson 1, is the same shop with more in it. Two of its costs are not in yours, the firewood (950 a month, in its `costs.yaml`) and the card company's 2.6% of every sale, which is why its year makes 786.32 and yours 22,247. It also has the cost of the oven, a net present value and a tornado.

## Check it yourself

A result from Forge proves only that Forge ran. Work one out another way: the first month's revenue, by hand or on a calculator. In November the shop sells 55 dozen a day, at 85% for the season, for 30 days, and the growth has not started (`month - 1` is 0):

55 × 0.85 × 30 = 1,402.5 dozen, and 1,402.5 × 16 = 22,440.

That is the first number of `revenue` above. Then the year's profit from the scenarios' table: 0.6 × 22,247.06 + 0.1 × 3,630.99 + 0.3 × (−36,832.04) = 2,661.72.

## Your turn

The landlord may raise the rent to 6,000 a month. Add a scenario `rent_hike` with a probability of 0.1, take 0.1 from `base`, and find the year's profit in it, and the expected profit of all four.

<details>
<summary>The answer</summary>

The scenarios become:

```yaml
# bagel.yaml, with rent_hike
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
    probability: 0.5
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
  rent_hike:
    probability: 0.1
    description: The lease goes up to 6,000 a month
    scalars:
      shop.rent_per_month: 6000
```

```text
$ forge scenarios bagel.yaml
   rent_hike (p=10%): The lease goes up to 6,000 a month
      year.profit: 7,847.06
      year.profit: 1,221.72   (range -36,832.04 to 22,247.06, P(>0) = 70%)
```

The year makes `7,847.06` with the higher rent: 1,200 more each month, 14,400 in the year, off the plan's 22,247.06. The expected profit of the four falls to `1,221.72`.

</details>

Next: [3. Keeping a model](03-keeping-a-model.md), where the file changes and Forge keeps it honest.
