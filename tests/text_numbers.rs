//! Text that looks like a number, in the workbook (roadmap T1-TEXT-NUMBERS,
//! ADR-054 decision 6).
//!
//! Measured on `s: ["5", "apple", "2", "x"]` with the formulas written live,
//! as Forge 10.0.0-rc.12 writes them: `SUM(t.s)` is 7 in Forge and 0 in
//! Gnumeric and `LibreOffice`, `COUNTIF(t.s, ">3")` 1 and 0, and
//! `IFERROR(IF(s > 3, 1, 0), -1)` is 1, -1, 0, -1 in Forge and 1 in every row
//! in both (any text is greater than any number there).

use calamine::{open_workbook, Data, Reader, Xlsx};
use mollendorff_forge::cli::export_core;

const MODEL: &str = r#"_forge_version: "5.1.0"
t:
  s: ["5", "apple", "2", "x"]
  region: ["north", "south", "east", "west"]
  n: [1, 2, 3, 4]
  twice: "=IFERROR(s * 2, -1)"
  above: "=IFERROR(IF(s > 3, 1, 0), -1)"
  is_five: "=IF(s = \"5\", 1, 0)"
  size: "=LEN(s)"
  label: "=region & \"-\" & s"
r:
  total: {formula: "=SUM(t.s)"}
  over_three: {formula: "=COUNTIF(t.s, \">3\")"}
  apples: {formula: "=COUNTIF(t.s, \"apple\")"}
  filled: {formula: "=COUNTA(t.s)"}
  words: {formula: "=SUMPRODUCT(t.region, t.n)"}
"#;

#[test]
fn text_with_digits_read_as_a_number_is_written_as_values() {
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("m.yaml");
    std::fs::write(&model, MODEL).unwrap();
    let xlsx = dir.path().join("m.xlsx");
    let warnings = export_core(&model, &xlsx).unwrap().warnings;
    let names: Vec<&str> = warnings
        .iter()
        .filter_map(|w| w.split_once(':').map(|(name, _)| name.trim()))
        .collect();
    assert_eq!(
        names,
        ["t.twice", "t.above", "r.total", "r.over_three"],
        "{warnings:?}"
    );

    let mut workbook: Xlsx<_> = open_workbook(&xlsx).unwrap();
    let t = workbook.worksheet_formula("t").unwrap();
    let r = workbook.worksheet_formula("r").unwrap();
    let values = workbook.worksheet_range("r").unwrap();
    // Read as text: the same in both, so live (columns F is_five, G size,
    // H label; r rows 4 to 6)
    assert_eq!(t.get_value((1, 5)).unwrap(), "IF(A2 = \"5\", 1, 0)");
    assert_eq!(t.get_value((1, 6)).unwrap(), "LEN(A2)");
    assert_eq!(t.get_value((1, 7)).unwrap(), "B2 & \"-\" & A2");
    assert_eq!(
        r.get_value((3, 1)).unwrap(),
        "COUNTIF('t'!$A$2:$A$5, \"apple\")"
    );
    assert_eq!(r.get_value((4, 1)).unwrap(), "COUNTA('t'!$A$2:$A$5)");
    // Words read as numbers are nothing in both
    assert_eq!(
        r.get_value((5, 1)).unwrap(),
        "SUMPRODUCT('t'!$B$2:$B$5, 't'!$C$2:$C$5)"
    );
    // Forge's values, with no formula
    assert_eq!(r.get_value((1, 1)).map_or("", String::as_str), "");
    assert_eq!(values.get_value((1, 1)), Some(&Data::Float(7.0)));
    assert_eq!(values.get_value((2, 1)), Some(&Data::Float(1.0)));
}
