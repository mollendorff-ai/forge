# Forge v1.0.0: Array Model Design Specification

**Status:** DRAFT - Strategic Design Phase
**Date:** 2025-11-23
**Goal:** Zero-error financial models with trivial Excel export
**Principle:** Excel compatibility = Excel data structures + Excel formulas

---

## Executive Summary

**The Problem:** v0.2.0 uses discrete scalars. Excel uses columns. This mismatch makes Excel export complex and limits formula power.

**The Solution:** v1.0.0 introduces **column arrays** that map 1:1 with Excel columns.

**Key Insight:** Financial models are fundamentally tabular. YAML should reflect that.

---

## Design Principles

1. **Excel Native:** YAML structure maps directly to Excel structure
2. **Zero Ambiguity:** One way to do things, no confusion
3. **Type Safe:** Arrays are homogeneous (all numbers or all text)
4. **Formula Compatible:** Functions work on columns (like Excel)
5. **Backwards Compatible:** v0.2.0 models still work
6. **Testable:** Every feature has unit + E2E + round-trip tests
7. **Financial Grade:** Precision matters, errors are unacceptable

---

## Array Model Syntax

### Basic Table Structure

```yaml

# This is a TABLE - maps to Excel sheet

quarterly_revenue:
  # Column arrays - each becomes an Excel column
  period: [Q1, Q2, Q3, Q4]
  revenue: [100000, 120000, 150000, 180000]
  expenses: [80000, 90000, 100000, 110000]

  # Formula applied row-wise (like Excel dragging down)
  profit: "=revenue - expenses"

  # Formula on entire column (aggregation)
  total_revenue: "=SUM(revenue)"
  avg_profit: "=AVERAGE(profit)"
```text

**Excel export:**

```text
| Period | Revenue | Expenses | Profit |
|--------|---------|----------|--------|
| Q1     | 100000  | 80000    | 20000  |
| Q2     | 120000  | 90000    | 30000  |
| Q3     | 150000  | 100000   | 50000  |
| Q4     | 180000  | 110000   | 70000  |
|--------|---------|----------|--------|
| Total  | 550000  |          |        |
| Avg    |         |          | 42500  |
```text

---

## Model Types

### Type 1: Table (Column Arrays)

**Use case:** Time-series data, metrics over periods

```yaml
table_name:
  column1: [val1, val2, val3]
  column2: [val4, val5, val6]
  formula_column: "=column1 + column2"  # Row-wise
  summary: "=SUM(column1)"  # Scalar
```text

**Detection:** Object with array values
**Excel mapping:** Sheet with columns
**Formula scope:**

- Array formula → Applied to each row
- Aggregation → Single cell

### Type 2: Scalar (v0.2.0 Compatible)

**Use case:** Single values, assumptions, constants

```yaml
assumptions:
  platform_take_rate:
    value: 0.10
    formula: null

  gross_margin:
    value: 0.90
    formula: "=1 - platform_take_rate"
```text

**Detection:** Object with `value` and `formula` keys
**Excel mapping:** Named cells or key-value table
**Formula scope:** Scalar only

### Type 3: Mixed (Table + Scalars)

**Use case:** Tables with summary rows

```yaml
quarterly_data:
  # Table section
  period: [Q1, Q2, Q3, Q4]
  revenue: [100, 120, 150, 180]

  # Scalar summaries
  annual_total:
    value: 550
    formula: "=SUM(revenue)"

  avg_quarter:
    value: 137.5
    formula: "=AVERAGE(revenue)"
```text

**Detection:** Mix of arrays and {value, formula} objects
**Excel mapping:** Table + summary rows below

---

## Formula Semantics

### Row-wise Formulas (Applied to Each Row)

```yaml
profit: "=revenue - expenses"
```text

**Evaluation:**

- `profit[0] = revenue[0] - expenses[0]`
- `profit[1] = revenue[1] - expenses[1]`
- ...

**Excel equivalent:** `=A2-B2` dragged down

**Requirements:**

- All referenced columns must have same length
- Error if mismatched lengths
- Result is an array of same length

### Column Aggregation (Scalar Result)

```yaml
total: "=SUM(revenue)"
max_revenue: "=MAX(revenue)"
```text

**Evaluation:**

- `total = revenue[0] + revenue[1] + ... + revenue[n]`

