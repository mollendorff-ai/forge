//! The model read back after a change (ADR-075, decision 3): the change is
//! kept only when Forge reads the model valid, and reads in it that change
//! and nothing else, as Forge Studio checked each change it made (S9).

use super::{Change, Kind, Op, Target, Value};
use crate::core::ArrayCalculator;
use crate::parser::parse_model_text;
use crate::types::{ParsedModel, Variable};
use serde_json::Value as Json;
use std::collections::HashMap;

/// The model in `files`, `main` and the files it includes
pub(super) fn read(files: &HashMap<String, String>, main: &str) -> Result<ParsedModel, String> {
    let text = files
        .get(main)
        .ok_or_else(|| format!("'main' names {main}, which is not among the files"))?;
    parse_model_text(main, text, files).map_err(|e| e.to_string())
}

/// Whether Forge calculates the model: as `validate` does, and an uncertain
/// model at plan, each `MC.*` input at its plan figure (ADR-076)
pub(super) fn valid(model: &ParsedModel) -> Result<(), String> {
    model.check_table_lengths()?;
    let mut model = model.clone();
    for (name, input) in crate::monte_carlo::engine::plan_of(&model)? {
        model
            .set_input(&name, input.value)
            .expect("the engine read the input from this model");
    }
    ArrayCalculator::new(model)
        .calculate_all()
        .map(drop)
        .map_err(|e| e.to_string())
}

/// Whether `after` is `before` with `change` and nothing else
pub(super) fn only(
    before: &ParsedModel,
    after: &ParsedModel,
    target: &Target<'_>,
    kind: Kind,
    change: &Change,
) -> Result<(), String> {
    let mut at: Vec<&str> = Vec::new();
    for alias in &target.aliases {
        at.extend(["includes", alias.as_str()]);
    }
    at.push("model");
    let name = target.name.as_str();
    let scalar = |field: Option<&'static str>| {
        let mut path = at.clone();
        path.extend(["scalars", name]);
        path.extend(field);
        vec![path]
    };
    let column = |fields: &[&'static str]| {
        let (table, column) = name
            .rsplit_once('.')
            .expect("a column is named with its table");
        fields
            .iter()
            .map(|field| {
                let mut path = at.clone();
                path.extend(["tables", table, field, column]);
                path
            })
            .collect::<Vec<_>>()
    };
    let sub = target.aliases.iter().try_fold(after, |model, alias| {
        model.resolved_includes.get(alias).map(|r| &r.model)
    });
    let mut was = snapshot(before);
    let mut now = snapshot(after);
    match (change.op, kind) {
        (Op::SetValue, _) => copy(&mut was, &now, &scalar(Some("value"))),
        (Op::SetFormula, Kind::Formula) => copy(&mut was, &now, &scalar(Some("formula"))),
        (Op::SetFormula, _) => copy(&mut was, &now, &column(&["row_formulas"])),
        (Op::Add, _) if sub.is_some_and(|m| m.scalars.contains_key(name)) => {
            remove(&mut now, &scalar(None));
        },
        (Op::Delete, Kind::Input | Kind::Formula) => remove(&mut was, &scalar(None)),
        // A column added or deleted (a rename is checked by its own)
        _ => {
            let entry = column(&["columns", "row_formulas", "formula_metadata"]);
            remove(
                if change.op == Op::Add {
                    &mut now
                } else {
                    &mut was
                },
                &entry,
            );
        },
    }
    if was != now {
        return Err("Forge reads the new text as more than this change, so it was not made".into());
    }
    let sub = sub.expect("the model is the same, with the same includes");
    if as_asked(sub, name, change) {
        Ok(())
    } else {
        Err(format!(
            "Forge reads {name} in the new text differently from what was asked, so it was not made"
        ))
    }
}

