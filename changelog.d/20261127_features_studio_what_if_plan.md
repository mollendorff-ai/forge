### Fixed

- **Forge Studio's Analyze view runs an uncertain model's goal seek, break-even, tornado and scenarios at plan.** It sends `plan: true` to each, as its Model view does to `calculate`, where it used to show Forge's refusal. On `bagel_simulate.yaml` the break-even price and a goal seek are `bagel.yaml`'s; a model with no uncertain input gives the same answers as before (ADR-076, addendum of 2026-11-26).
