//! Reference function implementations: INDIRECT, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS
//!
//! Functions

use crate::core::array_calculator::evaluator::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};
use crate::core::array_calculator::evaluator::{evaluate_without_row, number_or};
use crate::core::precision;
use num_traits::ToPrimitive;

/// Evaluate INDIRECT function
/// `INDIRECT(ref_text`, [a1])
/// Returns the reference specified by a text string
pub fn eval_indirect(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("INDIRECT", args, 1)?;
    let ref_str = evaluate(&args[0], ctx)?.as_text();
    if let Some(val) = ctx.get_scalar(&ref_str) {
        return Ok(val.clone());
    }
    // `table.column`, the column split off at the first dot
    let column = ref_str
        .split_once('.')
        .and_then(|(table, column)| ctx.tables.get(table)?.get(column));
    if let Some(col) = column {
        return Ok(Value::Array(col.clone()));
    }
    Err(EvalError::new(format!(
        "INDIRECT: cannot resolve '{ref_str}'"
    )))
}

/// Evaluate OFFSET function
/// OFFSET(reference, rows, cols, [height], [width])
/// Returns a reference offset from a given reference
pub fn eval_offset(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("OFFSET", args, 3, 5)?;
    let base = evaluate_without_row(&args[0], ctx)?;
    // Whole rows and columns, as Excel cuts them
    let rows = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("OFFSET: rows must be a number"))?
        .trunc();
    let cols = evaluate(&args[2], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("OFFSET: cols must be a number"))?
        .trunc();

    // OFFSET returns a reference offset by rows and cols
    // Simplified: for array, return element at offset position
    match base {
        Value::Array(arr) => rows
            .to_usize()
            .and_then(|row| arr.get(row))
            .cloned()
            .ok_or_else(|| EvalError::new("OFFSET: row out of bounds")),
        other => {
            // For scalar, just return the value (offset of 0,0)
            if rows == 0.0 && cols == 0.0 {
                Ok(other)
            } else {
                Err(EvalError::new("OFFSET: cannot offset scalar"))
            }
        },
    }
}

/// Evaluate ADDRESS function
/// `ADDRESS(row_num`, `column_num`, [`abs_num`], [a1], [`sheet_text`])
/// Returns a cell reference as a text string
pub fn eval_address(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("ADDRESS", args, 2, 5)?;
    let row_num = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ADDRESS: row must be a number"))?;
    let col_num = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ADDRESS: column must be a number"))?;
    let abs_num = if args.len() > 2 {
        number_or(&args[2], ctx, 1.0)?
    } else {
        1.0
    };
    let a1_style = if args.len() > 3 {
        evaluate(&args[3], ctx)?.is_truthy()
    } else {
        true
    };

    // Excel: #VALUE! outside the sheet, or for an abs_num other than 1 to 4,
    // each cut to a whole number first
    let Some(row_num) = row_num.to_u32().filter(|r| (1..=1_048_576).contains(r)) else {
        return Err(EvalError::new("#VALUE!: ADDRESS: row out of range"));
    };
    let Some(col_num) = col_num.to_usize().filter(|c| (1..=16384).contains(c)) else {
        return Err(EvalError::new("ADDRESS: column out of range"));
    };
    let Some(abs_num) = abs_num.to_u8().filter(|a| (1..=4).contains(a)) else {
        return Err(EvalError::new("#VALUE!: ADDRESS: abs_num must be 1 to 4"));
    };

    // Convert column number to letter(s)
    let col_letter = col_to_letter(col_num);

    // abs_num: 1=absolute, 2=absolute row/relative col, 3=relative row/absolute col, 4=relative
    let address = if a1_style {
        match abs_num {
            2 => format!("{col_letter}${row_num}"),
            3 => format!("${col_letter}{row_num}"),
            4 => format!("{col_letter}{row_num}"),
            _ => format!("${col_letter}${row_num}"), // 1 and default: absolute
        }
    } else {
        // R1C1 style: brackets mark the relative part
        match abs_num {
            2 => format!("R{row_num}C[{col_num}]"),
            3 => format!("R[{row_num}]C{col_num}"),
            4 => format!("R[{row_num}]C[{col_num}]"),
            _ => format!("R{row_num}C{col_num}"),
        }
    };
    // sheet_text: `Sheet1!$A$1`, quoted when the name needs it (`'My Sheet'!`)
    if args.len() > 4 {
        let sheet = evaluate(&args[4], ctx)?.as_text();
        let plain = sheet
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            && !sheet.starts_with(|c: char| c.is_ascii_digit());
        let prefix = if plain {
            sheet
        } else {
            format!("'{}'", sheet.replace('\'', "''"))
        };
        return Ok(Value::Text(format!("{prefix}!{address}")));
    }
    Ok(Value::Text(address))
}

