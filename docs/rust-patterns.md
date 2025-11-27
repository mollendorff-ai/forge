# Forge Rust Patterns

Reference document for advanced Rust patterns used in Forge. Read on-demand, not at session start.

## Error Handling

```rust
// Use thiserror for rich error context
#[derive(Error, Debug)]
pub enum ForgeError {
    #[error("Formula evaluation failed: {0}")]
    Eval(String),

    #[error("Column '{column}' in table '{table}': {reason}")]
    ColumnError { column: String, table: String, reason: String },
}
```

**Best Practices:**
- Use `Result<T, E>` everywhere, avoid panics in library code
- Custom error types with thiserror for rich context
- `?` operator for error propagation
- Add context with `.map_err()` when wrapping errors

**Avoid:**
- `unwrap()` or `expect()` in library code
- String errors (not type-safe)
- Losing error context when propagating

## Type-Driven Design

**Principles:**
- Make illegal states unrepresentable
- Use newtypes for domain concepts (TableName, ColumnName)
- Prefer enums over booleans for state
- Use builder pattern for complex construction

```rust
// Newtype pattern - prevents mixing up table and column names
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TableName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ColumnName(String);
```

## Zero-Copy Optimization

- Use `&str` instead of `String` when possible
- Use `Cow<str>` for conditional ownership
- Pass large structs by reference
- Avoid `clone()` unless necessary

## Testing Patterns

### Property-Based Testing (proptest)
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

### Snapshot Testing (insta)
- CLI output formatting
- Error messages
- Large calculation results
- `cargo insta review` to review changes

### Mutation Testing (cargo-mutants)
```bash
cargo install cargo-mutants
cargo mutants
```

### Fuzzing (cargo-fuzz)
```bash
cargo fuzz run parser_target
```

## Performance Patterns

### Benchmarking (criterion)
```bash
cargo bench
cargo bench --baseline <name>
cargo flamegraph --bench <name>
```

### Profiling
- Linux: perf, flamegraph
- Mac: Instruments, cargo-instruments
- Profile before optimizing, optimize hot paths only

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

| Tool | Purpose | Command |
|------|---------|---------|
| cargo-audit | Security vulnerabilities | `cargo audit` |
| cargo-deny | License/security policy | `cargo deny check` |
| cargo-machete | Unused dependencies | `cargo machete` |
| cargo-nextest | Fast parallel tests | `cargo nextest run` |
| miri | Undefined behavior | `cargo miri test` |
| cargo-tarpaulin | Code coverage | `cargo tarpaulin --out Html` |

## Documentation Style

**Do:**
- Use relatable metaphors and analogies
- Add examples that tell a story
- Celebrate what the code does well

**Don't:**
- Humor in error messages (user is frustrated!)
- Jargon without explanation
- Just say "invalid input" - be specific

```rust
/// Parses a YAML file into a delicious `ParsedModel` sandwich
///
/// Think of this as your YAML -> Rust translator. Reads the YAML,
/// checks if it's the new hotness (v1.0.0 arrays) or the classic
/// recipe (v0.2.0 scalars), and serves it up in a type-safe wrapper.
pub fn parse_model(path: &Path) -> ForgeResult<ParsedModel>
```
