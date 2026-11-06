//! Excel day-count conventions (`basis` 0 to 4), shared by `YEARFRAC` and the
//! security functions (`PRICEDISC`, `YIELDDISC`, `ACCRINT`).
//!
//! - 0: US (NASD) 30/360 — the default
//! - 1: actual/actual
//! - 2: actual/360
//! - 3: actual/365
//! - 4: European 30/360

use super::EvalError;
use chrono::{Datelike, Months, NaiveDate};

/// A validated day-count basis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Us30360,
    ActualActual,
    Actual360,
    Actual365,
    European30360,
}

impl Basis {
    /// Read Excel's `basis` argument (truncated to an integer, as Excel does)
    ///
    /// # Errors
    ///
    /// Returns #NUM! for a basis outside 0 to 4.
    pub fn from_number(basis: f64, function: &str) -> Result<Self, EvalError> {
        // Excel truncates the basis; values are tiny integers
        #[allow(clippy::cast_possible_truncation)]
        match basis.trunc() as i64 {
            0 => Ok(Self::Us30360),
            1 => Ok(Self::ActualActual),
            2 => Ok(Self::Actual360),
            3 => Ok(Self::Actual365),
            4 => Ok(Self::European30360),
            _ => Err(EvalError::new(format!(
                "#NUM!: {function} basis must be 0, 1, 2, 3 or 4, got {basis}"
            ))),
        }
    }
}

const fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn is_last_day_of_february(date: NaiveDate) -> bool {
    date.month() == 2 && date.succ_opt().is_some_and(|next| next.month() == 3)
}

fn actual_days(start: NaiveDate, end: NaiveDate) -> f64 {
    // Day counts between real dates fit easily in f64
    #[allow(clippy::cast_precision_loss)]
    let days = (end - start).num_days() as f64;
    days
}

/// Days between two dates in 30/360, the way Excel's `YEARFRAC` counts them
fn days_30_360(start: NaiveDate, end: NaiveDate, european: bool) -> f64 {
    let (y1, m1, mut d1) = (
        start.year(),
        start.month().cast_signed(),
        start.day().cast_signed(),
    );
    let (y2, m2, mut d2) = (
        end.year(),
        end.month().cast_signed(),
        end.day().cast_signed(),
    );
    if european {
        d1 = d1.min(30);
        d2 = d2.min(30);
    } else {
        // US (NASD) rules, including the end-of-February rules
        let start_feb_end = is_last_day_of_february(start);
        if start_feb_end && is_last_day_of_february(end) {
            d2 = 30;
        }
        if start_feb_end {
            d1 = 30;
        }
        if d2 == 31 && d1 >= 30 {
            d2 = 30;
        }
        if d1 == 31 {
            d1 = 30;
        }
    }
    f64::from((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1))
}

/// Days from `start` to `end` under `basis` (negative when `end` is earlier)
pub fn days_between(start: NaiveDate, end: NaiveDate, basis: Basis) -> f64 {
    match basis {
        Basis::Us30360 => days_30_360(start, end, false),
        Basis::European30360 => days_30_360(start, end, true),
        Basis::ActualActual | Basis::Actual360 | Basis::Actual365 => actual_days(start, end),
    }
}

/// Days in the year that `basis` divides by, for a security settling on `date`
pub fn year_length(date: NaiveDate, basis: Basis) -> f64 {
    match basis {
        Basis::Us30360 | Basis::European30360 | Basis::Actual360 => 360.0,
        Basis::Actual365 => 365.0,
        Basis::ActualActual => {
            if is_leap(date.year()) {
                366.0
            } else {
                365.0
            }
        },
    }
}

/// Excel's `YEARFRAC`: the fraction of a year between two dates, in either order
pub fn year_fraction(a: NaiveDate, b: NaiveDate, basis: Basis) -> f64 {
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    match basis {
        Basis::Us30360 | Basis::European30360 => days_between(start, end, basis) / 360.0,
        Basis::Actual360 => actual_days(start, end) / 360.0,
        Basis::Actual365 => actual_days(start, end) / 365.0,
        Basis::ActualActual => actual_actual_fraction(start, end),
    }
}

