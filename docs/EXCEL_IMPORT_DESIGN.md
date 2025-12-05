# Excel Import Design (Phase 4)

**Status:** Design Phase
**Target:** Excel .xlsx → YAML v1.0.0 array model
**Library:** calamine 0.31.0 (MIT licensed, 4M+ downloads)

---

## Overview

Import existing Excel files to YAML format, enabling:

1. **AI-assisted financial modeling** with existing Excel files
2. **Version control** for Excel files (convert to YAML!)
3. **Round-trip workflow** (Excel → YAML → AI + Forge → Excel)
4. **Black-box testing** (import → export = identical)

## Use Cases for Financial Modeling Professionals 💼

### Use Case 1: Convert Existing Models to Version Control

**Problem:** You have 50+ Excel files for a financial project, no version control
**Solution:**

```bash

# Convert all Excel files to YAML

for file in *.xlsx; do
    forge import "$file" "${file%.xlsx}.yaml"
done

# Now track in git!

git add *.yaml
git commit -m "Initial import of financial models"
```

**Benefits:**

- See diffs in YAML (what changed?)
- Merge changes from multiple analysts
- Rollback to previous versions
- Never lose work again!

### Use Case 2: AI-Assisted Model Enhancement

**Problem:** Stakeholder sends you complex_model.xlsx, wants scenario analysis
**Solution:**

```bash

# Import their Excel

forge import complex_model.xlsx model.yaml

# Work with Claude to add scenarios


# (Claude reads YAML, suggests formulas)

# Validate with Forge (zero errors!)

forge calculate model.yaml

# Export back to Excel with new scenarios

forge export model.yaml complex_model_v2.xlsx
```

**Benefits:**

- AI understands YAML better than Excel
- Forge validates every formula (no hallucinations!)
- Stakeholder gets Excel file back (familiar format)

### Use Case 3: Collaborative Workflow

**Problem:** CFO edits financials.xlsx, you need to integrate changes
**Solution:**

```bash

# Import CFO's updated Excel

forge import financials_v2.xlsx financials_new.yaml

# Diff against your version

git diff financials.yaml financials_new.yaml

# Merge changes

git merge ...

# Export final version

forge export financials.yaml financials_final.xlsx
```

**Benefits:**

- Non-technical stakeholders work in Excel
- Developers work in YAML (version controlled)
- Best of both worlds!

### Use Case 4: Audit Trail & Documentation

**Problem:** Auditors ask "where did this number come from?"
**Solution:**

```bash

# Import auditor's Excel

forge import audit_request.xlsx audit.yaml

# YAML shows ALL formulas clearly:


# revenue: [1000, 1200, 1500]


# cogs: [300, 360, 450]


# gross_profit: "=revenue - cogs"  <- VISIBLE!

# Validate calculations

forge validate audit.yaml  # Proves formulas are correct!
```

**Benefits:**

- All formulas visible in YAML (not hidden in cells)
- Forge validation = mathematical proof of correctness
- Git history = complete audit trail

### Use Case 5: Template Conversion

**Problem:** Company has 100 Excel templates for financial models
**Solution:**

```bash

# Import template once

forge import template.xlsx template.yaml

# Use template.yaml as base for all new models


# (Version controlled, AI-friendly, validated)

# Export customized versions to Excel

forge export custom_model.yaml → client_deliverable.xlsx
```

**Benefits:**

- One source of truth (YAML)
- Easy to update templates
- Consistent structure across models

---

## Design Goals

1. **Lossless Round-Trip** - Excel → YAML → Excel should be identical
2. **Formula Preservation** - Translate Excel formulas to YAML syntax
3. **Structure Recognition** - Detect tables vs scalars automatically
4. **Error Recovery** - Handle non-standard Excel files gracefully
5. **Fast** - Import large Excel files (<1 second)

## Architecture

### File Structure

```text
Input: quarterly_pl.xlsx
  ├── Sheet: pl_2025
  │   ├── Row 1 (header): quarter, revenue, cogs, gross_profit
  │   ├── Row 2: Q1, 1000, 300, =B2-C2
  │   ├── Row 3: Q2, 1200, 360, =B3-C3
  │   └── ...
  ├── Sheet: opex_2025
  │   └── ...
  └── Sheet: Scalars (optional)
      ├── Name, Value, Formula
      └── total_revenue, 5500, =SUM(pl_2025!B:B)

Output: quarterly_pl.yaml
  pl_2025:
    quarter: ["Q1", "Q2", "Q3", "Q4"]
    revenue: [1000, 1200, 1500, 1800]
    cogs: [300, 360, 450, 540]
    gross_profit: "=revenue - cogs"

  opex_2025:
    ...

  annual_2025:
    total_revenue:
      formula: "=SUM(pl_2025.revenue)"
```

