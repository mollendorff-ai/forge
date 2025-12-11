//! Tests for the parser module
//!
//! Split from mod.rs for file size management (v5.4.0)

use super::*;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

// =========================================================================
// v1.0.0 Array Model Tests
// =========================================================================

#[test]
fn test_parse_number_array() {
    let yaml_seq: Vec<Value> = vec![
        Value::Number(1.into()),
        Value::Number(2.into()),
        Value::Number(3.into()),
    ];
    let result = parse_array_value("test_col", &yaml_seq).unwrap();

    match result {
        ColumnValue::Number(nums) => {
            assert_eq!(nums, vec![1.0, 2.0, 3.0]);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_parse_text_array() {
    let yaml_seq: Vec<Value> = vec![
        Value::String("A".to_string()),
        Value::String("B".to_string()),
        Value::String("C".to_string()),
    ];
    let result = parse_array_value("test_col", &yaml_seq).unwrap();

    match result {
        ColumnValue::Text(texts) => {
            assert_eq!(texts, vec!["A", "B", "C"]);
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_parse_date_array() {
    let yaml_seq: Vec<Value> = vec![
        Value::String("2025-01".to_string()),
        Value::String("2025-02".to_string()),
        Value::String("2025-03".to_string()),
    ];
    let result = parse_array_value("test_col", &yaml_seq).unwrap();

    match result {
        ColumnValue::Date(dates) => {
            assert_eq!(dates, vec!["2025-01", "2025-02", "2025-03"]);
        }
        _ => panic!("Expected Date array"),
    }
}

#[test]
fn test_parse_boolean_array() {
    let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)];
    let result = parse_array_value("test_col", &yaml_seq).unwrap();

    match result {
        ColumnValue::Boolean(bools) => {
            assert_eq!(bools, vec![true, false, true]);
        }
        _ => panic!("Expected Boolean array"),
    }
}

#[test]
fn test_mixed_type_array_error() {
    let yaml_seq: Vec<Value> = vec![Value::Number(1.into()), Value::String("text".to_string())];
    let result = parse_array_value("test_col", &yaml_seq);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Expected Number, found String"));
}

#[test]
fn test_empty_array_error() {
    let yaml_seq: Vec<Value> = vec![];
    let result = parse_array_value("test_col", &yaml_seq);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot be empty"));
}

#[test]
fn test_invalid_date_format_error() {
    let yaml_seq: Vec<Value> = vec![
        Value::String("2025-01".to_string()),
        Value::String("2025-1".to_string()),
    ];
    let result = parse_array_value("test_col", &yaml_seq);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid date format"));
}

#[test]
fn test_valid_date_formats() {
    assert!(is_valid_date_format("2025-01"));
    assert!(is_valid_date_format("2025-12"));
    assert!(is_valid_date_format("2025-01-15"));
    assert!(is_valid_date_format("2025-12-31"));
    assert!(!is_valid_date_format("2025-1"));
    assert!(!is_valid_date_format("2025-1-1"));
    assert!(!is_valid_date_format("25-01-01"));
    assert!(!is_valid_date_format("not-a-date"));
}

#[test]
fn test_parse_table_with_arrays() {
    let yaml = r"
    month: ['Jan', 'Feb', 'Mar']
    revenue: [100, 200, 300]
    ";
    let parsed: Value = serde_yaml::from_str(yaml).unwrap();

    if let Value::Mapping(map) = parsed {
        let table = parse_table("test_table", &map).unwrap();

        assert_eq!(table.name, "test_table");
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns.contains_key("month"));
        assert!(table.columns.contains_key("revenue"));
        assert_eq!(table.row_count(), 3);
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_parse_table_with_formula() {
    let yaml = r"
    revenue: [100, 200, 300]
    expenses: [50, 100, 150]
    profit: '=revenue - expenses'
    ";
    let parsed: Value = serde_yaml::from_str(yaml).unwrap();

    if let Value::Mapping(map) = parsed {
        let table = parse_table("test_table", &map).unwrap();

        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.row_formulas.len(), 1);
        assert!(table.row_formulas.contains_key("profit"));
        assert_eq!(
            table.row_formulas.get("profit").unwrap(),
            "=revenue - expenses"
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_table_validate_lengths_ok() {
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "col1".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "col2".to_string(),
        ColumnValue::Number(vec![4.0, 5.0, 6.0]),
    ));

    assert!(table.validate_lengths().is_ok());
}

#[test]
fn test_table_validate_lengths_error() {
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "col1".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "col2".to_string(),
        ColumnValue::Number(vec![4.0, 5.0]),
    ));

    let result = table.validate_lengths();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("col1") || err_msg.contains("col2"));
    assert!(err_msg.contains("2 rows"));
    assert!(err_msg.contains("3 rows"));
}

#[test]
fn test_parse_v1_model_simple() {
    let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.2"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    assert!(result.tables.contains_key("sales"));

    let sales_table = result.tables.get("sales").unwrap();
    assert_eq!(sales_table.columns.len(), 2);
    assert_eq!(sales_table.row_formulas.len(), 1);
}

#[test]
fn test_parse_v1_model_with_scalars() {
    let yaml_content = r#"
_forge_version: "5.0.0"

data:
  values: [1, 2, 3]

summary:
  total:
    value: null
    formula: "=SUM(data.values)"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.scalars.len(), 1);
    assert!(result.scalars.contains_key("summary.total"));
}

#[test]
fn test_column_value_type_name() {
    let num_col = ColumnValue::Number(vec![1.0]);
    let text_col = ColumnValue::Text(vec!["A".to_string()]);
    let date_col = ColumnValue::Date(vec!["2025-01".to_string()]);
    let bool_col = ColumnValue::Boolean(vec![true]);

    assert_eq!(num_col.type_name(), "Number");
    assert_eq!(text_col.type_name(), "Text");
    assert_eq!(date_col.type_name(), "Date");
    assert_eq!(bool_col.type_name(), "Boolean");
}

#[test]
fn test_column_value_len() {
    let col = ColumnValue::Number(vec![1.0, 2.0, 3.0]);
    assert_eq!(col.len(), 3);
    assert!(!col.is_empty());

    let empty_col = ColumnValue::Number(vec![]);
    assert_eq!(empty_col.len(), 0);
    assert!(empty_col.is_empty());
}

#[test]
fn test_parse_scenarios() {
    let yaml_content = r#"
_forge_version: "1.0.0"

growth_rate:
  value: 0.05
  formula: null

scenarios:
  base:
    growth_rate: 0.05
  optimistic:
    growth_rate: 0.12
  pessimistic:
    growth_rate: 0.02
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.scenarios.len(), 3);
    assert!(result.scenarios.contains_key("base"));
    assert!(result.scenarios.contains_key("optimistic"));
    assert!(result.scenarios.contains_key("pessimistic"));

    let base = result.scenarios.get("base").unwrap();
    assert_eq!(base.overrides.get("growth_rate"), Some(&0.05));

    let optimistic = result.scenarios.get("optimistic").unwrap();
    assert_eq!(optimistic.overrides.get("growth_rate"), Some(&0.12));

    let pessimistic = result.scenarios.get("pessimistic").unwrap();
    assert_eq!(pessimistic.overrides.get("growth_rate"), Some(&0.02));
}

