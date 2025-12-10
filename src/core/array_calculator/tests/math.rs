//! Math function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_round_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.456, 2.789, 3.123, 4.555]),
    ));
    table.add_row_formula("rounded_1".to_string(), "=ROUND(values, 1)".to_string());
    table.add_row_formula("rounded_2".to_string(), "=ROUND(values, 2)".to_string());
    table.add_row_formula("rounded_0".to_string(), "=ROUND(values, 0)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let rounded_1 = result_table.columns.get("rounded_1").unwrap();
    match &rounded_1.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.5);
            assert_eq!(nums[1], 2.8);
            assert_eq!(nums[2], 3.1);
            assert_eq!(nums[3], 4.6);
        }
        _ => panic!("Expected Number array"),
    }

    let rounded_2 = result_table.columns.get("rounded_2").unwrap();
    match &rounded_2.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.46);
            assert_eq!(nums[1], 2.79);
            assert_eq!(nums[2], 3.12);
            assert_eq!(nums[3], 4.56);
        }
        _ => panic!("Expected Number array"),
    }

    let rounded_0 = result_table.columns.get("rounded_0").unwrap();
    match &rounded_0.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.0);
            assert_eq!(nums[1], 3.0);
            assert_eq!(nums[2], 3.0);
            assert_eq!(nums[3], 5.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_roundup_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.231, 2.678, 3.449]),
    ));
    table.add_row_formula("rounded_up".to_string(), "=ROUNDUP(values, 1)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let rounded_up = result_table.columns.get("rounded_up").unwrap();
    match &rounded_up.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.3);
            assert_eq!(nums[1], 2.7);
            assert_eq!(nums[2], 3.5);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_rounddown_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.789, 2.345, 3.999]),
    ));
    table.add_row_formula(
        "rounded_down".to_string(),
        "=ROUNDDOWN(values, 1)".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let rounded_down = result_table.columns.get("rounded_down").unwrap();
    match &rounded_down.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.7);
            assert_eq!(nums[1], 2.3);
            assert_eq!(nums[2], 3.9);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_ceiling_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.1, 2.3, 4.7, 10.2]),
    ));
    table.add_row_formula("ceiling_1".to_string(), "=CEILING(values, 1)".to_string());
    table.add_row_formula("ceiling_5".to_string(), "=CEILING(values, 5)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let ceiling_1 = result_table.columns.get("ceiling_1").unwrap();
    match &ceiling_1.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 2.0);
            assert_eq!(nums[1], 3.0);
            assert_eq!(nums[2], 5.0);
            assert_eq!(nums[3], 11.0);
        }
        _ => panic!("Expected Number array"),
    }

    let ceiling_5 = result_table.columns.get("ceiling_5").unwrap();
    match &ceiling_5.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 5.0);
            assert_eq!(nums[1], 5.0);
            assert_eq!(nums[2], 5.0);
            assert_eq!(nums[3], 15.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_floor_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.9, 2.7, 4.3, 10.8]),
    ));
    table.add_row_formula("floor_1".to_string(), "=FLOOR(values, 1)".to_string());
    table.add_row_formula("floor_5".to_string(), "=FLOOR(values, 5)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let floor_1 = result_table.columns.get("floor_1").unwrap();
    match &floor_1.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.0);
            assert_eq!(nums[1], 2.0);
            assert_eq!(nums[2], 4.0);
            assert_eq!(nums[3], 10.0);
        }
        _ => panic!("Expected Number array"),
    }

    let floor_5 = result_table.columns.get("floor_5").unwrap();
    match &floor_5.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 0.0);
            assert_eq!(nums[1], 0.0);
            assert_eq!(nums[2], 0.0);
            assert_eq!(nums[3], 10.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_mod_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 15.0, 23.0, 7.0]),
    ));
    table.add_row_formula("mod_3".to_string(), "=MOD(values, 3)".to_string());
    table.add_row_formula("mod_5".to_string(), "=MOD(values, 5)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let mod_3 = result_table.columns.get("mod_3").unwrap();
    match &mod_3.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.0);
            assert_eq!(nums[1], 0.0);
            assert_eq!(nums[2], 2.0);
            assert_eq!(nums[3], 1.0);
        }
        _ => panic!("Expected Number array"),
    }

    let mod_5 = result_table.columns.get("mod_5").unwrap();
    match &mod_5.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 0.0);
            assert_eq!(nums[1], 0.0);
            assert_eq!(nums[2], 3.0);
            assert_eq!(nums[3], 2.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_sqrt_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![4.0, 9.0, 16.0, 25.0, 100.0]),
    ));
    table.add_row_formula("sqrt_values".to_string(), "=SQRT(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let sqrt_values = result_table.columns.get("sqrt_values").unwrap();
    match &sqrt_values.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 2.0);
            assert_eq!(nums[1], 3.0);
            assert_eq!(nums[2], 4.0);
            assert_eq!(nums[3], 5.0);
            assert_eq!(nums[4], 10.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_power_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_row_formula("power_2".to_string(), "=POWER(base, 2)".to_string());
    table.add_row_formula("power_3".to_string(), "=POWER(base, 3)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let power_2 = result_table.columns.get("power_2").unwrap();
    match &power_2.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 4.0);
            assert_eq!(nums[1], 9.0);
            assert_eq!(nums[2], 16.0);
            assert_eq!(nums[3], 25.0);
        }
        _ => panic!("Expected Number array"),
    }

    let power_3 = result_table.columns.get("power_3").unwrap();
    match &power_3.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 8.0);
            assert_eq!(nums[1], 27.0);
            assert_eq!(nums[2], 64.0);
            assert_eq!(nums[3], 125.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_abs_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10.0, 5.0, -3.0, 8.0]),
    ));
    data.row_formulas
        .insert("absolute".to_string(), "=ABS(values)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let abs_col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("absolute")
        .unwrap();
    if let ColumnValue::Number(values) = &abs_col.values {
        assert_eq!(values[0], 10.0);
        assert_eq!(values[1], 5.0);
        assert_eq!(values[2], 3.0);
        assert_eq!(values[3], 8.0);
    } else {
        panic!("Expected numeric column");
    }
}

