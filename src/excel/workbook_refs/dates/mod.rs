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

mod kinds;
mod print;
mod rewrite;

use kinds::{Kind, Read};

use super::{Context, WorkbookRefs};
use crate::core::array_calculator::parser::{parse, Expr};
use crate::core::array_calculator::tokenizer::{tokenize, Token};
pub use print::formula_of;
use print::{formula_text, push};
use rewrite::{date_of_numbers, date_of_text, map, upper_names, Dates};

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
        // The latest of two dates is a date (ADR-060 addendum), so a column of it
        // holds dates, and a number column of it is its day numbers
        assert_eq!(
            excel("=MAX(d, e)", ROW, Holds::Dates).unwrap(),
            "=MAX(A2, B2)"
        );
        assert_eq!(
            excel("=MAX(d, e)", ROW, Holds::Numbers).unwrap(),
            "=MAX(A2, B2)"
        );
        assert_eq!(row("=MAX(v, 1)"), "=MAX(C2, 1)");
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

    /// A lookup that touches a date has no rule, so its values are written;
    /// one beside a date, but not over one, is written as a formula
    #[test]
    fn a_lookup_over_dates_is_written_as_values() {
        for formula in [
            "=VLOOKUP(d, t.e, 1, FALSE)",
            "=VLOOKUP(v, t.d, 1)",
            "=HLOOKUP(d, {1,2}, 1)",
        ] {
            let reason = excel(formula, ROW, Holds::Other).unwrap_err();
            assert!(reason.contains("a date in"), "{formula}: {reason}");
        }
        assert_eq!(
            row("=IF(YEAR(d) > 2020, VLOOKUP(v, t.v, 1, FALSE), 0)"),
            "=IF(YEAR(A2) > 2020, VLOOKUP(C2, $C$2:$C$5, 1, FALSE), 0)"
        );
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

    mod real_dates {
        //! Dates are dates in the workbook (ADR-054, roadmap T3-EXPORT-DATES).
        //!
        //! The expected numbers are the same calculation in Python (`datetime`, not
        //! Forge), as in forge-e2e-gnumeric's `edge_real_dates.yaml`, where Gnumeric
        //! and `LibreOffice` recalculate the exported cells:
        //! ```python
        //! base = date(1899, 12, 30)
        //! d = ["2026-01-31", "2026-02-28", "2026-12-01", "2026-02-28"]   # amount 10, 20, 30, 40
        //! [(date.fromisoformat(x) - base).days for x in d]   # 46053, 46081, 46357, 46081
        //! ```

        use crate::cli::{calculate_core, export_core, import_core};
        use calamine::{open_workbook, Data, Reader, Xlsx};
        use std::path::Path;

        const MODEL: &str = r#"_forge_version: "5.2.0"
due:
  d: ["2026-01-31", "2026-02-28", "2026-12-01", "2026-02-28"]
  amount: [10, 20, 30, 40]
  label: ["a", "b", "c", "d"]
  next_month: "=EDATE(d, 1)"
  late: "=IF(d > \"2026-06-01\", 1, 0)"
  is_jan31: "=IF(d = DATE(2026, 1, 31), 1, 0)"
  year_text: "=LEFT(d, 4)"
  tag: "=label & \"-\" & d"
  tomorrow: "=d + 1"
  later_of: "=MAX(d, DATE(2026, 6, 1))"
  or_default: "=IF(amount > 15, d, \"2000-01-01\")"
  before: "=PREV(d, \"2026-01-01\")"
  shown: "=TEXT(d, \"mmm d, yyyy\")"
plan:
  month: {start: 2026-11, count: 4, unit: month}
  month_end: "=EOMONTH(month, 0)"
checks:
  first: {formula: "=MIN(due.d)"}
  count: {formula: "=COUNT(due.d)"}
  after: {formula: "=COUNTIF(due.d, \">2026-02-01\")"}
  between: {formula: "=SUMIFS(due.amount, due.d, \">=2026-02-01\", due.d, \"<=2026-02-28\")"}
  found: {formula: "=MATCH(\"2026-12-01\", due.d, 0)"}
  late: {formula: "=SUM(due.late)"}
  jan31: {formula: "=SUM(due.is_jan31)"}
  tomorrow: {formula: "=SUM(due.tomorrow)"}
  later_of: {formula: "=SUM(due.later_of)"}
  last_month_end: {formula: "=INDEX(plan.month_end, 4)"}
"#;

        fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }

        fn column(
            result: &crate::cli::commands::results::CalculationResult,
            name: &str,
        ) -> Vec<String> {
            let (table, column) = name.split_once('.').unwrap();
            result.tables[table].values[column]
                .iter()
                .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
                .collect()
        }

        fn numbers(
            result: &crate::cli::commands::results::CalculationResult,
            name: &str,
        ) -> Vec<f64> {
            let (table, column) = name.split_once('.').unwrap();
            result.tables[table].values[column]
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        }

        #[test]
        fn a_date_is_its_day_number_where_a_number_is_read() {
            let dir = tempfile::tempdir().unwrap();
            let model = write(dir.path(), "m.yaml", MODEL);
            let result = calculate_core(&model, true, None).unwrap();
            for (name, expected) in [
                ("checks.first", 46053.0),
                ("checks.count", 4.0),
                ("checks.after", 3.0),
                ("checks.between", 60.0),
                ("checks.found", 3.0),
                ("checks.late", 1.0),
                ("checks.jan31", 1.0),
                ("checks.tomorrow", 184_576.0),
                ("checks.later_of", 184_879.0),
                ("checks.last_month_end", 46446.0),
            ] {
                let got = result.scalars[name].unwrap();
                assert!((got - expected).abs() < 1e-9, "{name}: {got}");
            }
            assert_eq!(
                column(&result, "due.next_month"),
                ["2026-02-28", "2026-03-28", "2027-01-01", "2026-03-28"]
            );
            assert_eq!(column(&result, "due.year_text"), ["2026"; 4]);
            assert_eq!(column(&result, "due.tag")[2], "c-2026-12-01");
            assert_eq!(column(&result, "due.shown")[0], "Jan 31, 2026");
            assert_eq!(
                column(&result, "due.or_default"),
                ["2000-01-01", "2026-02-28", "2026-12-01", "2026-02-28"]
            );
        }

        /// The cell at a 0-based (row, column) of a sheet: its value and its formula.
        fn cell(xlsx: &Path, sheet: &str, row: u32, col: u32) -> (Data, String) {
            let mut workbook: Xlsx<_> = open_workbook(xlsx).unwrap();
            let values = workbook.worksheet_range(sheet).unwrap();
            let formulas = workbook.worksheet_formula(sheet).unwrap();
            (
                values.get_value((row, col)).cloned().unwrap_or(Data::Empty),
                formulas.get_value((row, col)).cloned().unwrap_or_default(),
            )
        }

        fn day_number(cell: &Data) -> f64 {
            match cell {
                Data::DateTime(date) if date.is_datetime() => date.as_f64(),
                other => panic!("not a date cell: {other:?}"),
            }
        }

        #[test]
        fn dates_are_date_cells_and_formulas_read_them_as_forge_does() {
            let dir = tempfile::tempdir().unwrap();
            let model = write(dir.path(), "m.yaml", MODEL);
            let xlsx = dir.path().join("m.xlsx");
            let exported = export_core(&model, &xlsx).unwrap();
            assert_eq!(exported.warnings, Vec::<String>::new());

            // Columns of `due`: d A, amount B, label C, next_month D, late E,
            // is_jan31 F, year_text G, tag H, tomorrow I, later_of J, or_default K,
            // before L, shown M
            let text_of_a = "TEXT(YEAR(A2) * 10000 + MONTH(A2) * 100 + DAY(A2), \"0000-00-00\")";
            // A column of dates: the day number, with a date format
            assert!((day_number(&cell(&xlsx, "due", 1, 0).0) - 46053.0).abs() < 1e-9);
            assert!((day_number(&cell(&xlsx, "due", 3, 0).0) - 46357.0).abs() < 1e-9);
            // A formula that gives a date: a date cell too, the result cached
            let (value, formula) = cell(&xlsx, "due", 1, 3);
            assert_eq!(formula, "EDATE(A2, 1)");
            assert!((day_number(&value) - 46081.0).abs() < 1e-9);
            for (col, formula) in [
                (4, "IF(A2 > DATE(2026, 6, 1), 1, 0)".to_string()),
                (5, "IF(A2 = DATE(2026, 1, 31), 1, 0)".to_string()),
                (6, format!("LEFT({text_of_a}, 4)")),
                (7, format!("C2 & \"-\" & {text_of_a}")),
                (8, "A2 + 1".to_string()),
                (9, "MAX(A2, DATE(2026, 6, 1))".to_string()),
                (10, "IF(B2 > 15, A2, DATE(2000, 1, 1))".to_string()),
                (11, "(DATE(2026, 1, 1))".to_string()),
                (12, "TEXT(A2, \"mmm d, yyyy\")".to_string()),
            ] {
                assert_eq!(cell(&xlsx, "due", 1, col).1, formula, "column {col}");
            }
            // The row above, in a column of dates
            assert_eq!(cell(&xlsx, "due", 2, 11).1, "(A2)");
            assert!((day_number(&cell(&xlsx, "due", 2, 11).0) - 46053.0).abs() < 1e-9);
            // Text made from a date is text
            assert_eq!(
                cell(&xlsx, "due", 1, 7).0,
                Data::String("a-2026-01-31".to_string())
            );
            // A period column and what is calculated from it
            assert!((day_number(&cell(&xlsx, "plan", 1, 0).0) - 46327.0).abs() < 1e-9);
            assert_eq!(cell(&xlsx, "plan", 4, 1).1, "EOMONTH(A5, 0)");
            assert!((day_number(&cell(&xlsx, "plan", 4, 1).0) - 46446.0).abs() < 1e-9);
            // Criteria hold day numbers, for any language of Excel
            assert_eq!(
                cell(&xlsx, "checks", 3, 1).1,
                "COUNTIF('due'!$A$2:$A$5, \">\" & DATE(2026, 2, 1))"
            );
            assert_eq!(
                cell(&xlsx, "checks", 5, 1).1,
                "MATCH(DATE(2026, 12, 1), 'due'!$A$2:$A$5, 0)"
            );
        }

        #[test]
        fn export_and_import_give_the_model_back() {
            let dir = tempfile::tempdir().unwrap();
            let model = write(dir.path(), "m.yaml", MODEL);
            let xlsx = dir.path().join("m.xlsx");
            export_core(&model, &xlsx).unwrap();
            let back = dir.path().join("back.yaml");
            let imported = import_core(&xlsx, &back, false, false).unwrap();
            assert_eq!(imported.warnings, Vec::<String>::new());

            let text = std::fs::read_to_string(&back).unwrap();
            for formula in [
                "next_month: =EDATE(d, 1)",
                "late: =IF(d > \"2026-06-01\", 1, 0)",
                "is_jan31: =IF(d = \"2026-01-31\", 1, 0)",
                "year_text: =LEFT(d, 4)",
                "tag: =label & \"-\" & d",
                "tomorrow: =d + 1",
                "later_of: =MAX(d, \"2026-06-01\")",
                "or_default: =IF(amount > 15, d, \"2000-01-01\")",
                "before: =PREV(d, \"2026-01-01\")",
                "shown: =TEXT(d, \"mmm d, yyyy\")",
                "month_end: =EOMONTH(month, 0)",
                "formula: =COUNTIF(due.d, \">2026-02-01\")",
                "formula: =SUMIFS(due.amount, due.d, \">=2026-02-01\", due.d, \"<=2026-02-28\")",
                "formula: =MATCH(\"2026-12-01\", due.d, 0)",
            ] {
                assert!(text.contains(formula), "{formula} in:\n{text}");
            }
            // The dates come back as dates
            assert!(
                text.contains("  d:\n  - 2026-01-31\n  - 2026-02-28"),
                "{text}"
            );

            // And it calculates to what the original does, every table and scalar
            let original = calculate_core(&model, true, None).unwrap();
            let again = calculate_core(&back, true, None).unwrap();
            assert_eq!(original.scalars, again.scalars);
            for (name, table) in &original.tables {
                assert_eq!(table.values, again.tables[name].values, "{name}");
            }
        }

        #[test]
        fn a_formula_excel_would_read_another_way_is_written_as_its_values() {
            let dir = tempfile::tempdir().unwrap();
            let model = write(
                dir.path(),
                "m.yaml",
                r#"_forge_version: "5.1.0"
t:
  d: ["2026-01-31", "2026-02-28", "2026-12-01"]
  n: [30, 10, 40]
  shout: "=UPPER(d)"
  mixed: "=IF(n > 15, d, 0)"
  numbers: "=IF(n > 15, 0, d)"
  month: ["2026-01", "2026-02", "2026-03"]
  old: ["1899-12-31", "1900-01-15", "1900-02-28"]
"#,
            );
            let xlsx = dir.path().join("m.xlsx");
            let warnings = export_core(&model, &xlsx).unwrap().warnings;
            // A date made by a text function would be text in Excel; a column of
            // dates and numbers is text in Forge when its first row is a date
            assert_eq!(warnings.len(), 2, "{warnings:?}");
            assert!(warnings[0]
                .contains("t.shout: written as values (its result is dates, made from text)"));
            assert!(warnings[1]
                .contains("t.mixed: written as values (its values are dates in some rows"));
            // Their values: dates, and the text Forge holds
            assert!((day_number(&cell(&xlsx, "t", 1, 2).0) - 46053.0).abs() < 1e-9);
            assert_eq!(cell(&xlsx, "t", 2, 3).0, Data::String("0".to_string()));
            // First row a number: Forge made the column numbers, and the formula stays
            assert_eq!(cell(&xlsx, "t", 1, 4).1, "IF(B2 > 15, 0, A2)");
            // Months, and dates before 1900-03-01 (Excel has no day numbers for
            // them), stay text
            assert_eq!(
                cell(&xlsx, "t", 1, 5).0,
                Data::String("2026-01".to_string())
            );
            assert_eq!(
                cell(&xlsx, "t", 1, 6).0,
                Data::String("1899-12-31".to_string())
            );
        }

        /// A workbook that did not come from Forge: dates typed into cells, and the
        /// formulas a person writes over them.
        #[test]
        fn import_reads_a_workbook_with_dates() {
            use rust_xlsxwriter::{Format, Formula, Workbook};
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("invoices.xlsx");
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet().set_name("invoices").unwrap();
            let date = Format::new().set_num_format("d mmm yyyy");
            for (col, header) in ["issued", "year", "due", "next", "late", "code", "month"]
                .iter()
                .enumerate()
            {
                sheet
                    .write_string(0, u16::try_from(col).unwrap(), *header)
                    .unwrap();
            }
            // 2026-01-31, 2026-02-28, 2026-12-01
            for (i, serial) in [46053.0, 46081.0, 46357.0].iter().enumerate() {
                let row = u32::try_from(i).unwrap() + 1;
                let n = row + 1;
                sheet
                    .write_number_with_format(row, 0, *serial, &date)
                    .unwrap();
                let year = Formula::new(format!("=YEAR(A{n})")).set_result("2026");
                sheet.write_formula(row, 1, year).unwrap();
                // A date plus 30 days, shown as a date
                let due =
                    Formula::new(format!("=A{n}+30")).set_result(format!("{}", serial + 30.0));
                sheet.write_formula_with_format(row, 2, due, &date).unwrap();
                let next = Formula::new(format!("=EDATE(A{n},1)")).set_result("46081");
                sheet
                    .write_formula_with_format(row, 3, next, &date)
                    .unwrap();
                let late = Formula::new(format!("=IF(A{n}>DATE(2026,6,1),1,0)")).set_result("0");
                sheet.write_formula(row, 4, late).unwrap();
                // In Excel, the first four characters of the day number: "4605"
                let code = Formula::new(format!("=LEFT(A{n},4)")).set_result("4605");
                sheet.write_formula(row, 5, code).unwrap();
                let month = Formula::new(format!("=TEXT(A{n},\"yyyy-mm\")")).set_result("2026-01");
                sheet.write_formula(row, 6, month).unwrap();
            }
            workbook.save(&xlsx).unwrap();

            let back = dir.path().join("invoices.yaml");
            let imported = import_core(&xlsx, &back, false, false).unwrap();
            let text = std::fs::read_to_string(&back).unwrap();
            // The dates are dates, and the format Excel showed them in comes back as
            // their format: (ADR-061)
            assert!(
                text.contains(
                    "issued:\n    value:\n    - 2026-01-31\n    - 2026-02-28\n    - 2026-12-01\n    format: d mmm yyyy"
                ),
                "{text}"
            );
            assert!(
                text.contains("next:\n    formula: =EDATE(issued, 1)\n    format: d mmm yyyy"),
                "{text}"
            );
            for formula in [
                "year: =YEAR(issued)",
                // A number in Forge, the day number Excel holds and shows as a date
                "due: =issued + 30",
                "late: =IF(issued > \"2026-06-01\", 1, 0)",
                "month: =TEXT(issued, \"yyyy-mm\")",
            ] {
                assert!(text.contains(formula), "{formula} in:\n{text}");
            }
            // Excel reads the date as its day number there, Forge as its text: the
            // values Excel calculated are kept, and the import says so
            assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
            assert!(
                imported.warnings[0].contains("invoices.code: imported as values")
                    && imported.warnings[0].contains("day number"),
                "{:?}",
                imported.warnings
            );
            assert!(text.contains("code:\n  - 4605\n  - 4605"), "{text}");

            let result = calculate_core(&back, true, None).unwrap();
            assert_eq!(numbers(&result, "invoices.year"), [2026.0; 3]);
            assert_eq!(
                numbers(&result, "invoices.due"),
                [46083.0, 46111.0, 46387.0]
            );
            assert_eq!(
                column(&result, "invoices.next"),
                ["2026-02-28", "2026-03-28", "2027-01-01"]
            );
            assert_eq!(numbers(&result, "invoices.late"), [0.0, 0.0, 1.0]);
            assert_eq!(
                column(&result, "invoices.month"),
                ["2026-01", "2026-02", "2026-12"]
            );
        }

        /// A column that is `n/a` in some rows and a date in others stays text in
        /// the workbook. Forge reads each date in it as its day number where a
        /// number is read; Excel reads text there (greater than any number, skipped
        /// by `MAX`, equal to no number). Those formulas are written as values.
        #[test]
        fn text_that_is_a_date_in_some_rows_is_not_read_as_a_number_in_excel() {
            let dir = tempfile::tempdir().unwrap();
            let model = write(
                dir.path(),
                "loans.yaml",
                r#"_forge_version: "5.1.0"
loans:
  maturity: ["n/a", "2026-01-31", "2026-12-01", "none"]
  amount: [10, 20, 30, 40]
  open: "=IF(maturity = \"n/a\", 1, 0)"
  jan31: "=IF(maturity = \"2026-01-31\", 1, 0)"
  year: "=IFERROR(YEAR(maturity), 0)"
  late: "=IFERROR(IF(maturity > DATE(2026, 6, 1), 1, 0), 0)"
  is_jan31: "=IF(maturity = DATE(2026, 1, 31), 1, 0)"
checks:
  last: {formula: "=MAX(loans.maturity)"}
  not_set: {formula: "=COUNTIF(loans.maturity, \"n/a\")"}
  after_june: {formula: "=COUNTIF(loans.maturity, \">2026-06-01\")"}
  years: {formula: "=SUM(loans.year)"}
"#,
            );
            // 2026-12-01 is day 46357; the two years are 2026 + 2026
            let result = calculate_core(&model, true, None).unwrap();
            assert_eq!(numbers(&result, "loans.year"), [0.0, 2026.0, 2026.0, 0.0]);
            assert_eq!(numbers(&result, "loans.late"), [0.0, 0.0, 1.0, 0.0]);
            assert_eq!(numbers(&result, "loans.is_jan31"), [0.0, 1.0, 0.0, 0.0]);
            for (name, expected) in [
                ("checks.last", 46357.0),
                ("checks.not_set", 1.0),
                ("checks.after_june", 1.0),
                ("checks.years", 4052.0),
            ] {
                let value = result.scalars[name].unwrap();
                assert!((value - expected).abs() < 1e-9, "{name}: {value}");
            }

            let xlsx = dir.path().join("loans.xlsx");
            let warnings = export_core(&model, &xlsx).unwrap().warnings;
            let written_as_values: Vec<&str> = warnings
                .iter()
                .filter_map(|w| w.split_once(':').map(|(name, _)| name.trim()))
                .collect();
            assert_eq!(
                written_as_values,
                // `maturity = DATE(2026, 1, 31)` is written live: DATE gives a date,
                // compared with text as its text, which Excel finds equal too (ADR-060)
                [
                    "loans.year",
                    "loans.late",
                    "checks.last",
                    "checks.after_june"
                ],
                "{warnings:?}"
            );
            // The column is text, as Forge holds it
            assert_eq!(
                cell(&xlsx, "loans", 2, 0).0,
                Data::String("2026-01-31".to_string())
            );
            // Text read as text stays a formula: C open, D jan31
            assert_eq!(cell(&xlsx, "loans", 2, 2).1, "IF(A3 = \"n/a\", 1, 0)");
            assert_eq!(
                cell(&xlsx, "loans", 2, 3).1,
                "IF(A3 = \"2026-01-31\", 1, 0)"
            );
            assert_eq!(
                cell(&xlsx, "checks", 2, 1).1,
                "COUNTIF('loans'!$A$2:$A$5, \"n/a\")"
            );
            // Compared with a DATE, the date as its text (ADR-060): G is_jan31
            assert!(
                cell(&xlsx, "loans", 2, 6)
                    .1
                    .starts_with("IF(A3 = TEXT(YEAR(DATE(2026, 1, 31))"),
                "{:?}",
                cell(&xlsx, "loans", 2, 6)
            );
            // A number read from it is Forge's value, with no formula: E year,
            // F late, and the latest maturity
            for (sheet, row, col, value) in [
                ("loans", 2, 4, 2026.0),
                ("loans", 3, 5, 1.0),
                ("checks", 3, 1, 1.0),
            ] {
                let (held, formula) = cell(&xlsx, sheet, row, col);
                assert_eq!(formula, "", "{sheet} {row} {col}");
                assert_eq!(held, Data::Float(value), "{sheet} {row} {col}");
            }
            // The latest maturity is a date (ADR-060 addendum): a date cell, as a
            // date scalar is, with Forge's value and no formula
            let (held, formula) = cell(&xlsx, "checks", 1, 1);
            assert_eq!(formula, "");
            assert!(
                matches!(&held, Data::DateTime(d) if (d.as_f64() - 46357.0).abs() < 1e-9),
                "{held:?}"
            );
            // A formula over those values is live
            assert_eq!(cell(&xlsx, "checks", 4, 1).1, "SUM('loans'!$E$2:$E$5)");
        }

        /// A workbook that holds dates as text (`2026-01-31` typed into a text cell,
        /// or written by a Forge before 10.0.0-rc.12): Excel reads text there, so
        /// `A2 > DATE(2026, 6, 1)` is TRUE for every row. Forge reads the same text
        /// as a date. The formula is imported as the values Excel calculated.
        #[test]
        fn import_reads_dates_held_as_text_the_way_excel_reads_them() {
            use rust_xlsxwriter::{Formula, Workbook};
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("text_dates.xlsx");
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet().set_name("t").unwrap();
            for (col, header) in ["d", "late", "year_text", "is_jan31"].iter().enumerate() {
                sheet
                    .write_string(0, u16::try_from(col).unwrap(), *header)
                    .unwrap();
            }
            for (i, date) in ["2026-01-31", "2026-07-31", "2026-12-01"]
                .iter()
                .enumerate()
            {
                let row = u32::try_from(i).unwrap() + 1;
                let n = row + 1;
                sheet.write_string(row, 0, *date).unwrap();
                // Text is greater than any number in Excel: 1 in every row
                let late = Formula::new(format!("=IF(A{n}>DATE(2026,6,1),1,0)")).set_result("1");
                sheet.write_formula(row, 1, late).unwrap();
                let year = Formula::new(format!("=LEFT(A{n},4)")).set_result("2026");
                sheet.write_formula(row, 2, year).unwrap();
                let is_jan31 = Formula::new(format!("=IF(A{n}=\"2026-01-31\",1,0)"))
                    .set_result(if i == 0 { "1" } else { "0" });
                sheet.write_formula(row, 3, is_jan31).unwrap();
            }
            workbook.save(&xlsx).unwrap();

            let back = dir.path().join("text_dates.yaml");
            let imported = import_core(&xlsx, &back, false, false).unwrap();
            let text = std::fs::read_to_string(&back).unwrap();
            assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
            assert!(
                imported.warnings[0].starts_with("t.late: imported as values"),
                "{:?}",
                imported.warnings
            );
            assert!(text.contains("late:\n  - 1\n  - 1\n  - 1"), "{text}");
            assert!(text.contains("year_text: =LEFT(d, 4)"), "{text}");
            assert!(
                text.contains("is_jan31: =IF(d = \"2026-01-31\", 1, 0)"),
                "{text}"
            );
            // What came in calculates to what the workbook held
            let result = calculate_core(&back, true, None).unwrap();
            assert_eq!(numbers(&result, "t.late"), [1.0, 1.0, 1.0]);
            assert_eq!(numbers(&result, "t.is_jan31"), [1.0, 0.0, 0.0]);
        }
    }
}
