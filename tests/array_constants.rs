//! Array constants (ADR-064): `{1,2,3}` (a row) and `{1;2;3}` (a column) of
//! numbers, texts and TRUE or FALSE, as in Excel. Before, `{` was a syntax
//! error, and `forge import` turned a formula with a constant into values.
//! The answers are checked against Gnumeric and `LibreOffice` in
//! `validation/gnumeric/tests/functions/array_constants.yaml`; these tests
//! cover what the engines cannot: the refusals and the import.

use mollendorff_forge::cli::{calculate_core, export_core, import_core};
use rust_xlsxwriter::{Formula, Workbook};

fn calculate(yaml: &str) -> Result<std::collections::BTreeMap<String, Option<f64>>, String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(&path, yaml).unwrap();
    calculate_core(&path, true, None)
        .map(|r| r.scalars)
        .map_err(|e| e.to_string())
}

fn one(formula: &str) -> Result<Option<f64>, String> {
    let yaml = format!(
        "_forge_version: \"5.4.0\"\ns:\n  a: {{formula: {}}}\n",
        serde_json::to_string(formula).unwrap()
    );
    calculate(&yaml).map(|s| s["s.a"])
}

#[test]
#[allow(clippy::float_cmp)] // small whole numbers
fn constants_calculate() {
    assert_eq!(one("=SUM({1,2,3})").unwrap(), Some(6.0));
    assert_eq!(one("=SUM({1;2;3})").unwrap(), Some(6.0));
    assert_eq!(one("=SUM({-1, +2, -3.5E1})").unwrap(), Some(-34.0));
    assert_eq!(
        one("=MATCH(\"b\", {\"a\";\"b\";\"c\"}, 0)").unwrap(),
        Some(2.0)
    );
    assert_eq!(one("=INDEX({10,20,30}, 2)").unwrap(), Some(20.0));
    assert_eq!(one("=SUMPRODUCT({1,2,3}, {4,5,6})").unwrap(), Some(32.0));
    assert_eq!(one("=IF(AND({TRUE, FALSE}), 1, 0)").unwrap(), Some(0.0));
}

#[test]
#[allow(clippy::float_cmp)] // small whole numbers
fn a_row_formula_reads_a_constant_whole() {
    let yaml = "_forge_version: \"5.4.0\"\nt:\n  x: [1, 2, 3]\n  y: \"=INDEX({10;20;30}, x)\"\n\
                s:\n  total: {formula: \"=SUM(t.y)\"}\n";
    assert_eq!(calculate(yaml).unwrap()["s.total"], Some(60.0));
}

#[test]
fn what_forge_cannot_hold_is_refused_with_a_reason() {
    for (formula, reason) in [
        ("=SUM({1,2;3,4})", "two-dimensional"),
        ("=SUM({1,#N/A})", "NA() gives #N/A"),
        ("=SUM({1,x})", "numbers, texts and TRUE or FALSE only"),
        ("=SUM({1,2+3})", "numbers, texts and TRUE or FALSE only"),
        ("=SUM({})", "at least one item"),
        ("=SUM({1,2", "numbers, texts and TRUE or FALSE only"),
        ("={1,2,3}", "returns 3 values"),
    ] {
        let err = one(formula).unwrap_err();
        assert!(err.contains(reason), "{formula}: {err}");
    }
    let yaml = "_forge_version: \"5.4.0\"\nt:\n  x: [1, 2, 3]\n  y: \"=x * {1,2,3}\"\n";
    let err = calculate(yaml).unwrap_err();
    assert!(err.contains("has 3 values"), "{err}");
}

#[test]
fn import_keeps_a_formula_with_a_constant() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx = dir.path().join("book.xlsx");
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("t").unwrap();
    sheet.write_string(0, 0, "a").unwrap();
    sheet.write_string(0, 1, "b").unwrap();
    sheet.write_string(0, 2, "c").unwrap();
    for row in 1..=2u32 {
        sheet.write_number(row, 0, f64::from(row)).unwrap();
        let b = Formula::new(format!("=SUM({{1,2,3}})+A{}", row + 1)).set_result("0");
        sheet.write_formula(row, 1, b).unwrap();
        let c = Formula::new("=SUM({1,2;3,4})").set_result("10");
        sheet.write_formula(row, 2, c).unwrap();
    }
    workbook.save(&xlsx).unwrap();

    let out = dir.path().join("book.yaml");
    let imported = import_core(&xlsx, &out, false, false).unwrap();
    let yaml = std::fs::read_to_string(&out).unwrap();
    assert!(yaml.contains("SUM({1, 2, 3}) + a"), "{yaml}");
    // The two-dimensional one is imported as values, with the reason
    assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
    assert!(
        imported.warnings[0].contains("rows and columns"),
        "{:?}",
        imported.warnings
    );
}

#[test]
fn export_then_import_gives_the_same_answers() {
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("m.yaml");
    std::fs::write(
        &model,
        "_forge_version: \"5.4.0\"\ns:\n  a: {formula: \"=SUM({1;2;3}) * 2\"}\n  \
         b: {formula: \"=MATCH(\\\"c\\\", {\\\"a\\\",\\\"b\\\",\\\"c\\\"}, 0)\"}\n  \
         c: {formula: \"=LARGE({5,-1,3}, 3)\"}\n",
    )
    .unwrap();
    let first = calculate_core(&model, true, None).unwrap().scalars;
    let xlsx = dir.path().join("m.xlsx");
    export_core(&model, &xlsx).unwrap();
    let back = dir.path().join("back.yaml");
    let imported = import_core(&xlsx, &back, false, false).unwrap();
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    let text = std::fs::read_to_string(&back).unwrap();
    assert!(text.contains("{1; 2; 3}"), "{text}");
    let again = calculate_core(&back, true, None).unwrap().scalars;
    assert_eq!(first, again);
}
