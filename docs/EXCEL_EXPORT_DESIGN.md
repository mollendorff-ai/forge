# Excel Export Design (Phase 3)

**Status:** Design Phase
**Target:** v1.0.0 Array Model → Excel .xlsx export
**Library:** rust_xlsxwriter 0.90.2

---

## Overview

Export v1.0.0 array models to Excel spreadsheets with **100% formula fidelity**. This is THE killer feature that makes Forge a complete Excel-YAML bridge.

## Design Goals

1. **1:1 Column Mapping** - YAML column arrays → Excel columns (A, B, C, ...)
2. **Formula Preservation** - Row formulas → Excel cell formulas
3. **Cross-Table References** - `table.column` → `Sheet!Column` references
4. **Scalar Support** - Export scalars to dedicated sheet
5. **Round-Trip Compatibility** - Excel → YAML → Excel should preserve structure

## Architecture

### File Structure

```
quarterly_pl.yaml (input)
  ├── tables:
  │   ├── pl_2025 (3 columns: revenue, cogs, gross_profit)
  │   ├── opex_2025 (4 columns: ...)
  │   └── final_pl (3 columns: ...)
  └── scalars:
      ├── annual_2025.total_revenue
      ├── annual_2025.total_cogs
      └── ...

quarterly_pl.xlsx (output)
  ├── Sheet: pl_2025
  │   ├── Column A: revenue [1000, 1200, 1500, 1800]
  │   ├── Column B: cogs [300, 360, 450, 540]
  │   └── Column C: gross_profit =A2-B2, =A3-B3, ... (formulas!)
  ├── Sheet: opex_2025
  │   └── ...
  ├── Sheet: final_pl
  │   └── ...
  └── Sheet: Scalars
      ├── annual_2025.total_revenue: =SUM(pl_2025!A:A)
      └── ...
```

### Mapping Strategy

#### 1. Tables → Worksheets

Each table becomes a separate worksheet:
- **Sheet name:** `table_name` (e.g., "pl_2025")
- **Header row:** Column names in row 1 (A1, B1, C1, ...)
- **Data rows:** Start at row 2

#### 2. Columns → Excel Columns

Direct 1:1 mapping:
- First column → Column A
- Second column → Column B
- Formula column → Excel formula (e.g., `=A2-B2` for revenue - cogs)

#### 3. Row Formulas → Cell Formulas

Convert YAML row formulas to Excel cell formulas:

**YAML:**
```yaml
tables:
  pl_2025:
    columns:
      revenue: [1000, 1200, 1500, 1800]
      cogs: [300, 360, 450, 540]
    row_formulas:
      gross_profit: =revenue - cogs
```

**Excel (pl_2025 sheet):**
```
   A        B      C
1  revenue  cogs   gross_profit
2  1000     300    =A2-B2       <- Formula!
3  1200     360    =A3-B3       <- Formula!
4  1500     450    =A4-B4       <- Formula!
5  1800     540    =A5-B5       <- Formula!
```

#### 4. Cross-Table References

Convert `table.column` syntax to Excel sheet references:

**YAML:**
```yaml
final_pl:
  row_formulas:
    revenue: =pl_2025.revenue
```

**Excel (final_pl sheet):**
```
   A
1  revenue
2  =pl_2025!A2    <- Sheet reference!
3  =pl_2025!A3
4  =pl_2025!A4
5  =pl_2025!A5
```

#### 5. Scalars → Dedicated Sheet

Create "Scalars" worksheet:

**Layout:**
```
   A                               B
1  Name                            Formula
2  annual_2025.total_revenue       =SUM(pl_2025!A:A)
3  annual_2025.total_cogs          =SUM(pl_2025!B:B)
4  annual_2025.gross_profit        =B2-B3
```

## Implementation Plan

### Phase 3.1: Basic Table Export
- Create `src/excel/mod.rs` module
- Implement `ExcelExporter` struct
- Export table columns (data only, no formulas yet)
- Generate one sheet per table with headers
- **Test:** Simple table with 2 data columns

### Phase 3.2: Formula Translation
- Implement `FormulaTranslator` to convert YAML → Excel formulas
- Handle variable name → column letter mapping (revenue → A)
- Generate cell formulas for each row (=A2-B2, =A3-B3, ...)
- **Test:** Table with row formula (gross_profit = revenue - cogs)

### Phase 3.3: Cross-Table References
- Detect `table.column` pattern in formulas
- Convert to Excel sheet references (=Sheet!Column)
- Handle row-wise expansion (=pl_2025!A2 for row 2)
- **Test:** final_pl referencing pl_2025.revenue

### Phase 3.4: Scalar Export
- Create "Scalars" worksheet
- Export scalar names and formulas
- Convert aggregations: SUM(table.column) → =SUM(Sheet!Column:Column)
- Convert array indexing: table.column[3] → =Sheet!Column4
- **Test:** annual_2025.total_revenue = SUM(pl_2025.revenue)

### Phase 3.5: CLI Integration
- Add `export` subcommand to CLI
- Usage: `forge export input.yaml output.xlsx`
- Support `--verbose` flag for progress
- **Test:** E2E test with quarterly_pl.yaml

## Formula Translation Examples

