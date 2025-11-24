# 🎯 Forge Features

Complete reference for all Forge capabilities.

## Core Features

### Formula Evaluation

- **Row-wise formulas**: `=revenue - expenses` (applies to each row)
- **Aggregation formulas**: `=SUM(revenue)`, `=AVERAGE(profit)`
- **Cross-table references**: `=pl_2025.revenue`
- **Array indexing**: `revenue[3]`
- **Nested functions**: `=ROUND(SQRT(revenue), 2)`

### 47+ Excel-Compatible Functions

**v1.1.0 Functions (27 new):**

- Conditional: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS, MAXIFS, MINIFS
- Math: ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER
- Text: CONCAT, TRIM, UPPER, LOWER, LEN, MID
- Date: TODAY, DATE, YEAR, MONTH, DAY

**v1.0.0 Functions:**

- Aggregation: SUM, AVERAGE, MAX, MIN, COUNT, PRODUCT
- Logical: IF, AND, OR, NOT, XOR
- Math: ABS

See full function reference: https://docs.rs/xlformula_engine

### Excel Integration

**Export (`forge export`):**

- YAML → Excel (.xlsx)
- Tables → Worksheets
- Formulas preserved
- Cross-table refs → Sheet refs

**Import (`forge import`):**

- Excel → YAML
- Reverse formula translation
- Round-trip verified

### Type System

**Column Types:**

- Numbers: `[100, 200, 300]`
- Text: `["Q1", "Q2", "Q3"]`
- Dates: `["2025-01", "2025-02", "2025-03"]`
- Booleans: `[true, false, true]`

**Type Safety:**

- Compile-time checks
- No mixed-type arrays
- Rust guarantees

### Performance

- **Validation**: <200ms for 850 formulas
- **Calculation**: <200ms for complex models
- **Zero tokens**: Local execution

For full docs, see [README.md](../README.md)
