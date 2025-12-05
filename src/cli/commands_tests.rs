#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::*;
use std::io::Write;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_format_number_integer() {
    assert_eq!(format_number(100.0), "100");
    assert_eq!(format_number(0.0), "0");
    assert_eq!(format_number(-50.0), "-50");
}

#[test]
fn test_format_number_decimal() {
    assert_eq!(format_number(3.14), "3.14");
    assert_eq!(format_number(0.5), "0.5");
    assert_eq!(format_number(-2.75), "-2.75");
}

#[test]
fn test_format_number_removes_trailing_zeros() {
    assert_eq!(format_number(1.10), "1.1");
    assert_eq!(format_number(2.500), "2.5");
    assert_eq!(format_number(10.000), "10");
}

#[test]
fn test_format_number_precision() {
    // Rounds to 6 decimal places
    assert_eq!(format_number(0.123456789), "0.123457");
    assert_eq!(format_number(1.0000001), "1");
}

#[test]
fn test_format_number_very_small() {
    assert_eq!(format_number(0.000001), "0.000001");
    assert_eq!(format_number(0.0000001), "0");
}

#[test]
fn test_format_number_large() {
    assert_eq!(format_number(1000000.0), "1000000");
    // 999999.999999 stays as-is since it's within 6 decimal precision
    assert_eq!(format_number(999999.999999), "999999.999999");
    // Very small differences beyond 6 decimals get rounded
    assert_eq!(format_number(1000000.0000001), "1000000");
}

// =========================================================================
// chrono_lite_timestamp Tests
// =========================================================================

#[test]
fn test_chrono_lite_timestamp_format() {
    let ts = chrono_lite_timestamp();
    // Format should be "HH:MM:SS UTC" (12 chars)
    assert_eq!(ts.len(), 12);
    assert!(ts.contains(':'));
    assert!(ts.ends_with(" UTC"));
}

#[test]
fn test_chrono_lite_timestamp_valid_time() {
    let ts = chrono_lite_timestamp();
    // Parse the HH:MM:SS part
    let time_part = &ts[..8];
    let parts: Vec<&str> = time_part.split(':').collect();
    assert_eq!(parts.len(), 3);

    let hours: u32 = parts[0].parse().unwrap();
    let minutes: u32 = parts[1].parse().unwrap();
    let seconds: u32 = parts[2].parse().unwrap();

    assert!(hours < 24);
    assert!(minutes < 60);
    assert!(seconds < 60);
}

// =========================================================================
// parse_range Tests
// =========================================================================

#[test]
fn test_parse_range_basic() {
    // Format is "start,end,step"
    let result = parse_range("0,10,2").unwrap();
    assert_eq!(result, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
}

#[test]
fn test_parse_range_decimal_step() {
    let result = parse_range("0,1,0.25").unwrap();
    assert_eq!(result.len(), 5);
    assert!((result[0] - 0.0).abs() < 0.0001);
    assert!((result[1] - 0.25).abs() < 0.0001);
    assert!((result[4] - 1.0).abs() < 0.0001);
}

#[test]
fn test_parse_range_negative_values() {
    let result = parse_range("-5,-1,1").unwrap();
    assert_eq!(result, vec![-5.0, -4.0, -3.0, -2.0, -1.0]);
}

#[test]
fn test_parse_range_financial() {
    // Typical rate sensitivity: 1% to 15% in 2% steps
    let result = parse_range("0.01,0.15,0.02").unwrap();
    assert_eq!(result.len(), 8); // 0.01, 0.03, 0.05, 0.07, 0.09, 0.11, 0.13, 0.15
}

#[test]
fn test_parse_range_invalid_format_too_few_parts() {
    let result = parse_range("1,5");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid range format"));
}

#[test]
fn test_parse_range_invalid_format_too_many_parts() {
    let result = parse_range("1,2,3,4");
    assert!(result.is_err());
}

#[test]
fn test_parse_range_invalid_start_value() {
    let result = parse_range("abc,10,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid start value"));
}

#[test]
fn test_parse_range_invalid_end_value() {
    let result = parse_range("0,xyz,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid end value"));
}

#[test]
fn test_parse_range_invalid_step_value() {
    let result = parse_range("0,10,bad");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid step value"));
}

#[test]
fn test_parse_range_zero_step() {
    let result = parse_range("0,10,0");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Step must be positive"));
}

#[test]
fn test_parse_range_negative_step() {
    let result = parse_range("0,10,-1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Step must be positive"));
}

#[test]
fn test_parse_range_start_greater_than_end() {
    let result = parse_range("10,0,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Start must be less than or equal to end"));
}

#[test]
fn test_parse_range_start_equals_end() {
    let result = parse_range("5,5,1").unwrap();
    assert_eq!(result, vec![5.0]);
}

