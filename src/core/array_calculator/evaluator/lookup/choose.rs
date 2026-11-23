//! CHOOSE function implementation

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};
use num_traits::ToPrimitive;

/// Evaluate CHOOSE function
/// CHOOSE(index, value1, [value2], ...)
/// Returns a value from a list based on a given index
pub fn eval_choose(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("CHOOSE", args, 2, 255)?;
    let index = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("CHOOSE index must be a number"))?;

    // From 1 to the number of values, cut to a whole number
    let Some(at) = index.to_usize().filter(|&at| (1..args.len()).contains(&at)) else {
        return Err(EvalError::new(format!("CHOOSE index {index} out of range")));
    };
    evaluate(&args[at], ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_choose() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("CHOOSE(1, \"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("a".to_string())
        );
        assert_eq!(
            eval("CHOOSE(2, \"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("b".to_string())
        );
    }

    #[test]
    fn test_choose_function() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "chosen_rate".to_string(),
            Variable::new(
                "chosen_rate".to_string(),
                None,
                Some("=CHOOSE(2, 0.05, 0.10, 0.02)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let rate = result.scalars.get("chosen_rate").unwrap().value.unwrap();

        assert!(
            (rate - 0.10).abs() < 0.001,
            "CHOOSE(2, ...) should return 0.10, got {rate}"
        );
    }

    #[test]
    fn test_choose_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "index".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=CHOOSE(index, 100, 200, 300)".to_string(),
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
        let values = col.values.numbers().unwrap();
        let values = values.as_ref();
        assert_exact!(values[0], 100.0);
        assert_exact!(values[1], 200.0);
        assert_exact!(values[2], 300.0);
    }

    #[test]
    fn test_choose_function_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        data.row_formulas
            .insert("chosen".to_string(), "=CHOOSE(idx, 10, 20, 30)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "CHOOSE function should calculate successfully"
        );
        let model_result = result.unwrap();
        let table = model_result.tables.get("data").unwrap();
        let col = table.columns.get("chosen").unwrap();
        let vals = col.values.numbers().unwrap();
        let vals = vals.as_ref();
        assert_exact!(vals[0], 10.0);
        assert_exact!(vals[1], 20.0);
        assert_exact!(vals[2], 30.0);
    }

    #[test]
    fn test_choose_valid_index() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(2, 100, 200, 300)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok(), "CHOOSE with valid index should succeed");
        let model_result = result.unwrap();
        let val = model_result.scalars.get("result").unwrap().value.unwrap();
        assert_exact!(val, 200.0);
    }

    #[test]
    fn test_choose_index_out_of_range() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(10, 100, 200, 300)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_err(),
            "CHOOSE with index 10 out of range [1-3] should error"
        );
    }

    #[test]
    fn test_choose_first_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(1, 10, 20, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_choose_second_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(2, 10, 20, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
    }

    #[test]
    fn test_choose_last_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(3, 10, 20, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    #[test]
    fn test_choose_formula() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CHOOSE(2, 1+1, 2+2, 3+3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    /// An index from 1 to the number of values, cut to a whole number
    #[test]
    fn an_index_is_a_whole_number_among_the_values() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(h::text("CHOOSE(1.9, \"a\", \"b\")"), "a");
        assert_eq!(h::text("CHOOSE(2.9, \"a\", \"b\")"), "b");
        h::fails_with("CHOOSE(0.5, \"a\", \"b\")", "index 0.5 out of range");
        h::fails_with("CHOOSE(3, \"a\", \"b\")", "out of range");
        h::fails_with("CHOOSE(-1, \"a\", \"b\")", "out of range");
        h::fails_with("CHOOSE(1E+20, \"a\", \"b\")", "out of range");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("CHOOSE(\"x\", 1, 2)", "index must be a number");
    }

    #[test]
    fn an_error_in_any_argument_is_the_error() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        each_argument("CHOOSE", &[("1", None), ("2", None)]);
    }
}
