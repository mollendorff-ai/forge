//! Scenario tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_find_variable_scalar() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "revenue".to_string(),
        crate::types::Variable::new("revenue".to_string(), Some(1000.0), None),
    );

    let (var_type, formula, value) = find_variable(&model, "revenue").unwrap();
    assert_eq!(var_type, "Scalar");
    assert!(formula.is_none());
    assert_eq!(value, Some(1000.0));
}

#[test]
fn test_find_variable_scalar_with_formula() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "profit".to_string(),
        crate::types::Variable::new(
            "profit".to_string(),
            Some(500.0),
            Some("=revenue - costs".to_string()),
        ),
    );

    let (var_type, formula, value) = find_variable(&model, "profit").unwrap();
    assert_eq!(var_type, "Scalar");
    assert_eq!(formula, Some("=revenue - costs".to_string()));
    assert_eq!(value, Some(500.0));
}

#[test]
fn test_find_variable_aggregation() {
    let mut model = crate::types::ParsedModel::new();
    model
        .aggregations
        .insert("total_sales".to_string(), "=SUM(sales.amount)".to_string());

    let (var_type, formula, value) = find_variable(&model, "total_sales").unwrap();
    assert_eq!(var_type, "Aggregation");
    assert_eq!(formula, Some("=SUM(sales.amount)".to_string()));
    assert!(value.is_none());
}

#[test]
fn test_find_variable_table_column() {
    let mut model = crate::types::ParsedModel::new();
    let mut table = crate::types::Table::new("sales".to_string());
    table.columns.insert(
        "amount".to_string(),
        crate::types::Column::new(
            "amount".to_string(),
            crate::types::ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ),
    );
    model.tables.insert("sales".to_string(), table);

    let (var_type, formula, value) = find_variable(&model, "amount").unwrap();
    assert!(var_type.contains("Column"));
    assert!(var_type.contains("sales"));
    assert!(formula.is_none()); // Column without formula
    assert!(value.is_none());
}

#[test]
fn test_find_variable_table_column_with_formula() {
    let mut model = crate::types::ParsedModel::new();
    let mut table = crate::types::Table::new("orders".to_string());
    table.columns.insert(
        "total".to_string(),
        crate::types::Column::new(
            "total".to_string(),
            crate::types::ColumnValue::Number(vec![110.0, 220.0, 330.0]),
        ),
    );
    table
        .row_formulas
        .insert("total".to_string(), "=price * quantity".to_string());
    model.tables.insert("orders".to_string(), table);

    let (var_type, formula, _value) = find_variable(&model, "total").unwrap();
    assert!(var_type.contains("orders"));
    assert_eq!(formula, Some("=price * quantity".to_string()));
}

#[test]
fn test_find_variable_not_found() {
    let model = crate::types::ParsedModel::new();
    let result = find_variable(&model, "nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"));
}

#[test]
fn test_apply_scenario_overrides_existing() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        crate::types::Variable::new(
            "rate".to_string(),
            Some(0.05),
            Some("=base_rate".to_string()),
        ),
    );

    let mut scenario = crate::types::Scenario::new();
    scenario.overrides.insert("rate".to_string(), 0.10);
    model.scenarios.insert("high_rate".to_string(), scenario);

    apply_scenario(&mut model, "high_rate").unwrap();

    let rate = model.scalars.get("rate").unwrap();
    assert_eq!(rate.value, Some(0.10));
    assert!(rate.formula.is_none()); // Formula cleared
}

#[test]
fn test_apply_scenario_creates_new_scalar() {
    let mut model = crate::types::ParsedModel::new();

    let mut scenario = crate::types::Scenario::new();
    scenario.overrides.insert("new_var".to_string(), 42.0);
    model.scenarios.insert("test".to_string(), scenario);

    apply_scenario(&mut model, "test").unwrap();

    assert!(model.scalars.contains_key("new_var"));
    assert_eq!(model.scalars.get("new_var").unwrap().value, Some(42.0));
}

#[test]
fn test_apply_scenario_not_found() {
    let model = crate::types::ParsedModel::new();
    let mut model = model;
    let result = apply_scenario(&mut model, "nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"));
}