#[test]
fn test_parse_range_with_spaces() {
    let result = parse_range(" 0 , 10 , 2 ").unwrap();
    assert_eq!(result, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
}

// =========================================================================
// extract_references_from_formula Tests
// =========================================================================

#[test]
fn test_extract_references_simple() {
    let refs = extract_references_from_formula("=a + b");
    assert!(refs.contains(&"a".to_string()));
    assert!(refs.contains(&"b".to_string()));
}

#[test]
fn test_extract_references_with_underscores() {
    let refs = extract_references_from_formula("=total_revenue - total_costs");
    assert!(refs.contains(&"total_revenue".to_string()));
    assert!(refs.contains(&"total_costs".to_string()));
}

#[test]
fn test_extract_references_filters_functions() {
    let refs = extract_references_from_formula("=SUM(revenue) + MAX(costs)");
    assert!(refs.contains(&"revenue".to_string()));
    assert!(refs.contains(&"costs".to_string()));
    assert!(!refs.contains(&"SUM".to_string()));
    assert!(!refs.contains(&"MAX".to_string()));
}

#[test]
fn test_extract_references_filters_all_known_functions() {
    let refs = extract_references_from_formula("=IF(AND(x, OR(y, z)), AVERAGE(data), MIN(values))");
    assert!(refs.contains(&"x".to_string()));
    assert!(refs.contains(&"y".to_string()));
    assert!(refs.contains(&"z".to_string()));
    assert!(refs.contains(&"data".to_string()));
    assert!(refs.contains(&"values".to_string()));
    assert!(!refs.contains(&"IF".to_string()));
    assert!(!refs.contains(&"AND".to_string()));
    assert!(!refs.contains(&"OR".to_string()));
    assert!(!refs.contains(&"AVERAGE".to_string()));
    assert!(!refs.contains(&"MIN".to_string()));
}

#[test]
fn test_extract_references_filters_numbers() {
    let refs = extract_references_from_formula("=revenue * 0.15 + 100");
    assert!(refs.contains(&"revenue".to_string()));
    assert!(!refs.iter().any(|r| r.starts_with('0')));
    assert!(!refs.iter().any(|r| r.starts_with('1')));
}

#[test]
fn test_extract_references_empty_formula() {
    let refs = extract_references_from_formula("");
    assert!(refs.is_empty());
}

#[test]
fn test_extract_references_literal_only() {
    let refs = extract_references_from_formula("=100");
    assert!(refs.is_empty());
}

#[test]
fn test_extract_references_no_duplicates() {
    let refs = extract_references_from_formula("=a + a + a");
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&"a".to_string()));
}

#[test]
fn test_extract_references_strips_equals() {
    let refs = extract_references_from_formula("=price");
    assert!(refs.contains(&"price".to_string()));
    assert!(!refs.contains(&"=price".to_string()));
}

#[test]
fn test_extract_references_complex_formula() {
    let refs = extract_references_from_formula(
        "=SUMIF(categories, expenses) + IFERROR(overhead / months, 0)",
    );
    assert!(refs.contains(&"categories".to_string()));
    assert!(refs.contains(&"expenses".to_string()));
    assert!(refs.contains(&"overhead".to_string()));
    assert!(refs.contains(&"months".to_string()));
    // Numbers in formulas should not be extracted
    assert!(!refs.iter().any(|r| r == "0"));
}

#[test]
fn test_extract_references_case_insensitive_functions() {
    // Functions should be filtered regardless of case
    let refs = extract_references_from_formula("=sum(data) + Sum(more) + SUM(again)");
    assert!(refs.contains(&"data".to_string()));
    assert!(refs.contains(&"more".to_string()));
    assert!(refs.contains(&"again".to_string()));
    assert!(!refs.iter().any(|r| r.to_uppercase() == "SUM"));
}

// =========================================================================
// find_variable Tests
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

// =========================================================================
// apply_scenario Tests
// =========================================================================

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

// =========================================================================
// build_dependency_tree Tests
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

// =========================================================================
// AuditDependency / print_dependency Tests
// =========================================================================

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

// =========================================================================
// VarianceResult Tests
// =========================================================================

#[test]
fn test_variance_result_struct() {
    let vr = VarianceResult {
        name: "revenue".to_string(),
        budget: 1000.0,
        actual: 1200.0,
        variance: 200.0,
        variance_pct: 20.0,
        is_favorable: true,
        exceeds_threshold: true,
    };

    assert_eq!(vr.name, "revenue");
    assert_eq!(vr.budget, 1000.0);
    assert_eq!(vr.actual, 1200.0);
    assert_eq!(vr.variance, 200.0);
    assert_eq!(vr.variance_pct, 20.0);
    assert!(vr.is_favorable);
    assert!(vr.exceeds_threshold);
}

