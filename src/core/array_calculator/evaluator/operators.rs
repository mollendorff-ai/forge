//! The operators of a formula: arithmetic, comparison and `&`, element by
//! element over arrays, and unary minus. `+` adds as Excel does, a text that
//! reads as a number or a date included; `&` joins.

use super::{date_serial, EvalError, Value};
use crate::core::precision::{excel_equal, excel_order};
use std::cmp::Ordering;

/// Evaluate a binary operation, element by element when an operand is an array.
///
/// Arrays of equal length pair up by position. A scalar or one-element array
/// applies to every element. Any other length mismatch is an error.
pub(super) fn evaluate_binary_op(
    op: &str,
    left: &Value,
    right: &Value,
) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::Array(l), Value::Array(r)) if l.len() == r.len() => l
            .iter()
            .zip(r.iter())
            .map(|(a, b)| evaluate_binary_op(op, a, b))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| Value::Array(items.into())),
        (Value::Array(l), Value::Array(r)) if r.len() == 1 => broadcast_left(op, l, &r[0]),
        (Value::Array(l), Value::Array(r)) if l.len() == 1 => broadcast_right(op, &l[0], r),
        (Value::Array(l), Value::Array(r)) => Err(EvalError::new(format!(
            "Array length mismatch for '{op}': {} and {}",
            l.len(),
            r.len()
        ))),
        (Value::Array(l), scalar) => broadcast_left(op, l, scalar),
        (scalar, Value::Array(r)) => broadcast_right(op, scalar, r),
        _ => evaluate_scalar_binary_op(op, left, right),
    }
}

/// Apply `op` between each element of `items` and a single right operand.
fn broadcast_left(op: &str, items: &[Value], right: &Value) -> Result<Value, EvalError> {
    items
        .iter()
        .map(|item| evaluate_binary_op(op, item, right))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| Value::Array(items.into()))
}

/// Apply `op` between a single left operand and each element of `items`.
fn broadcast_right(op: &str, left: &Value, items: &[Value]) -> Result<Value, EvalError> {
    items
        .iter()
        .map(|item| evaluate_binary_op(op, left, item))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| Value::Array(items.into()))
}

/// Evaluate a binary operation on two non-array values
// Arithmetic + comparison + logical operators require many match arms.
#[allow(clippy::too_many_lines)]
/// `base ^ exponent` as Excel computes it, for `^`, `POWER` and `POW`: 0^0
/// and a negative base with a fractional exponent are #NUM!, and 0 to a
/// negative power is #DIV/0!.
pub fn power(base: f64, exponent: f64) -> Result<Value, EvalError> {
    if base == 0.0 && exponent == 0.0 {
        return Err(EvalError::new("#NUM!: 0^0 is undefined"));
    }
    if base == 0.0 && exponent < 0.0 {
        return Err(EvalError::new("#DIV/0!: zero to a negative power"));
    }
    let result = base.powf(exponent);
    if result.is_nan() {
        return Err(EvalError::new(
            "#NUM!: negative base with a fractional exponent",
        ));
    }
    finite(result)
}

/// Excel returns #NUM! when arithmetic overflows; an f64 would quietly become inf.
fn finite(result: f64) -> Result<Value, EvalError> {
    if result.is_infinite() {
        Err(EvalError::new("#NUM!: result is too large"))
    } else {
        Ok(Value::Number(result))
    }
}

/// Both operands as numbers, or an error naming the side that is not one.
fn numbers(left: &Value, right: &Value) -> Result<(f64, f64), EvalError> {
    let l = left
        .as_number()
        .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
    let r = right
        .as_number()
        .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
    Ok((l, r))
}

