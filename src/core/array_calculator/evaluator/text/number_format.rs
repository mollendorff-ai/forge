//! `TEXT(value, format)`: a value written with an Excel format code.
//!
//! The codes Forge reads are a defined part of Excel's:
//!
//! - numbers: `0` and `#` digits, one `.`, `,` between digits for thousands,
//!   `%`, scientific `E+00`/`E-00`, and literal characters (`$ - + ( ) :`,
//!   space, text in quotes, a character after `\`), which may sit between the
//!   digits (`0000-00-00`);
//! - dates: `yy yyyy`, `m mm mmm mmmm mmmmm`, `d dd ddd dddd`, with the same
//!   literals and `. , /` (month and day names in English);
//! - `General` and `@`.
//!
//! Any other code (several sections with `;`, times, fractions, `?`, `*`,
//! `_`, `[...]`, a `,` that scales) is an error that says so: a format Forge
//! does not know must not come out as a wrong string.

use super::super::{date_serial, EvalError, Value};
use chrono::{Datelike, NaiveDate};
use std::fmt::Write as _;

/// One part of a format code.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    /// `0` (always a digit) or `#` (a digit if there is one)
    Digit(char),
    Point,
    Comma,
    Percent,
    Literal(String),
    /// `E+00` or `E-00`: whether `+` is written, and the digits of the exponent
    Exponent {
        plus: bool,
        digits: usize,
    },
    /// A run of `y`, `m` or `d`: the letter and its length
    Date(char, usize),
}

fn unsupported(format: &str, why: &str) -> EvalError {
    EvalError::new(format!(
        "TEXT: the format \"{format}\" is not one Forge reads ({why})"
    ))
}

fn pieces(format: &str) -> Result<Vec<Piece>, EvalError> {
    let chars: Vec<char> = format.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' => {
                let end = chars[i + 1..]
                    .iter()
                    .position(|q| *q == '"')
                    .ok_or_else(|| unsupported(format, "a quote is not closed"))?;
                out.push(Piece::Literal(chars[i + 1..i + 1 + end].iter().collect()));
                i += end + 1;
            },
            '\\' => {
                let next = chars
                    .get(i + 1)
                    .ok_or_else(|| unsupported(format, "nothing follows the \\"))?;
                out.push(Piece::Literal(next.to_string()));
                i += 1;
            },
            '0' | '#' => out.push(Piece::Digit(c)),
            '.' => out.push(Piece::Point),
            ',' => out.push(Piece::Comma),
            '%' => out.push(Piece::Percent),
            'E' | 'e' if matches!(chars.get(i + 1), Some('+' | '-')) => {
                let digits = chars[i + 2..].iter().take_while(|d| **d == '0').count();
                if digits == 0 {
                    return Err(unsupported(format, "E+ or E- needs 0s after it"));
                }
                out.push(Piece::Exponent {
                    plus: chars[i + 1] == '+',
                    digits,
                });
                i += 1 + digits;
            },
            'y' | 'm' | 'd' | 'Y' | 'M' | 'D' => {
                let letter = c.to_ascii_lowercase();
                let run = chars[i..]
                    .iter()
                    .take_while(|x| x.to_ascii_lowercase() == letter)
                    .count();
                out.push(Piece::Date(letter, run));
                i += run - 1;
            },
            '$' | '-' | '+' | '(' | ')' | ':' | ' ' | '/' => {
                out.push(Piece::Literal(c.to_string()));
            },
            ';' => return Err(unsupported(format, "sections with ; are not read")),
            '?' | '*' | '_' | '[' | ']' | '@' => {
                return Err(unsupported(format, &format!("{c} is not read")))
            },
            'h' | 'H' | 's' | 'S' | 'a' | 'A' | 'p' | 'P' => {
                return Err(unsupported(
                    format,
                    "times are not read; Forge has dates only",
                ))
            },
            _ => {
                return Err(unsupported(
                    format,
                    &format!("write the text {c} in quotes"),
                ))
            },
        }
        i += 1;
    }
    Ok(out)
}

