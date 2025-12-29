//! Text functions: CONCAT, UPPER, LOWER, TRIM, LEN, LEFT, RIGHT, MID, REPT, TEXT, VALUE, FIND, SEARCH, REPLACE, SUBSTITUTE
//!
//! DEMO functions (9): CONCAT, LEFT, RIGHT, MID, REPT, LEN, UPPER, LOWER, TRIM
//! ENTERPRISE functions: CONCATENATE, TEXT, VALUE, FIND, SEARCH, REPLACE, SUBSTITUTE

mod case;
mod concat;
mod convert;
mod length;
mod replace;
mod search;

use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate a text function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        // ═══════════════════════════════════════════════════════════════════════════
        // DEMO FUNCTIONS (always available)
        // ═══════════════════════════════════════════════════════════════════════════
        "UPPER" => case::eval_upper(args, ctx)?,
        "LOWER" => case::eval_lower(args, ctx)?,
        "TRIM" => case::eval_trim(args, ctx)?,

        "LEN" => length::eval_len(args, ctx)?,
        "LEFT" => length::eval_left(args, ctx)?,
        "RIGHT" => length::eval_right(args, ctx)?,
        "MID" => length::eval_mid(args, ctx)?,

        "CONCAT" => concat::eval_concat(args, ctx)?,
        "REPT" => concat::eval_rept(args, ctx)?,

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "CONCATENATE" => concat::eval_concatenate(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "TEXT" => convert::eval_text(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "VALUE" => convert::eval_value(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "FIND" => search::eval_find(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "SEARCH" => search::eval_search(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "REPLACE" => replace::eval_replace(args, ctx)?,

        #[cfg(not(feature = "demo"))]
        "SUBSTITUTE" => replace::eval_substitute(args, ctx)?,

        _ => return Ok(None),
    };

    Ok(Some(result))
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
}