#[test]
fn test_variance_result_clone() {
    let vr = VarianceResult {
        name: "cost".to_string(),
        budget: 500.0,
        actual: 600.0,
        variance: 100.0,
        variance_pct: 20.0,
        is_favorable: false,
        exceeds_threshold: false,
    };
    let cloned = vr.clone();
    assert_eq!(cloned.name, vr.name);
    assert_eq!(cloned.budget, vr.budget);
}

// =========================================================================
// print_variance_table Tests
// =========================================================================

#[test]
fn test_print_variance_table_empty() {
    let variances: Vec<VarianceResult> = vec![];
    // Just verify it doesn't panic
    print_variance_table(&variances, 10.0);
}

#[test]
fn test_print_variance_table_with_data() {
    let variances = vec![
        VarianceResult {
            name: "revenue".to_string(),
            budget: 1000.0,
            actual: 1100.0,
            variance: 100.0,
            variance_pct: 10.0,
            is_favorable: true,
            exceeds_threshold: false,
        },
        VarianceResult {
            name: "expense".to_string(),
            budget: 500.0,
            actual: 600.0,
            variance: 100.0,
            variance_pct: 20.0,
            is_favorable: false,
            exceeds_threshold: true,
        },
    ];
    // Just verify it doesn't panic
    print_variance_table(&variances, 15.0);
}

// =========================================================================
// FunctionCategory Tests
// =========================================================================

#[test]
fn test_function_category_struct() {
    let cat = FunctionCategory {
        name: "Financial",
        functions: vec![
            ("NPV", "Net Present Value"),
            ("IRR", "Internal Rate of Return"),
        ],
    };

    assert_eq!(cat.name, "Financial");
    assert_eq!(cat.functions.len(), 2);
    assert_eq!(cat.functions[0].0, "NPV");
}

// =========================================================================
// calculate_with_override Tests
// =========================================================================

#[test]
fn test_calculate_with_override_existing_scalar() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        crate::types::Variable::new("rate".to_string(), Some(0.05), None),
    );
    model.scalars.insert(
        "result".to_string(),
        crate::types::Variable::new("result".to_string(), None, Some("=rate * 100".to_string())),
    );

    let output = calculate_with_override(&model, "rate", 0.10, "result").unwrap();
    assert!((output - 10.0).abs() < 0.0001);
}

#[test]
fn test_calculate_with_override_new_scalar() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "result".to_string(),
        crate::types::Variable::new("result".to_string(), None, Some("=rate * 100".to_string())),
    );

    let output = calculate_with_override(&model, "rate", 0.15, "result").unwrap();
    assert!((output - 15.0).abs() < 0.0001);
}

#[test]
fn test_calculate_with_override_output_not_found() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        crate::types::Variable::new("rate".to_string(), Some(0.05), None),
    );

    let result = calculate_with_override(&model, "rate", 0.10, "nonexistent");
    assert!(result.is_err());
}

// =========================================================================
// Command Integration Tests (with temp files)
// =========================================================================

fn create_test_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn test_validate_single_file_empty_model() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "empty.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"empty\"\n",
    );

    // Empty model should pass validation with warning
    let result = validate_single_file(&yaml);
    assert!(result.is_ok());
}

#[test]
fn test_validate_single_file_valid_model() {
    let dir = TempDir::new().unwrap();
    // Use simple scalar format (like test.yaml) that works with v1.0.0
    let yaml = create_test_yaml(
        &dir,
        "valid.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
  result:
    value: 200
    formula: "=price * 2"
"#,
    );

    let result = validate_single_file(&yaml);
    assert!(result.is_ok());
}

#[test]
fn test_validate_batch() {
    let dir = TempDir::new().unwrap();
    let yaml1 = create_test_yaml(
        &dir,
        "file1.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"file1\"\n",
    );
    let yaml2 = create_test_yaml(
        &dir,
        "file2.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"file2\"\n",
    );

    let result = validate(vec![yaml1, yaml2]);
    assert!(result.is_ok());
}

#[test]
fn test_validate_internal_success() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "test.yaml",
        r#"
_forge_version: "5.0.0"
_name: "test"
inputs:
  x:
    value: 10
"#,
    );

    let result = validate_internal(&yaml, false);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_internal_success() {
    let dir = TempDir::new().unwrap();
    // Use simple scalar format that works with v1.0.0
    let yaml = create_test_yaml(
        &dir,
        "calc.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 50
    formula: null
  total:
    value: 100
    formula: "=price * 2"
"#,
    );

    let result = calculate_internal(&yaml, true);
    assert!(result.is_ok());
}

#[test]
fn test_functions_command_text() {
    // Just verify it doesn't panic
    let result = functions(false);
    assert!(result.is_ok());
}

#[test]
fn test_functions_command_json() {
    // Just verify it doesn't panic
    let result = functions(true);
    assert!(result.is_ok());
}

