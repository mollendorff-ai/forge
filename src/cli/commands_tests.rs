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