#[test]
fn test_round_functions() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "val".to_string(),
        Variable::new("val".to_string(), Some(3.567), None),
    );
    model.add_scalar(
        "rounded".to_string(),
        Variable::new(
            "rounded".to_string(),
            None,
            Some("=ROUND(val, 2)".to_string()),
        ),
    );
    model.add_scalar(
        "up".to_string(),
        Variable::new("up".to_string(), None, Some("=ROUNDUP(val, 1)".to_string())),
    );
    model.add_scalar(
        "down".to_string(),
        Variable::new(
            "down".to_string(),
            None,
            Some("=ROUNDDOWN(val, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("rounded").unwrap().value.unwrap() - 3.57).abs() < 0.01);
    assert!((result.scalars.get("up").unwrap().value.unwrap() - 3.6).abs() < 0.01);
    assert!((result.scalars.get("down").unwrap().value.unwrap() - 3.5).abs() < 0.01);
}

#[test]
fn test_power_and_sqrt() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "base".to_string(),
        Variable::new("base".to_string(), Some(2.0), None),
    );
    model.add_scalar(
        "squared".to_string(),
        Variable::new(
            "squared".to_string(),
            None,
            Some("=POWER(base, 2)".to_string()),
        ),
    );
    model.add_scalar(
        "root".to_string(),
        Variable::new("root".to_string(), None, Some("=SQRT(squared)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("squared").unwrap().value.unwrap() - 4.0).abs() < 0.01);
    assert!((result.scalars.get("root").unwrap().value.unwrap() - 2.0).abs() < 0.01);
}

#[test]
fn test_mod_scalar_formula() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=MOD(17, 5)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 2.0).abs() < 0.01); // 17 mod 5 = 2
}

#[test]
fn test_floor_and_ceiling() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "val".to_string(),
        Variable::new("val".to_string(), Some(4.3), None),
    );
    model.add_scalar(
        "floor".to_string(),
        Variable::new(
            "floor".to_string(),
            None,
            Some("=FLOOR(val, 1)".to_string()),
        ),
    );
    model.add_scalar(
        "ceil".to_string(),
        Variable::new(
            "ceil".to_string(),
            None,
            Some("=CEILING(val, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("floor").unwrap().value.unwrap() - 4.0).abs() < 0.01);
    assert!((result.scalars.get("ceil").unwrap().value.unwrap() - 5.0).abs() < 0.01);
}

#[test]
fn test_round_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![3.14159]),
    ));
    data.row_formulas
        .insert("rounded".to_string(), "=ROUND(value, 2)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("rounded") {
        if let ColumnValue::Number(vals) = &col.values {
            assert!((vals[0] - 3.14).abs() < 0.001);
        }
    }
}

