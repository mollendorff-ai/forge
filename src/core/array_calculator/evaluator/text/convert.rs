//! Convert functions: TEXT, VALUE

// Text convert casts: f64 to integer types for formatting (bounded by format specifiers).
#![allow(clippy::cast_possible_truncation)]

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// TEXT(value, format) - a value written with an Excel format code
/// (`number_format` says which codes are read).
pub fn eval_text(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("TEXT", args, 2)?;
    let value = match evaluate(&args[0], ctx)? {
        Value::Array(items) => match <[Value; 1]>::try_from(items) {
            Ok([only]) => only,
            Err(items) => {
                return Err(EvalError::new(format!(
                    "Expected a single value, got an array of {}",
                    items.len()
                )))
            },
        },
        // An empty value is formatted as 0, as an empty cell is in Excel
        Value::Null => Value::Number(0.0),
        other => other,
    };
    let format = evaluate(&args[1], ctx)?.as_text();
    Ok(Value::Text(super::number_format::format_value(
        &value, &format,
    )?))
}

/// VALUE(text) - Converts text to a number
pub fn eval_value(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("VALUE", args, 1)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    // Parse the text as a number
    let num = text
        .trim()
        .replace(',', "") // Remove thousand separators
        .parse::<f64>()
        .map_err(|_| EvalError::new(format!("VALUE: Cannot convert '{text}' to number")))?;
    Ok(Value::Number(num))
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

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

    // VALUE FUNCTION TESTS

    #[test]
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
        assert_eq!(var.value, Some(1_234_567.0));
    }

    #[test]
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

    // TEXT FUNCTION TESTS

    #[test]
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
}
