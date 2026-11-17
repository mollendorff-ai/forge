//! Excel's day numbers for dates written `YYYY-MM-DD` (ADR-054, ADR-060).

use num_traits::ToPrimitive;

/// The day number of a date written `YYYY-MM-DD`: the days since 1899-12-30,
/// which is Excel's serial number for every date from 1900-03-01 on.
#[must_use]
pub fn date_serial(text: &str) -> Option<f64> {
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    #[allow(clippy::cast_precision_loss)] // a day count is far below 2^52
    Some((date - base).num_days() as f64)
}

/// The day number of a date Excel holds, written `YYYY-MM-DD`.
///
/// Four, two and two digits, from 1900-03-01 on: a date in a scalar, a date
/// column's cell in a workbook (ADR-054, ADR-060). `None` for any other text.
#[must_use]
pub fn excel_date_serial(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    let serial = date_serial(text).filter(|_| shaped)?;
    // 1900-03-01 is day 61: Excel counts a 29 February 1900 that never was
    (serial >= 61.0).then_some(serial)
}

/// The text of the date with day number `serial` (`2026-10-31` for 46326).
#[must_use]
pub fn serial_date_text(serial: f64) -> String {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap_or_default();
    // Whole days on from it; one no calendar holds is shown as its number
    serial
        .to_i64()
        .and_then(chrono::TimeDelta::try_days)
        .and_then(|days| base.checked_add_signed(days))
        .map_or_else(|| serial.to_string(), |d| d.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whole days on from 1899-12-30, the fraction cut; a number no calendar
    /// holds is shown as its number, where `TimeDelta::days` panicked
    #[test]
    fn a_serial_is_shown_as_its_date_or_its_number() {
        assert_eq!(serial_date_text(46326.0), "2026-10-31");
        assert_eq!(serial_date_text(46326.9), "2026-10-31");
        assert_eq!(serial_date_text(-0.5), "1899-12-30");
        assert_eq!(serial_date_text(-1.0), "1899-12-29");
        assert_eq!(serial_date_text(1e300), 1e300.to_string());
        assert_eq!(serial_date_text(1e15), "1000000000000000");
    }
}