#[test]
fn test_run_watch_action_validate() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"watch\"\n",
    );

    // Just verify it doesn't panic
    run_watch_action(&yaml, true, false);
}

#[test]
fn test_run_watch_action_calculate() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch.yaml",
        r#"
_forge_version: "5.0.0"
_name: "watch"
inputs:
  x:
    value: 5
"#,
    );

    // Just verify it doesn't panic
    run_watch_action(&yaml, false, true);
}

#[test]
fn test_run_watch_action_validate_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_invalid.yaml",
        r#"_forge_version: "1.0.0"
result:
  value: null
  formula: "=nonexistent_var"
"#,
    );

    // Should not panic even with validation error
    run_watch_action(&yaml, true, false);
}

#[test]
fn test_run_watch_action_calculate_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_calc_err.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: null
  formula: "=undefined_var"
"#,
    );

    // Should not panic even with calculation error
    run_watch_action(&yaml, false, false);
}

#[test]
fn test_run_watch_action_validate_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_verbose.yaml",
        r#"_forge_version: "1.0.0"
sales:
  revenue: [100, 200, 300]
total:
  value: null
  formula: "=SUM(sales.revenue)"
"#,
    );

    // Test with verbose mode (covers verbose output paths)
    run_watch_action(&yaml, true, true);
}

#[test]
fn test_run_watch_action_calculate_with_tables() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_tables.yaml",
        r#"_forge_version: "1.0.0"
data:
  qty: [1, 2, 3]
  price: [10, 20, 30]
  total:
    formula: "=data.qty * data.price"
"#,
    );

    // Test calculate with tables (covers table output path)
    run_watch_action(&yaml, false, true);
}

#[test]
fn test_run_watch_action_mismatch() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_mismatch.yaml",
        r#"_forge_version: "1.0.0"
a:
  value: 10
  formula: null
b:
  value: 999
  formula: "=a * 2"
"#,
    );

    // Validation should fail due to mismatch, testing error path
    run_watch_action(&yaml, true, false);
}

// =========================================================================
// split_scalars_to_inputs_outputs Tests
// =========================================================================

#[test]
fn test_split_scalars_preserves_special_keys() {
    let yaml_str = r#"
_forge_version: "4.0.0"
_name: "test"
inputs:
  existing: {value: 1}
"#;
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // Special keys should be preserved
    assert!(yaml_map.contains_key(serde_yaml::Value::String("_forge_version".to_string())));
    assert!(yaml_map.contains_key(serde_yaml::Value::String("_name".to_string())));
}

#[test]
fn test_split_scalars_moves_value_only_to_inputs() {
    let yaml_str = r#"
_forge_version: "4.0.0"
my_input:
  value: 100
"#;
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // my_input should move to inputs
    assert!(!yaml_map.contains_key(serde_yaml::Value::String("my_input".to_string())));
    let inputs = yaml_map.get(serde_yaml::Value::String("inputs".to_string()));
    assert!(inputs.is_some());
}

#[test]
fn test_split_scalars_moves_formula_to_outputs() {
    let yaml_str = r#"
_forge_version: "4.0.0"
my_output:
  value: 200
  formula: "=x * 2"
"#;
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // my_output should move to outputs
    assert!(!yaml_map.contains_key(serde_yaml::Value::String("my_output".to_string())));
    let outputs = yaml_map.get(serde_yaml::Value::String("outputs".to_string()));
    assert!(outputs.is_some());
}

// =========================================================================
// Export variance tests
// =========================================================================

#[test]
fn test_export_variance_to_yaml() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("variance.yaml");

    let variances = vec![VarianceResult {
        name: "revenue".to_string(),
        budget: 1000.0,
        actual: 1100.0,
        variance: 100.0,
        variance_pct: 10.0,
        is_favorable: true,
        exceeds_threshold: false,
    }];

    let result = export_variance_to_yaml(&output_path, &variances, 5.0);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("revenue"));
    assert!(content.contains("1000"));
    assert!(content.contains("1100"));
}

#[test]
fn test_export_variance_to_excel() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("variance.xlsx");

    let variances = vec![VarianceResult {
        name: "costs".to_string(),
        budget: 500.0,
        actual: 600.0,
        variance: 100.0,
        variance_pct: 20.0,
        is_favorable: false,
        exceeds_threshold: true,
    }];

    let result = export_variance_to_excel(&output_path, &variances, 10.0);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

// =========================================================================
// calculate() Command Handler Tests
// =========================================================================

#[test]
fn test_calculate_success() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
  total:
    value: null
    formula: "=price * 2"
"#,
    );

    let result = calculate(yaml, true, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_verbose.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 3"
"#,
    );

    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_scenario() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars (not nested in summary) so scenario overrides work
    let yaml = create_test_yaml(
        &dir,
        "calc_scenario.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
result:
  value: null
  formula: "=rate * 100"
scenarios:
  high_rate:
    rate: 0.15
"#,
    );

    let result = calculate(yaml, true, true, Some("high_rate".to_string()));
    assert!(result.is_ok());
}