/// Whether Forge reads at `name` what `change` asked for
fn as_asked(model: &ParsedModel, name: &str, change: &Change) -> bool {
    let var = model.scalars.get(name);
    let column = || {
        let (table, column) = name.rsplit_once('.')?;
        model.tables.get(table)?.row_formulas.get(column)
    };
    let reads = |v: &Variable| match &change.value {
        Some(Value::Number(n)) => v.value == Some(*n),
        _ => v.value.is_some(),
    };
    match change.op {
        Op::Delete | Op::Rename => true,
        Op::SetValue => var.is_some_and(reads),
        Op::SetFormula => {
            var.and_then(|v| v.formula.as_ref()).or_else(column) == change.formula.as_ref()
        },
        Op::Add => var.map_or_else(
            || column() == change.formula.as_ref(),
            |v| {
                v.formula == change.formula
                    && v.metadata.unit == change.unit
                    && (change.value.is_none() || reads(v))
            },
        ),
    }
}

/// A model and the models it includes, by alias, as JSON
fn snapshot(model: &ParsedModel) -> Json {
    let includes: serde_json::Map<String, Json> = model
        .resolved_includes
        .iter()
        .map(|(alias, included)| (alias.clone(), snapshot(&included.model)))
        .collect();
    serde_json::json!({"model": model, "includes": includes})
}

/// The value at `path`, followed key by key
fn at<'j>(json: &'j mut Json, path: &[&str]) -> Option<&'j mut Json> {
    path.iter().try_fold(json, |j, key| j.get_mut(*key))
}

/// Each of `paths` in `to` set to what it is in `from`
fn copy(to: &mut Json, from: &Json, paths: &[Vec<&str>]) {
    for path in paths {
        let value = path.iter().try_fold(from, |j, key| j.get(*key)).cloned();
        if let (Some(slot), Some(value)) = (at(to, path), value) {
            *slot = value;
        }
    }
}

/// Each of `paths` removed from `json`
fn remove(json: &mut Json, paths: &[Vec<&str>]) {
    for path in paths {
        let (key, parent) = path.split_last().expect("a path names a key");
        if let Some(map) = at(json, parent).and_then(Json::as_object_mut) {
            map.remove(*key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{locate, Change, Op, Value};
    use super::*;

    fn model(text: &str) -> Result<ParsedModel, String> {
        read(
            &HashMap::from([("m.yaml".to_string(), text.to_string())]),
            "m.yaml",
        )
    }

    #[test]
    fn a_model_forge_cannot_calculate_is_not_valid() {
        let unequal = model("_forge_version: \"5.1.0\"\nt:\n  a: [1, 2]\n  b: [1]\n").unwrap();
        assert_eq!(
            valid(&unequal),
            Err("Table 't': Column 'b' has 1 rows, expected 2 rows".to_string())
        );
        let distribution =
            model("_forge_version: \"5.1.0\"\nx:\n  formula: \"=MC.Normal(1)\"\n").unwrap();
        assert!(valid(&distribution).unwrap_err().contains("MC.Normal"));
        // An uncertain model is calculated at plan
        let uncertain = model("_forge_version: \"5.1.0\"\nx:\n  formula: \"=MC.Normal(10, 1)\"\ny:\n  formula: \"=x * 2\"\n").unwrap();
        assert_eq!(valid(&uncertain), Ok(()));
        assert!(model("a: [\n").unwrap_err().contains("YAML"));
    }

    /// `only` for a change of `a` from `before` to `after`
    fn only_of(before: &str, after: &str, change: &Change) -> Result<(), String> {
        let before = model(before).unwrap();
        let after = model(after).unwrap();
        let target = locate(&before, "m.yaml", &change.path).unwrap();
        only(&before, &after, &target, target.kind(), change)
    }

    #[test]
    fn a_text_forge_reads_as_more_or_other_than_the_change_is_refused() {
        let set = Change {
            op: Op::SetValue,
            path: "a".into(),
            value: Some(Value::Number(3.0)),
            formula: None,
            unit: None,
            after: None,
            new_name: None,
        };
        let before = "_forge_version: \"5.1.0\"\na: 1\nb: 2\n";
        assert_eq!(
            only_of(before, &before.replace("a: 1", "a: 3"), &set),
            Ok(())
        );
        assert_eq!(
            only_of(before, &before.replace("a: 1\nb: 2", "a: 3\nb: 4"), &set),
            Err(
                "Forge reads the new text as more than this change, so it was not made".to_string()
            )
        );
        assert_eq!(
            only_of(before, &before.replace("a: 1", "a: 4"), &set),
            Err(
                "Forge reads a in the new text differently from what was asked, so it was not made"
                    .to_string()
            )
        );
    }
}
