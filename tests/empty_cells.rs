//! An empty cell in a column of numbers (ADR-059): written `null` in model
//! format 5.3.0, kept by `forge calculate`, shown as null in JSON, exported as
//! an empty cell and imported back as one. Formats before 5.3.0 refuse it.

use mollendorff_forge::cli::{calculate_core, import_core};
use std::path::Path;

const MODEL: &str = r#"_forge_version: "5.3.0"
months:
  actual: [985, 1032, null, null]
  is_actual: "=ISNUMBER(actual)"
  shown: "=actual"
totals:
  average:
    formula: "=AVERAGE(months.actual)"
  months_closed:
    formula: "=COUNT(months.actual)"
"#;

fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn an_empty_cell_is_skipped_by_aggregates_and_read_as_0_by_formulas() {
    let dir = tempfile::tempdir().unwrap();
    let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
    // (985 + 1032) / 2, of the two months that have one
    assert_eq!(result.scalars["totals.average"], Some(1008.5));
    assert_eq!(result.scalars["totals.months_closed"], Some(2.0));
    let column = |name: &str| -> Vec<serde_json::Value> {
        result.tables["months"].values[name]
            .iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect()
    };
    // JSON shows the empty cell as null; a formula that reads it gives 0
    assert_eq!(column("actual")[2], serde_json::Value::Null);
    assert_eq!(column("shown")[2], serde_json::json!(0.0));
    assert_eq!(column("is_actual")[2], serde_json::json!(false));
}

#[test]
fn forge_calculate_keeps_the_empty_cells_it_writes_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "m.yaml", MODEL);
    calculate_core(&path, false, None).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("actual: [985, 1032, null, null]"), "{text}");
    assert!(text.contains("value: 1008.5"), "{text}");
}

#[test]
fn an_earlier_format_says_which_one_has_the_empty_cell() {
    let dir = tempfile::tempdir().unwrap();
    for version in ["5.1.0", "5.2.0"] {
        let text = MODEL.replace("5.3.0", version);
        let err = calculate_core(&write(dir.path(), "m.yaml", &text), true, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("'months.actual' row 3 is empty"), "{err}");
        assert!(err.contains("5.3.0"), "{err}");
    }
}

#[test]
fn only_a_column_of_numbers_has_empty_cells() {
    let dir = tempfile::tempdir().unwrap();
    let text = "_forge_version: \"5.3.0\"\nt:\n  name: [\"a\", null]\n";
    let err = calculate_core(&write(dir.path(), "m.yaml", text), true, None)
        .unwrap_err()
        .to_string();
    // The schema says so before the parser does
    assert!(
        err.contains("'t.name' mixes text and empty entries"),
        "{err}"
    );
}

#[test]
fn export_leaves_the_cell_empty_and_import_reads_it_back() {
    use calamine::{open_workbook, Data, Reader, Xlsx};
    let dir = tempfile::tempdir().unwrap();
    let model = write(dir.path(), "m.yaml", MODEL);
    let xlsx = dir.path().join("m.xlsx");
    mollendorff_forge::cli::export(&model, &xlsx, false).unwrap();
    let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
    let sheet = book.worksheet_range("months").unwrap();
    assert_eq!(sheet.get((1, 0)), Some(&Data::Float(985.0)));
    assert!(
        matches!(sheet.get((3, 0)), None | Some(Data::Empty)),
        "{:?}",
        sheet.get((3, 0))
    );

    let back = dir.path().join("back.yaml");
    import_core(&xlsx, &back, false, false).unwrap();
    let result = calculate_core(&back, true, None).unwrap();
    assert_eq!(result.scalars["totals.average"], Some(1008.5));
    let actual = &result.tables["months"].values["actual"];
    assert_eq!(
        serde_json::to_value(&actual[3]).unwrap(),
        serde_json::Value::Null
    );
}
