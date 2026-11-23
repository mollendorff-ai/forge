# 3. Keeping a model

A model is not written once. Prices change, a month's actuals come in, someone renames an input. This lesson is about changing a model the way it is kept: Forge writes the results into the file, `forge edit` makes the changes, `forge validate` tells you when a stored result is out of date, and `git` keeps the history.

You need the two files of [lesson 2](02-a-first-model.md), `bagel.yaml` and `costs.yaml`, in the folder you work in.

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

## forge calculate writes the results

The model reads, and nothing is calculated in it yet:

```text
$ forge validate bagel.yaml
✅ The formulas are valid.
   4 formula(s) have no stored value yet; run 'forge calculate' to store them
```

So far every `forge calculate` had `--dry-run`. Without it, Forge writes each result into the file:

```text
$ forge calculate bagel.yaml
💾 4 values updated in bagel.yaml
   Backup saved to bagel.yaml.bak
```

Open `bagel.yaml`. The four formulas of `year` have their results next to them now, as a spreadsheet's cell holds its formula and its value:

```yaml
# bagel.yaml, after forge calculate
year:
  revenue: {formula: "=SUM(months.revenue)", value: 386827.449537828}
  profit: {formula: "=SUM(months.profit)", value: 22247.0576117754}
  margin: {formula: "=profit / revenue", value: 0.0575115794868115}
  worst_month: {formula: "=MIN(months.profit)", value: -6402.78528}
```

Nothing else in the file changed: the comments, the quotes, the blank lines and the order of the keys are as you typed them. A table's columns are not written, only the scalars' results, which is what someone reading the file on GitHub, or a tool that reads YAML, wants to see without running Forge. `forge report` prints them side by side, from the file alone:

```text
$ forge report bagel.yaml
group     revenue     profit   margin  worst_month
year   386,827.45  22,247.06  0.05751    -6,402.79
```

`bagel.yaml.bak` is the file as it was before. Each time Forge writes a file, it keeps the one before as `.bak`, so one step back is always there.

## Change it with forge edit

The shop raises its price to 17. Change it in your editor, or let `forge edit` change it:

```text
$ forge edit bagel.yaml --set shop.price_per_dozen=17
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

`forge edit` changes that one line and no other byte of the file. It makes a change only when the model still reads afterwards, and says why when it cannot:

```text
$ forge edit bagel.yaml --set shop.price_per_dozne=17
Error: Validation error: set_value shop.price_per_dozne: it is not in the model (bagel.yaml). No file was changed
```

## forge validate: the results are out of date

The price is 17 now, but the results in the file are still those of 16: `forge edit` changes the model's text, and only `forge calculate` writes results. `forge validate` checks every stored result against its formula:

```text
$ forge validate bagel.yaml
❌ Found 4 value mismatches!
   The formulas are valid; the stored values are out of date.
   year.profit
      Current:  22247.057612
      Expected: 46423.773208
      Diff:     24176.715596
💡 Run 'forge calculate' to update values
Error: Validation error: Stored values are out of date (the formulas are valid): run forge calculate
```

It fails, with an exit code of 1, so a script, a git hook or a continuous integration job can stop a file whose numbers are stale from going further. Calculate, and it passes:

```text
$ forge calculate bagel.yaml
💾 4 values updated in bagel.yaml
```

```text
$ forge validate bagel.yaml
✅ All tables are valid!
✅ All scalar values match their formulas!
```

## Keep it in git

The file is text, so git keeps its history and shows each change. Once, in the folder:

```bash
git init
git add bagel.yaml costs.yaml
git commit -m "Bagel Lune: the first year's plan"
```

and a line in `.gitignore` so the backups stay out of it:

```gitignore
*.bak
```

Then the rent goes up:

```text
$ forge edit bagel.yaml --set shop.rent_per_month=5200
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```text
$ forge calculate bagel.yaml
💾 3 values updated in bagel.yaml
```

`git diff` now shows what a reviewer needs to see: the rent's line, and the three results it moved (the year's profit, margin and worst month; the revenue did not move). A change to a model is a change to its text, and the results changed with it are in the same commit.

## Add and rename

`forge edit` adds an input or a formula at the end of its group, in the layout of the lines there:

```text
$ forge edit bagel.yaml --add "year.food_cost_share==SUM(months.ingredients) / revenue"
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

The `==` says that what follows is a formula; `--add year.tax_rate=0.15` adds an input. And it renames a name with every formula, scenario and analysis that reads it:

```text
$ forge edit bagel.yaml --rename shop.staff_hours_per_day=staff_hours
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```yaml
# bagel.yaml, after forge edit --rename
  labour: "=shop.staff_hours * shop.open_days_per_month * @costs.labour.loaded_wage"
```

`forge edit --help` lists the rest: `--delete`, `--formula`, and making an input uncertain, which [Monte Carlo simulation](../FEATURES.md#monte-carlo-simulation-forge-simulate) needs.

## A newer format: forge upgrade

`_forge_version: "5.1.0"` is the format the file is written in. Later formats add to it: a date as a scalar and `format:` (how a number is shown, `0.0%`) in 5.4.0, games in 5.5.0. When you want one of them, `forge upgrade` raises the file and the files it includes, and changes nothing else:

```text
$ forge upgrade bagel.yaml --to 5.4.0
   ↑ costs.yaml (v5.1.0 → v5.4.0)
   ↑ bagel.yaml (v5.1.0 → v5.4.0)
✅ 2 file(s) upgraded to v5.4.0
```

```yaml
# bagel.yaml, after forge upgrade
_forge_version: "5.4.0"
```

The included file is raised first, and each keeps a `.bak`. `--dry-run` shows what would change. [The model file](../FEATURES.md#the-model-file) lists every format.

## Check it yourself

`forge validate` said that at 17 a dozen the year's profit would rise by 24,176.715596 (and the revenue by as much). This model has no card fees, so the profit rises by exactly what the revenue does, and the revenue by a dollar for each dozen sold. Add up the twelve months' `dozens` from lesson 2:

1,402.5 + 2,124.375 + 1,400.388 + 1,442.39964 + 1,671.38058 + 1,817.16211 + 2,068.69560 + 2,333.68566 + 2,612.71329 + 2,691.09469 + 2,328.33513 + 2,283.98589 = 24,176.72

## Your turn

The shop wants two more staff hours a day. Before you calculate, ask `forge validate` what that does to the year's profit.

<details>
<summary>The answer</summary>

```text
$ forge edit bagel.yaml --set shop.staff_hours=34
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```text
$ forge validate bagel.yaml
   year.profit
      Current:  41623.773208
      Expected: 26305.773208
      Diff:     15318.000000
Error: Validation error: Stored values are out of date (the formulas are valid): run forge calculate
```

The year's profit falls by `15318.000000`: two hours a day, 30 days a month, 12 months, at the loaded wage of 21.275, is 2 × 30 × 12 × 21.275 = 15,318. Then `forge calculate bagel.yaml` writes it, and `forge validate` passes again.

</details>

That is a model kept: changed with `forge edit`, calculated, checked by `forge validate`, and kept in git, one commit for each change.

Next: [4. What if](04-what-if.md).
