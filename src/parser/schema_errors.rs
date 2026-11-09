//! What is wrong with a model that fails its schema, in words: the key path
//! (`loan.payment`), what is there and what is expected, instead of the
//! validator's "is not valid under any of the schemas listed in 'anyOf'"
//! followed by the whole group as JSON.

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::{JsonType, ValidationError};
use serde_json::Value;
use std::fmt::Write as _;

/// At most this many problems are listed; the rest are counted.
const MAX_LISTED: usize = 10;

/// One line per problem, most specific first.
pub(super) fn describe_errors<'a>(
    errors: impl Iterator<Item = ValidationError<'a>>,
    model: &Value,
    schema: &Value,
) -> String {
    let mut problems: Vec<Problem> = Vec::new();
    for error in errors {
        collect(&error, schema, &mut problems);
    }
    misspelt_keys(model, schema, &mut problems);
    // An unknown key explains the rest of what fails at the same place or
    // under it (a misspelt `formla:` also leaves the scalar without a formula)
    let unknown_at: Vec<String> = problems
        .iter()
        .filter(|p| p.unknown_key)
        .map(|p| p.path.clone())
        .collect();
    let under = |path: &str, at: &str| {
        path == at
            || path
                .strip_prefix(at)
                .is_some_and(|rest| rest.starts_with(['.', '[']))
    };
    problems.retain(|p| p.unknown_key || !unknown_at.iter().any(|u| under(&p.path, u)));
    let mut lines: Vec<String> = Vec::new();
    for p in problems {
        if !lines.contains(&p.text) {
            lines.push(p.text);
        }
    }
    let total = lines.len();
    let mut out: Vec<String> = lines
        .into_iter()
        .take(MAX_LISTED)
        .map(|l| format!("  - {l}"))
        .collect();
    if total > MAX_LISTED {
        out.push(format!("  ... and {} more", total - MAX_LISTED));
    }
    out.join("\n")
}

/// A key that the schema cannot call unknown, because there it would be the
/// name of something (a misspelt `_includs` is read as a table named so, a
/// misspelt `probabilty` makes a scenario the flat form), but that is one
/// letter or two from a key of that place. Only where something already fails.
fn misspelt_keys(model: &Value, schema: &Value, problems: &mut Vec<Problem>) {
    let fails_under = |problems: &Vec<Problem>, at: &str| {
        problems.iter().any(|p| {
            p.path == at
                || p.path
                    .strip_prefix(at)
                    .is_some_and(|r| r.starts_with(['.', '[']))
        })
    };
    let keys_of = |pointer: &str| -> Vec<String> {
        schema
            .pointer(pointer)
            .and_then(|s| s.get("properties"))
            .and_then(Value::as_object)
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut found = Vec::new();
    let sections = keys_of("");
    let sections: Vec<&str> = sections.iter().map(String::as_str).collect();
    if let Some(root) = model.as_object() {
        for key in root.keys() {
            if sections.contains(&key.as_str()) || !fails_under(problems, key) {
                continue;
            }
            if let Some(near) = crate::error::closest(key, &sections) {
                found.push((
                    key.clone(),
                    format!("'{key}': there is no section '{key}'. Did you mean '{near}'?"),
                ));
            }
        }
        let scenario_keys = keys_of("/definitions/ScenarioDefinition");
        let scenario_keys: Vec<&str> = scenario_keys.iter().map(String::as_str).collect();
        if let Some(scenarios) = root.get("scenarios").and_then(Value::as_object) {
            for (name, scenario) in scenarios {
                let at = format!("scenarios.{name}");
                let Some(scenario) = scenario.as_object() else {
                    continue;
                };
                if !fails_under(problems, &at) {
                    continue;
                }
                for key in scenario.keys() {
                    if scenario_keys.contains(&key.as_str()) {
                        continue;
                    }
                    if let Some(near) = crate::error::closest(key, &scenario_keys) {
                        found.push((
                            at.clone(),
                            format!("'{at}.{key}': a scenario has no key '{key}'. Did you mean '{near}'? The keys of a scenario are {}", scenario_keys.join(", ")),
                        ));
                    }
                }
            }
        }
    }
    for (path, text) in found {
        problems.insert(
            0,
            Problem {
                path,
                text,
                unknown_key: true,
            },
        );
    }
}

struct Problem {
    /// Where, as a key path
    path: String,
    /// The line to print
    text: String,
    /// The problem is a key the schema does not have there
    unknown_key: bool,
}

/// `/loan/payment` as `loan.payment`, `/months/revenue/2` as `months.revenue[2]`.
fn key_path(pointer: &str) -> String {
    let mut out = String::new();
    for part in pointer.split('/').skip(1) {
        let part = part.replace("~1", "/").replace("~0", "~");
        if part.parse::<usize>().is_ok() {
            let _ = write!(out, "[{part}]");
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(&part);
        }
    }
    if out.is_empty() {
        "the model".to_string()
    } else {
        out
    }
}

/// A JSON value in a few words.
fn what(value: &Value) -> String {
    match value {
        Value::Null => "empty".to_string(),
        Value::Bool(b) => format!("{b}"),
        Value::Number(n) => format!("the number {n}"),
        Value::String(s) => {
            let s = if s.chars().count() > 40 {
                format!("{}...", s.chars().take(40).collect::<String>())
            } else {
                s.clone()
            };
            format!("the text \"{s}\"")
        },
        Value::Array(_) => "a list".to_string(),
        Value::Object(_) => "a mapping".to_string(),
    }
}

const fn type_words(t: JsonType) -> &'static str {
    match t {
        JsonType::Array => "a list",
        JsonType::Boolean => "true or false",
        JsonType::Integer => "a whole number",
        JsonType::Null => "empty",
        JsonType::Number => "a number",
        JsonType::Object => "a mapping",
        JsonType::String => "text",
    }
}

