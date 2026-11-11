# Which months does the bagel shop lose money?

Bagel Lune opens in November, and its sales follow the seasons: a December rush, a slow winter, a busy summer. The plan is in [`examples/bagel/bagel.yaml`](../../examples/bagel/bagel.yaml); the figures are made up. Numbers below are what Forge printed (10.0.0-rc.15).

## The year hides a long winter

The plan earns 786.32 over twelve months. Month by month it looks very different:

```
forge calculate examples/bagel/bagel.yaml --dry-run
...
      profit = [-7908.027, 1493.57675, -7935.5334064, -7388.37940859199, -4406.16213970598, -2507.52294855811, 768.415716983582, 4219.61087311149, 7853.62956446177, 8874.45845139562, 4149.92625214749, 3572.3276568685]
```

| Month | Profit |
| --- | ---: |
| November | -7,908.03 |
| December | 1,493.58 |
| January | -7,935.53 |
| February | -7,388.38 |
| March | -4,406.16 |
| April | -2,507.52 |
| May | 768.42 |
| June | 4,219.61 |
| July | 7,853.63 |
| August | 8,874.46 |
| September | 4,149.93 |
| October | 3,572.33 |

Five months lose money, and January is the worst at -7,935.53 (the `year.worst_month_profit` line of the same run). Adding the months in order, the shop is 28,652.05 down by the end of April before the summer brings it back.

## How sure is that?

`forge simulate` runs the uncertain version 5,000 times and counts, for each month, the runs that lose money:

```
forge simulate examples/bagel/bagel_simulate.yaml
...
      months.profit by row:
          row           mean            P10            P50            P90     P(< 0)
            1       -9178.19      -12454.58       -9151.67       -5898.38    100.00%
            2          78.94       -4237.66         119.24        4382.57     48.72%
            3       -9205.27      -12480.11       -9179.52       -5929.26    100.00%
            4       -8666.53      -11995.39       -8636.92       -5332.60    100.00%
            5       -5730.14       -9385.25       -5704.48       -2092.63     98.30%
            6       -3860.67       -7710.00       -3831.95          -4.79     90.00%
            7        -635.07       -4854.07        -598.16        3580.98     56.92%
            8        2763.09       -1901.21        2817.71        7388.40     23.52%
            9        6341.26        1224.14        6377.91       11401.50      4.86%
           10        7346.41        2107.31        7387.59       12511.30      2.96%
           11        2694.47       -1959.01        2748.22        7307.21     24.08%
           12        2125.75       -2462.90        2169.96        6669.09     28.52%
```

Rows 1 to 12 are November to October. November, January and February lose in every run; March and April in nine runs out of ten or more. December is a toss-up (48.72%). Only July and August (rows 9 and 10) are safe, losing in under 5% of the runs.

## What to take from it

The yearly total says the shop breaks even. The monthly view says it needs about 28,700 in cash on top of the 95,000 fit-out just to reach May, and that the summer has to bring it back. If the cash is not there, the plan fails in the spring even if the year would have paid.
