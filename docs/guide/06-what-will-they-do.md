# 6. What will they do

The plans so far have no one on the other side. A bagel shop across the street does: if it cuts its price, Bagel Lune's plan is wrong, and what Bagel Lune should charge depends on what it expects the rival to charge. A **game** is a model of that: players, the choices each has (its **strategies**), and what each gets (its **payoff**) for every combination of choices. This lesson reads the price war of [`examples/bagel/price_war.yaml`](../../examples/bagel/price_war.yaml), whose payoffs come from the plan's own numbers.

## The payoffs, from the plan

Each month both shops either hold the plan's 16 a dozen or cut to 14. The file includes `bagel.yaml` and `costs.yaml`, and works out a month's margin for each shop in each case:

```yaml
# examples/bagel/price_war.yaml, part of it
market:
  dozens_per_day: 110       # both shops together, at the plan's price
  undercut_share: 0.70      # the cheaper shop's share when only one cuts
  cut_price: 14
  cut_growth: 0.05          # two $14 shops sell 5% more than two $16 ones
```

```text
$ forge calculate examples/bagel/price_war.yaml --dry-run
      market.margin_hold = 13.0238666666667
      market.margin_cut = 11.0758666666667
      market.R = 21489.38
      market.T = 25585.252
      market.S = 12893.628
      market.P = 19188.939
```

The four letters are the usual names for a game of two choices:

| Lune \ rival | holds | cuts |
| --- | --- | --- |
| **holds** | R = 21,489.38 each | S = 12,893.63 to Lune, T to the rival |
| **cuts** | T = 25,585.25 to Lune, S to the rival | P = 19,188.94 each |

Holding together pays both 21,489.38 a month; cutting alone pays the cutter 25,585.25 and leaves the other 12,893.63; cutting together pays both 19,188.94.

## The game

The game itself is a few lines, its payoffs formulas that read those scalars:

```yaml
# examples/bagel/price_war.yaml, part of it
games:
  price_war:
    description: Hold $16 a dozen, or cut to $14
    players: [lune, rival]
    strategies: [hold, cut]         # a tournament reads the first as cooperate
    symmetric: true                 # a cell is the payoff of the row's shop
    payoffs:
      hold: {hold: =market.R, cut: =market.S}
      cut: {hold: =market.T, cut: =market.P}
```

`symmetric: true` says the rival is in Lune's place: the same costs, the same table seen from its side. `forge game` works out what rational shops would do:

```text
$ forge game examples/bagel/price_war.yaml
   Strictly dominated, removed in this order:
      lune: hold is dominated by cut
      rival: hold is dominated by cut
   1 equilibrium:
      1. pure, stable: a population returns to it
         lune: cut (payoff 19,188.94)
         rival: cut (payoff 19,188.94)
```

- **Dominated.** Whatever the rival does, cutting pays Lune more: 25,585.25 against 21,489.38 if the rival holds, 19,188.94 against 12,893.63 if it cuts. So holding is **dominated**, and the same for the rival.
- **The equilibrium.** Both cut, each earning 19,188.94: neither can do better by changing alone. It is a **Nash equilibrium**, and a worse one for both than holding together, 2,300.44 a month each. This is the prisoner's dilemma, made by the plan's numbers, not assumed.

## Month after month

A price war is not played once: the shops meet every month, and a shop can answer last month's cut. The file's `rivalry` plays the month again and again between five pricing habits, with a 0.95 chance that both shops are still there the next month:

```text
$ forge game examples/bagel/price_war.yaml
rivalry: a tournament of price_war
   Ranking (payoff per round; share of moves that cooperate):
       1. discounter  21065.191  cooperates 0.000  (always_defect)
       2. matcher     20966.339  cooperates 0.810  (tit_for_tat)
       2. grudge      20966.339  cooperates 0.810  (grim_trigger)
       4. forgiving   20567.636  cooperates 0.873  (generous_tit_for_tat)
       5. steady      19770.230  cooperates 1.000  (always_cooperate)
   Collectively stable (no player does better against it than it does against itself): matcher, grudge, discounter
```

The shop that always cuts scores most on average, 21,065.19 a month, but it gets there by taking the shop that always holds. The matcher (charge what the rival charged last month) and the grudge (hold until the first cut, then cut for good) score 20,966.34, and are **collectively stable**: a street of matchers is not invaded by a discounter, which earns more in the first month and the low price after. [Tournaments](../FEATURES.md#tournaments) says how a tournament is written.

## Across the city

Say a tenth of Montreal's bagel shops sell at 14 today, and each month an owner is likelier to copy the prices of the shops earning more. The game's `dynamics:` follows the city's shares, a month at a time:

```text
$ forge game examples/bagel/price_war.yaml
      generation       hold        cut     payoff
               0   0.900000   0.100000  21,061.39
              11   0.452531   0.547469  19,685.06
              24   0.008664   0.991336  19,189.98
```

Cutting spreads: more than half the city cuts after 11 months and 99.1% after two years, when a shop earns 19,189.98 a month on average against 21,061.39 today. [Strategies that spread](../FEATURES.md#strategies-that-spread-dynamics) says how it is worked out.

## Check it yourself

R, the month when both hold, from the plan. A dozen at 16 brings 16 × (1 − 0.026) = 15.584 after the card fee, and its flour, sesame and honey water cost 12 / 9 × 1.15 + 12 × 6 / 1000 × 9.40 + 0.35 = 2.56013, so it leaves 13.02387. Each shop sells half of 110 dozen a day, for 30 days: 55 × 30 × 13.02387 = 21,489.38. Then the dilemma is two comparisons: T 25,585.25 is more than R 21,489.38, and P 19,188.94 is more than S 12,893.63, so cutting pays more whichever the rival chooses.

## Your turn

The file has a scenario, `loyal_regulars`, in which a cheaper shop takes only 55% of the street, because the regulars stay. Does cutting still spread across the city? What share of the shops still cuts after two years?

<details>
<summary>The answer</summary>

```text
$ forge game examples/bagel/price_war.yaml --scenario loyal_regulars
      lune: cut is dominated by hold
   After the last generation: hold 0.976461, cut 0.023539, average payoff 21,406.84
```

With loyal regulars, holding is the dominant strategy, and the cutters dwindle: `0.023539` of the city, 2.4%, still cuts after 24 months. The price war is not about prices: it is about how many customers a cut takes.

</details>

That is the end of the guide's lessons for now. The [reference](README.md#where-to-look-things-up) has the rest, and [the posts](../posts/README.md) ask more questions of the bagel shop.