#[test]
fn test_calculate_invalid_scenario() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_bad_scenario.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = calculate(yaml, true, false, Some("nonexistent".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_calculate_file_not_found() {
    let result = calculate(PathBuf::from("/nonexistent/file.yaml"), true, false, None);
    assert!(result.is_err());
}

#[test]
fn test_calculate_invalid_yaml() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(&dir, "invalid.yaml", "not: valid: yaml: content:");

    let result = calculate(yaml, true, false, None);
    assert!(result.is_err());
}

#[test]
fn test_calculate_with_tables() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_tables.yaml",
        r#"_forge_version: "1.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200, 300]
"#,
    );

    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_dry_run_no_write() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "dry_run.yaml",
        r#"_forge_version: "1.0.0"
summary:
  a:
    value: 5
    formula: null
"#,
    );

    // Store original content
    let original = std::fs::read_to_string(&yaml).unwrap();

    let result = calculate(yaml.clone(), true, false, None);
    assert!(result.is_ok());

    // Verify file unchanged in dry run
    let after = std::fs::read_to_string(&yaml).unwrap();
    assert_eq!(original, after);
}

#[test]
fn test_calculate_writes_results() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "write_test.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 2"
"#,
    );

    // Not dry run - should write results
    let result = calculate(yaml.clone(), false, false, None);
    assert!(result.is_ok());

    // Backup should be created
    let backup = yaml.with_extension("yaml.bak");
    assert!(backup.exists());
}

#[test]
fn test_calculate_multi_doc() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("multi_doc.yaml");
    std::fs::write(
        &yaml_path,
        r#"---
_forge_version: "1.0.0"
_name: "doc1"
x:
  value: 10
  formula: null
---
_forge_version: "1.0.0"
_name: "doc2"
y:
  value: 20
  formula: null
"#,
    )
    .unwrap();

    // Multi-doc with dry_run=false triggers write-back not supported message
    let result = calculate(yaml_path, false, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_unit_warnings() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "unit_warn.yaml",
        r#"_forge_version: "1.0.0"
price:
  value: 100
  formula: null
  unit: "USD"
quantity:
  value: 5
  formula: null
  unit: "units"
result:
  value: null
  formula: "=price + quantity"
  unit: "USD"
"#,
    );

    // Adding USD + units should trigger unit warning
    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}

// =========================================================================
// audit() Command Handler Tests
// =========================================================================

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

// =========================================================================
// export() Command Handler Tests
// =========================================================================

#[test]
fn test_export_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "export.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
"#,
    );
    let output = dir.path().join("output.xlsx");

    let result = export(yaml, output.clone(), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_export_verbose() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "export_verbose.yaml",
        r#"_forge_version: "1.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200, 300]
"#,
    );
    let output = dir.path().join("output_verbose.xlsx");

    let result = export(yaml, output.clone(), true);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_export_file_not_found() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.xlsx");

    let result = export(PathBuf::from("/nonexistent.yaml"), output, false);
    assert!(result.is_err());
}

// =========================================================================
// import() Command Handler Tests
// =========================================================================

#[test]
fn test_import_basic() {
    // First export a YAML to Excel, then import it back
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_source.yaml",
        r#"_forge_version: "1.0.0"
sales:
  product: ["A", "B", "C"]
  revenue: [100, 200, 300]
"#,
    );
    let xlsx = dir.path().join("temp.xlsx");
    let output_yaml = dir.path().join("imported.yaml");

    // Export first
    export(yaml, xlsx.clone(), false).unwrap();

    // Now import
    let result = import(xlsx, output_yaml.clone(), false, false, false);
    assert!(result.is_ok());
    assert!(output_yaml.exists());
}

#[test]
fn test_import_verbose() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_verbose_source.yaml",
        r#"_forge_version: "1.0.0"
data:
  x: [1, 2, 3]
"#,
    );
    let xlsx = dir.path().join("temp_verbose.xlsx");
    let output_yaml = dir.path().join("imported_verbose.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_yaml.clone(), true, false, false);
    assert!(result.is_ok());
}

#[test]
fn test_import_split_files() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_split_source.yaml",
        r#"_forge_version: "1.0.0"
table1:
  a: [1, 2]
table2:
  b: [3, 4]
"#,
    );
    let xlsx = dir.path().join("split.xlsx");
    let output_dir = dir.path().join("split_output");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_dir.clone(), false, true, false);
    assert!(result.is_ok());
    assert!(output_dir.exists());
}