/// `value` written with `format`, as Excel's `TEXT` writes it.
///
/// # Errors
///
/// Returns an error for a format code Forge does not read, or a number that
/// is not a date where the format is a date's.
pub fn format_value(value: &Value, format: &str) -> Result<String, EvalError> {
    let trimmed = format.trim();
    if trimmed.eq_ignore_ascii_case("general") || trimmed == "@" {
        return Ok(value.as_text());
    }
    let pieces = pieces(format)?;
    let has_date = pieces.iter().any(|p| matches!(p, Piece::Date(..)));
    let has_number = pieces
        .iter()
        .any(|p| matches!(p, Piece::Digit(_) | Piece::Percent | Piece::Exponent { .. }));
    if has_date && has_number {
        return Err(unsupported(format, "it mixes a date and digits"));
    }
    // Text that is neither a number nor a date comes back as it is, as in
    // Excel; so does TRUE or FALSE.
    let number = match value {
        Value::Number(n) => *n,
        Value::Text(text) => match text
            .trim()
            .parse::<f64>()
            .ok()
            .or_else(|| date_serial(text))
        {
            Some(n) => n,
            None => return Ok(text.clone()),
        },
        other => return Ok(other.as_text()),
    };
    if has_date {
        write_date(number, &pieces, format)
    } else {
        write_number(number, &pieces, format)
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

fn write_date(serial: f64, pieces: &[Piece], format: &str) -> Result<String, EvalError> {
    let days = serial.floor();
    let date = if (0.0..=2_958_465.0).contains(&days) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked range
        NaiveDate::from_ymd_opt(1899, 12, 30)
            .and_then(|base| base.checked_add_days(chrono::Days::new(days as u64)))
    } else {
        None
    }
    .ok_or_else(|| EvalError::new(format!("#VALUE!: TEXT: {serial} is not a date")))?;

    let month = MONTHS[date.month0() as usize];
    let weekday = DAYS[date.weekday().num_days_from_monday() as usize];
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Date('y', n) if *n <= 2 => {
                let _ = write!(out, "{:02}", date.year() % 100);
            },
            Piece::Date('y', _) => {
                let _ = write!(out, "{:04}", date.year());
            },
            Piece::Date('m', 1) => out.push_str(&date.month().to_string()),
            Piece::Date('m', 2) => {
                let _ = write!(out, "{:02}", date.month());
            },
            Piece::Date('m', 3) => out.push_str(&month[..3]),
            Piece::Date('m', 4) => out.push_str(month),
            Piece::Date('m', 5) => out.push_str(&month[..1]),
            Piece::Date('d', 1) => out.push_str(&date.day().to_string()),
            Piece::Date('d', 2) => {
                let _ = write!(out, "{:02}", date.day());
            },
            Piece::Date('d', 3) => out.push_str(&weekday[..3]),
            Piece::Date('d', _) => out.push_str(weekday),
            Piece::Date(..) => return Err(unsupported(format, "more than five m")),
            Piece::Literal(text) => out.push_str(text),
            Piece::Point => out.push('.'),
            Piece::Comma => out.push(','),
            Piece::Digit(_) | Piece::Percent | Piece::Exponent { .. } => {},
        }
    }
    Ok(out)
}

/// The whole and fractional digits of a number, as a spreadsheet shows them.
///
/// A non-negative number is rounded to `decimals` places, half away from
/// zero, from its 15 significant digits: 2.675 is ("2", "68") at two places
/// although the nearest double is a little under 2.675.
#[must_use]
pub fn rounded_digits(magnitude: f64, decimals: usize) -> (String, String) {
    let scientific = format!("{magnitude:.14e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    let digit_at = |i: i64| -> u8 {
        usize::try_from(i)
            .ok()
            .and_then(|i| digits.get(i).copied())
            .unwrap_or(0)
    };

    // digits[0] is the 10^exponent place; keep every place down to 10^-decimals.
    let decimals_i64 = i64::try_from(decimals).unwrap_or(i64::MAX);
    let keep = exponent.saturating_add(1).saturating_add(decimals_i64);
    let mut kept: Vec<u8> = (0..keep.max(0)).map(digit_at).collect();
    if keep >= 0 && digit_at(keep) >= 5 {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, 1);
                break;
            }
            i -= 1;
            if kept[i] == 9 {
                kept[i] = 0;
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    while kept.len() <= decimals {
        kept.insert(0, 0);
    }
    let split = kept.len() - decimals;
    let to_text = |d: &[u8]| -> String { d.iter().map(|d| char::from(b'0' + d)).collect() };
    let whole = to_text(&kept[..split]);
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    (whole.to_string(), to_text(&kept[split..]))
}

