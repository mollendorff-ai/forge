//! Checking a model against a schema read by `schema_reader` (ADR-069):
//! every error, in the order and the places `jsonschema` 0.60 reported them,
//! because the messages in `schema_errors` depend on both.

use super::schema_reader::{
    step, Check, JsonType, Kind, Node, Properties, Rest, Schema, SchemaError,
};
use serde_json::Value;

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

impl JsonType {
    /// Whether a value is of this type. An integer is a number with no
    /// fraction, so `1.0` is one (draft 6 on).
    pub(super) fn fits(self, value: &Value) -> bool {
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
            (Check::MinProperties(limit), Value::Object(map)) if map.len() < *limit => {
                push("minProperties", Kind::MinProperties(*limit));
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