#[test]
fn test_multi_document_yaml_with_leading_separator() {
    let yaml_content = r#"---
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    let sales = result.tables.get("sales").unwrap();
    assert_eq!(sales.row_count(), 3);
}

#[test]
fn test_null_in_numeric_array_error() {
    let yaml_content = r#"
_forge_version: "5.0.0"
data:
  values: [1000, null, 2000]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());

    let err = result.unwrap_err().to_string();
    assert!(err.contains("null values not allowed"));
    assert!(err.contains("Use 0 or remove the row"));
}

#[test]
fn test_null_first_element_error() {
    let yaml_content = r#"
_forge_version: "5.0.0"
data:
  values: [null, 1000, 2000]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());

    let err = result.unwrap_err().to_string();
    assert!(err.contains("cannot start with null"));
}

#[test]
fn test_table_named_scenarios() {
    let yaml_content = r#"
_forge_version: "5.0.0"

scenarios:
  name: ["Base", "Optimistic", "Pessimistic"]
  probability: [0.3, 0.5, 0.2]
  revenue: [100000, 150000, 80000]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.scenarios.len(), 0);
    assert_eq!(result.tables.len(), 1);

    let scenarios_table = result.tables.get("scenarios").unwrap();
    assert_eq!(scenarios_table.columns.len(), 3);
    assert!(scenarios_table.columns.contains_key("name"));
    assert!(scenarios_table.columns.contains_key("probability"));
    assert!(scenarios_table.columns.contains_key("revenue"));
    assert_eq!(scenarios_table.row_count(), 3);
}