#[test]
fn test_import_split_files_with_scalars_verbose() {
    let dir = TempDir::new().unwrap();
    // Create YAML with both tables and scalars
    let yaml = create_test_yaml(
        &dir,
        "import_mixed.yaml",
        r#"_forge_version: "1.0.0"
sales:
  revenue: [100, 200, 300]
tax_rate:
  value: 0.1
  formula: null
"#,
    );
    let xlsx = dir.path().join("mixed.xlsx");
    let output_dir = dir.path().join("mixed_output");

    export(yaml, xlsx.clone(), false).unwrap();

    // verbose=true, split_files=true
    let result = import(xlsx, output_dir.clone(), true, true, false);
    assert!(result.is_ok());
    // Should create both table and scalar files
    assert!(output_dir.exists());
}

#[test]
fn test_import_multi_doc_with_scalars() {
    let dir = TempDir::new().unwrap();
    // Create YAML with tables and scalars for multi-doc output
    let yaml = create_test_yaml(
        &dir,
        "import_multidoc_mixed.yaml",
        r#"_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
constant:
  value: 42
  formula: null
"#,
    );
    let xlsx = dir.path().join("multidoc.xlsx");
    let output_yaml = dir.path().join("multidoc_out.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    // multi_doc=true - should include scalars document
    let result = import(xlsx, output_yaml.clone(), false, false, true);
    assert!(result.is_ok());
}

#[test]
fn test_import_multi_doc() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_multi_source.yaml",
        r#"_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
"#,
    );
    let xlsx = dir.path().join("multi.xlsx");
    let output_yaml = dir.path().join("multi.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_yaml.clone(), false, false, true);
    assert!(result.is_ok());
}

#[test]
fn test_import_file_not_found() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.yaml");

    let result = import(
        PathBuf::from("/nonexistent.xlsx"),
        output,
        false,
        false,
        false,
    );
    assert!(result.is_err());
}

// =========================================================================
// compare() Command Handler Tests
// =========================================================================

#[test]
fn test_compare_two_scenarios() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars for scenario overrides to work
    let yaml = create_test_yaml(
        &dir,
        "compare.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
revenue:
  value: 1000
  formula: null
profit:
  value: null
  formula: "=revenue * rate"
scenarios:
  low:
    rate: 0.03
  high:
    rate: 0.10
"#,
    );

    let result = compare(yaml, vec!["low".to_string(), "high".to_string()], false);
    assert!(result.is_ok());
}

#[test]
fn test_compare_verbose() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars
    let yaml = create_test_yaml(
        &dir,
        "compare_verbose.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
scenarios:
  a:
    x: 5
  b:
    x: 15
"#,
    );

    let result = compare(yaml, vec!["a".to_string(), "b".to_string()], true);
    assert!(result.is_ok());
}

#[test]
fn test_compare_with_formula_only_vars() {
    let dir = TempDir::new().unwrap();
    // Variable with formula but no value (covers value=None path)
    let yaml = create_test_yaml(
        &dir,
        "compare_formula.yaml",
        r#"_forge_version: "1.0.0"
base:
  value: 100
  formula: null
derived:
  value: null
  formula: "=base * 2"
scenarios:
  s1:
    base: 50
  s2:
    base: 200
"#,
    );

    let result = compare(yaml, vec!["s1".to_string(), "s2".to_string()], false);
    assert!(result.is_ok());
}

#[test]
fn test_compare_scenario_not_found() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars and correct scenario format
    let yaml = create_test_yaml(
        &dir,
        "compare_notfound.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
scenarios:
  exists:
    x: 5
"#,
    );

    let result = compare(
        yaml,
        vec!["exists".to_string(), "missing".to_string()],
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_compare_file_not_found() {
    let result = compare(
        PathBuf::from("/nonexistent.yaml"),
        vec!["a".to_string()],
        false,
    );
    assert!(result.is_err());
}

// =========================================================================
// variance() Command Handler Tests
// =========================================================================

#[test]
fn test_variance_basic() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget.yaml",
        r#"_forge_version: "1.0.0"
summary:
  revenue:
    value: 1000
    formula: null
  expense:
    value: 500
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual.yaml",
        r#"_forge_version: "1.0.0"
summary:
  revenue:
    value: 1100
    formula: null
  expense:
    value: 550
    formula: null
"#,
    );

    let result = variance(budget, actual, 10.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_verbose() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_v.yaml",
        r#"_forge_version: "1.0.0"
summary:
  sales:
    value: 500
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_v.yaml",
        r#"_forge_version: "1.0.0"
summary:
  sales:
    value: 600
    formula: null
"#,
    );

    let result = variance(budget, actual, 5.0, None, true);
    assert!(result.is_ok());
}

#[test]
fn test_variance_output_xlsx() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_xlsx.yaml",
        r#"_forge_version: "1.0.0"
summary:
  profit:
    value: 200
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_xlsx.yaml",
        r#"_forge_version: "1.0.0"
summary:
  profit:
    value: 250
    formula: null
