//! Logical functions: IF, AND, OR, NOT, XOR, TRUE, FALSE, IFERROR, IFNA, IFS
//!
//! DEMO functions (5): IF, AND, OR, NOT, IFERROR
//! ENTERPRISE functions: IFNA, XOR, TRUE, FALSE, IFS

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a logical function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "IF" => {
            require_args_range(name, args, 2, 3)?;
            let condition = evaluate(&args[0], ctx)?;
            if condition.is_truthy() {
                evaluate(&args[1], ctx)?
            } else if args.len() > 2 {
                evaluate(&args[2], ctx)?
            } else {
                Value::Boolean(false)
            }
        },

        "AND" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if !val.is_truthy() {
                    return Ok(Some(Value::Boolean(false)));
                }
            }
            Value::Boolean(true)
        },

        "OR" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    return Ok(Some(Value::Boolean(true)));
                }
            }
            Value::Boolean(false)
        },

        "NOT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Boolean(!val.is_truthy())
        },

        "IFERROR" => {
            require_args(name, args, 2)?;
            match evaluate(&args[0], ctx) {
                Ok(val) => val,
                Err(_) => evaluate(&args[1], ctx)?,
            }
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "IFNA" => {
            require_args(name, args, 2)?;
            let val = evaluate(&args[0], ctx)?;
            // In Excel, IFNA returns value_if_na when the result is #N/A
            // Since we don't have a proper NA error type, treat Null as NA
            if matches!(val, Value::Null) {
                evaluate(&args[1], ctx)?
            } else {
                val
            }
        },

        #[cfg(not(feature = "demo"))]
        "XOR" => {
            // XOR returns TRUE if an odd number of arguments are TRUE
            let mut true_count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    true_count += 1;
                }
            }
            Value::Boolean(true_count % 2 == 1)
        },

        #[cfg(not(feature = "demo"))]
        "TRUE" => {
            require_args(name, args, 0)?;
            Value::Boolean(true)
        },

        #[cfg(not(feature = "demo"))]
        "FALSE" => {
            require_args(name, args, 0)?;
            Value::Boolean(false)
        },

        #[cfg(not(feature = "demo"))]
        "IFS" => {
            // IFS(condition1, value1, condition2, value2, ...)
            // Returns the value corresponding to the first TRUE condition
            if args.is_empty() || !args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "IFS requires an even number of arguments (condition, value pairs)",
                ));
            }
            for pair in args.chunks(2) {
                let condition = evaluate(&pair[0], ctx)?;
                if condition.is_truthy() {
                    return Ok(Some(evaluate(&pair[1], ctx)?));
                }
            }
            return Err(EvalError::new(
                "IFS: No matching condition found (consider adding TRUE as final condition)",
            ));
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
    fn test_if() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("IF(5 > 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("yes".to_string())
        );
        assert_eq!(
            eval("IF(5 < 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("no".to_string())
        );
    }

    #[test]
    fn test_logical() {
        let ctx = EvalContext::new();
        assert_eq!(eval("AND(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("AND(1, 0, 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("OR(0, 0, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_logical_and_edge_cases() {
        let ctx = EvalContext::new();
        // AND(TRUE, TRUE) = TRUE
        assert_eq!(
            eval("AND(TRUE(), TRUE())", &ctx).unwrap_or(eval("AND(1, 1)", &ctx).unwrap()),
            Value::Boolean(true)
        );
        // AND(TRUE, FALSE) = FALSE
        assert_eq!(eval("AND(1, 0)", &ctx).unwrap(), Value::Boolean(false));
        // AND(1, 1) = TRUE (nonzero as true)
        assert_eq!(eval("AND(1, 1)", &ctx).unwrap(), Value::Boolean(true));
        // AND(1, 0) = FALSE (zero as false)
        assert_eq!(eval("AND(1, 0)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_logical_or_edge_cases() {
        let ctx = EvalContext::new();
        // OR(FALSE, FALSE) = FALSE
        assert_eq!(eval("OR(0, 0)", &ctx).unwrap(), Value::Boolean(false));
        // OR(TRUE, FALSE) = TRUE
        assert_eq!(eval("OR(1, 0)", &ctx).unwrap(), Value::Boolean(true));
        // OR(0, 1) = TRUE
        assert_eq!(eval("OR(0, 1)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_logical_not_edge_cases() {
        let ctx = EvalContext::new();
        // NOT(FALSE) = TRUE
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
        // NOT(TRUE) = FALSE
        assert_eq!(eval("NOT(1)", &ctx).unwrap(), Value::Boolean(false));
        // NOT(0) = TRUE
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
        // NOT(1) = FALSE
        assert_eq!(eval("NOT(1)", &ctx).unwrap(), Value::Boolean(false));
        // NOT(5) = FALSE (any nonzero is truthy)
        assert_eq!(eval("NOT(5)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_if_edge_cases() {
        let ctx = EvalContext::new();
        // IF(2, 100, 200) = 100 (nonzero condition)
        assert_eq!(eval("IF(2, 100, 200)", &ctx).unwrap(), Value::Number(100.0));
        // IF(0, 100, 200) = 200 (zero condition)
        assert_eq!(eval("IF(0, 100, 200)", &ctx).unwrap(), Value::Number(200.0));
    }

    #[test]
    fn test_iferror() {
        let ctx = EvalContext::new();
        // Division by zero returns the fallback value
        assert_eq!(eval("IFERROR(1/0, 0)", &ctx).unwrap(), Value::Number(0.0));
        // No error returns the original value
        assert_eq!(eval("IFERROR(10/2, 0)", &ctx).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_iferror_edge_cases_comprehensive() {
        let ctx = EvalContext::new();

        // Edge case 1: IFERROR(1/0, -1) = -1 (div by zero caught)
        assert_eq!(eval("IFERROR(1/0, -1)", &ctx).unwrap(), Value::Number(-1.0));

        // Edge case 2: IFERROR(5, -1) = 5 (no error)
        assert_eq!(eval("IFERROR(5, -1)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 3: IFERROR(10/2, -1) = 5 (division ok)
        assert_eq!(eval("IFERROR(10/2, -1)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 6: IFERROR(SQRT(-1), -1) = -1 (sqrt negative caught)
        assert_eq!(
            eval("IFERROR(SQRT(-1), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 7: IFERROR(LOG10(0), -1) = -1 (log zero caught)
        assert_eq!(
            eval("IFERROR(LOG10(0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 8: IFERROR(LN(0), -1) = -1 (ln zero caught)
        assert_eq!(
            eval("IFERROR(LN(0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 12: IFERROR(MOD(5, 0), -1) = -1 (mod by zero caught)
        assert_eq!(
            eval("IFERROR(MOD(5, 0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );
    }

    #[test]
    fn test_iferror_nested_edge_case() {
        let ctx = EvalContext::new();

        // Edge case 9: IFERROR(IFERROR(1/0, 1/0), -99) = -99 (nested)
        // Inner IFERROR tries to catch 1/0 but fallback is also 1/0 (error)
        // Outer IFERROR catches that error and returns -99
        assert_eq!(
            eval("IFERROR(IFERROR(1/0, 1/0), -99)", &ctx).unwrap(),
            Value::Number(-99.0)
        );

        // Additional nested test with valid fallback
        assert_eq!(
            eval("IFERROR(IFERROR(1/0, 42), 99)", &ctx).unwrap(),
            Value::Number(42.0)
        );
    }

    #[test]
    fn test_if_lazy_evaluation() {
        let ctx = EvalContext::new();

        // Edge case 4: IF(FALSE, 1/0, 5) = 5 (lazy eval - false branch not evaluated)
        assert_eq!(eval("IF(FALSE, 1/0, 5)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 5: IF(TRUE, 10, 1/0) = 10 (lazy eval - false branch not evaluated)
        assert_eq!(
            eval("IF(TRUE, 10, 1/0)", &ctx).unwrap(),
            Value::Number(10.0)
        );

        // Additional lazy eval test: IF(0, SQRT(-1), 100) = 100
        // True branch (error) should not be evaluated when condition is false
        assert_eq!(
            eval("IF(0, SQRT(-1), 100)", &ctx).unwrap(),
            Value::Number(100.0)
        );

        // Additional lazy eval test: IF(1, 200, MOD(5, 0)) = 200
        // False branch (error) should not be evaluated when condition is true
        assert_eq!(
            eval("IF(1, 200, MOD(5, 0))", &ctx).unwrap(),
            Value::Number(200.0)
        );

        // Additional lazy eval test: IF(1, LN(0), 300) = error (true branch is evaluated and has error)
        let result = eval("IF(1, LN(0), 300)", &ctx);
        assert!(result.is_err());

        // Additional lazy eval test: IF(0, 400, LOG10(0)) = error (false branch is evaluated and has error)
        let result = eval("IF(0, 400, LOG10(0))", &ctx);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor() {
        let ctx = EvalContext::new();
        // XOR returns TRUE if odd number of TRUE values
        assert_eq!(eval("XOR(1, 0, 0)", &ctx).unwrap(), Value::Boolean(true)); // 1 true
        assert_eq!(eval("XOR(1, 1, 0)", &ctx).unwrap(), Value::Boolean(false)); // 2 true
        assert_eq!(eval("XOR(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true)); // 3 true
        assert_eq!(eval("XOR(0, 0, 0)", &ctx).unwrap(), Value::Boolean(false)); // 0 true
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_true_false() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TRUE()", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("FALSE()", &ctx).unwrap(), Value::Boolean(false));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifs() {
        let ctx = EvalContext::new();
        // First matching condition returns its value
        assert_eq!(
            eval(
                "IFS(5>10, \"big\", 5>3, \"medium\", TRUE(), \"small\")",
                &ctx
            )
            .unwrap(),
            Value::Text("medium".to_string())
        );
        // First condition matches
        assert_eq!(
            eval("IFS(1, \"first\", 1, \"second\")", &ctx).unwrap(),
            Value::Text("first".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifs_no_match() {
        let ctx = EvalContext::new();
        // No matching condition returns error
        let result = eval("IFS(0, \"no\", 0, \"nope\")", &ctx);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifs_invalid_args() {
        let ctx = EvalContext::new();
        // Odd number of args is invalid
        let result = eval("IFS(1, \"yes\", 0)", &ctx);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifna() {
        let mut ctx = EvalContext::new();
        ctx.scalars.insert("valid".to_string(), Value::Number(10.0));
        // Non-null value returns the value
        assert_eq!(eval("IFNA(valid, 0)", &ctx).unwrap(), Value::Number(10.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests moved from tests/logical.rs
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_if_simple_condition() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, -5.0, 20.0]),
        ));
        data.row_formulas
            .insert("positive".to_string(), "=IF(value > 0, 1, 0)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("positive")
            .unwrap();
        if let ColumnValue::Number(values) = &col.values {
            assert_eq!(values[0], 1.0);
            assert_eq!(values[1], 0.0);
            assert_eq!(values[2], 1.0);
        }
    }

    #[test]
    fn test_cross_table_column_not_found_error() {
        let mut model = ParsedModel::new();

        let mut table1 = Table::new("table1".to_string());
        table1.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        model.add_table(table1);

        let mut table2 = Table::new("table2".to_string());
        table2.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table2
            .row_formulas
            .insert("result".to_string(), "=table1.nonexistent + x".to_string());
        model.add_table(table2);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_table_table_not_found_error() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=nonexistent_table.column + x".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_local_column_not_found_error() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=nonexistent_column + x".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_if_with_cross_table_reference() {
        let mut model = ParsedModel::new();

        let mut thresholds = Table::new("thresholds".to_string());
        thresholds.add_column(Column::new(
            "min".to_string(),
            ColumnValue::Number(vec![50.0]),
        ));
        model.add_table(thresholds);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![30.0, 60.0, 45.0]),
        ));
        data.row_formulas.insert(
            "above_min".to_string(),
            "=IF(value > SUM(thresholds.min), 1, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        assert!(
            result.is_err(),
            "Aggregation in row formula should error (invalid usage)"
        );
    }

    #[test]
    fn test_cross_table_column_not_found_error_v2() {
        let mut model = ParsedModel::new();

        let mut source = Table::new("source".to_string());
        source.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        model.add_table(source);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=source.nonexistent + x".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_table_table_not_found_error_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=nonexistent.column + x".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_local_column_not_found_error_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=nonexistent + x".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_if_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "score".to_string(),
            ColumnValue::Number(vec![45.0, 65.0, 85.0]),
        ));
        data.row_formulas.insert(
            "grade".to_string(),
            "=IF(score >= 80, 1, IF(score >= 60, 2, 3))".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let grade_col = table.columns.get("grade").unwrap();
        if let ColumnValue::Number(vals) = &grade_col.values {
            assert_eq!(vals[0], 3.0);
            assert_eq!(vals[1], 2.0);
            assert_eq!(vals[2], 1.0);
        } else {
            panic!("Expected Number column");
        }
    }

    #[test]
    fn test_and_or_functions() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Boolean(vec![true, true, false]),
        ));
        data.add_column(Column::new(
            "b".to_string(),
            ColumnValue::Boolean(vec![true, false, false]),
        ));
        data.row_formulas
            .insert("and_result".to_string(), "=IF(AND(a, b), 1, 0)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let and_col = table.columns.get("and_result").unwrap();
        if let ColumnValue::Number(vals) = &and_col.values {
            assert_eq!(vals[0], 1.0);
            assert_eq!(vals[1], 0.0);
            assert_eq!(vals[2], 0.0);
        } else {
            panic!("Expected Number column");
        }
    }

    #[test]
    fn test_not_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "flag".to_string(),
            ColumnValue::Boolean(vec![true, false]),
        ));
        data.row_formulas
            .insert("inverted".to_string(), "=IF(NOT(flag), 1, 0)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let inverted_col = table.columns.get("inverted").unwrap();
        if let ColumnValue::Number(vals) = &inverted_col.values {
            assert_eq!(vals[0], 0.0);
            assert_eq!(vals[1], 1.0);
        } else {
            panic!("Expected Number column");
        }
    }

    #[test]
    fn test_iferror_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "numerator".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        data.add_column(Column::new(
            "denominator".to_string(),
            ColumnValue::Number(vec![2.0, 0.0]),
        ));
        data.row_formulas.insert(
            "safe_div".to_string(),
            "=IFERROR(numerator / denominator, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let safe_div_col = table.columns.get("safe_div").unwrap();
        if let ColumnValue::Number(vals) = &safe_div_col.values {
            assert_eq!(vals[0], 5.0);
            assert_eq!(vals[1], 0.0);
        } else {
            panic!("Expected Number column");
        }
    }

    #[test]
    fn test_iferror_no_error() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFERROR(10/2, -1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_rowwise_if_formula() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        data.add_row_formula("status".to_string(), "=IF(value > 25, 1, 0)".to_string());
        model.add_table(data);
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_and_all_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(1>0, 2>0, 3>0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    fn test_and_one_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(1>0, 0>1, 2>0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_and_with_numbers() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(1, 2, 3), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    fn test_or_all_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(0>1, 0>2, 0>3), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_or_one_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(0>1, 1>0, 0>2), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    fn test_or_with_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(0, 0, 0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_not_true_integration() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(1>0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_not_false_integration() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(0>1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    fn test_not_with_number() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_true_via_comparison() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(1>0, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    fn test_false_via_comparison() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(0>1, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_complex_logical_expression() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(OR(1>0, 0>1), NOT(0>1)), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor_one_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(XOR(1>0, 0>1, 0>1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor_two_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(XOR(1>0, 2>1, 0>1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor_three_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(XOR(1>0, 2>1, 3>2), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor_all_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(XOR(0>1, 0>2, 0>3), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor_with_numbers() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(XOR(1, 0, 0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_true_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(TRUE(), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_false_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(FALSE(), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_true_in_and() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(TRUE(), 1>0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_false_in_or() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(FALSE(), 1>0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_not_with_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(TRUE()), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_not_with_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(FALSE()), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifna_with_value() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFNA(10+5, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(15.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifna_with_text() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(IFNA(\"test\", \"error\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(4.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifna_with_table_reference() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);

        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFNA(SUM(data.value), 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(60.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_combined_xor_and_not() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(XOR(TRUE(), TRUE())), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_true_false_in_arithmetic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(TRUE(), 1, 0) + IF(FALSE(), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Edge case tests moved from tests/logical_edge_cases.rs
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_and_true_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(TRUE, TRUE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_and_true_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(TRUE, FALSE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_and_nonzero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(1, 1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_and_with_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(AND(1, 0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_or_false_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(FALSE, FALSE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_or_true_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(TRUE, FALSE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_or_zero_one() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(OR(0, 1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_not_false_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(FALSE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_not_true_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(TRUE), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_not_zero_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(0), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_not_one_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(1), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_not_nonzero_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(NOT(5), 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_if_nonzero_condition() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(2, 100, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[test]
    fn test_if_zero_condition() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(0, 100, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_sum_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(1, 2, 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    #[test]
    fn test_average_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=AVERAGE(2, 4, 6)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    #[test]
    fn test_min_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MIN(5, 3, 8, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_max_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MAX(5, 3, 8, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(8.0));
    }

    #[test]
    fn test_count_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(1, 2, 3, 4, 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_sum_empty_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SUM()".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Comparison edge case tests moved from tests/comparison_edge_cases.rs
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod comparison_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_true_gt_false() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(TRUE > FALSE, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_true_eq_one() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(TRUE = 1, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_false_eq_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(FALSE = 0, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_string_exact_match() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(\"ABC\" = \"ABC\", 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_string_case_insensitive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(\"ABC\" = \"abc\", 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_float_eq_int() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(1.0 = 1, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_gt_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(10 > 9, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_gte_equal() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(10 >= 10, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_lt_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(10 < 11, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_lte_equal() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(10 <= 10, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_not_equal() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(10 <> 9, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_floating_point_rounded() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(ROUND(0.1 + 0.2, 1) = 0.3, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_floating_point_precision() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IF(0.1 + 0.2 = 0.3, 1, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }
}