// =========================================================================
// v4.0 Rich Metadata Tests
// =========================================================================

#[test]
fn test_parse_v4_scalar_with_metadata() {
    let yaml_content = r#"
_forge_version: "5.0.0"

price:
  value: 100
  formula: null
  unit: "CAD"
  notes: "Base price per unit"
  source: "market_research.yaml"
  validation_status: "VALIDATED"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.scalars.len(), 1);
    let price = result.scalars.get("price").unwrap();
    assert_eq!(price.value, Some(100.0));
    assert!(price.formula.is_none());
    assert_eq!(price.metadata.unit, Some("CAD".to_string()));
    assert_eq!(
        price.metadata.notes,
        Some("Base price per unit".to_string())
    );
    assert_eq!(
        price.metadata.source,
        Some("market_research.yaml".to_string())
    );
    assert_eq!(
        price.metadata.validation_status,
        Some("VALIDATED".to_string())
    );
}

#[test]
fn test_parse_v4_table_column_with_metadata() {
    let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month:
    value: ["Jan", "Feb", "Mar"]
    unit: "month"
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Monthly revenue projection"
    validation_status: "PROJECTED"
  profit:
    formula: "=revenue * 0.3"
    unit: "CAD"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    let sales = result.tables.get("sales").unwrap();

    let month = sales.columns.get("month").unwrap();
    assert_eq!(month.metadata.unit, Some("month".to_string()));

    let revenue = sales.columns.get("revenue").unwrap();
    assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
    assert_eq!(
        revenue.metadata.notes,
        Some("Monthly revenue projection".to_string())
    );
    assert_eq!(
        revenue.metadata.validation_status,
        Some("PROJECTED".to_string())
    );

    assert!(sales.row_formulas.contains_key("profit"));
    assert_eq!(sales.row_formulas.get("profit").unwrap(), "=revenue * 0.3");
}

#[test]
fn test_parse_v4_backward_compatible_with_v1() {
    let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.3"

summary:
  total:
    value: null
    formula: "=SUM(sales.revenue)"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    let sales = result.tables.get("sales").unwrap();
    assert_eq!(sales.columns.len(), 2);
    assert_eq!(sales.row_formulas.len(), 1);

    assert_eq!(result.scalars.len(), 1);
    let total = result.scalars.get("summary.total").unwrap();
    assert_eq!(total.formula, Some("=SUM(sales.revenue)".to_string()));

    assert!(sales.columns.get("revenue").unwrap().metadata.is_empty());
    assert!(total.metadata.is_empty());
}

#[test]
fn test_parse_v4_mixed_formats() {
    let yaml_content = r#"
_forge_version: "5.0.0"
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Rich format column"
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    let sales = result.tables.get("sales").unwrap();

    assert!(sales.columns.get("month").unwrap().metadata.is_empty());
    assert!(sales.columns.get("expenses").unwrap().metadata.is_empty());

    let revenue = sales.columns.get("revenue").unwrap();
    assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
    assert_eq!(
        revenue.metadata.notes,
        Some("Rich format column".to_string())
    );
}

// =========================================================================
// v5.0.0 Inputs/Outputs Tests
// =========================================================================

