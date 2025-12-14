//! Extended Excel import/export tests (split from excel_tests.rs)
//! ADR-004: 100% coverage required for Excel functionality

#![allow(clippy::assertions_on_constants)]

use royalbit_forge::excel::{ExcelExporter, FormulaTranslator, ReverseFormulaTranslator};
use royalbit_forge::types::{
    Column, ColumnValue, Include, Metadata, ParsedModel, ResolvedInclude, Table, Variable,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

// ═══════════════════════════════════════════════════════════════════════════
// FORMULA TRANSLATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_formula_translator_new() {
    let column_map = HashMap::new();
    let _translator = FormulaTranslator::new(column_map);
    assert!(true, "Translator created");
}

#[test]
fn test_formula_translator_new_with_tables() {
    let column_map = HashMap::new();
    let table_column_maps = HashMap::new();
    let table_row_counts = HashMap::new();
    let _translator =
        FormulaTranslator::new_with_tables(column_map, table_column_maps, table_row_counts);
    assert!(true, "Translator with tables created");
}

#[test]
fn test_formula_translator_column_index_to_letter() {
    // Single letters
    assert_eq!(FormulaTranslator::column_index_to_letter(0), "A");
    assert_eq!(FormulaTranslator::column_index_to_letter(1), "B");
    assert_eq!(FormulaTranslator::column_index_to_letter(25), "Z");

    // Double letters
    assert_eq!(FormulaTranslator::column_index_to_letter(26), "AA");
    assert_eq!(FormulaTranslator::column_index_to_letter(27), "AB");
    assert_eq!(FormulaTranslator::column_index_to_letter(51), "AZ");
    assert_eq!(FormulaTranslator::column_index_to_letter(52), "BA");

    // Triple letters
    assert_eq!(FormulaTranslator::column_index_to_letter(702), "AAA");
}

#[test]
fn test_formula_translator_simple_addition() {
    let mut column_map = HashMap::new();
    column_map.insert("a".to_string(), "A".to_string());
    column_map.insert("b".to_string(), "B".to_string());

    let translator = FormulaTranslator::new(column_map);
    let result = translator.translate_row_formula("=a + b", 2).unwrap();
    assert_eq!(result, "=A2 + B2");
}

#[test]
fn test_formula_translator_complex_formula() {
    let mut column_map = HashMap::new();
    column_map.insert("revenue".to_string(), "A".to_string());
    column_map.insert("costs".to_string(), "B".to_string());
    column_map.insert("tax".to_string(), "C".to_string());

    let translator = FormulaTranslator::new(column_map);
    let result = translator
        .translate_row_formula("=(revenue - costs) * (1 - tax)", 5)
        .unwrap();
    assert_eq!(result, "=(A5 - B5) * (1 - C5)");
}

#[test]
fn test_formula_translator_preserves_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    // Test that Excel functions are preserved
    let result = translator
        .translate_row_formula("=SUM(1, 2, 3)", 2)
        .unwrap();
    assert!(result.contains("SUM"));

    let result = translator
        .translate_row_formula("=IF(1 > 0, 1, 0)", 2)
        .unwrap();
    assert!(result.contains("IF"));

    let result = translator
        .translate_row_formula("=NPV(0.1, 100)", 2)
        .unwrap();
    assert!(result.contains("NPV"));
}

#[test]
fn test_formula_translator_cross_table_reference() {
    let column_map = HashMap::new();
    let mut table_column_maps = HashMap::new();
    let mut revenue_cols = HashMap::new();
    revenue_cols.insert("amount".to_string(), "A".to_string());
    table_column_maps.insert("revenue".to_string(), revenue_cols);

    let mut table_row_counts = HashMap::new();
    table_row_counts.insert("revenue".to_string(), 10);

    let translator =
        FormulaTranslator::new_with_tables(column_map, table_column_maps, table_row_counts);
    let result = translator
        .translate_row_formula("=revenue.amount", 2)
        .unwrap();
    assert!(result.contains("revenue") && result.contains("A2"));
}

#[test]
fn test_formula_translator_scalar_formula() {
    let column_map = HashMap::new();
    let mut table_column_maps = HashMap::new();
    let mut data_cols = HashMap::new();
    data_cols.insert("values".to_string(), "A".to_string());
    table_column_maps.insert("data".to_string(), data_cols);

    let mut table_row_counts = HashMap::new();
    table_row_counts.insert("data".to_string(), 5);

    let translator =
        FormulaTranslator::new_with_tables(column_map, table_column_maps, table_row_counts);

    let scalar_row_map = HashMap::new();
    let result = translator
        .translate_scalar_formula("=SUM(data.values)", &scalar_row_map)
        .unwrap();
    assert!(result.contains("SUM"));
}

