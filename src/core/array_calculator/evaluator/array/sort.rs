//! SORT array function

use crate::core::array_calculator::evaluator::{
    collect_numeric_values, evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// Evaluate SORT function - sorts numeric values in ascending or descending order
pub fn eval_sort(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SORT", args, 1, 2)?;
    let mut values = collect_numeric_values(args, ctx)?;
    let descending = if args.len() > 1 {
        evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) < 0.0
    } else {
        false
    };
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if descending {
        values.reverse();
    }
    Ok(Value::Array(
        values.into_iter().map(Value::Number).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};
    use std::collections::HashMap;

    #[test]
    fn test_sort() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(3.0),
                Value::Number(1.0),
                Value::Number(4.0),
                Value::Number(1.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        let result = eval("SORT(t.data)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], Value::Number(1.0));
            assert_eq!(arr[4], Value::Number(5.0));
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
    fn test_sort_function_coverage() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![30.0, 10.0, 20.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "min_value".to_string(),
            Variable::new(
                "min_value".to_string(),
                None,
                Some("=MIN(SORT(data.values))".to_string()),
            ),
        );
        model.add_scalar(
            "max_value".to_string(),
            Variable::new(
                "max_value".to_string(),
                None,
                Some("=MAX(SORT(data.values))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let min_val = result.scalars.get("min_value").unwrap().value.unwrap();
        assert_eq!(min_val, 10.0, "MIN(SORT(...)) should return 10.0");

        let max_val = result.scalars.get("max_value").unwrap().value.unwrap();
        assert_eq!(max_val, 40.0, "MAX(SORT(...)) should return 40.0");
    }
}

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_ascending_first() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(SORT(data.values), 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_and_min() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "min".to_string(),
            Variable::new(
                "min".to_string(),
                None,
                Some("=MIN(SORT(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let min_val = result.scalars.get("min").unwrap().value.unwrap();
        assert_eq!(min_val, 1.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_ascending_last() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(SORT(data.values), 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_preserves_count() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(SORT(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }
}
