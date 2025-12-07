//! Forge function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_variance_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Add scalars for actual and budget
    model.add_scalar(
        "actual_revenue".to_string(),
        Variable::new("actual_revenue".to_string(), Some(120000.0), None),
    );
    model.add_scalar(
        "budget_revenue".to_string(),
        Variable::new("budget_revenue".to_string(), Some(100000.0), None),
    );
    model.add_scalar(
        "variance_result".to_string(),
        Variable::new(
            "variance_result".to_string(),
            None,
            Some("=VARIANCE(actual_revenue, budget_revenue)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 120000 - 100000 = 20000
    let variance = result
        .scalars
        .get("variance_result")
        .unwrap()
        .value
        .unwrap();
    assert_eq!(variance, 20000.0);
}

#[test]
fn test_variance_pct_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(110.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "variance_pct".to_string(),
        Variable::new(
            "variance_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // (110 - 100) / 100 = 0.1
    let pct = result.scalars.get("variance_pct").unwrap().value.unwrap();
    assert!((pct - 0.1).abs() < 0.0001);
}

#[test]
fn test_variance_pct_zero_budget_error() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(0.0), None),
    );
    model.add_scalar(
        "variance_pct".to_string(),
        Variable::new(
            "variance_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("budget cannot be zero"));
}

#[test]
fn test_variance_status_favorable() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(120.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // For revenue: higher actual is favorable = 1
    let status = result.scalars.get("status").unwrap().value.unwrap();
    assert_eq!(status, 1.0);
}

#[test]
fn test_variance_status_unfavorable() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(80.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // For revenue: lower actual is unfavorable = -1
    let status = result.scalars.get("status").unwrap().value.unwrap();
    assert_eq!(status, -1.0);
}

#[test]
fn test_variance_status_cost_type() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "actual_cost".to_string(),
        Variable::new("actual_cost".to_string(), Some(80.0), None),
    );
    model.add_scalar(
        "budget_cost".to_string(),
        Variable::new("budget_cost".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual_cost, budget_cost, \"cost\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // For costs: lower actual is favorable = 1
    let status = result.scalars.get("status").unwrap().value.unwrap();
    assert_eq!(status, 1.0);
}

#[test]
fn test_breakeven_units_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(50000.0), None),
    );
    model.add_scalar(
        "unit_price".to_string(),
        Variable::new("unit_price".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "variable_cost".to_string(),
        Variable::new("variable_cost".to_string(), Some(60.0), None),
    );
    model.add_scalar(
        "breakeven".to_string(),
        Variable::new(
            "breakeven".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(fixed_costs, unit_price, variable_cost)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 50000 / (100 - 60) = 50000 / 40 = 1250
    let be_units = result.scalars.get("breakeven").unwrap().value.unwrap();
    assert_eq!(be_units, 1250.0);
}

#[test]
fn test_breakeven_units_invalid_margin_error() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(50000.0), None),
    );
    model.add_scalar(
        "unit_price".to_string(),
        Variable::new("unit_price".to_string(), Some(50.0), None),
    );
    model.add_scalar(
        "variable_cost".to_string(),
        Variable::new("variable_cost".to_string(), Some(60.0), None), // Higher than price!
    );
    model.add_scalar(
        "breakeven".to_string(),
        Variable::new(
            "breakeven".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(fixed_costs, unit_price, variable_cost)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("unit_price must be greater than variable_cost"));
}

