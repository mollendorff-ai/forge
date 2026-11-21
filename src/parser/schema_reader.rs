//! The model formats' JSON schemas, checked by the `jsonschema` crate
//! (ADR-072, which returned to it after ADR-069 had replaced it with a reader
//! of Forge's own).
//!
//! The seven schema files in `schema/` are the statement of each model
//! format. `jsonschema` compiles each once and checks a model against it on
//! the command line, in the MCP server, the REST API and the browser. Its
//! errors are turned into the few kinds `schema_errors` has words for, so
//! every surface says the same thing in Forge's words.

use serde_json::{Number, Value};
use std::sync::OnceLock;

/// The model formats and their schemas, built in.
const FORMATS: [(&str, &str); 7] = [
    (
        "1.0.0",
        include_str!("../../schema/forge-v1.0.0.schema.json"),
    ),
    (
        "5.0.0",
        include_str!("../../schema/forge-v5.0.0.schema.json"),
    ),
    (
        "5.1.0",
        include_str!("../../schema/forge-v5.1.0.schema.json"),
    ),
    (
        "5.2.0",
        include_str!("../../schema/forge-v5.2.0.schema.json"),
    ),
    (
        "5.3.0",
        include_str!("../../schema/forge-v5.3.0.schema.json"),
    ),
    (
        "5.4.0",
        include_str!("../../schema/forge-v5.4.0.schema.json"),
    ),
    (
        "5.5.0",
        include_str!("../../schema/forge-v5.5.0.schema.json"),
    ),
];

/// The schema of a model format, compiled once; `None` for a format Forge
/// does not have.
pub(super) fn for_format(version: &str) -> Option<&'static Schema> {
    static COMPILED: [OnceLock<Schema>; 7] = [const { OnceLock::new() }; 7];
    let index = FORMATS.iter().position(|(v, _)| *v == version)?;
    Some(COMPILED[index].get_or_init(|| Schema::from_text(FORMATS[index].1)))
}

/// A JSON type, in the order `jsonschema` lists several.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum JsonType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}

impl JsonType {
    const fn from_jsonschema(t: jsonschema::JsonType) -> Self {
        match t {
            jsonschema::JsonType::Array => Self::Array,
            jsonschema::JsonType::Boolean => Self::Boolean,
            jsonschema::JsonType::Integer => Self::Integer,
            jsonschema::JsonType::Null => Self::Null,
            jsonschema::JsonType::Number => Self::Number,
            jsonschema::JsonType::Object => Self::Object,
            jsonschema::JsonType::String => Self::String,
        }
    }
}

/// What does not fit, and where.
#[derive(Debug)]
pub(super) struct SchemaError<'v> {
    /// The value that does not fit
    pub instance: &'v Value,
    /// Where it is in the model, as a JSON pointer (`/loan/payment`)
    pub instance_path: String,
    /// The keyword it fails, as a JSON pointer into the schema with `$ref`
    /// followed (`/definitions/ScalarMapping/additionalProperties`)
    pub schema_path: String,
    pub kind: Kind<'v>,
}