/// Evaluate ROW function
/// ROW([reference])
/// Returns the row number of a reference
pub fn eval_row(args: &[Expr], ctx: &EvalContext) -> Value {
    // ROW() returns current row number (1-based)
    // ROW(reference) returns the row number of the reference
    if args.is_empty() {
        // Return current row if available
        Value::Number(ctx.row().map_or(1.0, |row| precision::count(row + 1)))
    } else {
        // For a reference, we'd need to parse it. Simplified: return 1
        Value::Number(1.0)
    }
}

/// Evaluate COLUMN function
/// COLUMN([reference])
/// Returns the column number of a reference
pub const fn eval_column(_args: &[Expr], _ctx: &EvalContext) -> Value {
    // COLUMN() returns current column number
    // Simplified implementation - always returns 1
    Value::Number(1.0)
}

/// Evaluate ROWS function
/// ROWS(array)
/// Returns the number of rows in a reference or array
pub fn eval_rows(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("ROWS", args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    match val {
        Value::Array(arr) => Ok(Value::Number(precision::count(arr.len()))),
        _ => Ok(Value::Number(1.0)),
    }
}

/// Evaluate COLUMNS function
/// COLUMNS(array)
/// Returns the number of columns in a reference or array
pub fn eval_columns(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("COLUMNS", args, 1)?;
    // Check if the argument is a table reference (bare table name)
    if let Expr::Reference(crate::core::array_calculator::parser::Reference::Scalar(name)) =
        &args[0]
    {
        // Check if this name exists in tables
        if let Some(table) = ctx.tables.get(name) {
            return Ok(Value::Number(precision::count(table.len())));
        }
    }
    // For 1D arrays (single column), columns is always 1
    Ok(Value::Number(1.0))
}

/// Convert column number (1-based) to Excel-style letter(s)
pub fn col_to_letter(col: usize) -> String {
    const LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut result = String::new();
    let mut n = col;
    while n > 0 {
        n -= 1;
        result.insert(0, char::from(LETTERS[n % 26]));
        n /= 26;
    }
    result
}

#[cfg(test)]
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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

        assert_eq!(eval("ROWS(t.col)", &ctx).unwrap(), Value::Number(4.0));
    }

    #[test]
    fn test_columns() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert("col".to_string(), vec![Value::Number(1.0)]);
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        assert_exact!(val, 60.0);
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
        assert_exact!(val, 600.0);
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
        assert_exact!(val, 20.0);
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
        assert_exact!(val, 1.0);
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
        assert_exact!(val, 10.0);
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
        assert_exact!(val, 1.0);
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
        assert_exact!(val, 6.0);
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
        assert_exact!(val, 5.0);
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
        assert_exact!(val, 1.0);
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
        assert_exact!(val, 30.0);
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
        assert_exact!(val, 1.0);
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
        assert_exact!(val, 4.0);
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
        assert_exact!(val, 4.0);
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
        assert_exact!(val, 2.0);
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
        assert_exact!(val, 20.0);
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
        assert_exact!(val, 600.0);
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
        assert_exact!(val, 15.0);
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
        assert_exact!(val, 20.0);
    }

    /// `ADDRESS` as Excel computes it: R1C1 brackets on the relative part,
    /// `sheet_text`, and #VALUE! outside the sheet (it gave `$A$0`).
    #[test]
    fn address_matches_excel() {
        let ctx = EvalContext::new();
        for (formula, text) in [
            ("ADDRESS(2, 3)", "$C$2"),
            ("ADDRESS(2, 3, 2)", "C$2"),
            ("ADDRESS(2, 3, 2, FALSE)", "R2C[3]"),
            ("ADDRESS(2, 3, 3, FALSE)", "R[2]C3"),
            ("ADDRESS(2, 3, 4, FALSE)", "R[2]C[3]"),
            ("ADDRESS(1, 1, 1, TRUE, \"Sheet1\")", "Sheet1!$A$1"),
            ("ADDRESS(1, 1, 1, TRUE, \"My Sheet\")", "'My Sheet'!$A$1"),
            ("ADDRESS(2, 3, 1, FALSE, \"S\")", "S!R2C3"),
        ] {
            assert_eq!(
                eval(formula, &ctx).unwrap(),
                Value::Text(text.to_string()),
                "{formula}"
            );
        }
        for formula in ["ADDRESS(0, 1)", "ADDRESS(1048577, 1)", "ADDRESS(1, 1, 5)"] {
            assert!(eval(formula, &ctx).is_err(), "{formula}");
        }
    }

    /// Rows, columns and `abs_num` are cut to whole numbers, as Excel cuts
    /// them; one past the sheet, or no place in the array, is an error.
    #[test]
    fn offsets_and_addresses_are_whole_places() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("v", h::numbers(&[10.0, 20.0, 30.0]))]);
        h::is_exactly_in("OFFSET(t.v, 1.9, 0)", &ctx, 20.0);
        h::is_exactly_in("OFFSET(t.v, -0.5, 0)", &ctx, 10.0);
        assert!(h::error_in("OFFSET(t.v, -1, 0)", &ctx).contains("out of bounds"));
        assert!(h::error_in("OFFSET(t.v, 3, 0)", &ctx).contains("out of bounds"));
        assert!(h::error_in("OFFSET(t.v, 1E+20, 0)", &ctx).contains("out of bounds"));
        h::is_exactly("OFFSET(5, 0.5, -0.5)", 5.0);
        h::fails_with("OFFSET(5, 1, 0)", "cannot offset");
        assert_eq!(h::text("ADDRESS(1.9, 2.9)"), "$B$1");
        assert_eq!(h::text("ADDRESS(1048576, 16384)"), "$XFD$1048576");
        assert_eq!(h::text("ADDRESS(1, 1, 4.9)"), "A1");
        h::fails_with("ADDRESS(1048577, 1)", "row out of range");
        h::fails_with("ADDRESS(1E+20, 1)", "row out of range");
        h::fails_with("ADDRESS(0.5, 1)", "row out of range");
        h::fails_with("ADDRESS(1, 16385)", "column out of range");
        h::fails_with("ADDRESS(1, -1)", "column out of range");
        h::fails_with("ADDRESS(1, 1, 1E+20)", "abs_num");
        h::fails_with("ADDRESS(1, 1, 0)", "abs_num");
        assert_eq!(col_to_letter(26), "Z");
        assert_eq!(col_to_letter(27), "AA");
        assert_eq!(col_to_letter(16384), "XFD");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0]))]);
        assert!(
            h::error_in("OFFSET(t.v, \"x\", 0)", &ctx).contains("rows must be a number"),
            "OFFSET(t.v, \"x\", 0)"
        );
        assert!(
            h::error_in("OFFSET(t.v, 0, \"x\")", &ctx).contains("cols must be a number"),
            "OFFSET(t.v, 0, \"x\")"
        );
        assert!(
            h::error_in("ADDRESS(\"x\", 1)", &ctx).contains("row must be a number"),
            "ADDRESS(\"x\", 1)"
        );
        assert!(
            h::error_in("ADDRESS(1, \"x\")", &ctx).contains("column must be a number"),
            "ADDRESS(1, \"x\")"
        );
        assert!(
            h::error_in("ADDRESS(1, 1, 1/0)", &ctx).contains(""),
            "ADDRESS(1, 1, 1/0)"
        );
    }

    /// An error in any argument is passed on, and text where a number is
    /// needed is refused
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        crate::core::array_calculator::evaluator::tests::fails_with(
            "INDIRECT(nope)",
            "Unknown variable: nope",
        );
        each_argument(
            "OFFSET",
            &[
                ("{1, 2, 3}", None),
                ("1", Some("OFFSET: rows must be a number")),
                ("0", Some("OFFSET: cols must be a number")),
            ],
        );
        each_argument(
            "ADDRESS",
            &[
                ("1", Some("ADDRESS: row must be a number")),
                ("1", Some("ADDRESS: column must be a number")),
                ("1", None),
                ("TRUE", None),
                ("\"S\"", None),
            ],
        );
        each_argument("ROWS", &[("{1, 2}", None)]);
    }

    /// INDIRECT finds a scalar or a table's column, and nothing else
    #[test]
    fn indirect_finds_a_scalar_or_a_column() {
        use crate::core::array_calculator::evaluator::tests as h;
        let mut ctx = h::table(&[("col", h::numbers(&[1.0, 2.0]))]);
        ctx.scalars.insert("rate".into(), Value::Number(0.5));
        h::is_exactly_in("INDIRECT(\"rate\")", &ctx, 0.5);
        h::is_exactly_in("SUM(INDIRECT(\"t.col\"))", &ctx, 3.0);
        for missing in ["t.nope", "nope.col", "nope"] {
            h::fails_in(
                &format!("INDIRECT(\"{missing}\")"),
                &ctx,
                &format!("INDIRECT: cannot resolve '{missing}'"),
            );
        }
    }

    /// R1C1 with `abs_num` 1 is absolute both ways; ROWS of one value is 1;
    /// COLUMNS of a table is its number of columns
    #[test]
    fn r1c1_rows_and_columns() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(h::text("ADDRESS(2, 3, 1, FALSE)"), "R2C3");
        h::is_exactly("ROWS(5)", 1.0);
        h::is_exactly_in("COLUMNS(t)", &h::sweep_table(), 4.0);
    }

    /// The simplified ROW and COLUMN, absolute and relative parts of an
    /// address, and a name that is no table has one column
    #[test]
    fn rows_columns_and_addresses() {
        use crate::core::array_calculator::evaluator::tests as h;
        let mut ctx = EvalContext::new();
        ctx.scalars.insert("x".into(), Value::Number(3.0));
        h::is_exactly_in("ROW(x)", &ctx, 1.0);
        h::is_exactly_in("COLUMN()", &ctx, 1.0);
        h::is_exactly_in("COLUMNS(x)", &ctx, 1.0);
        assert_eq!(h::text("ADDRESS(2, 3, 3)"), "$C2");
    }
}
