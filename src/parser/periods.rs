//! The period column (model format 5.2.0, ADR-053).
//!
//! A table column written as `{start: 2026-12, count: 300, unit: month}` is
//! the list of the dates on which `count` periods of one `unit` start, the
//! first at `start`. It is read as a column of dates (`2026-12-01`,
//! `2027-01-01`, ...), so everything that works on a date column works on it.

use crate::error::{ForgeError, ForgeResult};
use crate::types::ColumnValue;
use chrono::{Days, Months, NaiveDate};
use yaml_serde::{Mapping, Value};

/// The most rows a period column may have: the rows of an Excel sheet under
/// its header.
pub const MAX_PERIODS: u64 = 1_048_575;

const UNITS: [&str; 5] = ["day", "week", "month", "quarter", "year"];

/// Whether a column's mapping is a period column: it has `start` and `count`.
#[must_use]
pub fn is_period_column(map: &Mapping) -> bool {
    map.contains_key("start") && map.contains_key("count")
}

/// Whether a column's mapping was meant as a period column: it has `start` or
/// `count` (and no `value` or `formula`, the keys of the other mappings a
/// column may be).
#[must_use]
pub fn looks_like_period_column(map: &Mapping) -> bool {
    (map.contains_key("start") || map.contains_key("count"))
        && !map.contains_key("value")
        && !map.contains_key("formula")
}

/// The dates of a period column.
///
/// # Errors
///
/// Returns an error that names the column and the key when `start` is not a
/// month, a day or a year, `count` is not a whole number from 1 to
/// [`MAX_PERIODS`], `unit` is not one of the five units, a key is unknown, or
/// a date falls outside the calendar.
pub fn period_column(table: &str, column: &str, map: &Mapping) -> ForgeResult<ColumnValue> {
    let path = format!("{table}.{column}");
    let fail = |what: String| ForgeError::Parse(format!("'{path}': {what}"));
    if let Some(key) = map
        .keys()
        .filter_map(Value::as_str)
        .find(|k| !["start", "count", "unit"].contains(k))
    {
        return Err(fail(format!(
            "a period column has the keys start, count and unit; there is no key '{key}'"
        )));
    }

    let unit = map.get("unit").and_then(Value::as_str).unwrap_or_default();
    if !UNITS.contains(&unit) {
        return Err(fail(format!("unit must be one of {}", UNITS.join(", "))));
    }
    let count = map
        .get("count")
        .and_then(Value::as_u64)
        .filter(|n| (1..=MAX_PERIODS).contains(n))
        .ok_or_else(|| {
            fail(format!(
                "count must be a whole number from 1 to {MAX_PERIODS}"
            ))
        })?;
    let start = match map.get("start") {
        Some(Value::String(text)) => parse_start(text),
        Some(Value::Number(year)) => year
            .as_i64()
            .and_then(|y| i32::try_from(y).ok())
            .filter(|y| (1900..=9999).contains(y))
            .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1)),
        _ => None,
    }
    .ok_or_else(|| {
        fail("start must be a month (2026-12), a day (2026-12-15) or a year (2026)".to_string())
    })?;

    (0..count)
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)] // count is at most MAX_PERIODS
            let i = i as u32;
            let date = match unit {
                "day" => start.checked_add_days(Days::new(u64::from(i))),
                "week" => start.checked_add_days(Days::new(7 * u64::from(i))),
                "month" => start.checked_add_months(Months::new(i)),
                "quarter" => start.checked_add_months(Months::new(3 * i)),
                _ => start.checked_add_months(Months::new(12 * i)),
            };
            date.filter(|d| d.format("%Y").to_string().len() == 4)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .ok_or_else(|| fail(format!("period {} is past the year 9999", i + 1)))
        })
        .collect::<ForgeResult<Vec<String>>>()
        .map(ColumnValue::Date)
}

/// `2026-12` (the first of the month) or `2026-12-15`.
fn parse_start(text: &str) -> Option<NaiveDate> {
    let date = match text.len() {
        7 => NaiveDate::parse_from_str(&format!("{text}-01"), "%Y-%m-%d").ok(),
        10 => NaiveDate::parse_from_str(text, "%Y-%m-%d").ok(),
        _ => None,
    }?;
    (date >= NaiveDate::from_ymd_opt(1900, 1, 1)?).then_some(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dates(yaml: &str) -> Result<Vec<String>, String> {
        let map: Mapping = yaml_serde::from_str(yaml).unwrap();
        match period_column("t", "p", &map) {
            Ok(ColumnValue::Date(dates)) => Ok(dates),
            Ok(other) => Err(format!("{other:?}")),
            Err(e) => Err(e.to_string()),
        }
    }

    #[test]
    fn each_unit_steps_from_the_start() {
        assert_eq!(
            dates("{start: 2026-11, count: 3, unit: month}").unwrap(),
            ["2026-11-01", "2026-12-01", "2027-01-01"]
        );
        assert_eq!(
            dates("{start: 2026-12-30, count: 3, unit: day}").unwrap(),
            ["2026-12-30", "2026-12-31", "2027-01-01"]
        );
        assert_eq!(
            dates("{start: 2026-01-05, count: 2, unit: week}").unwrap(),
            ["2026-01-05", "2026-01-12"]
        );
        assert_eq!(
            dates("{start: 2026-10, count: 3, unit: quarter}").unwrap(),
            ["2026-10-01", "2027-01-01", "2027-04-01"]
        );
        assert_eq!(
            dates("{start: 2026, count: 2, unit: year}").unwrap(),
            ["2026-01-01", "2027-01-01"]
        );
    }

    /// Each date is counted from the start, so a month-end start stays as
    /// late in the month as the month allows (the way EDATE does).
    #[test]
    fn a_month_end_start_is_clamped_month_by_month() {
        assert_eq!(
            dates("{start: 2027-12-31, count: 4, unit: month}").unwrap(),
            ["2027-12-31", "2028-01-31", "2028-02-29", "2028-03-31"]
        );
        assert_eq!(
            dates("{start: 2028-02-29, count: 2, unit: year}").unwrap(),
            ["2028-02-29", "2029-02-28"]
        );
    }

    #[test]
    fn mistakes_name_the_column_and_the_key() {
        for (yaml, why) in [
            ("{start: 2026-13, count: 3, unit: month}", "start must be"),
            (
                "{start: \"December\", count: 3, unit: month}",
                "start must be",
            ),
            ("{start: 2026-12, count: 0, unit: month}", "count must be"),
            ("{start: 2026-12, count: 2.5, unit: month}", "count must be"),
            (
                "{start: 2026-12, count: 3, unit: months}",
                "unit must be one of day",
            ),
            ("{start: 2026-12, count: 3}", "unit must be one of day"),
            (
                "{start: 2026-12, count: 3, unit: month, step: 2}",
                "no key 'step'",
            ),
            (
                "{start: 9999-12, count: 3, unit: month}",
                "past the year 9999",
            ),
        ] {
            let err = dates(yaml).unwrap_err();
            assert!(err.contains("'t.p'") && err.contains(why), "{yaml}: {err}");
        }
    }
}