#[test]
fn test_parse_v5_inputs_outputs_sections() {
    let yaml_content = r#"
_forge_version: "5.0.0"

inputs:
  tax_rate:
    value: 0.25
    formula: null
    unit: "%"
    notes: "Corporate tax rate"
  discount_rate:
    value: 0.10
    formula: null
    unit: "%"

outputs:
  net_profit:
    value: 75000
    formula: "=revenue * (1 - tax_rate)"
    unit: "CAD"
  npv:
    value: null
    formula: "=NPV(discount_rate, cashflows)"

data:
  quarter: ["Q1", "Q2", "Q3", "Q4"]
  revenue: [100000, 120000, 150000, 180000]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert_eq!(result.tables.len(), 1);
    assert!(result.tables.contains_key("data"));

    assert_eq!(result.scalars.len(), 4);

    let tax_rate = result.scalars.get("inputs.tax_rate").unwrap();
    assert_eq!(tax_rate.value, Some(0.25));
    assert!(tax_rate.formula.is_none());
    assert_eq!(tax_rate.metadata.unit, Some("%".to_string()));
    assert_eq!(
        tax_rate.metadata.notes,
        Some("Corporate tax rate".to_string())
    );

    let discount_rate = result.scalars.get("inputs.discount_rate").unwrap();
    assert_eq!(discount_rate.value, Some(0.10));

    let net_profit = result.scalars.get("outputs.net_profit").unwrap();
    assert_eq!(net_profit.value, Some(75000.0));
    assert_eq!(
        net_profit.formula,
        Some("=revenue * (1 - tax_rate)".to_string())
    );
    assert_eq!(net_profit.metadata.unit, Some("CAD".to_string()));

    let npv = result.scalars.get("outputs.npv").unwrap();
    assert!(npv.value.is_none());
    assert_eq!(
        npv.formula,
        Some("=NPV(discount_rate, cashflows)".to_string())
    );
}

// =========================================================================
// Multi-Document YAML Tests (v4.4.2)
// =========================================================================

#[test]
fn test_detect_multi_document_true() {
    let content = "---\nfirst: 1\n---\nsecond: 2\n";
    assert!(detect_multi_document(content));
}

#[test]
fn test_detect_multi_document_false_single_separator() {
    let content = "---\nfirst: 1\n";
    assert!(!detect_multi_document(content));
}

#[test]
fn test_detect_multi_document_false_no_separator() {
    let content = "first: 1\nsecond: 2\n";
    assert!(!detect_multi_document(content));
}

#[test]
fn test_detect_multi_document_with_trailing_content() {
    let content = "--- first doc\nfirst: 1\n--- second\nsecond: 2\n";
    assert!(detect_multi_document(content));
}

#[test]
fn test_split_yaml_documents() {
    let content = "---\nfirst: 1\n---\nsecond: 2\n";
    let docs = split_yaml_documents(content);
    assert_eq!(docs.len(), 2);
    assert!(docs[0].contains("first: 1"));
    assert!(docs[1].contains("second: 2"));
}

#[test]
fn test_split_yaml_documents_empty() {
    let content = "";
    let docs = split_yaml_documents(content);
    assert!(docs.is_empty());
}

#[test]
fn test_split_yaml_documents_single() {
    let content = "---\nfirst: 1\n";
    let docs = split_yaml_documents(content);
    assert_eq!(docs.len(), 1);
}

#[test]
fn test_parse_multi_doc_with_names() {
    let yaml_content = r#"---
_forge_version: "5.0.0"
_name: "revenue"
data:
  values: [100, 200, 300]
---
_forge_version: "5.0.0"
_name: "costs"
expenses:
  amounts: [50, 100, 150]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert!(result.tables.contains_key("revenue.data"));
    assert!(result.tables.contains_key("costs.expenses"));
    assert_eq!(result.documents.len(), 2);
    assert!(result.documents.contains(&"revenue".to_string()));
    assert!(result.documents.contains(&"costs".to_string()));
}

#[test]
fn test_parse_multi_doc_auto_names() {
    let yaml_content = r#"---
_forge_version: "5.0.0"
data1:
  values: [1, 2, 3]
---
_forge_version: "5.0.0"
data2:
  values: [4, 5, 6]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert!(result.tables.contains_key("doc1.data1"));
    assert!(result.tables.contains_key("doc2.data2"));
}

