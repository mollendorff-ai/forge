//! Search functions: FIND, SEARCH (enterprise only)

// Text search casts: char position indices between f64 and usize (bounded by string length).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use super::super::{evaluate, require_args_range, EvalContext, EvalError, Expr, Value};

/// `FIND(find_text`, `within_text`, [`start_num`]) - Finds text within text (case-sensitive)
pub fn eval_find(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("FIND", args, 2, 3)?;
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

    within_text[start_idx..].find(&find_text).map_or_else(
        || Err(EvalError::new("FIND: text not found")),
        |pos| Ok(Value::Number((pos + start_num) as f64)),
    )
}

/// `SEARCH(find_text`, `within_text`, [`start_num`]) - Finds text within text (case-insensitive)
pub fn eval_search(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SEARCH", args, 2, 3)?;
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
    search_in.find(&find_text).map_or_else(
        || Err(EvalError::new("SEARCH: text not found")),
        |pos| Ok(Value::Number((pos + start_num) as f64)),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparison validated against Excel/Gnumeric/R
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

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

    // NOT FOUND TESTS

    #[test]
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

    // CASE SENSITIVITY TESTS

    #[test]
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
}
