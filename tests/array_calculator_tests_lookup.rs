// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::ParsedModel;

#[test]
fn test_choose_function() {
    use royalbit_forge::types::Variable;

    // Test CHOOSE function for scenario modeling
    let mut model = ParsedModel::new();

    // Add scalar for scenario index
    model.scalars.insert(
        "inputs.scenario_index".to_string(),
        Variable::new("inputs.scenario_index".to_string(), Some(2.0), None),
    );

    // Add scalar with CHOOSE formula
    model.scalars.insert(
        "outputs.scenario_value".to_string(),
        Variable::new(
            "outputs.scenario_value".to_string(),
            None,
            Some("=CHOOSE(inputs.scenario_index, 100, 200, 300)".to_string()),
        ),
    );

    // Add scalar with literal CHOOSE
    model.scalars.insert(
        "outputs.literal_choose".to_string(),
        Variable::new(
            "outputs.literal_choose".to_string(),
            None,
            Some("=CHOOSE(1, 10, 20, 30)".to_string()),
        ),
    );

    // Add scalar with expression CHOOSE
    model.scalars.insert(
        "outputs.expression_choose".to_string(),
        Variable::new(
            "outputs.expression_choose".to_string(),
            None,
            Some("=CHOOSE(1+2, 5, 10, 15)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("CHOOSE calculation should succeed");

    // Check scenario_value (index=2 should return 200)
    let scenario_value = result.scalars.get("outputs.scenario_value").unwrap();
    assert!(
        (scenario_value.value.unwrap() - 200.0).abs() < 0.0001,
        "CHOOSE(2, 100, 200, 300) should return 200, got {}",
        scenario_value.value.unwrap()
    );

    // Check literal_choose (index=1 should return 10)
    let literal_choose = result.scalars.get("outputs.literal_choose").unwrap();
    assert!(
        (literal_choose.value.unwrap() - 10.0).abs() < 0.0001,
        "CHOOSE(1, 10, 20, 30) should return 10, got {}",
        literal_choose.value.unwrap()
    );

    // Check expression_choose (index=1+2=3 should return 15)
    let expression_choose = result.scalars.get("outputs.expression_choose").unwrap();
    assert!(
        (expression_choose.value.unwrap() - 15.0).abs() < 0.0001,
        "CHOOSE(1+2, 5, 10, 15) should return 15, got {}",
        expression_choose.value.unwrap()
    );

    println!("✓ CHOOSE function test passed");
}

#[test]
fn test_choose_first_option() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=CHOOSE(1, 100, 200, 300)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 100.0).abs() < 0.0001);
    println!("✓ CHOOSE first option edge case passed");
}

#[test]
fn test_choose_last_option() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=CHOOSE(3, 100, 200, 300)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 300.0).abs() < 0.0001);
    println!("✓ CHOOSE last option edge case passed");
}
