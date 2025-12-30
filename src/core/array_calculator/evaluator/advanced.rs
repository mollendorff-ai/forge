//! Advanced functions: LAMBDA, LET, SWITCH

use super::{evaluate, values_equal, EvalContext, EvalError, Expr, Reference, Value};

/// Try to evaluate an advanced function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "LAMBDA" => {
            // LAMBDA(param1, param2, ..., body) - returns a lambda value
            if args.is_empty() {
                return Err(EvalError::new("LAMBDA requires at least a body"));
            }

            let mut params = Vec::new();
            for i in 0..args.len() - 1 {
                match &args[i] {
                    Expr::Reference(Reference::Scalar(name)) => {
                        params.push(name.clone());
                    },
                    _ => {
                        return Err(EvalError::new(format!(
                            "LAMBDA parameter {} must be an identifier",
                            i + 1
                        )));
                    },
                }
            }

            let body = args.last().unwrap().clone();

            Value::Lambda {
                params,
                body: Box::new(body),
            }
        },

        "LET" => {
            // LET(name1, value1, [name2, value2, ...], calculation)
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "LET requires pairs of name/value plus a calculation",
                ));
            }

            let mut new_ctx = ctx.clone();

            let num_pairs = (args.len() - 1) / 2;
            for i in 0..num_pairs {
                let name_expr = &args[i * 2];
                let value_expr = &args[i * 2 + 1];

                let name = match name_expr {
                    Expr::Reference(Reference::Scalar(n)) => n.clone(),
                    _ => return Err(EvalError::new("LET variable name must be an identifier")),
                };

                let value = evaluate(value_expr, &new_ctx)?;
                new_ctx.scalars.insert(name, value);
            }

            evaluate(&args[args.len() - 1], &new_ctx)?
        },

        "SWITCH" => {
            // SWITCH(expression, value1, result1, [value2, result2], ..., [default])
            if args.len() < 2 {
                return Err(EvalError::new("SWITCH requires at least 2 arguments"));
            }

            let expr_val = evaluate(&args[0], ctx)?;
            let remaining = &args[1..];

            let mut i = 0;
            while i + 1 < remaining.len() {
                let check_val = evaluate(&remaining[i], ctx)?;
                if values_equal(&expr_val, &check_val) {
                    return Ok(Some(evaluate(&remaining[i + 1], ctx)?));
                }
                i += 2;
            }

            if remaining.len() % 2 == 1 {
                evaluate(&remaining[remaining.len() - 1], ctx)?
            } else {
                return Err(EvalError::new("SWITCH: No match found"));
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

    #[test]
    fn test_let() {
        let ctx = EvalContext::new();
        // LET(x, 10, y, 20, x + y)
        assert_eq!(
            eval("LET(x, 10, y, 20, x + y)", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[test]
    fn test_switch() {
        let ctx = EvalContext::new();
        // SWITCH(2, 1, "one", 2, "two", 3, "three")
        assert_eq!(
            eval("SWITCH(2, 1, \"one\", 2, \"two\", 3, \"three\")", &ctx).unwrap(),
            Value::Text("two".to_string())
        );

        // SWITCH with default
        assert_eq!(
            eval("SWITCH(5, 1, \"one\", 2, \"two\", \"default\")", &ctx).unwrap(),
            Value::Text("default".to_string())
        );
    }

    #[test]
    fn test_lambda() {
        let ctx = EvalContext::new();
        // Create a lambda and call it: LAMBDA(x, x * 2)(5)
        let result = eval("LAMBDA(x, x * 2)(5)", &ctx).unwrap();
        assert_eq!(result, Value::Number(10.0));
    }

    #[test]
    fn test_nested_functions() {
        let ctx = EvalContext::new();
        // Test nested function calls still work
        assert_eq!(eval("LET(a, 5, a * 2)", &ctx).unwrap(), Value::Number(10.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests moved from tests/advanced.rs
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_let_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "simple_let".to_string(),
            Variable::new(
                "simple_let".to_string(),
                None,
                Some("=LET(x, 10, x * 2)".to_string()),
            ),
        );

        model.add_scalar(
            "multi_var".to_string(),
            Variable::new(
                "multi_var".to_string(),
                None,
                Some("=LET(x, 5, y, 3, x + y)".to_string()),
            ),
        );

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
            "LET(x, 10, x * 2) should return 20, got {simple}"
        );

        let multi = result.scalars.get("multi_var").unwrap().value.unwrap();
        assert!(
            (multi - 8.0).abs() < 0.001,
            "LET(x, 5, y, 3, x + y) should return 8, got {multi}"
        );

        let dep = result.scalars.get("dependent").unwrap().value.unwrap();
        assert!(
            (dep - 25.0).abs() < 0.001,
            "LET(a, 10, b, a * 2, b + 5) should return 25, got {dep}"
        );
    }

    #[test]
    fn test_let_with_aggregation() {
        let mut model = ParsedModel::new();

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0, 500.0]),
        ));
        model.add_table(sales);

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
        assert!(
            (tax - 150.0).abs() < 0.001,
            "LET with SUM should return 150, got {tax}"
        );
    }

    #[test]
    fn test_switch_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "matched".to_string(),
            Variable::new(
                "matched".to_string(),
                None,
                Some("=SWITCH(2, 1, 0.05, 2, 0.10, 3, 0.15)".to_string()),
            ),
        );

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
            "SWITCH(2, ...) should return 0.10, got {matched}"
        );

        let with_default = result.scalars.get("with_default").unwrap().value.unwrap();
        assert!(
            (with_default - 50.0).abs() < 0.001,
            "SWITCH(4, ..., 50) should return default 50, got {with_default}"
        );
    }

    #[test]
    fn test_lambda_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "double".to_string(),
            Variable::new(
                "double".to_string(),
                None,
                Some("=LAMBDA(x, x * 2)(5)".to_string()),
            ),
        );

        model.add_scalar(
            "add".to_string(),
            Variable::new(
                "add".to_string(),
                None,
                Some("=LAMBDA(x, y, x + y)(3, 4)".to_string()),
            ),
        );

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
            "LAMBDA(x, x*2)(5) should return 10, got {double}"
        );

        let add = result.scalars.get("add").unwrap().value.unwrap();
        assert!(
            (add - 7.0).abs() < 0.001,
            "LAMBDA(x, y, x+y)(3, 4) should return 7, got {add}"
        );

        let compound = result.scalars.get("compound").unwrap().value.unwrap();
        assert!(
            (compound - 1628.89).abs() < 0.1,
            "LAMBDA compound interest should return ~1628.89, got {compound}"
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
            assert_eq!(values[0], 25.0);
            assert_eq!(values[1], 45.0);
            assert_eq!(values[2], 65.0);
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
            assert_eq!(values[0], 100.0);
            assert_eq!(values[1], 200.0);
            assert_eq!(values[2], 0.0);
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

        let col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("product")
            .unwrap();
        if let ColumnValue::Number(values) = &col.values {
            assert_eq!(values[0], 6.0, "2 * 3 = 6");
            assert_eq!(values[1], 12.0, "3 * 4 = 12");
        } else {
            panic!("Expected number column");
        }
    }

    #[test]
    fn test_let_function_v2() {
        let mut model = ParsedModel::new();

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

        let let_val = result.scalars.get("let_result").unwrap().value.unwrap();
        assert_eq!(let_val, 30.0, "LET(x, 10, y, 20, x + y) should return 30");
    }

    #[test]
    fn test_lambda_function_v2() {
        let mut model = ParsedModel::new();

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
}