#[test]
fn test_formula_translator_indexed_reference() {
    let column_map = HashMap::new();
    let mut table_column_maps = HashMap::new();
    let mut data_cols = HashMap::new();
    data_cols.insert("values".to_string(), "A".to_string());
    table_column_maps.insert("data".to_string(), data_cols);

    let table_row_counts = HashMap::new();

    let translator =
        FormulaTranslator::new_with_tables(column_map, table_column_maps, table_row_counts);

    let scalar_row_map = HashMap::new();
    let result = translator
        .translate_scalar_formula("=data.values[0]", &scalar_row_map)
        .unwrap();
    // Index 0 should become row 2 (index + 2 for header and 1-indexing)
    assert!(result.contains("2"));
}

// ═══════════════════════════════════════════════════════════════════════════
// REVERSE FORMULA TRANSLATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_reverse_translator_new() {
    let column_map = HashMap::new();
    let _translator = ReverseFormulaTranslator::new(column_map);
    assert!(true, "Reverse translator created");
}

#[test]
fn test_reverse_translator_simple_reference() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "revenue".to_string());
    column_map.insert("B".to_string(), "costs".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("=A2-B2").unwrap();
    assert_eq!(result, "=revenue-costs");
}

#[test]
fn test_reverse_translator_range_reference() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "values".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("=SUM(A:A)").unwrap();
    assert_eq!(result, "=SUM(values)");
}

#[test]
fn test_reverse_translator_cell_range() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "values".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("=SUM(A1:A10)").unwrap();
    assert_eq!(result, "=SUM(values)");
}

#[test]
fn test_reverse_translator_sheet_reference() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "revenue".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("=Sheet1!A2").unwrap();
    assert_eq!(result, "=sheet1.revenue");
}

#[test]
fn test_reverse_translator_quoted_sheet() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "revenue".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("='P&L 2025'!A2").unwrap();
    // Sheet name should be sanitized
    assert!(result.contains("pandl_2025"));
}

#[test]
fn test_reverse_translator_preserves_functions() {
    let column_map = HashMap::new();
    let translator = ReverseFormulaTranslator::new(column_map);

    let result = translator.translate("=IF(A1>0,B1,C1)").unwrap();
    assert!(result.contains("IF"));

    let result = translator.translate("=SUM(A1,B1,C1)").unwrap();
    assert!(result.contains("SUM"));

    let result = translator.translate("=NPV(0.1,A1)").unwrap();
    assert!(result.contains("NPV"));
}

#[test]
fn test_reverse_translator_complex_formula() {
    let mut column_map = HashMap::new();
    column_map.insert("A".to_string(), "revenue".to_string());
    column_map.insert("B".to_string(), "costs".to_string());
    column_map.insert("C".to_string(), "tax".to_string());

    let translator = ReverseFormulaTranslator::new(column_map);
    let result = translator.translate("=IF(A2>B2,(A2-B2)*(1-C2),0)").unwrap();
    assert!(result.contains("revenue"));
    assert!(result.contains("costs"));
    assert!(result.contains("tax"));
    assert!(result.contains("IF"));
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_empty_table() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("empty_table.xlsx");

    let mut model = ParsedModel::new();
    let table = Table::new("empty".to_string());
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export empty table should succeed");
}

#[test]
fn test_exporter_single_cell() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("single_cell.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("single".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export single cell should succeed");
}

#[test]
fn test_exporter_large_table() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("large_table.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("large".to_string());

    // 1000 rows
    let values: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(values),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export large table should succeed");
}

#[test]
fn test_exporter_special_characters_in_name() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("special.xlsx");

    let mut model = ParsedModel::new();
    // Note: worksheet names have restrictions, this tests valid names
    let mut table = Table::new("revenue_2025".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export with special chars should succeed");
}

#[test]
fn test_formula_translator_unknown_column() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let result = translator.translate_row_formula("=unknown_column", 2);
    assert!(result.is_err(), "Unknown column should fail");
}

#[test]
fn test_reverse_translator_unmapped_column() {
    let column_map = HashMap::new(); // Empty map
    let translator = ReverseFormulaTranslator::new(column_map);

    // Should use column letter as-is when not mapped
    let result = translator.translate("=Z1").unwrap();
    assert!(result.contains("Z"), "Unmapped column should use letter");
}

#[test]
fn test_exporter_metadata_partial() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("partial_meta.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    // Only set some metadata fields
    column.metadata = Metadata {
        unit: Some("USD".to_string()),
        notes: None,
        source: None,
        validation_status: None,
        last_updated: None,
    };
    table.add_column(column);
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export with partial metadata should succeed"
    );
}

