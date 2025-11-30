# Forge

@warmup.yaml
@ethics.yaml

Rules: 4hr max, 1 milestone, tests pass, ship it.

ON SESSION START: Immediately read roadmap.yaml, run `asimov-mode validate`, present next milestone. Do NOT wait for user prompt.

```bash
cargo test && cargo clippy -- -D warnings
```
