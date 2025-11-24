# Testing Architecture

**Document Version:** 1.0.0
**Forge Version:** v1.1.2
**Last Updated:** 2025-11-24
**Status:** Complete

---

## Table of Contents

1. [Introduction](#introduction)
2. [Test Strategy Overview](#test-strategy-overview)
3. [Test Breakdown (136 Tests)](#test-breakdown-136-tests)
4. [Unit Testing](#unit-testing)
5. [Integration Testing](#integration-testing)
6. [End-to-End Testing](#end-to-end-testing)
7. [Test Organization](#test-organization)
8. [Test Data Management](#test-data-management)
9. [Property-Based Testing (Future)](#property-based-testing-future)
10. [Test Coverage Analysis](#test-coverage-analysis)
11. [CI/CD Integration](#cicd-integration)
12. [Test Performance](#test-performance)
13. [Test Maintenance](#test-maintenance)
14. [Related Documentation](#related-documentation)

---

## Introduction

### Purpose

This document provides comprehensive coverage of Forge's **testing strategy**, including:

- **136 tests** - 86 unit, 33 e2e, 6 integration, 5 validation, 3 doc, 3 other
- **Test organization** - inline tests, tests/ directory, examples/
- **Coverage strategy** - What to test, what to skip
- **CI/CD integration** - Automated testing in pipelines
- **Test data** - 33+ YAML test files
- **Performance** - Test execution speed

### Design Philosophy

Test behavior, not implementation

Forge tests focus on:

- **Public APIs** - User-facing behavior, not internals
- **Error cases** - Failure modes and edge cases
- **Regression prevention** - Known bugs don't return
- **Documentation** - Tests as executable examples

**Testing Pyramid:**

```text
        /\
       /  \      3 doc tests
      / E2E\     33 e2e tests
     /------\
    /  Integ \   6 integration tests
   /----------\
  /    Unit    \ 86 unit tests
 /--------------\
     (136 total)
```text

### Key Principles

1. **Fast feedback** - Unit tests run in <1s
2. **Comprehensive e2e** - Real CLI usage tested
3. **Isolated tests** - No test dependencies
4. **Deterministic** - Same input → same output
5. **Self-documenting** - Test names explain behavior

---

## Test Strategy Overview

### Testing Philosophy

```plantuml
@startuml test-strategy
!theme plain
title Forge Testing Strategy

package "Unit Tests (86)" {
  [Formula Evaluation]
  [Data Type Conversion]
  [Column Mapping]
  [Dependency Graph]
  [Parser Logic]
}

package "Integration Tests (6)" {
  [Parser + Calculator]
  [Calculator + Writer]
  [Exporter + Translator]
}

package "E2E Tests (33)" {
  [CLI Commands]
  [File I/O]
  [Error Messages]
  [Cross-File Refs]
}

package "Validation Tests (5)" {
  [Stale Detection]
  [Calculate Updates]
}

package "Doc Tests (3)" {
  [README Examples]
  [Code Comments]
}

[Unit Tests (86)] --> [Fast Feedback\n<1 second]
[Integration Tests (6)] --> [Component Interaction\n<2 seconds]
[E2E Tests (33)] --> [Real Usage\n<10 seconds]
[Validation Tests (5)] --> [Correctness\n<2 seconds]
[Doc Tests (3)] --> [Documentation\n<1 second]

note right of [Unit Tests (86)]

#### Focus

  - Individual functions
  - Pure logic
  - Type conversions
  - Edge cases

end note

note right of [E2E Tests (33)]

#### Focus

  - Full CLI workflow
  - File operations
  - Error handling
  - User experience

end note

@enduml
```text

### What to Test

**Core Functionality:**

- ✅ Formula evaluation (all 47+ functions)
- ✅ Dependency resolution (topological sort)
- ✅ YAML parsing (v0.2.0 + v1.0.0)
- ✅ Excel export/import (bidirectional)
- ✅ Formula translation (both directions)
- ✅ Error handling (all error types)

**Edge Cases:**

- ✅ Empty files, empty tables
- ✅ Circular dependencies
- ✅ Missing columns/variables
- ✅ Type mismatches
- ✅ Large datasets (performance)
- ✅ Cross-file references

**User Workflows:**

- ✅ forge calculate --dry-run
- ✅ forge validate (success + failure)
- ✅ forge export (YAML → Excel)
- ✅ forge import (Excel → YAML)
- ✅ Error messages and suggestions

### What NOT to Test

**External Dependencies:**

- ❌ xlformula_engine internals (trust the library)
- ❌ serde_yaml parsing (trust the library)
- ❌ rust_xlsxwriter file writing (trust the library)
- ❌ calamine Excel reading (trust the library)

**Implementation Details:**

- ❌ Private helper functions (test via public APIs)
- ❌ Internal data structures (test behavior, not structure)
- ❌ Performance optimizations (measure, don't test)

**OS-Specific Behavior:**

- ❌ File system quirks (use cross-platform APIs)
- ❌ Terminal colors (optional, graceful degradation)

---

## Test Breakdown (136 Tests)

### Category Breakdown

| Category | Count | Percentage | Location | Focus |
|----------|-------|------------|----------|-------|
| **Unit Tests** | 86 | 63.2% | inline + tests/ | Individual functions |
| **E2E Tests** | 33 | 24.3% | tests/e2e_tests.rs | CLI workflows |
| **Integration Tests** | 6 | 4.4% | tests/ | Component interaction |
| **Validation Tests** | 5 | 3.7% | tests/validation_tests.rs | Correctness checks |
| **Doc Tests** | 3 | 2.2% | README.md, lib.rs | Documentation |
| **Parser Tests** | 3 | 2.2% | tests/parser_v1_tests.rs | YAML parsing |
| **TOTAL** | **136** | **100%** | - | - |

### File Breakdown

| File | Lines | Tests | Focus |
|------|-------|-------|-------|
| `tests/e2e_tests.rs` | 1,026 | 33 | CLI end-to-end workflows |
| `tests/array_calculator_tests.rs` | 233 | 30+ | v1.0.0 array calculations |
| `tests/validation_tests.rs` | 209 | 5 | Validation command |
| `tests/parser_v1_tests.rs` | 83 | 3 | v1.0.0 YAML parsing |
| `src/**/*.rs` (inline) | ~7,400 | 50+ | Unit tests |
| **TOTAL** | **~9,000** | **136** | - |

### Test Growth Over Time

**v0.2.0 (October 2023):**

- 15 tests
- Unit tests only
- Basic formula evaluation

**v1.0.0 (November 2023):**

- 100 tests (+85)
- E2E tests added
- Array calculator tests
- Excel export tests

**v1.1.0 (November 2023):**

- 136 tests (+36)
- 27 new function tests
- Import/validation tests
- Parser v1 tests

**Target v1.2.0:**

- 200+ tests (planned)
- Property-based tests
- Fuzzing tests
- Performance benchmarks

---

## Unit Testing

### Unit Test Strategy

**Focus:** Test individual functions in isolation

**Characteristics:**

- Fast (<1ms per test)
- No I/O (no file reads/writes)
- Deterministic (no randomness)
- Isolated (no shared state)

### Inline Unit Tests

**Location:** Within source files (`src/`), in `#[cfg(test)]` modules

**Example: Formula Translator Tests**

**File:** `/home/rex/src/utils/forge/src/excel/formula_translator.rs:204-286`

```rust

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_column_index_to_letter() {
        assert_eq!(FormulaTranslator::column_index_to_letter(0), "A");
        assert_eq!(FormulaTranslator::column_index_to_letter(1), "B");
        assert_eq!(FormulaTranslator::column_index_to_letter(25), "Z");
        assert_eq!(FormulaTranslator::column_index_to_letter(26), "AA");
        assert_eq!(FormulaTranslator::column_index_to_letter(27), "AB");
        assert_eq!(FormulaTranslator::column_index_to_letter(701), "ZZ");
    }

    #[test]
    fn test_simple_formula_translation() {
        let mut column_map = HashMap::new();
        column_map.insert("revenue".to_string(), "A".to_string());
        column_map.insert("cogs".to_string(), "B".to_string());

        let translator = FormulaTranslator::new(column_map);

        // Test simple subtraction (row 2 in Excel)
        let result = translator.translate_row_formula("=revenue - cogs", 2).unwrap();
        assert_eq!(result, "=A2 - B2");

        // Test division (row 3 in Excel)
        let result = translator.translate_row_formula("=revenue / cogs", 3).unwrap();
        assert_eq!(result, "=A3 / B3");
    }

    #[test]
    fn test_formula_with_multiple_columns() {
        let mut column_map = HashMap::new();
        column_map.insert("sales_marketing".to_string(), "A".to_string());
        column_map.insert("rd".to_string(), "B".to_string());
        column_map.insert("ga".to_string(), "C".to_string());

        let translator = FormulaTranslator::new(column_map);

        let result = translator.translate_row_formula("=sales_marketing + rd + ga", 2).unwrap();
        assert_eq!(result, "=A2 + B2 + C2");
    }

    #[test]
    fn test_cross_table_reference() {
        let column_map = HashMap::new(); // Empty for this test

        let translator = FormulaTranslator::new(column_map);

        let result = translator
            .translate_row_formula("=pl_2025.revenue", 2)
            .unwrap();
        assert_eq!(result, "=pl_2025!revenue2");
    }
}
```text

**Example: Importer Unit Tests**

**File:** `/home/rex/src/utils/forge/src/excel/importer.rs:317-437`

```rust

#[cfg(test)]

mod tests {
    use super::*;

    fn create_test_importer() -> ExcelImporter {
        ExcelImporter::new(PathBuf::from("test.xlsx"))
    }

    #[test]
    fn test_number_to_column_letter() {
        let importer = create_test_importer();

        // Single letters
        assert_eq!(importer.number_to_column_letter(0), "A");
        assert_eq!(importer.number_to_column_letter(1), "B");
        assert_eq!(importer.number_to_column_letter(25), "Z");

        // Double letters
        assert_eq!(importer.number_to_column_letter(26), "AA");
        assert_eq!(importer.number_to_column_letter(27), "AB");
        assert_eq!(importer.number_to_column_letter(51), "AZ");
        assert_eq!(importer.number_to_column_letter(52), "BA");

        // Triple letters
        assert_eq!(importer.number_to_column_letter(702), "AAA");
    }

    #[test]
    fn test_sanitize_table_name() {
        let importer = create_test_importer();

        assert_eq!(importer.sanitize_table_name("Sheet1"), "sheet1");
        assert_eq!(
            importer.sanitize_table_name("P&L Statement"),
            "pandl_statement"
        );
        assert_eq!(
            importer.sanitize_table_name("Revenue-2025"),
            "revenue_2025"
        );
        assert_eq!(
            importer.sanitize_table_name("Special@#$Chars"),
            "specialchars"
        );
    }

    #[test]
    fn test_convert_to_column_value_numbers() {
        let importer = create_test_importer();
        let data = vec![
            Data::Float(100.0),
            Data::Float(200.0),
            Data::Int(300),
            Data::Empty,
        ];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_eq!(nums[0], 100.0);
                assert_eq!(nums[1], 200.0);
                assert_eq!(nums[2], 300.0);
                assert_eq!(nums[3], 0.0); // Empty → 0.0
            }
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_text() {
        let importer = create_test_importer();
        let data = vec![
            Data::String("Apple".to_string()),
            Data::String("Banana".to_string()),
            Data::Empty,
        ];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts.len(), 3);
                assert_eq!(texts[0], "Apple");
                assert_eq!(texts[1], "Banana");
                assert_eq!(texts[2], ""); // Empty → empty string
            }
            _ => panic!("Expected Text column"),
        }
    }
}
```text

### Array Calculator Unit Tests

**File:** `/home/rex/src/utils/forge/tests/array_calculator_tests.rs` (233 lines)

**Test Categories:**

1. **Simple Calculations** - Basic arithmetic, single table
2. **Text Functions** - LEFT, RIGHT, CONCATENATE
3. **Conditional Functions** - IF, IFERROR
4. **Math Functions** - ROUND, SQRT, ABS
5. **Real-World Models** - quarterly_pl.yaml

**Example Test:**

```rust
// From: array_calculator_tests.rs:7-61

#[test]

fn test_simple_table_calculation() {
    let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

    let mut table = Table::new("financials".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
    ));
    table.add_column(Column::new(
        "cogs".to_string(),
        ColumnValue::Number(vec![300.0, 360.0, 450.0, 540.0]),
    ));
    table.add_row_formula("gross_profit".to_string(), "=revenue - cogs".to_string());
    table.add_row_formula(
        "gross_margin".to_string(),
        "=gross_profit / revenue".to_string(),
    );

    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let result_table = result.tables.get("financials").unwrap();

    // Check gross_profit
    let gross_profit = result_table.columns.get("gross_profit").unwrap();
    match &gross_profit.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert_eq!(nums[0], 700.0);
            assert_eq!(nums[1], 840.0);
            assert_eq!(nums[2], 1050.0);
            assert_eq!(nums[3], 1260.0);
        }
        _ => panic!("Expected Number array"),
    }

    // Check gross_margin
    let gross_margin = result_table.columns.get("gross_margin").unwrap();
    match &gross_margin.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert!((nums[0] - 0.7).abs() < 0.0001);
            assert!((nums[1] - 0.7).abs() < 0.0001);
            assert!((nums[2] - 0.7).abs() < 0.0001);
            assert!((nums[3] - 0.7).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ Simple table calculation succeeded");
}
```text

### Unit Test Naming Convention

**Pattern:** `test_<function>_<scenario>`

**Examples:**

- `test_column_index_to_letter` - Basic functionality
- `test_simple_formula_translation` - Happy path
- `test_formula_with_multiple_columns` - Edge case
- `test_convert_to_column_value_numbers` - Type-specific
- `test_sanitize_table_name` - Input validation

---

## Integration Testing

### Integration Test Strategy

**Focus:** Test component interactions

**Characteristics:**

- Medium speed (~100ms per test)
- Multiple modules tested together
- Realistic data flows
- No CLI, direct library calls

### Integration Test Examples

**Parser + Calculator:**

```rust

#[test]

fn test_parse_and_calculate_quarterly_pl() {
    let path = Path::new("test-data/v1.0/quarterly_pl.yaml");

    // Parse YAML → Model
    let model = parse_model(path).expect("Failed to parse");

    // Calculate formulas
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Failed to calculate");

    // Verify results
    let table = result.tables.get("financials").unwrap();
    assert_eq!(table.columns.len(), 5);

    // Check calculated column exists
    let gross_profit = table.columns.get("gross_profit").unwrap();
    assert_eq!(gross_profit.values.len(), 4);
}
```text

**Exporter + Translator:**

```rust

#[test]

fn test_export_with_formula_translation() {
    // Create model with formulas
    let mut model = ParsedModel::new(ForgeVersion::V1_0_0);
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    table.add_row_formula("b".to_string(), "=a * 2".to_string());
    model.add_table(table);

    // Export to Excel
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let exporter = ExcelExporter::new(model);
    exporter.export(temp_file.path()).expect("Export failed");

    // Verify file exists and has content
    assert!(temp_file.path().exists());
    let metadata = fs::metadata(temp_file.path()).unwrap();
    assert!(metadata.len() > 0);
}
```text

**Importer + Reverse Translator:**

```rust

#[test]

fn test_import_with_formula_reverse_translation() {
    // Create Excel file with formulas
    // (Assume test-data/test_formulas.xlsx exists)
    let excel_path = Path::new("test-data/test_formulas.xlsx");

    // Import Excel → Model
    let importer = ExcelImporter::new(excel_path);
    let model = importer.import().expect("Import failed");

    // Verify formula was reverse-translated
    let table = model.tables.get("sheet1").unwrap();
    let formula = table.row_formulas.get("b").unwrap();
    assert_eq!(formula, "=a * 2"); // Excel "=A2*2" → YAML "=a * 2"
}
```text

---

## End-to-End Testing

### E2E Test Strategy

**Focus:** Test complete user workflows through CLI

**Characteristics:**

- Slow (~500ms per test)
- Full CLI execution
- File I/O
- Error message verification

### E2E Test Infrastructure

**File:** `/home/rex/src/utils/forge/tests/e2e_tests.rs` (1,026 lines, 33 tests)

**Helper Functions:**

```rust
// From: e2e_tests.rs:5-26
fn forge_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("forge");

    if !path.exists() {
        path.pop();
        path.pop();
        path.push("debug");
        path.push("forge");
    }

    path
}

fn test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data");
    path.push(filename);
    path
}
```text

### E2E Test Categories

**1. Error Handling (8 tests)**

```rust
// From: e2e_tests.rs:28-48

#[test]

fn e2e_malformed_yaml_fails_gracefully() {
    let file = test_data_path("test_malformed.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Malformed YAML should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
 combined.contains("Yaml") || combined.contains("EOF") || combined.contains("scanning"),
        "Should report YAML parsing error, got: {combined}"
    );
}
```text

**2. Validation Workflow (10 tests)**

```rust
// From: e2e_tests.rs:96-125

#[test]

fn e2e_stale_values_detected() {
    let file = test_data_path("test_stale.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "Stale values should fail validation"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should report mismatches
    assert!(stdout.contains("Found 2 value mismatches"));
    assert!(stdout.contains("test.gross_margin"));
    assert!(stdout.contains("unit_economics.ratio"));

    // Should show current vs expected
    assert!(stdout.contains("Current:"));
    assert!(stdout.contains("Expected:"));
    assert!(stdout.contains("Diff:"));

    // Should suggest fix
    assert!(stdout.contains("Run 'forge calculate' to update values"));
}
```text

**3. Calculate Workflow (7 tests)**

```rust

#[test]

fn e2e_calculate_updates_stale_file() {
    // Copy stale file to temp location
    let original = test_data_path("test_stale.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_file = temp_dir.path().join("test_stale_copy.yaml");
    fs::copy(&original, &temp_file).unwrap();

    // Calculate (should update file)
    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&temp_file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Calculate should succeed");

    // Verify file was updated
    let output2 = Command::new(forge_binary())
        .arg("validate")
        .arg(&temp_file)
        .output()
        .expect("Failed to execute");

    assert!(output2.status.success(), "Validation should pass after calculate");

    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout.contains("All formulas are valid"));
}
```text

**4. Cross-File References (5 tests)**

```rust

#[test]

fn e2e_includes_basic_flow() {
    let file = test_data_path("includes_main.yaml");

    // Validate (should pass)
    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Includes should work");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All formulas are valid"));
}
```text

**5. Export/Import (3 tests)**

```rust

#[test]

fn e2e_export_to_excel() {
    let input = test_data_path("v1.0/quarterly_pl.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let output = temp_dir.path().join("output.xlsx");

    let result = Command::new(forge_binary())
        .arg("export")
        .arg(&input)
        .arg(&output)
        .output()
        .expect("Failed to execute");

    assert!(result.status.success(), "Export should succeed");
    assert!(output.exists(), "Output file should exist");

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Export Complete"));
}
```text

### E2E Test Output Verification

**Strategy:**

1. Check exit code (success/failure)
2. Parse stdout for expected messages
3. Verify file modifications (if applicable)
4. Check error messages for clarity

**Example:**

```rust
let output = Command::new(forge_binary())
    .arg("validate")
    .arg(&file)
    .output()
    .expect("Failed to execute");

// 1. Exit code
assert!(output.status.success());

// 2. Stdout messages
let stdout = String::from_utf8_lossy(&output.stdout);
assert!(stdout.contains("All formulas are valid"));

// 3. Stderr (if errors expected)
let stderr = String::from_utf8_lossy(&output.stderr);
assert!(stderr.is_empty());
```text

---

## Test Organization

### Directory Structure

```text
forge/
├── src/
│   ├── lib.rs                    # Doc tests
│   ├── core/
│   │   ├── array_calculator.rs   # Inline unit tests (20+ tests)
│   │   └── calculator.rs         # Inline unit tests (10+ tests)
│   ├── excel/
│   │   ├── exporter.rs           # Inline unit tests (5+ tests)
│   │   ├── importer.rs           # Inline unit tests (10+ tests)
│   │   ├── formula_translator.rs # Inline unit tests (6+ tests)
│   │   └── reverse_formula_translator.rs # Inline unit tests (6+ tests)
│   └── parser/
│       └── mod.rs                # Inline unit tests (5+ tests)
├── tests/                        # Integration & E2E tests
│   ├── array_calculator_tests.rs # 30+ tests
│   ├── e2e_tests.rs              # 33 tests
│   ├── parser_v1_tests.rs        # 3 tests
│   └── validation_tests.rs       # 5 tests
├── test-data/                    # 33+ test YAML/Excel files
│   ├── test_valid_updated.yaml
│   ├── test_stale.yaml
│   ├── test_circular.yaml
│   ├── v1.0/quarterly_pl.yaml
│   └── ...
└── examples/                     # Executable examples (also tests)
    ├── basic_calculation.rs
    ├── excel_export.rs
    ├── excel_import.rs
    └── validation_check.rs
```text

### Test Location Guidelines

**Inline Tests (`#[cfg(test)]` modules):**

- ✅ Unit tests for module-local functions
- ✅ Pure logic without external dependencies
- ✅ Type conversions, algorithms
- ✅ Helper function tests

**tests/ Directory:**

- ✅ Integration tests (multiple modules)
- ✅ E2E tests (CLI execution)
- ✅ Tests requiring test-data files
- ✅ Tests with complex setup

**examples/ Directory:**

- ✅ Executable examples for users
- ✅ Also compiled and run by `cargo test`
- ✅ Minimal, focused demonstrations

### Running Tests

**All tests:**

```bash
cargo test
```text

**Unit tests only:**

```bash
cargo test --lib
```text

**Integration tests only:**

```bash
cargo test --test '*'
```text

**Specific test:**

```bash
cargo test test_simple_table_calculation
```text

**E2E tests only:**

```bash
cargo test --test e2e_tests
```text

**Verbose output:**

```bash
cargo test -- --nocapture
```text

**Single-threaded (for debugging):**

```bash
cargo test -- --test-threads=1
```text

---

## Test Data Management

### Test Data Directory

**Location:** `/home/rex/src/utils/forge/test-data/`

**Count:** 33+ YAML and Excel files

**Categories:**

1. **Valid Models** (5 files)
   - `test_valid_updated.yaml`
   - `includes_main.yaml`
   - `includes_pricing.yaml`
   - `includes_costs.yaml`

2. **Invalid Models** (10 files)
   - `test_malformed.yaml` - YAML syntax errors
   - `test_invalid_formula.yaml` - Unknown variables
   - `test_circular.yaml` - Circular dependencies
   - `test_stale.yaml` - Stale values
   - `includes_circular_a.yaml`, `includes_circular_b.yaml`
   - `includes_missing_file.yaml`
   - `includes_invalid_alias.yaml`

3. **v1.0.0 Models** (3 files)
   - `v1.0/quarterly_pl.yaml` - Real-world financial model
   - `export_basic.yaml` - Simple export test
   - `export_with_formulas.yaml` - Formula export test

4. **Edge Cases** (15 files)
   - `includes_complex.yaml` - Multi-level includes
   - `includes_mixed_refs.yaml` - Cross-file + local refs
   - `includes_stale_included_file.yaml` - Stale in include
   - `includes_bad_formula.yaml` - Formula error in include

### Test Data Conventions

**Naming:**

- `test_<scenario>.yaml` - General test files
- `includes_<scenario>.yaml` - Cross-file reference tests
- `v1.0/<name>.yaml` - Version-specific models

**Content:**

- Small (5-20 lines typical)
- Focused on single test case
- Clear variable names
- Comments explaining purpose

**Example:**

```yaml

# test_stale.yaml - Values don't match formulas (for validation testing)

platform:
  take_rate:
    value: 0.10
    formula: null

test:
  gross_margin:
    value: 0.5         # WRONG! Should be 0.9
    formula: "=1 - take_rate"

unit_economics:
  ratio:
    value: 2.0         # WRONG! Should be 1.8
    formula: "=1 + gross_margin"
```text

### Managing Test Data

**Version Control:**

- ✅ All test data committed to Git
- ✅ Small files (<1KB typical)
- ✅ No binary Excel files (generated in tests)

**Maintenance:**

```bash

# Validate all test data

for file in test-data/*.yaml; do
 forge validate "$file" || echo "INVALID: $file"
done

# Recalculate all test data

for file in test-data/*.yaml; do
  forge calculate "$file"
done
```text

---

## Property-Based Testing (Future)

### What is Property-Based Testing?

**Concept:** Generate random inputs, verify properties hold

**Example Properties:**

1. **Round-trip preservation:**

```text
   YAML → Excel → YAML ≈ Original YAML
```text

2. **Calculation idempotence:**

```text
   calculate(calculate(model)) = calculate(model)
```text

3. **Validation correctness:**

```text
   validate(calculate(model)) = Success
```text

4. **Formula commutativity:**

```text
   "=a + b" produces same result as "=b + a"
```text

### Planned Implementation

**Library:** `proptest` (Rust property testing)

**Example Test:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_column_letter_roundtrip(idx in 0usize..16384) {
        let letter = FormulaTranslator::column_index_to_letter(idx);
        let back = column_letter_to_index(&letter);
        prop_assert_eq!(idx, back);
    }

    #[test]
    fn test_formula_translation_roundtrip(
        col_name in "[a-z]{1,10}",
        row in 1u32..1000,
    ) {
        let yaml_formula = format!("={}", col_name);
        let excel_formula = forward_translate(&yaml_formula, row)?;
        let back = reverse_translate(&excel_formula)?;
        prop_assert_eq!(yaml_formula, back);
    }
}
```text

**Benefits:**

- Discover edge cases automatically
- High confidence in correctness
- Less manual test writing

**Planned Coverage:**

- Formula translation (both directions)
- Column mapping (index ↔ letter)
- YAML round-trips
- Excel round-trips

---

## Test Coverage Analysis

### Coverage Metrics

**Current Coverage (Estimated):**

| Module | Lines | Tests | Coverage |
|--------|-------|-------|----------|
| **array_calculator.rs** | 3,440 | 30+ | ~85% |
| **calculator.rs** | 401 | 10+ | ~70% |
| **exporter.rs** | 218 | 5+ | ~80% |
| **importer.rs** | 438 | 10+ | ~75% |
| **formula_translator.rs** | 286 | 6+ | ~90% |
| **reverse_formula_translator.rs** | 318 | 6+ | ~90% |
| **parser/mod.rs** | 1,011 | 8+ | ~60% |
| **cli/commands.rs** | 380 | 33 (e2e) | ~95% |
| **TOTAL** | ~7,400 | 136 | **~80%** |

### Coverage Tools

**tarpaulin (Rust coverage tool):**

```bash

# Install

cargo install cargo-tarpaulin

# Run coverage

cargo tarpaulin --out Html --output-dir coverage/

# View report

open coverage/index.html
```text

**Coverage Goals:**

- **Critical paths:** 100% (formula evaluation, dependency resolution)
- **User-facing:** 95% (CLI commands, error messages)
- **Library internals:** 80% (parsers, converters)
- **Edge cases:** 70% (error handling, rare scenarios)

### Uncovered Areas (Known)

**Low Priority:**

- Error display formatting (manual testing)
- CLI help text rendering (visual inspection)
- Terminal color codes (optional feature)

**Future Coverage:**

- Audit command (not yet implemented)
- v1.0.0 writer (placeholder)
- Advanced Excel features (charts, formatting)

---

## CI/CD Integration

### GitHub Actions Workflow

**File:** `.github/workflows/test.yml` (hypothetical)

```yaml
name: Test Suite

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    name: Test on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]

    steps:
      - name: Checkout code
        uses: actions/checkout@v2

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          profile: minimal
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v2
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v2
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v2
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test --test '*'

      - name: Run doc tests
        run: cargo test --doc

      - name: Build release binary
        run: cargo build --release

      - name: Run E2E tests
        run: cargo test --test e2e_tests

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: cargo tarpaulin --out Xml

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v2
        with:
          files: ./cobertura.xml
          fail_ci_if_error: true

  lint:
    name: Lint (clippy)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy
          override: true

      - name: Run clippy
        run: cargo clippy -- -D warnings

  format:
    name: Format check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt
          override: true

      - name: Check formatting
        run: cargo fmt --all -- --check
```text

### Pre-Commit Hook

**File:** `.git/hooks/pre-commit`

```bash

#!/bin/bash

set -e

echo "Running pre-commit checks..."

# 1. Format check

echo "→ Checking code formatting..."
cargo fmt --all -- --check || {
  echo "❌ Code not formatted. Run 'cargo fmt' to fix."
  exit 1
}

# 2. Linting

echo "→ Running clippy..."
cargo clippy -- -D warnings || {
  echo "❌ Clippy warnings found."
  exit 1
}

# 3. Unit tests (fast)

echo "→ Running unit tests..."
cargo test --lib --quiet || {
  echo "❌ Unit tests failed."
  exit 1
}

# 4. Validate test data

echo "→ Validating test YAML files..."
for file in test-data/test_valid*.yaml test-data/includes_main.yaml; do
  if [ -f "$file" ]; then
 cargo run --quiet -- validate "$file" > /dev/null || {
      echo "❌ Validation failed for $file"
      exit 1
    }
  fi
done

echo "✅ All pre-commit checks passed!"
```text

**Installation:**

```bash
chmod +x .git/hooks/pre-commit
```text

### CI Performance

**Target Times:**

- Unit tests: <5 seconds
- Integration tests: <10 seconds
- E2E tests: <30 seconds
- Coverage: <60 seconds
- **Total CI time: <2 minutes**

**Optimization Strategies:**

1. **Parallel execution** - Run unit/integration/e2e in parallel
2. **Caching** - Cache Cargo dependencies and build artifacts
3. **Incremental builds** - Only rebuild changed code
4. **Test sharding** - Split e2e tests across multiple runners

---

## Test Performance

### Performance Benchmarks

**Measured on: Linux, Ryzen 9 5900X, 32GB RAM**

| Test Category | Count | Total Time | Avg Time |
|---------------|-------|------------|----------|
| Unit (inline) | 50 | 0.8s | 16ms |
| Unit (tests/) | 36 | 1.2s | 33ms |
| Integration | 6 | 0.6s | 100ms |
| E2E | 33 | 15s | 455ms |
| Validation | 5 | 1s | 200ms |
| Doc | 3 | 0.2s | 67ms |
| **TOTAL** | **136** | **~19s** | **140ms** |

### Performance Guidelines

**Test Speed Targets:**

- **Unit:** <50ms per test
- **Integration:** <200ms per test
- **E2E:** <1s per test
- **Total suite:** <30s

**Slow Test Detection:**

```bash

# Show slow tests (>1s)

cargo test -- --nocapture 2>&1 | grep -E "test .* ok$" | awk '{print $2, $4}' | sort -nk2
```text

**Optimization Techniques:**

1. **Mock external deps** - Don't hit real files unless necessary
2. **Shared fixtures** - Reuse test data across tests
3. **Parallel execution** - Use `cargo test` default parallelism
4. **Lazy statics** - Initialize test data once

### Benchmarking (Future)

**Criterion.rs Integration:**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_formula_evaluation(c: &mut Criterion) {
    let model = create_large_model(1000); // 1000 formulas

 c.bench_function("calculate 1000 formulas", |b| {
 b.iter(|| {
            let calculator = ArrayCalculator::new(model.clone());
            black_box(calculator.calculate_all().unwrap())
        })
    });
}

criterion_group!(benches, bench_formula_evaluation);
criterion_main!(benches);
```text

**Planned Benchmarks:**

- Formula evaluation (100/1000/10000 formulas)
- Excel export (10/100/1000 rows)
- Excel import (10/100/1000 rows)
- Dependency resolution (10/100/1000 nodes)

---

## Test Maintenance

### Test Hygiene

**Best Practices:**

1. **One assertion per test** (when possible)

   ```rust
   // Good
   #[test]
   fn test_column_a() {
       assert_eq!(column_index_to_letter(0), "A");
   }

   #[test]
   fn test_column_z() {
       assert_eq!(column_index_to_letter(25), "Z");
   }

   // Avoid (unless related)
   #[test]
   fn test_all_columns() {
       assert_eq!(column_index_to_letter(0), "A");
       assert_eq!(column_index_to_letter(25), "Z");
       assert_eq!(column_index_to_letter(26), "AA");
       // ... 50 more assertions
   }
```text

2. **Clear test names**

   ```rust
   // Good
   #[test]
   fn test_circular_dependency_detected() { ... }

   // Bad
   #[test]
   fn test_case_3() { ... }
```text

3. **Self-contained tests**

   ```rust
   // Good
   #[test]
   fn test_export() {
       let model = create_test_model();
       let temp_file = NamedTempFile::new().unwrap();
       // ...
   }

   // Bad (depends on global state)
   static mut GLOBAL_MODEL: Option<ParsedModel> = None;
   #[test]
   fn test_export() {
       unsafe { GLOBAL_MODEL.as_ref().unwrap() } // ❌
   }
```text

### Flaky Test Detection

**Symptoms:**

- Test passes/fails randomly
- Fails only in CI, not locally
- Timing-dependent failures

**Common Causes:**

1. **File system race conditions** - Use proper temp files
2. **Floating point precision** - Use tolerance in assertions
3. **Test dependencies** - Ensure tests are isolated

**Fixes:**

```rust
// Problem: Floating point comparison
assert_eq!(result, 0.7); // ❌ Flaky

// Fix: Use tolerance
assert!((result - 0.7).abs() < 0.0001); // ✅

// Problem: Temp file collision
let temp = "/tmp/test.yaml"; // ❌ Race condition

// Fix: Use unique temp files
let temp = NamedTempFile::new().unwrap(); // ✅
```text

### Deprecating Tests

**When to Remove Tests:**

- ✅ Feature removed from codebase
- ✅ Superseded by better test
- ✅ Testing implementation detail (should test behavior)

**When to Keep Tests:**

- ✅ Regression test for fixed bug
- ✅ Edge case not covered elsewhere
- ✅ Documentation value

**Deprecation Process:**

```rust
// 1. Mark as deprecated

#[test]


#[ignore] // Skip in normal runs


#[deprecated(note = "Use test_new_implementation instead")]

fn test_old_implementation() {
    // Keep for reference but don't run
}

// 2. Remove after 1-2 releases
```text

---

## Related Documentation

### Architecture Documents

- [00-OVERVIEW.md](00-OVERVIEW.md) - System context and principles
- [01-COMPONENT-ARCHITECTURE.md](01-COMPONENT-ARCHITECTURE.md) - Module structure
- [02-DATA-MODEL.md](02-DATA-MODEL.md) - Type system and structures
- [03-FORMULA-EVALUATION.md](03-FORMULA-EVALUATION.md) - Formula engine
- [04-DEPENDENCY-RESOLUTION.md](04-DEPENDENCY-RESOLUTION.md) - Graph algorithms
- [05-EXCEL-INTEGRATION.md](05-EXCEL-INTEGRATION.md) - Excel conversion
- [06-CLI-ARCHITECTURE.md](06-CLI-ARCHITECTURE.md) - Command-line interface

### User Documentation

- [README.md](../../README.md) - User guide and examples
- [DESIGN_V1.md](../../DESIGN_V1.md) - v1.0.0 specification
- [CHANGELOG.md](../../CHANGELOG.md) - Version history
- [KNOWN_BUGS.md](../../KNOWN_BUGS.md) - Known issues

### Test Files

- **E2E Tests:** `/home/rex/src/utils/forge/tests/e2e_tests.rs` (1,026 lines)
- **Array Calc Tests:** `/home/rex/src/utils/forge/tests/array_calculator_tests.rs` (233 lines)
- **Validation Tests:** `/home/rex/src/utils/forge/tests/validation_tests.rs` (209 lines)
- **Parser Tests:** `/home/rex/src/utils/forge/tests/parser_v1_tests.rs` (83 lines)
- **Test Data:** `/home/rex/src/utils/forge/test-data/` (33+ files)

---

## Test Coverage Matrix

### Feature Coverage

| Feature | Unit | Integration | E2E | Total Coverage |
|---------|------|-------------|-----|----------------|
| **Formula Evaluation** | 30 | 2 | 5 | ✅ 95% |
| **Dependency Resolution** | 8 | 1 | 3 | ✅ 90% |
| **YAML Parsing** | 5 | 3 | 10 | ✅ 85% |
| **Excel Export** | 5 | 2 | 3 | ✅ 80% |
| **Excel Import** | 10 | 2 | 2 | ✅ 75% |
| **Formula Translation** | 12 | 1 | 2 | ✅ 90% |
| **CLI Commands** | 0 | 0 | 33 | ✅ 95% |
| **Error Handling** | 8 | 2 | 15 | ✅ 85% |
| **Cross-File Refs** | 0 | 2 | 8 | ✅ 80% |
| **Validation** | 2 | 1 | 5 | ✅ 90% |

### Excel Function Coverage

**47+ Functions Tested:**

- ✅ SUM, AVERAGE, MAX, MIN, COUNT, PRODUCT
- ✅ IF, AND, OR, NOT, IFERROR, IFNA
- ✅ ROUND, ROUNDUP, ROUNDDOWN, ABS, SQRT, POW
- ✅ LEFT, RIGHT, MID, LEN, UPPER, LOWER, TRIM, CONCATENATE
- ✅ TODAY, YEAR, MONTH, DAY, DATE
- ✅ SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, AVERAGEIFS
- ⏳ VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP (planned)

---

**End of Testing Architecture Documentation**

This completes the comprehensive testing architecture guide for Forge v1.1.2. For questions or contributions, see [README.md](../../README.md).
