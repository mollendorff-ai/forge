//! Reference function implementations: INDIRECT, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS
//!
//! ENTERPRISE functions - only available in full build

use crate::core::array_calculator::evaluator::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// Evaluate INDIRECT function
/// INDIRECT(ref_text, [a1])
/// Returns the reference specified by a text string
#[cfg(not(feature = "demo"))]
pub fn eval_indirect(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("INDIRECT", args, 1)?;
    let ref_str = evaluate(&args[0], ctx)?.as_text();
    if let Some(val) = ctx.scalars.get(&ref_str) {
        return Ok(val.clone());
    }
    if ref_str.contains('.') {
        let parts: Vec<&str> = ref_str.splitn(2, '.').collect();
        if parts.len() == 2 {
            if let Some(table) = ctx.tables.get(parts[0]) {
                if let Some(col) = table.get(parts[1]) {
                    return Ok(Value::Array(col.clone()));
                }
            }
        }
    }
    Err(EvalError::new(format!(
        "INDIRECT: cannot resolve '{ref_str}'"
    )))
}

/// Evaluate OFFSET function
/// OFFSET(reference, rows, cols, [height], [width])
/// Returns a reference offset from a given reference
#[cfg(not(feature = "demo"))]
pub fn eval_offset(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("OFFSET", args, 3, 5)?;

    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let base = evaluate(&args[0], &array_ctx)?;
    let rows = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("OFFSET: rows must be a number"))? as i64;
    let cols = evaluate(&args[2], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("OFFSET: cols must be a number"))? as i64;

    // OFFSET returns a reference offset by rows and cols
    // Simplified: for array, return element at offset position
    match base {
        Value::Array(arr) => {
            if rows < 0 || rows as usize >= arr.len() {
                return Err(EvalError::new("OFFSET: row out of bounds"));
            }
            Ok(arr.get(rows as usize).cloned().unwrap_or(Value::Null))
        },
        other => {
            // For scalar, just return the value (offset of 0,0)
            if rows == 0 && cols == 0 {
                Ok(other)
            } else {
                Err(EvalError::new("OFFSET: cannot offset scalar"))
            }
        },
    }
}

/// Evaluate ADDRESS function
/// ADDRESS(row_num, column_num, [abs_num], [a1], [sheet_text])
/// Returns a cell reference as a text string
#[cfg(not(feature = "demo"))]
pub fn eval_address(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("ADDRESS", args, 2, 5)?;
    let row_num = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("ADDRESS: row must be a number"))? as i64;
    let col_num = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("ADDRESS: column must be a number"))?
        as i64;
    let abs_num = if args.len() > 2 {
        evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0) as i32
    } else {
        1
    };
    let a1_style = if args.len() > 3 {
        evaluate(&args[3], ctx)?.is_truthy()
    } else {
        true
    };

    if !(1..=16384).contains(&col_num) {
        return Err(EvalError::new("ADDRESS: column out of range"));
    }

    // Convert column number to letter(s)
    let col_letter = col_to_letter(col_num as usize);

    // abs_num: 1=absolute, 2=absolute row/relative col, 3=relative row/absolute col, 4=relative
    let address = if a1_style {
        match abs_num {
            1 => format!("${col_letter}${row_num}"),
            2 => format!("{col_letter}${row_num}"),
            3 => format!("${col_letter}{row_num}"),
            4 => format!("{col_letter}{row_num}"),
            _ => format!("${col_letter}${row_num}"),
        }
    } else {
        // R1C1 style
        match abs_num {
            1 => format!("R{row_num}C{col_num}"),
            4 => format!("R[{row_num}]C[{col_num}]"),
            _ => format!("R{row_num}C{col_num}"),
        }
    };
    Ok(Value::Text(address))
}

/// Evaluate ROW function
/// ROW([reference])
/// Returns the row number of a reference
#[cfg(not(feature = "demo"))]
pub fn eval_row(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    // ROW() returns current row number (1-based)
    // ROW(reference) returns the row number of the reference
    if args.is_empty() {
        // Return current row if available
        if let Some(row) = ctx.current_row {
            Ok(Value::Number((row + 1) as f64))
        } else {
            Ok(Value::Number(1.0))
        }
    } else {
        // For a reference, we'd need to parse it. Simplified: return 1
        Ok(Value::Number(1.0))
    }
}

/// Evaluate COLUMN function
/// COLUMN([reference])
/// Returns the column number of a reference
#[cfg(not(feature = "demo"))]
pub fn eval_column(_args: &[Expr], _ctx: &EvalContext) -> Result<Value, EvalError> {
    // COLUMN() returns current column number
    // Simplified implementation - always returns 1
    Ok(Value::Number(1.0))
}

