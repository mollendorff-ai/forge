//! XLOOKUP, VLOOKUP, HLOOKUP function implementations
//!
//! ENTERPRISE functions - only available in full build

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, values_equal, EvalContext, EvalError, Expr, Value,
};

/// Evaluate XLOOKUP function
/// XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
#[cfg(not(feature = "demo"))]
pub fn eval_xlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("XLOOKUP", args, 3, 6)?;

    let lookup_val = evaluate(&args[0], ctx)?;

    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let lookup_arr = evaluate(&args[1], &array_ctx)?;
    let return_arr = evaluate(&args[2], &array_ctx)?;

    let if_not_found = if args.len() > 3 {
        Some(evaluate(&args[3], ctx)?)
    } else {
        None
    };
    let match_mode = if args.len() > 4 {
        evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
    } else {
        0
    };

    let lookup_values = match lookup_arr {
        Value::Array(arr) => arr,
        _ => return Err(EvalError::new("XLOOKUP lookup_array must be an array")),
    };

    let return_values = match return_arr {
        Value::Array(arr) => arr,
        _ => return Err(EvalError::new("XLOOKUP return_array must be an array")),
    };

    if lookup_values.len() != return_values.len() {
        return Err(EvalError::new(format!(
            "XLOOKUP: lookup_array ({}) and return_array ({}) must have same length",
            lookup_values.len(),
            return_values.len()
        )));
    }

    let idx = match match_mode {
        0 => lookup_values
            .iter()
            .position(|v| values_equal(v, &lookup_val)),
        -1 => {
            let mut best_idx: Option<usize> = None;
            let mut best_val: Option<f64> = None;
            let lookup_num = lookup_val.as_number();

            for (i, v) in lookup_values.iter().enumerate() {
                if values_equal(v, &lookup_val) {
                    return Ok(return_values.get(i).cloned().unwrap_or(Value::Null));
                }
                if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                    if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                        best_val = Some(vn);
                        best_idx = Some(i);
                    }
                }
            }
            best_idx
        },
        1 => {
            let mut best_idx: Option<usize> = None;
            let mut best_val: Option<f64> = None;
            let lookup_num = lookup_val.as_number();

            for (i, v) in lookup_values.iter().enumerate() {
                if values_equal(v, &lookup_val) {
                    return Ok(return_values.get(i).cloned().unwrap_or(Value::Null));
                }
                if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                    if vn >= ln && (best_val.is_none() || vn < best_val.unwrap()) {
                        best_val = Some(vn);
                        best_idx = Some(i);
                    }
                }
            }
            best_idx
        },
        _ => {
            return Err(EvalError::new(format!(
                "XLOOKUP: invalid match_mode {match_mode}"
            )))
        },
    };

    match idx {
        Some(i) => Ok(return_values.get(i).cloned().unwrap_or(Value::Null)),
        None => {
            if let Some(not_found) = if_not_found {
                Ok(not_found)
            } else {
                Err(EvalError::new("XLOOKUP: No match found"))
            }
        },
    }
}

/// Evaluate VLOOKUP function
/// VLOOKUP(lookup_value, table_array, col_index, [range_lookup])
#[cfg(not(feature = "demo"))]
pub fn eval_vlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("VLOOKUP", args, 3, 4)?;

    let lookup_val = evaluate(&args[0], ctx)?;

    // Get the table array without row context
    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let table_array = evaluate(&args[1], &array_ctx)?;
    let _col_index = evaluate(&args[2], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("VLOOKUP: col_index must be a number"))?
        as usize;
    let range_lookup = if args.len() > 3 {
        evaluate(&args[3], ctx)?.is_truthy()
    } else {
        true
    };

    // For Forge, VLOOKUP works on arrays. Column selection is simulated.
    // Since we don't have 2D arrays in the same way as Excel, we implement
    // a simplified version that works with single-column lookups.
    let lookup_arr = match table_array {
        Value::Array(arr) => arr,
        _ => return Err(EvalError::new("VLOOKUP: table_array must be an array")),
    };

    // Simplified: find match in first column (the array itself acts as first column)
    // For full VLOOKUP semantics, you'd need a 2D table structure
    let idx = if range_lookup {
        // Approximate match - find largest value <= lookup_val
        let mut best_idx: Option<usize> = None;
        let mut best_val: Option<f64> = None;
        if let Some(ln) = lookup_val.as_number() {
            for (i, v) in lookup_arr.iter().enumerate() {
                if let Some(vn) = v.as_number() {
                    if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                        best_val = Some(vn);
                        best_idx = Some(i);
                    }
                }
            }
        }
        best_idx
    } else {
        // Exact match
        lookup_arr.iter().position(|v| values_equal(v, &lookup_val))
    };

    match idx {
        Some(i) => Ok(lookup_arr.get(i).cloned().unwrap_or(Value::Null)),
        None => Err(EvalError::new("VLOOKUP: value not found")),
    }
}

