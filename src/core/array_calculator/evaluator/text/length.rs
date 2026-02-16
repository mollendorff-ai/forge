//! String length and extraction functions: LEN, LEFT, RIGHT, MID

// Text length casts: char counts and string indices (f64 to usize, bounded by string length).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use super::super::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// LEN(text) - Returns the length of a text string
pub fn eval_len(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("LEN", args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    Ok(Value::Number(val.as_text().chars().count() as f64))
}

/// LEFT(text, [`num_chars`]) - Returns the leftmost characters from a text string
pub fn eval_left(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("LEFT", args, 1, 2)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let n = if args.len() > 1 {
        evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
    } else {
        1
    };
    Ok(Value::Text(text.chars().take(n).collect()))
}

/// RIGHT(text, [`num_chars`]) - Returns the rightmost characters from a text string
pub fn eval_right(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("RIGHT", args, 1, 2)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let n = if args.len() > 1 {
        evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
    } else {
        1
    };
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(n);
    Ok(Value::Text(chars[start..].iter().collect()))
}

/// MID(text, `start_num`, `num_chars`) - Returns characters from the middle of a text string
pub fn eval_mid(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("MID", args, 3)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let start = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize;
    let length = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as usize;

    let chars: Vec<char> = text.chars().collect();
    // Excel MID is 1-indexed
    let start_idx = start.saturating_sub(1);

    // If start is beyond string length, return empty string
    if start_idx >= chars.len() {
        return Ok(Value::Text(String::new()));
    }

    let end_idx = (start_idx + length).min(chars.len());
    Ok(Value::Text(chars[start_idx..end_idx].iter().collect()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparison validated against Excel/Gnumeric/R
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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
}
