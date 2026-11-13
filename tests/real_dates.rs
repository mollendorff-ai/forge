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

use calamine::{open_workbook, Data, Reader, Xlsx};
use mollendorff_forge::cli::{calculate_core, export_core, import_core};
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
    result: &mollendorff_forge::cli::commands::results::CalculationResult,
    name: &str,
) -> Vec<String> {
    let (table, column) = name.split_once('.').unwrap();
    result.tables[table].values[column]
        .iter()
        .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
        .collect()
}

fn numbers(
    result: &mollendorff_forge::cli::commands::results::CalculationResult,
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
    assert!(
        warnings[0].contains("t.shout: written as values (its result is dates, made from text)")
    );
    assert!(warnings[1].contains("t.mixed: written as values (its values are dates in some rows"));
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
        let due = Formula::new(format!("=A{n}+30")).set_result(format!("{}", serial + 30.0));
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
    // The dates are dates, whatever format Excel showed them in
    assert!(
        text.contains("issued:\n  - 2026-01-31\n  - 2026-02-28\n  - 2026-12-01"),
        "{text}"
    );
    for formula in [
        "year: =YEAR(issued)",
        // A number in Forge, the day number Excel holds and shows as a date
        "due: =issued + 30",
        "next: =EDATE(issued, 1)",
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
        ("checks", 1, 1, 46357.0),
        ("checks", 3, 1, 1.0),
    ] {
        let (held, formula) = cell(&xlsx, sheet, row, col);
        assert_eq!(formula, "", "{sheet} {row} {col}");
        assert_eq!(held, Data::Float(value), "{sheet} {row} {col}");
    }
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
