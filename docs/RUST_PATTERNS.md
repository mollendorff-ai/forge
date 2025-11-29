# Rust Patterns for Forge

Essential patterns for autonomous development.

## Error Handling

**Approach:** `thiserror` for rich error context

```rust
#[derive(Error, Debug)]
pub enum ForgeError {
    #[error("Formula evaluation failed: {0}")]
    Eval(String),
    #[error("Column '{column}' in table '{table}': {reason}")]
    ColumnError { column: String, table: String, reason: String },
}
```

### Best Practices
- Use `Result<T, E>` everywhere, avoid panics in library code
- Custom error types with thiserror for rich context
- `?` operator for error propagation
- Add context with `.map_err()` when wrapping errors

### Avoid
- `unwrap()` or `expect()` in library code
- String errors (not type-safe)
- Losing error context when propagating

## Type-Driven Design

### Principles
- Make illegal states unrepresentable
- Use newtypes for domain concepts (TableName, ColumnName)
- Prefer enums over booleans for state
- Use builder pattern for complex construction

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TableName(String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ColumnName(String);
```

## Zero-Copy

- Use `&str` instead of `String` when possible
- Use `Cow<str>` for conditional ownership
- Pass large structs by reference
- Avoid `clone()` unless necessary

## Testing Patterns

| Type | Tool | Command |
|------|------|---------|
| Property-based | proptest | - |
| Snapshot | insta | `cargo insta review` |
| Mutation | cargo-mutants | `cargo mutants` |
| Fuzzing | cargo-fuzz | `cargo fuzz run parser_target` |

### Property-Based Example

```rust
proptest! {
    #[test]
    fn test_sum_commutative(a: f64, b: f64) {
        let sum1 = calculate("=SUM(a, b)");
        let sum2 = calculate("=SUM(b, a)");
        assert!((sum1 - sum2).abs() < 1e-10);
    }
}
```

## Performance

| Task | Tool |
|------|------|
| Benchmarking | criterion (`cargo bench`) |
| Profiling | perf/flamegraph (Linux), Instruments (Mac) |

**Rule:** Profile before optimizing, optimize hot paths only.

## Architecture Patterns

### Strategy Pattern

```rust
trait Calculator {
    fn calculate(&mut self) -> ForgeResult<CalculationResult>;
}
struct ArrayCalculator { /* v1.0.0 */ }
impl Calculator for ArrayCalculator { /* ... */ }
```

### Builder Pattern

```rust
let table = TableBuilder::new("sales")
    .add_column(revenue_col)
    .add_formula("profit", "=revenue - expenses")
    .build()?;
```

## Code Quality Tools

| Tool | Purpose |
|------|---------|
| `cargo audit` | Security vulnerabilities |
| `cargo deny` | License/security policy |
| `cargo machete` | Unused dependencies |
| `cargo nextest` | Fast parallel tests |
| `miri` | Undefined behavior detection |
| `cargo tarpaulin` | Code coverage |

## Documentation Style

### Do
- Use relatable metaphors and analogies
- Add examples that tell a story
- Celebrate what the code does well

### Don't
- Humor in error messages (user is frustrated!)
- Jargon without explanation
- Just say "invalid input" - be specific