/// The ways a value can fail its schema, as `schema_errors` words them.
#[derive(Debug)]
pub(super) enum Kind<'v> {
    /// Keys the mapping cannot have
    AdditionalProperties(Vec<String>),
    /// The types it could have been
    Type(Vec<JsonType>),
    /// The values it could have been
    Enum(Vec<Value>),
    Minimum(Number),
    Maximum(Number),
    MinItems(usize),
    MaxItems(usize),
    /// A mapping with fewer entries than it needs
    MinProperties(usize),
    /// A key the mapping must have
    Required(String),
    /// The pattern, as the schema writes it
    Pattern(String),
    /// Not a date (`format: date`, the only format the schemas use)
    Format,
    /// None of the alternatives fits; the errors of each
    AnyOf(Vec<Vec<SchemaError<'v>>>),
    /// None of the alternatives of a `oneOf` fits; the errors of each
    OneOfNone(Vec<Vec<SchemaError<'v>>>),
    /// More than one alternative of a `oneOf` fits
    OneOfMany,
    /// It fits what it must not
    Not,
    /// The schema cannot be used, or gives an error Forge has no words for
    /// (a keyword no schema of Forge's uses): a bug in Forge
    Unreadable(Vec<String>),
}

/// A schema: its text as JSON, for the names of keys in messages, and
/// `jsonschema`'s validator for it, or why there is none.
#[derive(Debug)]
pub(super) struct Schema {
    pub doc: Value,
    validator: Result<jsonschema::Validator, String>,
}

impl Schema {
    /// The schema in `text`, read as JSON Schema draft 07, the draft every
    /// schema file declares (in which `format: date` is a rule, not a note).
    /// A schema that is not JSON, or that `jsonschema` cannot compile,
    /// refuses every model and says why.
    pub(super) fn from_text(text: &str) -> Self {
        match serde_json::from_str::<Value>(text) {
            Ok(doc) => {
                let validator = jsonschema::draft7::new(&doc).map_err(|e| e.to_string());
                Self { doc, validator }
            },
            Err(e) => Self {
                doc: Value::Null,
                validator: Err(format!("not JSON ({e})")),
            },
        }
    }

    /// Everything in `model` that does not fit, in `jsonschema`'s order.
    pub(super) fn errors<'v>(&self, model: &'v Value) -> Vec<SchemaError<'v>> {
        match &self.validator {
            Ok(validator) => validator
                .iter_errors(model)
                .map(|error| convert(&error, model))
                .collect(),
            Err(why) => vec![SchemaError {
                instance: model,
                instance_path: String::new(),
                schema_path: String::new(),
                kind: Kind::Unreadable(vec![why.clone()]),
            }],
        }
    }
}

/// An error of `jsonschema`'s as one of the kinds `schema_errors` words.
fn convert<'v>(error: &jsonschema::ValidationError<'_>, model: &'v Value) -> SchemaError<'v> {
    use jsonschema::error::{TypeKind, ValidationErrorKind as Their};
    let instance_path = error.instance_path().to_string();
    let branches = |context: &[Vec<jsonschema::ValidationError<'_>>]| {
        context
            .iter()
            .map(|branch| branch.iter().map(|e| convert(e, model)).collect())
            .collect()
    };
    let count = |limit: u64| usize::try_from(limit).unwrap_or(usize::MAX);
    let kind = match error.kind() {
        Their::AdditionalProperties { unexpected } => {
            Kind::AdditionalProperties(unexpected.clone())
        },
        Their::Type {
            kind: TypeKind::Single(t),
        } => Kind::Type(vec![JsonType::from_jsonschema(*t)]),
        Their::Type {
            kind: TypeKind::Multiple(set),
        } => Kind::Type(set.iter().map(JsonType::from_jsonschema).collect()),
        Their::Enum { options } => Kind::Enum(options.as_array().cloned().unwrap_or_default()),
        Their::Minimum {
            limit: Value::Number(limit),
        } => Kind::Minimum(limit.clone()),
        Their::Maximum {
            limit: Value::Number(limit),
        } => Kind::Maximum(limit.clone()),
        Their::MinItems { limit } => Kind::MinItems(count(*limit)),
        Their::MaxItems { limit } => Kind::MaxItems(count(*limit)),
        Their::MinProperties { limit } => Kind::MinProperties(count(*limit)),
        Their::Required {
            property: Value::String(property),
        } => Kind::Required(property.clone()),
        Their::Pattern { pattern } => Kind::Pattern(pattern.clone()),
        Their::Format { .. } => Kind::Format,
        Their::AnyOf { context } => Kind::AnyOf(branches(context)),
        Their::OneOfNotValid { context } => Kind::OneOfNone(branches(context)),
        Their::OneOfMultipleValid { .. } => Kind::OneOfMany,
        Their::Not { .. } => Kind::Not,
        other => Kind::Unreadable(vec![format!("no words for {other:?}")]),
    };
    SchemaError {
        instance: model.pointer(&instance_path).unwrap_or(model),
        instance_path,
        schema_path: error.schema_path().to_string(),
        kind,
    }
}