#[test]
fn test_parse_multi_doc_with_scalars() {
    let yaml_content = r#"---
_forge_version: "5.0.0"
_name: "config"
rate:
  value: 0.05
  formula: null
---
_forge_version: "5.0.0"
_name: "data"
values:
  items: [1, 2, 3]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();

    assert!(result.scalars.contains_key("config.rate"));
    assert!(result.tables.contains_key("data.values"));
}

// =========================================================================
// Include Tests (v4.0)
// =========================================================================

#[test]
fn test_parse_includes_section() {
    let temp_dir = TempDir::new().unwrap();

    let included_path = temp_dir.path().join("external.yaml");
    std::fs::write(
        &included_path,
        r#"
_forge_version: "5.0.0"
ext_data:
  values: [10, 20, 30]
"#,
    )
    .unwrap();

    let main_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "external.yaml"
    as: "ext"
main_data:
  values: [1, 2, 3]
"#
    .to_string();

    let main_path = temp_dir.path().join("main.yaml");
    std::fs::write(&main_path, main_content).unwrap();

    let result = parse_model(&main_path).unwrap();

    assert!(result.tables.contains_key("main_data"));
    assert!(result.resolved_includes.contains_key("ext"));
}

#[test]
fn test_parse_includes_missing_file() {
    let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "nonexistent.yaml"
    as: "ext"
data:
  values: [1, 2, 3]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("nonexistent"));
}

