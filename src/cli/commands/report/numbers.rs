//! A number as a report shows it: two decimals and thousands separators, four
//! significant digits under 1, an input as it was written.

use crate::core::array_calculator::evaluator::text::number_format::rounded_digits;

/// A number as a report shows it in a table or in Markdown.
///
/// Two decimals and thousands separators (`1,450.00`). A number under 1 keeps
/// four significant digits, at most six decimals, so that a rate of 0.0479 is
/// not shown as 0.05.
///
/// The digits are those a spreadsheet shows: the number to 15 significant
/// digits, then rounded half away from zero, so 2.675 is `2.68` although the
/// nearest double is a little under it.
#[must_use]
pub fn display_number(x: f64) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let magnitude = x.abs();
    let decimals = if magnitude == 0.0 || magnitude >= 1.0 {
        2
    } else if magnitude >= 0.1 {
        4
    } else if magnitude >= 0.01 {
        5
    } else {
        6
    };
    let (whole, mut fraction) = rounded_digits(magnitude, decimals);
    while fraction.len() > 2 && fraction.ends_with('0') {
        fraction.pop();
    }
    let is_zero = whole == "0" && fraction.bytes().all(|b| b == b'0');
    let sign = if x < 0.0 && !is_zero { "-" } else { "" };
    format!("{sign}{}.{fraction}", with_thousands(&whole))
}

/// The same number with no decimals, for a column of whole numbers
/// (`1,700`, `3`, `0`).
pub fn display_whole(x: f64) -> String {
    display_number(x).trim_end_matches(".00").to_string()
}

/// An input is shown as written when the column's rule would round it:
/// `0.14975` is not `0.1498`.
pub fn shown_input(x: f64, shown: String) -> String {
    let same = shown
        .replace(',', "")
        .parse::<f64>()
        .is_ok_and(|p| p.to_bits() == x.to_bits());
    if same || !x.is_finite() {
        return shown;
    }
    let text = x.abs().to_string();
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let sign = if x < 0.0 { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{}", with_thousands(whole))
    } else {
        format!("{sign}{}.{fraction}", with_thousands(whole))
    }
}

fn with_thousands(whole: &str) -> String {
    let mut out = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_two_decimals_and_thousands_separators() {
        assert_eq!(display_number(1450.0), "1,450.00");
        assert_eq!(display_number(1_234_567.891), "1,234,567.89");
        assert_eq!(display_number(-1234.5), "-1,234.50");
        assert_eq!(display_number(999.995), "1,000.00");
        assert_eq!(display_number(100.0), "100.00");
        assert_eq!(display_number(0.0), "0.00");
    }

    #[test]
    fn numbers_under_one_keep_four_significant_digits() {
        assert_eq!(display_number(0.0479), "0.0479");
        assert_eq!(display_number(0.5), "0.50");
        assert_eq!(display_number(0.25), "0.25");
        assert_eq!(display_number(0.123_456), "0.1235");
        assert_eq!(display_number(-0.0479), "-0.0479");
        assert_eq!(display_number(0.99996), "1.00");
        assert_eq!(display_number(0.000_123_4), "0.000123");
    }

    #[test]
    fn rounding_is_half_away_from_zero_on_the_shown_digits() {
        // The nearest doubles to these are a little under the tie.
        assert_eq!(display_number(2.675), "2.68");
        assert_eq!(display_number(1.005), "1.01");
        assert_eq!(display_number(-2.675), "-2.68");
        assert_eq!(display_number(1_234.565), "1,234.57");
    }

    #[test]
    fn a_residue_near_zero_is_zero_without_a_sign() {
        assert_eq!(display_number(-1.2e-10), "0.00");
        assert_eq!(display_number(4e-9), "0.00");
        assert_eq!(display_number(-0.0), "0.00");
    }

    #[test]
    fn numbers_that_are_not_finite_say_so() {
        assert_eq!(display_number(f64::NAN), "NaN");
        assert_eq!(display_number(f64::INFINITY), "inf");
    }

    /// An input the column's rule would round is shown as it was written, a
    /// whole one too, and a number that is not finite as it is.
    #[test]
    fn an_input_is_shown_as_written_where_rounding_would_change_it() {
        let shown = |x: f64| shown_input(x, display_number(x));
        assert_eq!(shown(0.14975), "0.14975");
        assert_eq!(shown(-0.14975), "-0.14975");
        assert_eq!(shown(1450.0), "1,450.00");
        assert_eq!(
            shown(1_234_567_890_123_456_789.0),
            "1,234,567,890,123,456,800"
        );
        assert_eq!(
            shown(-1_234_567_890_123_456_789.0),
            "-1,234,567,890,123,456,800"
        );
        assert_eq!(shown(f64::INFINITY), "inf");
    }
}