**Excel equivalent:** `=SUM(A2:A5)`

**Requirements:**

- Function must be aggregation (SUM, AVG, MAX, MIN, COUNT)
- Result is a scalar
- Can reference multiple columns: `=SUM(revenue, expenses)`

### Cross-Column References

```yaml

# In table1

revenue: [100, 120, 150]

# In table2 (different table/sheet)

growth: "=table1.revenue[1] / table1.revenue[0] - 1"
```text

**Requirements:**

- Explicit table.column notation
- Array indexing with [n]
- Or full array reference: `=SUM(table1.revenue)`

---

## Type System

### Data Types

1. **Number Arrays:** `[100, 120.5, 150]`
   - All elements must be numeric
   - Excel: Number format

2. **Text Arrays:** `["Q1", "Q2", "Q3"]`
   - All elements must be strings
   - Excel: Text format

3. **Date Arrays:** `["2025-01", "2025-02", "2025-03"]`
   - ISO date strings
   - Excel: Date format

4. **Boolean Arrays:** `[true, false, true]`
   - Boolean values
   - Excel: TRUE/FALSE

### Type Checking

```yaml

# Valid - homogeneous

revenue: [100, 120, 150]

# INVALID - mixed types (ERROR!)

mixed: [100, "Q2", true]  # ❌ Type error

# Valid - explicit conversion

quarter_num: "=NUMBERVALUE(quarter_text)"
```text

**Enforcement:**

- Parser validates array homogeneity
- Type mismatch = immediate error
- No implicit conversions (fail fast)

---

## Excel Export Mapping

### Table → Excel Sheet

```yaml
quarterly_revenue:
  period: [Q1, Q2, Q3, Q4]
  revenue: [100, 120, 150, 180]
  profit: "=revenue * 0.2"
```text

**Excel output:**

Sheet name: `quarterly_revenue`

```text
A      | B       | C
-------|---------|-------
period | revenue | profit
Q1     | 100     | 20
Q2     | 120     | 24
Q3     | 150     | 30
Q4     | 180     | 36
```text

**Formula mapping:**

- YAML: `profit: "=revenue * 0.2"`
- Excel: `C2: =B2*0.2` (dragged to C5)

### Scalar → Named Cell

```yaml
assumptions:
  tax_rate:
    value: 0.25
    formula: null
```text

**Excel output:**

Sheet name: `assumptions`

```text
A         | B
----------|-----
tax_rate  | 0.25
```text

Named range: `tax_rate` = 0.25

### Summary Rows

```yaml
quarterly:
  revenue: [100, 120, 150, 180]

  total:
    value: 550
    formula: "=SUM(revenue)"
```text

**Excel output:**

```text
A      | B
-------|-------
       | revenue
Q1     | 100
Q2     | 120
Q3     | 150
Q4     | 180
-------|-------
Total  | 550     # =SUM(B2:B5)
```text

---

## Function Library (v1.0.0)

### Aggregation (Column → Scalar)

- `SUM(column)` - Sum all values
- `AVERAGE(column)` - Mean
- `MAX(column)` - Maximum value ⭐ NEW
- `MIN(column)` - Minimum value ⭐ NEW
- `COUNT(column)` - Count non-null values ⭐ NEW
- `PRODUCT(column)` - Multiply all values

### Row-wise Operations (Column → Column)

- `ABS(column)` - Absolute value of each element
- `ROUND(column, decimals)` - Round each element ⭐ NEW
- `column1 + column2` - Element-wise addition
- `column * scalar` - Scalar multiplication

### Conditional Aggregation ⭐ NEW

- `SUMIF(column, condition)` - Sum where condition is true
- `AVERAGEIF(column, condition)` - Average where true
- `COUNTIF(column, condition)` - Count where true

Example:

```yaml
high_revenue_count: "=COUNTIF(revenue, > 150000)"
high_revenue_total: "=SUMIF(revenue, > 150000)"
```text

### Logical (Row-wise)

- `IF(condition_col, true_col, false_col)` - Element-wise conditional

---

## Backwards Compatibility

### Auto-Detection Algorithm

