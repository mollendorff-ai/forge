//! INDEX function implementation
//!
//! DEMO function - always available

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// Evaluate INDEX function
/// INDEX(array, row_num, [col_num])
/// Returns the value at a given position in an array
pub fn eval_index(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("INDEX", args, 2, 3)?;

    // Evaluate array without row context to get full array
    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let array = evaluate(&args[0], &array_ctx)?;
    let row_num = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as i64;

    // INDEX is 1-based, row_num must be >= 1
    if row_num < 1 {
        return Err(EvalError::new(format!(
            "INDEX: row_num {row_num} must be >= 1"
        )));
    }

    match array {
        Value::Array(arr) => {
            let idx = (row_num - 1) as usize;
            arr.get(idx)
                .cloned()
                .ok_or_else(|| EvalError::new(format!("INDEX row {row_num} out of bounds")))
        },
        _ => Err(EvalError::new("INDEX requires an array")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

    #[test]
    fn test_index_function() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "col".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("INDEX(t.col, 1)", &ctx).unwrap(), Value::Number(10.0));
        assert_eq!(eval("INDEX(t.col, 2)", &ctx).unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_index_basic() {
        let mut model = ParsedModel::new();
        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
            ]),
        ));
        model.add_table(products);

        let mut test = Table::new("test".to_string());
        test.add_column(Column::new(
            "index".to_string(),
            ColumnValue::Number(vec![1.0, 3.0, 2.0]),
        ));
        test.add_row_formula(
            "name".to_string(),
            "=INDEX(products.product_name, index)".to_string(),
        );
        model.add_table(test);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("test").unwrap();

        let name = result_table.columns.get("name").unwrap();
        match &name.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Widget A");
                assert_eq!(texts[1], "Widget C");
                assert_eq!(texts[2], "Widget B");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_index_function_basic() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);

        model.add_scalar(
            "third".to_string(),
            Variable::new(
                "third".to_string(),
                None,
                Some("=INDEX(data.values, 3)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("third").unwrap().value.unwrap();
        assert!((val - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_array_index_access() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "first".to_string(),
            Variable::new(
                "first".to_string(),
                None,
                Some("=data.values[0]".to_string()),
            ),
        );
        model.add_scalar(
            "last".to_string(),
            Variable::new(
                "last".to_string(),
                None,
                Some("=data.values[2]".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("first").unwrap().value.unwrap() - 10.0).abs() < 0.01);
        assert!((result.scalars.get("last").unwrap().value.unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_index_single_column() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "second".to_string(),
            Variable::new(
                "second".to_string(),
                None,
                Some("=INDEX(data.values, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let second = result.scalars.get("second").unwrap().value.unwrap();
        assert!((second - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_index_bounds_error() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("items".to_string());
        lookup_table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![10.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=INDEX(items.value, idx)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_index_zero_row_num() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("items".to_string());
        lookup_table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![0.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=INDEX(items.value, idx)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_array_index_out_of_bounds() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "val".to_string(),
            Variable::new(
                "val".to_string(),
                None,
                Some("=data.values[100]".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_err(),
            "Array index [100] out of bounds [0-2] should error"
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_first() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_last() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(50.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_middle() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_single() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![42.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(42.0));
    }
}