#[test]
fn test_parse_includes_missing_as_field() {
    let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "external.yaml"
data:
  values: [1, 2, 3]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_parse_includes_invalid_format() {
    let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - "just a string, not a mapping"
data:
  values: [1, 2, 3]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

// =========================================================================
// Scenario Parsing Tests
// =========================================================================

#[test]
fn test_parse_scenario_invalid_value_type() {
    let yaml_content = r#"
_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
scenarios:
  base:
    rate: "not a number"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_parse_scenario_not_mapping() {
    let yaml_content = r#"
_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
scenarios:
  base: "not a mapping"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

// =========================================================================
// Column Parsing Edge Cases
// =========================================================================

#[test]
fn test_parse_table_column_not_array() {
    let yaml_content = r#"
_forge_version: "1.0.0"
data:
  values: 123
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_parse_date_format_yyyy_mm_dd() {
    let yaml_seq: Vec<Value> = vec![
        Value::String("2025-01-15".to_string()),
        Value::String("2025-02-20".to_string()),
    ];
    let result = parse_array_value("test_col", &yaml_seq).unwrap();

    match result {
        ColumnValue::Date(dates) => {
            assert_eq!(dates, vec!["2025-01-15", "2025-02-20"]);
        }
        _ => panic!("Expected Date array"),
    }
}

#[test]
fn test_type_name_function() {
    assert_eq!(type_name(&Value::Null), "Null");
    assert_eq!(type_name(&Value::Bool(true)), "Boolean");
    assert_eq!(type_name(&Value::Number(1.into())), "Number");
    assert_eq!(type_name(&Value::String("test".to_string())), "String");
    assert_eq!(type_name(&Value::Sequence(vec![])), "Array");
    assert_eq!(
        type_name(&Value::Mapping(serde_yaml::Mapping::new())),
        "Mapping"
    );
}

#[test]
fn test_detect_array_type_unsupported() {
    let val = Value::Sequence(vec![]);
    let result = detect_array_type(&val);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported"));
}

#[test]
fn test_boolean_array_wrong_type() {
    let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::String("not bool".to_string())];
    let result = parse_array_value("test_col", &yaml_seq);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Expected Boolean"));
}

#[test]
fn test_date_array_wrong_type() {
    let yaml_seq: Vec<Value> = vec![
        Value::String("2025-01".to_string()),
        Value::Number(123.into()),
    ];
    let result = parse_array_value("test_col", &yaml_seq);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Expected Date"));
}

#[test]
fn test_text_array_wrong_type() {
    let yaml_seq: Vec<Value> = vec![Value::String("text".to_string()), Value::Bool(true)];
    let result = parse_array_value("test_col", &yaml_seq);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Expected Text"));
}

#[test]
fn test_metadata_last_updated() {
    let mut map = serde_yaml::Mapping::new();
    map.insert(
        Value::String("last_updated".to_string()),
        Value::String("2025-01-01".to_string()),
    );
    let metadata = parse_metadata(&map);
    assert_eq!(metadata.last_updated, Some("2025-01-01".to_string()));
}

#[test]
fn test_is_nested_scalar_section_false_for_v4_rich_column() {
    let mut map = serde_yaml::Mapping::new();
    let mut child = serde_yaml::Mapping::new();
    child.insert(
        Value::String("value".to_string()),
        Value::Sequence(vec![Value::Number(1.into())]),
    );
    map.insert(Value::String("col".to_string()), Value::Mapping(child));
    assert!(!is_nested_scalar_section(&map));
}

#[test]
fn test_is_nested_scalar_section_true_for_scalar() {
    let mut map = serde_yaml::Mapping::new();
    let mut child = serde_yaml::Mapping::new();
    child.insert(Value::String("value".to_string()), Value::Number(42.into()));
    map.insert(Value::String("total".to_string()), Value::Mapping(child));
    assert!(is_nested_scalar_section(&map));
}

#[test]
fn test_parse_scalar_variable_not_mapping() {
    let val = Value::String("not a mapping".to_string());
    let result = parse_scalar_variable(&val, "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Expected mapping"));
}

#[test]
fn test_parse_file_not_found() {
    let result = parse_model(Path::new("/nonexistent/path/file.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_yaml() {
    let yaml_content = "not: valid: yaml: [[[";

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_parse_multi_doc_skip_comments() {
    let yaml_content = r#"---
# This is a comment-only document
# No actual content
---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();
    assert!(!result.tables.is_empty());
}

#[test]
fn test_parse_multi_doc_with_empty_doc() {
    let yaml_content = r#"---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
---

---
_forge_version: "5.0.0"
data2:
  values: [4, 5, 6]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();
    assert_eq!(result.tables.len(), 2);
}

#[test]
fn test_parse_multi_doc_invalid_yaml_error() {
    let yaml_content = r#"---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
---
invalid: yaml: [[[
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse document"));
}

#[test]
fn test_parse_multi_doc_with_scenarios() {
    let yaml_content = r#"---
_name: doc1
_forge_version: "5.0.0"
budget:
  revenue: [1000, 2000]
scenarios:
  optimistic:
    growth: 1.2
---
_name: doc2
_forge_version: "5.0.0"
budget:
  costs: [500, 600]
scenarios:
  pessimistic:
    growth: 0.8
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();

    let result = parse_model(temp_file.path()).unwrap();
    assert!(result.scenarios.contains_key("doc1.optimistic"));
    assert!(result.scenarios.contains_key("doc2.pessimistic"));
}

#[test]
fn test_parse_table_named_scenarios_as_table() {
    let yaml_str = r#"
_forge_version: "5.0.0"
scenarios:
  year: [2023, 2024, 2025]
  revenue: [1000, 2000, 3000]
"#;
    let yaml: Value = serde_yaml::from_str(yaml_str).unwrap();
    let result = parse_v1_model(&yaml).unwrap();
    assert!(result.tables.contains_key("scenarios"));
    assert!(result.scenarios.is_empty());
}

#[test]
fn test_is_nested_scalar_section_empty_child() {
    let mut map = serde_yaml::Mapping::new();
    map.insert(
        Value::String("empty".to_string()),
        Value::Mapping(serde_yaml::Mapping::new()),
    );
    assert!(!is_nested_scalar_section(&map));
}

#[test]
fn test_parse_table_column_scalar_not_array() {
    let mut map = serde_yaml::Mapping::new();
    map.insert(Value::String("col".to_string()), Value::Number(42.into()));
    let result = parse_table("test", &map);
    assert!(result.is_err());
}
