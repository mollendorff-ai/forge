//! Tests for `@namespace.path`: formulas that read values from included files

use crate::types::ParsedModel;
use std::path::Path;

/// Write `files` into a temp directory and calculate `main.yaml`.
fn calculate(files: &[(&str, &str)]) -> Result<ParsedModel, String> {
    let dir = tempfile::TempDir::new().unwrap();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    calculate_at(&dir.path().join("main.yaml"))
}

fn calculate_at(path: &Path) -> Result<ParsedModel, String> {
    let model = crate::parser::parse_model(path).map_err(|e| e.to_string())?;
    super::ArrayCalculator::new(model)
        .calculate_all()
        .map_err(|e| e.to_string())
}

const RATES: &str = r#"
_forge_version: "5.0.0"
fx:
  usd_eur:
    value: 0.92
  eur_usd:
    value: null
    formula: "=1 / fx.usd_eur"
"#;

fn main_with(body: &str) -> String {
    format!("_forge_version: \"5.0.0\"\n_includes:\n  - file: rates.yaml\n    as: rates\n{body}")
}

#[test]
fn scalar_formula_reads_an_included_value() {
    let main = main_with(
        "s:\n  amount:\n    value: 1000\n  eur:\n    value: null\n    formula: \"=s.amount * @rates.fx.usd_eur\"\n",
    );
    let model = calculate(&[("main.yaml", &main), ("rates.yaml", RATES)]).unwrap();
    assert_eq!(model.scalars["s.eur"].value, Some(920.0));
}

#[test]
fn included_formulas_are_calculated_first() {
    // fx.eur_usd has a formula in the included file and no stored value
    let main = main_with("s:\n  r:\n    value: null\n    formula: \"=@rates.fx.eur_usd\"\n");
    let model = calculate(&[("main.yaml", &main), ("rates.yaml", RATES)]).unwrap();
    let r = model.scalars["s.r"].value.unwrap();
    assert!((r - 1.0 / 0.92).abs() < 1e-12, "{r}");
}

#[test]
fn row_formula_reads_an_included_value() {
    let main = main_with("t:\n  usd: [10, 20, 30]\n  eur: \"=usd * @rates.fx.usd_eur\"\n");
    let model = calculate(&[("main.yaml", &main), ("rates.yaml", RATES)]).unwrap();
    let crate::types::ColumnValue::Number(eur) = &model.tables["t"].columns["eur"].values else {
        panic!("eur should be numbers");
    };
    for (got, want) in eur.iter().zip([10.0 * 0.92, 20.0 * 0.92, 30.0 * 0.92]) {
        assert!((got - want).abs() < 1e-12, "{eur:?}");
    }
}

#[test]
fn nested_includes_resolve() {
    let base = "_forge_version: \"5.0.0\"\ncosts:\n  fee:\n    value: 0.005\n";
    let rates = "_forge_version: \"5.0.0\"\n_includes:\n  - file: base.yaml\n    as: base\nfx:\n  spread:\n    value: null\n    formula: \"=@base.costs.fee * 2\"\n";
    let main = main_with("s:\n  x:\n    value: null\n    formula: \"=@rates.fx.spread\"\n");
    let model = calculate(&[
        ("main.yaml", &main),
        ("rates.yaml", rates),
        ("base.yaml", base),
    ])
    .unwrap();
    assert_eq!(model.scalars["s.x"].value, Some(0.01));
}

#[test]
fn group_named_like_the_namespace_is_not_a_cycle() {
    // `rates.usd` reads `@rates.fx.usd_eur`; without the `@` kept in the word,
    // the dependency scan would read `rates...` and could find a false cycle.
    let main =
        main_with("rates:\n  usd:\n    value: null\n    formula: \"=@rates.fx.usd_eur * 100\"\n");
    let model = calculate(&[("main.yaml", &main), ("rates.yaml", RATES)]).unwrap();
    assert_eq!(model.scalars["rates.usd"].value, Some(92.0));
}

#[test]
fn unknown_included_value_says_what_is_missing() {
    let main = main_with("s:\n  x:\n    value: null\n    formula: \"=@rates.fx.nope\"\n");
    let err = calculate(&[("main.yaml", &main), ("rates.yaml", RATES)]).unwrap_err();
    assert!(
        err.contains("Unknown included value: @rates.fx.nope"),
        "{err}"
    );
    assert!(err.contains("`as: rates`"), "{err}");
}

#[test]
fn reference_without_an_include_is_an_error() {
    let main = "_forge_version: \"5.0.0\"\ns:\n  x:\n    value: null\n    formula: \"=@rates.fx.usd_eur\"\n";
    let err = calculate(&[("main.yaml", main)]).unwrap_err();
    assert!(
        err.contains("Unknown included value: @rates.fx.usd_eur"),
        "{err}"
    );
}

