//! Dates in an exported formula (ADR-054).
//!
//! In Forge a date is the text `2026-01-31`, read as its day number wherever
//! a number is needed. In the workbook a date is a real date: the day number,
//! shown as a date. A formula means the same in both only if each date in it
//! is written the way Forge reads it there:
//!
//! - where Forge reads a number (arithmetic, `<`, `YEAR`, `SUM`...), the cell
//!   itself, and a date written in the formula as `DATE(2026, 1, 31)`;
//! - where Forge reads text (`LEFT`, `&`, `LEN`...), the text Forge holds:
//!   `TEXT(YEAR(c)*10000+MONTH(c)*100+DAY(c), "0000-00-00")`, which does not
//!   depend on the language of Excel as `TEXT(c, "yyyy-mm-dd")` does;
//! - where a value is passed on (`IF`, `INDEX`, `PREV`...), whichever of the
//!   two the reader of the result needs.
//!
//! A column that holds dates as text (`n/a` in some rows, or a date before
//! 1900-03-01) is text in the workbook too. Forge reads a date in it as its
//! day number where a number is read; Excel reads text there, greater than
//! any number and skipped by `MAX`. Such a reading is an error here.
//!
//! A place this module has no rule for is an error, and the caller writes the
//! formula's values: a date read the wrong way would be a wrong answer.

mod print;
mod rewrite;

use super::{Context, WorkbookRefs};
use crate::core::array_calculator::parser::{parse, Expr};
use crate::core::array_calculator::tokenizer::{tokenize, Token};
pub use print::formula_of;
use print::{formula_text, push};
use rewrite::{date_of_numbers, date_of_text, map, upper_names, Dates};

/// What a part of a formula is in Forge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
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
    const fn is_date(self) -> bool {
        matches!(self, Self::Date | Self::Written)
    }

    /// Whether Forge may read a day number from it.
    const fn may_be_read_as_a_date(self) -> bool {
        matches!(self, Self::Date | Self::Written | Self::Mixed)
    }
}

/// What the column of a row formula holds in Forge, once calculated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holds {
    /// Dates: its cells must hold dates in the workbook too
    Dates,
    /// Numbers: a date among its values is its day number
    Numbers,
    /// Text or TRUE/FALSE
    Other,
    /// Not calculated: the column has no values to tell
    Unknown,
}

/// How the reader of a value reads a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Number,
    Text,
}

/// Functions that give a date in Forge (date text) and in Excel (a day number).
const GIVE_A_DATE: [&str; 5] = ["DATE", "EDATE", "EOMONTH", "WORKDAY", "TODAY"];