fn evaluate_scalar_binary_op(op: &str, left: &Value, right: &Value) -> Result<Value, EvalError> {
    // An empty cell is 0 in arithmetic and comparison, and equal to both 0 and
    // "" (and to FALSE), as in Excel (ADR-059); `&` reads it as ""
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        let empty_equal = |other: &Value| match other {
            Value::Null => true,
            Value::Text(t) => t.is_empty(),
            Value::Boolean(b) => !b,
            other => other.as_number() == Some(0.0),
        };
        let other = if matches!(left, Value::Null) {
            right
        } else {
            left
        };
        match op {
            "=" => return Ok(Value::Boolean(empty_equal(other))),
            "<>" => return Ok(Value::Boolean(!empty_equal(other))),
            "&" => {},
            _ => {
                let zero = Value::Number(0.0);
                let l = if matches!(left, Value::Null) {
                    &zero
                } else {
                    left
                };
                let r = if matches!(right, Value::Null) {
                    &zero
                } else {
                    right
                };
                return evaluate_scalar_binary_op(op, l, r);
            },
        }
    }
    match op {
        // Arithmetic operators
        // `+` adds, as in Excel: a date is its day number and "5" is 5.
        // Text that is neither is #VALUE!; `&` joins text (ADR-054).
        "+" => {
            let (Some(l), Some(r)) = (left.as_number(), right.as_number()) else {
                return Err(EvalError::new(
                    "#VALUE!: + adds numbers; use & or CONCAT to join text",
                ));
            };
            finite(l + r)
        },
        "-" => {
            let (l, r) = numbers(left, right)?;
            finite(l - r)
        },
        "*" => {
            let (l, r) = numbers(left, right)?;
            finite(l * r)
        },
        "/" => {
            let (l, r) = numbers(left, right)?;
            if r == 0.0 {
                Err(EvalError::new("Division by zero"))
            } else {
                finite(l / r)
            }
        },
        "^" => {
            let (l, r) = numbers(left, right)?;
            power(l, r)
        },

        // String concatenation operator
        "&" => Ok(Value::Text(format!(
            "{}{}",
            left.as_text(),
            right.as_text()
        ))),

        // Comparison operators
        "=" => Ok(Value::Boolean(values_equal(left, right))),
        "<>" => Ok(Value::Boolean(!values_equal(left, right))),
        // To 15 significant digits, as `=` (ADR-065)
        "<" | ">" | "<=" | ">=" => {
            let (l, r) = numbers(left, right)?;
            let order = excel_order(l, r);
            Ok(Value::Boolean(match op {
                "<" => order == Some(Ordering::Less),
                ">" => order == Some(Ordering::Greater),
                "<=" => matches!(order, Some(Ordering::Less | Ordering::Equal)),
                _ => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
            }))
        },

        _ => Err(EvalError::new(format!("Unknown operator: {op}"))),
    }
}

/// Check if two values are equal
pub fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        // Rounded to 15 significant digits, as Excel's `=` (ADR-065)
        (Value::Number(l), Value::Number(r)) => excel_equal(*l, *r),
        (Value::Text(l), Value::Text(r)) => l.to_lowercase() == r.to_lowercase(),
        (Value::Boolean(l), Value::Boolean(r)) => l == r,
        (Value::Null, Value::Null) => true,
        (Value::Array(l), Value::Array(r)) => {
            // Arrays are equal if same length and all elements equal
            if l.len() != r.len() {
                return false;
            }
            l.iter().zip(r.iter()).all(|(a, b)| values_equal(a, b))
        },
        // Single-element array compared with scalar
        (Value::Array(arr), other) if arr.len() == 1 => values_equal(&arr[0], other),
        (other, Value::Array(arr)) if arr.len() == 1 => values_equal(other, &arr[0]),
        // A date is equal to its day number: `d = DATE(2026, 1, 31)` (ADR-054).
        // Other text is never equal to a number, as in Excel (`"5" = 5` is FALSE).
        (Value::Text(text), Value::Number(n)) | (Value::Number(n), Value::Text(text)) => {
            date_serial(text).is_some_and(|serial| excel_equal(serial, *n))
        },
        // Boolean vs Number is never equal: in Excel and Gnumeric `TRUE = 1`
        // and `FALSE = 0` are FALSE (arithmetic still coerces: TRUE + 1 = 2).
        _ => false,
    }
}

/// Evaluate a unary operation
pub(super) fn evaluate_unary_op(op: &str, operand: &Value) -> Result<Value, EvalError> {
    if let Value::Array(items) = operand {
        return items
            .iter()
            .map(|item| evaluate_unary_op(op, item))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| Value::Array(items.into()));
    }
    match op {
        // -(an empty cell) is 0 (ADR-059)
        "-" if matches!(operand, Value::Null) => Ok(Value::Number(0.0)),
        "-" => {
            let n = operand
                .as_number()
                .ok_or_else(|| EvalError::new("Operand must be a number"))?;
            // 0 - n, so that -(0) is 0 and not -0, as Excel shows it
            Ok(Value::Number(0.0 - n))
        },
        _ => Err(EvalError::new(format!("Unknown unary operator: {op}"))),
    }
}

