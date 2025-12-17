//! Date functions: TODAY, NOW, YEAR, MONTH, DAY, WEEKDAY, HOUR, MINUTE, SECOND, DATE, EDATE, EOMONTH, DATEDIF, DAYS, TIME, WORKDAY, etc.
//!
//! DEMO functions (6): TODAY, DATE, YEAR, MONTH, DAY, DATEDIF
//! ENTERPRISE functions: NOW, WEEKDAY, HOUR, MINUTE, SECOND, TIME, DAYS, WORKDAY, EDATE, EOMONTH, NETWORKDAYS, YEARFRAC

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
        }

        #[cfg(not(feature = "demo"))]
        "NOW" => {
            use chrono::Local;
            let now = Local::now();
            Value::Text(now.format("%Y-%m-%d %H:%M:%S").to_string())
        }

        "YEAR" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.year() as f64)
        }

        "MONTH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.month() as f64)
        }

        "DAY" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(date.day() as f64)
        }

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
        }

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
        }

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
        }

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
        }

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
        }

        #[cfg(not(feature = "demo"))]
        "DAYS" => {
            require_args(name, args, 2)?;
            let end = evaluate(&args[0], ctx)?;
            let start = evaluate(&args[1], ctx)?;

            let end_date = parse_date_value(&end)?;
            let start_date = parse_date_value(&start)?;

            Value::Number((end_date - start_date).num_days() as f64)
        }

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
        }

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
                as u32;

            // Handle month overflow/underflow (Excel-compatible behavior)
            let total_months = (year * 12 + month - 1) as i32;
            let adj_year = total_months.div_euclid(12);
            let adj_month = (total_months.rem_euclid(12) + 1) as u32;

            use chrono::NaiveDate;
            let date = NaiveDate::from_ymd_opt(adj_year, adj_month, day).ok_or_else(|| {
                EvalError::new(format!(
                    "DATE: invalid date {}-{}-{}",
                    adj_year, adj_month, day
                ))
            })?;
            Value::Text(date.format("%Y-%m-%d").to_string())
        }

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
        }

        #[cfg(not(feature = "demo"))]
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
        }

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
                    let years = end_date.year() - start_date.year();
                    let months = end_date.month() as i32 - start_date.month() as i32;
                    (years * 12 + months) as f64
                }
                "Y" => (end_date.year() - start_date.year()) as f64,
                "MD" => {
                    let mut day_diff = end_date.day() as i32 - start_date.day() as i32;
                    if day_diff < 0 {
                        day_diff += 30;
                    }
                    day_diff as f64
                }
                "YM" => {
                    let mut month_diff = end_date.month() as i32 - start_date.month() as i32;
                    if month_diff < 0 {
                        month_diff += 12;
                    }
                    month_diff as f64
                }
                "YD" => {
                    let start_doy = start_date.ordinal() as i32;
                    let end_doy = end_date.ordinal() as i32;
                    let mut day_diff = end_doy - start_doy;
                    if day_diff < 0 {
                        day_diff += 365;
                    }
                    day_diff as f64
                }
                _ => return Err(EvalError::new(format!("DATEDIF: unknown unit '{}'", unit))),
            };
            Value::Number(result)
        }

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
        }

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
                }
                1 => {
                    // Actual/actual
                    let days = (end_date - start_date).num_days() as f64;
                    let year = start_date.year();
                    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                    let year_days = if is_leap { 366.0 } else { 365.0 };
                    days / year_days
                }
                2 => {
                    // Actual/360
                    let days = (end_date - start_date).num_days() as f64;
                    days / 360.0
                }
                3 => {
                    // Actual/365
                    let days = (end_date - start_date).num_days() as f64;
                    days / 365.0
                }
                _ => return Err(EvalError::new(format!("YEARFRAC: unknown basis {}", basis))),
            };
            Value::Number(result)
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

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
        assert_eq!(
            eval("DATE(2024, 6, 15)", &ctx).unwrap(),
            Value::Text("2024-06-15".to_string())
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
        // MD where end day < start day (triggers day_diff += 30)
        assert_eq!(
            eval("DATEDIF(\"2024-01-20\", \"2024-02-10\", \"MD\")", &ctx).unwrap(),
            Value::Number(20.0) // -10 + 30 = 20
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
        // Month 14 = February of next year
        assert_eq!(
            eval("DATE(2024, 14, 15)", &ctx).unwrap(),
            Value::Text("2025-02-15".to_string())
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
}
