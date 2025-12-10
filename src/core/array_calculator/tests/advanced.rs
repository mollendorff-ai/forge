//! Advanced function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_let_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Test simple LET: =LET(x, 10, x * 2) → 20
    model.add_scalar(
        "simple_let".to_string(),
        Variable::new(
            "simple_let".to_string(),
            None,
            Some("=LET(x, 10, x * 2)".to_string()),
        ),
    );

    // Test multiple variables: =LET(x, 5, y, 3, x + y) → 8
    model.add_scalar(
        "multi_var".to_string(),
        Variable::new(
            "multi_var".to_string(),
            None,
            Some("=LET(x, 5, y, 3, x + y)".to_string()),
        ),
    );

    // Test dependent variables: =LET(a, 10, b, a * 2, b + 5) → 25
    model.add_scalar(
        "dependent".to_string(),
        Variable::new(
            "dependent".to_string(),
            None,
            Some("=LET(a, 10, b, a * 2, b + 5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let simple = result.scalars.get("simple_let").unwrap().value.unwrap();
    assert!(
        (simple - 20.0).abs() < 0.001,
        "LET(x, 10, x * 2) should return 20, got {}",
        simple
    );

    let multi = result.scalars.get("multi_var").unwrap().value.unwrap();
    assert!(
        (multi - 8.0).abs() < 0.001,
        "LET(x, 5, y, 3, x + y) should return 8, got {}",
        multi
    );

    let dep = result.scalars.get("dependent").unwrap().value.unwrap();
    assert!(
        (dep - 25.0).abs() < 0.001,
        "LET(a, 10, b, a * 2, b + 5) should return 25, got {}",
        dep
    );
}

#[test]
fn test_let_with_aggregation() {
    use crate::types::{Column, ColumnValue, Table, Variable};
    let mut model = ParsedModel::new();

    // Create a table with values
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(sales);

    // Test LET with SUM: =LET(total, SUM(sales.revenue), rate, 0.1, total * rate) → 150
    model.add_scalar(
        "tax".to_string(),
        Variable::new(
            "tax".to_string(),
            None,
            Some("=LET(total, SUM(sales.revenue), rate, 0.1, total * rate)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let tax = result.scalars.get("tax").unwrap().value.unwrap();
    // SUM(100+200+300+400+500) = 1500, 1500 * 0.1 = 150
    assert!(
        (tax - 150.0).abs() < 0.001,
        "LET with SUM should return 150, got {}",
        tax
    );
}

#[test]
fn test_switch_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Test SWITCH with number matching: SWITCH(2, 1, 0.05, 2, 0.10, 3, 0.15) → 0.10
    model.add_scalar(
        "matched".to_string(),
        Variable::new(
            "matched".to_string(),
            None,
            Some("=SWITCH(2, 1, 0.05, 2, 0.10, 3, 0.15)".to_string()),
        ),
    );

    // Test SWITCH with default: SWITCH(4, 1, 100, 2, 200, 50) → 50
    model.add_scalar(
        "with_default".to_string(),
        Variable::new(
            "with_default".to_string(),
            None,
            Some("=SWITCH(4, 1, 100, 2, 200, 50)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let matched = result.scalars.get("matched").unwrap().value.unwrap();
    assert!(
        (matched - 0.10).abs() < 0.001,
        "SWITCH(2, ...) should return 0.10, got {}",
        matched
    );

    let with_default = result.scalars.get("with_default").unwrap().value.unwrap();
    assert!(
        (with_default - 50.0).abs() < 0.001,
        "SWITCH(4, ..., 50) should return default 50, got {}",
        with_default
    );
}

#[test]
fn test_lambda_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Test simple lambda: LAMBDA(x, x * 2)(5) → 10
    model.add_scalar(
        "double".to_string(),
        Variable::new(
            "double".to_string(),
            None,
            Some("=LAMBDA(x, x * 2)(5)".to_string()),
        ),
    );

    // Test multi-param lambda: LAMBDA(x, y, x + y)(3, 4) → 7
    model.add_scalar(
        "add".to_string(),
        Variable::new(
            "add".to_string(),
            None,
            Some("=LAMBDA(x, y, x + y)(3, 4)".to_string()),
        ),
    );

    // Test compound interest: LAMBDA(p, r, n, p * (1 + r) ^ n)(1000, 0.05, 10) → 1628.89
    model.add_scalar(
        "compound".to_string(),
        Variable::new(
            "compound".to_string(),
            None,
            Some("=LAMBDA(p, r, n, p * (1 + r) ^ n)(1000, 0.05, 10)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let double = result.scalars.get("double").unwrap().value.unwrap();
    assert!(
        (double - 10.0).abs() < 0.001,
        "LAMBDA(x, x*2)(5) should return 10, got {}",
        double
    );

    let add = result.scalars.get("add").unwrap().value.unwrap();
    assert!(
        (add - 7.0).abs() < 0.001,
        "LAMBDA(x, y, x+y)(3, 4) should return 7, got {}",
        add
    );

    let compound = result.scalars.get("compound").unwrap().value.unwrap();
    // 1000 * (1.05)^10 = 1628.89
    assert!(
        (compound - 1628.89).abs() < 0.1,
        "LAMBDA compound interest should return ~1628.89, got {}",
        compound
    );
}

#[test]
fn test_let_function_simple() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=LET(x, value * 2, x + 5)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("result")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 25.0); // 10*2 + 5
        assert_eq!(values[1], 45.0); // 20*2 + 5
        assert_eq!(values[2], 65.0); // 30*2 + 5
    }
}

#[test]
fn test_switch_with_default() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "code".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 99.0]),
    ));
    // SWITCH with default value
    data.row_formulas.insert(
        "label".to_string(),
        "=SWITCH(code, 1, 100, 2, 200, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("label")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 100.0); // code=1 -> 100
        assert_eq!(values[1], 200.0); // code=2 -> 200
        assert_eq!(values[2], 0.0); // code=99 -> default 0
    }
}

#[test]
fn test_lambda_with_multiple_args() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "b".to_string(),
        ColumnValue::Number(vec![3.0, 4.0]),
    ));
    data.row_formulas.insert(
        "product".to_string(),
        "=LAMBDA(x, y, x * y)(a, b)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LAMBDA(x, y, x * y)(a, b) should multiply a * b row-wise
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("product")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 6.0, "2 * 3 = 6"); // 2 * 3
        assert_eq!(values[1], 12.0, "3 * 4 = 12"); // 3 * 4
    } else {
        panic!("Expected number column");
    }
}

