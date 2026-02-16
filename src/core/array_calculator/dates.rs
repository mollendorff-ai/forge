//! Date Functions (v1.1.0)
//! TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH, NETWORKDAYS, WORKDAY, YEARFRAC
//!
//! Legacy date utility methods used internally. The public date function API
//! is provided by `evaluator::dates`.

#![allow(dead_code)] // Legacy date utilities kept for potential future use

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue};

use super::ArrayCalculator;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unused_self,
    clippy::unnecessary_wraps
)]
// Date serial numbers and day/month/year conversions involve bounded integer casts
// where values are constrained by calendar rules (months 1-12, days 1-31, years < 9999).
// Methods take &self for API consistency within the ArrayCalculator impl block.
// Some methods return Result for API uniformity with failable date operations.
impl ArrayCalculator {
    /// Evaluate TODAY function: `TODAY()`
    pub(super) fn eval_today(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Convert Unix timestamp to date (simplified, no timezone handling)
        let days_since_epoch = now / 86400;
        let (year, month, day) = Self::days_to_date(days_since_epoch as i32);

        format!("{year:04}-{month:02}-{day:02}")
    }

    /// Evaluate DATE function: DATE(year, month, day)
    pub(super) fn eval_date(&self, year: i32, month: i32, day: i32) -> ForgeResult<String> {
        Ok(format!("{year:04}-{month:02}-{day:02}"))
    }