#[test]
fn test_ceiling_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![4.3]),
    ));
    data.row_formulas
        .insert("ceil".to_string(), "=CEILING(value, 1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("ceil") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 5.0);
        }
    }
}

#[test]
fn test_floor_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![4.9]),
    ));
    data.row_formulas
        .insert("floor_val".to_string(), "=FLOOR(value, 1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("floor_val") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 4.0);
        }
    }
}

#[test]
fn test_mod_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    data.row_formulas
        .insert("remainder".to_string(), "=MOD(value, 3)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("remainder") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 1.0);
        }
    }
}

#[test]
fn test_sqrt_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![16.0]),
    ));
    data.row_formulas
        .insert("root".to_string(), "=SQRT(value)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("root") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 4.0);
        }
    }
}

#[test]
fn test_power_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=POWER(base, 10)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("result") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 1024.0);
        }
    }
}

#[test]
#[cfg(feature = "full")]
fn test_sln_function_coverage() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "depreciation".to_string(),
        Variable::new(
            "depreciation".to_string(),
            None,
            Some("=SLN(30000, 7500, 10)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // SLN(30000, 7500, 10) = (30000 - 7500) / 10 = 2250
    let val = result.scalars.get("depreciation").unwrap().value.unwrap();
    assert!((val - 2250.0).abs() < 0.01);
}

#[test]
fn test_abs_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![-10.0, 20.0, -30.0]),
    ));
    data.row_formulas
        .insert("abs_val".to_string(), "=ABS(value)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "full")]
fn test_exp_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, 2.0]),
    ));
    data.row_formulas
        .insert("exp_x".to_string(), "=EXP(x)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("exp_x") {
        if let ColumnValue::Number(vals) = &col.values {
            assert!((vals[0] - 1.0).abs() < 0.001); // EXP(0) = 1
            assert!((vals[1] - std::f64::consts::E).abs() < 0.001); // EXP(1) = e ≈ 2.718
            assert!((vals[2] - (std::f64::consts::E * std::f64::consts::E)).abs() < 0.001);
            // EXP(2) = e^2 ≈ 7.389
        }
    }
}

#[test]
#[cfg(feature = "full")]
fn test_ln_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.718, 10.0]),
    ));
    data.row_formulas
        .insert("ln_x".to_string(), "=LN(x)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("ln_x") {
        if let ColumnValue::Number(vals) = &col.values {
            assert!((vals[0] - 0.0).abs() < 0.001); // LN(1) = 0
            assert!((vals[1] - 1.0).abs() < 0.01); // LN(e) ≈ 1
            assert!((vals[2] - 2.302585).abs() < 0.001); // LN(10) ≈ 2.302585
        }
    }
}

#[test]
fn test_log_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 100.0, 1000.0]),
    ));
    data.row_formulas
        .insert("log_x".to_string(), "=LOG(x, 10)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // LOG function is not currently implemented - test verifies error handling
    assert!(
        result.is_err(),
        "LOG function should error (not implemented)"
    );
}

#[test]
#[cfg(feature = "full")]
fn test_sln_depreciation() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=SLN(10000, 1000, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // SLN(10000, 1000, 5) = (10000 - 1000) / 5 = 1800
    let val = result.scalars.get("depr").unwrap().value.unwrap();
    assert!((val - 1800.0).abs() < 0.01);
}

#[test]
fn test_abs_negative_value() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=ABS(-5)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 5.0).abs() < 0.01);
}

#[test]
#[cfg(feature = "full")]
fn test_exp_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=EXP(1)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // EXP(1) = e ≈ 2.718
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - std::f64::consts::E).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_ln_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=LN(2.718)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LN(e) ≈ 1.0
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 1.0).abs() < 0.01);
}

#[test]
fn test_log_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LOG(100, 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // LOG function is not currently implemented - test verifies error handling
    assert!(
        result.is_err(),
        "LOG function should error (not implemented)"
    );
}

#[test]
#[cfg(feature = "full")]
fn test_log10_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=LOG10(1000)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LOG10(1000) = 3.0
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 3.0).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_sign_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SIGN(-5)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // SIGN(-5) = -1.0
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - (-1.0)).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_int_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=INT(5.7)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // INT(5.7) = 5.0
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 5.0).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_trunc_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=TRUNC(5.789, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // TRUNC(5.789, 2) = 5.78
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 5.78).abs() < 0.001);
}

