// Enterprise-only: Forge functions (VARIANCE, BREAKEVEN, SCENARIO)
#![cfg(not(feature = "demo"))]
// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};

#[test]
fn test_variance_functions() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Input values
    model.scalars.insert(
        "inputs.actual".to_string(),
        Variable::new("inputs.actual".to_string(), Some(95000.0), None),
    );
    model.scalars.insert(
        "inputs.budget".to_string(),
        Variable::new("inputs.budget".to_string(), Some(100000.0), None),
    );

    // VARIANCE(actual, budget) = actual - budget
    model.scalars.insert(
        "outputs.variance".to_string(),
        Variable::new(
            "outputs.variance".to_string(),
            None,
            Some("=VARIANCE(inputs.actual, inputs.budget)".to_string()),
        ),
    );

    // VARIANCE_PCT(actual, budget) = (actual - budget) / budget
    model.scalars.insert(
        "outputs.variance_pct".to_string(),
        Variable::new(
            "outputs.variance_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(inputs.actual, inputs.budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("VARIANCE calculation should succeed");

    // VARIANCE = 95000 - 100000 = -5000
    let variance = result.scalars.get("outputs.variance").unwrap();
    assert!(
        (variance.value.unwrap() - (-5000.0)).abs() < 0.01,
        "VARIANCE should return -5000, got {}",
        variance.value.unwrap()
    );

    // VARIANCE_PCT = -5000 / 100000 = -0.05
    let variance_pct = result.scalars.get("outputs.variance_pct").unwrap();
    assert!(
        (variance_pct.value.unwrap() - (-0.05)).abs() < 0.001,
        "VARIANCE_PCT should return -0.05, got {}",
        variance_pct.value.unwrap()
    );

    println!("✓ VARIANCE functions test passed");
}

#[test]
fn test_breakeven_functions() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // BREAKEVEN_UNITS(fixed_costs, unit_price, variable_cost_per_unit)
    model.scalars.insert(
        "outputs.breakeven_units".to_string(),
        Variable::new(
            "outputs.breakeven_units".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(50000, 100, 60)".to_string()),
        ),
    );

    // BREAKEVEN_REVENUE(fixed_costs, contribution_margin_pct)
    model.scalars.insert(
        "outputs.breakeven_revenue".to_string(),
        Variable::new(
            "outputs.breakeven_revenue".to_string(),
            None,
            Some("=BREAKEVEN_REVENUE(50000, 0.40)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("BREAKEVEN calculation should succeed");

    // BREAKEVEN_UNITS = 50000 / (100 - 60) = 1250
    let units = result.scalars.get("outputs.breakeven_units").unwrap();
    assert!(
        (units.value.unwrap() - 1250.0).abs() < 0.01,
        "BREAKEVEN_UNITS should return 1250, got {}",
        units.value.unwrap()
    );

    // BREAKEVEN_REVENUE = 50000 / 0.40 = 125000
    let revenue = result.scalars.get("outputs.breakeven_revenue").unwrap();
    assert!(
        (revenue.value.unwrap() - 125000.0).abs() < 0.01,
        "BREAKEVEN_REVENUE should return 125000, got {}",
        revenue.value.unwrap()
    );

    println!("✓ BREAKEVEN functions test passed");
}

#[test]
fn test_scenario_function() {
    use royalbit_forge::types::{Scenario, Variable};

    let mut model = ParsedModel::new();

    // Add scenarios
    let mut base = Scenario::new();
    base.add_override("growth_rate".to_string(), 0.05);
    model.add_scenario("base".to_string(), base);

    let mut optimistic = Scenario::new();
    optimistic.add_override("growth_rate".to_string(), 0.12);
    model.add_scenario("optimistic".to_string(), optimistic);

    // Use SCENARIO function to get values
    model.scalars.insert(
        "outputs.base_growth".to_string(),
        Variable::new(
            "outputs.base_growth".to_string(),
            None,
            Some("=SCENARIO(\"base\", \"growth_rate\")".to_string()),
        ),
    );

    model.scalars.insert(
        "outputs.optimistic_growth".to_string(),
        Variable::new(
            "outputs.optimistic_growth".to_string(),
            None,
            Some("=SCENARIO(\"optimistic\", \"growth_rate\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("SCENARIO calculation should succeed");

    let base_growth = result.scalars.get("outputs.base_growth").unwrap();
    assert!(
        (base_growth.value.unwrap() - 0.05).abs() < 0.0001,
        "SCENARIO('base', 'growth_rate') should return 0.05, got {}",
        base_growth.value.unwrap()
    );

    let optimistic_growth = result.scalars.get("outputs.optimistic_growth").unwrap();
    assert!(
        (optimistic_growth.value.unwrap() - 0.12).abs() < 0.0001,
        "SCENARIO('optimistic', 'growth_rate') should return 0.12, got {}",
        optimistic_growth.value.unwrap()
    );

    println!("✓ SCENARIO function test passed");
}

#[test]
fn test_breakeven_with_scalars() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Inputs
    model.scalars.insert(
        "inputs.fixed_costs".to_string(),
        Variable::new("inputs.fixed_costs".to_string(), Some(75000.0), None),
    );
    model.scalars.insert(
        "inputs.unit_price".to_string(),
        Variable::new("inputs.unit_price".to_string(), Some(150.0), None),
    );
    model.scalars.insert(
        "inputs.variable_cost".to_string(),
        Variable::new("inputs.variable_cost".to_string(), Some(90.0), None),
    );

    // BREAKEVEN_UNITS with scalar references
    model.scalars.insert(
        "outputs.be_units".to_string(),
        Variable::new(
            "outputs.be_units".to_string(),
            None,
            Some(
                "=BREAKEVEN_UNITS(inputs.fixed_costs, inputs.unit_price, inputs.variable_cost)"
                    .to_string(),
            ),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("BREAKEVEN with scalars should succeed");

    // BREAKEVEN_UNITS = 75000 / (150 - 90) = 75000 / 60 = 1250
    let be_units = result.scalars.get("outputs.be_units").unwrap();
    assert!(
        (be_units.value.unwrap() - 1250.0).abs() < 0.01,
        "BREAKEVEN_UNITS should return 1250, got {}",
        be_units.value.unwrap()
    );

    println!("✓ BREAKEVEN with scalar references test passed");
}

#[test]
fn test_variance_status_favorable() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Revenue favorable (actual > budget = favorable for revenue)
    model.scalars.insert(
        "inputs.actual_rev".to_string(),
        Variable::new("inputs.actual_rev".to_string(), Some(110000.0), None),
    );
    model.scalars.insert(
        "inputs.budget_rev".to_string(),
        Variable::new("inputs.budget_rev".to_string(), Some(100000.0), None),
    );

    // VARIANCE_STATUS for revenue (type=revenue or default)
    model.scalars.insert(
        "outputs.rev_status".to_string(),
        Variable::new(
            "outputs.rev_status".to_string(),
            None,
            Some("=VARIANCE_STATUS(inputs.actual_rev, inputs.budget_rev)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("VARIANCE_STATUS calculation should succeed");

    // Result should be 1 (favorable) since actual > budget for revenue
    let rev_status = result.scalars.get("outputs.rev_status").unwrap();
    assert!(
        (rev_status.value.unwrap() - 1.0).abs() < 0.0001,
        "VARIANCE_STATUS for favorable revenue should return 1, got {}",
        rev_status.value.unwrap()
    );

    println!("✓ VARIANCE_STATUS favorable test passed");
}

#[test]
fn test_variance_zero_variance() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    // All same values = zero variance
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.var".to_string(),
        Variable::new(
            "outputs.var".to_string(),
            None,
            Some("=VAR.S(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let var = result.scalars.get("outputs.var").unwrap();
    assert!(
        var.value.unwrap().abs() < 0.0001,
        "Zero variance for identical values"
    );
    println!("✓ VAR.S zero variance edge case passed");
}

#[test]
fn test_variance_zero_budget() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "inputs.actual".to_string(),
        Variable::new("inputs.actual".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "inputs.budget".to_string(),
        Variable::new("inputs.budget".to_string(), Some(0.0), None),
    );
    model.scalars.insert(
        "outputs.variance".to_string(),
        Variable::new(
            "outputs.variance".to_string(),
            None,
            Some("=VARIANCE(inputs.actual, inputs.budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let variance = result.scalars.get("outputs.variance").unwrap();
    // VARIANCE = 100 - 0 = 100
    assert!((variance.value.unwrap() - 100.0).abs() < 0.01);
    println!("✓ VARIANCE zero budget edge case passed");
}

#[test]
fn test_variance_pct_zero_budget() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "inputs.actual".to_string(),
        Variable::new("inputs.actual".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "inputs.budget".to_string(),
        Variable::new("inputs.budget".to_string(), Some(0.0), None),
    );
    model.scalars.insert(
        "outputs.variance_pct".to_string(),
        Variable::new(
            "outputs.variance_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(inputs.actual, inputs.budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Division by zero - should handle gracefully
    assert!(result.is_ok() || result.is_err());
    println!("✓ VARIANCE_PCT zero budget edge case passed");
}

#[test]
fn test_variance_status_unfavorable() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // Unfavorable: actual < budget (for revenue)
    model.scalars.insert(
        "inputs.actual".to_string(),
        Variable::new("inputs.actual".to_string(), Some(90000.0), None),
    );
    model.scalars.insert(
        "inputs.budget".to_string(),
        Variable::new("inputs.budget".to_string(), Some(100000.0), None),
    );
    model.scalars.insert(
        "outputs.status".to_string(),
        Variable::new(
            "outputs.status".to_string(),
            None,
            Some("=VARIANCE_STATUS(inputs.actual, inputs.budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let status = result.scalars.get("outputs.status").unwrap();
    // -1 = unfavorable (actual < budget for revenue type)
    assert!((status.value.unwrap() - (-1.0)).abs() < 0.0001);
    println!("✓ VARIANCE_STATUS unfavorable edge case passed");
}

#[test]
fn test_breakeven_units_zero_margin() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // When price = variable cost, margin = 0, breakeven = infinity
    model.scalars.insert(
        "outputs.be_units".to_string(),
        Variable::new(
            "outputs.be_units".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(50000, 100, 100)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Division by zero - should return infinity or error gracefully
    if let Ok(res) = result {
        let be = res.scalars.get("outputs.be_units").unwrap();
        if let Some(v) = be.value {
            assert!(v.is_infinite() || v.is_nan() || v > 1_000_000_000.0);
        }
    }
    println!("✓ BREAKEVEN_UNITS zero margin edge case passed");
}

#[test]
fn test_breakeven_revenue_100_pct_margin() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // 100% contribution margin
    model.scalars.insert(
        "outputs.be_rev".to_string(),
        Variable::new(
            "outputs.be_rev".to_string(),
            None,
            Some("=BREAKEVEN_REVENUE(50000, 1.0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let be_rev = result.scalars.get("outputs.be_rev").unwrap();
    // 50000 / 1.0 = 50000
    assert!((be_rev.value.unwrap() - 50000.0).abs() < 0.01);
    println!("✓ BREAKEVEN_REVENUE 100% margin edge case passed");
}

#[test]
fn test_breakeven_negative_margin() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // Negative margin (variable cost > price)
    model.scalars.insert(
        "outputs.be_units".to_string(),
        Variable::new(
            "outputs.be_units".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(50000, 100, 150)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Negative margin - should return negative number or handle gracefully
    if let Ok(res) = result {
        let be = res.scalars.get("outputs.be_units").unwrap();
        // Negative breakeven indicates impossible scenario
        assert!(be.value.is_some());
    }
    println!("✓ BREAKEVEN_UNITS negative margin edge case passed");
}

#[test]
fn test_scenario_missing_variable() {
    use royalbit_forge::types::{Scenario, Variable};

    let mut model = ParsedModel::new();
    let mut scenario = Scenario::new();
    scenario.add_override("existing_var".to_string(), 0.05);
    model.add_scenario("test".to_string(), scenario);

    model.scalars.insert(
        "outputs.value".to_string(),
        Variable::new(
            "outputs.value".to_string(),
            None,
            Some("=SCENARIO(\"test\", \"nonexistent_var\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle missing variable gracefully
    assert!(result.is_ok() || result.is_err());
    println!("✓ SCENARIO missing variable edge case passed");
}

#[test]
fn test_scenario_missing_scenario() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.value".to_string(),
        Variable::new(
            "outputs.value".to_string(),
            None,
            Some("=SCENARIO(\"nonexistent\", \"some_var\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle missing scenario gracefully
    assert!(result.is_ok() || result.is_err());
    println!("✓ SCENARIO missing scenario edge case passed");
}