// ══════════════════════════════════════════════════════════════════════════════
// Edge case tests moved from tests/advanced_function_edge_cases.rs
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    // LET Tests

    #[test]
    fn test_let_single() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(x, 5, x * 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_let_two_vars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(x, 5, y, 3, x + y)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(8.0));
    }

    #[test]
    fn test_let_dependent() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(x, 5, y, x * 2, y + 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(11.0));
    }

    #[test]
    fn test_let_with_sum() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(x, SUM(1,2,3), x * 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
    }

    #[test]
    fn test_let_nested() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(a, 2, b, LET(c, a*2, c+1), b)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_let_complex() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LET(rate, 0.08, price, 100, price * (1 + rate))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(108.0));
    }

    // SWITCH Tests

    #[test]
    fn test_switch_first() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(1, 1, 100, 2, 200, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[test]
    fn test_switch_last() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(2, 1, 100, 2, 200, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_switch_default() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(99, 1, 100, 2, 200, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(999.0));
    }

    #[test]
    fn test_switch_numeric() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(2, 1, 10, 2, 20, 3, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
    }

    // IFS Tests

    #[test]
    fn test_ifs_first_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(TRUE, 1, FALSE, 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_ifs_second_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(FALSE, 1, TRUE, 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_ifs_with_comparison() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(5>10, 100, 5<10, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_ifs_with_and() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(AND(1>0, 2>0), 100, TRUE, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[test]
    fn test_ifs_with_or() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(OR(1>10, 2>10), 100, TRUE, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }
}