#[cfg(test)]
mod tests {

    /// Excel's comparison operators compare two numbers rounded to 15
    /// significant digits (ADR-065), all six alike: Microsoft's floating-point
    /// article and Q&A. `LibreOffice` 26.2 gives each of these; Gnumeric 1.12
    /// compares exactly.
    mod fifteen_digits {
        use crate::core::array_calculator::evaluator::tests as h;

        fn holds(formula: &str) -> bool {
            h::truth(formula)
        }

        #[test]
        fn binary_noise_is_equal_and_neither_above_nor_below() {
            for (a, b) in [
                ("0.1 + 0.2", "0.3"),
                ("100 * 1.1", "110"),
                ("0.3 - 0.1", "0.2"),
            ] {
                assert!(holds(&format!("{a} = {b}")), "{a} = {b}");
                assert!(!holds(&format!("{a} <> {b}")), "{a} <> {b}");
                assert!(holds(&format!("{a} <= {b}")), "{a} <= {b}");
                assert!(holds(&format!("{a} >= {b}")), "{a} >= {b}");
                assert!(!holds(&format!("{a} < {b}")), "{a} < {b}");
                assert!(!holds(&format!("{a} > {b}")), "{a} > {b}");
            }
        }

        /// The sixteenth digit does not count, the fifteenth does
        #[test]
        fn the_fifteenth_digit_counts() {
            assert!(holds("1 + 1E-15 = 1"));
            assert!(!holds("1 + 1E-14 = 1"));
            assert!(holds("1 + 1E-14 > 1"));
            assert!(!holds("1 + 1E-13 = 1"));
            assert!(!holds("1 + 5E-11 = 1"));
            assert!(holds("1E15 + 0.2 = 1E15"));
            assert!(holds("1E10 + 1E-6 = 1E10"));
            assert!(holds("123456789.12345678 = 123456789.12345679"));
            // 1000000000000001 is 1.00000000000000E+15 to 15 digits. LibreOffice
            // and Gnumeric find it unequal; the rule as Microsoft describes it
            // finds it equal (not checked in Excel: ADR-065)
            assert!(holds("1E15 + 1 = 1E15"));
            assert!(!holds("1E15 + 10 = 1E15"));
        }

        /// Fifteen digits of each number, however small: no tolerance near 0
        #[test]
        fn a_tiny_number_is_not_zero() {
            assert!(!holds("-1E-12 = 0"));
            assert!(holds("-1E-12 < 0"));
            assert!(!holds("1E-12 = 2E-12"));
            assert!(!holds("1E-300 = 0"));
            assert!(holds("1E-300 > 0"));
            assert!(holds("0 = -0"));
        }

        /// A date and its day number, by the same rule
        #[test]
        fn a_date_and_its_day_number() {
            assert!(holds("\"2026-01-31\" = 46053"));
            assert!(!holds("\"2026-01-31\" = 46053.0001"));
        }

        /// SWITCH matches as = does (`LibreOffice`; not checked in Excel)
        #[test]
        fn switch_matches_as_equals_does() {
            h::is_exactly("SWITCH(0.1 + 0.2, 0.3, 1, 0)", 1.0);
            h::is_exactly("SWITCH(-1E-12, 0, 1, 2)", 2.0);
        }
    }
    use super::*;
    use crate::core::array_calculator::evaluator::tests::{array_ctx, eval};
    use crate::core::array_calculator::evaluator::EvalContext;

