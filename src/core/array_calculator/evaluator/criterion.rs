//! The criteria of SUMIF, COUNTIFS and the like (`">=5"`, `""`, `0`)

use super::{criterion_number, date_serial, wildcard, Value};

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
    /// `>=x`, `<=x`, `>x`, `<x`: the text after the operator and its number
    AtLeast(String, Option<f64>),
    AtMost(String, Option<f64>),
    Above(String, Option<f64>),
    Below(String, Option<f64>),
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
            return Self::AtLeast(rest.to_string(), criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix("<=") {
            return Self::AtMost(rest.to_string(), criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix("<>").or_else(|| text.strip_prefix("!=")) {
            let rest = rest.trim();
            return Self::Not(rest.to_string(), criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix('>') {
            return Self::Above(rest.to_string(), criterion_number(rest));
        }
        if let Some(rest) = text.strip_prefix('<') {
            return Self::Below(rest.to_string(), criterion_number(rest));
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
            (Test::AtLeast(_, Some(c)), Some(v)) => return v >= *c,
            (Test::AtMost(_, Some(c)), Some(v)) => return v <= *c,
            (Test::Above(_, Some(c)), Some(v)) => return v > *c,
            (Test::Below(_, Some(c)), Some(v)) => return v < *c,
            // A text criterion compares text with text, ignoring case, as in
            // Excel (">c" counts "date" and "x"); a number never meets it
            (Test::AtLeast(rest, None), _) => {
                return text_order(rest, val).is_some_and(std::cmp::Ordering::is_ge)
            },
            (Test::AtMost(rest, None), _) => {
                return text_order(rest, val).is_some_and(std::cmp::Ordering::is_le)
            },
            (Test::Above(rest, None), _) => {
                return text_order(rest, val).is_some_and(std::cmp::Ordering::is_gt)
            },
            (Test::Below(rest, None), _) => {
                return text_order(rest, val).is_some_and(std::cmp::Ordering::is_lt)
            },
            (Test::Not(_, Some(c)), Some(v)) => return (v - c).abs() > f64::EPSILON,
            (Test::Not(rest, _), _) => return !text_matches(rest, val),
            (Test::Equal(_, Some(c)), Some(v)) => return (v - c).abs() < f64::EPSILON,
            (Test::Equal(rest, _), _) => return text_matches(rest, val),
            _ => {},
        }
        // Direct comparison (numeric or text)
        if let (Some(v), Some(c)) = (v, self.number) {
            return (v - c).abs() < f64::EPSILON;
        }
        // Text comparison (case-insensitive, with wildcards)
        text_matches(&self.text, val)
    }
}

/// Whether a value equals a criterion's text, ignoring case; with `*`, `?`
/// or `~` in it the criterion is a pattern that only text matches, as in
/// Excel (`"b*"` counts no number and no empty cell).
fn text_matches(criterion: &str, val: &Value) -> bool {
    if wildcard::has_wildcards(criterion) {
        // An empty text is written to the workbook as an empty cell, which no
        // pattern matches, and a date is a number there (ADR-054)
        return matches!(val, Value::Text(t) if !t.is_empty()
            && date_serial(t).is_none()
            && wildcard::matches(criterion, t));
    }
    val.as_text().to_lowercase() == criterion.to_lowercase()
}

/// How a text value compares with a criterion's text, ignoring case; None
/// for a value that is not text, or empty
fn text_order(criterion: &str, val: &Value) -> Option<std::cmp::Ordering> {
    match val {
        // A date is a number in the workbook, not text (ADR-054)
        Value::Text(t) if !t.is_empty() && date_serial(t).is_none() => {
            Some(t.to_lowercase().cmp(&criterion.to_lowercase()))
        },
        _ => None,
    }
}
