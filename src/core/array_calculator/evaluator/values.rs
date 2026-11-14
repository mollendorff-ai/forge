//! Values read the way Excel reads them: a number as text, an error that is
//! #N/A, and the pairs of numbers of two ranges

use super::{collect_values_as_vec, EvalContext, EvalError, Expr};

/// A number as Excel turns it into text (`CONCAT`, `LEN`, `&`).
///
/// At most 15 significant digits, so `0.1 + 0.2` is `0.3` and `1/3` is
/// `0.333333333333333`; a whole number below 1E+16 in full; from 1E+15 on
/// otherwise, and below 1E-10, in the form `1E+20`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn number_text(n: f64) -> String {
    if !n.is_finite() {
        return format!("{n}");
    }
    if n == 0.0 {
        return "0".to_string();
    }
    // A whole number below 1E+16 is written in full, as both engines write it
    if n.fract() == 0.0 && n.abs() < 1e16 {
        return format!("{}", n as i64);
    }
    // The shortest decimal that is this double, rounded half up to 15
    // significant digits, as LibreOffice does (Excel keeps 15): rounding the
    // binary value instead wrote -985667.1985084685 as ...468, not ...469
    let shortest = format!("{:e}", n.abs());
    let (mantissa, exponent) = shortest.split_once('e').unwrap_or((&shortest, "0"));
    let mut exponent: i32 = exponent.parse().unwrap_or(0);
    let mut digits: Vec<u8> = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|d| d - b'0')
        .collect();
    if digits.len() > 15 {
        let up = digits[15] >= 5;
        digits.truncate(15);
        if up {
            let mut i = 15;
            loop {
                if i == 0 {
                    digits.insert(0, 1);
                    digits.truncate(15);
                    exponent += 1;
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
    let sign = if n < 0.0 { "-" } else { "" };
    if !(-10..15).contains(&exponent) {
        let (first, rest) = text.split_at(1);
        let rest = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let e_sign = if exponent < 0 { '-' } else { '+' };
        return format!("{sign}{first}{rest}E{e_sign}{:02}", exponent.abs());
    }
    let point = exponent + 1; // digits before the decimal point
    if point <= 0 {
        let zeros = "0".repeat(usize::try_from(-point).unwrap_or(0));
        format!("{sign}0.{zeros}{text}")
    } else {
        let point = usize::try_from(point).unwrap_or(0);
        if text.len() <= point {
            format!("{sign}{text}{}", "0".repeat(point - text.len()))
        } else {
            format!("{sign}{}.{}", &text[..point], &text[point..])
        }
    }
}

/// Whether an error is #N/A: `NA()`, or a lookup that finds nothing (MATCH,
/// XMATCH, XLOOKUP, VLOOKUP, HLOOKUP, RANK.EQ, IFS and SWITCH with no match).
/// IFNA and ISNA catch these and no other.
pub fn is_na_error(error: &EvalError) -> bool {
    error.to_string().contains("#N/A")
}

/// The numbers of two ranges taken in pairs, as SLOPE, CORREL and the like
/// read them: a pair where either cell is empty or not a number is left out,
/// as in Excel. Ranges of different sizes are #N/A.
pub fn paired_numbers(
    name: &str,
    first: &Expr,
    second: &Expr,
    ctx: &EvalContext,
) -> Result<(Vec<f64>, Vec<f64>), EvalError> {
    let a = collect_values_as_vec(first, ctx)?;
    let b = collect_values_as_vec(second, ctx)?;
    if a.len() != b.len() {
        return Err(EvalError::new(format!(
            "#N/A: {name} needs two ranges of the same size, got {} and {}",
            a.len(),
            b.len()
        )));
    }
    Ok(a.iter()
        .zip(&b)
        .filter_map(|(x, y)| Some((x.as_number()?, y.as_number()?)))
        .unzip())
}
