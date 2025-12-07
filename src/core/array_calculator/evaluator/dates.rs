//! Date functions: TODAY, YEAR, MONTH, DAY, DATE, EDATE, EOMONTH, DATEDIF, etc.

use super::{
    evaluate, parse_date_value, require_args, require_args_range, EvalContext, EvalError, Expr,
    Value,
};
use chrono::Datelike;

/// Try to evaluate a date function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "TODAY" => {
            use chrono::Local;
            let today = Local::now().date_naive();
            Value::Text(today.format("%Y-%m-%d").to_string())
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

    #[test]
    fn test_edate() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EDATE(\"2024-01-15\", 3)", &ctx).unwrap(),
            Value::Text("2024-04-15".to_string())
        );
    }

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
}
