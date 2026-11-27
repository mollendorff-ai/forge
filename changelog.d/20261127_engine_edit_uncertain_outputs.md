### Fixed

- **`forge edit --uncertain` asks the simulation it adds about every result, the year's profit included.** When a model had no `monte_carlo:` section, the one written for it listed only the results no other formula reads, so a profit that a margin reads was left out: on the bagel shop, six outputs without revenue and profit. It now lists every scalar with a formula, in the file's order.
