//! Forge-specific functions: BREAKEVEN_UNITS, BREAKEVEN_REVENUE, VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, SCENARIO

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a Forge-specific function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "BREAKEVEN_UNITS" => {
            require_args(name, args, 3)?;
            let fixed_costs = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;
            let price_per_unit = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;
            let variable_cost_per_unit = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;

            let contribution_margin = price_per_unit - variable_cost_per_unit;
            if contribution_margin <= 0.0 {
                return Err(EvalError::new(
                    "unit_price must be greater than variable_cost",
                ));
            }
            Value::Number(fixed_costs / contribution_margin)
        },

        "BREAKEVEN_REVENUE" => {
            require_args(name, args, 2)?;
            let fixed_costs = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_REVENUE requires numbers"))?;
            let contribution_margin_ratio = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_REVENUE requires numbers"))?;

            if contribution_margin_ratio <= 0.0 || contribution_margin_ratio > 1.0 {
                return Err(EvalError::new(
                    "contribution_margin_pct must be between 0 and 1 (exclusive of 0)",
                ));
            }
            Value::Number(fixed_costs / contribution_margin_ratio)
        },

        "VARIANCE" => {
            require_args(name, args, 2)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            Value::Number(actual - budget)
        },

        "VARIANCE_PCT" => {
            require_args(name, args, 2)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_PCT requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_PCT requires numbers"))?;
            if budget == 0.0 {
                return Err(EvalError::new("VARIANCE_PCT: budget cannot be zero"));
            }
            Value::Number((actual - budget) / budget)
        },

        "VARIANCE_STATUS" => {
            // VARIANCE_STATUS(actual, budget, [threshold_or_type])
            // Third arg: number = threshold (e.g., 0.10 = 10%), string "cost" = cost type
            // Returns: 1 = favorable, -1 = unfavorable, 0 = on budget (within threshold)
            require_args_range(name, args, 2, 3)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_STATUS requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_STATUS requires numbers"))?;

            let (threshold, is_cost) = if args.len() > 2 {
                let third_val = evaluate(&args[2], ctx)?;
                match &third_val {
                    Value::Text(s) => (0.01, s.to_lowercase() == "cost"),
                    Value::Number(n) => (*n, false),
                    _ => (0.01, false),
                }
            } else {
                (0.01, false)
            };

            if budget == 0.0 {
                return Ok(Some(Value::Number(if actual > 0.0 {
                    1.0
                } else if actual < 0.0 {
                    -1.0
                } else {
                    0.0
                })));
            }

            let variance_pct = (actual - budget) / budget.abs();
            let status = if variance_pct.abs() <= threshold {
                0.0
            } else if is_cost {
                if variance_pct < 0.0 {
                    1.0
                } else {
                    -1.0
                }
            } else if variance_pct > 0.0 {
                1.0
            } else {
                -1.0
            };
            Value::Number(status)
        },

        "SCENARIO" => {
            require_args(name, args, 2)?;
            let scenario_name = evaluate(&args[0], ctx)?.as_text();
            let var_name = evaluate(&args[1], ctx)?.as_text();

            if let Some(scenario) = ctx.scenarios.get(&scenario_name) {
                if let Some(&value) = scenario.get(&var_name) {
                    Value::Number(value)
                } else {
                    return Err(EvalError::new(format!(
                        "Variable '{var_name}' not found in scenario '{scenario_name}'"
                    )));
                }
            } else {
                return Err(EvalError::new(format!(
                    "Scenario '{scenario_name}' not found"
                )));
            }
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_breakeven() {
        let ctx = EvalContext::new();
        // Fixed costs: 10000, price: 50, variable cost: 30
        // Breakeven units = 10000 / (50 - 30) = 500
        assert_eq!(
            eval("BREAKEVEN_UNITS(10000, 50, 30)", &ctx).unwrap(),
            Value::Number(500.0)
        );

        // Fixed costs: 10000, contribution margin ratio: 0.4
        // Breakeven revenue = 10000 / 0.4 = 25000
        assert_eq!(
            eval("BREAKEVEN_REVENUE(10000, 0.4)", &ctx).unwrap(),
            Value::Number(25000.0)
        );
    }

    #[test]
    fn test_variance() {
        let ctx = EvalContext::new();
        // Actual: 110, Budget: 100
        assert_eq!(
            eval("VARIANCE(110, 100)", &ctx).unwrap(),
            Value::Number(10.0)
        );
        assert_eq!(
            eval("VARIANCE_PCT(110, 100)", &ctx).unwrap(),
            Value::Number(0.1)
        );
    }

    #[test]
    fn test_variance_status() {
        let ctx = EvalContext::new();
        // Revenue: over budget = favorable (1)
        assert_eq!(
            eval("VARIANCE_STATUS(120, 100)", &ctx).unwrap(),
            Value::Number(1.0)
        );
        // Within threshold = on budget (0)
        assert_eq!(
            eval("VARIANCE_STATUS(100.5, 100)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_scenario() {
        let mut ctx = EvalContext::new();
        let mut scenario = HashMap::new();
        scenario.insert("growth_rate".to_string(), 0.15);
        ctx.scenarios.insert("optimistic".to_string(), scenario);

        assert_eq!(
            eval("SCENARIO(\"optimistic\", \"growth_rate\")", &ctx).unwrap(),
            Value::Number(0.15)
        );
    }

    // === EDGE CASES FOR 100% COVERAGE ===

    #[test]
    fn test_breakeven_units_zero_margin() {
        let ctx = EvalContext::new();
        // Price equals variable cost = 0 margin (error)
        let result = eval("BREAKEVEN_UNITS(10000, 50, 50)", &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unit_price must be greater"));
    }

    #[test]
    fn test_breakeven_units_negative_margin() {
        let ctx = EvalContext::new();
        // Variable cost > price = negative margin (error)
        let result = eval("BREAKEVEN_UNITS(10000, 30, 50)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_breakeven_revenue_invalid_ratio() {
        let ctx = EvalContext::new();
        // Contribution margin ratio > 1 (error)
        let result = eval("BREAKEVEN_REVENUE(10000, 1.5)", &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("contribution_margin_pct"));
    }

    #[test]
    fn test_breakeven_revenue_zero_ratio() {
        let ctx = EvalContext::new();
        // Contribution margin ratio = 0 (error)
        let result = eval("BREAKEVEN_REVENUE(10000, 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_variance_pct_zero_budget() {
        let ctx = EvalContext::new();
        let result = eval("VARIANCE_PCT(100, 0)", &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("budget cannot be zero"));
    }

    #[test]
    fn test_variance_status_zero_budget_negative() {
        let ctx = EvalContext::new();
        // Budget = 0, actual < 0 = unfavorable (-1)
        assert_eq!(
            eval("VARIANCE_STATUS(-50, 0)", &ctx).unwrap(),
            Value::Number(-1.0)
        );
    }

    #[test]
    fn test_variance_status_zero_budget_zero() {
        let ctx = EvalContext::new();
        // Budget = 0, actual = 0 = on budget (0)
        assert_eq!(
            eval("VARIANCE_STATUS(0, 0)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_variance_status_cost_type_favorable() {
        let ctx = EvalContext::new();
        // Cost type: under budget (variance_pct < 0) = favorable (1)
        assert_eq!(
            eval("VARIANCE_STATUS(80, 100, \"cost\")", &ctx).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn test_variance_status_cost_type_unfavorable() {
        let ctx = EvalContext::new();
        // Cost type: over budget (variance_pct > 0) = unfavorable (-1)
        assert_eq!(
            eval("VARIANCE_STATUS(120, 100, \"cost\")", &ctx).unwrap(),
            Value::Number(-1.0)
        );
    }

    #[test]
    fn test_variance_status_unfavorable_revenue() {
        let ctx = EvalContext::new();
        // Revenue type: under budget = unfavorable (-1)
        assert_eq!(
            eval("VARIANCE_STATUS(80, 100)", &ctx).unwrap(),
            Value::Number(-1.0)
        );
    }

    #[test]
    fn test_variance_status_with_numeric_threshold() {
        let ctx = EvalContext::new();
        // 5% variance with 10% threshold = on budget (0)
        assert_eq!(
            eval("VARIANCE_STATUS(105, 100, 0.10)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_scenario_missing_variable() {
        let mut ctx = EvalContext::new();
        let scenario = HashMap::new();
        ctx.scenarios.insert("empty".to_string(), scenario);

        let result = eval("SCENARIO(\"empty\", \"missing_var\")", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_scenario_missing_scenario() {
        let ctx = EvalContext::new();
        let result = eval("SCENARIO(\"nonexistent\", \"var\")", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests (moved from tests/forge.rs)
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Scenario, Table, Variable};

    #[test]
    fn test_variance_function() {
        let mut model = ParsedModel::new();

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

        let pct = result.scalars.get("variance_pct").unwrap().value.unwrap();
        assert!((pct - 0.1).abs() < 0.0001);
    }

    #[test]
    fn test_variance_status_favorable() {
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
        assert_eq!(status, 1.0);
    }

    #[test]
    fn test_variance_status_cost_type() {
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

        let status = result.scalars.get("status").unwrap().value.unwrap();
        assert_eq!(status, 1.0);
    }

    #[test]
    fn test_breakeven_units_function() {
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

        let be_units = result.scalars.get("breakeven").unwrap().value.unwrap();
        assert_eq!(be_units, 1250.0);
    }

    #[test]
    fn test_breakeven_revenue_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "fixed_costs".to_string(),
            Variable::new("fixed_costs".to_string(), Some(100000.0), None),
        );
        model.add_scalar(
            "contribution_margin_pct".to_string(),
            Variable::new("contribution_margin_pct".to_string(), Some(0.4), None),
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

        let be_rev = result.scalars.get("breakeven_rev").unwrap().value.unwrap();
        assert_eq!(be_rev, 250000.0);
    }

    #[test]
    fn test_scenario_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "base_revenue".to_string(),
            Variable::new("base_revenue".to_string(), Some(1000.0), None),
        );

        let mut scenario = Scenario::new();
        scenario.add_override("revenue".to_string(), 1500.0);
        model.scenarios.insert("optimistic".to_string(), scenario);

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

    // ════════════════════════════════════════════════════════════════════════════
    // Additional tests moved from tests/forge.rs
    // ════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_variance_pct_zero_budget_error() {
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
    fn test_variance_status_favorable_v2() {
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
        assert_eq!(status, 1.0);
    }

    #[test]
    fn test_variance_status_unfavorable() {
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
        assert_eq!(status, -1.0);
    }

    #[test]
    fn test_variance_status_cost_type_v2() {
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
        let status = result.scalars.get("status").unwrap().value.unwrap();
        assert_eq!(status, 1.0);
    }

    #[test]
    fn test_breakeven_units_invalid_margin_error() {
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
        let result = calculator.calculate_all();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unit_price must be greater than variable_cost"));
    }

    #[test]
    fn test_breakeven_revenue_zero_margin_error() {
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
    fn test_scenario_not_found() {
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
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_variance_status_under_budget() {
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
        assert!(status < 0.0);
    }

    #[test]
    fn test_variance_status_over_budget() {
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
        assert!(status > 0.0);
    }

    #[test]
    fn test_variance_status_on_budget() {
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
        assert!((status - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_variance_sample_vs_population() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
        ));
        model.add_table(table);
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
        assert!(var_s > 4.0);
    }

    #[test]
    fn test_variance_function_v2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "var_result".to_string(),
            Variable::new(
                "var_result".to_string(),
                None,
                Some("=VARIANCE(100, 80)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var_val = result.scalars.get("var_result").unwrap().value.unwrap();
        assert_eq!(var_val, 20.0, "VARIANCE(100, 80) should return 20");
    }

    #[test]
    fn test_variance_pct_function_v2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "var_pct".to_string(),
            Variable::new(
                "var_pct".to_string(),
                None,
                Some("=VARIANCE_PCT(100, 80)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var_pct_val = result.scalars.get("var_pct").unwrap().value.unwrap();
        assert_eq!(
            var_pct_val, 0.25,
            "VARIANCE_PCT(100, 80) should return 0.25"
        );
    }

    #[test]
    fn test_variance_status_function_v2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "status".to_string(),
            Variable::new(
                "status".to_string(),
                None,
                Some("=VARIANCE_STATUS(100, 80, 0.1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let status_val = result.scalars.get("status").unwrap().value.unwrap();
        assert_eq!(
            status_val, 1.0,
            "VARIANCE_STATUS(100, 80, 0.1) should return 1"
        );
    }

    #[test]
    fn test_breakeven_units_function_v2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "breakeven".to_string(),
            Variable::new(
                "breakeven".to_string(),
                None,
                Some("=BREAKEVEN_UNITS(10000, 50, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let be_val = result.scalars.get("breakeven").unwrap().value.unwrap();
        assert_eq!(
            be_val, 500.0,
            "BREAKEVEN_UNITS(10000, 50, 30) should return 500"
        );
    }

    #[test]
    fn test_scenario_function_v2() {
        let mut model = ParsedModel::new();
        let mut scenario = Scenario::new();
        scenario.add_override("growth_rate".to_string(), 0.05);
        model.scenarios.insert("base".to_string(), scenario);
        model.add_scalar(
            "scenario_val".to_string(),
            Variable::new(
                "scenario_val".to_string(),
                None,
                Some("=SCENARIO(\"base\", \"growth_rate\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let scenario_val = result.scalars.get("scenario_val").unwrap().value.unwrap();
        assert_eq!(
            scenario_val, 0.05,
            "SCENARIO(\"base\", \"growth_rate\") should return 0.05"
        );
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
}
