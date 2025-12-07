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
        }

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
        }

        "VARIANCE" => {
            require_args(name, args, 2)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            Value::Number(actual - budget)
        }

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
        }

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
        }

        "SCENARIO" => {
            require_args(name, args, 2)?;
            let scenario_name = evaluate(&args[0], ctx)?.as_text();
            let var_name = evaluate(&args[1], ctx)?.as_text();

            if let Some(scenario) = ctx.scenarios.get(&scenario_name) {
                if let Some(&value) = scenario.get(&var_name) {
                    Value::Number(value)
                } else {
                    return Err(EvalError::new(format!(
                        "Variable '{}' not found in scenario '{}'",
                        var_name, scenario_name
                    )));
                }
            } else {
                return Err(EvalError::new(format!(
                    "Scenario '{}' not found",
                    scenario_name
                )));
            }
        }

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
}