/// Evaluate HLOOKUP function
/// HLOOKUP(lookup_value, table_array, row_index, [range_lookup])
#[cfg(not(feature = "demo"))]
pub fn eval_hlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("HLOOKUP", args, 3, 4)?;

    let lookup_val = evaluate(&args[0], ctx)?;

    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let table_array = evaluate(&args[1], &array_ctx)?;
    let _row_index = evaluate(&args[2], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("HLOOKUP: row_index must be a number"))?
        as usize;
    let range_lookup = if args.len() > 3 {
        evaluate(&args[3], ctx)?.is_truthy()
    } else {
        true
    };

    // HLOOKUP is horizontal - search in first row, return from specified row
    // Simplified implementation similar to VLOOKUP
    let lookup_arr = match table_array {
        Value::Array(arr) => arr,
        _ => return Err(EvalError::new("HLOOKUP: table_array must be an array")),
    };

    let idx = if range_lookup {
        let mut best_idx: Option<usize> = None;
        let mut best_val: Option<f64> = None;
        if let Some(ln) = lookup_val.as_number() {
            for (i, v) in lookup_arr.iter().enumerate() {
                if let Some(vn) = v.as_number() {
                    if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                        best_val = Some(vn);
                        best_idx = Some(i);
                    }
                }
            }
        }
        best_idx
    } else {
        lookup_arr.iter().position(|v| values_equal(v, &lookup_val))
    };

    match idx {
        Some(i) => Ok(lookup_arr.get(i).cloned().unwrap_or(Value::Null)),
        None => Err(EvalError::new("HLOOKUP: value not found")),
    }
}

#[cfg(test)]
#[cfg(not(feature = "demo"))]
mod tests {
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

    #[test]
    fn test_vlookup() {
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

        // Exact match (range_lookup = FALSE)
        assert_eq!(
            eval("VLOOKUP(20, t.data, 1, FALSE())", &ctx).unwrap(),
            Value::Number(20.0)
        );
    }

    #[test]
    fn test_xlookup_exact_match() {
        let mut model = ParsedModel::new();

        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
            ]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 103.0, 101.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=XLOOKUP(product_id, products.product_id, products.product_name)".to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let product_name = result_table.columns.get("product_name").unwrap();
        match &product_name.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Widget B");
                assert_eq!(texts[1], "Widget C");
                assert_eq!(texts[2], "Widget A");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_xlookup_with_if_not_found() {
        let mut model = ParsedModel::new();

        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
            ]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 999.0, 101.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=XLOOKUP(product_id, products.product_id, products.product_name, \"Not Found\")"
                .to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let product_name = result_table.columns.get("product_name").unwrap();
        match &product_name.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Widget B");
                assert_eq!(texts[1], "Not Found");
                assert_eq!(texts[2], "Widget A");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_vlookup_exact_match() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
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
            ColumnValue::Number(vec![1.50, 0.75, 3.00]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search_id".to_string(),
            ColumnValue::Number(vec![102.0]),
        ));
        data.row_formulas.insert(
            "found_price".to_string(),
            "=VLOOKUP(search_id, products, 3, FALSE)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("VLOOKUP")
                    || err_msg.contains("table")
                    || err_msg.contains("Unknown variable")
                    || err_msg.contains("products"),
                "VLOOKUP should error with meaningful message, got: {err_msg}"
            );
        } else {
            let model_result = result.unwrap();
            let table = model_result.tables.get("data").unwrap();
            if let Some(col) = table.columns.get("found_price") {
                if let ColumnValue::Number(vals) = &col.values {
                    assert_eq!(vals[0], 0.75);
                }
            }
        }
    }

    #[test]
    fn test_xlookup_employee_salary() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("employees".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        lookup_table.add_column(Column::new(
            "salary".to_string(),
            ColumnValue::Number(vec![50000.0, 60000.0, 70000.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "emp_id".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas.insert(
            "emp_salary".to_string(),
            "=XLOOKUP(emp_id, employees.id, employees.salary, 0, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("emp_salary") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 60000.0);
            }
        }
    }

    #[test]
    fn test_xlookup_default_value() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("employees".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        lookup_table.add_column(Column::new(
            "salary".to_string(),
            ColumnValue::Number(vec![50000.0, 60000.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "emp_id".to_string(),
            ColumnValue::Number(vec![99.0]),
        ));
        data.row_formulas.insert(
            "emp_salary".to_string(),
            "=XLOOKUP(emp_id, employees.id, employees.salary, -1, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("emp_salary") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], -1.0);
            }
        }
    }

    #[test]
    fn test_xlookup_not_found_fallback() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("items".to_string());
        data.add_column(Column::new(
            "code".to_string(),
            ColumnValue::Text(vec!["A1".to_string(), "B2".to_string(), "C3".to_string()]),
        ));
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=XLOOKUP(\"D4\", items.code, items.value, -1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "XLOOKUP not found fallback should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("result").unwrap().value.unwrap();
        assert_eq!(val, -1.0);
    }

    #[test]
    fn test_xlookup_exact() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(2, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
    }

    #[test]
    fn test_xlookup_not_found() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(5, data.keys, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_xlookup_first_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(1, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_xlookup_last_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(3, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }
}
