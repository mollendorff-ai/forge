# Integration Tests Migrated

As per **ADR-027**, all integration tests have been moved to the `forge-e2e` package.

## Test Structure (ADR-027)

- **Unit tests**: Inline in `src/` modules using `#[cfg(test)]`
- **Integration tests**: In `../forge-e2e/tests/integration/`
- **E2E tests**: In `../forge-e2e/tests/e2e/`

## Migration Details

All `.rs` and `.yaml` test files previously in this directory have been moved to:

```
/Users/rex/src/mollendorff/forge-e2e/tests/integration/
```

This includes:
- 34 `.rs` test files (array_calculator, formula_edge_cases, unit_*, validation, excel, parser, error, math_complete, xlformula_equivalence)
- 14 `.yaml` test fixture files (aggregation, date, edge_*, logical, lookup, math, text)

## Running Tests

To run the integration tests:

```bash
cd ../forge-e2e
cargo test
```

## Why This Directory Exists

This empty `tests/` directory is preserved for potential future use and to maintain the standard Rust project structure. Per ADR-027, `forge` is a library crate with unit tests inline in source files, while integration tests live in the separate `forge-e2e` package.
