//! Replace functions: REPLACE, SUBSTITUTE (enterprise only)

use super::super::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// REPLACE(old_text, start_num, num_chars, new_text) - Replaces characters within text
pub fn eval_replace(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("REPLACE", args, 4)?;
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

    Ok(Value::Text(format!("{prefix}{new_text}{suffix}")))
}

/// SUBSTITUTE(text, old_text, new_text, [instance_num]) - Substitutes text occurrences
pub fn eval_substitute(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SUBSTITUTE", args, 3, 4)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let old_text = evaluate(&args[1], ctx)?.as_text();
    let new_text = evaluate(&args[2], ctx)?.as_text();

    // If old_text is empty, return original text unchanged (Excel behavior)
    if old_text.is_empty() {
        return Ok(Value::Text(text));
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
        Ok(Value::Text(result))
    } else {
        // Replace all occurrences
        Ok(Value::Text(text.replace(&old_text, &new_text)))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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

    #[test]
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
}
