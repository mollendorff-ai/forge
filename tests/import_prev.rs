//! `forge import` reads every `PREV` back as `forge export` wrote it
//! (roadmap T3-IMPORT-PREV, ADR-053).
//!
//! The workbook writes `PREV(value, first, back)` as `first` in the first
//! `back` rows and as the cell `back` rows up in the others. A column whose
//! PREV was part of a larger formula came back as `IF(ROW() = 1, ...)`, and
//! one that read two rows up came back as values.

use mollendorff_forge::cli::{export_core, import_core};
use mollendorff_forge::core::ArrayCalculator;
use mollendorff_forge::ColumnValue;
use std::path::Path;

const MODEL: &str = r#"_forge_version: "5.1.0"
loan:
  principal: 1000
  rate: 0.01
t:
  x: [10, 20, 30, 40, 50]
  opening: "=PREV(closing, loan.principal)"
  closing: "=PREV(closing, loan.principal) * (1 + loan.rate) + x"
  two_back: "=PREV(x, 0, 2)"
  both: "=PREV(x, 1) + PREV(x, 5, 2)"
  change: "=x - PREV(x, x)"
  negative: "=PREV(x, -1)"
  of_a_calculation: "=PREV(closing - x, 0) * 2"
"#;

fn round_trip() -> String {
    round_trip_files().1
}

/// The model's text and the text imported from its workbook.
fn round_trip_files() -> (String, String) {
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("m.yaml");
    std::fs::write(&model, MODEL).unwrap();
    let xlsx = dir.path().join("m.xlsx");
    export_core(&model, &xlsx).unwrap();
    let back = dir.path().join("back.yaml");
    import_core(&xlsx, &back, false, false).unwrap();
    (MODEL.to_string(), std::fs::read_to_string(back).unwrap())
}

fn columns(text: &str) -> Vec<(String, Vec<f64>)> {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("m.yaml");
    std::fs::write(&file, text).unwrap();
    let model = mollendorff_forge::parser::parse_model(Path::new(&file)).unwrap();
    let model = ArrayCalculator::new(model).calculate_all().unwrap();
    model.tables["t"]
        .columns
        .iter()
        .map(|(name, column)| match &column.values {
            ColumnValue::Number(v) => (name.clone(), v.clone()),
            other => panic!("{name}: {other:?}"),
        })
        .collect()
}

#[test]
fn the_imported_model_calculates_the_same_numbers() {
    let (model, back) = round_trip_files();
    let mut before = columns(&model);
    let mut after = columns(&back);
    before.sort_by(|a, b| a.0.cmp(&b.0));
    after.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(before, after);
}

#[test]
fn every_prev_comes_back_as_written() {
    let text = round_trip();
    for written in [
        "opening: =PREV(closing, loan.principal)",
        "closing: =PREV(closing, loan.principal) * (1 + loan.rate) + x",
        "two_back: =PREV(x, 0, 2)",
        "both: =PREV(x, 1) + PREV(x, 5, 2)",
        "change: =x - PREV(x, x)",
        "negative: =PREV(x, -1)",
        "of_a_calculation: =PREV(closing - x, 0) * 2",
    ] {
        assert!(text.contains(written), "{written}\n{text}");
    }
    assert!(!text.contains("IF(ROW()"), "{text}");
}
