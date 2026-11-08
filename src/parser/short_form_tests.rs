//! Model format 5.1.0: the scalar short form (ADR-051)

use crate::parser::parse_model;
use std::io::Write;

fn parse(text: &str) -> crate::error::ForgeResult<crate::types::ParsedModel> {
    let mut f = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
    f.write_all(text.as_bytes()).unwrap();
    parse_model(f.path())
}

#[test]
fn plain_numbers_and_formula_only_scalars() {
    let m = parse(
        "_forge_version: \"5.1.0\"\ntotal_budget: 1000\nassumptions:\n  price: 16\n  rate: 0.03\nyear:\n  revenue:\n    formula: \"=assumptions.price * 100\"\n    unit: CAD\n",
    )
    .unwrap();
    assert_eq!(m.scalars["assumptions.price"].value, Some(16.0));
    assert_eq!(m.scalars["assumptions.price"].formula, None);
    assert_eq!(m.scalars["total_budget"].value, Some(1000.0));
    let revenue = &m.scalars["year.revenue"];
    assert_eq!(revenue.value, None);
    assert_eq!(revenue.formula.as_deref(), Some("=assumptions.price * 100"));
    assert_eq!(revenue.metadata.unit.as_deref(), Some("CAD"));
}

#[test]
fn short_and_long_forms_mean_the_same() {
    let short =
        parse("_forge_version: \"5.1.0\"\na:\n  x: 2\nb:\n  y:\n    formula: \"=a.x * 3\"\n")
            .unwrap();
    let long = parse(
        "_forge_version: \"5.0.0\"\na:\n  x:\n    value: 2\n    formula: null\nb:\n  y:\n    value: null\n    formula: \"=a.x * 3\"\n",
    )
    .unwrap();
    for name in ["a.x", "b.y"] {
        assert_eq!(short.scalars[name].value, long.scalars[name].value);
        assert_eq!(short.scalars[name].formula, long.scalars[name].formula);
    }
}

#[test]
fn table_with_rich_formula_column_before_data_column_is_a_table() {
    // Order used to decide: a rich formula column first made it a scalar group
    let m = parse(
        "_forge_version: \"5.1.0\"\nt:\n  y:\n    formula: \"=x * 2\"\n    unit: CAD\n  x: [1, 2]\n",
    )
    .unwrap();
    assert!(m.tables.contains_key("t"), "{:?}", m.tables.keys());
    assert!(m.scalars.is_empty(), "{:?}", m.scalars.keys());
}

#[test]
fn short_form_in_a_5_0_0_file_says_which_format_to_use() {
    let err = parse("_forge_version: \"5.0.0\"\na:\n  x: 2\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("'a.x' is a plain number"), "{err}");
    assert!(err.contains("_forge_version: \"5.1.0\""), "{err}");
}

#[test]
fn bare_formula_string_in_a_scalar_group_is_explained() {
    let err = parse("_forge_version: \"5.1.0\"\na:\n  x: 2\n  y: \"=a.x * 3\"\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("`y: {formula: \"=a.x * 3\"}`"), "{err}");
}

#[test]
fn calculate_writes_results_next_to_formulas_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    let text = "_forge_version: \"5.1.0\"\n# plan\na:\n  x: 2   # typed\nb:\n  y:\n    formula: \"=a.x * 3\"\n";
    std::fs::write(&path, text).unwrap();
    let result = crate::cli::calculate_core(&path, false, None).unwrap();
    assert!(result.file_updated);
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, format!("{text}    value: 6\n"));
    // A second run finds nothing to change
    let again = crate::cli::calculate_core(&path, false, None).unwrap();
    assert!(!again.file_updated);
}

#[test]
fn upgrade_converts_and_the_result_calculates_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    let long = "_forge_version: \"5.0.0\"\n# keep\na:\n  x:\n    value: 2\n    formula: null\nb:\n  y:\n    value: null\n    formula: \"=a.x * 3\"\n";
    std::fs::write(&path, long).unwrap();
    let before = crate::cli::calculate_core(&path, true, None).unwrap();
    crate::cli::upgrade(&path, false, "5.1.0", false).unwrap();
    let converted = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        converted,
        "_forge_version: \"5.1.0\"\n# keep\na:\n  x: 2\nb:\n  y:\n    formula: \"=a.x * 3\"\n"
    );
    let after = crate::cli::calculate_core(&path, true, None).unwrap();
    assert_eq!(before.scalars, after.scalars);
}
