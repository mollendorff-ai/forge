//! The checks that an edit of a model's text changed only what it should
//! (ADR-051): the edited text is read back and compared with the original.

use super::{ScalarPath, Written};
use yaml_serde::Value;

pub(super) fn parse(text: &str) -> Result<Value, String> {
    yaml_serde::from_str(text).map_err(|e| format!("the edited file does not parse: {e}"))
}

/// The edited text must equal the original with only these values set.
pub(super) fn verify_values(
    original: &str,
    edited: &str,
    values: &[(ScalarPath, Written)],
) -> Result<(), String> {
    let mut expected = parse(original)?;
    for (path, value) in values {
        let mut node = &mut expected;
        for part in path {
            node = node
                .get_mut(part.as_str())
                .ok_or_else(|| format!("'{}' is not in the file", path.join(".")))?;
        }
        let map = node
            .as_mapping_mut()
            .ok_or_else(|| format!("'{}' is not a mapping", path.join(".")))?;
        map.insert(Value::from("value"), value.value());
    }
    let actual = parse(edited)?;
    if same(&expected, &actual) {
        Ok(())
    } else {
        Err("writing the results would have changed more than the values".to_string())
    }
}

/// The converted text must be the original document with only scalars
/// shortened, and must parse to the same model.
pub(super) fn verify_same_model(original: &str, converted: &str) -> Result<(), String> {
    let a = parse(original)?;
    let b = parse(converted)?;
    if !same(&long_form(&a), &long_form(&b)) {
        return Err("the converted file differs from the original outside its scalars".to_string());
    }
    let a = crate::parser::parse_v1_model(&a)
        .map_err(|e| format!("the original does not parse: {e}"))?;
    let b = crate::parser::parse_v1_model(&b)
        .map_err(|e| format!("the converted file does not parse: {e}"))?;
    let scalars =
        |m: &crate::types::ParsedModel| -> Vec<(String, Option<u64>, Option<String>, String)> {
            m.scalars
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.value.map(f64::to_bits),
                        v.formula.clone(),
                        format!("{:?}", v.metadata),
                    )
                })
                .collect()
        };
    let tables = |m: &crate::types::ParsedModel| -> Vec<String> {
        m.tables
            .iter()
            .map(|(k, t)| format!("{k}:{:?}:{:?}", t.columns, t.row_formulas))
            .collect()
    };
    if scalars(&a) != scalars(&b) {
        return Err("the converted file has different scalars".to_string());
    }
    if tables(&a) != tables(&b) {
        return Err("the converted file has different tables".to_string());
    }
    Ok(())
}

/// A document with every scalar in one shape (`{value, formula}`, nulls
/// removed) and no version, so two spellings of the same model compare equal.
fn long_form(doc: &Value) -> Value {
    let Value::Mapping(root) = doc else {
        return doc.clone();
    };
    let scalar = |v: &Value| -> Value {
        match v {
            Value::Number(_) => {
                let mut m = yaml_serde::Mapping::new();
                m.insert(Value::from("value"), v.clone());
                Value::Mapping(m)
            },
            Value::Mapping(m) => {
                let mut m = m.clone();
                for k in ["value", "formula"] {
                    if m.get(k).is_some_and(Value::is_null) {
                        m.remove(k);
                    }
                }
                Value::Mapping(m)
            },
            other => other.clone(),
        }
    };
    let mut out = yaml_serde::Mapping::new();
    for (key, value) in root {
        let name = key.as_str().unwrap_or("");
        if name == "_forge_version" {
            continue;
        }
        let normalized = if super::NOT_SCALARS.contains(&name) {
            value.clone()
        } else {
            match value {
                Value::Number(_) => scalar(value),
                Value::Mapping(m) if m.contains_key("value") || m.contains_key("formula") => {
                    if crate::parser::model::is_table(m) {
                        value.clone()
                    } else {
                        scalar(value)
                    }
                },
                Value::Mapping(m) if !crate::parser::model::is_table(m) => {
                    Value::Mapping(m.iter().map(|(k, v)| (k.clone(), scalar(v))).collect())
                },
                other => other.clone(),
            }
        };
        out.insert(key.clone(), normalized);
    }
    Value::Mapping(out)
}

