//! `format:` beside `unit:` (ADR-061, model format 5.4.0): an Excel number
//! format that changes how a number is shown in the workbook, `forge report`
//! and JSON's `formatted`, and never its value. Forge accepts only codes it
//! writes as Excel does; an earlier format refuses `format:`.

use mollendorff_forge::cli::{calculate_core, export_core, import_core};
use std::path::{Path, PathBuf};

const MODEL: &str = r##"_forge_version: "5.4.0"
year:
  price: {value: 16, format: "$#,##0.00"}
  close: {value: 2026-10-31, format: "mmm yyyy"}
  margin:
    formula: "=-0.1234"
    format: "0.0%;(0.0%)"
months:
  units: {value: [1200, 1500, 0], format: "#,##0"}
  revenue:
    formula: "=units * year.price * -1"
    format: "#,##0;(#,##0);\"-\""
    unit: CAD
    notes: "a formula column keeps its metadata"
"##;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_format_changes_how_a_number_is_shown_and_never_its_value() {
    let dir = tempfile::tempdir().unwrap();
    let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
    // The values are what they were
    assert_eq!(result.scalars["year.price"], Some(16.0));
    assert_eq!(result.scalars["year.margin"], Some(-0.1234));
    assert_eq!(
        result.tables["months"].values["revenue"],
        [-19200.0, -24000.0, 0.0]
    );
    // JSON shows them as their formats do, beside the values
    assert_eq!(result.formatted["year.price"], "$16.00");
    assert_eq!(result.formatted["year.close"], "Oct 2026");
    assert_eq!(result.formatted["year.margin"], "(12.3%)");
    assert_eq!(
        result.tables["months"].formatted["units"],
        ["1,200", "1,500", "0"]
    );
    assert_eq!(
        result.tables["months"].formatted["revenue"],
        ["(19,200)", "(24,000)", "-"]
    );
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["scalars"]["year.price"], 16.0);
    assert_eq!(json["formatted"]["year.price"], "$16.00");
}

#[test]
fn a_model_without_format_has_no_formatted_in_json() {
    let dir = tempfile::tempdir().unwrap();
    let plain = "_forge_version: \"5.1.0\"\nyear:\n  price: 16\nt:\n  x: [1, 2]\n";
    let result = calculate_core(&write(dir.path(), "p.yaml", plain), true, None).unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("formatted").is_none(), "{json}");
    assert!(json["tables"]["t"].get("formatted").is_none(), "{json}");
}

#[test]
fn a_format_forge_does_not_read_or_on_the_wrong_value_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let error = |text: &str| {
        calculate_core(&write(dir.path(), "bad.yaml", text), true, None)
            .unwrap_err()
            .to_string()
    };
    let older = error("_forge_version: \"5.3.0\"\ny:\n  price: {value: 16, format: \"0.00\"}\n");
    assert!(older.contains("model format 5.4.0"), "{older}");
    let condition =
        error("_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"[>10]0\"}\n");
    assert!(
        condition.contains("'y.price.format'") && condition.contains("only a colour"),
        "{condition}"
    );
    let fourth =
        error("_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"0;0;0;@\"}\n");
    assert!(fourth.contains("fourth section"), "{fourth}");
    let date_on_number =
        error("_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"mmm yyyy\"}\n");
    assert!(
        date_on_number.contains("is a date format"),
        "{date_on_number}"
    );
    let number_on_date =
        error("_forge_version: \"5.4.0\"\ny:\n  close: {value: 2026-10-31, format: \"0.00\"}\n");
    assert!(
        number_on_date.contains("is a number format"),
        "{number_on_date}"
    );
    // A formula's value is known once calculated
    let formula =
        error("_forge_version: \"5.4.0\"\ny:\n  end:\n    formula: \"=DATE(2026, 1, 31)\"\n    format: \"0.00\"\n");
    assert!(
        formula.contains("the value of 'y.end' is a date"),
        "{formula}"
    );
}

#[test]
fn export_writes_the_format_and_import_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "m.yaml", MODEL);
    let xlsx = dir.path().join("m.xlsx");
    export_core(&path, &xlsx).unwrap();
    let back = dir.path().join("back.yaml");
    let imported = import_core(&xlsx, &back, false, false).unwrap();
    assert!(
        !imported
            .warnings
            .iter()
            .any(|w| w.contains("number format")),
        "{:?}",
        imported.warnings
    );
    let text = std::fs::read_to_string(&back).unwrap();
    assert!(text.starts_with("_forge_version: \"5.4.0\""), "{text}");
    for format in [
        "$#,##0.00",
        "mmm yyyy",
        "0.0%;(0.0%)",
        "#,##0",
        "#,##0;(#,##0);\"-\"",
    ] {
        assert!(text.contains(format), "{format} in {text}");
    }
    // The same values and the same text
    let original = calculate_core(&path, true, None).unwrap();
    let again = calculate_core(&back, true, None).unwrap();
    assert_eq!(again.scalars, original.scalars);
    assert_eq!(again.formatted, original.formatted);
    assert_eq!(
        again.tables["months"].formatted,
        original.tables["months"].formatted
    );
}

#[test]
fn a_file_without_format_exports_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let plain = "_forge_version: \"5.1.0\"\ny:\n  price: 16\n  total:\n    formula: \"=price * 2\"\nt:\n  x: [1, 2]\n  y: \"=x * 2\"\n";
    let path = write(dir.path(), "p.yaml", plain);
    let xlsx = dir.path().join("p.xlsx");
    export_core(&path, &xlsx).unwrap();
    let back = dir.path().join("back.yaml");
    import_core(&xlsx, &back, false, false).unwrap();
    let text = std::fs::read_to_string(&back).unwrap();
    assert!(!text.contains("format"), "{text}");
    assert!(text.starts_with("_forge_version: \"5.1.0\""), "{text}");
}

#[test]
fn text_reads_the_formats_format_added() {
    let dir = tempfile::tempdir().unwrap();
    let model = r##"_forge_version: "5.1.0"
t:
  v: [1234.567, -1234.567, 0]
  negatives: "=TEXT(v, \"#,##0;(#,##0)\")"
  millions: "=TEXT(v * 1000, \"0.0,,\"\"M\"\"\")"
  aligned: "=TEXT(v, \"#,##0_);(#,##0)\")"
"##;
    let result = calculate_core(&write(dir.path(), "t.yaml", model), true, None).unwrap();
    let column = |name: &str| result.tables["t"].values[name].clone();
    assert_eq!(column("negatives"), ["1,235", "(1,235)", "0"]);
    assert_eq!(column("millions"), ["1.2M", "-1.2M", "0.0M"]);
    assert_eq!(column("aligned"), ["1,235 ", "(1,235)", "0 "]);
}