### Mapping Strategy

#### 1. Worksheets → Tables/Scalars

**Strategy:**

- Sheet name = "Scalars" → Parse as scalar section
- Other sheets → Parse as tables

**Detection:**

- If row 1 has column headers → Table
- If column A has "Name", column C has "Formula" → Scalars sheet

#### 2. Excel Columns → YAML Arrays

**Algorithm:**

1. Read row 1 as column names
2. Read rows 2+ as data values
3. Detect formulas vs values
4. Group into `columns:` (data) and `row_formulas:` (calculated)

**Example:**

```text
Excel:
   A      B       C        D
1  quarter revenue cogs     gross_profit
2  Q1     1000    300      =B2-C2
3  Q2     1200    360      =B3-C3

YAML:
quarter: ["Q1", "Q2"]
revenue: [1000, 1200]
cogs: [300, 360]
gross_profit: "=revenue - cogs"  # Translated!
```

#### 3. Excel Formulas → YAML Syntax

**Translation Rules:**

| Excel Formula | YAML Formula |
|--------------|-------------|
| `=B2-C2` | `=revenue - cogs` |
| `=B2+C2+D2` | `=revenue + cogs + opex` |
| `=B2/C2` | `=revenue / cogs` |
| `=pl_2025!B2` | `=pl_2025.revenue` |
| `=SUM(pl_2025!B:B)` | `=SUM(pl_2025.revenue)` |
| `=pl_2025!B5` | `=pl_2025.revenue[3]` (if row 5 = index 3) |

**Reverse Formula Translator:**

1. Parse Excel formula (=B2-C2)
2. Extract cell references (B2, C2)
3. Map cell references to column names (B → revenue, C → cogs)
4. Replace with column names (=revenue - cogs)
5. Handle sheet references (Sheet! → table.)

#### 4. Excel Ranges → Array Syntax

**Examples:**

- `=SUM(A:A)` → `=SUM(column_name)`
- `=SUM(A2:A5)` → `=SUM(column_name)` (assume full column)
- `=AVERAGE(B:B)` → `=AVERAGE(column_name)`

#### 5. Cross-Sheet References

**Translation:**

- `=pl_2025!B2` → `=pl_2025.revenue` (cell ref)
- `=SUM(pl_2025!B:B)` → `=SUM(pl_2025.revenue)` (range ref)
- `='Sheet Name'!A1` → `=sheet_name.column` (sanitize name)

## Implementation Plan

### Phase 4.1: Excel Reader

- Add calamine dependency
- Implement `ExcelReader` struct
- Read worksheet names
- Read cell values (strings, numbers, formulas)
- Handle data types (Number, Text, Date, Boolean)

### Phase 4.2: Structure Detection

- Detect header rows (row 1 has strings)
- Group columns by name
- Detect formula columns vs data columns
- Identify "Scalars" sheet pattern

### Phase 4.3: Reverse Formula Translation

- Implement `ReverseFormulaTranslator`
- Parse Excel formulas (cell refs, functions)
- Map cell references → column names
- Translate sheet refs → table.column
- Handle array formulas and ranges

### Phase 4.4: YAML Generation

- Build `ParsedModel` from Excel data
- Generate YAML structure
- Write to file

### Phase 4.5: CLI Integration

- Add `import` subcommand
- Usage: `forge import input.xlsx output.yaml`
- Support `--verbose` flag

### Phase 4.6: Round-Trip Testing

- Test: Excel → YAML → Excel = identical
- Verify formulas translate correctly
- Verify data preserves types
- Handle edge cases

## Formula Translation Examples

### Simple Cell References

```text
Excel:  =B2-C2
YAML:   =revenue - cogs

Excel:  =B2/C2*100
YAML:   =revenue / cogs * 100
```

### Cross-Sheet References

```text
Excel:  =pl_2025!B2
YAML:   =pl_2025.revenue

Excel:  ='Sheet Name'!A1
YAML:   =sheet_name.column_name
```

### Array Formulas

```text
Excel:  =SUM(B:B)
YAML:   =SUM(revenue)

Excel:  =SUM(pl_2025!B:B)
YAML:   =SUM(pl_2025.revenue)

Excel:  =AVERAGE(C2:C5)
YAML:   =AVERAGE(cogs)
```