/// YAML values equal, with numbers compared as numbers (`16` equals `16.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64(), y.as_f64());
            x == y || x.zip(y).is_some_and(|(x, y)| x.is_nan() && y.is_nan())
        },
        (Value::Mapping(x), Value::Mapping(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        },
        (Value::Sequence(x), Value::Sequence(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(v, w)| same(v, w))
        },
        (Value::Tagged(x), Value::Tagged(y)) => x.tag == y.tag && same(&x.value, &y.value),
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(path: &str) -> Vec<(ScalarPath, Written)> {
        vec![(vec![path.to_string()], Written::Number(1.0))]
    }

    fn model(text: &str) -> Value {
        yaml_serde::from_str(text).unwrap()
    }

    #[test]
    fn a_value_is_checked_against_the_file_it_was_written_to() {
        let one = scalar("zz");
        let parse_error = verify_values("a: [", "a: 1", &[]).unwrap_err();
        assert!(
            parse_error.starts_with("the edited file does not parse"),
            "{parse_error}"
        );
        assert_eq!(
            verify_values("a:\n  formula: x\n", "a:\n  formula: x\n", &one),
            Err("'zz' is not in the file".to_string())
        );
        assert_eq!(
            verify_values("a: 5\n", "a: 5\n", &scalar("a")),
            Err("'a' is not a mapping".to_string())
        );
        let parse_error = verify_values("a: 1\n", "a: [", &[]).unwrap_err();
        assert!(
            parse_error.starts_with("the edited file does not parse"),
            "{parse_error}"
        );
        let parse_error = verify_same_model("a: [", "a: 1\n").unwrap_err();
        assert!(
            parse_error.starts_with("the edited file does not parse"),
            "{parse_error}"
        );
    }

    #[test]
    fn long_form_writes_a_number_in_a_group_as_a_mapping_and_keeps_the_rest() {
        assert_eq!(
            long_form(&model("g:\n  a: 1\n  note: hi\n")),
            model("g:\n  a:\n    value: 1\n  note: hi\n")
        );
        let table = "t:\n  value: [1, 2]\n  other: [3, 4]\n";
        assert_eq!(long_form(&model(table)), model(table));
        assert_eq!(long_form(&model("- 1\n")), model("- 1\n"));
    }

    #[test]
    fn yaml_is_the_same_when_numbers_and_tags_are_the_same() {
        assert!(same(&model("a: !t 1\n"), &model("a: !t 1.0\n")));
        assert!(!same(&model("a: !t 1\n"), &model("a: !u 1\n")));
        assert!(!same(&model("a: !t 1\n"), &model("a: !t 2\n")));
        assert!(!same(&model("[1]\n"), &model("[1, 2]\n")));
        assert!(same(&Value::from(f64::NAN), &Value::from(f64::NAN)));
        assert!(!same(&Value::from(f64::NAN), &Value::from(1.0)));
    }

    #[test]
    fn a_converted_file_that_does_not_parse_is_refused() {
        let reason = verify_same_model("a: 1\n", "a: [\n").unwrap_err();
        assert!(
            reason.starts_with("the edited file does not parse"),
            "{reason}"
        );
    }

    #[test]
    fn a_converted_file_that_differs_is_refused_by_what_differs() {
        let version = |v: &str, rest: &str| format!("_forge_version: \"{v}\"\n{rest}");
        assert_eq!(
            verify_same_model("foo: 1\n", "foo: 2\n"),
            Err("the converted file differs from the original outside its scalars".to_string())
        );
        let original = version("5.0.0", "x:\n  value: 0\n");
        let negative = version("5.1.0", "x:\n  value: -0.0\n");
        assert_eq!(
            verify_same_model(&original, &negative),
            Err("the converted file has different scalars".to_string())
        );
        let columns = |order: &str| version("5.1.0", &format!("t:\n{order}"));
        assert_eq!(
            verify_same_model(
                &columns("  x: [1, 2]\n  y: [3, 4]\n"),
                &columns("  y: [3, 4]\n  x: [1, 2]\n")
            ),
            Err("the converted file has different tables".to_string())
        );
        let unreadable =
            verify_same_model(&original, &version("9.9.9", "x:\n  value: 0\n")).unwrap_err();
        assert!(
            unreadable.starts_with("the converted file does not parse"),
            "{unreadable}"
        );
        let unreadable =
            verify_same_model(&version("9.9.9", "x:\n  value: 0\n"), &original).unwrap_err();
        assert!(
            unreadable.starts_with("the original does not parse"),
            "{unreadable}"
        );
    }
}