fn expected_types(kind: &TypeKind) -> Vec<&'static str> {
    match kind {
        TypeKind::Single(t) => vec![type_words(*t)],
        TypeKind::Multiple(set) => set.iter().map(type_words).collect(),
    }
}

fn or_list(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// What a value of the wrong type should have been, with a hint for the two
/// usual mistakes: a bare formula where a scalar belongs, a number in a table.
fn type_text(
    path: &str,
    mut expected: Vec<&'static str>,
    instance: &Value,
    column: bool,
) -> String {
    if column {
        expected = vec!["a list", "a formula (text starting with =)", "a mapping"];
    }
    let mut text = format!(
        "'{path}' must be {}, not {}",
        or_list(&expected),
        what(instance)
    );
    if !column
        && expected.contains(&"a mapping")
        && instance.as_str().is_some_and(|f| f.starts_with('='))
    {
        text.push_str(". A formula of its own is a mapping with `formula:` (`name: {formula: \"=...\"}`); a bare formula is a table column");
    }
    if column && instance.is_number() {
        text.push_str(". A single number belongs in a group of scalars, not in a table with lists");
    }
    text
}

/// The schema object at a JSON pointer.
fn schema_at<'s>(schema: &'s Value, pointer: &str) -> Option<&'s Value> {
    schema.pointer(pointer)
}

/// The keys the schema allows where `schema_path` (ending in
/// `/additionalProperties`) points.
fn allowed_keys(schema: &Value, schema_path: &str) -> Vec<String> {
    let parent = schema_path
        .strip_suffix("/additionalProperties")
        .unwrap_or(schema_path);
    schema_at(schema, parent)
        .and_then(|s| s.get("properties"))
        .and_then(Value::as_object)
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default()
}