### Cell Index to Array Index

```text
Excel:  =B5  (if B2=first data row, B5=4th row)
YAML:   =revenue[3]  (0-indexed)

Excel:  =pl_2025!B5
YAML:   =pl_2025.revenue[3]
```

## Technical Challenges

### Challenge 1: Column Name Mapping

- **Problem:** Excel uses letters (A, B, C), YAML uses names
- **Solution:** Read header row, build mapping (A → revenue, B → cogs)
- **Edge Case:** What if no header row? → Error with helpful message

### Challenge 2: Formula Parsing

- **Problem:** Excel formulas are complex (nested functions, operators)
- **Solution:** Use regex to extract cell references, replace with column names
- **Edge Case:** Functions like SUM(B2:B10) → Detect range pattern, convert to SUM(column)

### Challenge 3: Cross-Sheet References

- **Problem:** `=Sheet1!B2` needs to become `=table.column`
- **Solution:** Parse sheet name, extract cell ref, map to column name
- **Edge Case:** Sheet names with spaces → Sanitize to valid YAML keys

### Challenge 4: Data vs Formula Detection

- **Problem:** Cell might contain number or formula
- **Solution:** calamine provides cell type (Data vs Formula)
- **Edge Case:** Formula that evaluates to constant → Treat as formula

### Challenge 5: Round-Trip Guarantee

- **Problem:** Excel → YAML → Excel must be identical
- **Solution:** Extensive testing with real Excel files
- **Edge Case:** Excel formatting, charts → Warn user (not supported)

## API Design

### Public Interface

```rust
// src/excel/importer.rs

pub struct ExcelImporter {
    path: PathBuf,
}

impl ExcelImporter {
    pub fn new(path: PathBuf) -> Self { ... }

    pub fn import(&self) -> ForgeResult<ParsedModel> { ... }
}

// Usage:
let importer = ExcelImporter::new("input.xlsx".into());
let model = importer.import()?;
// model is now a ParsedModel ready to export as YAML
```

### Internal Modules

```rust
// src/excel/reader.rs
pub struct ExcelReader {
    workbook: Xlsx<BufReader<File>>,
}

impl ExcelReader {
    pub fn read_worksheets(&self) -> Vec<Worksheet> { ... }
    pub fn read_cells(&self, sheet: &str) -> Vec<Vec<Cell>> { ... }
}

// src/excel/reverse_formula_translator.rs
pub struct ReverseFormulaTranslator {
    column_map: HashMap<String, String>, // A → revenue
}

impl ReverseFormulaTranslator {
    pub fn translate_formula(
        &self,
        excel_formula: &str,
    ) -> ForgeResult<String> { ... }
}
```

## Success Criteria

✅ **Phase 4 Complete When:**

1. Import simple Excel file → YAML ✅
2. Import file with formulas → YAML with correct formula syntax ✅
3. Import file with cross-sheet refs → table.column syntax ✅
4. Round-trip test passes (Excel → YAML → Excel = identical) ✅
5. CLI command `forge import` working ✅
6. Real Excel file import (quarterly_pl.xlsx) ✅
7. Documentation with use cases ✅

## Round-Trip Testing Strategy

**Black-Box Test:**

```bash

# Start with original Excel file

original.xlsx

# Import to YAML

forge import original.xlsx imported.yaml

# Export back to Excel

forge export imported.yaml roundtrip.xlsx

# Compare (should be identical!)


# - Same number of sheets


# - Same column names


# - Same formulas (semantically equivalent)


# - Same data values

```

**Validation:**

- Use Excel formula evaluator to verify formulas work
- Check data types preserved (Number vs Text vs Date)
- Verify cross-sheet references maintained
- Test with various Excel files (simple, complex, real-world)

## Error Handling

- **No header row:** Error with message "Sheet must have header row in row 1"
- **Empty sheet:** Warning, skip sheet
- **Unsupported formula:** Best-effort translation, warn user
- **Charts/Formatting:** Warn user "Charts and formatting not imported (data only)"
- **Protected sheets:** Error with helpful message

---

## Implementation Estimate

**Phase 4.1-4.2 (Reading):** 30-45 minutes
**Phase 4.3 (Formula Translation):** 45-60 minutes
**Phase 4.4-4.5 (YAML + CLI):** 15-30 minutes
**Phase 4.6 (Testing):** 30-45 minutes

**Total: 2-3 hours**

---

**Next Step:** Add calamine dependency and implement Phase 4.1!
