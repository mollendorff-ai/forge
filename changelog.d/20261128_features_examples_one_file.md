### Changed

- **`examples/microgrid` and `examples/realestate` are one file each, the plan and its uncertainty.** `microgrid_risk.yaml` and `realestate_risk.yaml` are folded into `microgrid.yaml` and `realestate.yaml`: each of the five uncertain inputs has its plan figure as `value:` and its range as `formula:`, so the plan is the same and the two can no longer drift. Run the what-if commands with `--plan`; `forge simulate` draws the ranges, with the same percentiles as before. In Forge Studio each set has one model for both questions.

### Fixed

- **Forge Studio's formula suggestions offer a function with a dot.** Typing `=MC.` in a formula now offers `MC.Normal(`, `MC.PERT(` and the others; they were never offered, since a word with a dot was taken for a name.
