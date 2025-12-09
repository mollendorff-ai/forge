//! Lookup functions: INDEX, MATCH, CHOOSE, XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS
//!
//! DEMO functions (3): INDEX, MATCH, CHOOSE
//! ENTERPRISE functions: XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS

use super::{evaluate, require_args_range, values_equal, EvalContext, EvalError, Expr, Value};

#[cfg(feature = "full")]
use super::require_args;

/// Try to evaluate a lookup function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "INDEX" => {
            require_args_range(name, args, 2, 3)?;

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
                    "INDEX: row_num {} must be >= 1",
                    row_num
                )));
            }

            match array {
                Value::Array(arr) => {
                    let idx = (row_num - 1) as usize;
                    arr.get(idx).cloned().ok_or_else(|| {
                        EvalError::new(format!("INDEX row {} out of bounds", row_num))
                    })?
                }
                _ => return Err(EvalError::new("INDEX requires an array")),
            }
        }

        "MATCH" => {
            require_args_range(name, args, 2, 3)?;

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
                            return Ok(Some(Value::Number((i + 1) as f64)));
                        }
                    }
                    return Err(EvalError::new("MATCH: value not found"));
                }
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
                            .ok_or_else(|| EvalError::new("MATCH: value not found"))?
                    } else {
                        let lookup_text = lookup_value.as_text().to_lowercase();
                        for (i, val) in arr.iter().enumerate() {
                            if val.as_text().to_lowercase() == lookup_text {
                                return Ok(Some(Value::Number((i + 1) as f64)));
                            }
                        }
                        return Err(EvalError::new("MATCH: value not found"));
                    }
                }
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
                        .ok_or_else(|| EvalError::new("MATCH: value not found"))?
                }
                _ => {
                    return Err(EvalError::new(format!(
                        "MATCH: invalid match_type {}",
                        match_type
                    )))
                }
            }
        }

        "CHOOSE" => {
            require_args_range(name, args, 2, 255)?;
            let index = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("CHOOSE index must be a number"))?
                as usize;

            if index < 1 || index >= args.len() {
                return Err(EvalError::new(format!(
                    "CHOOSE index {} out of range",
                    index
                )));
            }
            evaluate(&args[index], ctx)?
        }

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        "INDIRECT" => {
            require_args(name, args, 1)?;
            let ref_str = evaluate(&args[0], ctx)?.as_text();
            if let Some(val) = ctx.scalars.get(&ref_str) {
                return Ok(Some(val.clone()));
            }
            if ref_str.contains('.') {
                let parts: Vec<&str> = ref_str.splitn(2, '.').collect();
                if parts.len() == 2 {
                    if let Some(table) = ctx.tables.get(parts[0]) {
                        if let Some(col) = table.get(parts[1]) {
                            return Ok(Some(Value::Array(col.clone())));
                        }
                    }
                }
            }
            return Err(EvalError::new(format!(
                "INDIRECT: cannot resolve '{}'",
                ref_str
            )));
        }

        #[cfg(feature = "full")]
        "XLOOKUP" => {
            require_args_range(name, args, 3, 6)?;

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
                            return Ok(Some(return_values.get(i).cloned().unwrap_or(Value::Null)));
                        }
                        if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                            if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                                best_val = Some(vn);
                                best_idx = Some(i);
                            }
                        }
                    }
                    best_idx
                }
                1 => {
                    let mut best_idx: Option<usize> = None;
                    let mut best_val: Option<f64> = None;
                    let lookup_num = lookup_val.as_number();

                    for (i, v) in lookup_values.iter().enumerate() {
                        if values_equal(v, &lookup_val) {
                            return Ok(Some(return_values.get(i).cloned().unwrap_or(Value::Null)));
                        }
                        if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                            if vn >= ln && (best_val.is_none() || vn < best_val.unwrap()) {
                                best_val = Some(vn);
                                best_idx = Some(i);
                            }
                        }
                    }
                    best_idx
                }
                _ => {
                    return Err(EvalError::new(format!(
                        "XLOOKUP: invalid match_mode {}",
                        match_mode
                    )))
                }
            };

            match idx {
                Some(i) => return_values.get(i).cloned().unwrap_or(Value::Null),
                None => {
                    if let Some(not_found) = if_not_found {
                        not_found
                    } else {
                        return Err(EvalError::new("XLOOKUP: No match found"));
                    }
                }
            }
        }

        #[cfg(feature = "full")]
        "VLOOKUP" => {
            require_args_range(name, args, 3, 4)?;

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
                Some(i) => lookup_arr.get(i).cloned().unwrap_or(Value::Null),
                None => return Err(EvalError::new("VLOOKUP: value not found")),
            }
        }

        #[cfg(feature = "full")]
        "HLOOKUP" => {
            require_args_range(name, args, 3, 4)?;

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
                Some(i) => lookup_arr.get(i).cloned().unwrap_or(Value::Null),
                None => return Err(EvalError::new("HLOOKUP: value not found")),
            }
        }

        #[cfg(feature = "full")]
        "OFFSET" => {
            require_args_range(name, args, 3, 5)?;

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
                .ok_or_else(|| EvalError::new("OFFSET: rows must be a number"))?
                as i64;
            let cols = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("OFFSET: cols must be a number"))?
                as i64;

            // OFFSET returns a reference offset by rows and cols
            // Simplified: for array, return element at offset position
            match base {
                Value::Array(arr) => {
                    if rows < 0 || rows as usize >= arr.len() {
                        return Err(EvalError::new("OFFSET: row out of bounds"));
                    }
                    arr.get(rows as usize).cloned().unwrap_or(Value::Null)
                }
                other => {
                    // For scalar, just return the value (offset of 0,0)
                    if rows == 0 && cols == 0 {
                        other
                    } else {
                        return Err(EvalError::new("OFFSET: cannot offset scalar"));
                    }
                }
            }
        }

        #[cfg(feature = "full")]
        "ADDRESS" => {
            require_args_range(name, args, 2, 5)?;
            let row_num = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ADDRESS: row must be a number"))?
                as i64;
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
                    1 => format!("${}${}", col_letter, row_num),
                    2 => format!("{}${}", col_letter, row_num),
                    3 => format!("${}{}", col_letter, row_num),
                    4 => format!("{}{}", col_letter, row_num),
                    _ => format!("${}${}", col_letter, row_num),
                }
            } else {
                // R1C1 style
                match abs_num {
                    1 => format!("R{}C{}", row_num, col_num),
                    4 => format!("R[{}]C[{}]", row_num, col_num),
                    _ => format!("R{}C{}", row_num, col_num),
                }
            };
            Value::Text(address)
        }

        #[cfg(feature = "full")]
        "ROW" => {
            // ROW() returns current row number (1-based)
            // ROW(reference) returns the row number of the reference
            if args.is_empty() {
                // Return current row if available
                if let Some(row) = ctx.current_row {
                    Value::Number((row + 1) as f64)
                } else {
                    Value::Number(1.0)
                }
            } else {
                // For a reference, we'd need to parse it. Simplified: return 1
                Value::Number(1.0)
            }
        }

        #[cfg(feature = "full")]
        "COLUMN" => {
            // COLUMN() returns current column number
            // Simplified implementation - always returns 1
            Value::Number(1.0)
        }

        #[cfg(feature = "full")]
        "ROWS" => {
            require_args(name, args, 1)?;
            let array_ctx = EvalContext {
                scalars: ctx.scalars.clone(),
                tables: ctx.tables.clone(),
                scenarios: ctx.scenarios.clone(),
                current_row: None,
                row_count: ctx.row_count,
            };
            let val = evaluate(&args[0], &array_ctx)?;
            match val {
                Value::Array(arr) => Value::Number(arr.len() as f64),
                _ => Value::Number(1.0),
            }
        }

        #[cfg(feature = "full")]
        "COLUMNS" => {
            require_args(name, args, 1)?;
            // For 1D arrays, columns is always 1
            // For Forge's model, we treat arrays as single-column
            Value::Number(1.0)
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Convert column number (1-based) to Excel-style letter(s)
#[cfg(feature = "full")]
fn col_to_letter(col: usize) -> String {
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
mod tests {
    use super::super::tests::eval;
    use super::*;
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

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(feature = "full")]
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

    #[cfg(feature = "full")]
    #[test]
    fn test_row_column() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROW()", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("COLUMN()", &ctx).unwrap(), Value::Number(1.0));
    }

    #[cfg(feature = "full")]
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

    #[cfg(feature = "full")]
    #[test]
    fn test_columns() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert("col".to_string(), vec![Value::Number(1.0)]);
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COLUMNS(t.col)", &ctx).unwrap(), Value::Number(1.0));
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_col_to_letter() {
        assert_eq!(col_to_letter(1), "A");
        assert_eq!(col_to_letter(26), "Z");
        assert_eq!(col_to_letter(27), "AA");
        assert_eq!(col_to_letter(52), "AZ");
        assert_eq!(col_to_letter(53), "BA");
    }

    #[cfg(feature = "full")]
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

    #[cfg(feature = "full")]
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
}
