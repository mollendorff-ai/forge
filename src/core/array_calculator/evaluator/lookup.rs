//! Lookup functions: INDEX, MATCH, CHOOSE, XLOOKUP, INDIRECT

use super::{
    evaluate, require_args, require_args_range, values_equal, EvalContext, EvalError, Expr, Value,
};

/// Try to evaluate a lookup function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
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

        _ => return Ok(None),
    };

    Ok(Some(result))
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
}
