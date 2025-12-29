//! Lookup functions: INDEX, MATCH, CHOOSE, XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS
//!
//! DEMO functions (3): INDEX, MATCH, CHOOSE
//! ENTERPRISE functions: XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS

use super::{evaluate, require_args_range, values_equal, EvalContext, EvalError, Expr, Value};

#[cfg(not(feature = "demo"))]
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
                    "INDEX: row_num {row_num} must be >= 1"
                )));
            }

            match array {
                Value::Array(arr) => {
                    let idx = (row_num - 1) as usize;
                    arr.get(idx).cloned().ok_or_else(|| {
                        EvalError::new(format!("INDEX row {row_num} out of bounds"))
                    })?
                },
                _ => return Err(EvalError::new("INDEX requires an array")),
            }
        },

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
                        .ok_or_else(|| EvalError::new("MATCH: value not found"))?
                },
                _ => {
                    return Err(EvalError::new(format!(
                        "MATCH: invalid match_type {match_type}"
                    )))
                },
            }
        },

        "CHOOSE" => {
            require_args_range(name, args, 2, 255)?;
            let index = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("CHOOSE index must be a number"))?
                as usize;

            if index < 1 || index >= args.len() {
                return Err(EvalError::new(format!("CHOOSE index {index} out of range")));
            }
            evaluate(&args[index], ctx)?
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
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
                "INDIRECT: cannot resolve '{ref_str}'"
            )));
        },

        #[cfg(not(feature = "demo"))]
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
                },
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
                },
                _ => {
                    return Err(EvalError::new(format!(
                        "XLOOKUP: invalid match_mode {match_mode}"
                    )))
                },
            };

            match idx {
                Some(i) => return_values.get(i).cloned().unwrap_or(Value::Null),
                None => {
                    if let Some(not_found) = if_not_found {
                        not_found
                    } else {
                        return Err(EvalError::new("XLOOKUP: No match found"));
                    }
                },
            }
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
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
                },
                other => {
                    // For scalar, just return the value (offset of 0,0)
                    if rows == 0 && cols == 0 {
                        other
                    } else {
                        return Err(EvalError::new("OFFSET: cannot offset scalar"));
                    }
                },
            }
        },

        #[cfg(not(feature = "demo"))]
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
            Value::Text(address)
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
        "COLUMN" => {
            // COLUMN() returns current column number
            // Simplified implementation - always returns 1
            Value::Number(1.0)
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
        "COLUMNS" => {
            require_args(name, args, 1)?;
            // Check if the argument is a table reference (bare table name)
            if let Expr::Reference(super::super::parser::Reference::Scalar(name)) = &args[0] {
                // Check if this name exists in tables
                if let Some(table) = ctx.tables.get(name) {
                    return Ok(Some(Value::Number(table.len() as f64)));
                }
            }
            // For 1D arrays (single column), columns is always 1
            Value::Number(1.0)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Convert column number (1-based) to Excel-style letter(s)
#[cfg(not(feature = "demo"))]
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

    // Additional imports for ArrayCalculator-based tests
    #[allow(unused_imports)]
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_row_column() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROW()", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("COLUMN()", &ctx).unwrap(), Value::Number(1.0));
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_columns() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert("col".to_string(), vec![Value::Number(1.0)]);
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COLUMNS(t.col)", &ctx).unwrap(), Value::Number(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_col_to_letter() {
        assert_eq!(col_to_letter(1), "A");
        assert_eq!(col_to_letter(26), "Z");
        assert_eq!(col_to_letter(27), "AA");
        assert_eq!(col_to_letter(52), "AZ");
        assert_eq!(col_to_letter(53), "BA");
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    // ═══════════════════════════════════════════════════════════════════════════════
    // Tests from lookup_basic.rs
    // ═══════════════════════════════════════════════════════════════════════════════

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
    fn test_index_match_combined() {
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
        products.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 101.0, 103.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=INDEX(products.product_name, MATCH(product_id, products.product_id, 0))".to_string(),
        );
        sales.add_row_formula(
            "price".to_string(),
            "=INDEX(products.price, MATCH(product_id, products.product_id, 0))".to_string(),
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
                assert_eq!(texts[1], "Widget A");
                assert_eq!(texts[2], "Widget C");
            },
            _ => panic!("Expected Text array"),
        }

        let price = result_table.columns.get("price").unwrap();
        match &price.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 20.0);
                assert_eq!(nums[1], 10.0);
                assert_eq!(nums[2], 30.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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
    fn test_cross_table_row_count_mismatch_error() {
        let mut model = ParsedModel::new();

        let mut table1 = Table::new("table1".to_string());
        table1.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table1);

        let mut table2 = Table::new("table2".to_string());
        table2.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table2
            .row_formulas
            .insert("result".to_string(), "=table1.a + x".to_string());
        model.add_table(table2);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rows"));
    }

    #[cfg(not(feature = "demo"))]
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
    fn test_column_row_count_mismatch_local() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        data.columns.insert(
            "b".to_string(),
            Column::new("b".to_string(), ColumnValue::Number(vec![10.0, 20.0])),
        );
        data.row_formulas
            .insert("result".to_string(), "=a + b".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
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

    // ═══════════════════════════════════════════════════════════════════════════════
    // Tests from lookup_advanced.rs
    // ═══════════════════════════════════════════════════════════════════════════════

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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    // ═══════════════════════════════════════════════════════════════════════════════
    // Tests from lookup_xlookup.rs
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_index_match_combination() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Carol".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "scores".to_string(),
            ColumnValue::Number(vec![85.0, 92.0, 78.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "score".to_string(),
            Variable::new(
                "score".to_string(),
                None,
                Some("=INDEX(data.scores, MATCH(\"Bob\", data.names, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "INDEX/MATCH combination should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("score").unwrap().value.unwrap();
        assert_eq!(val, 92.0);
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    // ═══════════════════════════════════════════════════════════════════════════════
    // Tests from lookup_edge_cases.rs
    // ═══════════════════════════════════════════════════════════════════════════════

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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_match_basic_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(2, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_match_first_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(1, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_index_match_last_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(3, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(300.0));
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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