"#,
    );
    let output = dir.path().join("variance_report.xlsx");

    let result = variance(budget, actual, 10.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_variance_output_yaml() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_yaml.yaml",
        r#"_forge_version: "1.0.0"
summary:
  cost:
    value: 100
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_yaml.yaml",
        r#"_forge_version: "1.0.0"
summary:
  cost:
    value: 120
    formula: null
"#,
    );
    let output = dir.path().join("variance_report.yaml");

    let result = variance(budget, actual, 5.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_variance_unsupported_format() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_bad.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_bad.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 15
    formula: null
"#,
    );
    let output = dir.path().join("variance.txt"); // Unsupported

    let result = variance(budget, actual, 10.0, Some(output), false);
    assert!(result.is_err());
}

#[test]
fn test_variance_file_not_found() {
    let dir = TempDir::new().unwrap();
    let actual = create_test_yaml(
        &dir,
        "actual_only.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = variance(
        PathBuf::from("/nonexistent.yaml"),
        actual,
        10.0,
        None,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_variance_unfavorable() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 800
  formula: null
"#,
    );

    // Revenue below budget = unfavorable
    let result = variance(budget, actual, 5.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_zero_budget() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_zero.yaml",
        r#"_forge_version: "1.0.0"
amount:
  value: 0
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_zero.yaml",
        r#"_forge_version: "1.0.0"
amount:
  value: 100
  formula: null
"#,
    );

    // Zero budget should not cause division by zero
    let result = variance(budget, actual, 10.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_xlsx_unfavorable() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_xlsx_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
cost:
  value: 500
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_xlsx_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 900
  formula: null
cost:
  value: 600
  formula: null
"#,
    );
    let output = dir.path().join("variance_unfav.xlsx");

    // Revenue below budget (unfavorable), cost above budget (unfavorable)
    let result = variance(budget, actual, 5.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

// =========================================================================
// sensitivity() Command Handler Tests
// =========================================================================

#[test]
fn test_sensitivity_one_variable() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars for sensitivity analysis
    let yaml = create_test_yaml(
        &dir,
        "sensitivity.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
principal:
  value: 1000
  formula: null
interest:
  value: null
  formula: "=principal * rate"
"#,
    );

    let result = sensitivity(
        yaml,
        "rate".to_string(),
        "0.01,0.10,0.02".to_string(),
        None,
        None,
        "interest".to_string(),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_sensitivity_two_variables() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_2var.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
years:
  value: 5
  formula: null
result:
  value: null
  formula: "=rate * years"
"#,
    );

    let result = sensitivity(
        yaml,
        "rate".to_string(),
        "0.01,0.05,0.02".to_string(),
        Some("years".to_string()),
        Some("1,5,2".to_string()),
        "result".to_string(),
        true,
    );
    assert!(result.is_ok());
}

#[test]
fn test_sensitivity_variable_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = sensitivity(
        yaml,
        "nonexistent".to_string(),
        "1,10,1".to_string(),
        None,
        None,
        "x".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_sensitivity_invalid_range() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_badrange.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 2"
"#,
    );

    let result = sensitivity(
        yaml,
        "x".to_string(),
        "invalid".to_string(),
        None,
        None,
        "y".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_sensitivity_second_var_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_2nd_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  result:
    value: null
    formula: "=x * 2"
"#,
    );

    let result = sensitivity(
        yaml,
        "x".to_string(),
        "1,10,1".to_string(),
        Some("missing".to_string()),
        Some("1,5,1".to_string()),
        "result".to_string(),
        false,
    );
    assert!(result.is_err());
}

// =========================================================================
// goal_seek() Command Handler Tests
// =========================================================================

#[test]
fn test_goal_seek_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
principal:
  value: 1000
  formula: null
interest:
  value: null
  formula: "=principal * rate"
"#,
    );

    // Find rate where interest = 100
    let result = goal_seek(
        yaml,
        "interest".to_string(),
        100.0,
        "rate".to_string(),
        Some(0.01),
        Some(0.5),
        0.0001,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_goal_seek_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_verbose.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 5
  formula: null
y:
  value: null
  formula: "=x * 10"
"#,
    );

    let result = goal_seek(
        yaml,
        "y".to_string(),
        100.0,
        "x".to_string(),
        Some(1.0),
        Some(20.0),
        0.001,
        true,
    );
    assert!(result.is_ok());
}

#[test]
fn test_goal_seek_variable_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = goal_seek(
        yaml,
        "x".to_string(),
        50.0,
        "nonexistent".to_string(),
        None,
        None,
        0.001,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_goal_seek_no_solution() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_nosol.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * x"
"#,
    );

    // Try to find x where x^2 = -100 (impossible in reals)
    let result = goal_seek(
        yaml,
        "y".to_string(),
        -100.0,
        "x".to_string(),
        Some(0.1),
        Some(10.0),
        0.001,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_goal_seek_default_bounds() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_defaults.yaml",
        r#"_forge_version: "1.0.0"
factor:
  value: 2.0
  formula: null
result:
  value: null
  formula: "=factor * 50"
"#,
    );

    // Don't specify min/max - use defaults
    let result = goal_seek(
        yaml,
        "result".to_string(),
        100.0,
        "factor".to_string(),
        None,
        None,
        0.0001,
        false,
    );
    assert!(result.is_ok());
}