#[test]
fn failing_included_formula_names_the_file() {
    let bad = "_forge_version: \"5.0.0\"\nfx:\n  r:\n    value: null\n    formula: \"=1/0\"\n";
    let main = main_with("s:\n  x:\n    value: null\n    formula: \"=@rates.fx.r\"\n");
    let err = calculate(&[("main.yaml", &main), ("rates.yaml", bad)]).unwrap_err();
    assert!(
        err.contains("included file 'rates.yaml' (as rates)"),
        "{err}"
    );
    assert!(err.contains("Division by zero"), "{err}");
    assert_eq!(err.matches("Formula evaluation error").count(), 1, "{err}");
}

/// An included file with a price table, whose `with_tax` column is a formula
/// that reads one of the file's own scalars.
const PRICES: &str = r#"
_forge_version: "5.1.0"
tax:
  rate: 0.15
list:
  item: ["flour", "sesame", "salt"]
  per_kg: [1.15, 9.40, 0.80]
  with_tax: "=per_kg * (1 + tax.rate)"
"#;

fn order(body: &str) -> String {
    format!("_forge_version: \"5.1.0\"\n_includes:\n  - file: prices.yaml\n    as: prices\n{body}")
}

fn numbers(model: &ParsedModel, table: &str, column: &str) -> Vec<f64> {
    match &model.tables[table].columns[column].values {
        crate::types::ColumnValue::Number(v) => v.clone(),
        other => panic!("{table}.{column}: {other:?}"),
    }
}

/// Checked outside Forge: 120 kg x 1.15 x 1.15, 6 x 9.40 x 1.15, 3 x 0.80 x 1.15.
#[test]
fn row_formula_reads_a_column_of_an_included_table() {
    let main = order(
        "order:\n  kg: [120, 6, 3]\n  cost: \"=kg * @prices.list.with_tax\"\n  raw: \"=kg * @prices.list.per_kg\"\n",
    );
    let model = calculate(&[("main.yaml", &main), ("prices.yaml", PRICES)]).unwrap();
    let cost = numbers(&model, "order", "cost");
    for (got, want) in cost.iter().zip([158.7, 64.86, 2.76]) {
        assert!((got - want).abs() < 1e-9, "{cost:?}");
    }
    let raw = numbers(&model, "order", "raw");
    for (got, want) in raw.iter().zip([138.0, 56.4, 2.4]) {
        assert!((got - want).abs() < 1e-9, "{raw:?}");
    }
}

#[test]
fn scalar_formulas_aggregate_and_index_an_included_column() {
    let main = order(
        "total:\n  sum:\n    formula: \"=SUM(@prices.list.per_kg)\"\n  dearest:\n    formula: \"=MAX(@prices.list.with_tax)\"\n  salt:\n    formula: \"=INDEX(@prices.list.per_kg, MATCH(\\\"salt\\\", @prices.list.item, 0))\"\n",
    );
    let model = calculate(&[("main.yaml", &main), ("prices.yaml", PRICES)]).unwrap();
    assert!((model.scalars["total.sum"].value.unwrap() - 11.35).abs() < 1e-12);
    assert!((model.scalars["total.dearest"].value.unwrap() - 10.81).abs() < 1e-12);
    assert_eq!(model.scalars["total.salt"].value, Some(0.8));
}

/// A row formula pairs rows one by one: a column of another length is an
/// error that names both files and says what to do instead.
#[test]
fn an_included_column_of_another_length_names_both_files() {
    let main = order("order:\n  kg: [120, 6]\n  cost: \"=kg * @prices.list.per_kg\"\n");
    let err = calculate(&[("main.yaml", &main), ("prices.yaml", PRICES)]).unwrap_err();
    for part in [
        "order.cost",
        "@prices.list.per_kg has 3 rows (in prices.yaml)",
        "table 'order' has 2 (in main.yaml)",
        "INDEX(@prices.list.per_kg, n)",
    ] {
        assert!(err.contains(part), "missing {part}: {err}");
    }
    // A lookup reads the whole column, so the lengths do not matter there
    let main = order("order:\n  kg: [120, 6]\n  flour: \"=kg * INDEX(@prices.list.per_kg, 1)\"\n");
    let model = calculate(&[("main.yaml", &main), ("prices.yaml", PRICES)]).unwrap();
    let flour = numbers(&model, "order", "flour");
    assert!(
        (flour[0] - 138.0).abs() < 1e-9 && (flour[1] - 6.9).abs() < 1e-9,
        "{flour:?}"
    );
}

#[test]
fn an_unknown_included_column_is_an_error() {
    let main = order("order:\n  kg: [1, 2, 3]\n  cost: \"=kg * @prices.list.nope\"\n");
    let err = calculate(&[("main.yaml", &main), ("prices.yaml", PRICES)]).unwrap_err();
    assert!(
        err.contains("Unknown included value: @prices.list.nope"),
        "{err}"
    );
}
