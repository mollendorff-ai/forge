### Added

- **`forge simulate` says when a driver's rank correlation is borrowed.** An input that is in a `monte_carlo.correlations` entry now shows the other input and the coefficient on its line in the drivers (`+0.359  @costs.wood.price_per_cord  (correlated 0.5 with assumptions.winter_lift)`), and the JSON (and so the `forge_simulate` tool over MCP and REST) has `correlated_with` on that driver. A wood price that looked like a driver of profit only rode on the cold winters it is correlated with; the line now says so. A model with no correlations prints as before.
