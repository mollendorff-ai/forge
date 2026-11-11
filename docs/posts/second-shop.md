# Open the second bagel shop now, or try a pop-up first?

Bagel Lune is thinking about a second shop in Verdun. The made-up numbers are in [`examples/bagel/second_shop.yaml`](../../examples/bagel/second_shop.yaml): each outcome is the value of three years of profit, and the costs sit on the choices. Numbers below are what Forge printed (10.0.0-rc.15).

## Three choices

- **Open now** for 180,000. If Verdun demand is strong (45%) the shop is worth 420,000; if weak (55%), 90,000.
- **Run a three-month pop-up stall first** for 12,000, then decide. A busy stall (50%) makes strong demand likely (80%); a quiet one makes it unlikely (10%). Opening after the stall costs 190,000.
- **Stay with one shop**, which is worth nothing extra.

## Opening blind

The same file with the `popup_first` branch deleted, so that only "open now" and "stay" are left:

```
forge decision-tree second_shop_without_popup.yaml
   Expected Value: $58500.00
      Best case:  $240000.00
      Worst case: $-90000.00
      P(value > 0): 45.0%
```

Opening now is worth 58,500 on average, but it loses 90,000 more often than not (55%).

## Testing first

```
forge decision-tree examples/bagel/second_shop.yaml

   Optimal Path:
      → Second shop? → popup_first
   ...
   Expected Value: $61500.00

   Decision Policy:
      At "open_after_busy": choose "open"
      At "open_after_quiet": choose "walk_away"
      At "root": choose "popup_first"

      Outcomes under the optimal policy:
         $198000.00 with probability 40.0%
         $-12000.00 with probability 50.0%
         $-117000.00 with probability 10.0%
```

The pop-up is worth 61,500, which is 3,000 more than opening now. The policy is: run the stall; open the shop only if the stall was busy; walk away if it was quiet.

## What to take from it

The 3,000 is small next to the risks, and the shape matters more than the average. With the pop-up, half of the time you lose only the stall's 12,000, and the worst outcome (-117,000, one time in ten) is worse than opening blind. What you buy is a much better chance of not losing 90,000. Whether that is worth it depends on how much loss the business can take, which no expected value can tell you.

The probabilities are guesses. Change them in the file and run it again; the 3,000 can disappear.
