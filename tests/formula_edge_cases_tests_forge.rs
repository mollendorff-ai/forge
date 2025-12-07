//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{ParsedModel, Variable};

// Helper to create a variable with formula
#[allow(dead_code)]
fn var_formula(path: &str, formula: &str) -> Variable {
    Variable::new(path.to_string(), None, Some(formula.to_string()))
}

// Helper to create a variable with value
#[allow(dead_code)]
fn var_value(path: &str, value: f64) -> Variable {
    Variable::new(path.to_string(), Some(value), None)
}

// ═══════════════════════════════════════════════════════════════════════════
// DATE FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_variance_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.actual".to_string(),
        var_value("metrics.actual", 100000.0),
    );
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 120000.0),
    );
    model.scalars.insert(
        "metrics.variance_result".to_string(),
        var_formula(
            "metrics.variance_result",
            "=VARIANCE(metrics.actual, metrics.budget)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let var = result.scalars.get("metrics.variance_result").unwrap();
    assert_eq!(var.value, Some(-20000.0));
}

#[test]
fn test_variance_pct_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.actual".to_string(),
        var_value("metrics.actual", 100000.0),
    );
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 120000.0),
    );
    model.scalars.insert(
        "metrics.variance_pct".to_string(),
        var_formula(
            "metrics.variance_pct",
            "=VARIANCE_PCT(metrics.actual, metrics.budget)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let var_pct = result.scalars.get("metrics.variance_pct").unwrap();
    // (100000 - 120000) / 120000 = -0.1667
    assert!(var_pct.value.unwrap() < -0.16);
    assert!(var_pct.value.unwrap() > -0.17);
}

#[test]
fn test_variance_status_under() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.actual".to_string(),
        var_value("metrics.actual", 100000.0),
    );
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 120000.0),
    );
    model.scalars.insert(
        "metrics.status".to_string(),
        var_formula(
            "metrics.status",
            "=VARIANCE_STATUS(metrics.actual, metrics.budget, 0.10)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("metrics.status").unwrap();
    assert_eq!(s.value, Some(-1.0)); // Under budget
}

#[test]
fn test_variance_status_on_target() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 120000.0),
    );
    // VARIANCE_STATUS uses 0.1% threshold internally - 119990 is within 0.1% of 120000
    model.scalars.insert(
        "metrics.status".to_string(),
        var_formula("metrics.status", "=VARIANCE_STATUS(120010, metrics.budget)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("metrics.status").unwrap();
    assert_eq!(s.value, Some(0.0)); // On target (within 0.1%)
}

#[test]
fn test_variance_status_over() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 120000.0),
    );
    model.scalars.insert(
        "metrics.status".to_string(),
        var_formula(
            "metrics.status",
            "=VARIANCE_STATUS(150000, metrics.budget, 0.10)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("metrics.status").unwrap();
    assert_eq!(s.value, Some(1.0)); // Over budget
}

#[test]
fn test_breakeven_units() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "breakeven.fixed_costs".to_string(),
        var_value("breakeven.fixed_costs", 50000.0),
    );
    model.scalars.insert(
        "breakeven.price".to_string(),
        var_value("breakeven.price", 100.0),
    );
    model.scalars.insert(
        "breakeven.variable_cost".to_string(),
        var_value("breakeven.variable_cost", 60.0),
    );
    model.scalars.insert(
        "breakeven.units".to_string(),
        var_formula(
            "breakeven.units",
            "=BREAKEVEN_UNITS(breakeven.fixed_costs, breakeven.price, breakeven.variable_cost)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let u = result.scalars.get("breakeven.units").unwrap();
    assert_eq!(u.value, Some(1250.0)); // 50000 / (100 - 60) = 1250
}

#[test]
fn test_breakeven_revenue() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "breakeven.fixed_costs".to_string(),
        var_value("breakeven.fixed_costs", 50000.0),
    );
    // contribution_margin_pct = (price - variable_cost) / price = (100 - 60) / 100 = 0.40
    model.scalars.insert(
        "breakeven.margin_pct".to_string(),
        var_value("breakeven.margin_pct", 0.40),
    );
    model.scalars.insert(
        "breakeven.revenue".to_string(),
        var_formula(
            "breakeven.revenue",
            "=BREAKEVEN_REVENUE(breakeven.fixed_costs, breakeven.margin_pct)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let r = result.scalars.get("breakeven.revenue").unwrap();
    assert_eq!(r.value, Some(125000.0)); // 50000 / 0.40 = 125000
}

#[test]
fn test_variance_pct_zero_budget_error() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "metrics.actual".to_string(),
        var_value("metrics.actual", 100.0),
    );
    model.scalars.insert(
        "metrics.budget".to_string(),
        var_value("metrics.budget", 0.0),
    );
    model.scalars.insert(
        "metrics.pct".to_string(),
        var_formula(
            "metrics.pct",
            "=VARIANCE_PCT(metrics.actual, metrics.budget)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err()); // budget cannot be zero
}

#[test]
fn test_breakeven_zero_margin_error() {
    let mut model = ParsedModel::new();
    model
        .scalars
        .insert("costs.fixed".to_string(), var_value("costs.fixed", 50000.0));
    model
        .scalars
        .insert("costs.price".to_string(), var_value("costs.price", 100.0));
    model.scalars.insert(
        "costs.variable".to_string(),
        var_value("costs.variable", 100.0),
    ); // Same as price = 0 margin
    model.scalars.insert(
        "costs.breakeven".to_string(),
        var_formula(
            "costs.breakeven",
            "=BREAKEVEN_UNITS(costs.fixed, costs.price, costs.variable)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err()); // margin cannot be zero
}

#[test]
fn test_variance_with_cost_type() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "costs.budget".to_string(),
        var_value("costs.budget", 100000.0),
    );
    // Lower actual cost is favorable for cost type
    model.scalars.insert(
        "costs.status".to_string(),
        var_formula(
            "costs.status",
            "=VARIANCE_STATUS(80000, costs.budget, \"cost\")",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("costs.status").unwrap();
    assert_eq!(s.value, Some(1.0)); // Favorable (under budget for costs)
}
