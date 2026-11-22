//! Array generator functions: SEQUENCE and RANDARRAY

// A place in the array, and a random whole number, become numbers.

use crate::core::array_calculator::evaluator::number_or;
use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};
use crate::core::precision;
use num_traits::ToPrimitive;
use rand::RngExt;

/// The most values an array SEQUENCE or RANDARRAY makes holds: the rows of a
/// sheet.
const MOST_VALUES: usize = 1_048_576;

/// How many values an array of `rows` by `cols` holds, each cut to a whole
/// number as Excel cuts them. None is #CALC! in Excel (an empty array); a
/// negative count is #VALUE!, as is one past [`MOST_VALUES`], which no sheet
/// could show.
fn array_size(name: &str, rows: f64, cols: f64) -> Result<usize, EvalError> {
    let size = rows
        .to_usize()
        .zip(cols.to_usize())
        .and_then(|(r, c)| r.checked_mul(c))
        .filter(|&n| n <= MOST_VALUES);
    match size {
        Some(0) => Err(EvalError::new(format!(
            "#CALC!: {name}: {rows} rows by {cols} columns is an empty array"
        ))),
        Some(n) => Ok(n),
        None => Err(EvalError::new(format!(
            "#VALUE!: {name}: {rows} rows by {cols} columns must be from 1 to 1,048,576 values"
        ))),
    }
}

/// Evaluate SEQUENCE function - generates a sequence of numbers
/// SEQUENCE(rows, [columns], [start], [step])
pub fn eval_sequence(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SEQUENCE", args, 1, 4)?;
    let rows = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("SEQUENCE: rows must be a number"))?;
    let cols = if args.len() > 1 {
        number_or(&args[1], ctx, 1.0)?
    } else {
        1.0
    };
    let start = if args.len() > 2 {
        number_or(&args[2], ctx, 1.0)?
    } else {
        1.0
    };
    let step = if args.len() > 3 {
        number_or(&args[3], ctx, 1.0)?
    } else {
        1.0
    };

    // Generate sequence
    let total = array_size("SEQUENCE", rows, cols)?;
    let values: Vec<Value> = (0..total)
        .map(|i| Value::Number(precision::count(i).mul_add(step, start)))
        .collect();
    Ok(Value::Array(values.into()))
}

