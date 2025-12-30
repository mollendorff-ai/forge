//! MATCH function implementation
//!
//! DEMO function - always available

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, values_equal, EvalContext, EvalError, Expr, Value,
};

/// Evaluate MATCH function
/// MATCH(lookup_value, lookup_array, [match_type])
/// Returns the relative position of an item in an array
pub fn eval_match(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("MATCH", args, 2, 3)?;

    let lookup_value = evaluate(&args[0], ctx)?;

    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let lookup_array = evaluate(&args[1], &array_ctx)?;

    let match_type = if args.len() > 2 {
        evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0) as i32
    } else {
        1
    };

    let arr = match lookup_array {
        Value::Array(a) => a,
        _ => return Err(EvalError::new("MATCH requires an array")),
    };

    let lookup_num = lookup_value.as_number();

    match match_type {
        0 => {
            // Exact match
            for (i, val) in arr.iter().enumerate() {
                if values_equal(&lookup_value, val) {
                    return Ok(Value::Number((i + 1) as f64));
                }
            }
            Err(EvalError::new("MATCH: value not found"))
        },
        1 => {
            // Find largest value <= lookup_value
            if let Some(ln) = lookup_num {
                let mut best_idx: Option<usize> = None;
                let mut best_val: Option<f64> = None;
                for (i, v) in arr.iter().enumerate() {
                    if let Some(vn) = v.as_number() {
                        if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                            best_val = Some(vn);
                            best_idx = Some(i);
                        }
                    }
                }
                best_idx
                    .map(|i| Value::Number((i + 1) as f64))
                    .ok_or_else(|| EvalError::new("MATCH: value not found"))
            } else {
                let lookup_text = lookup_value.as_text().to_lowercase();
                for (i, val) in arr.iter().enumerate() {
                    if val.as_text().to_lowercase() == lookup_text {
                        return Ok(Value::Number((i + 1) as f64));
                    }
                }
                Err(EvalError::new("MATCH: value not found"))
            }
        },
        -1 => {
            // Find smallest value >= lookup_value
            let mut best_idx: Option<usize> = None;
            let mut best_val: Option<f64> = None;

            if let Some(ln) = lookup_num {
                for (i, v) in arr.iter().enumerate() {
                    if let Some(vn) = v.as_number() {
                        if vn >= ln && (best_val.is_none() || vn < best_val.unwrap()) {
                            best_val = Some(vn);
                            best_idx = Some(i);
                        }
                    }
                }
            }
            best_idx
                .map(|i| Value::Number((i + 1) as f64))
                .ok_or_else(|| EvalError::new("MATCH: value not found"))
        },
        _ => Err(EvalError::new(format!(
            "MATCH: invalid match_type {match_type}"
        ))),
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
    fn test_match() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // Exact match (match_type = 0)
        assert_eq!(
            eval("MATCH(20, t.data, 0)", &ctx).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_match_exact() {
        let mut model = ParsedModel::new();
        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0, 104.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
                "Widget D".to_string(),
            ]),
        ));
        model.add_table(products);
        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "lookup_id".to_string(),
            ColumnValue::Number(vec![102.0, 104.0, 101.0]),
        ));
        sales.add_row_formula(
            "position".to_string(),
            "=MATCH(lookup_id, products.product_id, 0)".to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let position = result_table.columns.get("position").unwrap();
        match &position.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 2.0);
                assert_eq!(nums[1], 4.0);
                assert_eq!(nums[2], 1.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_match_function_basic() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);

        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=MATCH(30, data.values, 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("pos").unwrap().value.unwrap();
        assert!((val - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_match_text_exact() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        model.add_table(data);

        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=MATCH(\"Banana\", data.names, 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let pos = result.scalars.get("pos").unwrap().value.unwrap();
        assert!((pos - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_match_exact_match_found() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        lookup_table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Text(vec!["Banana".to_string()]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, products.name, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_exact_match_not_found() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["Apple".to_string(), "Banana".to_string()]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Text(vec!["Orange".to_string()]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, products.name, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_less_than_or_equal_ascending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![25.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, 1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_greater_than_or_equal_descending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![40.0, 30.0, 20.0, 10.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![25.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, -1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_invalid_match_type() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Number(vec![15.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, ranges.value, 2)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_no_value_found_ascending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![50.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, 1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_no_value_found_descending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![300.0, 200.0, 100.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![500.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, -1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_exact_first() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(10, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_match_exact_last() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(30, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_match_exact_middle() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(20, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_match_approx_less() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(25, data.values, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_match_approx_greater() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![30.0, 20.0, 10.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(25, data.values, -1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }
}