```rust
fn detect_model_version(yaml: &Value) -> ModelVersion {
    // Check for explicit version marker
    if yaml.get("_forge_version").is_some() {
        return parse_version(yaml);
    }

    // Check for array pattern (v1.0.0)
    if has_array_values(yaml) {
        return ModelVersion::V1_0;
    }

    // Check for {value, formula} pattern (v0.2.0)
    if has_value_formula_objects(yaml) {
        return ModelVersion::V0_2;
    }

    // Default to v0.2.0 for backwards compat
    ModelVersion::V0_2
}
```text

### Migration Path

**Manual upgrade:**

```bash
forge upgrade model-v0.2.yaml --output model-v1.0.yaml
```text

**Automatic upgrade (in-place):**

```bash
forge upgrade model.yaml --in-place --backup
```text

**Preview changes:**

```bash
forge upgrade model.yaml --dry-run
```text

**Output:**

```diff

- q1: 100
- q2: 120
- q3: 150
- q4: 180
- annual_total: "=q1 + q2 + q3 + q4"

+ revenue: [100, 120, 150, 180]
+ annual_total: "=SUM(revenue)"

```text

### Conversion Rules

1. **Sequential scalars → Array:**
   - `q1, q2, q3, q4` → `quarter: [q1, q2, q3, q4]`
   - Detect pattern: same prefix + number suffix

2. **Formula rewriting:**
   - `=q1 + q2 + q3 + q4` → `=SUM(quarter)`
   - `=MAX(q1, q2, q3, q4)` → `=MAX(quarter)`

3. **Preserve non-sequential:**
   - `platform_take_rate`, `gross_margin` → Keep as scalars

---

## Testing Strategy (100% Accuracy)

### Level 1: Unit Tests

**Parser Tests:**

- ✅ Parse homogeneous arrays
- ✅ Reject mixed-type arrays
- ✅ Detect model version correctly
- ✅ Handle empty arrays
- ✅ Handle single-element arrays

**Calculator Tests:**

- ✅ Row-wise formulas (element-wise ops)
- ✅ Column aggregation (SUM, AVG, MAX, MIN)
- ✅ Conditional aggregation (SUMIF, COUNTIF)
- ✅ Type checking (number vs text)
- ✅ Length validation (mismatched columns)

**Writer Tests:**

- ✅ Write arrays back to YAML
- ✅ Preserve formatting
- ✅ Update calculated values

### Level 2: Integration Tests

**Cross-file references:**

- ✅ Reference arrays from other files
- ✅ Mixed array + scalar references
- ✅ Circular dependency detection

**Formula evaluation:**

- ✅ Complex nested formulas
- ✅ Multiple array operations
- ✅ Edge cases (division by zero, null values)

### Level 3: E2E Tests (Real Financial Models)

**Test data:** Real-world financial model scenarios

1. **SaaS Unit Economics:**

   ```yaml
   monthly_cohorts:
     month: [Jan, Feb, Mar, Apr, May, Jun]
     new_mrr: [5000, 6000, 7000, 8000, 9000, 10000]
     churn_mrr: [500, 600, 700, 800, 900, 1000]
     net_mrr: "=new_mrr - churn_mrr"
     cumulative: "=CUMSUM(net_mrr)"  # New function

   summary:
     total_new_mrr: "=SUM(monthly_cohorts.new_mrr)"
     avg_churn_rate: "=AVERAGE(monthly_cohorts.churn_mrr / monthly_cohorts.new_mrr)"
```text

2. **Multi-Year Budget:**

   ```yaml
   budget:
     year: [2024, 2025, 2026]
     revenue: [1000000, 1500000, 2250000]
     cogs: "=revenue * 0.3"
     gross_profit: "=revenue - cogs"

   metrics:
     cagr: "=(budget.revenue[2] / budget.revenue[0]) ^ (1/2) - 1"
     avg_margin: "=AVERAGE(budget.gross_profit / budget.revenue)"
```text

3. **Quarterly P&L:**

   ```yaml
   pl_2025:
     quarter: [Q1, Q2, Q3, Q4]
     revenue: [100000, 120000, 150000, 180000]
     expenses: [80000, 90000, 100000, 110000]
     profit: "=revenue - expenses"
     margin: "=profit / revenue"

   annual:
     total_revenue: "=SUM(pl_2025.revenue)"
     total_profit: "=SUM(pl_2025.profit)"
     avg_margin: "=total_profit / total_revenue"
```text

### Level 4: Round-Trip Tests

**YAML → Excel → YAML:**