/// Evaluate ROWS function
/// ROWS(array)
/// Returns the number of rows in a reference or array
#[cfg(not(feature = "demo"))]
pub fn eval_rows(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("ROWS", args, 1)?;
    let array_ctx = EvalContext {
        scalars: ctx.scalars.clone(),
        tables: ctx.tables.clone(),
        scenarios: ctx.scenarios.clone(),
        current_row: None,
        row_count: ctx.row_count,
    };
    let val = evaluate(&args[0], &array_ctx)?;
    match val {
        Value::Array(arr) => Ok(Value::Number(arr.len() as f64)),
        _ => Ok(Value::Number(1.0)),
    }
}

/// Evaluate COLUMNS function
/// COLUMNS(array)
/// Returns the number of columns in a reference or array
#[cfg(not(feature = "demo"))]
pub fn eval_columns(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("COLUMNS", args, 1)?;
    // Check if the argument is a table reference (bare table name)
    if let Expr::Reference(crate::core::array_calculator::parser::Reference::Scalar(name)) =
        &args[0]
    {
        // Check if this name exists in tables
        if let Some(table) = ctx.tables.get(name) {
            return Ok(Value::Number(table.len() as f64));
        }
    }
    // For 1D arrays (single column), columns is always 1
    Ok(Value::Number(1.0))
}

/// Convert column number (1-based) to Excel-style letter(s)
#[cfg(not(feature = "demo"))]
pub fn col_to_letter(col: usize) -> String {
    let mut result = String::new();
    let mut n = col;
    while n > 0 {
        n -= 1;
        let remainder = n % 26;
        result.insert(0, (b'A' + remainder as u8) as char);
        n /= 26;
    }
    result
}

#[cfg(test)]
#[cfg(not(feature = "demo"))]
mod tests {
    use super::*;
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

    #[test]
    fn test_address() {
        let ctx = EvalContext::new();
        // Default is absolute A1 style
        assert_eq!(
            eval("ADDRESS(1, 1)", &ctx).unwrap(),
            Value::Text("$A$1".to_string())
        );
        // Column 27 = AA
        assert_eq!(
            eval("ADDRESS(1, 27)", &ctx).unwrap(),
            Value::Text("$AA$1".to_string())
        );
        // Relative style (abs_num = 4)
        assert_eq!(
            eval("ADDRESS(1, 1, 4)", &ctx).unwrap(),
            Value::Text("A1".to_string())
        );
    }

