# ADR-015: Function Scalar/Array Classification

## Status

Accepted

## Date

2025-12-09

## Context

Forge supports two schema versions:
- **v1.0.0 schema**: Scalar-only formulas. Each named column is a single value. No tables or arrays.
- **v5.0.0 schema**: Full array support with tables, ranges, and dynamic arrays.

The demo build (forge-demo) targets v1.0.0 schema for sales/evaluation purposes. It must work without tables or array context.

Some Excel-compatible functions require array context to work properly:
- `VLOOKUP(value, table, col, exact)` - needs a table to search
- `SUMIF(range, criteria, sum_range)` - needs ranges to filter
- `UNIQUE(array)` - needs and returns arrays

We need to classify functions to:
1. Ensure demo functions work with v1.0.0 schema
2. Document which functions require v5.0.0 array support
3. Prevent accidentally including array-only functions in demo

## Decision

Add a `scalar: bool` field to the `FunctionDef` struct in the function registry:

```rust
pub struct FunctionDef {
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub syntax: &'static str,
    pub demo: bool,
    /// Scalar compatible (true = works with v1.0.0 schema without tables/arrays)
    pub scalar: bool,
}
```

### Classification Rules

**Scalar functions (scalar: true)**:
- Take individual scalar values as arguments
- Return a single scalar value
- Work in v1.0.0 schema without tables
- Examples: `ABS(x)`, `IF(cond, true_val, false_val)`, `ROUND(num, digits)`

**Array-only functions (scalar: false)**:
- Require array/range inputs to function correctly
- May return arrays
- Only work in v5.0.0 schema with tables
- Examples: `VLOOKUP`, `SUMIF`, `UNIQUE`, `FILTER`

### Enforcement

A test enforces that all demo functions are scalar:

```rust
#[test]
fn test_demo_functions_are_scalar() {
    let non_scalar_demo: Vec<_> = demo_functions()
        .filter(|f| !f.scalar)
        .map(|f| f.name)
        .collect();
    assert!(
        non_scalar_demo.is_empty(),
        "Demo functions must be scalar (v1.0.0 compatible), but found: {:?}",
        non_scalar_demo
    );
}
```

### Array-Only Functions (24 total)

| Category | Functions |
|----------|-----------|
| Array (5) | UNIQUE, FILTER, SORT, SEQUENCE, RANDARRAY |
| Conditional (6) | SUMIF, SUMIFS, COUNTIF, COUNTIFS, AVERAGEIF, AVERAGEIFS |
| Aggregation (5) | MAXIFS, MINIFS, RANK.EQ, LARGE, SMALL |
| Statistical (3) | PERCENTILE, QUARTILE, CORREL |
| Lookup (7) | INDEX, MATCH, VLOOKUP, HLOOKUP, XLOOKUP, OFFSET, INDIRECT |

### Demo Function Changes

INDEX and MATCH removed from v1.0.0 (require array context):
- **Before**: 49 demo functions (3 Lookup: INDEX, MATCH, CHOOSE)
- **After**: 47 demo functions (1 Lookup: CHOOSE)

Reason: Both require array context that doesn't exist in v1.0.0 schema.

## Consequences

### Positive

1. Clear documentation of function requirements
2. Compile-time enforcement of demo/scalar compatibility
3. Easier reasoning about schema version compatibility
4. Prevents future bugs where array-only functions are added to demo

### Negative

1. Demo has 2 fewer functions (INDEX, MATCH removed)
2. Additional maintenance burden to classify new functions

### Neutral

1. Enterprise build unaffected (has all 159 functions)
2. Existing v5.0.0 users unaffected

## Related

- ADR-012: Feature Flags for Demo/Enterprise Binary Split
- ADR-013: Function Registry
