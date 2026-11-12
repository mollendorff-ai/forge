//! A formula that gives text keeps that text in the workbook (roadmap
//! T3-CACHED-TEXT).
//!
//! The workbook writer typed a cached result by reading it, so `010` was
//! stored as the number 10 and `TRUE` as a boolean. A viewer that does not
//! recalculate (Quick Look, iOS Files) then showed 10 and TRUE for the text
//! `010` and `TRUE`.

use calamine::{open_workbook, Data, Reader, Xlsx};
use mollendorff_forge::cli::export_core;

const MODEL: &str = r#"_forge_version: "5.1.0"
t:
  n: [1, 25, 40]
  padded: "=TEXT(n, \"000\")"
  word: "=IF(n > 20, \"TRUE\", \"FALSE\")"
  exponent: "=\"1e\" & n"
  plain: "=IF(n > 20, \"high\", \"low\")"
  flag: "=n > 20"
  double: "=n * 2"
"#;

fn cells(column: u32) -> Vec<Data> {
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("m.yaml");
    std::fs::write(&model, MODEL).unwrap();
    let xlsx = dir.path().join("m.xlsx");
    export_core(&model, &xlsx).unwrap();
    let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
    let range = book.worksheet_range("t").unwrap();
    (1..=3)
        .map(|row| range.get_value((row, column)).cloned().unwrap())
        .collect()
}

fn text(values: &[&str]) -> Vec<Data> {
    values.iter().map(|v| Data::String((*v).into())).collect()
}

#[test]
fn text_that_looks_like_a_number_is_cached_as_text() {
    assert_eq!(cells(1), text(&["001", "025", "040"]));
    assert_eq!(cells(3), text(&["1e1", "1e25", "1e40"]));
}

#[test]
fn the_words_true_and_false_are_cached_as_text() {
    assert_eq!(cells(2), text(&["FALSE", "TRUE", "TRUE"]));
}

#[test]
fn other_results_keep_their_type() {
    assert_eq!(cells(4), text(&["low", "high", "high"]));
    assert_eq!(
        cells(5),
        [Data::Bool(false), Data::Bool(true), Data::Bool(true)]
    );
    assert_eq!(
        cells(6),
        [Data::Float(2.0), Data::Float(50.0), Data::Float(80.0)]
    );
}

#[test]
fn the_formulas_are_still_there() {
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("m.yaml");
    std::fs::write(&model, MODEL).unwrap();
    let xlsx = dir.path().join("m.xlsx");
    export_core(&model, &xlsx).unwrap();
    let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
    let formulas = book.worksheet_formula("t").unwrap();
    assert!(formulas
        .get_value((1, 1))
        .is_some_and(|f| f.contains("TEXT(")));
}