/// One line for each key the schema does not have where it is.
fn unknown_keys(
    unexpected: &[String],
    path: &str,
    instance: &Value,
    schema_path: &str,
    schema: &Value,
) -> Vec<String> {
    let allowed = allowed_keys(schema, schema_path);
    let allowed_refs: Vec<&str> = allowed.iter().map(String::as_str).collect();
    let keys = if allowed.is_empty() {
        String::new()
    } else {
        format!(" The keys here are {}.", allowed.join(", "))
    };
    unexpected
        .iter()
        .map(|key| {
            let hint = crate::error::closest(key, &allowed_refs)
                .map(|k| format!(" Did you mean '{k}'?"))
                .unwrap_or_default();
            let at = if path == "the model" {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            // `single: {value: 3, doubled: {formula: ...}}` is one scalar,
            // so `doubled` cannot be a second one inside it
            let nested = if schema_path.starts_with("/definitions/ScalarMapping/")
                && instance.get(key).is_some_and(Value::is_object)
            {
                format!(
                    " '{path}' has `value:` or `formula:`, so it is read as one scalar \
                     and '{key}' cannot be a scalar inside it."
                )
            } else {
                String::new()
            };
            format!("'{at}': there is no key '{key}' here.{hint}{nested}{keys}")
        })
        .collect()
}

fn collect(error: &ValidationError<'_>, schema: &Value, out: &mut Vec<Problem>) {
    let pointer = error.instance_path().to_string();
    let path = key_path(&pointer);
    let instance = error.instance();
    let schema_path = error.schema_path().to_string();
    let push = |out: &mut Vec<Problem>, text: String, unknown_key: bool| {
        out.push(Problem {
            path: path.clone(),
            text,
            unknown_key,
        });
    };
    match error.kind() {
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            for text in unknown_keys(unexpected, &path, instance, &schema_path, schema) {
                push(out, text, true);
            }
        },
        ValidationErrorKind::Type { kind } => {
            let column = schema_path.starts_with("/definitions/TableColumn/");
            push(
                out,
                type_text(&path, expected_types(kind), instance, column),
                false,
            );
        },
        ValidationErrorKind::Pattern { pattern } if pattern.starts_with("^=") => {
            push(
                out,
                format!(
                    "'{path}' must be a formula starting with '=', not {}",
                    what(instance)
                ),
                false,
            );
        },
        ValidationErrorKind::Enum { options } => {
            let options: Vec<String> = options
                .as_array()
                .map(|a| a.iter().map(std::string::ToString::to_string).collect())
                .unwrap_or_default();
            let options: Vec<&str> = options.iter().map(String::as_str).collect();
            push(
                out,
                format!(
                    "'{path}' must be {}, not {}",
                    or_list(&options),
                    what(instance)
                ),
                false,
            );
        },
        ValidationErrorKind::Minimum { limit }
        | ValidationErrorKind::ExclusiveMinimum { limit } => {
            push(
                out,
                format!("'{path}' must be at least {limit}, not {}", what(instance)),
                false,
            );
        },
        ValidationErrorKind::Maximum { limit }
        | ValidationErrorKind::ExclusiveMaximum { limit } => {
            push(
                out,
                format!("'{path}' must be at most {limit}, not {}", what(instance)),
                false,
            );
        },
        ValidationErrorKind::MinItems { limit } => {
            push(
                out,
                format!("'{path}' needs at least {limit} entries"),
                false,
            );
        },
        ValidationErrorKind::Required { property } => {
            let name = property
                .as_str()
                .map_or_else(|| property.to_string(), str::to_string);
            push(out, format!("'{path}' needs '{name}'"), false);
        },
        ValidationErrorKind::AnyOf { context } | ValidationErrorKind::OneOfNotValid { context } => {
            any_of(error, context, &path, schema, out);
        },
        ValidationErrorKind::Not { .. } => {
            push(
                out,
                format!("'{path}' cannot be {} here", what(instance)),
                false,
            );
        },
        _ => push(out, format!("'{path}': {}", error.masked()), false),
    }
}

