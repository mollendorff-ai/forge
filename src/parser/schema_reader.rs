//! Forge's reader of its own JSON schemas (ADR-069).
//!
//! The six schema files in `schema/` are the statement of each model format.
//! This module reads them: the seventeen keywords they use, with the meaning
//! JSON Schema draft 07 gives them as the `jsonschema` crate (0.60) applied
//! them before this reader replaced it. Anything else in a schema is a
//! problem, and a schema with a problem refuses every model, so a keyword
//! this reader does not know is never passed over.
//!
//! What it shares with `jsonschema`, because the messages depend on it:
//! errors in the same order (each keyword in a fixed order of priority,
//! the keys of a mapping in alphabetical order, as `serde_json` keeps them),
//! the same places (in the model, and in the schema with `$ref` followed),
//! and for `anyOf` and `oneOf` the errors of each alternative.

use super::schema_pattern::Pattern as Matcher;
use serde_json::{Number, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

/// The model formats and their schemas, built in.
const FORMATS: [(&str, &str); 6] = [
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
];

/// The schema of a model format, read once; `None` for a format Forge does
/// not have.
pub(super) fn for_format(version: &str) -> Option<&'static Schema> {
    static READ: [OnceLock<Schema>; 6] = [const { OnceLock::new() }; 6];
    let index = FORMATS.iter().position(|(v, _)| *v == version)?;
    Some(READ[index].get_or_init(|| Schema::from_text(FORMATS[index].1)))
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
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "null" => Self::Null,
            "boolean" => Self::Boolean,
            "integer" => Self::Integer,
            "number" => Self::Number,
            "string" => Self::String,
            "array" => Self::Array,
            "object" => Self::Object,
            _ => return None,
        })
    }

    /// Whether a value is of this type. An integer is a number with no
    /// fraction, so `1.0` is one (draft 6 on).
    fn fits(self, value: &Value) -> bool {
        match (self, value) {
            (Self::Null, Value::Null)
            | (Self::Boolean, Value::Bool(_))
            | (Self::Number, Value::Number(_))
            | (Self::String, Value::String(_))
            | (Self::Array, Value::Array(_))
            | (Self::Object, Value::Object(_)) => true,
            (Self::Integer, Value::Number(n)) => {
                n.is_i64() || n.is_u64() || n.as_f64().is_some_and(|f| f.fract() == 0.0)
            },
            _ => false,
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

/// The ways a value can fail its schema.
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
    /// The schema itself has what this reader does not read
    Unreadable(Vec<String>),
}

/// A schema, read: one node per schema object reached, `$ref`s resolved.
#[derive(Debug)]
pub(super) struct Schema {
    /// The schema as JSON, for the names of keys in messages
    pub doc: Value,
    nodes: Vec<Node>,
    /// What could not be read; a schema with any refuses every model
    problems: Vec<String>,
}

#[derive(Debug)]
struct Node {
    /// Where the schema object is, as a JSON pointer
    at: String,
    /// In the order they run
    checks: Vec<Check>,
    /// The node a `$ref` points to; a schema object with one has nothing
    /// else (draft 07)
    reference: Option<usize>,
}

#[derive(Debug)]
enum Check {
    Type(Vec<JsonType>),
    Enum(Vec<Value>),
    Minimum(Number),
    Maximum(Number),
    MinItems(usize),
    MaxItems(usize),
    Required(Vec<String>),
    Pattern(String, Matcher),
    Date,
    Properties(Properties),
    Items(usize),
    /// `type: "array"`, `minItems` and `maxItems` beside an `items` schema,
    /// which `jsonschema` checks together, in the place of `items`
    Array {
        min: Option<usize>,
        max: Option<usize>,
        item: usize,
    },
    AllOf(Vec<usize>),
    AnyOf(Vec<usize>),
    OneOf(Vec<usize>),
    Not(usize),
    If {
        test: usize,
        then: Option<usize>,
        otherwise: Option<usize>,
    },
}

/// `properties`, `additionalProperties` and, when `jsonschema` checks it with
/// them, `required`.
#[derive(Debug)]
struct Properties {
    known: BTreeMap<String, usize>,
    rest: Rest,
    /// `required` with one key beside `properties` and
    /// `additionalProperties: false`: `jsonschema` reports it last
    required: Option<String>,
}

/// What a key not in `properties` may hold.
#[derive(Debug)]
enum Rest {
    Anything,
    Nothing,
    Schema(usize),
}

impl Check {
    /// The order `jsonschema` runs the keywords of one schema object in.
    const fn priority(&self) -> u8 {
        match self {
            Self::Type(_) => 1,
            Self::Enum(_) => 6,
            Self::Minimum(_) => 10,
            Self::Maximum(_) => 11,
            Self::MinItems(_) => 22,
            Self::MaxItems(_) => 23,
            Self::Required(_) => 26,
            Self::Pattern(..) => 30,
            Self::Date => 31,
            Self::Properties(_) => 40,
            Self::Items(_) | Self::Array { .. } => 44,
            Self::AllOf(_) => 50,
            Self::AnyOf(_) => 51,
            Self::OneOf(_) => 52,
            Self::Not(_) => 53,
            Self::If { .. } => 54,
        }
    }
}

/// Keywords that only describe; `definitions` holds what `$ref` points to.
const NOTES: [&str; 8] = [
    "title",
    "description",
    "examples",
    "default",
    "$schema",
    "$id",
    "$comment",
    "definitions",
];

/// A JSON pointer step: `~` and `/` escaped.
fn step(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// `format: date`: `YYYY-MM-DD`, a day that exists, leap years counted.
fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        b[range].iter().try_fold(0_u32, |n, d| {
            d.is_ascii_digit().then(|| n * 10 + u32::from(d - b'0'))
        })
    };
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=last).contains(&day)
}

