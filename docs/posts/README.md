# Eight questions, answered with Forge

Each post asks one question about a made-up household or business, runs the command that answers it on a file in [`examples/`](../../examples/), shows what Forge printed, and says what to take from it. All figures are invented. The numbers were printed by Forge 10.0.0-rc.15, and `tests/posts.rs` runs the commands and fails if a post stops matching the output.

**Bagel Lune** is a bagel shop in Mile End, Montreal ([`examples/bagel`](../../examples/bagel/README.md)):

1. [What price makes the bagel year pay?](bagel-price.md): break-even, simulation and a probability goal (`break-even`, `simulate --seek`).
2. [Which months does the bagel shop lose money?](bagel-seasons.md): the plan month by month, and the chance of a loss in each (`calculate`, `simulate`).
3. [Open the second shop now, or try a pop-up first?](second-shop.md): a decision tree (`decision-tree`).
4. [Was the slow week the rain or the roadwork?](slow-week.md): two causes of one effect (`bayesian`).
5. [What is it worth to wait a year before signing the lease?](waiting.md): real options (`real-options`).
6. [How sure can we be about an average day?](first-month.md): a bootstrap interval (`bootstrap`).

**At home:**

7. [What do 300 extra dollars a month do to a mortgage?](mortgage.md) ([`examples/mortgage`](../../examples/mortgage/README.md); `calculate`, `goal-seek`).
8. [Where did October's money go?](household.md) ([`examples/household`](../../examples/household/README.md); `calculate`, `variance`).