/// None of several alternatives fit. Say what they all expect when they are
/// alternative types, explain a list of mixed types, or else report the
/// alternative that got furthest into the value.
fn any_of(
    error: &ValidationError<'_>,
    context: &[Vec<ValidationError<'static>>],
    path: &str,
    schema: &Value,
    out: &mut Vec<Problem>,
) {
    let instance = error.instance();
    let pointer = error.instance_path().to_string();
    // value: number or null; formula: text or null
    let types: Option<Vec<&'static str>> = context
        .iter()
        .map(|branch| match branch.as_slice() {
            [e] if e.instance_path().to_string() == pointer => match e.kind() {
                ValidationErrorKind::Type { kind } => Some(expected_types(kind)),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|v| v.concat());
    if let Some(mut types) = types {
        types.dedup();
        out.push(Problem {
            path: path.to_string(),
            text: type_text(path, types, instance, false),
            unknown_key: false,
        });
        return;
    }
    // Alternatives whose type fitted say what is wrong (a formula without
    // its =); the others only say the type is not theirs
    let type_only = |branch: &Vec<ValidationError<'static>>| {
        matches!(branch.as_slice(), [e] if e.instance_path().to_string() == pointer
            && matches!(e.kind(), ValidationErrorKind::Type { .. }))
    };
    let fitted: Vec<&Vec<ValidationError<'static>>> = context
        .iter()
        .filter(|b| !b.is_empty() && !type_only(b))
        .collect();
    if fitted.len() == 1 && !instance.is_array() {
        for e in fitted[0] {
            collect(e, schema, out);
        }
        return;
    }
    if instance.is_array()
        && error
            .schema_path()
            .to_string()
            .starts_with("/definitions/TableColumn/")
    {
        let kinds: Vec<&str> = instance
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| match v {
                Value::Number(_) => "numbers",
                Value::String(s) if s.starts_with('=') => "formulas",
                Value::String(_) => "text",
                Value::Bool(_) => "true/false",
                Value::Null => "empty entries",
                Value::Array(_) => "lists",
                Value::Object(_) => "mappings",
            })
            .fold(Vec::new(), |mut acc, k| {
                if !acc.contains(&k) {
                    acc.push(k);
                }
                acc
            });
        out.push(Problem {
            path: path.to_string(),
            text: format!(
                "'{path}' mixes {}: a column's values must all be numbers, all text, all dates (\"2026-01-31\"), all true/false, or all formulas",
                or_list(&kinds).replace(" or ", " and ")
            ),
            unknown_key: false,
        });
        return;
    }
    // The alternative whose errors reach deepest into the value, then the
    // one with the fewest errors
    let depth = |branch: &Vec<ValidationError<'static>>| {
        branch
            .iter()
            .map(|e| e.instance_path().to_string().matches('/').count())
            .max()
            .unwrap_or(0)
    };
    if let Some(best) = context
        .iter()
        .filter(|b| !b.is_empty())
        .max_by(|a, b| depth(a).cmp(&depth(b)).then(b.len().cmp(&a.len())))
    {
        if depth(best) > pointer.matches('/').count() {
            for e in best {
                collect(e, schema, out);
            }
            return;
        }
    }
    out.push(Problem {
        path: path.to_string(),
        text: format!(
            "'{path}' is not any of the forms allowed here ({})",
            what(instance)
        ),
        unknown_key: false,
    });
}

#[cfg(test)]
mod tests {
    use crate::parser::schema::validate_against_schema;
    use std::fmt::Write as _;

    /// The message for a model, or "" if it fits its schema.
    fn message(model: &str) -> String {
        let yaml: yaml_serde::Value = yaml_serde::from_str(model).unwrap();
        validate_against_schema(&yaml).map_or_else(|e| e.to_string(), |()| String::new())
    }

    fn lines(model: &str) -> Vec<String> {
        message(model)
            .lines()
            .filter_map(|l| l.strip_prefix("  - ").map(str::to_string))
            .collect()
    }