/// Two JSON values are equal, numbers by their value (`1` is `1.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

struct Reader<'d> {
    doc: &'d Value,
    nodes: Vec<Node>,
    /// The node each `$ref` target was read into
    targets: HashMap<String, usize>,
    problems: Vec<String>,
}

impl<'d> Reader<'d> {
    fn problem(&mut self, at: &str, what: &str) {
        let at = if at.is_empty() { "/" } else { at };
        self.problems.push(format!("{at}: {what}"));
    }

    /// Read the schema object at `at` into a node, and return its index.
    fn read(&mut self, at: &str, schema: &'d Value) -> usize {
        let index = self.nodes.len();
        self.nodes.push(Node {
            at: at.to_string(),
            checks: Vec::new(),
            reference: None,
        });
        let Value::Object(map) = schema else {
            self.problem(at, "a schema that is not a mapping");
            return index;
        };
        let mut checks = Vec::new();
        if let Some(target) = map.get("$ref") {
            if let Some(other) = map
                .keys()
                .find(|k| *k != "$ref" && !NOTES.contains(&k.as_str()))
            {
                self.problem(
                    at,
                    &format!("'{other}' beside $ref, which draft 07 does not read"),
                );
            }
            self.nodes[index].reference = self.reference(at, target);
            return index;
        }
        {
            self.properties(at, map, &mut checks);
            for (keyword, value) in map {
                if let Some(check) = self.keyword(at, map, keyword, value) {
                    checks.push(check);
                }
            }
        }
        checks.sort_by_key(Check::priority);
        self.nodes[index].checks = checks;
        index
    }

    /// The node a `$ref` points to, read the first time it is reached.
    fn reference(&mut self, at: &str, target: &Value) -> Option<usize> {
        let pointer = target
            .as_str()
            .and_then(|t| t.strip_prefix('#'))
            .filter(|p| p.starts_with("/definitions/"));
        let Some(pointer) = pointer else {
            self.problem(at, &format!("$ref {target}, not to #/definitions/"));
            return None;
        };
        if let Some(&node) = self.targets.get(pointer) {
            return Some(node);
        }
        let Some(schema) = self.doc.pointer(pointer) else {
            self.problem(at, &format!("$ref {target}, which is not there"));
            return None;
        };
        // Read into the next node, which is the one `read` creates
        self.targets.insert(pointer.to_string(), self.nodes.len());
        Some(self.read(pointer, schema))
    }

    fn list(&mut self, at: &str, keyword: &str, value: &'d Value) -> Vec<usize> {
        match value {
            Value::Array(schemas) if !schemas.is_empty() => schemas
                .iter()
                .enumerate()
                .map(|(i, s)| self.read(&format!("{at}/{keyword}/{i}"), s))
                .collect(),
            _ => {
                self.problem(at, &format!("{keyword} that is not a list of schemas"));
                Vec::new()
            },
        }
    }

    fn limit(&mut self, at: &str, keyword: &str, value: &Value) -> Option<usize> {
        let limit = value.as_u64().and_then(|n| usize::try_from(n).ok());
        if limit.is_none() {
            self.problem(at, &format!("{keyword} that is not a whole number"));
        }
        limit
    }

    fn number(&mut self, at: &str, keyword: &str, value: &Value) -> Option<Number> {
        if let Value::Number(n) = value {
            return Some(n.clone());
        }
        self.problem(at, &format!("{keyword} that is not a number"));
        None
    }

    fn types(&mut self, at: &str, value: &Value) -> Option<Vec<JsonType>> {
        let names: Vec<&Value> = match value {
            Value::Array(names) => names.iter().collect(),
            single => vec![single],
        };
        let types: Option<Vec<JsonType>> = names
            .iter()
            .map(|n| n.as_str().and_then(JsonType::named))
            .collect();
        let Some(mut types) = types.filter(|t| !t.is_empty()) else {
            self.problem(at, &format!("type {value}, which is not a JSON type"));
            return None;
        };
        types.sort();
        types.dedup();
        Some(types)
    }

    /// Whether `jsonschema` checks `type`, `minItems` and `maxItems` with
    /// `items`: `items` is a schema and `type` is `"array"` alone.
    fn array_shape(map: &serde_json::Map<String, Value>) -> bool {
        map.get("items").is_some_and(Value::is_object)
            && map.get("type").and_then(Value::as_str) == Some("array")
    }

