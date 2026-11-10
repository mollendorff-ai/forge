use super::*;
use crate::types::{Column, Variable};

// =========================================================================
// Metadata Note Formatting Tests
// =========================================================================

#[test]
fn test_format_metadata_note_empty() {
    let metadata = Metadata::default();
    assert!(ExcelExporter::format_metadata_note(&metadata).is_none());
}

#[test]
fn test_format_metadata_note_with_unit() {
    let metadata = Metadata {
        unit: Some("CAD".to_string()),
        ..Default::default()
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Unit: CAD"));
}

#[test]
fn test_format_metadata_note_with_notes() {
    let metadata = Metadata {
        notes: Some("Revenue projection".to_string()),
        ..Default::default()
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Notes: Revenue projection"));
}

#[test]
fn test_format_metadata_note_with_source() {
    let metadata = Metadata {
        source: Some("data.yaml".to_string()),
        ..Default::default()
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Source: data.yaml"));
}

#[test]
fn test_format_metadata_note_with_validation_status() {
    let metadata = Metadata {
        validation_status: Some("VALIDATED".to_string()),
        ..Default::default()
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Status: VALIDATED"));
}

#[test]
fn test_format_metadata_note_with_last_updated() {
    let metadata = Metadata {
        last_updated: Some("2025-01-01".to_string()),
        ..Default::default()
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Updated: 2025-01-01"));
}

#[test]
fn test_format_metadata_note_multiple_fields() {
    let metadata = Metadata {
        unit: Some("CAD".to_string()),
        notes: Some("Important".to_string()),
        source: Some("finance.yaml".to_string()),
        validation_status: Some("PROJECTED".to_string()),
        last_updated: Some("2025-11-26".to_string()),
        better: None,
    };
    let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
    assert!(note.contains("Unit: CAD"));
    assert!(note.contains("Notes: Important"));
    assert!(note.contains("Source: finance.yaml"));
    assert!(note.contains("Status: PROJECTED"));
    assert!(note.contains("Updated: 2025-11-26"));
    // Check newlines
    assert!(note.contains('\n'));
}

// =========================================================================
// ExcelExporter Construction Tests
// =========================================================================

#[test]
fn test_exporter_new_empty_model() {
    let model = ParsedModel::new();
    let exporter = ExcelExporter::new(model);
    assert!(exporter.refs.columns.is_empty());
    assert!(exporter.refs.rows.is_empty());
}

#[test]
fn test_exporter_new_with_table() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    assert!(exporter.refs.columns.contains_key("sales"));
    assert_eq!(exporter.refs.rows.get("sales"), Some(&3));
}

#[test]
fn test_exporter_new_with_row_formula() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("calc".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0]),
    ));
    table.add_row_formula("total".to_string(), "=SUM(amount)".to_string());
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    // Column map should include both data column and formula column
    let col_map = exporter.refs.columns.get("calc").unwrap();
    assert!(col_map.contains_key("amount"));
    assert!(col_map.contains_key("total"));
}

#[test]
fn test_exporter_new_multiple_tables() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("sales".to_string());
    table1.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    model.add_table(table1);

    let mut table2 = Table::new("costs".to_string());
    table2.add_column(Column::new(
        "expense".to_string(),
        ColumnValue::Number(vec![50.0, 75.0, 100.0]),
    ));
    model.add_table(table2);

    let exporter = ExcelExporter::new(model);

    assert!(exporter.refs.columns.contains_key("sales"));
    assert!(exporter.refs.columns.contains_key("costs"));
    assert_eq!(exporter.refs.rows.get("sales"), Some(&2));
    assert_eq!(exporter.refs.rows.get("costs"), Some(&3));
}