    #[test]
    fn a_misspelt_key_is_named_with_the_key_it_is_close_to() {
        let got = lines(
            "_forge_version: \"5.1.0\"\nloan:\n  principal: 450000\n  payment:\n    formla: \"=principal * 2\"\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].starts_with(
                "'loan.payment.formla': there is no key 'formla' here. Did you mean 'formula'?"
            ),
            "{got:?}"
        );
        // The same in 5.0.0, whose schema still tries every form in turn
        let got = lines(
            "_forge_version: \"5.0.0\"\nloan:\n  principal:\n    value: 1\n    formula: null\n  payment:\n    formla: \"=principal * 2\"\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].contains("'loan.payment.formla'") && got[0].contains("Did you mean 'formula'?")
        );
    }

    #[test]
    fn each_usual_mistake_says_what_is_expected() {
        let model = "_forge_version: \"5.1.0\"
price: \"16\"
g:
  a:
    value: 1
    unti: CAD
  b:
    formula: \"a * 2\"
  c:
    value: \"high\"
    better: down
t:
  revenue: [100, \"Q2\", 150]
  x: \"SUM(revenue)\"
u:
  quarter: [1, 2]
  rate: 0.05
monte_carlo:
  iterations: 5
  outputs:
    - variable: g.a
";
        let got = lines(model);
        for expected in [
            "'price' must be a number or a mapping, not the text \"16\"",
            "'g.a.unti': there is no key 'unti' here. Did you mean 'unit'?",
            "'g.b.formula' must be a formula starting with '=', not the text \"a * 2\"",
            "'g.c.value' must be a number or empty, not the text \"high\"",
            "'g.c.better' must be \"higher\" or \"lower\", not the text \"down\"",
            "'t.revenue' mixes numbers and text: a column's values must all be numbers",
            "'t.x' must be a formula starting with '=', not the text \"SUM(revenue)\"",
            "'u.rate' must be a list, a formula (text starting with =) or a mapping, not the number 0.05. A single number belongs in a group of scalars",
            "'monte_carlo.iterations' must be at least 100, not the number 5",
        ] {
            assert!(got.iter().any(|l| l.starts_with(expected)), "missing {expected}\n{got:#?}");
        }
        assert_eq!(got.len(), 9, "{got:#?}");
        // Never the validator's words or the model as JSON
        let text = message(model);
        assert!(!text.contains("anyOf") && !text.contains("{\""), "{text}");
    }

    #[test]
    fn a_misspelt_section_or_scenario_key_is_named() {
        let got = lines(
            "_forge_version: \"5.1.0\"\n_includs:\n  - file: x.yaml\n    as: x\na:\n  b: 1\nscenarios:\n  up:\n    probabilty: 0.5\n    scalars:\n      a.b: 2\n",
        );
        assert_eq!(
            got,
            [
                "'scenarios.up.probabilty': a scenario has no key 'probabilty'. Did you mean 'probability'? The keys of a scenario are description, probability, scalars",
                "'_includs': there is no section '_includs'. Did you mean '_includes'?",
            ]
        );
    }

    /// The old schema let these through, and the parser then dropped them
    /// without a word: `max_amount` had no scalar, `doubled` no formula.
    #[test]
    fn what_the_parser_would_drop_is_an_error() {
        let got = lines("_forge_version: \"5.1.0\"\nmax_amount:\n  formula: 3\n");
        assert_eq!(
            got,
            ["'max_amount.formula' must be text or empty, not the number 3"]
        );
        let got = lines(
            "_forge_version: \"5.1.0\"\nsingle:\n  value: 3\n  doubled:\n    formula: \"=value * 2\"\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].contains("'single' has `value:` or `formula:`, so it is read as one scalar"),
            "{got:?}"
        );
    }

    #[test]
    fn a_bare_formula_where_a_scalar_belongs_says_how_to_write_it() {
        let got = lines("_forge_version: \"5.0.0\"\ntotal: \"=1 + 2\"\n");
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].contains("`name: {formula: \"=...\"}`"), "{got:?}");
    }

    #[test]
    fn many_problems_are_counted_after_ten() {
        let mut model = String::from("_forge_version: \"5.1.0\"\ng:\n");
        for i in 0..14 {
            let _ = write!(model, "  s{i}:\n    value: \"x\"\n");
        }
        let text = message(&model);
        assert_eq!(text.lines().filter(|l| l.starts_with("  - ")).count(), 10);
        assert!(text.ends_with("  ... and 4 more"), "{text}");
    }

    #[test]
    fn every_example_still_fits() {
        for entry in walk(std::path::Path::new("examples")) {
            let text = std::fs::read_to_string(&entry).unwrap();
            assert_eq!(message(&text), "", "{}", entry.display());
        }
    }

    fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "yaml") {
                out.push(p);
            }
        }
        out
    }
}
