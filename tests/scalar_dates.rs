//! A date as a scalar (ADR-060): written `2026-10-31` in model format 5.4.0,
//! read as a cell of a date column is, printed and written back as a date,
//! text in JSON, a date cell in the exported workbook and a date again after
//! `forge import`. `DATE` gives a date. Formats before 5.4.0 refuse a date as a
//! scalar, and the commands that vary an input refuse a date input.

use mollendorff_forge::cli::{calculate_core, export_core, goal_seek_core, import_core};
use mollendorff_forge::cli::{compare_core, sensitivity_core};
use std::path::{Path, PathBuf};

const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  last_actual: 2026-10-31
  start:
    value: 2026-04-01
    notes: "the first day of the fiscal year"
  rate: 0.1
months:
  month: {start: 2026-04, count: 19, unit: month}
  after: "=IF(month > inputs.last_actual, 1, 0)"
  year_end: "=DATE(YEAR(month), 12, 31)"
out:
  year:
    formula: "=YEAR(inputs.last_actual)"
  plus_one:
    formula: "=inputs.last_actual + 1"
  days:
    formula: "=inputs.last_actual - inputs.start"
  open_months:
    formula: "=COUNTIF(months.month, \">\" & inputs.last_actual)"
  same_day:
    formula: "=IF(inputs.last_actual = DATE(2026, 10, 31), 1, 0)"
  left:
    formula: "=VALUE(LEFT(inputs.last_actual, 4))"
  is_number:
    formula: "=IF(ISNUMBER(inputs.last_actual), 1, 0)"
  next_year:
    formula: "=EOMONTH(inputs.last_actual, 12)"
  close:
    formula: "=inputs.last_actual"
  made:
    formula: "=DATE(2026, 10, 31)"
  grown:
    formula: "=100 * (1 + rate)"
scenarios:
  later:
    last_actual: 2026-11-30
"#;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_date_scalar_is_read_as_a_cell_of_a_date_column() {
    let dir = tempfile::tempdir().unwrap();
    let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
    let number = |name: &str| result.scalars[name].unwrap();
    // Where a number is read, the day number (2026-10-31 is 46326)
    assert_eq!(number("out.year"), 2026.0);
    assert_eq!(number("out.plus_one"), 46327.0);
    assert_eq!(number("out.days"), 213.0);
    // November 2026 to October 2027
    assert_eq!(number("out.open_months"), 12.0);
    assert_eq!(number("out.same_day"), 1.0);
    // Where text is read, the date's text, as for a column (ADR-054)
    assert_eq!(number("out.left"), 2026.0);
    assert_eq!(number("out.is_number"), 0.0);
    // A row formula reads it too
    let after = &result.tables["months"].values["after"];
    assert_eq!(after.iter().filter(|v| v.as_f64() == Some(1.0)).count(), 12);

    // Dates, and formulas whose value is a date, are shown as dates
    for (name, date) in [
        ("inputs.last_actual", "2026-10-31"),
        ("inputs.start", "2026-04-01"),
        ("out.next_year", "2027-10-31"),
        ("out.close", "2026-10-31"),
        ("out.made", "2026-10-31"),
    ] {
        assert_eq!(result.shown(name).as_deref(), Some(date), "{name}");
    }
    // A date plus days is a number, and so is a number
    assert_eq!(result.shown("out.plus_one").as_deref(), Some("46327"));
    assert_eq!(result.shown("out.grown").as_deref(), Some("110"));
    // DATE gives a date in a column too
    assert_eq!(result.tables["months"].values["year_end"][0], "2026-12-31");
}

#[test]
fn json_gives_a_date_as_its_text() {
    let dir = tempfile::tempdir().unwrap();
    let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["scalars"]["inputs.last_actual"], "2026-10-31");
    assert_eq!(json["scalars"]["out.next_year"], "2027-10-31");
    assert_eq!(json["scalars"]["out.year"], 2026.0);
    assert!(json.get("dates").is_none(), "{json}");
    // And reads back to the same result
    let back: mollendorff_forge::cli::commands::results::CalculationResult =
        serde_json::from_value(json).unwrap();
    assert_eq!(back.scalars["inputs.last_actual"], Some(46326.0));
    assert!(back.dates.contains("out.next_year"));
    assert!(!back.dates.contains("out.year"));
}