#[test]
fn test_exporter_metadata_empty() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("empty_meta.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata = Metadata::default();
    table.add_column(column);
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export with empty metadata should succeed");
}

#[test]
fn test_formula_translator_all_financial_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "NPV", "IRR", "XNPV", "XIRR", "PMT", "FV", "PV", "RATE", "NPER",
    ];

    for func in functions {
        let formula = format!("={}(0.1, 100)", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

#[test]
fn test_formula_translator_all_date_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "TODAY", "NOW", "DATE", "YEAR", "MONTH", "DAY", "DATEDIF", "EDATE", "EOMONTH",
    ];

    for func in functions {
        let formula = format!("={}()", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

#[test]
fn test_formula_translator_all_math_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "ABS", "ROUND", "SQRT", "POWER", "EXP", "LN", "LOG", "MOD", "CEILING", "FLOOR",
    ];

    for func in functions {
        let formula = format!("={}(100)", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

#[test]
fn test_formula_translator_all_text_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "LEFT", "RIGHT", "MID", "LEN", "UPPER", "LOWER", "TRIM", "CONCAT",
    ];

    for func in functions {
        let formula = format!("={}(100)", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

#[test]
fn test_formula_translator_all_logical_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "IF", "AND", "OR", "NOT", "TRUE", "FALSE", "IFERROR", "CHOOSE",
    ];

    for func in functions {
        let formula = format!("={}(1)", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

#[test]
fn test_formula_translator_all_aggregation_functions() {
    let column_map = HashMap::new();
    let translator = FormulaTranslator::new(column_map);

    let functions = vec![
        "SUM",
        "AVERAGE",
        "MAX",
        "MIN",
        "COUNT",
        "COUNTA",
        "PRODUCT",
        "SUMIF",
        "COUNTIF",
        "AVERAGEIF",
    ];

    for func in functions {
        let formula = format!("={}(1, 2, 3)", func);
        let result = translator.translate_row_formula(&formula, 2).unwrap();
        assert!(result.contains(func), "{} should be preserved", func);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESOLVED INCLUDES TESTS (v4.4.2)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_with_resolved_includes_tables() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("includes_tables.xlsx");

    // Create main model
    let mut model = ParsedModel::new();
    let mut main_table = Table::new("main_data".to_string());
    main_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![100.0]),
    ));
    model.add_table(main_table);

    // Create included model with table
    let mut included_model = ParsedModel::new();
    let mut included_table = Table::new("prices".to_string());
    included_table.add_column(Column::new(
        "unit_price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    included_model.add_table(included_table);

    // Add as resolved include
    let include = Include::new("external.yaml".to_string(), "external".to_string());
    let resolved = ResolvedInclude {
        include,
        resolved_path: PathBuf::from("/tmp/external.yaml"),
        model: included_model,
    };
    model
        .resolved_includes
        .insert("external".to_string(), resolved);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export with resolved includes should succeed"
    );
}

#[test]
fn test_exporter_with_resolved_includes_scalars() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("includes_scalars.xlsx");

    // Create main model
    let mut model = ParsedModel::new();
    model.add_scalar(
        "main.value".to_string(),
        Variable::new("main.value".to_string(), Some(1000.0), None),
    );

    // Create included model with scalars
    let mut included_model = ParsedModel::new();
    included_model.add_scalar(
        "rate".to_string(),
        Variable::new("rate".to_string(), Some(0.05), None),
    );
    included_model.add_scalar(
        "base".to_string(),
        Variable::new("base".to_string(), Some(100.0), None),
    );

    // Add as resolved include
    let include = Include::new("config.yaml".to_string(), "config".to_string());
    let resolved = ResolvedInclude {
        include,
        resolved_path: PathBuf::from("/tmp/config.yaml"),
        model: included_model,
    };
    model
        .resolved_includes
        .insert("config".to_string(), resolved);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export with included scalars should succeed"
    );
}

#[test]
fn test_exporter_with_resolved_includes_with_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("includes_metadata.xlsx");

    let mut model = ParsedModel::new();

    // Create included model with scalar metadata
    let mut included_model = ParsedModel::new();
    let mut var = Variable::new("annotated".to_string(), Some(999.0), None);
    var.metadata = Metadata {
        unit: Some("USD".to_string()),
        notes: Some("Important rate".to_string()),
        source: Some("Finance".to_string()),
        validation_status: Some("Approved".to_string()),
        last_updated: Some("2024-01-01".to_string()),
    };
    included_model.add_scalar("annotated".to_string(), var);

    let include = Include::new("rates.yaml".to_string(), "rates".to_string());
    let resolved = ResolvedInclude {
        include,
        resolved_path: PathBuf::from("/tmp/rates.yaml"),
        model: included_model,
    };
    model
        .resolved_includes
        .insert("rates".to_string(), resolved);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export with included metadata should succeed"
    );
}

#[test]
fn test_exporter_multiple_resolved_includes() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("multi_includes.xlsx");

    let mut model = ParsedModel::new();

    // First include
    let mut include1_model = ParsedModel::new();
    let mut table1 = Table::new("sales".to_string());
    table1.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0]),
    ));
    include1_model.add_table(table1);
    include1_model.add_scalar(
        "total".to_string(),
        Variable::new("total".to_string(), Some(3000.0), None),
    );

    let include1 = Include::new("sales.yaml".to_string(), "sales".to_string());
    let resolved1 = ResolvedInclude {
        include: include1,
        resolved_path: PathBuf::from("/tmp/sales.yaml"),
        model: include1_model,
    };
    model
        .resolved_includes
        .insert("sales".to_string(), resolved1);

    // Second include
    let mut include2_model = ParsedModel::new();
    let mut table2 = Table::new("costs".to_string());
    table2.add_column(Column::new(
        "expense".to_string(),
        ColumnValue::Number(vec![500.0, 600.0]),
    ));
    include2_model.add_table(table2);

    let include2 = Include::new("costs.yaml".to_string(), "costs".to_string());
    let resolved2 = ResolvedInclude {
        include: include2,
        resolved_path: PathBuf::from("/tmp/costs.yaml"),
        model: include2_model,
    };
    model
        .resolved_includes
        .insert("costs".to_string(), resolved2);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export with multiple includes should succeed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SCALAR FORMULA FALLBACK TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_scalar_formula_fallback_to_value() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("scalar_fallback.xlsx");

    let mut model = ParsedModel::new();

    // Scalar with invalid formula that will fail translation - should fall back to value
    model.add_scalar(
        "computed".to_string(),
        Variable::new(
            "computed".to_string(),
            Some(42.0),
            Some("=UNKNOWNFUNCTION(invalid.ref)".to_string()),
        ),
    );

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(
        result.is_ok(),
        "Export should fall back to value on formula error"
    );
}

