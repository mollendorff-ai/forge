//! Audit tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_build_dependency_tree_no_formula() {
    let model = crate::types::ParsedModel::new();
    let deps = build_dependency_tree(&model, "test", &None, 0).unwrap();
    assert!(deps.is_empty());
}

#[test]
fn test_build_dependency_tree_simple() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "a".to_string(),
        crate::types::Variable::new("a".to_string(), Some(10.0), None),
    );

    let formula = Some("=a + 5".to_string());
    let deps = build_dependency_tree(&model, "result", &formula, 0).unwrap();

    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "a");
    assert_eq!(deps[0].dep_type, "Scalar");
}

#[test]
fn test_build_dependency_tree_max_depth() {
    let model = crate::types::ParsedModel::new();
    let formula = Some("=x".to_string());
    // Should return empty at depth > 20
    let deps = build_dependency_tree(&model, "test", &formula, 21).unwrap();
    assert!(deps.is_empty());
}

#[test]
fn test_audit_dependency_struct() {
    let dep = AuditDependency {
        name: "revenue".to_string(),
        dep_type: "Scalar".to_string(),
        formula: Some("=price * qty".to_string()),
        value: Some(1000.0),
        children: vec![],
    };

    assert_eq!(dep.name, "revenue");
    assert_eq!(dep.dep_type, "Scalar");
    assert_eq!(dep.formula, Some("=price * qty".to_string()));
    assert_eq!(dep.value, Some(1000.0));
}

#[test]
fn test_print_dependency_basic() {
    let dep = AuditDependency {
        name: "test".to_string(),
        dep_type: "Scalar".to_string(),
        formula: None,
        value: Some(100.0),
        children: vec![],
    };
    // Just verify it doesn't panic
    print_dependency(&dep, 0);
    print_dependency(&dep, 1);
    print_dependency(&dep, 5);
}

#[test]
fn test_print_dependency_with_children() {
    let child = AuditDependency {
        name: "child".to_string(),
        dep_type: "Scalar".to_string(),
        formula: None,
        value: Some(50.0),
        children: vec![],
    };
    let parent = AuditDependency {
        name: "parent".to_string(),
        dep_type: "Aggregation".to_string(),
        formula: Some("=SUM(child)".to_string()),
        value: None,
        children: vec![child],
    };
    // Just verify it doesn't panic
    print_dependency(&parent, 0);
}

#[test]
fn test_audit_scalar() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
  quantity:
    value: 5
    formula: null
  total:
    value: 500
    formula: "=summary.price * summary.quantity"
"#,
    );

    let result = audit(yaml, "summary.total".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_variable_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = audit(yaml, "nonexistent".to_string());
    assert!(result.is_err());
}

#[test]
fn test_audit_aggregation() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_agg.yaml",
        r#"_forge_version: "1.0.0"
sales:
  revenue: [100, 200, 300]
total_revenue:
  value: null
  formula: "=SUM(sales.revenue)"
"#,
    );

    let result = audit(yaml, "total_revenue".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_table_column() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_col.yaml",
        r#"_forge_version: "1.0.0"
orders:
  price: [10, 20, 30]
  quantity: [2, 3, 4]
  total:
    formula: "=orders.price * orders.quantity"
"#,
    );

    // Table column uses tablename.columnname format
    let result = audit(yaml, "orders.total".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_file_not_found() {
    let result = audit(PathBuf::from("/nonexistent.yaml"), "x".to_string());
    assert!(result.is_err());
}

#[test]
fn test_audit_value_mismatch() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_mismatch.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: 999
    formula: "=summary.x * 2"
"#,
    );

    // Should complete but show mismatch
    let result = audit(yaml, "summary.y".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_literal_no_deps() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_literal.yaml",
        r#"_forge_version: "1.0.0"
constant:
  value: 42
  formula: "=42"
"#,
    );

    // Formula is a literal, no dependencies
    let result = audit(yaml, "constant".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_aggregation_with_deps() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_agg_deps.yaml",
        r#"_forge_version: "1.0.0"
data:
  values: [10, 20, 30]
subtotal:
  value: null
  formula: "=SUM(data.values)"
tax_rate:
  value: 0.1
  formula: null
total:
  value: null
  formula: "=subtotal * (1 + tax_rate)"
"#,
    );

    // Tests aggregation dependency tree path
    let result = audit(yaml, "total".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_audit_table_column_audit() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "audit_tbl_col.yaml",
        r#"_forge_version: "1.0.0"
items:
  qty: [1, 2, 3]
  price: [10, 20, 30]
  total:
    formula: "=items.qty * items.price"
"#,
    );

    // Audit a table column with formula
    let result = audit(yaml, "items.total".to_string());
    assert!(result.is_ok());
}
