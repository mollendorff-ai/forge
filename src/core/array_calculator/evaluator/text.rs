//! Text functions: CONCAT, UPPER, LOWER, TRIM, LEN, LEFT, RIGHT, MID, REPT, TEXT, VALUE, FIND, SEARCH, REPLACE, SUBSTITUTE
//!
//! DEMO functions (9): CONCAT, LEFT, RIGHT, MID, REPT, LEN, UPPER, LOWER, TRIM
//! ENTERPRISE functions: CONCATENATE, TEXT, VALUE, FIND, SEARCH, REPLACE, SUBSTITUTE

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a text function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "CONCAT" => {
            let mut result = String::new();
            for arg in args {
                let val = evaluate(arg, ctx)?;
                result.push_str(&val.as_text());
            }
            Value::Text(result)
        }

        "UPPER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_uppercase())
        }

        "LOWER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_lowercase())
        }

        "TRIM" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().trim().to_string())
        }

        "LEN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Number(val.as_text().chars().count() as f64)
        }

        "LEFT" => {
            require_args_range(name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().take(n).collect();
            Value::Text(chars.into_iter().collect())
        }

        "RIGHT" => {
            require_args_range(name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(n);
            Value::Text(chars[start..].iter().collect())
        }

        "MID" => {
            require_args(name, args, 3)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let start = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize;
            let length = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as usize;

            let chars: Vec<char> = text.chars().collect();
            // Excel MID is 1-indexed
            let start_idx = start.saturating_sub(1);

            // If start is beyond string length, return empty string
            if start_idx >= chars.len() {
                return Ok(Some(Value::Text(String::new())));
            }

            let end_idx = (start_idx + length).min(chars.len());
            Value::Text(chars[start_idx..end_idx].iter().collect())
        }

        "REPT" => {
            require_args(name, args, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let times = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as usize;
            Value::Text(text.repeat(times))
        }

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        "CONCATENATE" => {
            // Excel's CONCATENATE (alias for CONCAT, enterprise only)
            let mut result = String::new();
            for arg in args {
                let val = evaluate(arg, ctx)?;
                result.push_str(&val.as_text());
            }
            Value::Text(result)
        }

        #[cfg(feature = "full")]
        "TEXT" => {
            require_args(name, args, 2)?;
            let val = evaluate(&args[0], ctx)?;
            let format = evaluate(&args[1], ctx)?.as_text();
            // Simplified TEXT implementation - basic number formatting
            let num = val.as_number().unwrap_or(0.0);
            let formatted = format_number(num, &format);
            Value::Text(formatted)
        }

        #[cfg(feature = "full")]
        "VALUE" => {
            require_args(name, args, 1)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            // Parse the text as a number
            let num = text
                .trim()
                .replace(',', "") // Remove thousand separators
                .parse::<f64>()
                .map_err(|_| {
                    EvalError::new(format!("VALUE: Cannot convert '{}' to number", text))
                })?;
            Value::Number(num)
        }

        #[cfg(feature = "full")]
        "FIND" => {
            require_args_range(name, args, 2, 3)?;
            let find_text = evaluate(&args[0], ctx)?.as_text();
            let within_text = evaluate(&args[1], ctx)?.as_text();
            let start_num = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };

            // FIND is case-sensitive and 1-indexed
            let start_idx = start_num.saturating_sub(1);
            if start_idx >= within_text.len() {
                return Err(EvalError::new("FIND: start_num out of range"));
            }

            match within_text[start_idx..].find(&find_text) {
                Some(pos) => Value::Number((pos + start_num) as f64),
                None => return Err(EvalError::new("FIND: text not found")),
            }
        }

        #[cfg(feature = "full")]
        "SEARCH" => {
            require_args_range(name, args, 2, 3)?;
            let find_text = evaluate(&args[0], ctx)?.as_text().to_lowercase();
            let within_text = evaluate(&args[1], ctx)?.as_text();
            let start_num = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };

            // SEARCH is case-insensitive and 1-indexed
            let start_idx = start_num.saturating_sub(1);
            if start_idx >= within_text.len() {
                return Err(EvalError::new("SEARCH: start_num out of range"));
            }

            let search_in = within_text[start_idx..].to_lowercase();
            match search_in.find(&find_text) {
                Some(pos) => Value::Number((pos + start_num) as f64),
                None => return Err(EvalError::new("SEARCH: text not found")),
            }
        }

        #[cfg(feature = "full")]
        "REPLACE" => {
            require_args(name, args, 4)?;
            let old_text = evaluate(&args[0], ctx)?.as_text();
            let start_num = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize;
            let num_chars = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as usize;
            let new_text = evaluate(&args[3], ctx)?.as_text();

            // REPLACE is position-based (1-indexed)
            let chars: Vec<char> = old_text.chars().collect();
            let start_idx = start_num.saturating_sub(1);
            let end_idx = (start_idx + num_chars).min(chars.len());

            let prefix: String = chars[..start_idx].iter().collect();
            let suffix: String = chars[end_idx..].iter().collect();

            Value::Text(format!("{}{}{}", prefix, new_text, suffix))
        }

        #[cfg(feature = "full")]
        "SUBSTITUTE" => {
            require_args_range(name, args, 3, 4)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let old_text = evaluate(&args[1], ctx)?.as_text();
            let new_text = evaluate(&args[2], ctx)?.as_text();

            // If old_text is empty, return original text unchanged (Excel behavior)
            if old_text.is_empty() {
                return Ok(Some(Value::Text(text)));
            }

            if args.len() > 3 {
                // Replace only the nth occurrence
                let instance = evaluate(&args[3], ctx)?.as_number().unwrap_or(1.0) as usize;
                let mut result = text.clone();
                let mut count = 0;
                let mut pos = 0;

                while let Some(found) = result[pos..].find(&old_text) {
                    count += 1;
                    if count == instance {
                        let abs_pos = pos + found;
                        result = format!(
                            "{}{}{}",
                            &result[..abs_pos],
                            new_text,
                            &result[abs_pos + old_text.len()..]
                        );
                        break;
                    }
                    pos += found + old_text.len();
                }
                Value::Text(result)
            } else {
                // Replace all occurrences
                Value::Text(text.replace(&old_text, &new_text))
            }
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Format a number according to a format string (simplified implementation)
#[cfg(feature = "full")]
fn format_number(num: f64, format: &str) -> String {
    let format_upper = format.to_uppercase();

    // Handle percentage formats
    if format.contains('%') {
        let decimal_places = format.matches('0').count().saturating_sub(1);
        return format!("{:.prec$}%", num * 100.0, prec = decimal_places);
    }

    // Handle currency formats
    if format.starts_with('$') || format.starts_with("[$") {
        let decimal_places = format
            .rfind('.')
            .map(|i| format[i + 1..].chars().take_while(|c| *c == '0').count())
            .unwrap_or(2);
        return format!("${:.prec$}", num, prec = decimal_places);
    }

    // Handle fixed decimal formats like "0.00"
    if let Some(dot_pos) = format.find('.') {
        let decimal_places = format[dot_pos + 1..].len();
        return format!("{:.prec$}", num, prec = decimal_places);
    }

    // Handle scientific notation
    if format_upper.contains('E') {
        return format!("{:E}", num);
    }

    // Handle comma thousands separator
    if format.contains(',') {
        let int_part = num.trunc() as i64;
        let frac_part = num.fract();
        let formatted_int = int_part
            .to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap_or_default()
            .join(",");

        if frac_part == 0.0 {
            return formatted_int;
        } else {
            return format!("{}{:.2}", formatted_int, frac_part);
        }
    }

    // Default: just convert to string
    if num.fract() == 0.0 {
        format!("{}", num as i64)
    } else {
        format!("{}", num)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    #[test]
    fn test_text_functions() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("UPPER(\"hello\")", &ctx).unwrap(),
            Value::Text("HELLO".to_string())
        );
        assert_eq!(
            eval("LOWER(\"HELLO\")", &ctx).unwrap(),
            Value::Text("hello".to_string())
        );
        assert_eq!(
            eval("CONCAT(\"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("abc".to_string())
        );
        assert_eq!(eval("LEN(\"hello\")", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(
            eval("LEFT(\"hello\", 2)", &ctx).unwrap(),
            Value::Text("he".to_string())
        );
        assert_eq!(
            eval("RIGHT(\"hello\", 2)", &ctx).unwrap(),
            Value::Text("lo".to_string())
        );
        assert_eq!(
            eval("MID(\"hello\", 2, 3)", &ctx).unwrap(),
            Value::Text("ell".to_string())
        );
    }

    #[test]
    fn test_trim() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("TRIM(\"  hello  \")", &ctx).unwrap(),
            Value::Text("hello".to_string())
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(feature = "full")]
    #[test]
    fn test_text() {
        let ctx = EvalContext::new();
        // Percentage format
        assert_eq!(
            eval("TEXT(0.25, \"0%\")", &ctx).unwrap(),
            Value::Text("25%".to_string())
        );
        // Currency format
        assert_eq!(
            eval("TEXT(1234.5, \"$0.00\")", &ctx).unwrap(),
            Value::Text("$1234.50".to_string())
        );
        // Fixed decimal
        assert_eq!(
            eval("TEXT(3.14159, \"0.00\")", &ctx).unwrap(),
            Value::Text("3.14".to_string())
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_value() {
        let ctx = EvalContext::new();
        assert_eq!(eval("VALUE(\"123\")", &ctx).unwrap(), Value::Number(123.0));
        assert_eq!(
            eval("VALUE(\"  45.67  \")", &ctx).unwrap(),
            Value::Number(45.67)
        );
        assert_eq!(
            eval("VALUE(\"1,234\")", &ctx).unwrap(),
            Value::Number(1234.0)
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_value_error() {
        let ctx = EvalContext::new();
        assert!(eval("VALUE(\"abc\")", &ctx).is_err());
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_find() {
        let ctx = EvalContext::new();
        // Basic find
        assert_eq!(
            eval("FIND(\"l\", \"hello\")", &ctx).unwrap(),
            Value::Number(3.0)
        );
        // With start position
        assert_eq!(
            eval("FIND(\"l\", \"hello\", 4)", &ctx).unwrap(),
            Value::Number(4.0)
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_find_not_found() {
        let ctx = EvalContext::new();
        assert!(eval("FIND(\"x\", \"hello\")", &ctx).is_err());
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_search() {
        let ctx = EvalContext::new();
        // Case insensitive
        assert_eq!(
            eval("SEARCH(\"L\", \"hello\")", &ctx).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            eval("SEARCH(\"HELLO\", \"hello world\")", &ctx).unwrap(),
            Value::Number(1.0)
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_replace() {
        let ctx = EvalContext::new();
        // REPLACE(old_text, start, num_chars, new_text)
        assert_eq!(
            eval("REPLACE(\"hello\", 1, 2, \"XX\")", &ctx).unwrap(),
            Value::Text("XXllo".to_string())
        );
        assert_eq!(
            eval("REPLACE(\"hello\", 3, 2, \"XXX\")", &ctx).unwrap(),
            Value::Text("heXXXo".to_string())
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_substitute() {
        let ctx = EvalContext::new();
        // Replace all occurrences
        assert_eq!(
            eval("SUBSTITUTE(\"hello\", \"l\", \"X\")", &ctx).unwrap(),
            Value::Text("heXXo".to_string())
        );
        // Replace specific occurrence
        assert_eq!(
            eval("SUBSTITUTE(\"hello\", \"l\", \"X\", 1)", &ctx).unwrap(),
            Value::Text("heXlo".to_string())
        );
        assert_eq!(
            eval("SUBSTITUTE(\"hello\", \"l\", \"X\", 2)", &ctx).unwrap(),
            Value::Text("helXo".to_string())
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_format_number_helper() {
        // Test the helper function directly
        assert_eq!(format_number(0.25, "0%"), "25%");
        assert_eq!(format_number(1234.0, "$0.00"), "$1234.00");
        assert_eq!(format_number(1.2345, "0.000"), "1.234");
        assert_eq!(format_number(1000000.0, "#,##0"), "1,000,000");
    }
}