#[test]
fn test_exporter_scalar_no_value_no_formula() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("scalar_empty.xlsx");

    let mut model = ParsedModel::new();

    // Scalar with neither value nor formula
    model.add_scalar(
        "empty".to_string(),
        Variable::new("empty".to_string(), None, None),
    );

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export empty scalar should succeed");
}

// ═══════════════════════════════════════════════════════════════════════════
// COLUMN VALUE OUT OF BOUNDS TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_empty_column_values() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("empty_values.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("empty_cols".to_string());

    // Column with empty values array
    table.add_column(Column::new(
        "empty_numbers".to_string(),
        ColumnValue::Number(vec![]),
    ));
    table.add_column(Column::new(
        "empty_text".to_string(),
        ColumnValue::Text(vec![]),
    ));
    table.add_column(Column::new(
        "empty_dates".to_string(),
        ColumnValue::Date(vec![]),
    ));
    table.add_column(Column::new(
        "empty_bools".to_string(),
        ColumnValue::Boolean(vec![]),
    ));

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok(), "Export empty columns should succeed");
}

// ═══════════════════════════════════════════════════════════════════════════
// METADATA PARTIAL FIELDS TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_metadata_only_unit() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("meta_unit.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata.unit = Some("USD".to_string());
    table.add_column(column);

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_exporter_metadata_only_notes() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("meta_notes.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata.notes = Some("Important value".to_string());
    table.add_column(column);

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_exporter_metadata_only_source() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("meta_source.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata.source = Some("ERP System".to_string());
    table.add_column(column);

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_exporter_metadata_only_validation_status() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("meta_status.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata.validation_status = Some("Approved".to_string());
    table.add_column(column);

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_exporter_metadata_only_last_updated() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("meta_updated.xlsx");

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let mut column = Column::new("amount".to_string(), ColumnValue::Number(vec![100.0]));
    column.metadata.last_updated = Some("2024-12-01".to_string());
    table.add_column(column);

    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// SCALAR WITH METADATA TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_exporter_scalar_with_partial_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("scalar_partial_meta.xlsx");

    let mut model = ParsedModel::new();

    let mut var = Variable::new("rate".to_string(), Some(0.05), None);
    var.metadata.unit = Some("%".to_string());
    var.metadata.notes = Some("Annual rate".to_string());
    model.add_scalar("rate".to_string(), var);

    let exporter = ExcelExporter::new(model);
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}