    /// `required` with one key that `jsonschema` checks with `properties` and
    /// `additionalProperties: false`.
    fn fused_required(map: &serde_json::Map<String, Value>) -> Option<&str> {
        match (
            map.get("additionalProperties"),
            map.contains_key("properties"),
            map.get("required")
                .and_then(Value::as_array)
                .map(Vec::as_slice),
        ) {
            (Some(Value::Bool(false)), true, Some([Value::String(key)])) => Some(key),
            _ => None,
        }
    }

    /// `properties` and `additionalProperties`, read together.
    fn properties(
        &mut self,
        at: &str,
        map: &'d serde_json::Map<String, Value>,
        checks: &mut Vec<Check>,
    ) {
        let rest = match map.get("additionalProperties") {
            None | Some(Value::Bool(true)) => Rest::Anything,
            Some(Value::Bool(false)) => Rest::Nothing,
            Some(schema) => Rest::Schema(self.read(&format!("{at}/additionalProperties"), schema)),
        };
        let mut known = BTreeMap::new();
        match map.get("properties") {
            None => {},
            Some(Value::Object(properties)) => {
                for (key, schema) in properties {
                    let node = self.read(&format!("{at}/properties/{}", step(key)), schema);
                    known.insert(key.clone(), node);
                }
            },
            Some(_) => self.problem(at, "properties that is not a mapping"),
        }
        // `jsonschema` reports this form apart (as a false schema); the six
        // schemas never use it
        if matches!(rest, Rest::Nothing) && !map.contains_key("properties") {
            self.problem(at, "additionalProperties: false without properties");
        }
        if !known.is_empty() || !matches!(rest, Rest::Anything) {
            checks.push(Check::Properties(Properties {
                known,
                rest,
                required: Self::fused_required(map).map(str::to_string),
            }));
        }
    }

    /// One keyword of a schema object, other than `$ref`, `properties` and
    /// `additionalProperties`.
    fn keyword(
        &mut self,
        at: &str,
        map: &'d serde_json::Map<String, Value>,
        keyword: &str,
        value: &'d Value,
    ) -> Option<Check> {
        Some(match keyword {
            "properties" | "additionalProperties" | "then" | "else" => return None,
            k if NOTES.contains(&k) => return None,
            "type" | "minItems" | "maxItems" if Self::array_shape(map) => return None,
            "items" if Self::array_shape(map) => {
                let mut bound = |keyword| map.get(keyword).and_then(|v| self.limit(at, keyword, v));
                let (min, max) = (bound("minItems"), bound("maxItems"));
                Check::Array {
                    min,
                    max,
                    item: self.read(&format!("{at}/items"), value),
                }
            },
            "type" => Check::Type(self.types(at, value)?),
            "enum" => match value {
                Value::Array(options) if !options.is_empty() => Check::Enum(options.clone()),
                _ => {
                    self.problem(at, "enum that is not a list of values");
                    return None;
                },
            },
            "minimum" => Check::Minimum(self.number(at, keyword, value)?),
            "maximum" => Check::Maximum(self.number(at, keyword, value)?),
            "minItems" => Check::MinItems(self.limit(at, keyword, value)?),
            "maxItems" => Check::MaxItems(self.limit(at, keyword, value)?),
            "required" => {
                let keys: Option<Vec<String>> = value.as_array().and_then(|keys| {
                    keys.iter()
                        .map(|k| k.as_str().map(str::to_string))
                        .collect()
                });
                let Some(keys) = keys else {
                    self.problem(at, "required that is not a list of keys");
                    return None;
                };
                if Self::fused_required(map).is_some() {
                    return None;
                }
                Check::Required(keys)
            },
            "pattern" => {
                let compiled = value
                    .as_str()
                    .and_then(|p| Matcher::new(p).map(|matcher| (p, matcher)));
                let Some((pattern, re)) = compiled else {
                    self.problem(at, &format!("pattern {value}, which Forge cannot read"));
                    return None;
                };
                Check::Pattern(pattern.to_string(), re)
            },
            "format" => {
                if value.as_str() != Some("date") {
                    self.problem(
                        at,
                        &format!("format {value}; Forge reads format \"date\" only"),
                    );
                    return None;
                }
                Check::Date
            },
            "items" => Check::Items(self.read(&format!("{at}/items"), value)),
            "allOf" => Check::AllOf(self.list(at, keyword, value)),
            "anyOf" => Check::AnyOf(self.list(at, keyword, value)),
            "oneOf" => Check::OneOf(self.list(at, keyword, value)),
            "not" => Check::Not(self.read(&format!("{at}/not"), value)),
            "if" => Check::If {
                test: self.read(&format!("{at}/if"), value),
                then: map.get("then").map(|s| self.read(&format!("{at}/then"), s)),
                otherwise: map.get("else").map(|s| self.read(&format!("{at}/else"), s)),
            },
            other => {
                self.problem(
                    at,
                    &format!("the keyword '{other}', which Forge does not read"),
                );
                return None;
            },
        })
    }
}

/// Where a value is in the model, built only when an error needs it.
enum Path<'p> {
    Root,
    Key(&'p Self, &'p str),
    Index(&'p Self, usize),
}