fn with_thousands(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[allow(clippy::too_many_lines)] // one pass over the pieces of the code
fn write_number(value: f64, pieces: &[Piece], format: &str) -> Result<String, EvalError> {
    if !value.is_finite() {
        return Err(EvalError::new("#NUM!: TEXT of a number that is not finite"));
    }
    let percents = pieces.iter().filter(|p| **p == Piece::Percent).count();
    let mut magnitude = value.abs();
    for _ in 0..percents {
        magnitude *= 100.0;
    }

    let point = pieces.iter().position(|p| *p == Piece::Point);
    if pieces.iter().filter(|p| **p == Piece::Point).count() > 1 {
        return Err(unsupported(format, "it has two decimal points"));
    }
    let exponent_at = pieces
        .iter()
        .position(|p| matches!(p, Piece::Exponent { .. }));
    // The digits before the point end at the point, the exponent or the end.
    let whole_end = point.or(exponent_at).unwrap_or(pieces.len());
    let whole_pieces = &pieces[..whole_end];
    let fraction_pieces: &[Piece] = match (point, exponent_at) {
        (Some(p), Some(e)) if e > p => &pieces[p + 1..e],
        (Some(p), None) => &pieces[p + 1..],
        (Some(_), Some(_)) => return Err(unsupported(format, "the point follows the exponent")),
        (None, _) => &[],
    };
    let after: &[Piece] = exponent_at.map_or(&[], |e| &pieces[e..]);

    let is_digit = |p: &Piece| matches!(p, Piece::Digit(_));
    let first_digit = whole_pieces.iter().position(is_digit);
    let last_digit = whole_pieces.iter().rposition(is_digit);
    // A comma groups thousands only between two digits; anywhere else it
    // would scale the number or be a literal Excel treats in its own way.
    let mut thousands = false;
    for (i, piece) in pieces.iter().enumerate() {
        if *piece == Piece::Comma {
            let between = matches!((first_digit, last_digit), (Some(a), Some(b)) if a < i && i < b);
            if !between {
                return Err(unsupported(format, "a comma that is not between digits"));
            }
            thousands = true;
        }
    }
    let whole_slots: Vec<char> = whole_pieces
        .iter()
        .filter_map(|p| match p {
            Piece::Digit(c) => Some(*c),
            _ => None,
        })
        .collect();
    let fraction_slots: Vec<char> = fraction_pieces
        .iter()
        .filter_map(|p| match p {
            Piece::Digit(c) => Some(*c),
            _ => None,
        })
        .collect();
    let decimals = fraction_slots.len();

    let exponent_text =
        if let Some(Piece::Exponent { plus, digits }) = exponent_at.map(|e| &pieces[e]) {
            if whole_slots.len() != 1 || thousands {
                return Err(unsupported(
                    format,
                    "scientific formats are read with one digit before the point",
                ));
            }
            let mut power = 0i32;
            if magnitude != 0.0 {
                let scientific = format!("{magnitude:.14e}");
                let (mantissa, exp) = scientific.split_once('e').unwrap_or(("0", "0"));
                power = exp.parse().unwrap_or(0);
                magnitude = mantissa.parse().unwrap_or(0.0);
                // A mantissa that rounds up to 10 is 1 with the next power
                if rounded_digits(magnitude, decimals).0.len() > 1 {
                    magnitude /= 10.0;
                    power += 1;
                }
            }
            let sign = if power < 0 {
                "-"
            } else if *plus {
                "+"
            } else {
                ""
            };
            format!("E{sign}{:0width$}", power.abs(), width = *digits)
        } else {
            String::new()
        };

    let (whole, fraction) = rounded_digits(magnitude, decimals);
    // A whole part of zero has no digit of its own: `#.00` writes .50
    let mut digits: Vec<char> = if whole == "0" {
        Vec::new()
    } else {
        whole.chars().collect()
    };

    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if thousands {
        let between_literal = match (first_digit, last_digit) {
            (Some(a), Some(b)) => whole_pieces[a..b]
                .iter()
                .any(|p| matches!(p, Piece::Literal(_))),
            _ => false,
        };
        if between_literal {
            return Err(unsupported(format, "text between digits that are grouped"));
        }
        // Zeros down to the leftmost `0` of the code
        let minimum = whole_slots
            .iter()
            .position(|c| *c == '0')
            .map_or(0, |first| whole_slots.len() - first);
        while digits.len() < minimum {
            digits.insert(0, '0');
        }
        let grouped = with_thousands(&digits.iter().collect::<String>());
        let mut written = false;
        for piece in whole_pieces {
            match piece {
                Piece::Literal(text) => out.push_str(text),
                Piece::Percent => out.push('%'),
                Piece::Digit(_) if !written => {
                    out.push_str(&grouped);
                    written = true;
                },
                _ => {},
            }
        }
    } else {
        // Digits fill the slots from the right; those left over go in front
        // of the first slot. A `0` slot without a digit is 0, a `#` nothing.
        let slots = whole_slots.len();
        let mut filled: Vec<String> = Vec::with_capacity(slots);
        for (index, slot) in whole_slots.iter().enumerate() {
            let from_right = slots - index;
            let digit = digits
                .len()
                .checked_sub(from_right)
                .map(|at| digits[at].to_string());
            let mut text = digit.unwrap_or_else(|| {
                if *slot == '0' {
                    "0".to_string()
                } else {
                    String::new()
                }
            });
            if index == 0 && digits.len() > slots {
                let extra: String = digits[..digits.len() - slots].iter().collect();
                text.insert_str(0, &extra);
            }
            filled.push(text);
        }
        let mut slot = 0;
        for piece in whole_pieces {
            match piece {
                Piece::Literal(text) => out.push_str(text),
                Piece::Percent => out.push('%'),
                Piece::Digit(_) => {
                    out.push_str(&filled[slot]);
                    slot += 1;
                },
                _ => {},
            }
        }
        if slots == 0 {
            // No digit before the point in the code (`.00`): the whole part
            // is written all the same
            out.push_str(&digits.iter().collect::<String>());
        }
    }

    if point.is_some() {
        out.push('.');
        // Trailing zeros leave where their slot is `#`
        let mut shown = decimals;
        let fraction: Vec<char> = fraction.chars().collect();
        while shown > 0 && fraction_slots[shown - 1] == '#' && fraction[shown - 1] == '0' {
            shown -= 1;
        }
        let mut slot = 0;
        for piece in fraction_pieces {
            match piece {
                Piece::Digit(_) => {
                    if slot < shown {
                        out.push(fraction[slot]);
                    }
                    slot += 1;
                },
                Piece::Literal(text) => out.push_str(text),
                Piece::Percent => out.push('%'),
                _ => {},
            }
        }
    }
    if exponent_at.is_some() {
        out.push_str(&exponent_text);
        for piece in after.iter().skip(1) {
            match piece {
                Piece::Literal(text) => out.push_str(text),
                Piece::Percent => out.push('%'),
                _ => {},
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: f64, format: &str) -> String {
        format_value(&Value::Number(value), format).unwrap()
    }

    /// Each of these is what Gnumeric 1.12 and `LibreOffice` 26.2 both give
    /// (checked on 2026-11-10 by recalculating the exported workbook).
    #[test]
    fn numbers_as_both_engines_write_them() {
        for (value, format, expected) in [
            (1234.5, "0.00", "1234.50"),
            (1234.5, "#,##0.00", "1,234.50"),
            (1_234_567.891, "#,##0", "1,234,568"),
            (0.25, "0%", "25%"),
            (0.07, "0.0%", "7.0%"),
            (0.5, "0.00%", "50.00%"),
            (1234.5, "$#,##0.00", "$1,234.50"),
            (-1234.5, "$#,##0.00", "-$1,234.50"),
            (5.0, "000", "005"),
            (5.0, "00000", "00005"),
            (0.5, "#.00", ".50"),
            (0.0, "#,##0", "0"),
            (20_260_131.0, "0000-00-00", "2026-01-31"),
            (5.0, "000-00", "000-05"),
            (1.5, "0.0#", "1.5"),
            (1.25, "0.0#", "1.25"),
            (1234.5, "0", "1235"),
            (1234.5, "#", "1235"),
            (123_456_789.0, "0.00E+00", "1.23E+08"),
            (0.000_123, "0.00E+00", "1.23E-04"),
            (0.000_123, "0.0E-0", "1.2E-4"),
            (9.999, "0.00E+00", "1.00E+01"),
            (-5.0, "0", "-5"),
            (1_234_567.0, "#,###", "1,234,567"),
            (999.995, "#,##0.00", "1,000.00"),
            (1_000_000.0, "0,000", "1,000,000"),
            (12.0, "#,##0.00", "12.00"),
            (0.125, "0.00", "0.13"),
            (1e15, "#,##0", "1,000,000,000,000,000"),
            (123.456, "0.0", "123.5"),
            (1234.5, "General", "1234.5"),
        ] {
            assert_eq!(text(value, format), expected, "TEXT({value}, \"{format}\")");
        }
    }

    /// 2026-01-31 is day 46053, a Saturday.
    #[test]
    fn dates_as_both_engines_write_them() {
        for (format, expected) in [
            ("yyyy-mm-dd", "2026-01-31"),
            ("yyyy", "2026"),
            ("yy", "26"),
            ("m/d/yyyy", "1/31/2026"),
            ("mmm d, yyyy", "Jan 31, 2026"),
            ("mmmm", "January"),
            ("mmmmm", "J"),
            ("dddd", "Saturday"),
            ("ddd", "Sat"),
            ("dd.mm.yyyy", "31.01.2026"),
            ("d-mmm-yy", "31-Jan-26"),
            ("yyyymmdd", "20260131"),
            ("YYYY-MM-DD", "2026-01-31"),
            ("dddd, mmmm d", "Saturday, January 31"),
        ] {
            assert_eq!(text(46053.0, format), expected, "{format}");
        }
        // A date as Forge holds it is formatted as its day number is
        let date = Value::Text("2026-01-31".to_string());
        assert_eq!(format_value(&date, "mmm yyyy").unwrap(), "Jan 2026");
        assert_eq!(format_value(&date, "0").unwrap(), "46053");
    }

    /// Where the two engines differ, Forge follows Excel: the digits shown
    /// are rounded half away from zero from 15 significant digits
    /// (`LibreOffice` agrees, Gnumeric rounds the binary number), `#` writes
    /// nothing for zero and `.00` keeps the whole part (`LibreOffice` agrees),
    /// `0.##` keeps its point and TRUE stays TRUE (Gnumeric agrees).
    #[test]
    fn where_the_engines_differ_forge_follows_excel() {
        assert_eq!(text(1.2345, "0.000"), "1.235");
        assert_eq!(text(2.675, "0.00"), "2.68");
        assert_eq!(text(0.0, "#"), "");
        assert_eq!(text(12.5, ".00"), "12.50");
        assert_eq!(text(1.0, "0.##"), "1.");
        assert_eq!(format_value(&Value::Boolean(true), "0").unwrap(), "TRUE");
        // Not checked in either engine: both write 0.00, Excel is believed
        // to keep the sign of a negative number that rounds to zero
        assert_eq!(text(-0.001, "0.00"), "-0.00");
    }

    #[test]
    fn text_that_is_not_a_number_comes_back_as_it_is() {
        let abc = Value::Text("abc".to_string());
        assert_eq!(format_value(&abc, "0.00").unwrap(), "abc");
        assert_eq!(
            format_value(&Value::Text("5".to_string()), "0.00").unwrap(),
            "5.00"
        );
    }

    /// A format Forge does not read is an error, not a wrong string.
    #[test]
    fn other_formats_are_errors_that_say_why() {
        for (format, why) in [
            ("0.00;(0.00)", "sections with ;"),
            ("h:mm", "times are not read"),
            ("# ?/?", "? is not read"),
            ("[Red]0", "[ is not read"),
            ("0.0,", "a comma that is not between digits"),
            ("0.00 units", "write the text u in quotes"),
            ("yyyy 0", "mixes a date and digits"),
            ("0.0.0", "two decimal points"),
            ("00.0E+00", "one digit before the point"),
        ] {
            let error = format_value(&Value::Number(1.0), format)
                .unwrap_err()
                .to_string();
            assert!(error.contains(why), "{format}: {error}");
            assert!(error.contains("is not one Forge reads"), "{error}");
        }
        let error = format_value(&Value::Number(-1.0), "yyyy")
            .unwrap_err()
            .to_string();
        assert!(error.contains("#VALUE!"), "{error}");
    }

    #[test]
    fn text_in_quotes_and_after_a_backslash_is_written_as_it_is() {
        assert_eq!(text(1234.5, "0.00 \"units\""), "1234.50 units");
        assert_eq!(text(5.0, "\\x0"), "x5");
        assert_eq!(text(46053.0, "\"due \"d mmm"), "due 31 Jan");
    }
}