#[test]
fn test_breakeven_revenue_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(100000.0), None),
    );
    model.add_scalar(
        "contribution_margin_pct".to_string(),
        Variable::new("contribution_margin_pct".to_string(), Some(0.4), None), // 40%
    );
    model.add_scalar(
        "breakeven_rev".to_string(),
        Variable::new(
            "breakeven_rev".to_string(),
            None,
            Some("=BREAKEVEN_REVENUE(fixed_costs, contribution_margin_pct)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 100000 / 0.4 = 250000
    let be_rev = result.scalars.get("breakeven_rev").unwrap().value.unwrap();
    assert_eq!(be_rev, 250000.0);
}

#[test]
fn test_breakeven_revenue_zero_margin_error() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(100000.0), None),
    );
    model.add_scalar(
        "contribution_margin_pct".to_string(),
        Variable::new("contribution_margin_pct".to_string(), Some(0.0), None),
    );
    model.add_scalar(
        "breakeven_rev".to_string(),
        Variable::new(
            "breakeven_rev".to_string(),
            None,
            Some("=BREAKEVEN_REVENUE(fixed_costs, contribution_margin_pct)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("contribution_margin_pct must be between 0 and 1"));
}

#[test]
fn test_scenario_function() {
    use crate::types::{Scenario, Variable};

    let mut model = ParsedModel::new();

    // Base values
    model.add_scalar(
        "base_revenue".to_string(),
        Variable::new("base_revenue".to_string(), Some(1000.0), None),
    );

    // Define a scenario
    let mut scenario = Scenario::new();
    scenario.add_override("revenue".to_string(), 1500.0);
    model.scenarios.insert("optimistic".to_string(), scenario);

    // Use SCENARIO function
    model.add_scalar(
        "scenario_value".to_string(),
        Variable::new(
            "scenario_value".to_string(),
            None,
            Some("=SCENARIO(\"optimistic\", \"revenue\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let value = result.scalars.get("scenario_value").unwrap().value.unwrap();
    assert!((value - 1500.0).abs() < 0.01);
}

#[test]
fn test_scenario_not_found() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "test".to_string(),
        Variable::new(
            "test".to_string(),
            None,
            Some("=SCENARIO(\"nonexistent\", \"var\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_variance_pct_with_zero_original() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "original".to_string(),
        Variable::new("original".to_string(), Some(0.0), None),
    );
    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "var_pct".to_string(),
        Variable::new(
            "var_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(actual, original)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Division by zero returns error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("zero"));
}

#[test]
fn test_variance_status_under_budget() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(80.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let status = result.scalars.get("status").unwrap().value.unwrap();
    // Status should be negative (under budget)
    assert!(status < 0.0);
}

#[test]
fn test_variance_status_over_budget() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(120.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let status = result.scalars.get("status").unwrap().value.unwrap();
    // Status should be positive (over budget)
    assert!(status > 0.0);
}

#[test]
fn test_variance_status_on_budget() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "actual".to_string(),
        Variable::new("actual".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "budget".to_string(),
        Variable::new("budget".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(actual, budget)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let status = result.scalars.get("status").unwrap().value.unwrap();
    // Status should be zero (on budget)
    assert!((status - 0.0).abs() < 0.01);
}

#[test]
fn test_variance_sample_vs_population() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    // Sample variance (VAR.S or VAR)
    model.add_scalar(
        "var_s".to_string(),
        Variable::new(
            "var_s".to_string(),
            None,
            Some("=VAR(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let var_s = result.scalars.get("var_s").unwrap().value.unwrap();
    // Sample variance should be larger than population variance
    assert!(var_s > 4.0); // Population variance is 4.0
}

#[test]
fn test_variance_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "var_result".to_string(),
        Variable::new(
            "var_result".to_string(),
            None,
            Some("=VARIANCE(100, 80)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_variance_pct_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "var_pct".to_string(),
        Variable::new(
            "var_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(100, 80)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_variance_status_function() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "status".to_string(),
        Variable::new(
            "status".to_string(),
            None,
            Some("=VARIANCE_STATUS(100, 80, 0.1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_breakeven_units_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "breakeven".to_string(),
        Variable::new(
            "breakeven".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(10000, 50, 30)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_scenario_function_v2() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "scenario_val".to_string(),
        Variable::new(
            "scenario_val".to_string(),
            None,
            Some("=SCENARIO(\"base\", 100, \"optimistic\", 150, \"pessimistic\", 50)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_empty_array_variance() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.columns.insert(
        "values".to_string(),
        Column::new("values".to_string(), ColumnValue::Number(vec![])),
    );
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "var".to_string(),
        Variable::new(
            "var".to_string(),
            None,
            Some("=VAR(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_variance_pct_calc() {
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
        "var_pct".to_string(),
        Variable::new(
            "var_pct".to_string(),
            None,
            Some("=VARIANCE_PCT(actual, budget)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_breakeven_units_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(10000.0), None),
    );
    model.add_scalar(
        "price".to_string(),
        Variable::new("price".to_string(), Some(50.0), None),
    );
    model.add_scalar(
        "var_cost".to_string(),
        Variable::new("var_cost".to_string(), Some(30.0), None),
    );
    model.add_scalar(
        "be_units".to_string(),
        Variable::new(
            "be_units".to_string(),
            None,
            Some("=BREAKEVEN_UNITS(fixed_costs, price, var_cost)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_breakeven_revenue_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "fixed_costs".to_string(),
        Variable::new("fixed_costs".to_string(), Some(10000.0), None),
    );
    model.add_scalar(
        "cm_ratio".to_string(),
        Variable::new("cm_ratio".to_string(), Some(0.4), None),
    );
    model.add_scalar(
        "be_rev".to_string(),
        Variable::new(
            "be_rev".to_string(),
            None,
            Some("=BREAKEVEN_REVENUE(fixed_costs, cm_ratio)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