1. Start with YAML model
2. Export to Excel
3. Manually edit values in Excel
4. Import back to YAML
5. Verify formulas preserved
6. Verify values match

**Precision Tests:**

- ✅ Financial calculations (0.01 precision)
- ✅ Percentage calculations (0.0001 precision)
- ✅ Large numbers (millions, billions)
- ✅ Small numbers (basis points)

**Edge Cases:**

- ✅ Empty arrays
- ✅ Single-element arrays
- ✅ Null/missing values
- ✅ Division by zero
- ✅ Negative percentages
- ✅ Very large/small numbers

### Level 5: Property-Based Tests

**Generate random financial models:**

- Random table sizes (1-1000 rows)
- Random formula combinations
- Random number ranges
- Verify: all formulas evaluate without error
- Verify: round-trip consistency

---

## Error Handling (Zero Tolerance)

### Parse-Time Errors (Fail Fast)

```yaml

# Mixed-type array

revenue: [100, "Q2", 150]  # ❌ ERROR: Array must be homogeneous
```text

Error:

```text
Error: Parse error in file 'model.yaml' at line 2
  revenue: [100, "Q2", 150]
           ^^^^^
  Type mismatch: Array contains both Number (100) and String ("Q2")

  Fix: Ensure all array elements are the same type
```text

### Formula Errors (Detailed Context)

```yaml
profit: "=revenue - expenses"

# But: revenue has 4 elements, expenses has 3

```text

Error:

```text
Error: Formula evaluation error in 'profit'
  profit: "=revenue - expenses"
          ^^^^^^^^^^^^^^^^^^^^
  Length mismatch: 'revenue' has 4 elements, 'expenses' has 3

  In file: model.yaml:5

  Fix: Ensure all columns in row-wise formula have same length
```text

### Validation Errors (Helpful Suggestions)

```yaml
total: "=SUM(revenu)"  # Typo
```text

Error:

```text
Error: Variable not found: 'revenu'
  total: "=SUM(revenu)"
              ^^^^^^^
  Did you mean: 'revenue'?

  Available columns: revenue, expenses, profit
```text

---

## JSON Schema Validation

### Why JSON Schema?

For **financial models**, structural validation is non-negotiable:

**Zero Errors Requirement:**

- Structure errors caught **before** formula evaluation
- Type safety enforced at parse time
- Homogeneous arrays validated
- Required fields checked

**Developer Experience:**

- **IDE autocomplete** - VSCode/IntelliJ suggest valid keys
- **Real-time validation** - Errors highlighted as you type
- **Self-documenting** - Schema IS the specification
- **Team guardrails** - Prevents invalid models

### Schema Location

`schema/forge-v1.0.schema.json` - Complete JSON Schema for v1.0.0 model

### IDE Integration

**VSCode (Recommended):**

```json
// .vscode/settings.json
{
  "yaml.schemas": {
    "./schema/forge-v1.0.schema.json": ["test-data/v1.0/*.yaml"]
  }
}
```text

**In YAML files:**

```yaml

# yaml-language-server: $schema=../../schema/forge-v1.0.schema.json

_forge_version: "1.0.0"

# IDE now provides autocomplete and validation!

```text

### Validation Rules

✅ **Enforced:**

- `_forge_version: "1.0.0"` required
- Arrays must be homogeneous (all numbers, all text, etc.)
- Formulas must start with `=`
- Scalars must have both `value` and `formula` keys
- Aggregations require function name (SUM, MAX, etc.)

❌ **Rejected:**

```yaml
revenue: [100, "Q2", 150]  # Mixed types
total: "SUM(revenue)"       # Missing '='
tax_rate: { value: 0.25 }  # Missing 'formula' key
```text

### Command-Line Validation

```bash

# Automatic validation (default in v1.0.0)

forge validate model.yaml

# Explicit schema validation

forge validate-schema model.yaml

# Skip validation (not recommended for financial models!)

forge calculate --skip-schema-validation model.yaml
```text

---

## Implementation Phases

### Phase 0: JSON Schema (Week 0 - DONE ✅)

- ✅ Complete JSON Schema definition
- ✅ IDE integration documentation
- ✅ Schema references in examples
- ✅ Validation rules documented

### Phase 1: Parser (Week 1)

- ✅ Detect arrays vs scalars
- ✅ Type checking (homogeneous arrays)
- ✅ Model version detection
- ✅ Error messages with context
- ✅ Unit tests (50+ tests)