#[test]
fn test_exporter_column_maps_keep_model_order() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "zebra".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    table.add_column(Column::new(
        "alpha".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    table.add_column(Column::new(
        "beta".to_string(),
        ColumnValue::Number(vec![3.0]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    let col_map = exporter.refs.columns.get("data").unwrap();

    // zebra -> A, alpha -> B, beta -> C (the order the columns were added)
    assert_eq!(col_map.get("zebra"), Some(&"A".to_string()));
    assert_eq!(col_map.get("alpha"), Some(&"B".to_string()));
    assert_eq!(col_map.get("beta"), Some(&"C".to_string()));
}

#[test]
fn test_exporter_empty_table() {
    let mut model = ParsedModel::new();
    let table = Table::new("empty".to_string());
    model.add_table(table);

    let exporter = ExcelExporter::new(model);
    assert_eq!(exporter.refs.rows.get("empty"), Some(&0));
}

// =========================================================================
// Excel Export Tests (File I/O)
// =========================================================================

#[test]
fn test_export_empty_model() {
    use tempfile::TempDir;

    let model = ParsedModel::new();
    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("empty.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_single_table() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("sales.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());

    // File should have non-zero size
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_export_with_text_column() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("names.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_with_date_column() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("timeline".to_string());
    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(),
            "2024-02-01".to_string(),
            "2024-03-01".to_string(),
        ]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("dates.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_with_boolean_column() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("flags".to_string());
    table.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false, true]),
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("flags.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_with_row_formula() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("calc".to_string());
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    table.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 4.0]),
    ));
    table.add_row_formula("total".to_string(), "=price * quantity".to_string());
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("calculated.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_with_scalars() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "tax_rate".to_string(),
        Variable::new("tax_rate".to_string(), Some(0.15), None),
    );
    model.add_scalar(
        "profit".to_string(),
        Variable::new(
            "profit".to_string(),
            Some(50000.0),
            Some("=revenue - costs".to_string()),
        ),
    );

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("scalars.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_multiple_tables() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    let mut revenue_table = Table::new("revenue".to_string());
    revenue_table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0, 3000.0]),
    ));
    model.add_table(revenue_table);

    let mut costs_table = Table::new("costs".to_string());
    costs_table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![500.0, 750.0, 1000.0]),
    ));
    model.add_table(costs_table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("multi.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_with_metadata() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    let metadata = Metadata {
        unit: Some("CAD".to_string()),
        notes: Some("Revenue data".to_string()),
        source: Some("finance.yaml".to_string()),
        validation_status: Some("VALIDATED".to_string()),
        last_updated: Some("2024-01-01".to_string()),
        better: None,
    };

    table.add_column(Column::with_metadata(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
        metadata,
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("metadata.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_mixed_column_types() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("mixed".to_string());

    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    table.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec!["A".to_string(), "B".to_string()]),
    ));
    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-02-01".to_string()]),
    ));
    table.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false]),
    ));

    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("mixed.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_to_nonexistent_directory_fails() {
    let model = ParsedModel::new();
    let exporter = ExcelExporter::new(model);

    let output_path = std::path::Path::new("/nonexistent/dir/output.xlsx");

    let result = exporter.export(output_path);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// Additional coverage tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_export_with_row_formulas() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    table.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));

    // Add a formula column
    table
        .row_formulas
        .insert("total".to_string(), "=price * quantity".to_string());

    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("formulas.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_scalars_with_formulas() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // Add input scalar (no formula)
    model.scalars.insert(
        "inputs.rate".to_string(),
        crate::types::Variable::new("inputs.rate".to_string(), Some(0.05), None),
    );

    // Add output scalar (with formula)
    model.scalars.insert(
        "outputs.result".to_string(),
        crate::types::Variable::new(
            "outputs.result".to_string(),
            Some(500.0),
            Some("=inputs.rate * 10000".to_string()),
        ),
    );

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("scalars.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_aggregations() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // Add a table
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(table);

    // Add aggregation
    model
        .aggregations
        .insert("total_sales".to_string(), "=SUM(sales.amount)".to_string());

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("aggregations.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_export_empty_table() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let table = Table::new("empty".to_string());
    // Don't add any columns
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("empty.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_export_large_table() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("large".to_string());

    // Create 1000 row table
    let values: Vec<f64> = (0..1000).map(f64::from).collect();
    table.add_column(Column::new("id".to_string(), ColumnValue::Number(values)));

    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("large.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_export_cross_table_formula() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // First table
    let mut revenue = Table::new("revenue".to_string());
    revenue.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0]),
    ));
    model.add_table(revenue);

    // Second table referencing first
    let mut profit = Table::new("profit".to_string());
    profit.add_column(Column::new(
        "margin".to_string(),
        ColumnValue::Number(vec![0.2, 0.3]),
    ));
    profit
        .row_formulas
        .insert("amount".to_string(), "=revenue.amount * margin".to_string());
    model.add_table(profit);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("cross_table.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_export_all_metadata_fields() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    let mut table = Table::new("complete".to_string());

    // Full metadata
    let metadata = Metadata {
        unit: Some("USD".to_string()),
        notes: Some("Complete metadata test".to_string()),
        source: Some("test.yaml".to_string()),
        validation_status: Some("PENDING".to_string()),
        last_updated: Some("2025-12-04".to_string()),
        better: None,
    };

    table.add_column(Column::with_metadata(
        "value".to_string(),
        ColumnValue::Number(vec![42.0]),
        metadata,
    ));
    model.add_table(table);

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("full_metadata.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

// =========================================================================
// Grouped Scalar Export Tests (v5.0.0 scalar models)
// =========================================================================

#[test]
fn test_export_grouped_scalars() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // Add scalars in different groups (simulating v5.0.0 scalar model)
    // Group: utilities
    model.scalars.insert(
        "utilities.extinction".to_string(),
        Variable::new("utilities.extinction".to_string(), Some(0.0), None),
    );
    model.scalars.insert(
        "utilities.flourishing".to_string(),
        Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
    );

    // Group: scenario_probs
    model.scalars.insert(
        "scenario_probs.p_unaligned".to_string(),
        Variable::new("scenario_probs.p_unaligned".to_string(), Some(0.35), None),
    );
    model.scalars.insert(
        "scenario_probs.p_aligned".to_string(),
        Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
    );

    // Group: analysis (with formula referencing other groups)
    model.scalars.insert(
        "analysis.expected_value".to_string(),
        Variable::new(
            "analysis.expected_value".to_string(),
            Some(25.0),
            Some("=scenario_probs.p_aligned * utilities.flourishing".to_string()),
        ),
    );

    let exporter = ExcelExporter::new(model);

    // Verify scalar locations were built correctly
    assert!(exporter.refs.scalars.contains_key("utilities.extinction"));
    assert!(exporter
        .refs
        .scalars
        .contains_key("scenario_probs.p_unaligned"));
    assert!(exporter
        .refs
        .scalars
        .contains_key("analysis.expected_value"));

    // Verify worksheet assignments
    let util_loc = exporter.refs.scalars.get("utilities.extinction").unwrap();
    assert_eq!(util_loc.0, "utilities");

    let prob_loc = exporter
        .refs
        .scalars
        .get("scenario_probs.p_unaligned")
        .unwrap();
    assert_eq!(prob_loc.0, "scenario_probs");

    let analysis_loc = exporter
        .refs
        .scalars
        .get("analysis.expected_value")
        .unwrap();
    assert_eq!(analysis_loc.0, "analysis");

    // Export and verify file creation
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("grouped_scalars.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_export_scalars_without_dots() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // Scalars without dots should go to "Scalars" worksheet
    model.scalars.insert(
        "tax_rate".to_string(),
        Variable::new("tax_rate".to_string(), Some(0.15), None),
    );
    model.scalars.insert(
        "discount_rate".to_string(),
        Variable::new("discount_rate".to_string(), Some(0.10), None),
    );

    let exporter = ExcelExporter::new(model);

    // Verify they go to "Scalars" worksheet
    let tax_loc = exporter.refs.scalars.get("tax_rate").unwrap();
    assert_eq!(tax_loc.0, "Scalars");

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("ungrouped_scalars.xlsx");

    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}

#[test]
fn test_export_warns_when_scenarios_are_not_exported() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        Variable::new("rate".to_string(), Some(0.1), None),
    );
    for name in ["bull", "base"] {
        model.scenarios.insert(
            name.to_string(),
            crate::types::Scenario {
                overrides: std::collections::HashMap::from([("rate".to_string(), 0.2)]),
            },
        );
    }

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("with_scenarios.xlsx");
    let warnings = ExcelExporter::new(model)
        .export_with_warnings(&path)
        .unwrap();

    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("scenarios: not exported (base, bull)"),
        "{}",
        warnings[0]
    );
}

#[test]
fn test_export_without_scenarios_has_no_scenario_warning() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        Variable::new("rate".to_string(), Some(0.1), None),
    );

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("no_scenarios.xlsx");
    let warnings = ExcelExporter::new(model)
        .export_with_warnings(&path)
        .unwrap();

    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn test_translate_grouped_scalar_formula() {
    use super::super::workbook_refs::Context;
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "utilities.flourishing".to_string(),
        Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "scenario_probs.p_aligned".to_string(),
        Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
    );
    let exporter = ExcelExporter::new(model);
    let t = exporter
        .refs
        .to_excel(
            "=scenario_probs.p_aligned * utilities.flourishing",
            Context::Scalar {
                group: Some("analysis"),
            },
        )
        .unwrap();
    assert_eq!(t.formula, "='scenario_probs'!$B$2 * 'utilities'!$B$2");
}

#[test]
fn test_export_scalar_with_mc_formula() {
    use tempfile::TempDir;

    let mut model = ParsedModel::new();

    // Scalar with Monte Carlo formula (not supported in Excel)
    model.scalars.insert(
        "inputs.probability".to_string(),
        Variable::new(
            "inputs.probability".to_string(),
            Some(0.35), // Calculated value
            Some("=MC.Triangular(0.25, 0.35, 0.45)".to_string()),
        ),
    );

    let exporter = ExcelExporter::new(model);

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("mc_scalar.xlsx");

    // Should succeed - MC formulas are written as values with formula in comment
    let result = exporter.export(&output_path);
    assert!(result.is_ok());
}
