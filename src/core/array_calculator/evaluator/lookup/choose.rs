//! CHOOSE function implementation
//!
//! DEMO function - always available

// Choose casts: f64 index to usize (bounded, 1-based choice index).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// Evaluate CHOOSE function
/// CHOOSE(index, value1, [value2], ...)
/// Returns a value from a list based on a given index
pub fn eval_choose(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("CHOOSE", args, 2, 255)?;
    let index = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("CHOOSE index must be a number"))? as usize;

    if index < 1 || index >= args.len() {
        return Err(EvalError::new(format!("CHOOSE index {index} out of range")));
    }
    evaluate(&args[index], ctx)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparison validated against Excel/Gnumeric/R
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
        if let ColumnValue::Number(values) = &col.values {
            assert_eq!(values[0], 100.0);
            assert_eq!(values[1], 200.0);
            assert_eq!(values[2], 300.0);
        }
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
        if let Some(col) = table.columns.get("chosen") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 10.0);
                assert_eq!(vals[1], 20.0);
                assert_eq!(vals[2], 30.0);
            }
        }
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
        assert_eq!(val, 200.0);
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
}