#[test]
fn test_ceiling_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CEILING(4.3, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // CEILING(4.3, 1) = 5
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 5.0).abs() < 0.0001);
}

#[test]
fn test_floor_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=FLOOR(4.7, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // FLOOR(4.7, 1) = 4
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 4.0).abs() < 0.0001);
}

#[test]
fn test_mod_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=MOD(10, 3)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // MOD(10, 3) = 1
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 1.0).abs() < 0.0001);
}

#[test]
fn test_sqrt_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SQRT(16)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // SQRT(16) = 4
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 4.0).abs() < 0.0001);
}

#[test]
fn test_power_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=POWER(2, 8)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // POWER(2, 8) = 256
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 256.0).abs() < 0.0001);
}

#[test]
#[cfg(feature = "full")]
fn test_variance_abs_value() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(120.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "var".to_string(),
        Variable::new(
            "var".to_string(),
            None,
            Some("=VARIANCE(actual, budget)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // VARIANCE(120, 100) = 120 - 100 = 20.0
    let val = result.scalars.get("var").unwrap().value.unwrap();
    assert!((val - 20.0).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_pi_constant() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "pi_value".to_string(),
        Variable::new("pi_value".to_string(), None, Some("=PI()".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let pi_val = result.scalars.get("pi_value").unwrap().value.unwrap();
    assert!((pi_val - std::f64::consts::PI).abs() < 0.000001);
}

#[test]
#[cfg(feature = "full")]
fn test_pi_in_formula() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "circle_area".to_string(),
        Variable::new(
            "circle_area".to_string(),
            None,
            Some("=PI() * POWER(5, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let area = result.scalars.get("circle_area").unwrap().value.unwrap();
    assert!((area - (std::f64::consts::PI * 25.0)).abs() < 0.0001);
}

#[test]
#[cfg(feature = "full")]
fn test_pi_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "radius".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_row_formula(
        "circumference".to_string(),
        "=2 * PI() * radius".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let circumference = result_table.columns.get("circumference").unwrap();
    match &circumference.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 2.0 * std::f64::consts::PI).abs() < 0.0001);
            assert!((nums[1] - 4.0 * std::f64::consts::PI).abs() < 0.0001);
            assert!((nums[2] - 6.0 * std::f64::consts::PI).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(feature = "full")]
fn test_e_constant() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "e_value".to_string(),
        Variable::new("e_value".to_string(), None, Some("=E()".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let e_val = result.scalars.get("e_value").unwrap().value.unwrap();
    assert!((e_val - std::f64::consts::E).abs() < 0.000001);
}

#[test]
#[cfg(feature = "full")]
fn test_e_in_formula() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "e_squared".to_string(),
        Variable::new(
            "e_squared".to_string(),
            None,
            Some("=POWER(E(), 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let e_sq = result.scalars.get("e_squared").unwrap().value.unwrap();
    assert!((e_sq - (std::f64::consts::E * std::f64::consts::E)).abs() < 0.0001);
}

#[test]
#[cfg(feature = "full")]
fn test_e_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "multiplier".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_row_formula("e_multiple".to_string(), "=E() * multiplier".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let e_multiple = result_table.columns.get("e_multiple").unwrap();
    match &e_multiple.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - std::f64::consts::E).abs() < 0.0001);
            assert!((nums[1] - 2.0 * std::f64::consts::E).abs() < 0.0001);
            assert!((nums[2] - 3.0 * std::f64::consts::E).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(feature = "full")]
fn test_pow_function_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=POW(2, 8)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 256.0).abs() < 0.0001);
}

#[test]
#[cfg(feature = "full")]
fn test_pow_function_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 5.0]),
    ));
    table.add_row_formula("cubed".to_string(), "=POW(base, 3)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let cubed = result_table.columns.get("cubed").unwrap();
    match &cubed.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 8.0);
            assert_eq!(nums[1], 27.0);
            assert_eq!(nums[2], 125.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(feature = "full")]
fn test_pow_negative_exponent() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=POW(2, -2)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 0.25).abs() < 0.0001);
}

#[test]
#[cfg(feature = "full")]
fn test_pow_fractional_exponent() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=POW(16, 0.5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 4.0).abs() < 0.0001);
}
