# tests/

Integration tests that run Forge as a library on whole models:

- `bagel_example.rs`: every analysis command on the `examples/bagel` models, with numbers checked against calculations made outside Forge.
- `doc_examples.rs`: every model in docs/EXAMPLES.md runs, and the output the doc shows is what Forge gives.
- `broken_pipe.rs`: the CLI exits quietly when its output is closed (`forge ... | head`).
- `conventions.rs`: the conventions in `.asimov/project.yaml` that a test can check.

Unit tests are next to the code (`#[cfg(test)]`, in the same file or a sibling `*_tests.rs` module).

The end-to-end suites live in two other repositories and run the release binary from `.github/workflows/validation.yml` on pull requests and weekly:

- [forge-e2e-gnumeric](https://github.com/mollendorff-ai/forge-e2e-gnumeric): formulas against Gnumeric, and the Excel export and import round trip;
- [forge-e2e-r](https://github.com/mollendorff-ai/forge-e2e-r): statistics and analytics against R.

```bash
cargo test               # unit and integration tests
make test-integration    # integration tests only
```