    /// `POWER` and `POW` gave 1 for 0^0, where Excel and Forge's own `^` give
    /// #NUM!; all three now follow Excel.
    #[test]
    fn power_functions_match_the_operator() {
        let ctx = EvalContext::new();
        for (formula, error) in [
            ("0^0", "#NUM!"),
            ("POWER(0, 0)", "#NUM!"),
            ("POW(0, 0)", "#NUM!"),
            ("POWER(0, -1)", "#DIV/0!"),
            ("POW(-8, 1/3)", "#NUM!"),
        ] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains(error), "{formula}: {err}");
        }
        assert_eq!(eval("POWER(2, 10)", &ctx).unwrap(), Value::Number(1024.0));
        assert_eq!(eval("POW(-2, 3)", &ctx).unwrap(), Value::Number(-8.0));
    }

    #[test]
    fn test_eval_arithmetic() {
        let ctx = EvalContext::new();
        assert_eq!(eval("2 + 3", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("10 - 4", &ctx).unwrap(), Value::Number(6.0));
        assert_eq!(eval("3 * 4", &ctx).unwrap(), Value::Number(12.0));
        assert_eq!(eval("15 / 3", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("2 ^ 3", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_eval_power_zero_to_zero() {
        let ctx = EvalContext::new();
        // Excel: 0^0 is #NUM!
        assert!(eval("0 ^ 0", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#NUM!"));
        // Excel: 0 to a negative power is #DIV/0!
        assert!(eval("0 ^ -1", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#DIV/0!"));
        // Excel: negative base, fractional exponent is #NUM!
        assert!(eval("(0 - 8) ^ 0.5", &ctx).is_err());
    }

    #[test]
    fn test_eval_overflow_is_num_error() {
        // Excel: arithmetic that overflows is #NUM!, never infinity
        let ctx = EvalContext::new();
        for formula in [
            "2 ^ 1024",
            "1E308 * 10",
            "1E308 + 1E308",
            "-1E308 - 1E308",
            "1E308 / 1E-10",
        ] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains("#NUM!"), "{formula}: {err}");
        }
        for formula in ["POWER(2, 1024)", "EXP(1000)", "SUM(1E308, 1E308)"] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains("#NUM!"), "{formula}: {err}");
        }
        assert_eq!(
            eval("2 ^ 1023", &ctx)
                .unwrap()
                .as_number()
                .map(f64::is_finite),
            Some(true)
        );
    }

    #[test]
    fn test_eval_precedence() {
        let ctx = EvalContext::new();
        assert_eq!(eval("2 + 3 * 4", &ctx).unwrap(), Value::Number(14.0));
        assert_eq!(eval("(2 + 3) * 4", &ctx).unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_eval_unary_minus() {
        let ctx = EvalContext::new();
        assert_eq!(eval("-5", &ctx).unwrap(), Value::Number(-5.0));
        assert_eq!(eval("10 + -3", &ctx).unwrap(), Value::Number(7.0));
    }

    #[test]
    fn test_eval_double_negative() {
        let ctx = EvalContext::new();
        // --5 = 5
        assert_eq!(eval("--5", &ctx).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_eval_triple_negative() {
        let ctx = EvalContext::new();
        // ---5 = -5
        assert_eq!(eval("---5", &ctx).unwrap(), Value::Number(-5.0));
    }

    #[test]
    fn test_eval_comparison() {
        let ctx = EvalContext::new();
        assert_eq!(eval("5 > 3", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 < 3", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("5 = 5", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 <> 3", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_boolean_comparison() {
        let ctx = EvalContext::new();
        // TRUE > FALSE = true (TRUE is 1, FALSE is 0)
        assert_eq!(eval("TRUE > FALSE", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_boolean_number_equality() {
        let ctx = EvalContext::new();
        // Excel and Gnumeric: a boolean never equals a number
        assert_eq!(eval("TRUE = 1", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("FALSE = 0", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("TRUE <> 1", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_string_equality() {
        let ctx = EvalContext::new();
        // "ABC" = "ABC" = true (exact string match)
        assert_eq!(
            eval("\"ABC\" = \"ABC\"", &ctx).unwrap(),
            Value::Boolean(true)
        );
        // Note: Forge uses case-insensitive string comparison (differs from Excel)
        // In Excel: "ABC" = "abc" would be false (case sensitive)
        // In Forge: "ABC" = "abc" is true (case insensitive)
        assert_eq!(
            eval("\"ABC\" = \"abc\"", &ctx).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_eval_floating_point_precision() {
        let ctx = EvalContext::new();
        // Two numbers are compared to 15 significant digits, as in Excel
        // (ADR-065): 0.1 + 0.2 is 0.30000000000000004, and 0.3 to 15 digits
        assert_eq!(eval("0.1 + 0.2 = 0.3", &ctx).unwrap(), Value::Boolean(true));
        // ROUND also ensures equality
        assert_eq!(
            eval("ROUND(0.1 + 0.2, 1) = 0.3", &ctx).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_eval_boolean_addition() {
        let ctx = EvalContext::new();
        // TRUE + 1 = 2 (TRUE coerces to 1)
        assert_eq!(eval("TRUE + 1", &ctx).unwrap(), Value::Number(2.0));
        // FALSE + 1 = 1 (FALSE coerces to 0)
        assert_eq!(eval("FALSE + 1", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_eval_boolean_multiplication() {
        let ctx = EvalContext::new();
        // TRUE * 5 = 5 (TRUE coerces to 1)
        assert_eq!(eval("TRUE * 5", &ctx).unwrap(), Value::Number(5.0));
        // FALSE * 5 = 0 (FALSE coerces to 0)
        assert_eq!(eval("FALSE * 5", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_array_arithmetic_inside_aggregates() {
        let ctx = array_ctx();
        for (formula, expected) in [
            ("SUM(p.a * p.b)", 140.0),
            ("SUM(p.a + 1)", 9.0),
            ("SUM(1 + p.a)", 9.0),
            ("SUM(-p.a)", -6.0),
            ("SUM(p.b / p.a)", 30.0),
            ("SUM(p.a ^ 2)", 14.0),
            ("SUM(p.a * p.one)", 30.0),
            ("AVERAGE(p.b / p.a)", 10.0),
            ("MAX(p.a + p.b)", 33.0),
            ("MIN(p.a * p.b)", 10.0),
            ("SUM((p.a > 1) * p.b)", 50.0),
        ] {
            assert_eq!(
                eval(formula, &ctx).unwrap(),
                Value::Number(expected),
                "{formula}"
            );
        }
    }

    #[test]
    fn test_array_arithmetic_length_mismatch_is_error() {
        let err = eval("SUM(p.a * p.short)", &array_ctx()).unwrap_err();
        assert!(err.message.contains("length mismatch"), "{}", err.message);
    }

    #[test]
    fn test_array_division_by_zero_element_is_error() {
        let err = eval("SUM(p.a / (p.a - 1))", &array_ctx()).unwrap_err();
        assert!(err.message.contains("zero"), "{}", err.message);
    }

    /// A date is a date (ADR-054): the text `2026-01-31` in a model, and its day
    /// number wherever a formula reads a number. `+` adds and `&` joins, as in
    /// Excel. Each result here is what Gnumeric and `LibreOffice` give for the
    /// exported formula over a real date (forge-e2e-gnumeric,
    /// `edge_real_dates.yaml`).
    mod dates_as_dates {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::EvalContext;

        /// `+` joined text when either side was text: `"5" + 1` was `"51"` (Excel:
        /// 6) and a date plus a day was `"2026-01-311"` (Excel: the next day).
        #[test]
        fn plus_adds_as_in_excel() {
            assert!((h::number("\"5\" + 1") - 6.0).abs() < 1e-12);
            assert!((h::number("\"2026-01-31\" + 1") - 46054.0).abs() < 1e-12);
            assert!((h::number("TRUE + 1") - 2.0).abs() < 1e-12);
            let error = h::eval("\"a\" + \"b\"", &EvalContext::new())
                .unwrap_err()
                .to_string();
            assert!(error.contains("#VALUE!"), "{error}");
            assert!(error.contains("use & or CONCAT"), "{error}");
        }

        #[test]
        fn ampersand_joins_text() {
            assert_eq!(h::text("\"a\" & \"b\""), "ab");
            assert_eq!(h::text("\"n\" & 1"), "n1");
            assert_eq!(h::text("1 & 2"), "12");
            assert_eq!(h::text("\"due \" & \"2026-01-31\""), "due 2026-01-31");
            // It binds less than + and more than a comparison, as in Excel
            assert_eq!(h::text("\"x\" & 1 + 2"), "x3");
            assert!(h::truth("\"a\" & \"b\" = \"ab\""));
        }

        /// `d = DATE(2026, 1, 31)` was FALSE for the date 2026-01-31.
        #[test]
        fn a_date_equals_its_day_number() {
            assert!(h::truth("\"2026-01-31\" = DATE(2026, 1, 31)"));
            assert!(h::truth("DATE(2026, 1, 31) = \"2026-01-31\""));
            assert!(!h::truth("\"2026-01-31\" <> 46053"));
            assert!(!h::truth("\"2026-01-31\" = 46054"));
            // Other text is never equal to a number, as in Excel
            assert!(!h::truth("\"5\" = 5"));
            assert!(h::truth("\"2026-01-31\" = \"2026-01-31\""));
        }
    }
}