impl Path<'_> {
    fn pointer(&self) -> String {
        match self {
            Self::Root => String::new(),
            Self::Key(parent, key) => format!("{}/{}", parent.pointer(), step(key)),
            Self::Index(parent, i) => format!("{}/{i}", parent.pointer()),
        }
    }
}

impl Schema {
    /// Read a schema from its text.
    pub(super) fn from_text(text: &str) -> Self {
        match serde_json::from_str(text) {
            Ok(doc) => Self::new(doc),
            Err(e) => Self {
                doc: Value::Null,
                nodes: Vec::new(),
                problems: vec![format!("/: not JSON ({e})")],
            },
        }
    }

    fn new(doc: Value) -> Self {
        let mut reader = Reader {
            doc: &doc,
            nodes: Vec::new(),
            targets: HashMap::new(),
            problems: Vec::new(),
        };
        reader.read("", &doc);
        let (nodes, problems) = (reader.nodes, reader.problems);
        Self {
            doc,
            nodes,
            problems,
        }
    }

    /// Everything in `model` that does not fit, in the order `jsonschema`
    /// reports it; empty if it fits.
    pub(super) fn errors<'v>(&self, model: &'v Value) -> Vec<SchemaError<'v>> {
        if !self.problems.is_empty() {
            return vec![SchemaError {
                instance: model,
                instance_path: String::new(),
                schema_path: String::new(),
                kind: Kind::Unreadable(self.problems.clone()),
            }];
        }
        self.collect(0, model, &Path::Root)
    }

    fn collect<'v>(&self, node: usize, value: &'v Value, path: &Path<'_>) -> Vec<SchemaError<'v>> {
        let mut out = Sink {
            errors: Vec::new(),
            quick: false,
        };
        self.node(node, value, path, &mut out);
        out.errors
    }

    /// Whether a value fits a node; stops at the first error.
    fn fits(&self, node: usize, value: &Value) -> bool {
        let mut out = Sink {
            errors: Vec::new(),
            quick: true,
        };
        self.node(node, value, &Path::Root, &mut out);
        out.errors.is_empty()
    }

    /// The errors of `value` against a node.
    fn node<'v>(&self, node: usize, value: &'v Value, path: &Path<'_>, out: &mut Sink<'v>) {
        let node = &self.nodes[node];
        if let Some(target) = node.reference {
            return self.node(target, value, path, out);
        }
        for check in &node.checks {
            if out.done() {
                return;
            }
            self.check(node, check, value, path, out);
        }
    }

    /// The errors of each alternative, for a message; none when only
    /// whether the value fits matters.
    fn alternatives<'v>(
        &self,
        branches: &[usize],
        value: &'v Value,
        path: &Path<'_>,
        out: &Sink<'v>,
    ) -> Vec<Vec<SchemaError<'v>>> {
        if out.quick {
            return Vec::new();
        }
        branches
            .iter()
            .map(|&b| self.collect(b, value, path))
            .collect()
    }

    /// `properties` and `additionalProperties` (and a `required` read with
    /// them): each key's errors in the order of the keys, then the keys the
    /// mapping cannot have, then the required key if it is missing.
    fn properties<'v>(
        &self,
        node: &Node,
        p: &Properties,
        map: &'v serde_json::Map<String, Value>,
        value: &'v Value,
        path: &Path<'_>,
        out: &mut Sink<'v>,
    ) {
        let mut unexpected = Vec::new();
        let mut found = false;
        for (key, child) in map {
            if out.done() {
                return;
            }
            let at = Path::Key(path, key);
            if let Some(&known) = p.known.get(key) {
                self.node(known, child, &at, out);
                found |= p.required.as_ref() == Some(key);
            } else if let Rest::Schema(rest) = p.rest {
                self.node(rest, child, &at, out);
            } else if matches!(p.rest, Rest::Nothing) {
                unexpected.push(key.clone());
            }
        }
        if !unexpected.is_empty() {
            let kind = Kind::AdditionalProperties(unexpected);
            out.errors
                .push(fail(node, "additionalProperties", value, path, kind));
        }
        if let (Some(key), false) = (&p.required, found) {
            let kind = Kind::Required(key.clone());
            out.errors.push(fail(node, "required", value, path, kind));
        }
    }

    fn items<'v>(&self, item: usize, items: &'v [Value], path: &Path<'_>, out: &mut Sink<'v>) {
        for (i, child) in items.iter().enumerate() {
            if out.done() {
                return;
            }
            self.node(item, child, &Path::Index(path, i), out);
        }
    }

    fn check<'v>(
        &self,
        node: &Node,
        check: &Check,
        value: &'v Value,
        path: &Path<'_>,
        out: &mut Sink<'v>,
    ) {
        let mut push = |keyword: &str, kind: Kind<'v>| {
            out.errors.push(fail(node, keyword, value, path, kind));
        };
        match (check, value) {
            (Check::Type(types), _) if !types.iter().any(|t| t.fits(value)) => {
                push("type", Kind::Type(types.clone()));
            },
            (Check::Enum(options), _) if !options.iter().any(|o| same(o, value)) => {
                push("enum", Kind::Enum(options.clone()));
            },
            (Check::Minimum(limit), Value::Number(n)) if n.as_f64() < limit.as_f64() => {
                push("minimum", Kind::Minimum(limit.clone()));
            },
            (Check::Maximum(limit), Value::Number(n)) if n.as_f64() > limit.as_f64() => {
                push("maximum", Kind::Maximum(limit.clone()));
            },
            (Check::MinItems(limit), Value::Array(items)) if items.len() < *limit => {
                push("minItems", Kind::MinItems(*limit));
            },
            (Check::MaxItems(limit), Value::Array(items)) if items.len() > *limit => {
                push("maxItems", Kind::MaxItems(*limit));
            },
            (Check::Required(keys), Value::Object(map)) => {
                for key in keys.iter().filter(|k| !map.contains_key(*k)) {
                    push("required", Kind::Required(key.clone()));
                }
            },
            (Check::Pattern(pattern, re), Value::String(text)) if !re.is_match(text) => {
                push("pattern", Kind::Pattern(pattern.clone()));
            },
            (Check::Date, Value::String(text)) if !is_date(text) => push("format", Kind::Format),
            (Check::Properties(p), Value::Object(map)) => {
                self.properties(node, p, map, value, path, out);
            },
            (Check::Items(item), Value::Array(items)) => self.items(*item, items, path, out),
            (Check::Array { min, max, item }, Value::Array(items)) => {
                if let Some(min) = min.filter(|min| items.len() < *min) {
                    push("minItems", Kind::MinItems(min));
                }
                if let Some(max) = max.filter(|max| items.len() > *max) {
                    push("maxItems", Kind::MaxItems(max));
                }
                self.items(*item, items, path, out);
            },
            (Check::Array { .. }, _) => push("type", Kind::Type(vec![JsonType::Array])),
            (Check::AllOf(all), _) => {
                for &b in all {
                    self.node(b, value, path, out);
                }
            },
            (Check::AnyOf(any), _) if !any.iter().any(|&b| self.fits(b, value)) => {
                let context = self.alternatives(any, value, path, out);
                out.errors
                    .push(fail(node, "anyOf", value, path, Kind::AnyOf(context)));
            },
            (Check::OneOf(one), _) => match one.iter().filter(|&&b| self.fits(b, value)).count() {
                0 => {
                    let context = self.alternatives(one, value, path, out);
                    out.errors
                        .push(fail(node, "oneOf", value, path, Kind::OneOfNone(context)));
                },
                1 => {},
                _ => out
                    .errors
                    .push(fail(node, "oneOf", value, path, Kind::OneOfMany)),
            },
            (Check::Not(not), _) if self.fits(*not, value) => push("not", Kind::Not),
            (
                Check::If {
                    test,
                    then,
                    otherwise,
                },
                _,
            ) => {
                let branch = if self.fits(*test, value) {
                    then
                } else {
                    otherwise
                };
                if let Some(b) = branch {
                    self.node(*b, value, path, out);
                }
            },
            _ => {},
        }
    }
}

