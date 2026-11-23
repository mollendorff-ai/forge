# 1. The first ten minutes

Forge calculates financial models written as text. A model is a YAML file: inputs you type, formulas such as `=revenue - costs`, and tables of months or products. Forge works out every formula, the same way every time, and the file stays something you can read, change and compare with `git diff`.

In this lesson you open a model of a bagel shop, calculate it, change its price and see what moved. It is the model the rest of the guide builds on.

## Without installing anything

[Forge Studio](https://forge.mollendorff.ai/?example=bagel) is Forge in your browser tab, made for a phone as much as a laptop. That link opens the bagel shop: its inputs, its results, and the formula under each one. Drag the price and the year's profit moves. Nothing you type leaves the tab.

The rest of this guide uses the command line. To go on there, install Forge.

## Install Forge

On Linux or macOS, this puts the `forge` binary in `~/.local/bin` and the examples in a folder called `examples` where you run it:

```bash
curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples examples
```

It downloads the latest release for your system, checks it against the release's checksums, and builds nothing. If it says that `~/.local/bin` is not on your `PATH`, run Forge as `~/.local/bin/forge` or add that folder to your `PATH`. On Windows, take the zip from the [release page](https://github.com/mollendorff-ai/forge/releases). The [README](../../README.md#quick-start) has Homebrew and Cargo too.

Every command in this guide runs in the folder that holds `examples/`. In a clone of this repository, that is its top folder.

## Calculate the bagel shop

Bagel Lune is a made-up wood-fired bagel shop, planned over its first twelve months. Calculate it:

```text
$ forge calculate examples/bagel/bagel.yaml --dry-run
      year.revenue = 386827.449537828
      year.profit = 786.320361712362
      year.margin = 0.2%  (0.00203274189215848)
      year.worst_month_profit = -7935.5334064
📋 Dry run complete - no changes written
```

`--dry-run` prints the results and leaves the file as it is; without it, `forge calculate` writes each result into the file, next to its formula ([lesson 3](03-keeping-a-model.md#forge-calculate-writes-the-results)). The first year sells 386,827.45 of bagels and makes 786.32: it barely breaks even. The worst month loses 7,935.53.

Above those lines Forge prints the months, a table with a row for each month, and every input. Open [`examples/bagel/bagel.yaml`](../../examples/bagel/bagel.yaml) in a text editor to see where they come from:

- `assumptions:` are the inputs, such as `price_per_dozen` (16) and `opening_dozens_per_day` (55).
- `months:` is the table. `season` is a column of twelve numbers; `revenue: "=dozens * assumptions.price_per_dozen"` is a formula worked out in each row.
- `year:` adds the months up: `profit: {formula: "=SUM(months.profit)"}`.
- `_includes:` reads the costs of flour and staff from [`costs.yaml`](../../examples/bagel/costs.yaml), next to it.

[Lesson 2](02-a-first-model.md) writes a model like it from an empty file.

## Change the price

A dollar more a dozen. You can change the file in your editor; `forge edit` changes it for you, and keeps the rest of it, comments included, as it was:

```text
$ forge edit examples/bagel/bagel.yaml --set assumptions.price_per_dozen=17
✅ Wrote bagel.yaml (as it was: examples/bagel/bagel.yaml.bak)
```

The file as it was is kept next to it, as `bagel.yaml.bak`. Calculate again:

```text
$ forge calculate examples/bagel/bagel.yaml --dry-run
      year.revenue = 411004.165133942
      year.profit = 24334.4413523276
      year.dozens_sold = 24176.7155961142
```

The same shop at 17 a dozen makes 24,334.44 in its first year instead of 786.32.

## Where a number comes from

`forge audit` follows a result down to what was typed in:

```text
$ forge audit examples/bagel/bagel.yaml year.profit
   Formula: =SUM(months.profit)
   └─ months.profit (Column[months]) = 12 rows: -6541.992 … 5796.929911 =revenue - costs
         └─ assumptions.price_per_dozen (Scalar) = 17
```

The year's profit is the sum of the months' profit, each month's is its revenue less its costs, and so on down to the inputs, the price among them. A number in a Forge model is never a number someone forgot to update: it is the end of a tree anyone can read.

## Check it yourself

A result from Forge proves only that Forge ran. Work this one out another way.

At 17 instead of 16, each dozen sold earns a dollar more, and the year sells 24,176.72 dozen, so the revenue should rise by 24,176.72: 386,827.45 + 24,176.72 = 411,004.17, as Forge says. The profit should rise by less, because the card company takes 2.6% of every sale (`card_fee_rate` in the file): 24,176.72 × (1 − 0.026) = 23,548.12, and 786.32 + 23,548.12 = 24,334.44.

## Your turn

At 16 a dozen the shop makes 786.32; at 17, 24,334.44. Somewhere a little below 16 it makes nothing at all. Find that price with `forge edit` and `forge calculate --dry-run`, to the cent.

<details>
<summary>The answer</summary>

Trying prices gives a profit just below zero at 15.96 and just above it at 15.97. Forge can find it for you:

```text
$ forge break-even examples/bagel/bagel.yaml --output-var year.profit --vary assumptions.price_per_dozen
   assumptions.price_per_dozen = 15.97 → year.profit = -0.000082
```

The shop breaks even at `15.97` a dozen. [Goal Seek, Break-Even and Sensitivity](../FEATURES.md#goal-seek-break-even-and-sensitivity) says how Forge finds it.

</details>

To put the price back, run `forge edit examples/bagel/bagel.yaml --set assumptions.price_per_dozen=16`.

Next: [2. A first model](02-a-first-model.md).
