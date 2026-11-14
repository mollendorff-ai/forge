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

/// VALUE(text) - Converts text to a number, reading what Excel reads in an
/// English locale: thousands commas, a leading `$`, a trailing `%`, a negative
/// in parentheses, an exponent, a date `YYYY-MM-DD` and a time `h:mm[:ss]`.
/// Anything else (`inf`, `nan`, `TRUE`, two decimal points) is #VALUE!.
pub fn eval_value(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("VALUE", args, 1)?;
    let value = evaluate(&args[0], ctx)?;
    if let Some(n) = match &value {
        Value::Number(n) => Some(*n),
        Value::Null => Some(0.0),
        _ => None,
    } {
        return Ok(Value::Number(n));
    }
    let text = value.as_text();
    read_number(&text)
        .map(Value::Number)
        .ok_or_else(|| EvalError::new(format!("#VALUE!: VALUE: '{text}' is not a number")))
}

/// The number written in `text`, as VALUE reads it
fn read_number(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(serial) = super::super::date_serial(t) {
        return Some(serial);
    }
    if let Some(time) = read_time(t) {
        return Some(time);
    }
    let (negative, t) = t
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        .map_or((false, t), |inner| (true, inner.trim()));
    let (percent, t) = t
        .strip_suffix('%')
        .map_or((false, t), |rest| (true, rest.trim_end()));
    // A sign before or after a `$`, or a minus at the end (`5-`)
    let (trailing_minus, t) = t.strip_suffix('-').map_or((false, t), |rest| (true, rest));
    let (sign, t) = leading_sign(t);
    let (dollar, t) = t.strip_prefix('$').map_or((false, t), |rest| (true, rest));
    let (sign, t) = if dollar && sign > 0.0 && !t.starts_with('+') {
        leading_sign(t)
    } else {
        (sign, t)
    };
    let sign = if trailing_minus { -sign } else { sign };
    if (negative || trailing_minus) && t.starts_with(['-', '+'])
        || negative && sign < 0.0
        || t.starts_with(['-', '+'])
    {
        return None;
    }
    // Digits, thousands commas before the decimal point, one point, an exponent
    let (mantissa, exponent) = t
        .find(['e', 'E'])
        .map_or((t, None), |i| (&t[..i], Some(&t[i + 1..])));
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty()
        || !thousands_ok(whole)
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let exponent: i32 = match exponent {
        Some(e) => {
            let digits = e.strip_prefix(['+', '-']).unwrap_or(e);
            if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            e.parse().ok()?
        },
        None => 0,
    };
    let whole = whole.replace(',', "");
    let whole = if whole.is_empty() { "0" } else { &whole };
    let fraction = if fraction.is_empty() { "0" } else { fraction };
    let mut n: f64 = format!("{whole}.{fraction}e{exponent}").parse().ok()?;
    if percent {
        n /= 100.0;
    }
    if negative {
        n = -n;
    }
    let n = sign * n;
    n.is_finite().then_some(n)
}

/// A `-` or `+` at the start, as a sign
fn leading_sign(t: &str) -> (f64, &str) {
    match t.as_bytes().first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    }
}

/// Digits, with commas only between groups of three (`1,234,567`)
fn thousands_ok(whole: &str) -> bool {
    let groups: Vec<&str> = whole.split(',').collect();
    groups.iter().all(|g| g.chars().all(|c| c.is_ascii_digit()))
        && (groups.len() == 1
            || (1..=3).contains(&groups[0].len()) && groups[1..].iter().all(|g| g.len() == 3))
}

/// A time `h:mm` or `h:mm:ss` as a fraction of a day
fn read_time(t: &str) -> Option<f64> {
    let parts: Vec<&str> = t.split(':').collect();
    if !(2..=3).contains(&parts.len())
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit() || c == '.'))
    {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts.get(2).map_or(Some(0.0), |p| p.parse().ok())?;
    if m >= 60.0 || s >= 60.0 || parts[0].contains('.') || parts[1].contains('.') {
        return None;
    }
    Some(h.mul_add(3600.0, m.mul_add(60.0, s)) / 86_400.0)
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