/// A key as a step of a JSON pointer (`~` and `/` escaped).
#[cfg(test)]
fn step(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::schema_errors::problem_lines;
    use serde_json::json;

    #[test]
    fn every_format_s_schema_compiles() {
        for (version, _) in FORMATS {
            let schema = for_format(version).unwrap();
            assert!(
                schema.validator.is_ok(),
                "{version}: {:?}",
                schema.validator
            );
        }
        assert!(for_format("4.0.0").is_none());
    }

    #[test]
    fn a_schema_that_cannot_be_used_refuses_every_model_and_says_why() {
        let model = json!({"a": 1});
        let not_json = Schema::from_text("not json");
        assert!(matches!(
            &not_json.errors(&model)[0].kind,
            Kind::Unreadable(why) if why[0].starts_with("not JSON (")
        ));
        let bad_pattern = Schema::from_text(r#"{"pattern": "("}"#);
        let kind = format!("{:?}", bad_pattern.errors(&model)[0].kind);
        assert!(kind.starts_with("Unreadable("), "{kind}");
    }

    #[test]
    fn an_error_forge_has_no_words_for_is_a_bug_in_forge() {
        let schema = Schema::from_text(r#"{"properties": {"a": {"const": 2}}}"#);
        let model = json!({"a": 1});
        let errors = schema.errors(&model);
        assert_eq!(errors[0].instance_path, "/a");
        assert_eq!(errors[0].instance, &json!(1));
        assert!(matches!(
            &errors[0].kind,
            Kind::Unreadable(why) if why[0].starts_with("no words for Constant")
        ));
    }

    #[test]
    fn several_types_are_listed() {
        let schema = Schema::from_text(r#"{"type": ["string", "null"]}"#);
        let model = json!(1);
        assert!(matches!(
            &schema.errors(&model)[0].kind,
            Kind::Type(types) if *types == [JsonType::Null, JsonType::String]
        ));
        let every = Schema::from_text(
            r#"{"type": ["object", "string", "null", "number", "integer", "boolean", "array"]}"#,
        );
        assert!(every.errors(&model).is_empty());
        let boolean = Schema::from_text(r#"{"type": "boolean"}"#);
        assert!(matches!(
            &boolean.errors(&model)[0].kind,
            Kind::Type(types) if *types == [JsonType::Boolean]
        ));
    }

    /// Names in any language, as Forge's names are (CLAUDE.md).
    #[test]
    fn names_in_any_language_fit() {
        let schema = for_format("5.1.0").unwrap();
        for model in [
            json!({"_forge_version": "5.1.0", "hypothèses": {"coût_unitaire": 3}}),
            json!({"_forge_version": "5.1.0", "假设": {"单价": 3}}),
        ] {
            assert!(schema.errors(&model).is_empty(), "{model}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Generated models (ADR-069, point 4): every example as each format, and
    // bad models made from each by fixed changes at every place in it
    // ─────────────────────────────────────────────────────────────────────

    /// What a value at one place is changed into, each with its name.
    fn changes(value: &Value) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = [
            json!(null),
            json!(true),
            json!(3),
            json!("text"),
            json!("=a * 2"),
            json!("a * 2"),
            json!("2025-01"),
            json!("2025-02-30"),
            json!("2025-01-31"),
            json!([]),
            json!([1, "x"]),
            json!({}),
        ]
        .into_iter()
        .map(|r| (format!("= {r}"), r))
        .collect();
        match value {
            // Each side of every minimum and maximum in the schemas
            Value::Number(_) => {
                for n in [-2, -1, 0, 1, 2, 99, 1899, 1900, 9999, 10000, 1_048_576] {
                    out.push((format!("= {n}"), json!(n)));
                }
                for x in [0.5, 1.5] {
                    out.push((format!("= {x}"), json!(x)));
                }
            },
            Value::Array(items) => {
                for extra in [json!(null), json!("x"), json!(1), json!(true)] {
                    let mut longer = items.clone();
                    longer.push(extra.clone());
                    out.push((format!("push {extra}"), Value::Array(longer)));
                }
                let rest = items.iter().skip(1).cloned().collect();
                out.push(("drop first".into(), Value::Array(rest)));
            },
            Value::Object(map) => {
                let mut more = map.clone();
                more.insert("extra".into(), json!(1));
                out.push(("add extra".into(), Value::Object(more)));
                for key in map.keys() {
                    let mut less = map.clone();
                    let moved = less.remove(key).unwrap();
                    out.push((format!("drop {key}"), Value::Object(less.clone())));
                    let count = key.chars().count();
                    let typo: String = if count > 2 {
                        key.chars().take(count - 1).collect()
                    } else {
                        format!("{key}x")
                    };
                    less.insert(typo.clone(), moved);
                    out.push((format!("rename {key} {typo}"), Value::Object(less)));
                }
            },
            _ => {},
        }
        out
    }

    fn change_everywhere(
        root: &Value,
        at: &str,
        value: &Value,
        label: &str,
        out: &mut Vec<(String, Value)>,
    ) {
        for (what, changed) in changes(value) {
            let mut model = root.clone();
            *model.pointer_mut(at).unwrap() = changed;
            out.push((format!("{label} {at} {what}"), model));
        }
        let children: Vec<(String, &Value)> = match value {
            Value::Object(map) => map.iter().map(|(k, v)| (step(k), v)).collect(),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v))
                .collect(),
            _ => Vec::new(),
        };
        for (step, child) in children {
            change_everywhere(root, &format!("{at}/{step}"), child, label, out);
        }
    }

    /// The models: each example as it is and as each format, then changed.
    fn corpus() -> Vec<(String, Value)> {
        let mut files = Vec::new();
        let mut dirs = vec![std::path::PathBuf::from("examples")];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|x| x == "yaml") {
                    files.push(path);
                }
            }
        }
        files.sort();
        let mut out = Vec::new();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            let yaml: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
            let root = serde_json::to_value(&yaml).unwrap();
            let label = file.display().to_string();
            for (version, _) in FORMATS {
                let mut model = root.clone();
                model["_forge_version"] = json!(version);
                out.push((format!("{label} as {version}"), model));
            }
            change_everywhere(&root, "", &root, &label, &mut out);
        }
        out
    }

    /// The name of each kind of error, inside alternatives too.
    fn kinds(errors: &[SchemaError<'_>], seen: &mut std::collections::BTreeSet<String>) {
        for e in errors {
            let name = format!("{:?}", e.kind);
            seen.insert(name.split(['(', ' ']).next().unwrap().to_string());
            if let Kind::AnyOf(context) | Kind::OneOfNone(context) = &e.kind {
                for branch in context {
                    kinds(branch, seen);
                }
            }
        }
    }

    /// Every model made from the examples: refused or not, each problem has
    /// words, and every kind of error the schemas can give is met.
    #[test]
    fn every_generated_model_s_problems_are_said_in_words() {
        let (mut checked, mut refused) = (0, 0);
        let mut seen = std::collections::BTreeSet::new();
        for (label, model) in corpus() {
            let Some(schema) = model["_forge_version"].as_str().and_then(for_format) else {
                continue;
            };
            let errors = schema.errors(&model);
            checked += 1;
            refused += usize::from(!errors.is_empty());
            kinds(&errors, &mut seen);
            let lines = problem_lines(&errors, &model, &schema.doc);
            assert_eq!(errors.is_empty(), lines.is_empty(), "{label}");
            assert!(
                lines.iter().all(|line| !line.contains("a bug in Forge")),
                "{label}: {lines:?}"
            );
        }
        // A corpus that is mostly refused
        assert!(checked > 70_000 && refused > 30_000, "{checked} {refused}");
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            [
                "AdditionalProperties",
                "AnyOf",
                "Enum",
                "Format",
                "MaxItems",
                "Maximum",
                "MinItems",
                "MinProperties",
                "Minimum",
                "Not",
                "OneOfNone",
                "Pattern",
                "Required",
                "Type",
            ]
        );
    }
}
