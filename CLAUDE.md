# Forge

@.asimov/warmup.yaml
@.asimov/asimov.yaml
@.asimov/green.yaml

Rules: 4hr max, 1 milestone, tests pass, ship it.

ON SESSION START: Immediately read .asimov/roadmap.yaml, run `asimov validate`, present next milestone. Do NOT wait for user prompt.

```bash
cargo test && cargo clippy -- -D warnings
```