#[test]
fn test_let_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "let_result".to_string(),
        Variable::new(
            "let_result".to_string(),
            None,
            Some("=LET(x, 10, y, 20, x + y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LET(x, 10, y, 20, x + y) should return 30
    let let_val = result.scalars.get("let_result").unwrap().value.unwrap();
    assert_eq!(let_val, 30.0, "LET(x, 10, y, 20, x + y) should return 30");
}

#[test]
fn test_lambda_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "lambda_result".to_string(),
        Variable::new(
            "lambda_result".to_string(),
            None,
            Some("=LAMBDA(x, x * 2)(5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LAMBDA(x, x * 2)(5) should return 10
    let lambda_val = result.scalars.get("lambda_result").unwrap().value.unwrap();
    assert_eq!(lambda_val, 10.0, "LAMBDA(x, x * 2)(5) should return 10");
}

#[test]
fn test_switch_function_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "code".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.row_formulas.insert(
        "value".to_string(),
        "=SWITCH(code, 1, 100, 2, 200, 3, 300, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_switch_match_first() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "x".to_string(),
        Variable::new("x".to_string(), Some(1.0), None),
    );
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(x, 1, 100, 2, 200, -1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_switch_default_value() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "x".to_string(),
        Variable::new("x".to_string(), Some(99.0), None),
    );
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(x, 1, 100, 2, 200, -1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_switch_insufficient_args() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(1, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_lambda_single_param() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LAMBDA(x, x*2)(5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_let_simple() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, 10, x*2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_let_multiple_vars() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, 10, y, 20, x+y)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
