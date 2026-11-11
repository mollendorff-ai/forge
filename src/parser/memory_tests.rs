//! A model held in memory (`parse_model_text`, ADR-055) parses and calculates
//! exactly as the same files read from disk.

use super::{parse_model, parse_model_text};
use crate::core::ArrayCalculator;
use crate::types::ParsedModel;
use std::collections::HashMap;
use std::path::PathBuf;

fn example(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(path)
}

fn text(path: &str) -> String {
    std::fs::read_to_string(example(path)).unwrap()
}

/// What two parses of one model must share: everything but where it came from.
fn same_model(from_disk: &ParsedModel, in_memory: &ParsedModel) {
    assert_eq!(
        format!("{:?}", from_disk.tables),
        format!("{:?}", in_memory.tables)
    );
    assert_eq!(
        format!("{:?}", from_disk.scalars),
        format!("{:?}", in_memory.scalars)
    );
    // Scenarios and their overrides are HashMaps, in no fixed order
    let scenarios = |m: &ParsedModel| {
        m.scenarios
            .iter()
            .map(|(name, s)| {
                let overrides = s
                    .overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_bits()))
                    .collect::<std::collections::BTreeMap<_, _>>();
                (name.clone(), overrides)
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    assert_eq!(scenarios(from_disk), scenarios(in_memory));
    let namespaces = |m: &ParsedModel| m.resolved_includes.keys().cloned().collect::<Vec<_>>();
    assert_eq!(namespaces(from_disk), namespaces(in_memory));
    for (namespace, include) in &from_disk.resolved_includes {
        let other = &in_memory.resolved_includes[namespace];
        same_model(&include.model, &other.model);
    }
}

fn calculated(model: ParsedModel) -> String {
    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    format!("{:?}{:?}", result.scalars, result.tables)
}

#[test]
fn a_model_with_an_include_calculates_as_on_disk() {
    let files = HashMap::from([("costs.yaml".to_string(), text("bagel/costs.yaml"))]);
    let in_memory = parse_model_text("bagel.yaml", &text("bagel/bagel.yaml"), &files).unwrap();
    let from_disk = parse_model(&example("bagel/bagel.yaml")).unwrap();

    assert!(in_memory.source.is_none());
    same_model(&from_disk, &in_memory);
    assert_eq!(calculated(from_disk), calculated(in_memory));
}

#[test]
fn an_include_of_an_include_is_read_from_memory_too() {
    // bagel_simulate.yaml includes costs_risk.yaml, which includes costs.yaml
    let files = HashMap::from([
        ("costs_risk.yaml".to_string(), text("bagel/costs_risk.yaml")),
        ("costs.yaml".to_string(), text("bagel/costs.yaml")),
    ]);
    let in_memory = parse_model_text(
        "bagel_simulate.yaml",
        &text("bagel/bagel_simulate.yaml"),
        &files,
    )
    .unwrap();
    let from_disk = parse_model(&example("bagel/bagel_simulate.yaml")).unwrap();

    same_model(&from_disk, &in_memory);
    assert!(in_memory.resolved_includes["costs"]
        .model
        .resolved_includes
        .contains_key("base"));
}

#[test]
fn a_model_without_includes_needs_no_files() {
    for path in [
        "mortgage/mortgage.yaml",
        "periods/monthly_plan.yaml",
        "starter.yaml",
    ] {
        let name = path.rsplit('/').next().unwrap();
        let in_memory = parse_model_text(name, &text(path), &HashMap::new()).unwrap();
        let from_disk = parse_model(&example(path)).unwrap();
        same_model(&from_disk, &in_memory);
        assert_eq!(calculated(from_disk), calculated(in_memory), "{path}");
    }
}

#[test]
fn a_missing_include_is_named_as_on_disk() {
    let error = parse_model_text("bagel.yaml", &text("bagel/bagel.yaml"), &HashMap::new())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Included file not found: costs.yaml (referenced as 'costs.yaml')"),
        "{error}"
    );
}

#[test]
fn include_paths_are_relative_to_the_including_file() {
    // models/main.yaml includes ../shared/rates.yaml, which includes fx.yaml
    // next to it: the keys are paths relative to the main model's folder
    let main = "_forge_version: \"5.1.0\"\n\
                _includes:\n  - file: ../shared/rates.yaml\n    as: rates\n\
                plan:\n  amount: 100\n  in_euros: {formula: \"=amount * @rates.fx.eur\"}\n";
    let rates = "_forge_version: \"5.1.0\"\n\
                 _includes:\n  - file: fx.yaml\n    as: market\n\
                 fx:\n  eur: {formula: \"=@market.quotes.eur\"}\n";
    let fx = "_forge_version: \"5.1.0\"\nquotes:\n  eur: 0.92\n";
    let files = HashMap::from([
        ("shared/rates.yaml".to_string(), rates.to_string()),
        ("shared/fx.yaml".to_string(), fx.to_string()),
    ]);
    let model = parse_model_text("models/main.yaml", main, &files).unwrap();
    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    let in_euros = result.scalars["plan.in_euros"].value.unwrap();
    assert!((in_euros - 92.0).abs() < 1e-9, "{in_euros}");
}

#[test]
fn a_circular_include_is_refused() {
    let a = "_forge_version: \"5.1.0\"\n_includes:\n  - file: b.yaml\n    as: b\nx:\n  v: 1\n";
    let b = "_forge_version: \"5.1.0\"\n_includes:\n  - file: a.yaml\n    as: a\ny:\n  w: 2\n";
    let files = HashMap::from([
        ("a.yaml".to_string(), a.to_string()),
        ("b.yaml".to_string(), b.to_string()),
    ]);
    let error = parse_model_text("a.yaml", a, &files)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Circular dependency detected"), "{error}");
}
