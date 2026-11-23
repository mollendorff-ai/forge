//! What each function gives and reads: a date, a number or a text, and
//! which of its arguments it passes on. The walk in rewrite.rs asks these
//! tables where a date must become a day number or a day number a date.

/// What a part of a formula is in Forge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A date: date text in Forge, a day number in the workbook
    Date,
    /// A date written in the formula (`"2026-01-31"`)
    Written,
    /// Text with a digit in some row: a date (`n/a` among dates), a date
    /// Excel cannot hold, a month (`2026-01`), a number (`"5"`, `"1,200"`).
    /// Forge reads it as a number where a number is read, or fails to, and
    /// Excel reads text there or parses it its own way.
    Mixed,
    Text,
    /// A number or TRUE/FALSE
    Number,
    Unknown,
}

impl Kind {
    pub(super) const fn is_date(self) -> bool {
        matches!(self, Self::Date | Self::Written)
    }

    /// Whether Forge may read a day number from it.
    pub(super) const fn may_be_read_as_a_date(self) -> bool {
        matches!(self, Self::Date | Self::Written | Self::Mixed)
    }
}

/// How the reader of a value reads a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Read {
    Number,
    Text,
}

/// Functions that give a date in Forge (date text) and in Excel (a day number).
pub(super) const GIVE_A_DATE: [&str; 5] = ["DATE", "EDATE", "EOMONTH", "WORKDAY", "TODAY"];

/// Functions that give text.
pub(super) const GIVE_TEXT: [&str; 13] = [
    "CONCAT",
    "CONCATENATE",
    "LEFT",
    "RIGHT",
    "MID",
    "UPPER",
    "LOWER",
    "TRIM",
    "REPT",
    "TEXT",
    "SUBSTITUTE",
    "REPLACE",
    "ADDRESS",
];

/// Functions that read every argument as a number, a date as its day number,
/// in Forge as in Excel. Each is checked on a date column in both engines of
/// forge-e2e-gnumeric (`edge_real_dates.yaml`).
pub(super) const READ_NUMBERS: [&str; 53] = [
    "ACCRINT",
    "PRICEDISC",
    "YIELDDISC",
    "HOUR",
    "MINUTE",
    "SECOND",
    "DATE",
    "ROUND",
    "ROUNDUP",
    "ROUNDDOWN",
    "INT",
    "TRUNC",
    "MOD",
    "FLOOR",
    "CEILING",
    "SIGN",
    "SQRT",
    "POWER",
    "LN",
    "LOG10",
    "ISEVEN",
    "ISODD",
    "PRODUCT",
    "LARGE",
    "SMALL",
    "RANK.EQ",
    "PERCENTILE",
    "QUARTILE",
    "VAR.S",
    "VARP",
    "STDEV.S",
    "STDEVP",
    "XNPV",
    "XIRR",
    "YEAR",
    "MONTH",
    "DAY",
    "WEEKDAY",
    "EDATE",
    "EOMONTH",
    "DAYS",
    "DATEDIF",
    "NETWORKDAYS",
    "WORKDAY",
    "YEARFRAC",
    "SUM",
    "AVERAGE",
    "AVG",
    "MIN",
    "MAX",
    "COUNT",
    "MEDIAN",
    "ABS",
];

/// Functions that count or measure a column whatever it holds.
pub(super) const ANY_VALUES: [&str; 5] = ["COUNTA", "COUNTBLANK", "ROWS", "COLUMNS", "COUNTUNIQUE"];

/// `IF`-like functions: (name, first value argument, step between values).
/// The other arguments are conditions.
pub(super) const PASS_ON: [(&str, usize, usize); 5] = [
    ("IF", 1, 1),
    ("IFS", 1, 2),
    ("CHOOSE", 1, 1),
    ("IFERROR", 0, 1),
    ("IFNA", 0, 1),
];

/// Criteria functions: (name, argument of the first range, whether a range
/// to add up or average comes first or last).
pub(super) const CRITERIA: [(&str, usize); 8] = [
    ("COUNTIF", 0),
    ("SUMIF", 0),
    ("AVERAGEIF", 0),
    ("COUNTIFS", 0),
    ("SUMIFS", 1),
    ("AVERAGEIFS", 1),
    ("MAXIFS", 1),
    ("MINIFS", 1),
];

pub(super) fn join(kinds: impl IntoIterator<Item = Kind>) -> Kind {
    join_all(&kinds.into_iter().collect::<Vec<_>>())
}

/// What a value is when it may be any of `kinds`.
fn join_all(kinds: &[Kind]) -> Kind {
    if kinds.is_empty() {
        Kind::Unknown
    } else if kinds.iter().all(|k| k.is_date()) {
        Kind::Date
    } else if kinds
        .iter()
        .all(|k| matches!(k, Kind::Date | Kind::Written | Kind::Text | Kind::Mixed))
    {
        if kinds.contains(&Kind::Mixed) {
            Kind::Mixed
        } else {
            Kind::Text
        }
    } else if kinds.iter().all(|k| *k == Kind::Number) {
        Kind::Number
    } else {
        Kind::Unknown
    }
}

/// The result arguments of `SWITCH(value, case, result, ..., default)`.
pub(super) fn switch_results(count: usize) -> Vec<usize> {
    let mut results: Vec<usize> = (2..count).step_by(2).collect();
    if count >= 2 && count.is_multiple_of(2) {
        results.push(count - 1);
    }
    results
}

/// The value arguments of an `IF`-like call.
pub(super) fn passed_on(name: &str, count: usize) -> Option<Vec<usize>> {
    let (_, first, step) = PASS_ON.iter().find(|(n, ..)| *n == name)?;
    // IFS(condition, value, ...) has no default, so its values are every
    // second argument; IF(c, a, b) and the others take every argument from
    // the first value on.
    Some((*first..count).step_by(*step).collect())
}
