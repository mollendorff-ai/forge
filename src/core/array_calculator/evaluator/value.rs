//! The value a formula evaluates to: a number, a text, a boolean, an
//! array, a lambda or an empty cell, and how each reads as a number or a
//! text where a formula needs one.

use super::{date_serial, number_text};
use crate::core::array_calculator::parser::Expr;

/// Value type that can be returned from evaluation
#[derive(Debug, Clone)]
pub enum Value {
    /// A numeric value
    Number(f64),
    /// A text value
    Text(String),
    /// A boolean value
    Boolean(bool),
    /// An array of values (for table columns), shared, not copied (T1-COLUMN-COPIES)
    Array(std::sync::Arc<Vec<Self>>),
    /// A lambda function value (parameter names, body expression)
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// Null/empty value
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Null, Self::Null) => true,
            _ => false, // Lambdas and mismatched types don't compare equal
        }
    }
}

impl Value {
    /// Try to convert to f64
    /// For text, tries numeric parsing first, then date parsing (YYYY-MM-DD -> Excel serial)
    ///
    /// # Panics
    ///
    /// Cannot panic in practice: the Excel epoch date (1899-12-30) is always valid.
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(s) => {
                // First a number, to 15 digits as Excel reads text
                if let Ok(n) = crate::core::precision::read_typed(s) {
                    return Some(n);
                }
                // Then a date (YYYY-MM-DD), as its day number
                date_serial(s)
            },
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            // A one-element array is its element. A longer array has no single
            // numeric value; callers must aggregate it or report an error.
            Self::Array(arr) => match arr.as_slice() {
                [only] => only.as_number(),
                _ => None,
            },
            Self::Lambda { .. } | Self::Null => None,
        }
    }

    /// A value used as one number, where an empty cell is 0, as in Excel
    /// (`=A3 + 1`, `ROUND(A3, 0)`, ADR-059). Elements of a range keep
    /// [`Value::as_number`], which skips an empty cell, as aggregates must.
    #[must_use]
    pub fn as_cell_number(&self) -> Option<f64> {
        match self {
            Self::Null => Some(0.0),
            other => other.as_number(),
        }
    }

    /// Try to convert to string
    pub fn as_text(&self) -> String {
        match self {
            Self::Number(n) => number_text(*n),
            Self::Text(s) => s.clone(),
            Self::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Self::Null => String::new(),
            Self::Array(arr) => {
                let strs: Vec<String> = arr.iter().map(Self::as_text).collect();
                format!("[{}]", strs.join(", "))
            },
            Self::Lambda { params, .. } => {
                format!("LAMBDA({})", params.join(", "))
            },
        }
    }

    /// Try to convert to boolean
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Array(arr) if arr.len() == 1 => arr[0].as_bool(),
            Self::Number(n) => Some(*n != 0.0),
            Self::Text(s) => {
                let upper = s.to_uppercase();
                if upper == "TRUE" || upper == "1" {
                    Some(true)
                } else if upper == "FALSE" || upper == "0" {
                    Some(false)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Check if value is truthy
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        self.as_bool().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_array_equality() {
        let arr1 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)].into());
        let arr2 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)].into());
        let arr3 = Value::Array(vec![Value::Number(1.0)].into());
        assert_eq!(arr1, arr2);
        assert_ne!(arr1, arr3);
    }

    #[test]
    fn test_value_null_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_ne!(Value::Null, Value::Number(0.0));
    }

    #[test]
    fn test_value_lambda_never_equal() {
        use crate::core::array_calculator::parser::Expr;
        let lambda1 = Value::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(Expr::Number(1.0)),
        };
        let lambda2 = Value::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(Expr::Number(1.0)),
        };
        // Lambdas never compare equal
        assert_ne!(lambda1, lambda2);
    }

    #[test]
    fn test_value_array_as_number() {
        // An array has no single number, so it must not become its length
        let arr =
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)].into());
        assert_eq!(arr.as_number(), None);
        assert_eq!(
            Value::Array(vec![Value::Number(7.0)].into()).as_number(),
            Some(7.0)
        );
    }

    #[test]
    fn test_value_lambda_as_number() {
        use crate::core::array_calculator::parser::Expr;
        let lambda = Value::Lambda {
            params: vec![],
            body: Box::new(Expr::Number(42.0)),
        };
        assert_eq!(lambda.as_number(), None);
    }

    #[test]
    fn test_value_null_as_number() {
        assert_eq!(Value::Null.as_number(), None);
    }

    #[test]
    fn test_value_date_string_as_number() {
        // Date string converts to Excel serial number
        let date = Value::Text("2024-01-01".to_string());
        // Excel serial: days since 1899-12-30
        // 2024-01-01 is 45292 days from 1899-12-30
        assert_eq!(date.as_number(), Some(45292.0));

        // Invalid date string returns None
        let invalid = Value::Text("not-a-date".to_string());
        assert_eq!(invalid.as_number(), None);

        // Numeric string still works
        let num = Value::Text("42.5".to_string());
        assert_eq!(num.as_number(), Some(42.5));
    }

    #[test]
    fn test_value_mixed_type_inequality() {
        // Different types should not be equal
        assert_ne!(Value::Number(1.0), Value::Text("1".to_string()));
        assert_ne!(Value::Boolean(true), Value::Number(1.0));
        assert_ne!(Value::Array(vec![].into()), Value::Null);
    }

    #[test]
    fn each_kind_of_value_reads_as_a_text() {
        let lambda = Value::Lambda {
            params: vec!["x".to_string(), "y".to_string()],
            body: Box::new(Expr::Number(1.0)),
        };
        let nested = Value::Array(
            vec![
                Value::Number(1.5),
                Value::Text("a".to_string()),
                Value::Boolean(false),
                Value::Null,
            ]
            .into(),
        );
        for (value, expected) in [
            (Value::Number(2.0), "2"),
            (Value::Text("b".to_string()), "b"),
            (Value::Boolean(true), "TRUE"),
            (Value::Boolean(false), "FALSE"),
            (Value::Null, ""),
            (nested, "[1.5, a, FALSE, ]"),
            (lambda, "LAMBDA(x, y)"),
        ] {
            assert_eq!(value.as_text(), expected);
        }
    }

    #[test]
    fn each_kind_of_value_reads_as_a_boolean() {
        let text = |s: &str| Value::Text(s.to_string());
        let one = |v: Value| Value::Array(vec![v].into());
        for (value, expected) in [
            (Value::Boolean(true), Some(true)),
            (one(Value::Boolean(false)), Some(false)),
            (one(Value::Number(3.0)), Some(true)),
            (Value::Number(0.0), Some(false)),
            (Value::Number(-0.5), Some(true)),
            (text("true"), Some(true)),
            (text("1"), Some(true)),
            (text("False"), Some(false)),
            (text("0"), Some(false)),
            (text("yes"), None),
            (Value::Null, None),
            (
                Value::Array(vec![Value::Boolean(true), Value::Boolean(true)].into()),
                None,
            ),
        ] {
            assert_eq!(value.as_bool(), expected, "{value:?}");
            assert_eq!(value.is_truthy(), expected == Some(true), "{value:?}");
        }
    }

    #[test]
    fn an_empty_cell_is_zero_as_one_number_and_others_read_as_numbers() {
        assert_eq!(Value::Null.as_cell_number(), Some(0.0));
        assert_eq!(Value::Boolean(true).as_cell_number(), Some(1.0));
        assert_eq!(Value::Boolean(false).as_number(), Some(0.0));
        assert_eq!(Value::Text("x".to_string()).as_cell_number(), None);
        assert_eq!(Value::Text("a".into()), Value::Text("a".into()));
        assert_ne!(Value::Text("a".into()), Value::Text("b".into()));
        assert_eq!(Value::Boolean(true), Value::Boolean(true));
        assert_ne!(Value::Number(1.0), Value::Number(2.0));
    }

    /// An empty cell (ADR-059): the table of the ADR, measured in Gnumeric 1.12
    /// and `LibreOffice` 26.2 on 2026-11-12, which agree on every value here.
    /// `x` is 10, empty, 30, empty, 0; `blank` is one empty cell; `b` is a column
    /// of empty cells.
    mod empty_cells {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::Value;

        #[test]
        fn an_empty_cell_is_what_excel_reads_it_as() {
            let ctx = h::empty_cells();
            let b = Value::Boolean;
            for (formula, expected) in [
                ("blank + 1", Value::Number(1.0)),
                ("blank * 5", Value::Number(0.0)),
                ("-blank", Value::Number(0.0)),
                ("blank = 0", b(true)),
                ("blank = \"\"", b(true)),
                ("blank = FALSE", b(true)),
                ("blank <> 0", b(false)),
                ("blank <> \"\"", b(false)),
                ("blank = 1", b(false)),
                ("blank < 1", b(true)),
                ("ISBLANK(blank)", b(true)),
                ("ISNUMBER(blank)", b(false)),
                ("IF(blank, 1, 2)", Value::Number(2.0)),
                ("\"[\" & blank & \"]\"", Value::Text("[]".into())),
                ("LEN(blank)", Value::Number(0.0)),
                ("ROUND(blank, 0)", Value::Number(0.0)),
                ("ABS(blank)", Value::Number(0.0)),
            ] {
                assert_eq!(h::eval(formula, &ctx).unwrap(), expected, "{formula}");
            }
        }
    }
}