/// Actual/actual as Excel computes it: within one year, divide by 365 or 366;
/// over more than a year, divide by the average length of the years spanned.
fn actual_actual_fraction(start: NaiveDate, end: NaiveDate) -> f64 {
    let days = actual_days(start, end);
    if days == 0.0 {
        return 0.0;
    }
    let (y1, y2) = (start.year(), end.year());
    let within_a_year = y1 == y2
        || (y2 == y1 + 1
            && (start.month() > end.month()
                || (start.month() == end.month() && start.day() >= end.day())));
    if within_a_year {
        let year_days = if y1 == y2 {
            if is_leap(y1) {
                366.0
            } else {
                365.0
            }
        } else if includes_february_29(start, end) {
            366.0
        } else {
            365.0
        };
        days / year_days
    } else {
        let years = f64::from(y2 - y1 + 1);
        let total: f64 = (y1..=y2)
            .map(|y| if is_leap(y) { 366.0 } else { 365.0 })
            .sum();
        days / (total / years)
    }
}

fn includes_february_29(start: NaiveDate, end: NaiveDate) -> bool {
    (start.year()..=end.year()).any(|y| {
        NaiveDate::from_ymd_opt(y, 2, 29).is_some_and(|feb29| start <= feb29 && feb29 <= end)
    })
}

/// Coupon date `months` months after `anchor` (negative = before), keeping month-end dates at month end
fn shift_months(anchor: NaiveDate, months: i32, month_end: bool) -> Option<NaiveDate> {
    let shifted = if months >= 0 {
        anchor.checked_add_months(Months::new(months.unsigned_abs()))?
    } else {
        anchor.checked_sub_months(Months::new(months.unsigned_abs()))?
    };
    if month_end {
        shifted
            .with_day(1)?
            .checked_add_months(Months::new(1))?
            .pred_opt()
    } else {
        Some(shifted)
    }
}

/// Excel's `ACCRINT` with `calc_method` TRUE: interest accrued from `issue` to
/// `settlement`, summed over coupon periods anchored on `first_interest`.
///
/// # Errors
///
/// Returns an error if the coupon schedule cannot be built.
pub fn accrued_interest(
    issue: NaiveDate,
    first_interest: NaiveDate,
    settlement: NaiveDate,
    rate: f64,
    par: f64,
    frequency: u32,
    basis: Basis,
) -> Result<f64, EvalError> {
    // frequency is 1, 2 or 4, so the step is 12, 6 or 3 months
    #[allow(clippy::cast_possible_wrap)]
    let step = (12 / frequency) as i32;
    let month_end = is_month_end(first_interest);
    let coupon = |k: i32| {
        shift_months(first_interest, k * step, month_end)
            .ok_or_else(|| EvalError::new("ACCRINT: cannot build the coupon schedule"))
    };

    // Start from the coupon period that contains the issue date
    let mut k = 0;
    while coupon(k)? > issue {
        k -= 1;
    }

    let mut total = 0.0;
    loop {
        let (start, end) = (coupon(k)?, coupon(k + 1)?);
        if start >= settlement {
            break;
        }
        let (from, to) = (start.max(issue), end.min(settlement));
        if from < to {
            let normal_length = match basis {
                Basis::ActualActual => actual_days(start, end),
                _ => year_length(start, basis) / f64::from(frequency),
            };
            total += days_between(from, to, basis) / normal_length;
        }
        k += 1;
    }
    Ok(par * rate / f64::from(frequency) * total)
}

