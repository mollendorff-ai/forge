//! What is wrong with a model that fails its schema, in words: the key path
//! (`loan.payment`), what is there and what is expected, instead of the
//! validator's "is not valid under any of the schemas listed in 'anyOf'"
//! followed by the whole group as JSON.

use super::schema_reader::{JsonType, Kind, SchemaError};
use serde_json::Value;
use std::fmt::Write as _;

/// At most this many problems are listed; the rest are counted.
const MAX_LISTED: usize = 10;

/// One line per problem, most specific first.
pub(super) fn describe_errors(errors: &[SchemaError<'_>], model: &Value, schema: &Value) -> String {
    let lines = problem_lines(errors, model, schema);
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

/// Every line, before the list is cut at `MAX_LISTED`.
pub(super) fn problem_lines(
    errors: &[SchemaError<'_>],
    model: &Value,
    schema: &Value,
) -> Vec<String> {
    let mut problems: Vec<Problem> = Vec::new();
    for error in errors {
        collect(error, schema, &mut problems);
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
    lines
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

fn expected_types(types: &[JsonType]) -> Vec<&'static str> {
    types.iter().copied().map(type_words).collect()
}

/// What text a pattern that is not a formula's asks for.
fn pattern_words(pattern: &str) -> String {
    match pattern {
        r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$" => "a date (YYYY-MM-DD)".to_string(),
        r"^\d{4}-\d{2}(-\d{2})?$" => "a date (YYYY-MM-DD) or a month (YYYY-MM)".to_string(),
        r"^[0-9]{4}-(0[1-9]|1[0-2])(-(0[1-9]|[12][0-9]|3[01]))?$" => {
            "a month (YYYY-MM) or a day (YYYY-MM-DD)".to_string()
        },
        other => format!("text that matches {other}"),
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
                .map_or_else(String::new, |k| format!(" Did you mean '{k}'?"));
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

fn collect(error: &SchemaError<'_>, schema: &Value, out: &mut Vec<Problem>) {
    let path = key_path(&error.instance_path);
    let instance = error.instance;
    let shown = what(instance);
    let text = match &error.kind {
        Kind::AdditionalProperties(unexpected) => {
            for text in unknown_keys(unexpected, &path, instance, &error.schema_path, schema) {
                out.push(Problem {
                    path: path.clone(),
                    text,
                    unknown_key: true,
                });
            }
            return;
        },
        Kind::AnyOf(context) | Kind::OneOfNone(context) => {
            any_of(error, context, &path, schema, out);
            return;
        },
        Kind::Type(types) => {
            let column = error.schema_path.starts_with("/definitions/TableColumn/");
            type_text(&path, expected_types(types), instance, column)
        },
        Kind::Pattern(pattern) if pattern.starts_with("^=") => {
            format!("'{path}' must be a formula starting with '=', not {shown}")
        },
        Kind::Pattern(pattern) => {
            format!("'{path}' must be {}, not {shown}", pattern_words(pattern))
        },
        Kind::Format => {
            format!("'{path}' must be a date (YYYY-MM-DD) that is on the calendar, not {shown}")
        },
        Kind::Enum(options) => {
            let options: Vec<String> = options.iter().map(Value::to_string).collect();
            let options: Vec<&str> = options.iter().map(String::as_str).collect();
            format!("'{path}' must be {}, not {shown}", or_list(&options))
        },
        Kind::Minimum(limit) => format!("'{path}' must be at least {limit}, not {shown}"),
        Kind::Maximum(limit) => format!("'{path}' must be at most {limit}, not {shown}"),
        Kind::MinItems(limit) | Kind::MinProperties(limit) => {
            format!("'{path}' needs at least {limit} entries")
        },
        Kind::MaxItems(limit) => format!("'{path}' has more than {limit} entries"),
        Kind::Required(name) => format!("'{path}' needs '{name}'"),
        Kind::OneOfMany => {
            format!("'{path}' fits more than one of the forms allowed here ({shown})")
        },
        Kind::Not => format!("'{path}' cannot be {shown} here"),
        Kind::Unreadable(problems) => format!(
            "Forge cannot read the schema of this model format, so no model of it is \
             accepted (a bug in Forge): {}",
            problems.join("; ")
        ),
    };
    out.push(Problem {
        path,
        text,
        unknown_key: false,
    });
}

/// None of several alternatives fit. Say what they all expect when they are
/// alternative types, explain a list of mixed types, or else report the
/// alternative that got furthest into the value.
fn any_of(
    error: &SchemaError<'_>,
    context: &[Vec<SchemaError<'_>>],
    path: &str,
    schema: &Value,
    out: &mut Vec<Problem>,
) {
    let instance = error.instance;
    let pointer = error.instance_path.as_str();
    // value: number or null; formula: text or null
    let types: Option<Vec<&'static str>> = context
        .iter()
        .map(|branch| match branch.as_slice() {
            [e] if e.instance_path == pointer => match &e.kind {
                Kind::Type(types) => Some(expected_types(types)),
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
    let type_only = |branch: &Vec<SchemaError<'_>>| {
        matches!(branch.as_slice(), [e] if e.instance_path == pointer
            && matches!(e.kind, Kind::Type(_)))
    };
    let fitted: Vec<&Vec<SchemaError<'_>>> = context
        .iter()
        .filter(|b| !b.is_empty() && !type_only(b))
        .collect();
    if fitted.len() == 1 && !instance.is_array() {
        for e in fitted[0] {
            collect(e, schema, out);
        }
        return;
    }
    if instance.is_array() && error.schema_path.starts_with("/definitions/TableColumn/") {
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
    let depth = |branch: &Vec<SchemaError<'_>>| {
        branch
            .iter()
            .map(|e| e.instance_path.matches('/').count())
            .max()
            .unwrap_or(0)
    };
    // (Some alternative has an error: none of them fitted)
    if let Some(best) = context
        .iter()
        .filter(|b| !b.is_empty())
        .max_by(|a, b| depth(a).cmp(&depth(b)).then(b.len().cmp(&a.len())))
        .filter(|best| depth(best) > pointer.matches('/').count())
    {
        for e in best {
            collect(e, schema, out);
        }
        return;
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
    use super::{describe_errors, key_path, or_list, problem_lines, unknown_keys, what};
    use crate::parser::schema::validate_against_schema;
    use crate::parser::schema_reader::Schema;
    use serde_json::{json, Value};
    use std::fmt::Write as _;

    /// What the errors of `model` against a schema of our own are called.
    fn described(schema: &Value, model: &Value) -> String {
        let schema = Schema::from_text(&schema.to_string());
        describe_errors(&schema.errors(model), model, &schema.doc)
    }

    #[test]
    fn a_limit_or_a_count_is_named_with_what_was_found() {
        let schema = json!({"properties": {
            "low": {"minimum": 5},
            "high": {"maximum": 9},
            "list": {"minItems": 2},
            "players": {"minProperties": 2},
        }});
        let model = json!({"low": 3, "high": 12, "list": [1], "players": {"a": 1}});
        let text = described(&schema, &model);
        for expected in [
            "  - 'low' must be at least 5, not the number 3",
            "  - 'high' must be at most 9, not the number 12",
            "  - 'list' needs at least 2 entries",
            "  - 'players' needs at least 2 entries",
        ] {
            assert!(text.lines().any(|l| l == expected), "{expected}\n{text}");
        }
        assert_eq!(text.lines().count(), 4, "{text}");
    }

    #[test]
    fn a_missing_key_and_a_forbidden_value_are_named() {
        let schema = json!({"required": ["a"], "properties": {
            "x": {"not": {"type": "string"}},
        }});
        let text = described(&schema, &json!({"x": "a"}));
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.contains(&"  - 'the model' needs 'a'"), "{text}");
        assert!(
            lines.contains(&"  - 'x' cannot be the text \"a\" here"),
            "{text}"
        );
        assert_eq!(lines.len(), 2, "{text}");
    }

    #[test]
    fn every_type_is_called_by_its_plain_name() {
        let schema = json!({"properties": {
            "list": {"type": "array"},
            "flag": {"type": "boolean"},
            "whole": {"type": "integer"},
            "none": {"type": "null"},
            "num": {"type": "number"},
            "map": {"type": "object"},
            "text": {"type": "string"},
        }});
        let model = json!({"list": 1, "flag": 1, "whole": 1.5, "none": 1, "num": "a", "map": [], "text": {}});
        let text = described(&schema, &model);
        for expected in [
            "  - 'list' must be a list, not the number 1",
            "  - 'flag' must be true or false, not the number 1",
            "  - 'whole' must be a whole number, not the number 1.5",
            "  - 'none' must be empty, not the number 1",
            "  - 'num' must be a number, not the text \"a\"",
            "  - 'map' must be a mapping, not a list",
            "  - 'text' must be text, not a mapping",
        ] {
            assert!(text.lines().any(|l| l == expected), "{expected}\n{text}");
        }
        assert_eq!(text.lines().count(), 7, "{text}");
    }

    #[test]
    fn a_value_is_described_in_a_few_words() {
        assert_eq!(what(&Value::Null), "empty");
        assert_eq!(what(&json!(true)), "true");
        assert_eq!(what(&json!(false)), "false");
        assert_eq!(what(&json!(2.5)), "the number 2.5");
        assert_eq!(what(&json!("abc")), "the text \"abc\"");
        let long = "x".repeat(41);
        assert_eq!(
            what(&json!(long)),
            format!("the text \"{}...\"", "x".repeat(40))
        );
        assert_eq!(
            what(&json!("y".repeat(40))),
            format!("the text \"{}\"", "y".repeat(40))
        );
        assert_eq!(what(&json!([1])), "a list");
        assert_eq!(what(&json!({"a": 1})), "a mapping");
    }

    #[test]
    fn a_pointer_is_a_key_path() {
        assert_eq!(key_path(""), "the model");
        assert_eq!(key_path("/loan/payment"), "loan.payment");
        assert_eq!(key_path("/months/revenue/2"), "months.revenue[2]");
        assert_eq!(key_path("/a~1b/c~0d"), "a/b.c~d");
    }

    #[test]
    fn a_list_of_words_is_joined_with_or() {
        assert_eq!(or_list(&[]), "");
        assert_eq!(or_list(&["a"]), "a");
        assert_eq!(or_list(&["a", "b"]), "a or b");
        assert_eq!(or_list(&["a", "b", "c"]), "a, b or c");
    }

    #[test]
    fn an_unknown_key_at_the_top_has_no_path_in_front_of_it() {
        let schema = json!({"properties": {"price": {}}, "additionalProperties": false});
        let text = described(&schema, &json!({"pirce": 1}));
        assert_eq!(
            text,
            "  - 'pirce': there is no key 'pirce' here. Did you mean 'price'? The keys here are price."
        );
        let none = unknown_keys(
            &["x".to_string()],
            "the model",
            &json!({}),
            "/additionalProperties",
            &json!({}),
        );
        assert_eq!(none, ["'x': there is no key 'x' here."]);
    }

    #[test]
    fn the_same_problem_is_listed_once() {
        let schema = Schema::from_text(&json!({"properties": {"n": {"minimum": 5}}}).to_string());
        let model = json!({"n": 3});
        let twice: Vec<_> = schema
            .errors(&model)
            .into_iter()
            .chain(schema.errors(&model))
            .collect();
        assert_eq!(
            describe_errors(&twice, &model, &schema.doc),
            "  - 'n' must be at least 5, not the number 3"
        );
    }

    #[test]
    fn a_column_of_mixed_values_names_each_kind_once() {
        let got = lines(
            "_forge_version: \"5.3.0\"\nt:\n  x: [1, \"=x\", true, [2], {a: 1}, null, \"y\", 3]\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].starts_with(
                "'t.x' mixes numbers, formulas, true/false, lists, mappings, empty entries and text: "
            ),
            "{got:?}"
        );
    }

    #[test]
    fn a_sound_scenario_is_not_searched_for_misspellings() {
        let got = lines(
            "_forge_version: \"5.1.0\"\na:\n  b: 1\nscenarios:\n  good:\n    probability: 0.5\n    scalars:\n      a.b: 2\nprice: \"x\"\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].starts_with("'price' must be"), "{got:?}");
    }

    #[test]
    fn a_scenario_that_is_not_a_mapping_is_left_to_the_schemas_words() {
        let got = lines("_forge_version: \"5.1.0\"\nscenarios:\n  bad: 5\n");
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].starts_with("'scenarios.bad'"), "{got:?}");
    }

    #[test]
    fn a_model_or_scenarios_that_are_not_mappings_are_named_as_they_are() {
        let text = described(&json!({"type": "object"}), &json!([1]));
        assert_eq!(text, "  - 'the model' must be a mapping, not a list");
        let got = lines("_forge_version: \"5.1.0\"\nscenarios: 5\n");
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].starts_with("'scenarios' must be"), "{got:?}");
    }

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

    /// Where `jsonschema` used its own words, Forge's (ADR-069, point 3).
    #[test]
    fn a_date_a_period_start_and_too_many_entries_are_said_in_words() {
        assert_eq!(
            lines("_forge_version: \"5.4.0\"\nopened: \"text\"\n"),
            ["'opened' must be a date (YYYY-MM-DD), not the text \"text\""]
        );
        assert_eq!(
            lines(
                "_forge_version: \"5.4.0\"\ncash:\n  year: {start: \"text\", count: 5, unit: year}\n  x: \"=1\"\n"
            ),
            ["'cash.year.start' must be a month (YYYY-MM) or a day (YYYY-MM-DD), not the text \"text\""]
        );
        assert_eq!(
            lines(
                "_forge_version: \"5.4.0\"\na: 1\nb: 2\nc: 3\nmonte_carlo:\n  iterations: 1000\n  correlations:\n    - variables: [a, b, c]\n      coefficient: 0\n  outputs:\n    - variable: a\n"
            ),
            ["'monte_carlo.correlations[0].variables' has more than 2 entries"]
        );
    }

    /// The lines for a model against a schema of its own.
    fn against(schema: &serde_json::Value, model: &serde_json::Value) -> Vec<String> {
        let schema = Schema::from_text(&schema.to_string());
        problem_lines(&schema.errors(model), model, &schema.doc)
    }

    /// The kinds the six schemas give only inside alternatives that are not
    /// reported still have words.
    #[test]
    fn every_kind_has_words() {
        let one = |schema: serde_json::Value, value: serde_json::Value| {
            against(
                &json!({"properties": {"x": schema}}),
                &json!({ "x": value }),
            )
        };
        assert_eq!(
            one(json!({"format": "date"}), json!("2025-02-30")),
            ["'x' must be a date (YYYY-MM-DD) that is on the calendar, not the text \"2025-02-30\""]
        );
        assert_eq!(
            one(json!({"pattern": r"^\d{4}-\d{2}(-\d{2})?$"}), json!("May")),
            ["'x' must be a date (YYYY-MM-DD) or a month (YYYY-MM), not the text \"May\""]
        );
        assert_eq!(
            one(json!({"pattern": "^[a-z]+$"}), json!("A")),
            ["'x' must be text that matches ^[a-z]+$, not the text \"A\""]
        );
        assert_eq!(
            one(json!({"type": "boolean"}), json!(1)),
            ["'x' must be true or false, not the number 1"]
        );
        // Two alternatives that fail alike, neither deeper than the other
        assert_eq!(
            one(
                json!({"anyOf": [{"pattern": "^a"}, {"pattern": "^b"}]}),
                json!("c")
            ),
            ["'x' is not any of the forms allowed here (the text \"c\")"]
        );
        // The same line twice is said once
        assert_eq!(
            one(
                json!({"allOf": [{"type": "string"}, {"type": "string"}]}),
                json!(1)
            ),
            ["'x' must be text, not the number 1"]
        );
        assert_eq!(
            one(
                json!({"oneOf": [{"type": "string"}, {"pattern": "^="}]}),
                json!("=a")
            ),
            ["'x' fits more than one of the forms allowed here (the text \"=a\")"]
        );
    }

    #[test]
    fn every_example_still_fits() {
        let failing: Vec<std::path::PathBuf> = walk(std::path::Path::new("examples"))
            .into_iter()
            .filter(|entry| !message(&std::fs::read_to_string(entry).unwrap()).is_empty())
            .collect();
        assert_eq!(failing, Vec::<std::path::PathBuf>::new());
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
