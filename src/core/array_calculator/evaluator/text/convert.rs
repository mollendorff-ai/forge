//! Convert functions: TEXT, VALUE

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// TEXT(value, format) - a value written with an Excel format code
/// (`number_format` says which codes are read).
pub fn eval_text(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("TEXT", args, 2)?;
    let value = match evaluate(&args[0], ctx)? {
        Value::Array(items) => match <[Value; 1]>::try_from(std::sync::Arc::unwrap_or_clone(items))
        {
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
    // Digits, a point, digits and a whole exponent: always a number's text
    // (were it not, NaN, which is refused below)
    // To 15 significant digits, as Excel's VALUE reads it
    let mut n: f64 = crate::core::precision::read_typed(&format!("{whole}.{fraction}e{exponent}"))
        .unwrap_or(f64::NAN);
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
        let model = calculator.calculate_all().unwrap();
        assert_exact!(model.scalars["result"].value.unwrap(), 1234.5);
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
        let model = calculator.calculate_all().unwrap();
        assert_exact!(model.scalars["result"].value.unwrap(), 123.45);
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

    /// A date is a date (ADR-054): the text `2026-01-31` in a model, and its day
    /// number wherever a formula reads a number. `+` adds and `&` joins, as in
    /// Excel. Each result here is what Gnumeric and `LibreOffice` give for the
    /// exported formula over a real date (forge-e2e-gnumeric,
    /// `edge_real_dates.yaml`).
    mod dates_as_dates {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::EvalContext;

        /// `TEXT(d, "yyyy")` was the day number as text ("46053").
        #[test]
        fn text_formats_a_date() {
            assert_eq!(h::text("TEXT(\"2026-01-31\", \"yyyy\")"), "2026");
            assert_eq!(
                h::text("TEXT(DATE(2026, 1, 31), \"mmm d, yyyy\")"),
                "Jan 31, 2026"
            );
            assert_eq!(h::text("TEXT(1234.5, \"#,##0.00\")"), "1,234.50");
            let error = h::eval("TEXT(1, \"h:mm\")", &EvalContext::new())
                .unwrap_err()
                .to_string();
            assert!(error.contains("is not one Forge reads"), "{error}");
        }
    }

    /// The five answers Excel gave differently from Forge when 40 formulas were
    /// checked in Excel on 2026-11-14
    /// (`validation/gnumeric/tests/edge/edge_excel_checks.yaml`), and what
    /// follows from each rule.
    mod checked_in_excel {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn text_writes_no_minus_on_a_number_that_rounds_to_zero() {
            assert_eq!(h::text("TEXT(-0.004, \"0.00\")"), "0.00"); // Excel
            assert_eq!(h::text("TEXT(-0.006, \"0.00\")"), "-0.01");
            assert_eq!(h::text("TEXT(-0.4, \"0\")"), "0");
        }
    }

    /// Wrong answers found by sweeping the statistical, lookup, math and text
    /// functions against Gnumeric and `LibreOffice` (`validation/sweep`), each pinned
    /// to what both engines give, or to Excel's documentation where they differ.
    mod sweep {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn value_reads_what_excel_reads() {
            h::sweep_is_exactly("VALUE(\"50%\")", 0.5);
            h::sweep_is_exactly("VALUE(\"$1,000\")", 1000.0);
            h::sweep_is_exactly("VALUE(\"(5)\")", -5.0);
            h::sweep_is_exactly("VALUE(\"5-\")", -5.0);
            h::sweep_is_exactly("VALUE(\"$-5\")", -5.0);
            h::sweep_is_exactly("VALUE(\"2024-01-15\")", 45306.0);
            assert!((h::sweep_number("VALUE(\"12:30\")") - 12.5 / 24.0).abs() < 1e-15);
            for bad in ["inf", "nan", "1,2,3", "", "TRUE", "1.2.3"] {
                h::sweep_fails(&format!("VALUE(\"{bad}\")"), "#VALUE!");
            }
        }

        /// Excel reads a number typed in a formula, or turned from text, to
        /// its first 15 significant digits and drops the rest (Excel for
        /// iPhone, 2026-11-24, T1-EXCEL-LITERAL-DIGITS): rounding would give
        /// 10 for the first and the third
        #[test]
        fn a_number_from_text_keeps_fifteen_digits_as_excel() {
            for (formula, excel) in [
                ("1234567890123456 - 1234567890123450", 0.0),
                ("1234567890123454 - 1234567890123450", 0.0),
                ("(0.1234567890123456 - 0.123456789012345) * 1E16", 0.0),
                ("(1.2345678901234567E20 - 1.23456789012345E20) / 1E5", 0.0),
                ("1E15 + 1 - 1000000000000000.5", 1.0),
                ("VALUE(\"1234567890123456\") - 1234567890123450", 0.0),
                ("VALUE(\"1,234,567,890,123,456\") - 1234567890123450", 0.0),
                ("(\"1234567890123456\" + 0) - 1234567890123450", 0.0),
            ] {
                h::sweep_is_exactly(formula, excel);
            }
        }
    }

    /// An error in an argument is passed on
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        each_argument("TEXT", &[("1", None), ("\"0\"", None)]);
        each_argument("VALUE", &[("\"5\"", None)]);
    }

    /// An array of one value is that value, and an empty cell is 0, to TEXT
    /// and to VALUE; a number is itself to VALUE
    #[test]
    fn one_value_an_empty_cell_and_a_number() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(h::text("TEXT({5}, \"0\")"), "5");
        let ctx = h::empty_cells();
        assert_eq!(h::text_in("TEXT(blank, \"0\")", &ctx), "0");
        h::is_exactly_in("VALUE(blank)", &ctx, 0.0);
        h::is_exactly("VALUE(5)", 5.0);
    }

    /// Signs, points, exponents and times VALUE reads, and the ones it
    /// refuses as #VALUE!
    #[test]
    fn value_reads_and_refuses() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("VALUE(\"+5\")", 5.0);
        h::is_exactly("VALUE(\".5\")", 0.5);
        h::is_exactly("VALUE(\"1e5\")", 100_000.0);
        assert!(h::near(
            h::number("VALUE(\"12:30:15.5\")"),
            45_015.5 / 86_400.0
        ));
        for text in [
            "--5",
            "(-5)",
            ".",
            "1e",
            "1ex",
            "1e99999999999",
            ".:30",
            "1:.",
            "1:30:.",
            "1:60",
        ] {
            h::fails_with(
                &format!("VALUE(\"{text}\")"),
                &format!("#VALUE!: VALUE: '{text}' is not a number"),
            );
        }
    }
}