### Phase 2: Calculator (Week 2)

- ✅ Row-wise formula evaluation
- ✅ Column aggregation
- ✅ New functions (MAX, MIN, COUNT)
- ✅ Conditional aggregation (SUMIF, etc.)
- ✅ Integration tests (20+ tests)

### Phase 3: Excel Export (Week 3)

- ✅ Table → Sheet mapping
- ✅ Formula translation
- ✅ Formatting (headers, summaries)
- ✅ rust_xlsxwriter integration
- ✅ E2E tests with real models

### Phase 4: Migration Tool (Week 4)

- ✅ Auto-detect conversion patterns
- ✅ Formula rewriting
- ✅ Backup/preview mode
- ✅ Batch conversion
- ✅ Migration tests

### Phase 5: Testing & Polish (Week 5)

- ✅ Round-trip tests
- ✅ Precision tests
- ✅ Property-based tests
- ✅ Documentation
- ✅ Examples

---

## Success Criteria

### Functional Requirements

- ✅ Parse array and scalar models
- ✅ Evaluate row-wise and aggregation formulas
- ✅ Export to Excel (.xlsx)
- ✅ Import from Excel (.xlsx)
- ✅ Upgrade v0.2.0 → v1.0.0

### Quality Requirements

- ✅ 100+ unit tests passing
- ✅ 50+ E2E tests with real models
- ✅ Round-trip tests (YAML → Excel → YAML)
- ✅ Zero precision errors (<0.000001)
- ✅ Helpful error messages (file:line context)

### Performance Requirements

- ✅ 10,000 row table in <1 second
- ✅ Complex model (100 formulas) in <500ms
- ✅ Excel export (10 sheets) in <2 seconds

### User Experience

- ✅ Clear documentation with examples
- ✅ Migration guide (v0.2.0 → v1.0.0)
- ✅ Video tutorial
- ✅ Real-world example models

---

## Open Questions

1. **Array indexing syntax:**
   - `table.column[0]` (Python-style, 0-indexed)
   - `table.column[1]` (Excel-style, 1-indexed)
   - Decision: **0-indexed** (Rust native, less confusion)

2. **Null handling in arrays:**
   - `[100, null, 150]` - Allow nulls?
   - Skip in SUM? Error?
   - Decision: **Allow nulls, skip in aggregations** (Excel behavior)

3. **Dynamic array formulas:**
   - `new_column: "=existing_column * 2"` (creates new array)
   - vs explicit length declaration
   - Decision: **Infer length from referenced columns**

4. **Mixed references:**
   - `=table1.revenue + scalar_value` (broadcast scalar?)
   - Decision: **Allow broadcasting** (scalar applied to each element)

5. **Excel formula preservation:**
   - On import, keep Excel formulas or convert to YAML?
   - Decision: **Convert to YAML** (single source of truth)

---

## Risk Mitigation

### Risk 1: xlformula_engine limitations

**Mitigation:**

- Fork xlformula_engine if needed
- Or build custom formula evaluator
- Fallback: Keep using xlformula_engine for scalars, custom for arrays

### Risk 2: Excel compatibility

**Mitigation:**

- Test with Excel 365, Excel 2019, LibreOffice
- Document known incompatibilities
- Provide export options (strict/compatible modes)

### Risk 3: Migration complexity

**Mitigation:**

- Extensive testing with real user models
- Dry-run mode shows changes before applying
- Backup files automatically
- Support both models indefinitely (no forced migration)

### Risk 4: Performance with large datasets

**Mitigation:**

- Benchmark with 100K+ row tables
- Lazy evaluation where possible
- Parallel formula evaluation
- Streaming Excel export for large files

---

## Next Steps

1. ✅ Review this design (stakeholder approval)
2. ⏭️ Create array parser (Phase 1)
3. ⏭️ Add comprehensive tests
4. ⏭️ Build Excel exporter
5. ⏭️ Migration tool
6. ⏭️ Documentation + examples

**Target:** v1.0.0 release in 5 weeks

---

**This design prioritizes:**

- ✅ Financial accuracy (zero errors)
- ✅ Excel compatibility (trivial export)
- ✅ Testing rigor (100% coverage)
- ✅ User experience (clear errors, good docs)

Let's build it! 🚀
