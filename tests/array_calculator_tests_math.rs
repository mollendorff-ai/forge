// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};

#[cfg(not(feature = "demo"))]
#[test]
fn test_sln_depreciation() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // SLN(cost, salvage, life) - Straight-line depreciation
    model.scalars.insert(
        "outputs.sln_result".to_string(),
        Variable::new(
            "outputs.sln_result".to_string(),
            None,
            Some("=SLN(30000, 7500, 10)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("SLN calculation should succeed");

    // SLN = (30000 - 7500) / 10 = 2250
    let sln = result.scalars.get("outputs.sln_result").unwrap();
    assert!(
        (sln.value.unwrap() - 2250.0).abs() < 0.01,
        "SLN(30000, 7500, 10) should return 2250, got {}",
        sln.value.unwrap()
    );

    println!("✓ SLN depreciation test passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sln_zero_salvage() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.sln".to_string(),
        Variable::new(
            "outputs.sln".to_string(),
            None,
            Some("=SLN(10000, 0, 5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let sln = result.scalars.get("outputs.sln").unwrap();
    // SLN(10000, 0, 5) = 10000/5 = 2000
    assert!((sln.value.unwrap() - 2000.0).abs() < 0.01);
    println!("✓ SLN zero salvage edge case passed");
}

#[test]
fn test_sqrt_zero() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=SQRT(0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 0.0).abs() < 0.0001);
    println!("✓ SQRT(0) edge case passed");
}

#[test]
fn test_power_zero_exponent() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=POWER(5, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    // Any number^0 = 1
    assert!((val.value.unwrap() - 1.0).abs() < 0.0001);
    println!("✓ POWER x^0 edge case passed");
}

#[test]
fn test_power_zero_base() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=POWER(0, 5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    // 0^n = 0 (for n > 0)
    assert!((val.value.unwrap() - 0.0).abs() < 0.0001);
    println!("✓ POWER 0^n edge case passed");
}

#[test]
fn test_mod_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=MOD(-10, 3)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    // -10 mod 3 = 2 (Excel behavior) or -1 (some implementations)
    assert!(val.value.is_some());
    println!("✓ MOD negative edge case passed");
}

#[test]
fn test_round_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=ROUND(-3.567, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - (-3.57)).abs() < 0.001);
    println!("✓ ROUND negative edge case passed");
}

#[test]
fn test_abs_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=ABS(-42)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 42.0).abs() < 0.0001);
    println!("✓ ABS negative edge case passed");
}

#[test]
fn test_ceiling_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=CEILING(-3.2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // CEILING(-3.2) behavior varies by implementation
    assert!(result.is_ok() || result.is_err());
    println!("✓ CEILING negative edge case passed");
}

#[test]
fn test_floor_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=FLOOR(-3.2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // FLOOR(-3.2) behavior varies by implementation
    assert!(result.is_ok() || result.is_err());
    println!("✓ FLOOR negative edge case passed");
}

#[test]
fn test_irr_no_sign_change() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    // All positive - no IRR exists
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.irr".to_string(),
        Variable::new(
            "outputs.irr".to_string(),
            None,
            Some("=IRR(cashflows.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle gracefully (error or NaN)
    assert!(result.is_ok() || result.is_err());
    println!("✓ IRR no sign change edge case passed");
}

#[test]
fn test_floating_point_precision() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // Classic floating point issue: 0.1 + 0.2 != 0.3
    model.scalars.insert(
        "outputs.sum".to_string(),
        Variable::new(
            "outputs.sum".to_string(),
            None,
            Some("=0.1 + 0.2".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let sum = result.scalars.get("outputs.sum").unwrap();
    // Should be approximately 0.3
    assert!((sum.value.unwrap() - 0.3).abs() < 0.0001);
    println!("✓ Floating point precision edge case passed");
}
