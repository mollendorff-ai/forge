//! The criteria of SUMIF, COUNTIFS and the like (`">=5"`, `""`, `0`)

use super::{criterion_number, Value};

/// A criterion of SUMIF, COUNTIFS and the like, read once and then tested
/// against each value of the range (reading it for each value turned the
/// criterion into text every time).
pub struct Criterion {
    /// The criterion as text
    text: String,
    /// The criterion as a number, for a direct comparison
    number: Option<f64>,
    test: Test,
}

/// A comparison written at the start of a criterion (`">=5"`)
enum Test {
    AtLeast(Option<f64>),
    AtMost(Option<f64>),
    Above(Option<f64>),
    Below(Option<f64>),
    /// `<>x`: the text after the operator and its number
    Not(String, Option<f64>),
    /// `=x`
    Equal(String, Option<f64>),
    /// No operator: equal to the criterion
    Plain,
}

impl Test {
    /// The comparison a criterion's text starts with
    fn of(text: &str) -> Self {
        if let Some(rest) = text.strip_prefix(">=") {
            return Self::AtLeast(criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix("<=") {
            return Self::AtMost(criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix("<>").or_else(|| text.strip_prefix("!=")) {
            let rest = rest.trim();
            return Self::Not(rest.to_string(), criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix('>') {
            return Self::Above(criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix('<') {
            return Self::Below(criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix('=') {
            let rest = rest.trim();
            return Self::Equal(rest.to_string(), criterion_number(rest));
        }
        Self::Plain
    }
}

impl Criterion {
    pub fn new(criteria: &Value) -> Self {
        let text = criteria.as_text();
        let comparison = Test::of(&text);
        Self {
            number: criteria.as_number(),
            test: comparison,
            text,
        }
    }

    /// Whether `val` meets the criterion. A comparison whose sides are not
    /// both numbers falls back to equality with the criterion's text.
    pub fn matches(&self, val: &Value) -> bool {
        let v = val.as_number();
        match (&self.test, v) {
            (Test::AtLeast(Some(c)), Some(v)) => return v >= *c,
            (Test::AtMost(Some(c)), Some(v)) => return v <= *c,
            (Test::Above(Some(c)), Some(v)) => return v > *c,
            (Test::Below(Some(c)), Some(v)) => return v < *c,
            (Test::Not(_, Some(c)), Some(v)) => return (v - c).abs() > f64::EPSILON,
            (Test::Not(rest, _), _) => return val.as_text() != *rest,
            (Test::Equal(_, Some(c)), Some(v)) => return (v - c).abs() < f64::EPSILON,
            (Test::Equal(rest, _), _) => return val.as_text().eq_ignore_ascii_case(rest),
            _ => {},
        }
        // Direct comparison (numeric or text)
        if let (Some(v), Some(c)) = (v, self.number) {
            return (v - c).abs() < f64::EPSILON;
        }
        // Text comparison (case-insensitive)
        val.as_text().eq_ignore_ascii_case(&self.text)
    }
}