/// Functions that give text.
const GIVE_TEXT: [&str; 13] = [
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
const READ_NUMBERS: [&str; 53] = [
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
const ANY_VALUES: [&str; 5] = ["COUNTA", "COUNTBLANK", "ROWS", "COLUMNS", "COUNTUNIQUE"];

/// `IF`-like functions: (name, first value argument, step between values).
/// The other arguments are conditions.
const PASS_ON: [(&str, usize, usize); 5] = [
    ("IF", 1, 1),
    ("IFS", 1, 2),
    ("CHOOSE", 1, 1),
    ("IFERROR", 0, 1),
    ("IFNA", 0, 1),
];

/// Criteria functions: (name, argument of the first range, whether a range
/// to add up or average comes first or last).
const CRITERIA: [(&str, usize); 8] = [
    ("COUNTIF", 0),
    ("SUMIF", 0),
    ("AVERAGEIF", 0),
    ("COUNTIFS", 0),
    ("SUMIFS", 1),
    ("AVERAGEIFS", 1),
    ("MAXIFS", 1),
    ("MINIFS", 1),
];

/// A date Excel can hold, written `YYYY-MM-DD`: its year, month and day.
/// Excel's day numbers are right from 1900-03-01 (it counts a 29 February
/// 1900 that never was) to 9999-12-31.
#[must_use]
pub fn excel_date(text: &str) -> Option<(i32, u32, u32)> {
    use chrono::Datelike;
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    if !shaped {
        return None;
    }
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    let first = chrono::NaiveDate::from_ymd_opt(1900, 3, 1)?;
    (date >= first).then_some((date.year(), date.month(), date.day()))
}

/// The day number Excel gives a date written `YYYY-MM-DD`, if it can hold it.
#[must_use]
pub fn excel_serial(text: &str) -> Option<f64> {
    excel_date(text)?;
    crate::core::array_calculator::evaluator::date_serial(text)
}

fn join(kinds: impl IntoIterator<Item = Kind>) -> Kind {
    let kinds: Vec<Kind> = kinds.into_iter().collect();
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
fn switch_results(count: usize) -> Vec<usize> {
    let mut results: Vec<usize> = (2..count).step_by(2).collect();
    if count >= 2 && count.is_multiple_of(2) {
        results.push(count - 1);
    }
    results
}

/// The value arguments of an `IF`-like call.
fn passed_on(name: &str, count: usize) -> Option<Vec<usize>> {
    let (_, first, step) = PASS_ON.iter().find(|(n, ..)| *n == name)?;
    let mut values: Vec<usize> = (*first..count).step_by(*step).collect();
    // IFS(condition, value, ...) has no default; IF(c, a, b) and the others
    // take every argument from the first value on.
    if name == "IFS" {
        values.retain(|i| i % 2 == 1);
    }
    Some(values)
}

//==============================================================================
// Back to tokens
//==============================================================================

/// A formula with its dates written for the workbook.
pub(super) struct Written {
    pub tokens: Vec<Token>,
    /// Whether the formula's result is a date (a day number in the workbook)
    pub is_date: bool,
}

impl WorkbookRefs {
    /// `expr` with its dates written the way the workbook must hold them, and
    /// whether its result is a date. `None` when it has no date in it.
    fn written(
        &self,
        expr: &Expr,
        ctx: Context<'_>,
        holds: Holds,
    ) -> Result<Option<(Expr, bool)>, String> {
        // A column not calculated holds what its formula gives
        let holds = match holds {
            Holds::Unknown => {
                let dates = Dates {
                    refs: self,
                    ctx,
                    locals: Vec::new(),
                };
                if dates.kind(expr).is_date() {
                    Holds::Dates
                } else {
                    Holds::Other
                }
            },
            known => known,
        };
        let column_holds_dates = holds == Holds::Dates;
        let mut dates = Dates {
            refs: self,
            ctx,
            locals: Vec::new(),
        };
        if !dates.touches_dates(expr) {
            if column_holds_dates {
                return Err("its result is dates, made from text".to_string());
            }
            // No date to write, but text may still be read as a number or
            // ordered against one, which Excel does its own way (ADR-054)
            let read = match ctx {
                Context::Scalar { .. } => Read::Number,
                Context::Row { .. } if holds == Holds::Numbers => Read::Number,
                Context::Row { .. } => Read::Text,
            };
            dates.write(expr, dates.result_read(expr, read), false)?;
            return Ok(None);
        }
        let kind = dates.kind(expr);
        let is_date = kind.is_date();
        let read = match ctx {
            // A scalar is a number: a date is its day number
            Context::Scalar { .. } => Read::Number,
            Context::Row { .. } if column_holds_dates => {
                if !is_date {
                    return Err("its result is dates, made from text".to_string());
                }
                Read::Number
            },
            // Numbers in Forge: any date among them is its day number
            Context::Row { .. } if holds == Holds::Numbers => Read::Number,
            Context::Row { .. } => {
                // Dates in some rows and other values in others: Forge made
                // the column text, a number in it included, and no formula
                // does that
                if !matches!(kind, Kind::Text | Kind::Mixed | Kind::Number)
                    && dates.may_be_a_date(expr)
                {
                    return Err("its values are dates in some rows and not in others".to_string());
                }
                Read::Text
            },
        };
        let written = dates.write(expr, dates.result_read(expr, read), false)?;
        Ok(Some((written, is_date && read == Read::Number)))
    }

    /// Write the dates of a formula the way the workbook must hold them.
    /// `None` when the formula has no date in it: it is translated as it is.
    ///
    /// `holds` says what the formula's column holds in Forge: the cells of a
    /// column of dates must hold dates too.
    ///
    /// # Errors
    ///
    /// Returns the reason when a date is read in a way this module has no
    /// rule for; the caller writes the formula's values.
    pub(super) fn write_dates(
        &self,
        tokens: &[Token],
        ctx: Context<'_>,
        holds: Holds,
    ) -> Result<Option<Written>, String> {
        // A formula the parser does not read is left to the translator, which
        // says what is wrong with it
        let Ok(expr) = parse(tokens.to_vec()) else {
            return Ok(None);
        };
        Ok(self.written(&expr, ctx, holds)?.map(|(written, is_date)| {
            let mut tokens = Vec::new();
            push(&written, &mut tokens);
            Written { tokens, is_date }
        }))
    }

    /// Whether the result of a formula is a date in Forge.
    #[must_use]
    pub fn gives_a_date(&self, formula: &str, ctx: Context<'_>) -> bool {
        let body = formula.trim().trim_start_matches('=');
        tokenize(body)
            .ok()
            .and_then(|tokens| parse(tokens).ok())
            .is_some_and(|expr| {
                Dates {
                    refs: self,
                    ctx,
                    locals: Vec::new(),
                }
                .kind(&expr)
                .is_date()
            })
    }

    /// Whether a workbook's formula, with its cells named, gives a date in
    /// Forge once its dates are read back ([`WorkbookRefs::read_dates`]).
    #[must_use]
    pub fn reads_as_a_date(&self, formula: &str, ctx: Context<'_>) -> bool {
        let body = formula.trim().trim_start_matches('=');
        let Some(excel) = tokenize(body).ok().and_then(|tokens| parse(tokens).ok()) else {
            return false;
        };
        let read = map(&excel, &|e| {
            date_of_text(&e)
                .or_else(|| date_of_numbers(&e))
                .unwrap_or(e)
        });
        Dates {
            refs: self,
            ctx,
            locals: Vec::new(),
        }
        .kind(&read)
        .is_date()
    }

    /// Read the dates of a formula that came from a workbook (`forge import`).
    ///
    /// `formula` is the workbook's formula with its cells named, so a date in
    /// it is read as Excel reads it: a day number. It is the same calculation
    /// in Forge only if Forge would write that very formula for some Forge
    /// formula, which is then returned: the text of a date read back as the
    /// date, `DATE(2026, 1, 31)` as `"2026-01-31"`. A formula with no date in
    /// it is returned as it is.
    ///
    /// # Errors
    ///
    /// Returns the reason when Excel reads a date in the formula in a way
    /// Forge does not; the caller imports the formula's values.
    pub fn read_dates(
        &self,
        formula: &str,
        ctx: Context<'_>,
        holds: Holds,
    ) -> Result<String, String> {
        let body = formula.trim().trim_start_matches('=');
        let Some(excel) = tokenize(body).ok().and_then(|tokens| parse(tokens).ok()) else {
            return Ok(formula.to_string());
        };
        let excel = upper_names(&excel);
        let texts = map(&excel, &|e| date_of_text(&e).unwrap_or(e));
        let both = map(&texts, &|e| date_of_numbers(&e).unwrap_or(e));
        // `">=" & "2026-02-01"` is the criterion `">=2026-02-01"`
        let both = map(&both, &|e| match &e {
            Expr::BinaryOp { op, left, right } if op == "&" => match (&**left, &**right) {
                (Expr::Text(operator), Expr::Text(date))
                    if [">=", "<=", "<>", ">", "<", "="].contains(&operator.as_str())
                        && excel_date(date).is_some() =>
                {
                    Expr::Text(format!("{operator}{date}"))
                },
                _ => e,
            },
            _ => e,
        });
        let mut reason = None;
        for candidate in [both, texts] {
            // A column Excel shows as dates is a column of dates in Forge only
            // if its formula gives a date; a date plus a number of days is a
            // number in Forge, the day number Excel holds.
            let dates = Dates {
                refs: self,
                ctx,
                locals: Vec::new(),
            };
            let holds = match holds {
                Holds::Dates if !dates.kind(&candidate).is_date() => Holds::Numbers,
                other => other,
            };
            match self.written(&candidate, ctx, holds) {
                Ok(Some((written, _))) if written == excel => {},
                Ok(None) if candidate == excel => return Ok(formula.to_string()),
                Ok(_) => continue,
                Err(why) => {
                    reason.get_or_insert(why);
                    continue;
                },
            }
            let mut tokens = Vec::new();
            push(&candidate, &mut tokens);
            return Ok(formula_text(&tokens));
        }
        Err(reason.unwrap_or_else(|| {
            "Excel reads a date here as its day number, where Forge reads its text".to_string()
        }))
    }
}

#[cfg(test)]
mod tests {
    //! How a date in a formula is written for the workbook, and read back from
    //! it (ADR-054). Every Excel formula expected here is one Gnumeric and
    //! `LibreOffice` recalculate to Forge's result (forge-e2e-gnumeric,
    //! `edge_real_dates.yaml`).

    use super::Holds;
    use crate::excel::workbook_refs::{Context, WorkbookRefs};

    /// A table `t`: d and e dates (A, B), v numbers (C), name text (D), four rows.
    fn refs() -> WorkbookRefs {
        let columns = ["d", "e", "v", "name"].map(String::from).to_vec();
        let mut refs = WorkbookRefs::new([("t", columns, 4)], ["r.x"]);
        refs.add_column_types("t", [("v", false), ("name", true)]);
        refs.add_date_columns("t", ["d", "e"]);
        refs
    }

    const ROW: Context<'static> = Context::Row { table: "t", row: 2 };
    const SCALAR: Context<'static> = Context::Scalar { group: Some("r") };
    /// The text of the date in A2, however Excel is set up.
    const A2_TEXT: &str = "TEXT(YEAR(A2) * 10000 + MONTH(A2) * 100 + DAY(A2), \"0000-00-00\")";

    fn excel(formula: &str, ctx: Context<'_>, holds: Holds) -> Result<String, String> {
        refs().to_excel_as(formula, ctx, holds).map(|t| t.formula)
    }

    fn row(formula: &str) -> String {
        excel(formula, ROW, Holds::Other).unwrap()
    }

    #[test]
    fn a_date_read_as_a_number_is_the_cell() {
        assert_eq!(row("=YEAR(d)"), "=YEAR(A2)");
        assert_eq!(row("=e - d"), "=B2 - A2");
        assert_eq!(row("=d + 1"), "=A2 + 1");
        assert_eq!(row("=IF(e > d, 1, 0)"), "=IF(B2 > A2, 1, 0)");
        assert_eq!(row("=MAX(d, e)"), "=MAX(A2, B2)");
        assert_eq!(row("=TEXT(d, \"yyyy\")"), "=TEXT(A2, \"yyyy\")");
        assert_eq!(row("=YEAR(EDATE(d, 1))"), "=YEAR(EDATE(A2, 1))");
    }

    #[test]
    fn a_date_read_as_text_is_its_text() {
        assert_eq!(row("=LEFT(d, 4)"), format!("=LEFT({A2_TEXT}, 4)"));
        assert_eq!(row("=LEN(d)"), format!("=LEN({A2_TEXT})"));
        assert_eq!(row("=name & \"-\" & d"), format!("=D2 & \"-\" & {A2_TEXT}"));
        assert_eq!(
            row("=IF(ISNUMBER(d), 1, 0)"),
            format!("=IF(ISNUMBER({A2_TEXT}), 1, 0)")
        );
        // A date function gives a day number in Excel: its text is written too
        assert_eq!(
            row("=LEFT(EDATE(d, 1), 4)"),
            "=LEFT(TEXT(YEAR(EDATE(A2, 1)) * 10000 + MONTH(EDATE(A2, 1)) * 100 + DAY(EDATE(A2, 1)), \
         \"0000-00-00\"), 4)"
        );
    }

    #[test]
    fn a_date_written_in_a_formula_is_a_date_where_a_number_is_read() {
        assert_eq!(
            row("=IF(d > \"2026-06-01\", 1, 0)"),
            "=IF(A2 > DATE(2026, 6, 1), 1, 0)"
        );
        assert_eq!(row("=d - \"2026-01-01\""), "=A2 - DATE(2026, 1, 1)");
        assert_eq!(
            row("=DATEDIF(\"2022-01-01\", \"2024-02-01\", \"YD\")"),
            "=DATEDIF(DATE(2022, 1, 1), DATE(2024, 2, 1), \"YD\")"
        );
        // Where text is read it stays the text it is
        assert_eq!(row("=LEFT(\"2026-06-01\", 4)"), "=LEFT(\"2026-06-01\", 4)");
        // A month (2026-06) or text that is not a date is not touched
        assert_eq!(
            row("=IF(name = \"2026-06\", 1, 0)"),
            "=IF(D2 = \"2026-06\", 1, 0)"
        );
    }

    #[test]
    fn equality_follows_what_is_compared() {
        assert_eq!(row("=IF(d = e, 1, 0)"), "=IF(A2 = B2, 1, 0)");
        assert_eq!(
            row("=IF(d = \"2026-01-31\", 1, 0)"),
            "=IF(A2 = DATE(2026, 1, 31), 1, 0)"
        );
        assert_eq!(
            row("=IF(d = DATE(2026, 1, 31), 1, 0)"),
            "=IF(A2 = DATE(2026, 1, 31), 1, 0)"
        );
        // With text, a date is compared as the text Forge holds
        assert_eq!(
            row("=IF(d <> name, 1, 0)"),
            format!("=IF({A2_TEXT} <> D2, 1, 0)")
        );
        // Two dates written in the formula are two texts
        assert_eq!(
            row("=IF(\"2026-01-31\" = \"2026-01-31\", 1, 0)"),
            "=IF(\"2026-01-31\" = \"2026-01-31\", 1, 0)"
        );
    }

    #[test]
    fn a_column_of_dates_holds_dates() {
        let dated = |formula: &str| refs().to_excel_as(formula, ROW, Holds::Dates).unwrap();
        let edate = dated("=EDATE(d, 1)");
        assert_eq!(
            (edate.formula.as_str(), edate.is_date),
            ("=EDATE(A2, 1)", true)
        );
        let chosen = dated("=IF(v > 15, d, \"2000-01-01\")");
        assert_eq!(chosen.formula, "=IF(C2 > 15, A2, DATE(2000, 1, 1))");
        assert!(chosen.is_date);
        assert_eq!(dated("=INDEX(d, 3)").formula, "=INDEX($A$2:$A$5, 3)");
        assert_eq!(dated("=\"2026-01-31\"").formula, "=DATE(2026, 1, 31)");
        // A date made by a text function would be text in Excel
        for formula in ["=UPPER(d)", "=d & \"\"", "=name"] {
            let reason = excel(formula, ROW, Holds::Dates).unwrap_err();
            assert!(reason.contains("made from text"), "{formula}: {reason}");
        }
        // Not a column of dates: no date format
        assert!(!refs().to_excel("=YEAR(d)", ROW).unwrap().is_date);
    }

    #[test]
    fn a_value_passed_on_is_read_as_its_reader_reads_it() {
        // Text or a date: the date is its text
        assert_eq!(
            row("=IF(v > 15, d, \"none\")"),
            format!("=IF(C2 > 15, {A2_TEXT}, \"none\")")
        );
        // Read as a number further on: the day number
        assert_eq!(row("=IF(v > 15, d, 5) + 1"), "=IF(C2 > 15, A2, 5) + 1");
        assert_eq!(row("=YEAR(IF(v > 15, d, e))"), "=YEAR(IF(C2 > 15, A2, B2))");
        // Read as text further on: its text
        assert_eq!(
            row("=LEFT(IF(v > 15, d, e), 4)"),
            "=LEFT(IF(C2 > 15, TEXT(YEAR(A2) * 10000 + MONTH(A2) * 100 + DAY(A2), \"0000-00-00\"), \
         TEXT(YEAR(B2) * 10000 + MONTH(B2) * 100 + DAY(B2), \"0000-00-00\")), 4)"
        );
        // Dates in some rows and numbers in others: Forge made the column
        // numbers (the formula stays) or text (no formula does that)
        assert_eq!(
            excel("=IF(v > 15, d, 0)", ROW, Holds::Numbers).unwrap(),
            "=IF(C2 > 15, A2, 0)"
        );
        let reason = excel("=IF(v > 15, d, 0)", ROW, Holds::Other).unwrap_err();
        assert!(reason.contains("dates in some rows"), "{reason}");
    }

    #[test]
    fn whole_columns_of_dates_in_scalars() {
        let scalar = |formula: &str| excel(formula, SCALAR, Holds::Other).unwrap();
        assert_eq!(scalar("=MIN(t.d)"), "=MIN('t'!$A$2:$A$5)");
        assert_eq!(scalar("=YEAR(MAX(t.d))"), "=YEAR(MAX('t'!$A$2:$A$5))");
        assert_eq!(scalar("=INDEX(t.d, 2)"), "=INDEX('t'!$A$2:$A$5, 2)");
        assert_eq!(
            scalar("=MATCH(\"2026-12-01\", t.d, 0)"),
            "=MATCH(DATE(2026, 12, 1), 't'!$A$2:$A$5, 0)"
        );
        assert_eq!(
            scalar("=SUMPRODUCT((t.d > DATE(2026, 2, 1)) * t.v)"),
            "=SUMPRODUCT(('t'!$A$2:$A$5 > DATE(2026, 2, 1)) * 't'!$C$2:$C$5)"
        );
        // The text of an indexed date
        assert!(
            scalar("=LEN(INDEX(t.d, 1))").starts_with("=LEN(TEXT(YEAR(INDEX('t'!$A$2:$A$5, 1))")
        );
        // Places with no rule: the caller writes the value
        for (formula, why) in [
            ("=SUMPRODUCT(t.d, t.v)", "SUMPRODUCT reads a date as 0"),
            ("=MATCH(\"x\", t.d, 0)", "not a date is looked up"),
            ("=LEN(t.d)", "read as text, a whole column"),
        ] {
            let reason = excel(formula, SCALAR, Holds::Other).unwrap_err();
            assert!(reason.contains(why), "{formula}: {reason}");
        }
    }

    #[test]
    fn criteria_hold_day_numbers_not_dates_for_excel_to_parse() {
        let scalar = |formula: &str| excel(formula, SCALAR, Holds::Other).unwrap();
        assert_eq!(
            scalar("=COUNTIF(t.d, \"2026-02-28\")"),
            "=COUNTIF('t'!$A$2:$A$5, DATE(2026, 2, 28))"
        );
        assert_eq!(
            scalar("=COUNTIF(t.d, \">2026-02-01\")"),
            "=COUNTIF('t'!$A$2:$A$5, \">\" & DATE(2026, 2, 1))"
        );
        assert_eq!(
            scalar("=SUMIFS(t.v, t.d, \">=2026-02-01\", t.d, \"<=2026-02-28\")"),
            "=SUMIFS('t'!$C$2:$C$5, 't'!$A$2:$A$5, \">=\" & DATE(2026, 2, 1), 't'!$A$2:$A$5, \"<=\" \
         & DATE(2026, 2, 28))"
        );
        assert_eq!(
            scalar("=COUNTIF(t.d, \">=\" & INDEX(t.d, 2))"),
            "=COUNTIF('t'!$A$2:$A$5, \">=\" & INDEX('t'!$A$2:$A$5, 2))"
        );
        // Over text, a criterion is the text it is
        assert_eq!(
            scalar("=COUNTIF(t.name, \"2026-02-28\")"),
            "=COUNTIF('t'!$D$2:$D$5, \"2026-02-28\")"
        );
        // A number criterion is untouched
        assert_eq!(
            scalar("=COUNTIF(t.v, \">15\")"),
            "=COUNTIF('t'!$C$2:$C$5, \">15\")"
        );
    }

    #[test]
    fn a_formula_without_dates_is_translated_as_before() {
        assert_eq!(row("=v * 2"), "=C2 * 2");
        assert_eq!(row("=(v + 1) * (v - 1)"), "=(C2 + 1) * (C2 - 1)");
        assert_eq!(row("=UPPER(name)"), "=UPPER(D2)");
    }

    #[test]
    fn parentheses_are_written_where_the_order_needs_them() {
        assert_eq!(row("=(e - d) * 2"), "=(B2 - A2) * 2");
        assert_eq!(row("=e - (d - 1)"), "=B2 - (A2 - 1)");
        assert_eq!(row("=e - d - 1"), "=B2 - A2 - 1");
        assert_eq!(row("=-(e - d)"), "=-(B2 - A2)");
        assert_eq!(row("=2^(e - d)"), "=2^(B2 - A2)");
        assert_eq!(row("=(e - d)^2"), "=(B2 - A2)^2");
        assert_eq!(
            row("=(name & d) = \"x\""),
            format!("=D2 & {A2_TEXT} = \"x\"")
        );
        // Comparisons group from the left, in Forge as in Excel
        assert_eq!(
            row("=IF((d > e) = (v > 1), 1, 0)"),
            "=IF(A2 > B2 = (C2 > 1), 1, 0)"
        );
    }

    /// `forge import` gets the workbook's formula with its cells named: a date in
    /// it is what Excel holds, a day number.
    fn back(formula: &str, holds: Holds) -> Result<String, String> {
        refs().read_dates(formula, ROW, holds)
    }

    #[test]
    fn import_reads_the_text_of_a_date_back_as_the_date() {
        let text = "TEXT(YEAR(d) * 10000 + MONTH(d) * 100 + DAY(d), \"0000-00-00\")";
        assert_eq!(
            back(&format!("=LEFT({text}, 4)"), Holds::Other).unwrap(),
            "=LEFT(d, 4)"
        );
        assert_eq!(
            back(&format!("=name & \"-\" & {text}"), Holds::Other).unwrap(),
            "=name & \"-\" & d"
        );
        assert_eq!(
            back("=IF(d > DATE(2026, 6, 1), 1, 0)", Holds::Numbers).unwrap(),
            "=IF(d > \"2026-06-01\", 1, 0)"
        );
        assert_eq!(
            back("=IF(v > 15, d, DATE(2000, 1, 1))", Holds::Dates).unwrap(),
            "=IF(v > 15, d, \"2000-01-01\")"
        );
        assert_eq!(back("=EDATE(d, 1)", Holds::Dates).unwrap(), "=EDATE(d, 1)");
        assert_eq!(back("=YEAR(d)", Holds::Numbers).unwrap(), "=YEAR(d)");
    }

    #[test]
    fn import_keeps_what_excel_calculates() {
        // A date plus days is a number in Forge, the day number Excel holds,
        // even where Excel shows it as a date
        assert_eq!(back("=d + 30", Holds::Dates).unwrap(), "=d + 30");
        // DATE where text is read: the day number's text in Excel (4605), the
        // date's in Forge (2026, ADR-060), so the workbook's formula is its values
        assert!(back("=LEFT(DATE(2026, 1, 1), 4)", Holds::Other).is_err());
        // No date at all
        assert_eq!(back("=v * 2", Holds::Numbers).unwrap(), "=v * 2");
    }

    #[test]
    fn import_refuses_a_date_excel_reads_as_text() {
        // In Excel, LEFT of a date cell is the start of its day number ("4605");
        // in Forge, of its text ("2026"). Not the same calculation.
        for formula in ["=LEFT(d, 4)", "=name & d", "=IF(d = name, 1, 0)", "=LEN(d)"] {
            let reason = back(formula, Holds::Other).unwrap_err();
            assert!(reason.contains("day number"), "{formula}: {reason}");
        }
    }

    /// The table `t` with one more column, `due` (E): text that is a date in
    /// some rows (`n/a`, `2026-01-31`). It is text in the workbook.
    fn with_text_dates() -> WorkbookRefs {
        let columns = ["d", "e", "v", "name", "due"].map(String::from).to_vec();
        let mut refs = WorkbookRefs::new([("t", columns, 4)], ["r.x"]);
        refs.add_column_types("t", [("v", false), ("name", true), ("due", true)]);
        refs.add_date_columns("t", ["d", "e"]);
        refs.add_mixed_columns("t", ["due"]);
        refs
    }

    #[test]
    fn text_that_may_be_a_date_is_live_where_text_is_read() {
        let live = |formula: &str, ctx| {
            with_text_dates()
                .to_excel_as(formula, ctx, Holds::Other)
                .map(|t| t.formula)
                .unwrap()
        };
        assert_eq!(live("=LEFT(due, 4)", ROW), "=LEFT(E2, 4)");
        assert_eq!(live("=name & due", ROW), "=D2 & E2");
        assert_eq!(
            live("=IF(due = \"n/a\", 0, 1)", ROW),
            "=IF(E2 = \"n/a\", 0, 1)"
        );
        // A date written in the formula stays text: text is compared with text
        assert_eq!(
            live("=IF(due = \"2026-01-31\", 1, 0)", ROW),
            "=IF(E2 = \"2026-01-31\", 1, 0)"
        );
        // A date cell is compared as the text Forge holds for it
        assert_eq!(
            live("=IF(due = d, 1, 0)", ROW),
            format!("=IF(E2 = {A2_TEXT}, 1, 0)")
        );
        assert_eq!(
            live("=COUNTIF(t.due, \"n/a\")", SCALAR),
            "=COUNTIF('t'!$E$2:$E$5, \"n/a\")"
        );
        assert_eq!(live("=COUNTA(t.due)", SCALAR), "=COUNTA('t'!$E$2:$E$5)");
        assert_eq!(
            live("=MATCH(\"2026-01-31\", t.due, 0)", SCALAR),
            "=MATCH(\"2026-01-31\", 't'!$E$2:$E$5, 0)"
        );
        assert_eq!(live("=INDEX(t.due, 2)", ROW), "=INDEX($E$2:$E$5, 2)");
        // DATE gives a date (ADR-060): compared with text, as its text, as `d` is
        let found = live("=MATCH(DATE(2026, 12, 1), t.due, 0)", SCALAR);
        assert!(
            found.starts_with("=MATCH(TEXT(YEAR(DATE(2026, 12, 1))"),
            "{found}"
        );
        let jan31 = live("=IF(due = DATE(2026, 1, 31), 1, 0)", ROW);
        assert!(
            jan31.starts_with("=IF(E2 = TEXT(YEAR(DATE(2026, 1, 31))"),
            "{jan31}"
        );
    }

    /// Forge reads `2026-01-31` in a text column as its day number where a
    /// number is read. Excel reads text: greater than any number, skipped by
    /// `MAX`, equal to no number. Such a formula is written as its values.
    #[test]
    fn text_that_may_be_a_date_is_not_written_where_a_number_is_read() {
        let refs = with_text_dates();
        let rows = [
            "=YEAR(due)",
            "=due - d",
            "=due + 1",
            "=IF(due > d, 1, 0)",
            "=IF(due > \"2026-06-01\", 1, 0)",
            "=IF(due = v, 1, 0)",
            "=TEXT(due, \"yyyy\")",
            "=YEAR(INDEX(t.due, 2))",
            "=YEAR(IF(v > 1, due, d))",
            "=SWITCH(due, 46053, 1, 0)",
            "=MATCH(d, t.v, 0) + MATCH(v, t.due, 0)",
        ];
        for formula in rows {
            let refused = refs.to_excel_as(formula, ROW, Holds::Numbers);
            assert!(refused.is_err(), "{formula}: {refused:?}");
        }
        let scalars = [
            "=MAX(t.due)",
            "=SUM(t.due)",
            "=COUNT(t.due)",
            "=COUNTIF(t.due, \">2026-06-01\")",
            "=COUNTIF(t.due, \"2026-01-31\")",
            "=COUNTIF(t.due, \">5\")",
            "=COUNTIF(t.d, INDEX(t.due, 2))",
            "=SUMIF(t.v, \">1\", t.due)",
            "=SUMIFS(t.due, t.v, \">1\")",
            "=INDEX(t.due, 2)",
            "=SUMPRODUCT((t.due > DATE(2026, 2, 1)) * t.v)",
        ];
        for formula in scalars {
            let refused = refs.to_excel_as(formula, SCALAR, Holds::Other);
            assert!(refused.is_err(), "{formula}: {refused:?}");
        }
        // A column of dates cannot be made of text
        assert!(refs
            .to_excel_as("=IF(v > 1, due, d)", ROW, Holds::Dates)
            .is_err());
    }

    #[test]
    fn import_reads_text_that_may_be_a_date_as_excel_does() {
        let refs = with_text_dates();
        let back = |formula: &str, holds| refs.read_dates(formula, ROW, holds);
        assert_eq!(
            back("=LEFT(due, 4)", Holds::Other).unwrap(),
            "=LEFT(due, 4)"
        );
        assert_eq!(
            back("=IF(due = \"n/a\", 0, 1)", Holds::Numbers).unwrap(),
            "=IF(due = \"n/a\", 0, 1)"
        );
        // Excel finds text greater than any number; Forge would compare dates
        for formula in [
            "=IF(due > DATE(2026, 6, 1), 1, 0)",
            "=YEAR(due)",
            "=due + 1",
        ] {
            assert!(back(formula, Holds::Numbers).is_err(), "{formula}");
        }
    }

    /// The table `t` with `due` replaced by `code` (E), text with digits in it
    /// (`"5"`, `apple`, `"2"`): Forge reads `"5"` as 5 in a column, Excel as text.
    fn with_number_text() -> WorkbookRefs {
        let columns = ["d", "e", "v", "name", "code"].map(String::from).to_vec();
        let mut refs = WorkbookRefs::new([("t", columns, 4)], ["r.x"]);
        refs.add_column_types("t", [("v", false), ("name", true), ("code", true)]);
        refs.add_date_columns("t", ["d", "e"]);
        refs.add_mixed_columns("t", ["code"]);
        refs
    }

    /// Measured with a text column `["5", "apple", "2", "x"]`: `SUM` 7 in Forge
    /// and 0 in Gnumeric and `LibreOffice`, `COUNTIF(">3")` 1 and 0, `s > 3` FALSE
    /// for `"2"` in Forge and TRUE in both, `IFERROR(IF(s > 3, 1, 0), -1)` -1 for
    /// `apple` in Forge and 1 in both. Each is written as its values.
    #[test]
    fn text_with_digits_is_not_read_as_a_number_in_the_workbook() {
        let refs = with_number_text();
        for formula in [
            "=code * 2",
            "=code + 1",
            "=ROUND(code, 0)",
            "=MAX(code, 1)",
            "=IFERROR(IF(code > 3, 1, 0), -1)",
            "=IF(code = 5, 1, 0)",
        ] {
            let refused = refs.to_excel_as(formula, ROW, Holds::Numbers);
            assert!(refused.is_err(), "{formula}: {refused:?}");
        }
        for formula in [
            "=SUM(t.code)",
            "=COUNT(t.code)",
            "=MAX(t.code)",
            "=COUNTIF(t.code, \">3\")",
            "=COUNTIF(t.code, \"5\")",
            "=SUMIF(t.v, \">1\", t.code)",
            "=SUMPRODUCT(t.code, t.v)",
            "=MATCH(5, t.code, 0)",
        ] {
            let refused = refs.to_excel_as(formula, SCALAR, Holds::Other);
            assert!(refused.is_err(), "{formula}: {refused:?}");
        }
    }

    #[test]
    fn text_with_digits_read_as_text_stays_a_formula() {
        let refs = with_number_text();
        let row = |formula: &str, holds| refs.to_excel_as(formula, ROW, holds).unwrap().formula;
        let scalar = |formula: &str| {
            refs.to_excel_as(formula, SCALAR, Holds::Other)
                .unwrap()
                .formula
        };
        assert_eq!(row("=LEN(code)", Holds::Numbers), "=LEN(E2)");
        assert_eq!(
            row("=code & \"-\" & name", Holds::Other),
            "=E2 & \"-\" & D2"
        );
        assert_eq!(
            row("=IF(code = \"5\", 1, 0)", Holds::Numbers),
            "=IF(E2 = \"5\", 1, 0)"
        );
        assert_eq!(scalar("=COUNTA(t.code)"), "=COUNTA('t'!$E$2:$E$5)");
        assert_eq!(
            scalar("=COUNTIF(t.code, \"apple\")"),
            "=COUNTIF('t'!$E$2:$E$5, \"apple\")"
        );
    }

    /// A column of words has no text a spreadsheet would turn into a number:
    /// read as a number it fails, or counts as nothing, in both. Ordered against
    /// a number or against text, Excel has a rule Forge does not.
    #[test]
    fn words_are_read_the_same_but_not_ordered_the_same() {
        let refs = refs();
        let scalar = |formula: &str| refs.to_excel_as(formula, SCALAR, Holds::Other);
        assert_eq!(
            scalar("=SUMPRODUCT(t.name, t.v)").unwrap().formula,
            "=SUMPRODUCT('t'!$D$2:$D$5, 't'!$C$2:$C$5)"
        );
        assert_eq!(
            scalar("=SUM(t.name)").unwrap().formula,
            "=SUM('t'!$D$2:$D$5)"
        );
        for formula in [
            "=IFERROR(IF(name > 3, 1, 0), -1)",
            "=IFERROR(IF(name > \"m\", 1, 0), -1)",
            "=IFERROR(IF(3 <= name, 1, 0), -1)",
        ] {
            let refused = refs.to_excel_as(formula, ROW, Holds::Numbers);
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|why| why.contains("compares text")),
                "{formula}: {refused:?}"
            );
        }
    }

    /// Text made in the formula can hold anything; read as a number, Excel
    /// parses more than Forge does (`"5%"`, `"1,200"`, `"2026-01"`).
    #[test]
    fn text_made_in_a_formula_is_not_read_as_a_number() {
        let refs = refs();
        let row = |formula: &str| refs.to_excel_as(formula, ROW, Holds::Numbers);
        assert!(row("=LEFT(name, 2) * 1").is_err());
        assert!(row("=(name & \"1\") + 0").is_err());
        // VALUE says what is meant, and reads its text as text
        assert_eq!(
            row("=VALUE(LEFT(name, 2))").unwrap().formula,
            "=VALUE(LEFT(D2, 2))"
        );
        // The formula's own text result is not a reading of text
        assert_eq!(
            refs.to_excel_as("=LEFT(name, 2)", ROW, Holds::Numbers)
                .unwrap()
                .formula,
            "=LEFT(D2, 2)"
        );
    }
}
