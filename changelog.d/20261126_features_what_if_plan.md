### Added

- **The what-if commands take `--plan`, so one file can hold a plan and its uncertainty.** `forge scenarios`, `compare`, `goal-seek`, `break-even`, `sensitivity`, `tornado` and `audit` used to refuse a model with `MC.*` inputs. With `--plan` (`plan: true` over MCP, REST and in forge-wasm) they run it with each uncertain input at its plan figure, as `forge calculate --plan` does; a scenario can still set an uncertain input to its own number. On `examples/bagel/bagel_simulate.yaml` at plan, the break-even price is `bagel.yaml`'s 15.97 (ADR-076, addendum of 2026-11-26).

### Changed

- **Without `--plan`, a what-if command's refusal of an uncertain model says to add it.** It used to point to `forge calculate --plan`.
- **The MCP tool list's shared field descriptions are shorter**, to leave room for the new `plan` fields under its size ceiling.