// =========================================================================
// break_even() Command Handler Tests
// =========================================================================

#[test]
fn test_break_even_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "break_even.yaml",
        r#"_forge_version: "1.0.0"
price:
  value: 100
  formula: null
cost:
  value: 60
  formula: null
units:
  value: 100
  formula: null
fixed_costs:
  value: 2000
  formula: null
profit:
  value: null
  formula: "=(price - cost) * units - fixed_costs"
"#,
    );

    // Find units where profit = 0
    let result = break_even(
        yaml,
        "profit".to_string(),
        "units".to_string(),
        Some(1.0),
        Some(200.0),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_break_even_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "break_even_verbose.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
costs:
  value: 1200
  formula: null
margin_pct:
  value: 0.20
  formula: null
net:
  value: null
  formula: "=revenue * margin_pct - costs * 0.1"
"#,
    );

    let result = break_even(
        yaml,
        "net".to_string(),
        "revenue".to_string(),
        Some(100.0),
        Some(10000.0),
        true,
    );
    assert!(result.is_ok());
}

// =========================================================================
// watch() Command Handler Tests
// =========================================================================

#[test]
fn test_watch_file_not_found() {
    let result = watch(PathBuf::from("/nonexistent.yaml"), false, false);
    assert!(result.is_err());
}

// Note: watch() loops forever so we can only test error cases
// The run_watch_action helper is already tested above

// =========================================================================
// upgrade() Command Handler Tests
// =========================================================================

#[test]
fn test_upgrade_dry_run() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_dry.yaml",
        r#"_forge_version: "4.0.0"
_name: "test"
my_input:
  value: 100
my_output:
  value: 200
  formula: "=my_input * 2"
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), false);
    assert!(result.is_ok());
}

#[test]
fn test_upgrade_already_current() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_current.yaml",
        r#"_forge_version: "5.0.0"
_name: "test"
inputs:
  x:
    value: 10
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), true);
    assert!(result.is_ok());
}

#[test]
fn test_upgrade_with_write() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_write.yaml",
        r#"_forge_version: "4.0.0"
_name: "test"
value_only:
  value: 50
"#,
    );

    let result = upgrade(yaml.clone(), false, "5.0.0".to_string(), true);
    assert!(result.is_ok());

    // Backup should exist
    let backup = yaml.with_extension("yaml.bak");
    assert!(backup.exists());
}

#[test]
fn test_upgrade_file_not_found() {
    let result = upgrade(
        PathBuf::from("/nonexistent.yaml"),
        true,
        "5.0.0".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_upgrade_with_includes() {
    let dir = TempDir::new().unwrap();

    // Create included file first
    create_test_yaml(
        &dir,
        "included.yaml",
        r#"_forge_version: "4.0.0"
_name: "included"
inc_value:
  value: 25
"#,
    );

    // Create main file with include reference
    let yaml = create_test_yaml(
        &dir,
        "main.yaml",
        r#"_forge_version: "4.0.0"
_name: "main"
_includes:
  - file: "included.yaml"
main_value:
  value: 100
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), true);
    assert!(result.is_ok());
}

// =========================================================================
// validate() Command Handler Additional Tests
// =========================================================================

#[test]
fn test_validate_single_invalid() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(&dir, "invalid.yaml", "not: [valid yaml");

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}

#[test]
fn test_validate_batch_with_failures() {
    let dir = TempDir::new().unwrap();
    let valid = create_test_yaml(
        &dir,
        "valid.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"valid\"\n",
    );
    let invalid = create_test_yaml(&dir, "invalid.yaml", "broken: [yaml");

    let result = validate(vec![valid, invalid]);
    assert!(result.is_err());
}

#[test]
fn test_validate_mismatch_values() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "mismatch.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: 999
    formula: "=x * 2"
"#,
    );

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}

#[test]
fn test_validate_table_length_mismatch() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad_table.yaml");
    // Create a YAML that will fail table length validation
    // (inconsistent column lengths)
    std::fs::write(
        &yaml_path,
        r#"_forge_version: "1.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200]
"#,
    )
    .unwrap();

    let result = validate(vec![yaml_path]);
    assert!(result.is_err());
}

#[test]
fn test_validate_calculation_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_error.yaml",
        r#"_forge_version: "1.0.0"
a:
  value: 10
  formula: null
b:
  value: null
  formula: "=nonexistent_var"
"#,
    );

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}
