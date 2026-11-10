//! How a date in a formula is written for the workbook, and read back from
//! it (ADR-054). Every Excel formula expected here is one Gnumeric and
//! `LibreOffice` recalculate to Forge's result (forge-e2e-gnumeric,
//! `edge_real_dates.yaml`).

use super::dates::Holds;
use super::{Context, WorkbookRefs};

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
    assert!(scalar("=LEN(INDEX(t.d, 1))").starts_with("=LEN(TEXT(YEAR(INDEX('t'!$A$2:$A$5, 1))"));
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
    // DATE where text is read is the day number's text in both
    assert_eq!(
        back("=LEFT(DATE(2026, 1, 1), 4)", Holds::Other).unwrap(),
        "=LEFT(DATE(2026, 1, 1), 4)"
    );
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
        "=IF(due = DATE(2026, 1, 31), 1, 0)",
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
        "=MATCH(DATE(2026, 12, 1), t.due, 0)",
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