fn is_month_end(date: NaiveDate) -> bool {
    date.succ_opt().is_some_and(|next| next.day() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn basis_is_read_and_checked() {
        assert_eq!(Basis::from_number(0.0, "F").unwrap(), Basis::Us30360);
        assert_eq!(Basis::from_number(1.9, "F").unwrap(), Basis::ActualActual);
        assert_eq!(Basis::from_number(2.0, "F").unwrap(), Basis::Actual360);
        assert_eq!(Basis::from_number(3.0, "F").unwrap(), Basis::Actual365);
        assert_eq!(Basis::from_number(4.0, "F").unwrap(), Basis::European30360);
        assert!(Basis::from_number(5.0, "F").is_err());
        assert!(Basis::from_number(-1.0, "F").is_err());
    }

    /// Values checked in Excel on 2026-11-06
    #[test]
    fn year_fraction_matches_excel() {
        // 2000-03-23 to 2029-02-28, actual/actual over many years
        assert!(close(
            year_fraction(date(2000, 3, 23), date(2029, 2, 28), Basis::ActualActual),
            28.935_024_639_532_76
        ));
        // 2002-02-28 to 2018-07-17, US 30/360 starting at the end of February
        assert!(close(
            year_fraction(date(2002, 2, 28), date(2018, 7, 17), Basis::Us30360),
            16.380_555_555_555_56
        ));
        // Order does not matter
        assert!(close(
            year_fraction(date(2018, 7, 17), date(2002, 2, 28), Basis::Us30360),
            16.380_555_555_555_56
        ));
    }

    #[test]
    fn thirty_360_rules() {
        // US: both dates at the end of February count as the 30th
        assert!(close(
            days_between(date(2023, 2, 28), date(2024, 2, 29), Basis::Us30360),
            360.0
        ));
        // US: the 31st becomes the 30th only when the start is the 30th or 31st
        assert!(close(
            days_between(date(2024, 1, 15), date(2024, 3, 31), Basis::Us30360),
            76.0
        ));
        assert!(close(
            days_between(date(2024, 1, 30), date(2024, 3, 31), Basis::Us30360),
            60.0
        ));
        // European: every 31st is the 30th
        assert!(close(
            days_between(date(2024, 1, 15), date(2024, 3, 31), Basis::European30360),
            75.0
        ));
    }

    #[test]
    fn actual_actual_within_a_year() {
        // Same leap year: 366
        assert!(close(
            year_fraction(date(2024, 1, 1), date(2024, 7, 1), Basis::ActualActual),
            182.0 / 366.0
        ));
        // Crossing years without a February 29: 365
        assert!(close(
            year_fraction(date(2000, 12, 25), date(2001, 2, 28), Basis::ActualActual),
            65.0 / 365.0
        ));
        // Crossing years with a February 29: 366
        assert!(close(
            year_fraction(date(2023, 12, 1), date(2024, 3, 1), Basis::ActualActual),
            91.0 / 366.0
        ));
        assert!(close(
            year_fraction(date(2024, 1, 1), date(2024, 1, 1), Basis::ActualActual),
            0.0
        ));
        assert!(close(
            year_fraction(date(2023, 1, 1), date(2023, 7, 1), Basis::Actual360),
            181.0 / 360.0
        ));
        assert!(close(
            year_fraction(date(2023, 1, 1), date(2023, 7, 1), Basis::Actual365),
            181.0 / 365.0
        ));
    }

    #[test]
    fn year_length_by_basis() {
        assert!(close(
            year_length(date(2024, 5, 1), Basis::ActualActual),
            366.0
        ));
        assert!(close(
            year_length(date(2023, 5, 1), Basis::ActualActual),
            365.0
        ));
        assert!(close(
            year_length(date(2023, 5, 1), Basis::Actual365),
            365.0
        ));
        assert!(close(year_length(date(2023, 5, 1), Basis::Us30360), 360.0));
    }

    /// Values checked in Excel on 2026-11-06
    #[test]
    fn accrued_interest_matches_excel() {
        let basis0 = accrued_interest(
            date(1995, 8, 27),
            date(1996, 2, 20),
            date(2007, 8, 31),
            0.06,
            1000.0,
            1,
            Basis::Us30360,
        )
        .unwrap();
        assert!(close(basis0, 720.666_666_666_666_6), "{basis0}");
        let basis1 = accrued_interest(
            date(2021, 4, 10),
            date(2021, 12, 18),
            date(2032, 5, 3),
            0.06,
            1000.0,
            2,
            Basis::ActualActual,
        )
        .unwrap();
        assert!(close(basis1, 663.832_642_767_068_9), "{basis1}");
    }

    #[test]
    fn accrued_interest_with_a_month_end_coupon() {
        // Quarterly coupons on month ends, issue before the first coupon
        let value = accrued_interest(
            date(2024, 1, 15),
            date(2024, 3, 31),
            date(2024, 2, 29),
            0.10,
            5000.0,
            4,
            Basis::Us30360,
        )
        .unwrap();
        assert!(close(value, 5000.0 * 0.10 * 44.0 / 360.0), "{value}");
    }
}