/// Where errors go. When `quick`, only whether there is one matters, and the
/// walk stops at the first.
struct Sink<'v> {
    errors: Vec<SchemaError<'v>>,
    quick: bool,
}

impl Sink<'_> {
    const fn done(&self) -> bool {
        self.quick && !self.errors.is_empty()
    }
}

/// An error of `value`, at `path`, against a keyword of `node`.
fn fail<'v>(
    node: &Node,
    keyword: &str,
    value: &'v Value,
    path: &Path<'_>,
    kind: Kind<'v>,
) -> SchemaError<'v> {
    SchemaError {
        instance: value,
        instance_path: path.pointer(),
        schema_path: format!("{}/{keyword}", node.at),
        kind,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::parser::schema_errors::problem_lines;
    use jsonschema::error::{TypeKind, ValidationErrorKind as Their};
    use serde_json::json;

    /// `jsonschema`'s validator for a model format, compiled once.
    fn theirs(version: &str) -> &'static jsonschema::Validator {
        static COMPILED: [OnceLock<jsonschema::Validator>; 6] = [const { OnceLock::new() }; 6];
        let index = FORMATS.iter().position(|(v, _)| *v == version).unwrap();
        COMPILED[index].get_or_init(|| {
            jsonschema::validator_for(&serde_json::from_str(FORMATS[index].1).unwrap()).unwrap()
        })
    }

    const fn their_type(t: jsonschema::JsonType) -> JsonType {
        match t {
            jsonschema::JsonType::Array => JsonType::Array,
            jsonschema::JsonType::Boolean => JsonType::Boolean,
            jsonschema::JsonType::Integer => JsonType::Integer,
            jsonschema::JsonType::Null => JsonType::Null,
            jsonschema::JsonType::Number => JsonType::Number,
            jsonschema::JsonType::Object => JsonType::Object,
            jsonschema::JsonType::String => JsonType::String,
        }
    }

    /// An error of `jsonschema`'s as the reader's, so that both go through the
    /// same words. A kind the reader has none for is `Unreadable`, which the
    /// reader never reports for a schema it reads, so it cannot agree.
    fn convert<'v>(error: &jsonschema::ValidationError<'_>, model: &'v Value) -> SchemaError<'v> {
        let instance_path = error.instance_path().to_string();
        let branches = |context: &[Vec<jsonschema::ValidationError<'_>>]| {
            context
                .iter()
                .map(|b| b.iter().map(|e| convert(e, model)).collect())
                .collect()
        };
        let kind = match error.kind() {
            Their::AdditionalProperties { unexpected } => {
                Kind::AdditionalProperties(unexpected.clone())
            },
            Their::Type {
                kind: TypeKind::Single(t),
            } => Kind::Type(vec![their_type(*t)]),
            Their::Type {
                kind: TypeKind::Multiple(set),
            } => Kind::Type(set.iter().map(their_type).collect()),
            Their::Enum { options } => Kind::Enum(options.as_array().unwrap().clone()),
            Their::Minimum { limit } => Kind::Minimum(limit.as_number().unwrap().clone()),
            Their::Maximum { limit } => Kind::Maximum(limit.as_number().unwrap().clone()),
            Their::MinItems { limit } => Kind::MinItems(usize::try_from(*limit).unwrap()),
            Their::MaxItems { limit } => Kind::MaxItems(usize::try_from(*limit).unwrap()),
            Their::Required { property } => Kind::Required(property.as_str().unwrap().to_string()),
            Their::Pattern { pattern } => Kind::Pattern(pattern.clone()),
            Their::Format { .. } => Kind::Format,
            Their::AnyOf { context } => Kind::AnyOf(branches(context)),
            Their::OneOfNotValid { context } => Kind::OneOfNone(branches(context)),
            Their::OneOfMultipleValid { .. } => Kind::OneOfMany,
            Their::Not { .. } => Kind::Not,
            other => Kind::Unreadable(vec![format!("{other:?}")]),
        };
        SchemaError {
            instance: model.pointer(&instance_path).unwrap(),
            instance_path,
            schema_path: error.schema_path().to_string(),
            kind,
        }
    }

    /// The reader and `jsonschema` refuse the same model, with the same
    /// lines (in any order: the order is `jsonschema`'s to change). Every
    /// model any test reads goes through this (`validate_against_schema`).
    pub(in crate::parser) fn agrees_with_jsonschema(
        version: &str,
        model: &Value,
        errors: &[SchemaError<'_>],
    ) {
        let their: Vec<SchemaError<'_>> = theirs(version)
            .iter_errors(model)
            .map(|e| convert(&e, model))
            .collect();
        assert_eq!(errors.is_empty(), their.is_empty(), "{version}: {model}");
        let doc = &for_format(version).unwrap().doc;
        let mut ours = problem_lines(errors, model, doc);
        let mut their = problem_lines(&their, model, doc);
        ours.sort();
        their.sort();
        assert_eq!(ours, their, "{version}: {model}");
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

    #[test]
    fn the_reader_and_jsonschema_refuse_the_same_generated_models() {
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
            let version = model["_forge_version"].as_str().unwrap();
            let result =
                std::panic::catch_unwind(|| agrees_with_jsonschema(version, &model, &errors));
            assert!(result.is_ok(), "{label}");
        }
        // Every kind of error the six schemas can give, in a corpus that is
        // mostly refused
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
                "Minimum",
                "Not",
                "OneOfNone",
                "Pattern",
                "Required",
                "Type",
            ]
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Each keyword on a small schema: the whole list of errors, in order,
    // as jsonschema gives it
    // ─────────────────────────────────────────────────────────────────────

    /// The reader's errors and `jsonschema`'s for a schema of draft 07.
    fn both(schema: Value, value: &Value) -> (String, String) {
        let mut schema = schema;
        schema["$schema"] = json!("http://json-schema.org/draft-07/schema#");
        let their = jsonschema::validator_for(&schema).unwrap();
        let their: Vec<SchemaError<'_>> = their
            .iter_errors(value)
            .map(|e| convert(&e, value))
            .collect();
        let ours = Schema::new(schema).errors(value);
        (format!("{ours:#?}"), format!("{their:#?}"))
    }

    #[track_caller]
    fn same(schema: &Value, values: &[Value]) {
        for value in values {
            let (ours, their) = both(schema.clone(), value);
            assert_eq!(ours, their, "{value}");
        }
    }

    #[test]
    fn types_one_or_several_and_a_whole_number() {
        same(
            &json!({"type": "integer"}),
            &[
                json!(1),
                json!(1.0),
                json!(1.5),
                json!(-3),
                json!(u64::MAX),
                json!("1"),
            ],
        );
        same(
            &json!({"type": ["object", "string", "null", "number", "integer", "boolean", "array"]}),
            &[json!(true)],
        );
        same(
            &json!({"type": ["array", "string", "object"]}),
            &[json!(1), json!(null), json!([])],
        );
        same(&json!({"type": "boolean"}), &[json!(false), json!(0)]);
    }

    #[test]
    fn values_limits_and_lengths() {
        same(
            &json!({"enum": ["a", 1]}),
            &[json!("a"), json!(1.0), json!(2), json!("b")],
        );
        same(
            &json!({"minimum": -1, "maximum": 1.5}),
            &[
                json!(-1),
                json!(-1.01),
                json!(1.5),
                json!(2),
                json!(u64::MAX),
                json!("x"),
            ],
        );
        same(
            &json!({"minItems": 1, "maxItems": 2}),
            &[json!([]), json!([1]), json!([1, 2, 3]), json!({})],
        );
    }

    #[test]
    fn keys_in_the_order_jsonschema_reports_them() {
        let values = [
            json!({}),
            json!({"a": 1, "b": 2}),
            json!({"a": "x", "z": 1, "c": 1, "b": "y"}),
            json!({"b": "y", "c": 1}),
            json!(3),
        ];
        // `required` alone runs before `properties`
        same(
            &json!({"properties": {"a": {"type": "string"}, "b": {"type": "string"}}, "required": ["a", "b"]}),
            &values,
        );
        // With `additionalProperties: false` and one required key, that key is
        // reported last
        same(
            &json!({"properties": {"a": {"type": "string"}, "b": {"type": "string"}}, "additionalProperties": false, "required": ["a"]}),
            &values,
        );
        same(
            &json!({"properties": {"a": {"type": "string"}}, "additionalProperties": false, "required": ["a", "b"]}),
            &values,
        );
        // A schema for the other keys: their errors where they come
        same(
            &json!({"properties": {"b": {"type": "string"}}, "additionalProperties": {"type": "string"}}),
            &values,
        );
        same(
            &json!({"additionalProperties": {"type": "integer"}}),
            &values,
        );
        same(
            &json!({"properties": {"a": {"type": "string"}}, "additionalProperties": true}),
            &values,
        );
        // A key that needs escaping in a JSON pointer
        same(
            &json!({"properties": {"a/b": {"type": "string"}, "c~d": {"type": "string"}}}),
            &[json!({"a/b": 1, "c~d": 2})],
        );
    }

    #[test]
    fn items_and_alternatives() {
        let values = [
            json!([1, "a", null, true]),
            json!("=x"),
            json!("x"),
            json!(2),
            json!(null),
        ];
        same(&json!({"items": {"type": "number"}}), &values);
        same(
            &json!({"allOf": [{"type": "string"}, {"pattern": "^="}]}),
            &values,
        );
        same(
            &json!({"anyOf": [{"type": "string", "pattern": "^="}, {"type": "null"}]}),
            &values,
        );
        same(
            &json!({"oneOf": [{"type": "string"}, {"pattern": "^="}, {"minimum": 5}]}),
            &values,
        );
        same(&json!({"not": {"type": "string"}}), &values);
        same(
            &json!({"if": {"type": "string"}, "then": {"pattern": "^="}, "else": {"type": "null"}}),
            &values,
        );
        same(
            &json!({"if": {"type": "string"}, "then": {"pattern": "^="}}),
            &values,
        );
        same(
            &json!({"if": {"type": "string"}, "else": {"type": "null"}}),
            &values,
        );
        // Alternatives inside alternatives, and an alternative that fails on
        // two keys or two items, each stopped at its first error
        same(
            &json!({"anyOf": [
                {"items": {"anyOf": [{"type": "string"}, {"type": "number"}]}},
                {"properties": {"a": {"type": "string"}, "b": {"type": "string"}}, "required": ["a"]}
            ]}),
            &[json!([true, null]), json!({"a": 1, "b": 2}), json!({})],
        );
    }

    /// Several keywords of one schema object, in `jsonschema`'s order.
    #[test]
    fn the_keywords_of_one_object_run_in_jsonschemas_order() {
        same(
            &json!({
                "type": "string", "enum": ["=b"], "pattern": "^=a", "format": "date",
                "allOf": [{"pattern": "^x"}], "anyOf": [{"pattern": "^y"}], "oneOf": [{"pattern": "^z"}],
                "not": {"type": "string"}, "if": {"type": "string"}, "then": {"pattern": "^w"}
            }),
            &[json!("c"), json!(1)],
        );
        same(
            &json!({
                "type": "array", "minItems": 3, "maxItems": 1, "items": {"type": "string"},
                "required": ["a"], "properties": {"a": {}}, "minimum": 1, "maximum": 0
            }),
            &[json!([1, 2]), json!({}), json!(0.5)],
        );
    }

    #[test]
    fn a_reference_is_followed_and_reported_where_it_points() {
        let schema = json!({
            "definitions": {
                "Tree": {"type": "object", "properties": {"kids": {"items": {"$ref": "#/definitions/Tree"}}, "n": {"type": "number"}}}
            },
            "$ref": "#/definitions/Tree"
        });
        same(
            &schema,
            &[json!({"n": "x", "kids": [{"n": 1}, {"kids": [{"n": true}]}]})],
        );
    }

    #[test]
    fn patterns_and_dates_read_as_jsonschema_reads_them() {
        // \d is an ASCII digit, not any script's; the pattern is searched for
        same(
            &json!({"pattern": r"^\d{4}-\d{2}(-\d{2})?$"}),
            &[
                json!("2025-01"),
                json!("٢٠٢٥-01"),
                json!("2025-01-31x"),
                json!(5),
            ],
        );
        same(
            &json!({"pattern": r"[\d]\D"}),
            &[json!("1a"), json!("a1"), json!("11")],
        );
        same(&json!({"pattern": "=."}), &[json!("x =y"), json!("=\n")]);
        same(
            &json!({"items": {"format": "date"}}),
            &[json!([
                "2024-02-29",
                "2023-02-29",
                "1900-02-29",
                "2000-02-29",
                "0000-01-01",
                "2025-04-30",
                "2025-04-31",
                "2025-01-31",
                "2025-01-32",
                "2025-13-01",
                "2025-00-10",
                "2025-01-00",
                "2025-1-01",
                "2025/01/01",
                "20a5-01-01",
                "2025-01",
                5
            ])],
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // A schema with what the reader does not read refuses every model
    // ─────────────────────────────────────────────────────────────────────

    fn problems(schema: &Value) -> Vec<String> {
        Schema::new(schema.clone()).problems
    }

    #[test]
    fn what_the_reader_cannot_read_is_named() {
        for (schema, problem) in [
            (
                json!({"additionalProperties": false}),
                "/: additionalProperties: false without properties",
            ),
            (
                json!({"const": 1}),
                "/: the keyword 'const', which Forge does not read",
            ),
            (
                json!({"items": true}),
                "/items: a schema that is not a mapping",
            ),
            (
                json!({"$ref": "#/definitions/A", "type": "string", "definitions": {"A": {}}}),
                "/: 'type' beside $ref, which draft 07 does not read",
            ),
            (
                json!({"$ref": "other.json#/A"}),
                "/: $ref \"other.json#/A\", not to #/definitions/",
            ),
            (json!({"$ref": 3}), "/: $ref 3, not to #/definitions/"),
            (
                json!({"$ref": "#/definitions/B"}),
                "/: $ref \"#/definitions/B\", which is not there",
            ),
            (
                json!({"anyOf": {}}),
                "/: anyOf that is not a list of schemas",
            ),
            (
                json!({"oneOf": []}),
                "/: oneOf that is not a list of schemas",
            ),
            (
                json!({"minItems": -1}),
                "/: minItems that is not a whole number",
            ),
            (
                json!({"type": "array", "items": {}, "maxItems": "2"}),
                "/: maxItems that is not a whole number",
            ),
            (
                json!({"maxItems": 1.5}),
                "/: maxItems that is not a whole number",
            ),
            (json!({"minimum": "1"}), "/: minimum that is not a number"),
            (json!({"maximum": null}), "/: maximum that is not a number"),
            (
                json!({"type": "date"}),
                "/: type \"date\", which is not a JSON type",
            ),
            (json!({"type": []}), "/: type [], which is not a JSON type"),
            (
                json!({"properties": []}),
                "/: properties that is not a mapping",
            ),
            (json!({"enum": []}), "/: enum that is not a list of values"),
            (
                json!({"required": "a"}),
                "/: required that is not a list of keys",
            ),
            (
                json!({"required": [1]}),
                "/: required that is not a list of keys",
            ),
            (
                json!({"pattern": "("}),
                "/: pattern \"(\", which Forge cannot read",
            ),
            (
                json!({"pattern": 1}),
                "/: pattern 1, which Forge cannot read",
            ),
            (
                json!({"format": "email"}),
                "/: format \"email\"; Forge reads format \"date\" only",
            ),
            (
                json!({"not": {"uniqueItems": true}}),
                "/not: the keyword 'uniqueItems', which Forge does not read",
            ),
        ] {
            assert_eq!(problems(&schema), [problem], "{schema}");
        }
        assert_eq!(
            Schema::from_text("{").problems,
            ["/: not JSON (EOF while parsing an object at line 1 column 1)"]
        );
    }

    #[test]
    fn a_schema_with_a_problem_refuses_every_model_and_says_why() {
        let schema = Schema::new(json!({"const": 1, "properties": {"a": {"format": "email"}}}));
        for model in [json!(1), json!({"a": "x"})] {
            let errors = schema.errors(&model);
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].instance_path, "");
            assert_eq!(
                problem_lines(&errors, &model, &schema.doc),
                ["Forge cannot read the schema of this model format, so no model of it is accepted (a bug in Forge): /properties/a: format \"email\"; Forge reads format \"date\" only; /: the keyword 'const', which Forge does not read"]
            );
        }
    }

    #[test]
    fn every_format_is_read_without_a_problem() {
        for (version, _) in FORMATS {
            assert_eq!(
                for_format(version).unwrap().problems,
                Vec::<String>::new(),
                "{version}"
            );
        }
        assert!(for_format("5.5.0").is_none());
    }

    /// A kind of jsonschema's that the reader does not have cannot agree.
    #[test]
    fn a_kind_the_reader_lacks_is_unreadable() {
        let value = json!(2);
        let their = jsonschema::validator_for(&json!({"const": 1})).unwrap();
        let errors: Vec<SchemaError<'_>> = their
            .iter_errors(&value)
            .map(|e| convert(&e, &value))
            .collect();
        assert!(
            matches!(&errors[..], [SchemaError { kind: Kind::Unreadable(what), .. }] if what[0].starts_with("Constant")),
            "{errors:?}"
        );
    }
}