#[test]
fn calculate_writes_a_date_back_as_a_date_in_5_4_0_and_as_its_day_number_before() {
    let dir = tempfile::tempdir().unwrap();
    let new = write(
        dir.path(),
        "new.yaml",
        "_forge_version: \"5.4.0\"\nend:\n  formula: \"=DATE(2026, 10, 31)\"\n",
    );
    calculate_core(&new, false, None).unwrap();
    let text = std::fs::read_to_string(&new).unwrap();
    assert!(text.contains("  value: 2026-10-31\n"), "{text}");
    // It reads back as the date, and a second run changes nothing
    let again = calculate_core(&new, false, None).unwrap();
    assert!(!again.file_updated);

    let old = write(
        dir.path(),
        "old.yaml",
        "_forge_version: \"5.3.0\"\nend:\n  formula: \"=DATE(2026, 10, 31)\"\n",
    );
    let result = calculate_core(&old, false, None).unwrap();
    let text = std::fs::read_to_string(&old).unwrap();
    assert!(text.contains("  value: 46326\n"), "{text}");
    // A date all the same where it is shown
    assert_eq!(result.shown("end").as_deref(), Some("2026-10-31"));
}

#[test]
fn a_date_is_refused_where_it_cannot_be() {
    let dir = tempfile::tempdir().unwrap();
    let error = |text: &str| {
        calculate_core(&write(dir.path(), "bad.yaml", text), true, None)
            .unwrap_err()
            .to_string()
    };
    let older = error("_forge_version: \"5.3.0\"\ninputs:\n  last_actual: 2026-10-31\n");
    assert!(
        older.contains("model format 5.4.0") && older.contains("5.3.0"),
        "{older}"
    );
    let month = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 2026-10\n");
    assert!(
        month.contains("the month 2026-10") && month.contains("2026-10-01"),
        "{month}"
    );
    let early = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 1900-01-15\n");
    assert!(early.contains("1900-03-01"), "{early}");
    let impossible = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 2026-02-30\n");
    assert!(
        impossible.contains("no day of the calendar"),
        "{impossible}"
    );
    let unit = error(
        "_forge_version: \"5.4.0\"\ninputs:\n  last:\n    value: 2026-10-31\n    unit: CAD\n",
    );
    assert!(unit.contains("a date has no unit"), "{unit}");
}

#[test]
fn a_scenario_gives_a_date_input_another_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "m.yaml", MODEL);
    let later = calculate_core(&path, true, Some("later")).unwrap();
    assert_eq!(
        later.shown("inputs.last_actual").as_deref(),
        Some("2026-11-30")
    );
    assert_eq!(later.scalars["out.open_months"], Some(11.0));
    let compared = compare_core(&path, &["later".to_string()]).unwrap();
    assert!(compared.dates.contains("inputs.last_actual"));
    assert!(!compared.dates.contains("out.year"));

    // A number for a date input is refused
    let wrong = MODEL.replace("last_actual: 2026-11-30", "last_actual: 46356");
    let error = calculate_core(&write(dir.path(), "w.yaml", &wrong), true, Some("later"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("is a date"), "{error}");
}

#[test]
fn the_commands_that_vary_an_input_refuse_a_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "m.yaml", MODEL);
    let sensitivity = sensitivity_core(
        &path,
        "inputs.last_actual",
        "46300,46400,50",
        None,
        None,
        "out.days",
    )
    .unwrap_err()
    .to_string();
    assert!(
        sensitivity.contains("is a date") && sensitivity.contains("sensitivity"),
        "{sensitivity}"
    );
    let seek = goal_seek_core(
        &path,
        "out.days",
        100.0,
        "inputs.last_actual",
        (None, None),
        0.001,
    )
    .unwrap_err()
    .to_string();
    assert!(seek.contains("is a date"), "{seek}");
    // A number input beside it is varied as before
    assert!(sensitivity_core(&path, "inputs.rate", "0.1,0.2,0.1", None, None, "out.grown").is_ok());
}

#[test]
fn export_writes_a_date_cell_and_import_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "m.yaml", MODEL);
    let xlsx = dir.path().join("m.xlsx");
    let exported = export_core(&path, &xlsx).unwrap();
    // Every formula is written live (scenarios are not exported, and say so)
    assert!(
        !exported
            .warnings
            .iter()
            .any(|w| w.contains("written as a value")),
        "{:?}",
        exported.warnings
    );
    let back = dir.path().join("back.yaml");
    import_core(&xlsx, &back, false, false).unwrap();
    let text = std::fs::read_to_string(&back).unwrap();
    assert!(text.starts_with("_forge_version: \"5.4.0\""), "{text}");
    assert!(text.contains("last_actual: 2026-10-31"), "{text}");
    assert!(text.contains("start: 2026-04-01"), "{text}");
    // The formulas come back as written, and give the same values
    assert!(
        text.contains("=VALUE(LEFT(inputs.last_actual, 4))"),
        "{text}"
    );
    assert!(text.contains("\">\" & inputs.last_actual"), "{text}");
    let original = calculate_core(&path, true, None).unwrap();
    let again = calculate_core(&back, true, None).unwrap();
    for (name, value) in &original.scalars {
        assert_eq!(again.scalars.get(name), Some(value), "{name}");
        assert_eq!(again.shown(name), original.shown(name), "{name}");
    }
}
