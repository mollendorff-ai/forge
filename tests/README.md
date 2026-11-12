# tests/

Integration tests that run Forge as a library on whole models:

- `bagel_example.rs`: every analysis command on the `examples/bagel` models, with numbers checked against calculations made outside Forge.
- `doc_examples.rs`: every model in docs/EXAMPLES.md runs, and the output the doc shows is what Forge gives.
- `broken_pipe.rs`: the CLI exits quietly when its output is closed (`forge ... | head`).
- `conventions.rs`: the conventions in `.asimov/project.yaml` that a test can check.

Unit tests are next to the code (`#[cfg(test)]`, in the same file or a sibling `*_tests.rs` module).

The end-to-end suites are in `validation/` (ADR-057). They run the release binary from `.github/workflows/validation.yml` on pull requests and weekly, and are not part of `cargo test`, which needs neither Gnumeric nor R:

- [forge-e2e-gnumeric](../validation/gnumeric): formulas against Gnumeric, and the Excel export and import round trip;
- [forge-e2e-r](../validation/r): statistics and analytics against R.

```bash
cargo test               # unit and integration tests
make test-integration    # integration tests only
```
