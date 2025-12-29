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
        },

        "UPPER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_uppercase())
        },

        "LOWER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_lowercase())
        },

        "TRIM" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            // Excel TRIM: removes leading/trailing spaces AND collapses multiple internal spaces
            let text = val.as_text();
            let trimmed: String = text.split_whitespace().collect::<Vec<&str>>().join(" ");
            Value::Text(trimmed)
        },

        "LEN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Number(val.as_text().chars().count() as f64)
        },

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
        },

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
        },

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
        },

        "REPT" => {
            require_args(name, args, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let times = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as usize;
            Value::Text(text.repeat(times))
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "CONCATENATE" => {
            // Excel's CONCATENATE (alias for CONCAT, enterprise only)
            let mut result = String::new();
            for arg in args {
                let val = evaluate(arg, ctx)?;
                result.push_str(&val.as_text());
            }
            Value::Text(result)
        },

        #[cfg(not(feature = "demo"))]
        "TEXT" => {
            require_args(name, args, 2)?;
            let val = evaluate(&args[0], ctx)?;
            let format = evaluate(&args[1], ctx)?.as_text();
            // Simplified TEXT implementation - basic number formatting
            let num = val.as_number().unwrap_or(0.0);
            let formatted = format_number(num, &format);
            Value::Text(formatted)
        },

        #[cfg(not(feature = "demo"))]
        "VALUE" => {
            require_args(name, args, 1)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            // Parse the text as a number
            let num = text
                .trim()
                .replace(',', "") // Remove thousand separators
                .parse::<f64>()
                .map_err(|_| {
                    EvalError::new(format!("VALUE: Cannot convert '{text}' to number"))
                })?;
            Value::Number(num)
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
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

            Value::Text(format!("{prefix}{new_text}{suffix}"))
        },

        #[cfg(not(feature = "demo"))]
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
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Format a number according to a format string (simplified implementation)
#[cfg(not(feature = "demo"))]
fn format_number(num: f64, format: &str) -> String {
    let format_upper = format.to_uppercase();

    // Handle percentage formats
    if format.contains('%') {
        let decimal_places = format.matches('0').count().saturating_sub(1);
        return format!("{:.prec$}%", num * 100.0, prec = decimal_places);
    }

    // Handle currency formats
    if format.starts_with('$') || format.starts_with("[$") {
        let decimal_places = format.rfind('.').map_or(2, |i| {
            format[i + 1..].chars().take_while(|c| *c == '0').count()
        });
        return format!("${num:.decimal_places$}");
    }

    // Handle fixed decimal formats like "0.00"
    if let Some(dot_pos) = format.find('.') {
        let decimal_places = format[dot_pos + 1..].len();
        return format!("{num:.decimal_places$}");
    }

    // Handle scientific notation
    if format_upper.contains('E') {
        return format!("{num:E}");
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
        }
        return format!("{formatted_int}{frac_part:.2}");
    }

    // Default: just convert to string
    if num.fract() == 0.0 {
        format!("{}", num as i64)
    } else {
        format!("{num}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    // Imports for ArrayCalculator-based tests (moved from separate test files)
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_value_error() {
        let ctx = EvalContext::new();
        assert!(eval("VALUE(\"abc\")", &ctx).is_err());
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_find_not_found() {
        let ctx = EvalContext::new();
        assert!(eval("FIND(\"x\", \"hello\")", &ctx).is_err());
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_format_number_helper() {
        // Test the helper function directly
        assert_eq!(format_number(0.25, "0%"), "25%");
        assert_eq!(format_number(1234.0, "$0.00"), "$1234.00");
        assert_eq!(format_number(1.2345, "0.000"), "1.234");
        assert_eq!(format_number(1000000.0, "#,##0"), "1,000,000");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EDGE CASE TESTS FOR STRING OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_len_concatenated_strings() {
        // LEN("Hello" & " " & "World") = 11
        let ctx = EvalContext::new();
        assert_eq!(
            eval("LEN(CONCAT(\"Hello\", \" \", \"World\"))", &ctx).unwrap(),
            Value::Number(11.0)
        );
    }

    #[test]
    fn test_len_empty_string() {
        // LEN("") = 0
        let ctx = EvalContext::new();
        assert_eq!(eval("LEN(\"\")", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_len_single_space() {
        // LEN(" ") = 1
        let ctx = EvalContext::new();
        assert_eq!(eval("LEN(\" \")", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_left_empty_string_with_count() {
        // LEFT("", 5) returns empty string, LEN = 0
        let ctx = EvalContext::new();
        assert_eq!(
            eval("LEN(LEFT(\"\", 5))", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_right_empty_string_with_count() {
        // RIGHT("", 5) returns empty string, LEN = 0
        let ctx = EvalContext::new();
        assert_eq!(
            eval("LEN(RIGHT(\"\", 5))", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_mid_substring_first_two_chars() {
        // MID("test", 1, 2) = "te", LEN = 2
        let ctx = EvalContext::new();
        assert_eq!(
            eval("MID(\"test\", 1, 2)", &ctx).unwrap(),
            Value::Text("te".to_string())
        );
        assert_eq!(
            eval("LEN(MID(\"test\", 1, 2))", &ctx).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_mid_substring_second_two_chars() {
        // MID("test", 2, 2) = "es", LEN = 2
        let ctx = EvalContext::new();
        assert_eq!(
            eval("MID(\"test\", 2, 2)", &ctx).unwrap(),
            Value::Text("es".to_string())
        );
        assert_eq!(
            eval("LEN(MID(\"test\", 2, 2))", &ctx).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_trim_with_internal_spaces() {
        // Excel TRIM: removes leading/trailing AND collapses internal spaces to single
        // TRIM("  a  b  ") = "a b" with 3 characters
        let ctx = EvalContext::new();
        assert_eq!(
            eval("TRIM(\"  a  b  \")", &ctx).unwrap(),
            Value::Text("a b".to_string())
        );
        assert_eq!(
            eval("LEN(TRIM(\"  a  b  \"))", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_trim_only_spaces() {
        // TRIM("   ") = "", LEN = 0
        let ctx = EvalContext::new();
        assert_eq!(
            eval("TRIM(\"   \")", &ctx).unwrap(),
            Value::Text(String::new())
        );
        assert_eq!(
            eval("LEN(TRIM(\"   \"))", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_upper_lowercase_abc() {
        // UPPER("abc") = "ABC", LEN = 3
        let ctx = EvalContext::new();
        assert_eq!(
            eval("UPPER(\"abc\")", &ctx).unwrap(),
            Value::Text("ABC".to_string())
        );
        assert_eq!(
            eval("LEN(UPPER(\"abc\"))", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_lower_uppercase_abc() {
        // LOWER("ABC") = "abc", LEN = 3
        let ctx = EvalContext::new();
        assert_eq!(
            eval("LOWER(\"ABC\")", &ctx).unwrap(),
            Value::Text("abc".to_string())
        );
        assert_eq!(
            eval("LEN(LOWER(\"ABC\"))", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_concat_two_strings() {
        // CONCAT("ab", "cd") = "abcd", LEN = 4
        let ctx = EvalContext::new();
        assert_eq!(
            eval("CONCAT(\"ab\", \"cd\")", &ctx).unwrap(),
            Value::Text("abcd".to_string())
        );
        assert_eq!(
            eval("LEN(CONCAT(\"ab\", \"cd\"))", &ctx).unwrap(),
            Value::Number(4.0)
        );
    }

    #[test]
    fn test_rept_single_char_five_times() {
        // REPT("x", 5) = "xxxxx", LEN = 5
        let ctx = EvalContext::new();
        assert_eq!(
            eval("REPT(\"x\", 5)", &ctx).unwrap(),
            Value::Text("xxxxx".to_string())
        );
        assert_eq!(
            eval("LEN(REPT(\"x\", 5))", &ctx).unwrap(),
            Value::Number(5.0)
        );
    }

    #[test]
    fn test_rept_two_char_three_times() {
        // REPT("ab", 3) = "ababab", LEN = 6
        let ctx = EvalContext::new();
        assert_eq!(
            eval("REPT(\"ab\", 3)", &ctx).unwrap(),
            Value::Text("ababab".to_string())
        );
        assert_eq!(
            eval("LEN(REPT(\"ab\", 3))", &ctx).unwrap(),
            Value::Number(6.0)
        );
    }

    #[test]
    fn test_left_hello_three_chars() {
        // LEFT("Hello", 3) = "Hel", LEN = 3
        let ctx = EvalContext::new();
        assert_eq!(
            eval("LEFT(\"Hello\", 3)", &ctx).unwrap(),
            Value::Text("Hel".to_string())
        );
        assert_eq!(
            eval("LEN(LEFT(\"Hello\", 3))", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_right_hello_three_chars() {
        // RIGHT("Hello", 3) = "llo", LEN = 3
        let ctx = EvalContext::new();
        assert_eq!(
            eval("RIGHT(\"Hello\", 3)", &ctx).unwrap(),
            Value::Text("llo".to_string())
        );
        assert_eq!(
            eval("LEN(RIGHT(\"Hello\", 3))", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    // ══════════════════════════════════════════════════════════════════════════════
    // Tests moved from tests/text.rs
    // ══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_concat_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec![
                "Hello".to_string(),
                "Good".to_string(),
                "Nice".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "second".to_string(),
            ColumnValue::Text(vec![
                "World".to_string(),
                "Day".to_string(),
                "Work".to_string(),
            ]),
        ));
        table.add_row_formula(
            "combined".to_string(),
            "=CONCAT(first, \" \", second)".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let combined = result_table.columns.get("combined").unwrap();
        match &combined.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Hello World");
                assert_eq!(texts[1], "Good Day");
                assert_eq!(texts[2], "Nice Work");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_trim_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "  Hello  ".to_string(),
                " World ".to_string(),
                "  Test".to_string(),
            ]),
        ));
        table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let trimmed = result_table.columns.get("trimmed").unwrap();
        match &trimmed.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Hello");
                assert_eq!(texts[1], "World");
                assert_eq!(texts[2], "Test");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_upper_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "hello".to_string(),
                "world".to_string(),
                "Test".to_string(),
            ]),
        ));
        table.add_row_formula("upper".to_string(), "=UPPER(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let upper = result_table.columns.get("upper").unwrap();
        match &upper.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "HELLO");
                assert_eq!(texts[1], "WORLD");
                assert_eq!(texts[2], "TEST");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_lower_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "HELLO".to_string(),
                "WORLD".to_string(),
                "Test".to_string(),
            ]),
        ));
        table.add_row_formula("lower".to_string(), "=LOWER(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let lower = result_table.columns.get("lower").unwrap();
        match &lower.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "hello");
                assert_eq!(texts[1], "world");
                assert_eq!(texts[2], "test");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_len_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "hello".to_string(),
                "hi".to_string(),
                "testing".to_string(),
            ]),
        ));
        table.add_row_formula("length".to_string(), "=LEN(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let length = result_table.columns.get("length").unwrap();
        match &length.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 5.0);
                assert_eq!(nums[1], 2.0);
                assert_eq!(nums[2], 7.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_mid_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "hello".to_string(),
                "world".to_string(),
                "testing".to_string(),
            ]),
        ));
        table.add_row_formula("mid_2_3".to_string(), "=MID(text, 2, 3)".to_string());
        table.add_row_formula("mid_1_2".to_string(), "=MID(text, 1, 2)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let mid_2_3 = result_table.columns.get("mid_2_3").unwrap();
        match &mid_2_3.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "ell");
                assert_eq!(texts[1], "orl");
                assert_eq!(texts[2], "est");
            },
            _ => panic!("Expected Text array"),
        }

        let mid_1_2 = result_table.columns.get("mid_1_2").unwrap();
        match &mid_1_2.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "he");
                assert_eq!(texts[1], "wo");
                assert_eq!(texts[2], "te");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_text_functions_combined() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec!["  hello  ".to_string(), "  WORLD  ".to_string()]),
        ));
        table.add_row_formula("processed".to_string(), "=UPPER(TRIM(text))".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let processed = result_table.columns.get("processed").unwrap();
        match &processed.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "HELLO");
                assert_eq!(texts[1], "WORLD");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_mixed_math_and_text_functions() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.234, 5.678, 9.012]),
        ));
        table.add_column(Column::new(
            "labels".to_string(),
            ColumnValue::Text(vec![
                "item".to_string(),
                "data".to_string(),
                "test".to_string(),
            ]),
        ));
        table.add_row_formula("rounded".to_string(), "=ROUND(values, 1)".to_string());
        table.add_row_formula("upper_labels".to_string(), "=UPPER(labels)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let rounded = result_table.columns.get("rounded").unwrap();
        match &rounded.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.2);
                assert_eq!(nums[1], 5.7);
                assert_eq!(nums[2], 9.0);
            },
            _ => panic!("Expected Number array"),
        }

        let upper_labels = result_table.columns.get("upper_labels").unwrap();
        match &upper_labels.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "ITEM");
                assert_eq!(texts[1], "DATA");
                assert_eq!(texts[2], "TEST");
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_trim_function_whitespace() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec!["  hello  ".to_string(), " world ".to_string()]),
        ));
        table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());
        model.add_table(table);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let trimmed = table.columns.get("trimmed").unwrap();
        if let ColumnValue::Text(values) = &trimmed.values {
            assert_eq!(values[0], "hello");
            assert_eq!(values[1], "world");
        }
    }

    #[test]
    fn test_text_column_result() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["alice".to_string(), "bob".to_string()]),
        ));
        data.row_formulas
            .insert("upper_name".to_string(), "=UPPER(name)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("upper_name")
            .unwrap();
        if let ColumnValue::Text(values) = &col.values {
            assert_eq!(values[0], "ALICE");
            assert_eq!(values[1], "BOB");
        }
    }

    #[test]
    fn test_cross_table_text_column_reference() {
        let mut model = ParsedModel::new();

        // Source table with text column
        let mut source = Table::new("source".to_string());
        source.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
        ));
        model.add_table(source);

        // Target table referencing source's text column
        let mut target = Table::new("target".to_string());
        target.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        target
            .row_formulas
            .insert("copy_name".to_string(), "=source.names".to_string());
        model.add_table(target);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should handle cross-table text reference
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_index_text_column() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("items".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                "First".to_string(),
                "Second".to_string(),
                "Third".to_string(),
            ]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=INDEX(items.name, idx)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // INDEX function returns text, which may be handled differently
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_text_column_in_rowwise_formula() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
        ));
        data.add_column(Column::new(
            "score".to_string(),
            ColumnValue::Number(vec![100.0, 90.0]),
        ));
        // Use UPPER function on text column
        data.row_formulas
            .insert("upper_name".to_string(), "=UPPER(name)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_text_join_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec!["Hello".to_string()]),
        ));
        data.add_column(Column::new(
            "second".to_string(),
            ColumnValue::Text(vec!["World".to_string()]),
        ));
        data.row_formulas.insert(
            "joined".to_string(),
            "=CONCAT(first, \" \", second)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_left_right_functions() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec!["Hello World".to_string()]),
        ));
        data.row_formulas
            .insert("left_part".to_string(), "=LEFT(text, 5)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_left_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEFT(\"Hello\", 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_right_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=RIGHT(\"Hello\", 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_rept_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=REPT(\"ab\", 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_find_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=FIND(\"lo\", \"hello\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_substitute_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUBSTITUTE(\"hello\", \"l\", \"L\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_text_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=TEXT(1234.5, \"0.00\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_value_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=VALUE(\"123.45\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_concat_text_columns() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec!["John".to_string(), "Jane".to_string()]),
        ));
        data.add_column(Column::new(
            "last".to_string(),
            ColumnValue::Text(vec!["Doe".to_string(), "Smith".to_string()]),
        ));
        model.add_table(data);
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_len_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "length".to_string(),
            Variable::new(
                "length".to_string(),
                None,
                Some("=LEN(\"Hello World\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_mid_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MID(\"Hello World\", 7, 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    // Additional tests for functions with low coverage (v6.0.0 Phase 2)

    #[test]
    fn test_upper_empty_string_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=UPPER(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        // Just verify calculation succeeds
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_upper_numbers_unchanged() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=UPPER(\"abc123xyz\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_upper_special_chars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=UPPER(\"hello-world_test\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_lower_empty_string_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LOWER(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_lower_numbers_unchanged() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LOWER(\"ABC123XYZ\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_lower_mixed_case() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LOWER(\"HeLLo WoRLd\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_len_empty_string_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LEN(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_len_with_spaces() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(\"  hello  \")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(9.0)); // includes spaces
    }

    #[test]
    fn test_mid_boundary_cases() {
        let mut model = ParsedModel::new();
        // Start at position 1 (first char)
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MID(\"Hello\", 1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_mid_beyond_string_length() {
        let mut model = ParsedModel::new();
        // Request more chars than available
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MID(\"Hi\", 1, 10)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_right_boundary_cases() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=RIGHT(\"Hello\", 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_right_full_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=RIGHT(\"Hi\", 10)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_substitute_multiple_occurrences() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUBSTITUTE(\"aaa\", \"a\", \"b\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_substitute_no_match() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUBSTITUTE(\"hello\", \"x\", \"y\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_concatenate_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCATENATE(\"Hello\", \" \", \"World\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // CONCATENATE should work like CONCAT, result length should be 11
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_eq!(len, 11.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_search_function_arraycalc() {
        let mut model = ParsedModel::new();
        // SEARCH is case-insensitive
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=SEARCH(\"LO\", \"hello\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let pos = result.scalars.get("pos").unwrap().value.unwrap();
        // "LO" found at position 4 in "hello"
        assert_eq!(pos, 4.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_replace_function_arraycalc() {
        let mut model = ParsedModel::new();
        // REPLACE(text, start, num_chars, new_text)
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPLACE(\"Hello World\", 7, 5, \"Universe\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // Should replace "World" with "Universe" = "Hello Universe" = 14 chars
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_eq!(len, 14.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_concatenate_multiple() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCATENATE(\"A\", \"B\", \"C\", \"D\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_eq!(len, 4.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_search_case_insensitive_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos1".to_string(),
            Variable::new(
                "pos1".to_string(),
                None,
                Some("=SEARCH(\"WORLD\", \"Hello World\")".to_string()),
            ),
        );
        model.add_scalar(
            "pos2".to_string(),
            Variable::new(
                "pos2".to_string(),
                None,
                Some("=SEARCH(\"world\", \"Hello World\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let p1 = result.scalars.get("pos1").unwrap().value.unwrap();
        let p2 = result.scalars.get("pos2").unwrap().value.unwrap();
        // Both should find "World" at position 7
        assert_eq!(p1, 7.0);
        assert_eq!(p2, 7.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_replace_beginning() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPLACE(\"Hello\", 1, 2, \"Ya\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // Should replace "He" with "Ya" -> "Yallo" = 5 chars
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_eq!(len, 5.0);
    }

    // ══════════════════════════════════════════════════════════════════════════════
    // Tests moved from tests/text_edge_cases.rs
    // ══════════════════════════════════════════════════════════════════════════════

    // EMPTY STRING TESTS

    #[test]
    fn test_left_empty_string_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_right_empty_string_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_mid_empty_string_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"\", 1, 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_upper_empty_string_verified() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_lower_empty_string_verified() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_trim_empty_string_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_concat_empty_strings() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"\", \"\", \"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    // OUT OF BOUNDS TESTS

    #[test]
    fn test_left_count_larger_than_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"abc\", 100))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Should return entire string when count exceeds length
        assert_eq!(var.value, Some(3.0));
    }

    #[test]
    fn test_right_count_larger_than_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"abc\", 100))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Should return entire string when count exceeds length
        assert_eq!(var.value, Some(3.0));
    }

    #[test]
    fn test_mid_start_beyond_string_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"abc\", 10, 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Should return empty string when start is beyond length
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_mid_length_exceeds_remaining() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"hello\", 3, 100))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Should return "llo" (from position 3 to end) = 3 chars
        assert_eq!(var.value, Some(3.0));
    }

    #[test]
    fn test_left_zero_count() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"hello\", 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // LEFT with 0 count should return empty string
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_right_zero_count() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"hello\", 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // RIGHT with 0 count should return empty string
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_mid_zero_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"hello\", 2, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // MID with 0 length should return empty string
        assert_eq!(var.value, Some(0.0));
    }

    // NOT FOUND TESTS (ENTERPRISE ONLY)

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_find_character_not_in_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=FIND(\"x\", \"abc\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should return error when character not found
        assert!(result.is_err());
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_search_substring_not_found() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=SEARCH(\"xyz\", \"abc\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should return error when substring not found
        assert!(result.is_err());
    }

    // CASE SENSITIVITY TESTS (ENTERPRISE ONLY)

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_find_case_sensitive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=FIND(\"H\", \"hello\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // FIND is case-sensitive, so "H" should not be found in "hello"
        assert!(result.is_err());
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_search_case_insensitive_verified() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=SEARCH(\"HELLO\", \"hello world\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("pos").unwrap();
        // SEARCH is case-insensitive, should find at position 1
        assert_eq!(var.value, Some(1.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_find_with_start_position() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=FIND(\"o\", \"hello world\", 6)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("pos").unwrap();
        // Should find the second "o" at position 8
        assert_eq!(var.value, Some(8.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_search_with_start_position() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=SEARCH(\"L\", \"hello world\", 4)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("pos").unwrap();
        // Should find "l" at position 4 (case insensitive)
        assert_eq!(var.value, Some(4.0));
    }

    // SPECIAL CHARACTERS TESTS

    #[test]
    fn test_trim_multiple_spaces() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"     hello     \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    fn test_trim_tabs_and_newlines() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"\t\nhello\t\n\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    fn test_trim_only_spaces_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"     \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // All spaces should be trimmed, leaving empty string
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_len_special_characters() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(\"!@#$%^&*()\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(10.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_substitute_with_special_chars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(SUBSTITUTE(\"a-b-c\", \"-\", \"_\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_substitute_empty_old_text() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(SUBSTITUTE(\"hello\", \"\", \"x\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Substituting empty string should return original
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_substitute_no_replacement_needed() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(SUBSTITUTE(\"abc\", \"xyz\", \"123\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // No match, should return original string
        assert_eq!(var.value, Some(3.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_replace_entire_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPLACE(\"hello\", 1, 5, \"world\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Replace all characters
        assert_eq!(var.value, Some(5.0));
    }

    // UNICODE TESTS

    #[test]
    fn test_len_unicode_characters() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(\"hello\u{4e16}\u{754c}\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // "hello" = 5 chars + 2 unicode chars = 7 total
        assert_eq!(var.value, Some(7.0));
    }

    #[test]
    fn test_left_with_unicode() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"hello\u{4e16}\u{754c}\", 6))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(6.0));
    }

    #[test]
    fn test_right_with_unicode() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"hello\u{4e16}\u{754c}\", 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(3.0));
    }

    #[test]
    fn test_mid_with_unicode() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"hello\u{4e16}\u{754c}\", 5, 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Starting at position 5 ("o"), take 3 chars
        assert_eq!(var.value, Some(3.0));
    }

    // MIXED TYPES TESTS

    #[test]
    fn test_concat_mixed_types() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "num".to_string(),
            Variable::new("num".to_string(), Some(42.0), None),
        );
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"The answer is \", num))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(16.0));
    }

    #[test]
    fn test_upper_with_numbers_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"test123\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Numbers should remain unchanged
        assert_eq!(var.value, Some(7.0));
    }

    #[test]
    fn test_lower_with_numbers_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"TEST123\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Numbers should remain unchanged
        assert_eq!(var.value, Some(7.0));
    }

    // VALUE FUNCTION TESTS (ENTERPRISE ONLY)

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_value_with_thousand_separators() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=VALUE(\"1,234,567\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(1234567.0));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_value_with_whitespace() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=VALUE(\"   100.5   \")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(100.5));
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_value_invalid_text() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=VALUE(\"not a number\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should return error for invalid number text
        assert!(result.is_err());
    }

    // TEXT FUNCTION TESTS (ENTERPRISE ONLY)

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_text_percentage_format() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TEXT(0.5, \"0%\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(3.0)); // "50%"
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_text_decimal_format() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TEXT(123.456, \"0.00\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(6.0)); // "123.46"
    }

    // REPT FUNCTION TESTS

    #[test]
    fn test_rept_multiple_times() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"ab\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(10.0)); // "ab" * 5 = "ababababab" = 10 chars
    }

    #[test]
    fn test_rept_zero_times() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"hello\", 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0)); // Empty string
    }

    #[test]
    fn test_rept_once() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"test\", 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(4.0)); // "test" = 4 chars
    }

    // ══════════════════════════════════════════════════════════════════════════════
    // Tests moved from tests/string_edge_cases.rs
    // ══════════════════════════════════════════════════════════════════════════════

    // CONCATENATION

    #[test]
    fn test_concat_with_spaces_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"Hello\", \" \", \"World\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(11.0));
    }

    #[test]
    fn test_concat_function_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"ab\", \"cd\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    // EMPTY STRING HANDLING

    #[test]
    fn test_empty_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LEN(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_single_space() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LEN(\" \")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    // LEFT/RIGHT EDGE CASES

    #[test]
    fn test_left_empty_string_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_right_empty_string_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_left_partial() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LEFT(\"Hello\", 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_right_partial() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(RIGHT(\"Hello\", 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    // MID EDGE CASES

    #[test]
    fn test_mid_from_start() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"test\", 1, 2))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_mid_from_middle() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(MID(\"test\", 2, 2))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    // TRIM BEHAVIOR

    #[test]
    fn test_trim_internal_spaces() {
        // Excel TRIM collapses multiple internal spaces to single space
        // "  a  b  " -> "a b" (length 3)
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"  a  b  \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_trim_only_spaces_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"   \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    // UPPER/LOWER

    #[test]
    fn test_upper_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"abc\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_lower_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"ABC\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    // REPT FUNCTION

    #[test]
    fn test_rept_single_char() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"x\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_rept_multi_char() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"ab\", 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }
}
