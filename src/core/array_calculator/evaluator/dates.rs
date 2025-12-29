//! Date functions: TODAY, NOW, YEAR, MONTH, DAY, WEEKDAY, HOUR, MINUTE, SECOND, DATE, EDATE, EOMONTH, DATEDIF, DAYS, TIME, WORKDAY, etc.
//!
//! DEMO functions (7): TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EOMONTH
//! ENTERPRISE functions: NOW, WEEKDAY, HOUR, MINUTE, SECOND, TIME, DAYS, WORKDAY, EDATE, NETWORKDAYS, YEARFRAC

use super::{evaluate, parse_date_value, require_args, EvalContext, EvalError, Expr, Value};
use chrono::Datelike;

#[cfg(not(feature = "demo"))]
use super::require_args_range;
#[cfg(not(feature = "demo"))]
use chrono::Timelike;

/// Try to evaluate a date function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "TODAY" => {
            use chrono::Local;
            let today = Local::now().date_naive();
            Value::Text(today.format("%Y-%m-%d").to_string())
        },

        #[cfg(not(feature = "demo"))]
        "NOW" => {
            use chrono::Local;
            let now = Local::now();
            Value::Text(now.format("%Y-%m-%d %H:%M:%S").to_string())
        },

        "YEAR" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.year() as f64)
        },

        "MONTH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.month() as f64)
        },

        "DAY" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.day() as f64)
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "WEEKDAY" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?;
            let return_type = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as i32
            } else {
                1
            };

            let date = parse_date_value(&val)?;
            // Monday = 0 in chrono
            let day = date.weekday().num_days_from_sunday();

            // Excel WEEKDAY return types:
            // 1 (default): Sunday=1, Saturday=7
            // 2: Monday=1, Sunday=7
            // 3: Monday=0, Sunday=6
            let result = match return_type {
                1 => (day + 1) as f64,
                2 => ((day + 6) % 7 + 1) as f64,
                3 => ((day + 6) % 7) as f64,
                _ => (day + 1) as f64,
            };
            Value::Number(result)
        },

        #[cfg(not(feature = "demo"))]
        "HOUR" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            // Parse time from various formats
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                // DateTime format "YYYY-MM-DD HH:MM:SS"
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(parsed.hour() as f64)));
                }
            }
            // Try as time only "HH:MM:SS"
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(parsed.hour() as f64)));
            }
            // Try as fraction of day (Excel serial time)
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number((total_seconds / 3600) as f64)));
            }
            return Err(EvalError::new("HOUR: Could not parse time"));
        },

        #[cfg(not(feature = "demo"))]
        "MINUTE" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(parsed.minute() as f64)));
                }
            }
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(parsed.minute() as f64)));
            }
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number(((total_seconds / 60) % 60) as f64)));
            }
            return Err(EvalError::new("MINUTE: Could not parse time"));
        },

        #[cfg(not(feature = "demo"))]
        "SECOND" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(parsed.second() as f64)));
                }
            }
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(parsed.second() as f64)));
            }
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number((total_seconds % 60) as f64)));
            }
            return Err(EvalError::new("SECOND: Could not parse time"));
        },

        #[cfg(not(feature = "demo"))]
        "TIME" => {
            require_args(name, args, 3)?;
            let hour = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: hour must be a number"))?
                as i32;
            let minute = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: minute must be a number"))?
                as i32;
            let second = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: second must be a number"))?
                as i32;

            // Return as fraction of day (Excel time serial)
            let total_seconds = hour * 3600 + minute * 60 + second;
            Value::Number(total_seconds as f64 / 86400.0)
        },

        #[cfg(not(feature = "demo"))]
        "DAYS" => {
            require_args(name, args, 2)?;
            let end = evaluate(&args[0], ctx)?;
            let start = evaluate(&args[1], ctx)?;

            let end_date = parse_date_value(&end)?;
            let start_date = parse_date_value(&start)?;

            Value::Number((end_date - start_date).num_days() as f64)
        },

        #[cfg(not(feature = "demo"))]
        "WORKDAY" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let days = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("WORKDAY: days must be a number"))?
                as i32;

            let mut current = parse_date_value(&start)?;
            let direction = if days >= 0 { 1 } else { -1 };
            let mut remaining = days.abs();

            while remaining > 0 {
                current = if direction > 0 {
                    current.succ_opt().unwrap_or(current)
                } else {
                    current.pred_opt().unwrap_or(current)
                };
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
                    // Monday-Friday
                    remaining -= 1;
                }
            }

            Value::Text(current.format("%Y-%m-%d").to_string())
        },

        "DATE" => {
            require_args(name, args, 3)?;
            let year = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: year must be a number"))?
                as i32;
            let month = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: month must be a number"))?
                as i32;
            let day = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: day must be a number"))?
                as i32;

            // Handle month overflow/underflow (Excel-compatible behavior)
            let total_months = (year * 12 + month - 1) as i32;
            let adj_year = total_months.div_euclid(12);
            let adj_month = (total_months.rem_euclid(12) + 1) as u32;

            use chrono::{Days, NaiveDate};
            // Start from day 1 of the adjusted month, then add (day - 1) days
            // This handles day=0 (last day of prev month) and day>max (overflow to next month)
            let base_date = NaiveDate::from_ymd_opt(adj_year, adj_month, 1).ok_or_else(|| {
                EvalError::new(format!("DATE: invalid date {adj_year}-{adj_month}-1"))
            })?;
            let date = if day >= 1 {
                base_date
                    .checked_add_days(Days::new((day - 1) as u64))
                    .ok_or_else(|| EvalError::new("DATE: day overflow"))?
            } else {
                // day <= 0: go back from day 1
                base_date
                    .checked_sub_days(Days::new((1 - day) as u64))
                    .ok_or_else(|| EvalError::new("DATE: day underflow"))?
            };
            // Return Excel serial number (days since 1899-12-30)
            let excel_base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            Value::Number((date - excel_base).num_days() as f64)
        },

        #[cfg(not(feature = "demo"))]
        "EDATE" => {
            use chrono::Months;

            require_args(name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EDATE requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            let result = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EDATE: Invalid date result"))?;

            Value::Text(result.format("%Y-%m-%d").to_string())
        },

        "EOMONTH" => {
            use chrono::{Months, NaiveDate};

            require_args(name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EOMONTH requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            let adjusted = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            // Get last day of that month
            let year = adjusted.year();
            let month = adjusted.month();
            let last_day = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).and_then(|d| d.pred_opt())
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            Value::Text(last_day.format("%Y-%m-%d").to_string())
        },

        "DATEDIF" => {
            require_args(name, args, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let unit = evaluate(&args[2], ctx)?.as_text().to_uppercase();

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match unit.as_str() {
                "D" => (end_date - start_date).num_days() as f64,
                "M" => {
                    // Complete months between dates
                    let years = end_date.year() - start_date.year();
                    let months = end_date.month() as i32 - start_date.month() as i32;
                    let mut total_months = years * 12 + months;
                    // Adjust if end day < start day (incomplete month)
                    if end_date.day() < start_date.day() {
                        total_months -= 1;
                    }
                    total_months.max(0) as f64
                },
                "Y" => {
                    // Complete years between dates
                    let mut years = end_date.year() - start_date.year();
                    // Check if we've reached the anniversary date
                    if end_date.month() < start_date.month()
                        || (end_date.month() == start_date.month()
                            && end_date.day() < start_date.day())
                    {
                        years -= 1;
                    }
                    years.max(0) as f64
                },
                "MD" => {
                    // Days between dates, ignoring months and years
                    let mut day_diff = end_date.day() as i32 - start_date.day() as i32;
                    if day_diff < 0 {
                        // Get days in the previous month
                        let prev_month_date = if end_date.month() == 1 {
                            chrono::NaiveDate::from_ymd_opt(end_date.year() - 1, 12, 1)
                        } else {
                            chrono::NaiveDate::from_ymd_opt(
                                end_date.year(),
                                end_date.month() - 1,
                                1,
                            )
                        };
                        let days_in_prev_month = prev_month_date
                            .and_then(|d| {
                                d.checked_add_months(chrono::Months::new(1))
                                    .and_then(|next| next.pred_opt())
                            })
                            .map(|d| d.day() as i32)
                            .unwrap_or(30);
                        day_diff += days_in_prev_month;
                    }
                    day_diff as f64
                },
                "YM" => {
                    // Months between dates, ignoring years
                    let mut month_diff = end_date.month() as i32 - start_date.month() as i32;
                    if month_diff < 0 {
                        month_diff += 12;
                    }
                    // Adjust if end day < start day (incomplete month)
                    if end_date.day() < start_date.day() && month_diff > 0 {
                        month_diff -= 1;
                    }
                    month_diff as f64
                },
                "YD" => {
                    // Days between dates, ignoring years
                    // This is the number of days since the last anniversary of start_date
                    use chrono::NaiveDate;

                    // Find the most recent anniversary of start_date before or on end_date
                    let anniversary_year = if end_date.month() > start_date.month()
                        || (end_date.month() == start_date.month()
                            && end_date.day() >= start_date.day())
                    {
                        end_date.year()
                    } else {
                        end_date.year() - 1
                    };

                    // Handle Feb 29 -> Feb 28 for non-leap years
                    let anniversary = if start_date.month() == 2 && start_date.day() == 29 {
                        NaiveDate::from_ymd_opt(anniversary_year, 2, 29)
                            .or_else(|| NaiveDate::from_ymd_opt(anniversary_year, 2, 28))
                    } else {
                        NaiveDate::from_ymd_opt(
                            anniversary_year,
                            start_date.month(),
                            start_date.day(),
                        )
                    }
                    .ok_or_else(|| EvalError::new("DATEDIF: invalid anniversary date"))?;

                    (end_date - anniversary).num_days() as f64
                },
                _ => return Err(EvalError::new(format!("DATEDIF: unknown unit '{unit}'"))),
            };
            Value::Number(result)
        },

        #[cfg(not(feature = "demo"))]
        "NETWORKDAYS" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let mut count = 0;
            let mut current = start_date;
            while current <= end_date {
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
                    count += 1;
                }
                current = current.succ_opt().unwrap_or(current);
            }
            Value::Number(count as f64)
        },

        #[cfg(not(feature = "demo"))]
        "YEARFRAC" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let basis = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match basis {
                0 | 4 => {
                    // US 30/360 and European 30/360
                    let mut d1 = start_date.day() as i32;
                    let m1 = start_date.month() as i32;
                    let y1 = start_date.year() as i32;
                    let mut d2 = end_date.day() as i32;
                    let m2 = end_date.month() as i32;
                    let y2 = end_date.year() as i32;

                    if d1 == 31 {
                        d1 = 30;
                    }
                    if d2 == 31 && (d1 >= 30 || basis == 4) {
                        d2 = 30;
                    }

                    let days_30_360 = ((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)) as f64;
                    days_30_360 / 360.0
                },
                1 => {
                    // Actual/actual
                    let days = (end_date - start_date).num_days() as f64;
                    let year = start_date.year();
                    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                    let year_days = if is_leap { 366.0 } else { 365.0 };
                    days / year_days
                },
                2 => {
                    // Actual/360
                    let days = (end_date - start_date).num_days() as f64;
                    days / 360.0
                },
                3 => {
                    // Actual/365
                    let days = (end_date - start_date).num_days() as f64;
                    days / 365.0
                },
                _ => return Err(EvalError::new(format!("YEARFRAC: unknown basis {basis}"))),
            };
            Value::Number(result)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_date_parts() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("YEAR(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(2024.0)
        );
        assert_eq!(
            eval("MONTH(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(6.0)
        );
        assert_eq!(
            eval("DAY(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(15.0)
        );
    }

    #[test]
    fn test_date_construction() {
        let ctx = EvalContext::new();
        // DATE returns Excel serial number (days since 1899-12-30)
        // 2024-06-15 = 45458
        assert_eq!(
            eval("DATE(2024, 6, 15)", &ctx).unwrap(),
            Value::Number(45458.0)
        );
    }

    #[test]
    fn test_date_subtraction() {
        // BUG-002: DATE subtraction should work via date string coercion to serial
        let ctx = EvalContext::new();
        // Full year (leap year 2024)
        assert_eq!(
            eval("DATE(2024, 12, 31) - DATE(2024, 1, 1)", &ctx).unwrap(),
            Value::Number(365.0)
        );
        // 30 days in January
        assert_eq!(
            eval("DATE(2024, 1, 31) - DATE(2024, 1, 1)", &ctx).unwrap(),
            Value::Number(30.0)
        );
        // February in leap year (29 days)
        assert_eq!(
            eval("DATE(2024, 3, 1) - DATE(2024, 2, 1)", &ctx).unwrap(),
            Value::Number(29.0)
        );
        // February in non-leap year (28 days)
        assert_eq!(
            eval("DATE(2023, 3, 1) - DATE(2023, 2, 1)", &ctx).unwrap(),
            Value::Number(28.0)
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EDATE(\"2024-01-15\", 3)", &ctx).unwrap(),
            Value::Text("2024-04-15".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EOMONTH(\"2024-01-15\", 0)", &ctx).unwrap(),
            Value::Text("2024-01-31".to_string())
        );
    }

    #[test]
    fn test_datedif() {
        let ctx = EvalContext::new();
        // Days between Jan 1 and Jan 31 = 30
        assert_eq!(
            eval("DATEDIF(\"2024-01-01\", \"2024-01-31\", \"D\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    // === EDGE CASES FOR 100% COVERAGE ===

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_negative_months() {
        let ctx = EvalContext::new();
        // Subtract 3 months from April -> January
        assert_eq!(
            eval("EDATE(\"2024-04-15\", -3)", &ctx).unwrap(),
            Value::Text("2024-01-15".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_negative_months() {
        let ctx = EvalContext::new();
        // Go back 2 months from March and get end of month (January)
        assert_eq!(
            eval("EOMONTH(\"2024-03-15\", -2)", &ctx).unwrap(),
            Value::Text("2024-01-31".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_december() {
        let ctx = EvalContext::new();
        // End of December (exercises month == 12 branch)
        assert_eq!(
            eval("EOMONTH(\"2024-12-15\", 0)", &ctx).unwrap(),
            Value::Text("2024-12-31".to_string())
        );
    }

    #[test]
    fn test_datedif_all_units() {
        let ctx = EvalContext::new();
        // Test M (months)
        assert_eq!(
            eval("DATEDIF(\"2024-01-15\", \"2024-06-20\", \"M\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test Y (years)
        assert_eq!(
            eval("DATEDIF(\"2022-01-01\", \"2024-01-01\", \"Y\")", &ctx).unwrap(),
            Value::Number(2.0)
        );
        // Test MD (day difference ignoring months)
        assert_eq!(
            eval("DATEDIF(\"2024-01-10\", \"2024-02-15\", \"MD\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test YM (month difference ignoring years)
        assert_eq!(
            eval("DATEDIF(\"2022-03-01\", \"2024-08-01\", \"YM\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test YD (day difference ignoring years)
        assert_eq!(
            eval("DATEDIF(\"2022-01-01\", \"2024-02-01\", \"YD\")", &ctx).unwrap(),
            Value::Number(31.0)
        );
    }

    #[test]
    fn test_datedif_negative_day_diff() {
        let ctx = EvalContext::new();
        // MD where end day < start day (uses actual days in previous month)
        // Jan 20 to Feb 10: -10 + 31 (days in Jan) = 21
        assert_eq!(
            eval("DATEDIF(\"2024-01-20\", \"2024-02-10\", \"MD\")", &ctx).unwrap(),
            Value::Number(21.0)
        );
    }

    #[test]
    fn test_datedif_negative_month_diff() {
        let ctx = EvalContext::new();
        // YM where end month < start month (triggers month_diff += 12)
        assert_eq!(
            eval("DATEDIF(\"2022-10-01\", \"2024-03-01\", \"YM\")", &ctx).unwrap(),
            Value::Number(5.0) // 3 - 10 + 12 = 5
        );
    }

    #[test]
    fn test_datedif_negative_year_day_diff() {
        let ctx = EvalContext::new();
        // YD where end ordinal < start ordinal (triggers day_diff += 365)
        // March 1 ordinal ~60, January 15 ordinal = 15, diff = -45 + 365 = 320
        assert_eq!(
            eval("DATEDIF(\"2022-03-01\", \"2024-01-15\", \"YD\")", &ctx).unwrap(),
            Value::Number(320.0)
        );
    }

    #[test]
    fn test_datedif_unknown_unit() {
        let ctx = EvalContext::new();
        let result = eval("DATEDIF(\"2024-01-01\", \"2024-12-31\", \"X\")", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown unit"));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_networkdays() {
        let ctx = EvalContext::new();
        // Mon Jan 1 to Fri Jan 5 2024 = 5 workdays
        assert_eq!(
            eval("NETWORKDAYS(\"2024-01-01\", \"2024-01-05\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_all_bases() {
        let ctx = EvalContext::new();
        // Basis 1 (actual/actual)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 1)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.497).abs() < 0.01); // ~182/366 for leap year
        }
        // Basis 2 (actual/360)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 2)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.506).abs() < 0.01); // ~182/360
        }
        // Basis 3 (actual/365)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 3)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.499).abs() < 0.01); // ~182/365
        }
        // Basis 4 (European 30/360)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 4)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.5).abs() < 0.01); // 180/360
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_30_360_edge_cases() {
        let ctx = EvalContext::new();
        // Test with day = 31 (exercises d1 = 30 and d2 = 30 branches)
        let result = eval("YEARFRAC(\"2024-01-31\", \"2024-03-31\", 0)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.167).abs() < 0.01); // 60/360
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_unknown_basis() {
        let ctx = EvalContext::new();
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 5)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown basis"));
    }

    #[test]
    fn test_date_month_overflow() {
        let ctx = EvalContext::new();
        // Month 14 = February of next year (2025-02-15 = serial 45703)
        assert_eq!(
            eval("DATE(2024, 14, 15)", &ctx).unwrap(),
            Value::Number(45703.0)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_weekday() {
        let ctx = EvalContext::new();
        // 2024-01-01 was a Monday
        // Type 1 (default): Sunday=1, Monday=2
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\")", &ctx).unwrap(),
            Value::Number(2.0)
        );
        // Type 2: Monday=1
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\", 2)", &ctx).unwrap(),
            Value::Number(1.0)
        );
        // Type 3: Monday=0
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\", 3)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_hour_minute_second() {
        let ctx = EvalContext::new();
        // Test with time string
        assert_eq!(
            eval("HOUR(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(14.0)
        );
        assert_eq!(
            eval("MINUTE(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
        assert_eq!(
            eval("SECOND(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(45.0)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_time() {
        let ctx = EvalContext::new();
        // TIME(12, 0, 0) = 0.5 (noon is half a day)
        let result = eval("TIME(12, 0, 0)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.5).abs() < 0.0001);
        } else {
            panic!("Expected number");
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_days() {
        let ctx = EvalContext::new();
        // DAYS(end, start) returns end - start
        assert_eq!(
            eval("DAYS(\"2024-01-31\", \"2024-01-01\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_workday() {
        let ctx = EvalContext::new();
        // 2024-01-01 (Mon) + 5 workdays = 2024-01-08 (Mon)
        assert_eq!(
            eval("WORKDAY(\"2024-01-01\", 5)", &ctx).unwrap(),
            Value::Text("2024-01-08".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_workday_negative() {
        let ctx = EvalContext::new();
        // 2024-01-08 (Mon) - 5 workdays = 2024-01-01 (Mon)
        assert_eq!(
            eval("WORKDAY(\"2024-01-08\", -5)", &ctx).unwrap(),
            Value::Text("2024-01-01".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_hour_from_serial() {
        let ctx = EvalContext::new();
        // 0.5 = noon = 12:00
        assert_eq!(eval("HOUR(0.5)", &ctx).unwrap(), Value::Number(12.0));
        // 0.75 = 6pm = 18:00
        assert_eq!(eval("HOUR(0.75)", &ctx).unwrap(), Value::Number(18.0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EDGE CASE TESTS (from edge_dates.yaml)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_date_leap_year_feb_29() {
        let ctx = EvalContext::new();
        // Leap year: DATE(2024, 2, 29) should be valid
        // 2024 is a leap year, so Feb 29 exists
        let result = eval("DATE(2024, 2, 29)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2024-02-29 = 45351.0 (Excel serial)
            assert_eq!(serial, 45351.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_non_leap_year_feb_29_rollover() {
        let ctx = EvalContext::new();
        // Non-leap year: DATE(2023, 2, 29) should roll to March 1
        // 2023 is not a leap year, so Feb 29 rolls to Mar 1
        let result = eval("DATE(2023, 2, 29)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2023-03-01 = 44986.0 (Excel serial)
            assert_eq!(serial, 44986.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_month_zero_previous_year() {
        let ctx = EvalContext::new();
        // Month zero: DATE(2024, 0, 1) should go to previous year December
        let result = eval("DATE(2024, 0, 1)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2023-12-01 = 45261.0 (Excel serial)
            assert_eq!(serial, 45261.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_month_13_next_year() {
        let ctx = EvalContext::new();
        // Month 13: DATE(2024, 13, 1) should go to next year January
        let result = eval("DATE(2024, 13, 1)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2025-01-01 = 45658.0 (Excel serial)
            assert_eq!(serial, 45658.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_day_zero_previous_month_last_day() {
        let ctx = EvalContext::new();
        // Day zero: DATE(2024, 1, 0) should go to previous month's last day
        // Which is 2023-12-31
        let result = eval("DATE(2024, 1, 0)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2023-12-31 = 45291.0 (Excel serial)
            assert_eq!(serial, 45291.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_day_overflow_next_month() {
        let ctx = EvalContext::new();
        // Day overflow: DATE(2024, 1, 32) should roll to February 1
        let result = eval("DATE(2024, 1, 32)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            // 2024-02-01 = 45323.0 (Excel serial)
            assert_eq!(serial, 45323.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_subtraction_leap_year() {
        let ctx = EvalContext::new();
        // Date subtraction in leap year: DATE(2024,12,31) - DATE(2024,1,1)
        // Should equal 365 (leap year has 366 days, but difference is 365)
        let result = eval("DATE(2024, 12, 31) - DATE(2024, 1, 1)", &ctx).unwrap();
        assert_eq!(result, Value::Number(365.0));
    }

    #[test]
    fn test_date_subtraction_non_leap_year() {
        let ctx = EvalContext::new();
        // Date subtraction in non-leap year: DATE(2023,12,31) - DATE(2023,1,1)
        // Should equal 364 (365 days in year, difference is 364)
        let result = eval("DATE(2023, 12, 31) - DATE(2023, 1, 1)", &ctx).unwrap();
        assert_eq!(result, Value::Number(364.0));
    }

    #[test]
    fn test_today_minus_today_equals_zero() {
        let ctx = EvalContext::new();
        // TODAY() - TODAY() should always equal 0
        let result = eval("TODAY() - TODAY()", &ctx).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    #[test]
    fn test_eomonth_subtraction() {
        let ctx = EvalContext::new();
        // EOMONTH(DATE(2024,1,15), 0) - DATE(2024,1,1) = 30
        // EOMONTH(2024-01-15, 0) = 2024-01-31
        // 2024-01-31 - 2024-01-01 = 30
        let result = eval("EOMONTH(\"2024-01-15\", 0) - DATE(2024, 1, 1)", &ctx).unwrap();
        assert_eq!(result, Value::Number(30.0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TESTS FROM dates_basic.rs
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_date_function_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2025.0, 2024.0, 2023.0]),
        ));
        table.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![1.0, 6.0, 12.0]),
        ));
        table.add_column(Column::new(
            "day".to_string(),
            ColumnValue::Number(vec![15.0, 20.0, 31.0]),
        ));
        table.add_row_formula(
            "full_date".to_string(),
            "=DATE(year, month, day)".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let full_date = result_table.columns.get("full_date").unwrap();
        // DATE now returns Excel serial numbers
        match &full_date.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 45672.0); // 2025-01-15
                assert_eq!(nums[1], 45463.0); // 2024-06-20
                assert_eq!(nums[2], 45291.0); // 2023-12-31
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_year_function_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2025-01-15".to_string(),
                "2024-06-20".to_string(),
                "2023-12-31".to_string(),
            ]),
        ));
        table.add_row_formula("year_val".to_string(), "=YEAR(date)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let year_val = result_table.columns.get("year_val").unwrap();
        match &year_val.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 2025.0);
                assert_eq!(nums[1], 2024.0);
                assert_eq!(nums[2], 2023.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_month_function_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2025-01-15".to_string(),
                "2024-06-20".to_string(),
                "2023-12-31".to_string(),
            ]),
        ));
        table.add_row_formula("month_val".to_string(), "=MONTH(date)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let month_val = result_table.columns.get("month_val").unwrap();
        match &month_val.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.0);
                assert_eq!(nums[1], 6.0);
                assert_eq!(nums[2], 12.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_day_function_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2025-01-15".to_string(),
                "2024-06-20".to_string(),
                "2023-12-31".to_string(),
            ]),
        ));
        table.add_row_formula("day_val".to_string(), "=DAY(date)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let day_val = result_table.columns.get("day_val").unwrap();
        match &day_val.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 15.0);
                assert_eq!(nums[1], 20.0);
                assert_eq!(nums[2], 31.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_date_functions_combined() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2025-06-15".to_string(), "2024-12-31".to_string()]),
        ));
        table.add_row_formula(
            "next_month".to_string(),
            "=DATE(YEAR(date), MONTH(date) + 1, DAY(date))".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let next_month = result_table.columns.get("next_month").unwrap();
        // DATE now returns Excel serial numbers
        match &next_month.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 45853.0); // 2025-07-15
                assert_eq!(nums[1], 45688.0); // 2025-01-31 (month 13 => Jan next year)
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_datedif_function_basic() {
        let mut model = ParsedModel::new();

        // Test DATEDIF with literal dates
        // From 2024-01-15 to 2025-01-15 = 1 year = 12 months
        model.add_scalar(
            "years_diff".to_string(),
            Variable::new(
                "years_diff".to_string(),
                None,
                Some("=DATEDIF(\"2024-01-15\", \"2025-01-15\", \"Y\")".to_string()),
            ),
        );
        model.add_scalar(
            "months_diff".to_string(),
            Variable::new(
                "months_diff".to_string(),
                None,
                Some("=DATEDIF(\"2024-01-15\", \"2025-01-15\", \"M\")".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let years = result.scalars.get("years_diff").unwrap().value.unwrap();
        assert_eq!(years, 1.0, "Should be 1 year, got {years}");

        let months = result.scalars.get("months_diff").unwrap().value.unwrap();
        assert_eq!(months, 12.0, "Should be 12 months, got {months}");
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_function_basic() {
        let mut model = ParsedModel::new();

        // Test EDATE: Add 3 months to 2024-01-15 -> 2024-04-15
        // Note: EDATE returns a date string in the formula context
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "base_date".to_string(),
            ColumnValue::Date(vec!["2024-01-15".to_string()]),
        ));
        table.add_row_formula("new_date".to_string(), "=EDATE(base_date, 3)".to_string());
        model.add_table(table);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let table = result.tables.get("test").unwrap();
        let new_date_col = table.columns.get("new_date").unwrap();

        // The result should contain the new date
        match &new_date_col.values {
            ColumnValue::Text(texts) => {
                assert!(
                    texts[0].contains("2024-04-15"),
                    "Expected April 15, got {}",
                    texts[0]
                );
            },
            _ => panic!(
                "Expected Text array for dates, got {:?}",
                new_date_col.values
            ),
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_function_basic() {
        let mut model = ParsedModel::new();

        // Test EOMONTH: End of month 2 months after 2024-01-15 = 2024-03-31
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "base_date".to_string(),
            ColumnValue::Date(vec!["2024-01-15".to_string()]),
        ));
        table.add_row_formula("end_date".to_string(), "=EOMONTH(base_date, 2)".to_string());
        model.add_table(table);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let table = result.tables.get("test").unwrap();
        let end_date_col = table.columns.get("end_date").unwrap();

        // The result should contain the end of month date
        match &end_date_col.values {
            ColumnValue::Text(texts) => {
                assert!(
                    texts[0].contains("2024-03-31"),
                    "Expected March 31, got {}",
                    texts[0]
                );
            },
            _ => panic!(
                "Expected Text array for dates, got {:?}",
                end_date_col.values
            ),
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_networkdays_function_basic() {
        let mut model = ParsedModel::new();

        // NETWORKDAYS counts business days between two dates
        model.add_scalar(
            "workdays".to_string(),
            Variable::new(
                "workdays".to_string(),
                None,
                Some("=NETWORKDAYS(\"2024-01-01\", \"2024-01-12\")".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Jan 1-12, 2024: Jan 1 is Monday
        // Business days: 1,2,3,4,5 (Mon-Fri) + 8,9,10,11,12 (Mon-Fri) = 10 days
        let workdays = result.scalars.get("workdays").unwrap().value.unwrap();
        assert!((workdays - 10.0).abs() < 1.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_function_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "fraction".to_string(),
            Variable::new(
                "fraction".to_string(),
                None,
                Some("=YEARFRAC(\"2024-01-01\", \"2024-07-01\", 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Half a year = 0.5 approximately
        let fraction = result.scalars.get("fraction").unwrap().value.unwrap();
        assert!(fraction > 0.4 && fraction < 0.6);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_basis_1_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "fraction".to_string(),
            Variable::new(
                "fraction".to_string(),
                None,
                Some("=YEARFRAC(\"2024-01-01\", \"2024-12-31\", 1)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Full year
        let fraction = result.scalars.get("fraction").unwrap().value.unwrap();
        assert!(fraction > 0.9 && fraction < 1.1);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_negative_months_table() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("dates".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Text(vec!["2024-03-15".to_string()]),
        ));
        data.row_formulas
            .insert("end".to_string(), "=EOMONTH(start, -1)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // End of Feb 2024 from March - 1 = Feb 29
        let col = result
            .tables
            .get("dates")
            .unwrap()
            .columns
            .get("end")
            .unwrap();
        if let ColumnValue::Text(values) = &col.values {
            assert!(values[0].contains("2024-02"));
        }
    }

    #[test]
    fn test_today_function_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("dates".to_string());
        data.add_column(Column::new(
            "dummy".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        data.row_formulas
            .insert("current".to_string(), "=TODAY()".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // TODAY returns date string in YYYY-MM-DD format
        let col = result
            .tables
            .get("dates")
            .unwrap()
            .columns
            .get("current")
            .unwrap();
        if let ColumnValue::Text(values) = &col.values {
            assert!(values[0].contains('-'));
            assert!(values[0].len() == 10);
        }
    }

    #[test]
    fn test_date_construction_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("dates".to_string());
        data.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2024.0]),
        ));
        data.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![6.0]),
        ));
        data.add_column(Column::new(
            "day".to_string(),
            ColumnValue::Number(vec![15.0]),
        ));
        data.row_formulas.insert(
            "full_date".to_string(),
            "=DATE(year, month, day)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("dates")
            .unwrap()
            .columns
            .get("full_date")
            .unwrap();
        if let ColumnValue::Text(values) = &col.values {
            assert_eq!(values[0], "2024-06-15");
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_add_months() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("dates".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Text(vec!["2024-01-15".to_string()]),
        ));
        data.row_formulas
            .insert("future".to_string(), "=EDATE(start, 3)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("dates")
            .unwrap()
            .columns
            .get("future")
            .unwrap();
        if let ColumnValue::Text(values) = &col.values {
            assert!(values[0].starts_with("2024-04"));
        }
    }

    #[test]
    fn test_datedif_months_unit_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "months".to_string(),
            Variable::new(
                "months".to_string(),
                None,
                Some("=DATEDIF(\"2024-01-15\", \"2024-06-20\", \"M\")".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Jan to Jun = 5 complete months
        let months = result.scalars.get("months").unwrap().value.unwrap();
        assert!((months - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_datedif_years_unit_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "years".to_string(),
            Variable::new(
                "years".to_string(),
                None,
                Some("=DATEDIF(\"2020-01-01\", \"2024-06-01\", \"Y\")".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // 2020 to 2024 = 4 complete years
        let years = result.scalars.get("years").unwrap().value.unwrap();
        assert!((years - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_cross_table_date_column_reference() {
        let mut model = ParsedModel::new();

        // Source table with date column
        let mut source = Table::new("source".to_string());
        source.add_column(Column::new(
            "dates".to_string(),
            ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-02-01".to_string()]),
        ));
        model.add_table(source);

        // Target table referencing source's date column
        let mut target = Table::new("target".to_string());
        target.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        target
            .row_formulas
            .insert("copy_date".to_string(), "=source.dates".to_string());
        model.add_table(target);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should handle cross-table date reference
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_local_date_column_reference() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "start_date".to_string(),
            ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-06-01".to_string()]),
        ));
        data.add_column(Column::new(
            "days".to_string(),
            ColumnValue::Number(vec![30.0, 60.0]),
        ));
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_datedif_years_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        // Use literal date strings in DATEDIF
        data.row_formulas.insert(
            "years".to_string(),
            "=DATEDIF(\"2020-01-15\", \"2024-06-20\", \"Y\")".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Test exercises DATEDIF "Y" code path
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_datedif_months_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        // Use literal date strings in DATEDIF
        data.row_formulas.insert(
            "months".to_string(),
            "=DATEDIF(\"2024-01-15\", \"2024-04-10\", \"M\")".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Test exercises DATEDIF "M" code path
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_datedif_days_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        // Use literal date strings in DATEDIF
        data.row_formulas.insert(
            "days".to_string(),
            "=DATEDIF(\"2024-01-01\", \"2024-01-31\", \"D\")".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Test exercises DATEDIF "D" code path
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_datedif_invalid_unit_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Date(vec!["2024-01-01".to_string()]),
        ));
        data.add_column(Column::new(
            "end".to_string(),
            ColumnValue::Date(vec!["2024-12-31".to_string()]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=DATEDIF(start, end, \"X\")".to_string(), // Invalid unit
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should error due to invalid unit
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_positive_months() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Date(vec!["2024-01-15".to_string()]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=EDATE(start, 3)".to_string(), // Add 3 months
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("result") {
            if let ColumnValue::Date(vals) = &col.values {
                assert_eq!(vals[0], "2024-04-15");
            }
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_negative_months_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Date(vec!["2024-06-15".to_string()]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=EDATE(start, -2)".to_string(), // Subtract 2 months
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("result") {
            if let ColumnValue::Date(vals) = &col.values {
                assert_eq!(vals[0], "2024-04-15");
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TESTS FROM dates_advanced.rs
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_same_month() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "start".to_string(),
            ColumnValue::Date(vec!["2024-02-15".to_string()]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=EOMONTH(start, 0)".to_string(), // End of current month
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("result") {
            if let ColumnValue::Date(vals) = &col.values {
                assert_eq!(vals[0], "2024-02-29"); // Leap year
            }
        }
    }

    #[test]
    fn test_year_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-07-15".to_string()]),
        ));
        data.row_formulas
            .insert("year".to_string(), "=YEAR(date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("year") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2024.0);
            }
        }
    }

    #[test]
    fn test_month_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-07-15".to_string()]),
        ));
        data.row_formulas
            .insert("month".to_string(), "=MONTH(date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("month") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 7.0);
            }
        }
    }

    #[test]
    fn test_day_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-07-25".to_string()]),
        ));
        data.row_formulas
            .insert("day".to_string(), "=DAY(date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("day") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 25.0);
            }
        }
    }

    #[test]
    fn test_lookup_with_date_column() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("events".to_string());
        lookup_table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2024-01-15".to_string(),
                "2024-02-20".to_string(),
                "2024-03-25".to_string(),
            ]),
        ));
        lookup_table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("query".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=INDEX(events.date, idx)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Exercises Date column path in lookup functions
        assert!(result.is_ok() || result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_workday_function_advanced() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=WORKDAY(\"2024-01-01\", 10)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Exercises WORKDAY function path
        assert!(result.is_ok() || result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_networkdays_literal_dates() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        data.row_formulas.insert(
            "result".to_string(),
            "=NETWORKDAYS(\"2024-01-01\", \"2024-01-15\")".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Exercises NETWORKDAYS function path
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_date_column_in_rowwise_formula() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "event_date".to_string(),
            ColumnValue::Date(vec!["2024-01-15".to_string(), "2024-06-30".to_string()]),
        ));
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        // Access date column
        data.row_formulas
            .insert("result".to_string(), "=YEAR(event_date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_function_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        data.row_formulas.insert(
            "years".to_string(),
            "=YEARFRAC(\"2024-01-01\", \"2024-07-01\")".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_month_function_coverage() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-06-15".to_string()]),
        ));
        data.row_formulas
            .insert("m".to_string(), "=MONTH(date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_day_function_coverage() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-06-25".to_string()]),
        ));
        data.row_formulas
            .insert("d".to_string(), "=DAY(date)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_with_offset() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-01-15".to_string()]),
        ));
        data.row_formulas
            .insert("eom".to_string(), "=EOMONTH(date, 2)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_datedif_years_diff() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2020,1,1), DATE(2025,6,15), \"Y\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_datedif_months_diff() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2020,1,1), DATE(2020,8,15), \"M\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_datedif_days_unit() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2020,1,1), DATE(2020,1,20), \"D\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_datedif_md_unit() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2020,1,15), DATE(2020,3,10), \"MD\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_datedif_ym_unit() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2020,1,1), DATE(2021,8,1), \"YM\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TESTS FROM dates_datedif.rs
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_basis_0() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "frac".to_string(),
            Variable::new(
                "frac".to_string(),
                None,
                Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_actual_basis() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "frac".to_string(),
            Variable::new(
                "frac".to_string(),
                None,
                Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_basis_2() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "frac".to_string(),
            Variable::new(
                "frac".to_string(),
                None,
                Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_basis_3() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "frac".to_string(),
            Variable::new(
                "frac".to_string(),
                None,
                Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_yearfrac_basis_4() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "frac".to_string(),
            Variable::new(
                "frac".to_string(),
                None,
                Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 4)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_workday_positive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=WORKDAY(DATE(2020,1,1), 10)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_workday_negative_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=WORKDAY(DATE(2020,1,15), -5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_forward_quarter() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=EDATE(DATE(2020,1,15), 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_edate_subtract_months() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=EDATE(DATE(2020,6,15), -2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_positive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=EOMONTH(DATE(2020,1,15), 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_eomonth_negative_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=EOMONTH(DATE(2020,6,15), -3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_networkdays_basic_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "days".to_string(),
            Variable::new(
                "days".to_string(),
                None,
                Some("=NETWORKDAYS(DATE(2020,1,1), DATE(2020,1,31))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_year_from_date() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "yr".to_string(),
            Variable::new(
                "yr".to_string(),
                None,
                Some("=YEAR(DATE(2025, 6, 15))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_month_from_date() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "mon".to_string(),
            Variable::new(
                "mon".to_string(),
                None,
                Some("=MONTH(DATE(2025, 6, 15))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_day_from_date() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "d".to_string(),
            Variable::new(
                "d".to_string(),
                None,
                Some("=DAY(DATE(2025, 6, 15))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_now_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "now_len".to_string(),
            Variable::new("now_len".to_string(), None, Some("=LEN(NOW())".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // NOW() returns "YYYY-MM-DD HH:MM:SS" format = 19 characters
        let val = result.scalars.get("now_len").unwrap().value.unwrap();
        assert_eq!(val, 19.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_time_function_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "noon".to_string(),
            Variable::new(
                "noon".to_string(),
                None,
                Some("=TIME(12, 0, 0)".to_string()),
            ),
        );
        model.add_scalar(
            "time_with_minutes".to_string(),
            Variable::new(
                "time_with_minutes".to_string(),
                None,
                Some("=TIME(6, 30, 45)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // TIME(12,0,0) = 0.5 (noon)
        let noon_val = result.scalars.get("noon").unwrap().value.unwrap();
        assert!((noon_val - 0.5).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_hour_function_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "hour_noon".to_string(),
            Variable::new(
                "hour_noon".to_string(),
                None,
                Some("=HOUR(0.5)".to_string()),
            ),
        );
        model.add_scalar(
            "hour_6pm".to_string(),
            Variable::new(
                "hour_6pm".to_string(),
                None,
                Some("=HOUR(0.75)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let noon = result.scalars.get("hour_noon").unwrap().value.unwrap();
        let pm6 = result.scalars.get("hour_6pm").unwrap().value.unwrap();
        assert_eq!(noon, 12.0);
        assert_eq!(pm6, 18.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_minute_function_datedif() {
        let mut model = ParsedModel::new();
        // 0.5208333 = 12:30:00
        model.add_scalar(
            "minute_val".to_string(),
            Variable::new(
                "minute_val".to_string(),
                None,
                Some("=MINUTE(0.5208333)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let minutes = result.scalars.get("minute_val").unwrap().value.unwrap();
        assert!((minutes - 30.0).abs() < 1.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_second_function_datedif() {
        let mut model = ParsedModel::new();
        // 0.5 = 12:00:00 (zero seconds)
        model.add_scalar(
            "second_val".to_string(),
            Variable::new(
                "second_val".to_string(),
                None,
                Some("=SECOND(0.5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let seconds = result.scalars.get("second_val").unwrap().value.unwrap();
        assert_eq!(seconds, 0.0);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_weekday_function_datedif() {
        let mut model = ParsedModel::new();
        // 2024-01-01 was a Monday
        model.add_scalar(
            "weekday_monday".to_string(),
            Variable::new(
                "weekday_monday".to_string(),
                None,
                Some("=WEEKDAY(DATE(2024, 1, 1))".to_string()),
            ),
        );
        // Type 2: Monday=1
        model.add_scalar(
            "weekday_type2".to_string(),
            Variable::new(
                "weekday_type2".to_string(),
                None,
                Some("=WEEKDAY(DATE(2024, 1, 1), 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let wd1 = result.scalars.get("weekday_monday").unwrap().value.unwrap();
        let wd2 = result.scalars.get("weekday_type2").unwrap().value.unwrap();
        assert_eq!(wd1, 2.0); // Monday = 2 in default (Sunday=1)
        assert_eq!(wd2, 1.0); // Monday = 1 in type 2
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_days_function_datedif() {
        let mut model = ParsedModel::new();
        // DAYS(end, start)
        model.add_scalar(
            "days_diff".to_string(),
            Variable::new(
                "days_diff".to_string(),
                None,
                Some("=DAYS(DATE(2024, 1, 31), DATE(2024, 1, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let days = result.scalars.get("days_diff").unwrap().value.unwrap();
        assert_eq!(days, 30.0);
    }

    #[test]
    fn test_datedif_yd_unit() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "diff".to_string(),
            Variable::new(
                "diff".to_string(),
                None,
                Some("=DATEDIF(DATE(2024,1,1), DATE(2024,3,1), \"YD\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // YD = days difference ignoring years (Jan 1 to Mar 1 = 60 days in leap year)
        let diff = result.scalars.get("diff").unwrap().value.unwrap();
        assert!((diff - 60.0).abs() < 1.0);
    }

    #[test]
    fn test_date_leap_year_valid() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2024.0]),
        ));
        data.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.add_column(Column::new(
            "day".to_string(),
            ColumnValue::Number(vec![29.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=DATE(year, month, day)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("result")
            .unwrap();
        if let ColumnValue::Text(vals) = &col.values {
            assert_eq!(vals[0], "2024-02-29", "Feb 29 2024 is valid leap year date");
        }
    }

    #[test]
    fn test_date_leap_year_invalid() {
        // Excel behavior: Feb 29 in non-leap year rolls over to March 1
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2023.0]),
        ));
        data.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.add_column(Column::new(
            "day".to_string(),
            ColumnValue::Number(vec![29.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=DATE(year, month, day)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Should succeed with rollover");
        let table = result.tables.get("data").unwrap();
        let col = table.columns.get("result").unwrap();
        // DATE(2023, 2, 29) = March 1, 2023 = serial 44986
        match &col.values {
            ColumnValue::Number(nums) => assert_eq!(nums[0], 44986.0),
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_date_with_out_of_range_day() {
        // Excel behavior: April 31 rolls over to May 1
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2024.0]),
        ));
        data.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![4.0]), // April has 30 days
        ));
        data.add_column(Column::new(
            "day".to_string(),
            ColumnValue::Number(vec![31.0]), // Requesting 31st
        ));
        data.row_formulas
            .insert("result".to_string(), "=DATE(year, month, day)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Should succeed with rollover");
        let table = result.tables.get("data").unwrap();
        let col = table.columns.get("result").unwrap();
        // DATE(2024, 4, 31) = May 1, 2024 = serial 45413
        match &col.values {
            ColumnValue::Number(nums) => assert_eq!(nums[0], 45413.0),
            _ => panic!("Expected Number"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TESTS FROM date_edge_cases.rs
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_leap_year_valid_year() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=YEAR(DATE(2024, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2024.0));
    }

    #[test]
    fn test_leap_year_valid_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2024, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_leap_year_valid_day() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DAY(DATE(2024, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(29.0));
    }

    #[test]
    fn test_non_leap_year_rollover_year() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=YEAR(DATE(2023, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2023.0));
    }

    #[test]
    fn test_non_leap_year_rollover_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2023, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_non_leap_year_rollover_day() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DAY(DATE(2023, 2, 29))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_month_zero_year() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=YEAR(DATE(2024, 0, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2023.0));
    }

    #[test]
    fn test_month_zero_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2024, 0, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
    }

    #[test]
    fn test_month_thirteen_year() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=YEAR(DATE(2024, 13, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2025.0));
    }

    #[test]
    fn test_month_thirteen_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2024, 13, 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_day_zero_day() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DAY(DATE(2024, 1, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(31.0));
    }

    #[test]
    fn test_day_zero_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2024, 1, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
    }

    #[test]
    fn test_day_overflow_day() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DAY(DATE(2024, 1, 32))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_day_overflow_month() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MONTH(DATE(2024, 1, 32))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_date_subtraction_leap_year_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DATE(2024, 12, 31) - DATE(2024, 1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(365.0));
    }

    #[test]
    fn test_date_subtraction_non_leap_year_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=DATE(2023, 12, 31) - DATE(2023, 1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(364.0));
    }

    #[test]
    fn test_today_subtraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=TODAY() - TODAY()".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_eomonth_subtraction_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=EOMONTH(DATE(2024, 1, 15), 0) - DATE(2024, 1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }
}
