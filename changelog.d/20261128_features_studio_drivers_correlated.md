### Changed

- **Forge Studio's "what drives it" list says when a driver is correlated with another input.** An input named in a `monte_carlo.correlations` entry now has a line under it, such as `correlated -0.7 with harvest.price_per_tonne`, as `forge simulate` prints it, because its rank correlation with the output can come through the input it moves with. A model with no correlations is drawn as before.
