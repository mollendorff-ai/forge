//! Convert functions: TEXT, VALUE (enterprise only)

#[cfg(not(feature = "demo"))]
use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// TEXT(value, format_text) - Converts a value to text with specified format
#[cfg(not(feature = "demo"))]
pub fn eval_text(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("TEXT", args, 2)?;
    let val = evaluate(&args[0], ctx)?;
    let format = evaluate(&args[1], ctx)?.as_text();
    // Simplified TEXT implementation - basic number formatting
    let num = val.as_number().unwrap_or(0.0);
    let formatted = format_number(num, &format);
    Ok(Value::Text(formatted))
}

/// VALUE(text) - Converts text to a number
#[cfg(not(feature = "demo"))]
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
#[cfg(not(feature = "demo"))]
mod tests {
    use super::format_number;
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_format_number_helper() {
        // Test the helper function directly
        assert_eq!(format_number(0.25, "0%"), "25%");
        assert_eq!(format_number(1234.0, "$0.00"), "$1234.00");
        assert_eq!(format_number(1.2345, "0.000"), "1.234");
        assert_eq!(format_number(1000000.0, "#,##0"), "1,000,000");
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
        assert_eq!(var.value, Some(1234567.0));
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
