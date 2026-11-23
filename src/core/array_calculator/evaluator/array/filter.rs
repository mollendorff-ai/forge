//! FILTER array function

use crate::core::array_calculator::evaluator::{
    collect_values_as_vec, require_args, EvalContext, EvalError, Expr, Value,
};

/// Evaluate FILTER function - filters array based on criteria array
pub fn eval_filter(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("FILTER", args, 2)?;
    let data = collect_values_as_vec(&args[0], ctx)?;
    let criteria = collect_values_as_vec(&args[1], ctx)?;
    let filtered: Vec<Value> = data
        .iter()
        .zip(criteria.iter())
        .filter(|(_, c)| c.is_truthy())
        .map(|(v, _)| v.clone())
        .collect();
    Ok(Value::Array(filtered.into()))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};
    use std::collections::HashMap;

    #[test]
    fn test_filter() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        // Create data array: [10, 20, 30, 40, 50]
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
                Value::Number(40.0),
                Value::Number(50.0),
            ],
        );

        // Create filter array: [true, false, true, false, true]
        table.insert(
            "include".to_string(),
            vec![
                Value::Boolean(true),
                Value::Boolean(false),
                Value::Boolean(true),
                Value::Boolean(false),
                Value::Boolean(true),
            ],
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

        // FILTER should return [10, 30, 50]
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        assert_eq!(
            result,
            Value::Array(
                vec![
                    Value::Number(10.0),
                    Value::Number(30.0),
                    Value::Number(50.0)
                ]
                .into()
            )
        );
    }

    #[test]
    fn test_filter_all_false() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        table.insert(
            "values".to_string(),
            vec![Value::Number(10.0), Value::Number(20.0)],
        );
        table.insert(
            "include".to_string(),
            vec![Value::Boolean(false), Value::Boolean(false)],
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

        // FILTER should return empty array
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        assert_eq!(result, Value::Array(vec![].into()));
    }

    #[test]
    fn test_filter_all_true() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        table.insert(
            "values".to_string(),
            vec![
                Value::Number(100.0),
                Value::Number(200.0),
                Value::Number(300.0),
            ],
        );
        table.insert(
            "include".to_string(),
            vec![
                Value::Boolean(true),
                Value::Boolean(true),
                Value::Boolean(true),
            ],
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

        // FILTER should return all values
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        assert_eq!(
            result,
            Value::Array(
                vec![
                    Value::Number(100.0),
                    Value::Number(200.0),
                    Value::Number(300.0)
                ]
                .into()
            )
        );
    }
}

#[cfg(test)]
mod argument_tests {
    use super::super::super::tests::{each_argument, fails_with};

    #[test]
    fn an_error_in_any_argument_is_the_filters_error() {
        each_argument("FILTER", &[("1", None), ("TRUE", None)]);
        // An error the criteria give when they are evaluated, not only a
        // name checked before
        fails_with("FILTER(1, 1/0)", "Division by zero");
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_filter_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 25.0, 5.0, 30.0]),
        ));
        data.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Boolean(vec![true, true, false, true]),
        ));
        model.add_table(data);

        model.add_scalar(
            "filtered_sum".to_string(),
            Variable::new(
                "filtered_sum".to_string(),
                None,
                Some("=SUM(FILTER(data.value, data.include))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let filtered_sum = result.scalars.get("filtered_sum").unwrap().value.unwrap();
        assert_exact!(filtered_sum, 65.0, "SUM(FILTER(...)) should return 65.0");
    }
}

#[cfg(test)]
mod edge_case_tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_filter_greater() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![0.0, 0.0, 0.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(9.0));
    }

    #[test]
    fn test_filter_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        data.add_column(Column::new(
            "flags".to_string(),
            ColumnValue::Boolean(vec![true, false, true, false, true]),
        ));
        model.add_table(data);
        model.add_scalar(
            "sum".to_string(),
            Variable::new(
                "sum".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.flags))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_result = result.scalars.get("sum").unwrap().value.unwrap();
        assert_exact!(sum_result, 9.0);
    }

    #[test]
    fn test_filter_less() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 0.0, 0.0, 0.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_filter_count() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![0.0, 0.0, 1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_filter_first_only() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![1.0, 0.0, 0.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }
}