/// Evaluate RANDARRAY function - generates an array of random numbers
/// RANDARRAY([rows], [columns], [min], [max], [`whole_number`])
pub fn eval_randarray(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("RANDARRAY", args, 0, 5)?;
    let rows = if args.is_empty() {
        1.0
    } else {
        number_or(&args[0], ctx, 1.0)?
    };
    let cols = if args.len() > 1 {
        number_or(&args[1], ctx, 1.0)?
    } else {
        1.0
    };
    let min = if args.len() > 2 {
        number_or(&args[2], ctx, 0.0)?
    } else {
        0.0
    };
    let max = if args.len() > 3 {
        number_or(&args[3], ctx, 1.0)?
    } else {
        1.0
    };
    let whole_number = if args.len() > 4 {
        evaluate(&args[4], ctx)?.is_truthy()
    } else {
        false
    };

    let total = array_size("RANDARRAY", rows, cols)?;
    // Excel: #VALUE! for a minimum above the maximum (or one that is no number)
    if min.is_nan() || max.is_nan() || min > max {
        return Err(EvalError::new(format!(
            "#VALUE!: RANDARRAY: the minimum {min} is above the maximum {max}"
        )));
    }
    let mut rng = rand::rng();
    let values: Vec<Value> = if whole_number {
        // Whole numbers from min to max, each rounded down
        let Some((low, high)) = min.floor().to_i64().zip(max.floor().to_i64()) else {
            return Err(EvalError::new(format!(
                "#NUM!: RANDARRAY: whole numbers from {min} to {max} are past what it counts"
            )));
        };
        (0..total)
            .map(|_| Value::Number(precision::whole(rng.random_range(low..=high))))
            .collect()
    } else if min < max {
        (0..total)
            .map(|_| Value::Number(rng.random_range(min..max)))
            .collect()
    } else {
        // No room between them: the one number
        vec![Value::Number(min); total]
    };
    Ok(Value::Array(values.into()))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};

    fn numbers(values: &[f64]) -> Value {
        Value::Array(
            values
                .iter()
                .copied()
                .map(Value::Number)
                .collect::<Vec<_>>()
                .into(),
        )
    }

    #[test]
    fn test_sequence() {
        let ctx = EvalContext::new();
        let result = eval("SEQUENCE(5)", &ctx).unwrap();
        assert_eq!(result, numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]));
    }

    #[test]
    fn test_sequence_with_params() {
        let ctx = EvalContext::new();
        let result = eval("SEQUENCE(3, 1, 10, 5)", &ctx).unwrap();
        assert_eq!(result, numbers(&[10.0, 15.0, 20.0]));
    }

    #[test]
    fn test_randarray() {
        // RANDARRAY(5) = 5 random numbers from 0 up to 1
        let ctx = EvalContext::new();
        let formula =
            "LET(a, RANDARRAY(5), AND(COUNT(a) = 5, ROWS(a) = 5, MIN(a) >= 0, MAX(a) < 1))";
        assert_eq!(eval(formula, &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_randarray_with_range() {
        // RANDARRAY(3, 1, 1, 10, TRUE()) = 3 whole numbers from 1 to 10
        let ctx = EvalContext::new();
        let formula =
            "LET(a, RANDARRAY(3, 1, 1, 10, TRUE()), AND(COUNT(a) = 3, MIN(a) >= 1, MAX(a) <= 10))";
        assert_eq!(eval(formula, &ctx).unwrap(), Value::Boolean(true));
        let drawn = eval("RANDARRAY(20, 1, 1, 10, TRUE())", &ctx)
            .unwrap()
            .as_text();
        assert!(!drawn.contains('.'), "{drawn}");
    }

    #[test]
    fn an_error_in_any_argument_is_the_arrays_error() {
        use super::super::super::tests::{each_argument, fails_with};
        fails_with(
            "RANDARRAY(1, 1, 0, 1, FALSE, 1)",
            "RANDARRAY requires 0-5 arguments, got 6",
        );
        each_argument(
            "SEQUENCE",
            &[("2", None), ("1", None), ("1", None), ("1", None)],
        );
        each_argument(
            "RANDARRAY",
            &[
                ("2", None),
                ("1", None),
                ("0", None),
                ("1", None),
                ("FALSE", None),
            ],
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

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
        assert_exact!(sum_val, 15.0, "SUM(SEQUENCE(5)) should return 15.0");
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
        assert_exact!(
            sum_val,
            70.0,
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
        assert_exact!(count_val, 5.0, "COUNT(RANDARRAY(5)) should return 5.0");
    }
}

#[cfg(test)]
mod edge_case_tests {
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
        let model = calculator.calculate_all().unwrap();
        assert_exact!(model.scalars["count"].value.unwrap(), 5.0);
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
        assert_exact!(max_val, 20.0);
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
        assert_exact!(count_val, 10.0);

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
        assert_exact!(count_val, 100.0);
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

    /// Rows and columns are cut to whole numbers, as Excel cuts them. An
    /// empty array is #CALC! and a negative count #VALUE!, as in Excel; more
    /// values than the 1,048,576 rows of a sheet is #VALUE! too, where rows
    /// times columns overflowed (1E+20 by 1E+20 crashed).
    #[test]
    fn an_array_holds_from_one_value_to_a_sheets_rows() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("COUNT(SEQUENCE(2.9, 1.5))", 2.0);
        h::is_exactly("COUNT(SEQUENCE(1024, 1024))", 1_048_576.0);
        h::fails_with("SEQUENCE(0)", "#CALC!");
        h::fails_with("SEQUENCE(3, 0.5)", "#CALC!");
        h::fails_with("SEQUENCE(-2)", "#VALUE!");
        h::fails_with("SEQUENCE(1048577)", "#VALUE!");
        h::fails_with("SEQUENCE(1E+20, 1E+20)", "#VALUE!");
        h::fails_with("RANDARRAY(0)", "#CALC!");
        h::fails_with("RANDARRAY(1E+20, 1E+20)", "#VALUE!");
    }

    /// A minimum above the maximum is #VALUE!, as in Excel, where the random
    /// range was empty and crashed; an equal one is that number.
    #[test]
    fn randarray_draws_from_the_minimum_to_the_maximum() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("RANDARRAY(1, 1, 5, 1)", "#VALUE!");
        h::fails_with("RANDARRAY(1, 1, 5, 1, TRUE)", "#VALUE!");
        h::is_exactly("SUM(RANDARRAY(2, 1, 3, 3))", 6.0);
        h::is_exactly("SUM(RANDARRAY(2, 1, 3, 3, TRUE))", 6.0);
        h::is_exactly("SUM(RANDARRAY(2, 1, 1.5, 1.7, TRUE))", 2.0);
        h::fails_with("RANDARRAY(1, 1, 1E+19, 1E+20, TRUE)", "#NUM!");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("SEQUENCE(\"x\")", "rows must be a number");
        h::fails_with("SEQUENCE(2, 1/0)", "");
        h::fails_with("RANDARRAY(1/0)", "");
        h::fails_with("RANDARRAY(1, 1/0)", "");
    }
}
