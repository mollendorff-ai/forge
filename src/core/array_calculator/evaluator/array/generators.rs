//! Array generator functions: SEQUENCE and RANDARRAY

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};
use rand::Rng;

/// Evaluate SEQUENCE function - generates a sequence of numbers
/// SEQUENCE(rows, [columns], [start], [step])
pub fn eval_sequence(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SEQUENCE", args, 1, 4)?;
    let rows = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("SEQUENCE: rows must be a number"))? as usize;
    let cols = if args.len() > 1 {
        evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
    } else {
        1
    };
    let start = if args.len() > 2 {
        evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0)
    } else {
        1.0
    };
    let step = if args.len() > 3 {
        evaluate(&args[3], ctx)?.as_number().unwrap_or(1.0)
    } else {
        1.0
    };

    // Generate sequence
    let total = rows * cols;
    let values: Vec<Value> = (0..total)
        .map(|i| Value::Number(start + (i as f64) * step))
        .collect();
    Ok(Value::Array(values))
}

/// Evaluate RANDARRAY function - generates an array of random numbers
/// RANDARRAY([rows], [columns], [min], [max], [whole_number])
pub fn eval_randarray(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("RANDARRAY", args, 0, 5)?;
    let rows = if !args.is_empty() {
        evaluate(&args[0], ctx)?.as_number().unwrap_or(1.0) as usize
    } else {
        1
    };
    let cols = if args.len() > 1 {
        evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
    } else {
        1
    };
    let min = if args.len() > 2 {
        evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0)
    } else {
        0.0
    };
    let max = if args.len() > 3 {
        evaluate(&args[3], ctx)?.as_number().unwrap_or(1.0)
    } else {
        1.0
    };
    let whole_number = if args.len() > 4 {
        evaluate(&args[4], ctx)?.is_truthy()
    } else {
        false
    };

    let mut rng = rand::rng();
    let total = rows * cols;
    let values: Vec<Value> = if whole_number {
        (0..total)
            .map(
                |_| Value::Number(rng.random_range(min.floor() as i64..=max.floor() as i64) as f64),
            )
            .collect()
    } else {
        (0..total)
            .map(|_| Value::Number(rng.random_range(min..max)))
            .collect()
    };
    Ok(Value::Array(values))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};

    #[test]
    fn test_sequence() {
        let ctx = EvalContext::new();

        // SEQUENCE(5) = [1, 2, 3, 4, 5]
        let result = eval("SEQUENCE(5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], Value::Number(1.0));
            assert_eq!(arr[4], Value::Number(5.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_sequence_with_params() {
        let ctx = EvalContext::new();

        // SEQUENCE(3, 1, 10, 5) = [10, 15, 20]
        let result = eval("SEQUENCE(3, 1, 10, 5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(10.0));
            assert_eq!(arr[1], Value::Number(15.0));
            assert_eq!(arr[2], Value::Number(20.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_randarray() {
        let ctx = EvalContext::new();

        // RANDARRAY(5) = 5 random numbers between 0 and 1
        let result = eval("RANDARRAY(5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            for v in arr {
                if let Value::Number(n) = v {
                    assert!((0.0..1.0).contains(&n));
                } else {
                    panic!("Expected number");
                }
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_randarray_with_range() {
        let ctx = EvalContext::new();

        // RANDARRAY(3, 1, 1, 10, TRUE()) = 3 whole numbers between 1 and 10
        let result = eval("RANDARRAY(3, 1, 1, 10, TRUE())", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            for v in arr {
                if let Value::Number(n) = v {
                    assert!((1.0..=10.0).contains(&n));
                    assert_eq!(n.fract(), 0.0); // whole number
                } else {
                    panic!("Expected number");
                }
            }
        } else {
            panic!("Expected array");
        }
    }
}

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_sequence_function_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_sum".to_string(),
            Variable::new(
                "seq_sum".to_string(),
                None,
                Some("=SUM(SEQUENCE(5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_val = result.scalars.get("seq_sum").unwrap().value.unwrap();
        assert_eq!(sum_val, 15.0, "SUM(SEQUENCE(5)) should return 15.0");
    }

    #[test]
    fn test_sequence_function_with_start_step() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_custom".to_string(),
            Variable::new(
                "seq_custom".to_string(),
                None,
                Some("=SUM(SEQUENCE(4, 1, 10, 5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_val = result.scalars.get("seq_custom").unwrap().value.unwrap();
        assert_eq!(
            sum_val, 70.0,
            "SUM(SEQUENCE(4, 1, 10, 5)) should return 70.0"
        );
    }

    #[test]
    fn test_randarray_function_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "rand_count".to_string(),
            Variable::new(
                "rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result.scalars.get("rand_count").unwrap().value.unwrap();
        assert_eq!(count_val, 5.0, "COUNT(RANDARRAY(5)) should return 5.0");
    }
}

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_sequence_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(15.0));
    }

    #[test]
    fn test_sequence_count() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(SEQUENCE(10))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_sequence_with_start() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5, 1, 10))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(60.0));
    }

    #[test]
    fn test_sequence_with_step() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5, 1, 1, 2))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(25.0));
    }

    #[test]
    fn test_rows_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=ROWS(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_sequence_function_zero_start() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_max".to_string(),
            Variable::new(
                "seq_max".to_string(),
                None,
                Some("=MAX(SEQUENCE(3, 1, 0, 10))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let max_val = result.scalars.get("seq_max").unwrap().value.unwrap();
        assert_eq!(max_val, 20.0);
    }

    #[test]
    fn test_randarray_function_range() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "rand_count".to_string(),
            Variable::new(
                "rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );
        model.add_scalar(
            "rand_min".to_string(),
            Variable::new(
                "rand_min".to_string(),
                None,
                Some("=MIN(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );
        model.add_scalar(
            "rand_max".to_string(),
            Variable::new(
                "rand_max".to_string(),
                None,
                Some("=MAX(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result.scalars.get("rand_count").unwrap().value.unwrap();
        assert_eq!(count_val, 10.0);

        let min_val = result.scalars.get("rand_min").unwrap().value.unwrap();
        assert!((1.0..=10.0).contains(&min_val));

        let max_val = result.scalars.get("rand_max").unwrap().value.unwrap();
        assert!((1.0..=10.0).contains(&max_val));
    }

    #[test]
    fn test_randarray_function_larger_count() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "large_rand_count".to_string(),
            Variable::new(
                "large_rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(100))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result
            .scalars
            .get("large_rand_count")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(count_val, 100.0);
    }

    #[test]
    fn test_sequence_single() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }
}