    /// Evaluate YEAR function: YEAR(date)
    pub(super) fn eval_year(&self, date: &str) -> ForgeResult<f64> {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return Err(ForgeError::Eval(format!(
                "YEAR: Invalid date format '{date}'"
            )));
        }
        let year = parts[0]
            .parse::<f64>()
            .map_err(|_| ForgeError::Eval(format!("YEAR: Invalid year in '{date}'")))?;
        Ok(year)
    }

    /// Evaluate MONTH function: MONTH(date)
    pub(super) fn eval_month(&self, date: &str) -> ForgeResult<f64> {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return Err(ForgeError::Eval(format!(
                "MONTH: Invalid date format '{date}'"
            )));
        }
        let month = parts[1]
            .parse::<f64>()
            .map_err(|_| ForgeError::Eval(format!("MONTH: Invalid month in '{date}'")))?;
        Ok(month)
    }

    /// Evaluate DAY function: DAY(date)
    pub(super) fn eval_day(&self, date: &str) -> ForgeResult<f64> {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return Err(ForgeError::Eval(format!(
                "DAY: Invalid date format '{date}'"
            )));
        }
        let day = parts[2]
            .parse::<f64>()
            .map_err(|_| ForgeError::Eval(format!("DAY: Invalid day in '{date}'")))?;
        Ok(day)
    }

    /// Evaluate DATEDIF function: `DATEDIF(start_date`, `end_date`, unit)
    /// unit: "Y" for years, "M" for months, "D" for days
    // Multiple unit cases require sequential match arms.
    #[allow(clippy::too_many_lines)]
    pub(super) fn eval_datedif(
        &self,
        start_date: &str,
        end_date: &str,
        unit: &str,
    ) -> ForgeResult<f64> {
        let start = start_date.trim().trim_matches('"');
        let end = end_date.trim().trim_matches('"');

        // Parse start date
        let start_parts: Vec<&str> = start.split('-').collect();
        let (start_year, start_month, start_day) = if start_parts.len() >= 2 {
            let y = start_parts[0].parse::<i32>().map_err(|_| {
                ForgeError::Eval(format!("DATEDIF: Invalid start year in '{start}'"))
            })?;
            let m = start_parts[1].parse::<i32>().map_err(|_| {
                ForgeError::Eval(format!("DATEDIF: Invalid start month in '{start}'"))
            })?;
            let d = if start_parts.len() == 3 {
                start_parts[2].parse::<i32>().map_err(|_| {
                    ForgeError::Eval(format!("DATEDIF: Invalid start day in '{start}'"))
                })?
            } else {
                1
            };
            (y, m, d)
        } else {
            return Err(ForgeError::Eval(format!(
                "DATEDIF: Invalid start date format '{start}'"
            )));
        };

        // Parse end date
        let end_parts: Vec<&str> = end.split('-').collect();
        let (end_year, end_month, end_day) = if end_parts.len() >= 2 {
            let y = end_parts[0]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("DATEDIF: Invalid end year in '{end}'")))?;
            let m = end_parts[1]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("DATEDIF: Invalid end month in '{end}'")))?;
            let d = if end_parts.len() == 3 {
                end_parts[2]
                    .parse::<i32>()
                    .map_err(|_| ForgeError::Eval(format!("DATEDIF: Invalid end day in '{end}'")))?
            } else {
                1
            };
            (y, m, d)
        } else {
            return Err(ForgeError::Eval(format!(
                "DATEDIF: Invalid end date format '{end}'"
            )));
        };

        match unit {
            "Y" => {
                let mut years = end_year - start_year;
                if end_month < start_month || (end_month == start_month && end_day < start_day) {
                    years -= 1;
                }
                Ok(f64::from(years.max(0)))
            },
            "M" => {
                let mut months = (end_year - start_year) * 12 + (end_month - start_month);
                if end_day < start_day {
                    months -= 1;
                }
                Ok(f64::from(months.max(0)))
            },
            "D" => {
                let start_serial =
                    self.date_to_excel_serial(start_year, start_month as u32, start_day as u32)?;
                let end_serial =
                    self.date_to_excel_serial(end_year, end_month as u32, end_day as u32)?;
                Ok((end_serial - start_serial).max(0.0))
            },
            "MD" => {
                // Days ignoring months and years
                let mut day_diff = end_day - start_day;
                if day_diff < 0 {
                    // Get days in previous month
                    let prev_month = if end_month == 1 { 12 } else { end_month - 1 };
                    let prev_year = if end_month == 1 {
                        end_year - 1
                    } else {
                        end_year
                    };
                    day_diff += self.days_in_month(prev_year, prev_month as u32) as i32;
                }
                Ok(f64::from(day_diff))
            },
            "YM" => {
                // Months ignoring years
                let mut month_diff = end_month - start_month;
                if end_day < start_day {
                    month_diff -= 1;
                }
                if month_diff < 0 {
                    month_diff += 12;
                }
                Ok(f64::from(month_diff))
            },
            "YD" => {
                // Days ignoring years
                let start_serial = Self::ymd_to_ordinal(2000, start_month, start_day);
                let mut end_serial = Self::ymd_to_ordinal(2000, end_month, end_day);
                if end_serial < start_serial {
                    end_serial += 365; // approximate
                }
                Ok(f64::from(end_serial - start_serial))
            },
            _ => Err(ForgeError::Eval(format!(
                "DATEDIF: Invalid unit '{unit}' (use Y, M, D, MD, YM, or YD)"
            ))),
        }
    }

    /// Evaluate EDATE function: `EDATE(start_date`, months)
    pub(super) fn eval_edate(&self, start_date: &str, months: i32) -> ForgeResult<String> {
        let start = start_date.trim().trim_matches('"');

        let parts: Vec<&str> = start.split('-').collect();
        let (year, month, day) = if parts.len() >= 2 {
            let y = parts[0]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("EDATE: Invalid year in '{start}'")))?;
            let m = parts[1]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("EDATE: Invalid month in '{start}'")))?;
            let d = if parts.len() == 3 {
                parts[2]
                    .parse::<i32>()
                    .map_err(|_| ForgeError::Eval(format!("EDATE: Invalid day in '{start}'")))?
            } else {
                1
            };
            (y, m, d)
        } else {
            return Err(ForgeError::Eval(format!(
                "EDATE: Invalid date format '{start}'"
            )));
        };

        let total_months = (year * 12 + (month - 1)) + months;
        let new_year = total_months / 12;
        let new_month = (total_months % 12) + 1;
        let new_month = if new_month <= 0 {
            new_month + 12
        } else {
            new_month
        };
        let new_year = if total_months < 0 && new_month > 0 {
            new_year - 1
        } else {
            new_year
        };

        let days_in_new_month = self.days_in_month(new_year, new_month as u32);
        let new_day = day.min(days_in_new_month as i32);

        Ok(format!("{new_year:04}-{new_month:02}-{new_day:02}"))
    }

    /// Evaluate EOMONTH function: `EOMONTH(start_date`, months)
    pub(super) fn eval_eomonth(&self, start_date: &str, months: i32) -> ForgeResult<String> {
        let start = start_date.trim().trim_matches('"');

        let parts: Vec<&str> = start.split('-').collect();
        let (year, month) = if parts.len() >= 2 {
            let y = parts[0]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("EOMONTH: Invalid year in '{start}'")))?;
            let m = parts[1]
                .parse::<i32>()
                .map_err(|_| ForgeError::Eval(format!("EOMONTH: Invalid month in '{start}'")))?;
            (y, m)
        } else {
            return Err(ForgeError::Eval(format!(
                "EOMONTH: Invalid date format '{start}'"
            )));
        };

        let total_months = (year * 12 + (month - 1)) + months;
        let new_year = total_months / 12;
        let new_month = (total_months % 12) + 1;
        let new_month = if new_month <= 0 {
            new_month + 12
        } else {
            new_month
        };
        let new_year = if total_months < 0 && new_month > 0 {
            new_year - 1
        } else {
            new_year
        };

        let last_day = self.days_in_month(new_year, new_month as u32);

        Ok(format!("{new_year:04}-{new_month:02}-{last_day:02}"))
    }

    /// Get the number of days in a given month
    pub(super) const fn days_in_month(&self, year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            },
            _ => 30, // Months 4, 6, 9, 11 and any invalid month
        }
    }

    /// Convert days since epoch to (year, month, day)
    pub(super) fn days_to_date(days: i32) -> (i32, i32, i32) {
        let mut year = 1970;
        let mut remaining_days = days;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        let days_in_months = if Self::is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        for &days_in_month in &days_in_months {
            if remaining_days < days_in_month {
                break;
            }
            remaining_days -= days_in_month;
            month += 1;
        }

        let day = remaining_days + 1;
        (year, month, day)
    }

    /// Check if a year is a leap year
    pub(super) const fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    /// Parse a date string into (year, month, day)
    pub(super) fn parse_date_ymd(date_str: &str) -> ForgeResult<(i32, u32, u32)> {
        let s = date_str.trim().trim_matches('"');
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(ForgeError::Eval(format!("Invalid date format: {s}")));
        }
        let year = parts[0]
            .parse::<i32>()
            .map_err(|_| ForgeError::Eval(format!("Invalid year in date: {s}")))?;
        let month = parts[1]
            .parse::<u32>()
            .map_err(|_| ForgeError::Eval(format!("Invalid month in date: {s}")))?;
        let day = parts[2]
            .parse::<u32>()
            .map_err(|_| ForgeError::Eval(format!("Invalid day in date: {s}")))?;
        Ok((year, month, day))
    }

    /// Convert (year, month, day) to day count for workday calculations
    pub(super) fn ymd_to_ordinal(year: i32, month: i32, day: i32) -> i32 {
        let mut days = 0i32;
        let base_year = 2000;

        for y in base_year..year {
            days += if Self::is_leap_year(y) { 366 } else { 365 };
        }
        for y in year..base_year {
            days -= if Self::is_leap_year(y) { 366 } else { 365 };
        }

        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for m in 1..month {
            days += days_in_month[(m - 1) as usize];
            if m == 2 && Self::is_leap_year(year) {
                days += 1;
            }
        }

        days += day;
        days
    }

    /// Convert ordinal back to (year, month, day)
    pub(super) const fn ordinal_to_ymd(mut days: i32) -> (i32, i32, i32) {
        let base_year = 2000;
        let mut year = base_year;

        loop {
            let year_days = if Self::is_leap_year(year) { 366 } else { 365 };
            if days <= year_days {
                break;
            }
            days -= year_days;
            year += 1;
        }

        let mut month = 1i32;
        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        while month <= 12 {
            let mut m_days = days_in_month[(month - 1) as usize];
            if month == 2 && Self::is_leap_year(year) {
                m_days += 1;
            }
            if days <= m_days {
                break;
            }
            days -= m_days;
            month += 1;
        }

        (year, month, days)
    }

    /// Get day of week (0=Monday, 6=Sunday) for a date
    #[allow(clippy::many_single_char_names)] // Zeller's congruence standard notation
    pub(super) const fn weekday(year: i32, month: i32, day: i32) -> i32 {
        let m = if month < 3 { month + 12 } else { month };
        let y = if month < 3 { year - 1 } else { year };
        let q = day;
        let k = y % 100;
        let j = y / 100;
        let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
        (h + 5) % 7
    }

    /// Calculate number of working days between two dates (excludes weekends)
    pub(super) fn eval_networkdays(&self, start: &str, end: &str) -> ForgeResult<i32> {
        let (start_y, start_m, start_d) = Self::parse_date_ymd(start)?;
        let (end_y, end_m, end_d) = Self::parse_date_ymd(end)?;

        let start_days = Self::ymd_to_ordinal(start_y, start_m as i32, start_d as i32);
        let end_days = Self::ymd_to_ordinal(end_y, end_m as i32, end_d as i32);

        let direction = if end_days >= start_days { 1 } else { -1 };
        let (from, to) = if direction > 0 {
            (start_days, end_days)
        } else {
            (end_days, start_days)
        };

        let mut count = 0;
        for d in from..=to {
            let (y, m, day) = Self::ordinal_to_ymd(d);
            let dow = Self::weekday(y, m, day);
            if dow < 5 {
                count += 1;
            }
        }

        Ok(count * direction)
    }

    /// Calculate date after N working days
    pub(super) fn eval_workday(&self, start: &str, days: i32) -> ForgeResult<String> {
        let (start_y, start_m, start_d) = Self::parse_date_ymd(start)?;
        let mut current_days = Self::ymd_to_ordinal(start_y, start_m as i32, start_d as i32);

        let mut remaining = days.abs();
        let direction = if days >= 0 { 1 } else { -1 };

        while remaining > 0 {
            current_days += direction;
            let (y, m, d) = Self::ordinal_to_ymd(current_days);
            let dow = Self::weekday(y, m, d);
            if dow < 5 {
                remaining -= 1;
            }
        }

        let (y, m, d) = Self::ordinal_to_ymd(current_days);
        Ok(format!("{y:04}-{m:02}-{d:02}"))
    }

    /// Calculate fraction of year between two dates
    pub(super) fn eval_yearfrac(&self, start: &str, end: &str, basis: i32) -> ForgeResult<f64> {
        let (start_year, start_month, start_day_raw) = Self::parse_date_ymd(start)?;
        let (end_year, end_month, end_day_raw) = Self::parse_date_ymd(end)?;

        let start_days = Self::ymd_to_ordinal(start_year, start_month as i32, start_day_raw as i32);
        let end_days = Self::ymd_to_ordinal(end_year, end_month as i32, end_day_raw as i32);
        let days = f64::from(end_days - start_days);

        match basis {
            0 | 4 => {
                let mut sd = start_day_raw.min(31);
                let mut ed = end_day_raw.min(31);

                if sd == 31 {
                    sd = 30;
                }
                if basis == 0 {
                    if ed == 31 && sd == 30 {
                        ed = 30;
                    }
                } else if ed == 31 {
                    ed = 30;
                }

                let diff = f64::from(end_year - start_year)
                    .mul_add(360.0, f64::from(end_month - start_month) * 30.0)
                    + f64::from(ed)
                    - f64::from(sd);
                Ok(diff / 360.0)
            },
            1 => {
                if start_year == end_year {
                    let days_in_year = if Self::is_leap_year(start_year) {
                        366.0
                    } else {
                        365.0
                    };
                    Ok(days / days_in_year)
                } else {
                    let avg_days_per_year = (start_year..=end_year).fold(0.0, |acc, y| {
                        acc + if Self::is_leap_year(y) { 366.0 } else { 365.0 }
                    }) / f64::from(end_year - start_year + 1);
                    Ok(days / avg_days_per_year)
                }
            },
            2 => Ok(days / 360.0),
            3 => Ok(days / 365.0),
            _ => Err(ForgeError::Eval(format!(
                "YEARFRAC: Invalid basis {basis}. Must be 0-4"
            ))),
        }
    }

    /// Convert date to Excel serial number
    pub(super) fn date_to_excel_serial(&self, year: i32, month: u32, day: u32) -> ForgeResult<f64> {
        // Excel serial: 1 = 1900-01-01
        // But Excel incorrectly treats 1900 as a leap year, so we need to adjust
        let base_year = 1900;
        let mut serial = 0i32;

        // Add days for complete years
        for y in base_year..year {
            serial += if Self::is_leap_year(y) { 366 } else { 365 };
        }

        // Add days for complete months
        for m in 1..month {
            serial += self.days_in_month(year, m) as i32;
        }

        // Add days
        serial += day as i32;

        // Excel's 1900 leap year bug: dates after Feb 28, 1900 are off by 1
        if serial > 59 {
            serial += 1;
        }

        Ok(f64::from(serial))
    }

    /// Convert date string to Excel serial number
    pub(super) fn date_string_to_serial(&self, date_str: &str) -> ForgeResult<f64> {
        let s = date_str.trim().trim_matches('"');

        // Try parsing as a number first (already a serial)
        if let Ok(n) = s.parse::<f64>() {
            return Ok(n);
        }

        // Parse as date string
        let (year, month, day) = Self::parse_date_ymd(s)?;
        self.date_to_excel_serial(year, month, day)
    }

    /// Convert a Column to a Vec<f64> of Excel serial dates
    pub(super) fn column_to_date_serial_vec(&self, col: &Column) -> ForgeResult<Vec<f64>> {
        match &col.values {
            ColumnValue::Date(dates) => {
                let mut serials = Vec::with_capacity(dates.len());
                for d in dates {
                    serials.push(self.date_string_to_serial(d)?);
                }
                Ok(serials)
            },
            ColumnValue::Number(nums) => {
                // Assume numbers are already serial dates
                Ok(nums.clone())
            },
            ColumnValue::Text(texts) => {
                // Try to parse as date strings
                let mut serials = Vec::with_capacity(texts.len());
                for t in texts {
                    serials.push(self.date_string_to_serial(t)?);
                }
                Ok(serials)
            },
            ColumnValue::Boolean(_) => Err(ForgeError::Eval(format!(
                "Cannot use boolean column '{}' as dates",
                col.name
            ))),
        }
    }
}