    #[test]
    fn test_row_column() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROW()", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("COLUMN()", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_rows() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "col".to_string(),
            vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("ROWS(t.col)", &ctx).unwrap(), Value::Number(4.0));
    }

    #[test]
    fn test_columns() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert("col".to_string(), vec![Value::Number(1.0)]);
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COLUMNS(t.col)", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_col_to_letter() {
        assert_eq!(col_to_letter(1), "A");
        assert_eq!(col_to_letter(26), "Z");
        assert_eq!(col_to_letter(27), "AA");
        assert_eq!(col_to_letter(52), "AZ");
        assert_eq!(col_to_letter(53), "BA");
    }

    #[test]
    fn test_offset() {
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

        assert_eq!(
            eval("OFFSET(t.col, 1, 0)", &ctx).unwrap(),
            Value::Number(20.0)
        );
    }

    #[test]
    fn test_indirect_function() {
        let mut model = ParsedModel::new();

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0, 500.0]),
        ));
        model.add_table(sales);

        model.add_scalar(
            "inputs.rate".to_string(),
            Variable::new("inputs.rate".to_string(), Some(0.1), None),
        );

        model.add_scalar(
            "sum_indirect".to_string(),
            Variable::new(
                "sum_indirect".to_string(),
                None,
                Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
            ),
        );

        model.add_scalar(
            "rate_indirect".to_string(),
            Variable::new(
                "rate_indirect".to_string(),
                None,
                Some("=INDIRECT(\"inputs.rate\") * 100".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let sum = result.scalars.get("sum_indirect").unwrap().value.unwrap();
        assert!(
            (sum - 1500.0).abs() < 0.001,
            "INDIRECT column SUM should return 1500, got {sum}"
        );

        let rate = result.scalars.get("rate_indirect").unwrap().value.unwrap();
        assert!(
            (rate - 10.0).abs() < 0.001,
            "INDIRECT scalar should return 10, got {rate}"
        );
    }

    #[test]
    fn test_indirect_function_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "indirect_val".to_string(),
            Variable::new(
                "indirect_val".to_string(),
                None,
                Some("=SUM(INDIRECT(\"data.values\"))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "INDIRECT function should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result
            .scalars
            .get("indirect_val")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(val, 60.0);
    }

    #[test]
    fn test_indirect_table_column() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "INDIRECT table column should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("total").unwrap().value.unwrap();
        assert_eq!(val, 600.0);
    }

    #[test]
    fn test_offset_function_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "offset_sum".to_string(),
            Variable::new(
                "offset_sum".to_string(),
                None,
                Some("=SUM(OFFSET(data.values, 1, 3))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "OFFSET function should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result
            .scalars
            .get("offset_sum")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(val, 20.0);
    }

    #[test]
    fn test_row_function_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "row_num".to_string(),
            Variable::new("row_num".to_string(), None, Some("=ROW()".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("row_num").unwrap().value.unwrap();
        assert_eq!(val, 1.0);
    }

    #[test]
    fn test_row_function_in_expression() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "calc".to_string(),
            Variable::new("calc".to_string(), None, Some("=ROW() * 10".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("calc").unwrap().value.unwrap();
        assert_eq!(val, 10.0);
    }

    #[test]
    fn test_column_function_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "col_num".to_string(),
            Variable::new("col_num".to_string(), None, Some("=COLUMN()".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("col_num").unwrap().value.unwrap();
        assert_eq!(val, 1.0);
    }

    #[test]
    fn test_column_function_in_expression() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "calc".to_string(),
            Variable::new("calc".to_string(), None, Some("=COLUMN() + 5".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("calc").unwrap().value.unwrap();
        assert_eq!(val, 6.0);
    }

    #[test]
    fn test_rows_function_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "row_count".to_string(),
            Variable::new(
                "row_count".to_string(),
                None,
                Some("=ROWS(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("row_count").unwrap().value.unwrap();
        assert_eq!(val, 5.0);
    }

    #[test]
    fn test_rows_function_single_element() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![42.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "row_count".to_string(),
            Variable::new(
                "row_count".to_string(),
                None,
                Some("=ROWS(data.value)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("row_count").unwrap().value.unwrap();
        assert_eq!(val, 1.0);
    }

    #[test]
    fn test_rows_function_in_calculation() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "items".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "calc".to_string(),
            Variable::new(
                "calc".to_string(),
                None,
                Some("=ROWS(data.items) * 10".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("calc").unwrap().value.unwrap();
        assert_eq!(val, 30.0);
    }

    #[test]
    fn test_columns_function_single_column() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "col_count".to_string(),
            Variable::new(
                "col_count".to_string(),
                None,
                Some("=COLUMNS(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("col_count").unwrap().value.unwrap();
        assert_eq!(val, 1.0);
    }

    #[test]
    fn test_address_absolute() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "cell_ref".to_string(),
            Variable::new(
                "cell_ref".to_string(),
                None,
                Some("=LEN(ADDRESS(1, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("cell_ref").unwrap().value.unwrap();
        assert_eq!(val, 4.0);
    }

    #[test]
    fn test_address_b2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "cell_ref".to_string(),
            Variable::new(
                "cell_ref".to_string(),
                None,
                Some("=LEN(ADDRESS(2, 2))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("cell_ref").unwrap().value.unwrap();
        assert_eq!(val, 4.0);
    }

    #[test]
    fn test_address_relative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "cell_ref".to_string(),
            Variable::new(
                "cell_ref".to_string(),
                None,
                Some("=LEN(ADDRESS(1, 1, 4))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("cell_ref").unwrap().value.unwrap();
        assert_eq!(val, 2.0);
    }

    #[test]
    fn test_offset_positive_offset() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "offset_val".to_string(),
            Variable::new(
                "offset_val".to_string(),
                None,
                Some("=OFFSET(data.values, 2, 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "OFFSET with positive offset should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("offset_val").unwrap().value;
        assert!(val.is_some(), "OFFSET should return a value");
    }

    #[test]
    fn test_offset_with_sum() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "offset_sum".to_string(),
            Variable::new(
                "offset_sum".to_string(),
                None,
                Some("=SUM(OFFSET(data.values, 1, 2))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "OFFSET with SUM should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result
            .scalars
            .get("offset_sum")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(val, 20.0);
    }

    #[test]
    fn test_indirect_column_reference() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("total").unwrap().value.unwrap();
        assert_eq!(val, 600.0);
    }

    #[test]
    fn test_indirect_scalar_reference() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "base_rate".to_string(),
            Variable::new("base_rate".to_string(), Some(0.15), None),
        );

        model.add_scalar(
            "rate_multiplied".to_string(),
            Variable::new(
                "rate_multiplied".to_string(),
                None,
                Some("=INDIRECT(\"base_rate\") * 100".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result
            .scalars
            .get("rate_multiplied")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(val, 15.0);
    }

    #[test]
    fn test_indirect_with_index() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "items".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "indirect_index".to_string(),
            Variable::new(
                "indirect_index".to_string(),
                None,
                Some("=INDEX(INDIRECT(\"data.items\"), 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let val = result.scalars.get("indirect_index").unwrap().value.unwrap();
        assert_eq!(val, 20.0);
    }
}