### Simple Arithmetic
```yaml
YAML:  =revenue - cogs
Excel: =A2-B2  (for row 2)
       =A3-B3  (for row 3)
       ...
```

### Cross-Table Reference
```yaml
YAML:  =pl_2025.revenue
Excel: =pl_2025!A2  (for row 2)
       =pl_2025!A3  (for row 3)
       ...
```

### Mixed Operations
```yaml
YAML:  =gross_profit / revenue
Excel: =C2/A2  (for row 2, assuming gross_profit is column C)
```

### Aggregation (Scalars)
```yaml
YAML:  =SUM(pl_2025.revenue)
Excel: =SUM(pl_2025!A:A)
```

### Array Indexing (Scalars)
```yaml
YAML:  =revenue[3] / revenue[0] - 1
Excel: =A4/A1-1  (0-indexed in YAML → 1-indexed in Excel, +1 for header)
```

## Technical Challenges

### Challenge 1: Formula Parser
- Need to parse YAML formulas to identify variable references
- Convert variable names to column letters (revenue → A, cogs → B)
- Preserve formula operators and functions (+, -, *, /, SUM, etc.)

**Solution:**
- Use regex to extract variable names
- Build column name → column letter mapping from table structure
- Replace variable names with Excel column references

### Challenge 2: Row-wise Formula Expansion
- Single YAML formula → Multiple Excel formulas (one per row)
- Must adjust row numbers (=A2-B2, =A3-B3, =A4-B4, ...)

**Solution:**
- For each row index i, generate formula with row number (i + 2) (1-indexed + header)
- Use rust_xlsxwriter's Formula struct for each cell

### Challenge 3: Cross-Table Reference Translation
- Detect `table.column` pattern
- Find column letter in target table
- Generate sheet reference (=SheetName!ColumnRow)

**Solution:**
- Pattern matching: if formula contains `.`, split on `.`
- Look up table name and column name
- Build Excel reference: `={table_name}!{col_letter}{row_num}`

### Challenge 4: Aggregation Translation
- SUM(table.column) → =SUM(Sheet!A:A)
- AVERAGE, MAX, MIN similarly

**Solution:**
- Detect aggregation functions (SUM, AVERAGE, MAX, MIN)
- Extract table.column argument
- Convert to Excel range reference (A:A for entire column)

## Error Handling

- Invalid table references → Error with helpful message
- Circular formula dependencies → Warn (Excel will handle)
- Unsupported formula syntax → Error with formula string
- Column name conflicts → Error (duplicate column names)

## Testing Strategy

### Unit Tests
1. `test_column_letter_mapping` - revenue → A, cogs → B
2. `test_simple_formula_translation` - =revenue-cogs → =A2-B2
3. `test_cross_table_reference` - =table.col → =Sheet!A2
4. `test_aggregation_translation` - =SUM(table.col) → =SUM(Sheet!A:A)
5. `test_array_indexing_translation` - col[3] → A4

### Integration Tests
1. `test_export_simple_table` - 2 data columns, 1 calculated
2. `test_export_multiple_tables` - 3 tables, cross-references
3. `test_export_with_scalars` - Tables + scalars sheet

### E2E Test
1. `test_export_quarterly_pl` - Full quarterly_pl.yaml → .xlsx
   - Verify Excel file opens in LibreOffice/Excel
   - Verify formulas calculate correctly
   - Verify cross-table references work
   - Verify scalars sheet present

## API Design

### Public Interface

```rust
// src/excel/mod.rs

pub struct ExcelExporter {
    model: ParsedModel,
}

impl ExcelExporter {
    pub fn new(model: ParsedModel) -> Self { ... }

    pub fn export(&self, output_path: &Path) -> ForgeResult<()> { ... }
}

// Usage:
let model = parse_model("input.yaml")?;
let exporter = ExcelExporter::new(model);
exporter.export("output.xlsx")?;
```

### Internal Modules

```rust
// src/excel/formula_translator.rs
pub struct FormulaTranslator {
    column_map: HashMap<String, String>, // revenue → A
}

impl FormulaTranslator {
    pub fn translate_row_formula(
        &self,
        formula: &str,
        row_num: usize,
    ) -> ForgeResult<String> { ... }

    pub fn translate_scalar_formula(
        &self,
        formula: &str,
    ) -> ForgeResult<String> { ... }
}
```

## Success Criteria

✅ **Phase 3 Complete When:**
1. quarterly_pl.yaml exports to .xlsx ✅
2. Excel file opens in Excel/LibreOffice ✅
3. All formulas calculate correctly ✅
4. Cross-table references work ✅
5. Scalars exported to dedicated sheet ✅
6. CLI command `forge export` working ✅
7. 100% test coverage maintained ✅
8. Documentation updated ✅

## Future Enhancements (Post-Phase 3)

- **Formatting:** Bold headers, number formats, column widths
- **Charts:** Auto-generate charts from data
- **Conditional Formatting:** Highlight key metrics
- **Named Ranges:** Convert scalars to Excel named ranges
- **Reverse Import:** Excel → YAML (round-trip support)
- **Template Support:** Apply Excel templates to exports

---

**Next Step:** Implement Phase 3.1 - Basic Table Export
