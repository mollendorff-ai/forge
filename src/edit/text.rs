//! One change to one file's text (ADR-075).
//!
//! yaml-edit's lossless tree finds the place; Forge writes the bytes, in the
//! style the file already uses, as Forge Studio's `text.js` did. Nothing here
//! reads a model's meaning: the caller says what a name is (an input, a
//! formula, a column) and reads the result back.

use std::ops::Range;
use yaml_edit::{Mapping, Scalar, YamlFile, YamlNode};

/// What `add` writes: an input, a formula, or a table's column
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum New<'a> {
    /// `name: 16`, or `value:` and `unit:` under `name:`
    Input {
        value: &'a str,
        unit: Option<&'a str>,
    },
    /// `formula:` (and `unit:`) under `name:`
    Formula {
        formula: &'a str,
        unit: Option<&'a str>,
    },
    /// `name: "=..."` in a table
    Column { formula: &'a str },
}

/// The file as a tree, or why it cannot be edited
pub(super) fn parse(text: &str) -> Result<YamlFile, String> {
    let parsed = YamlFile::parse(text);
    if let Some(error) = parsed.positioned_errors().first() {
        return Err(format!("its YAML cannot be read ({error})"));
    }
    let file = parsed.tree();
    if file.documents().count() > 1 {
        return Err("it holds several YAML documents, which edit does not change".into());
    }
    Ok(file)
}

/// The mapping holding the last of `names`, and that last name
pub(super) fn place<'n>(
    file: &YamlFile,
    names: &'n [&'n str],
) -> Result<(Mapping, &'n str), String> {
    let (name, groups) = names.split_last().ok_or("no name was given")?;
    let mut mapping = file
        .document()
        .and_then(|d| d.as_mapping())
        .ok_or("it is not a mapping of names")?;
    for group in groups {
        mapping = mapping
            .get_mapping(group)
            .ok_or_else(|| format!("{group} is not written as a group of names"))?;
    }
    Ok((mapping, name))
}

/// The scalar `name` holds: itself, or its `key:` (`value:`, `formula:`)
/// when it is a mapping; and whether the mapping holding it is a flow one
fn scalar_of(mapping: &Mapping, name: &str, key: &str) -> Result<(Scalar, bool), String> {
    let missing = || format!("{name} has no {key}: written where edit can change it");
    match mapping.get(name).ok_or_else(missing)? {
        YamlNode::Scalar(s) => Ok((s, mapping.is_flow_style())),
        YamlNode::Mapping(inner) => match inner.get(key) {
            Some(YamlNode::Scalar(s)) => Ok((s, inner.is_flow_style())),
            _ => Err(missing()),
        },
        _ => Err(missing()),
    }
}

/// Set an input's value, written as `value` is (a number or a date as YAML
/// text). The rest of the line, a comment or the rest of a flow mapping, stays.
///
/// # Errors
///
/// Returns why, when the file cannot be read as YAML or the value is not
/// written where edit can find it.
pub fn set_value(text: &str, names: &[&str], value: &str) -> Result<String, String> {
    let file = parse(text)?;
    let (mapping, name) = place(&file, names)?;
    let (scalar, _) = scalar_of(&mapping, name, "value")?;
    // `value:` with nothing after it: the space goes in too
    let spaced;
    let value = if scalar.value().is_empty() {
        spaced = format!(" {value}");
        &spaced
    } else {
        value
    };
    scalar.set_value(value);
    Ok(file.to_string())
}

/// Set a formula, in the quotes the file already uses for it.
///
/// # Errors
///
/// As [`set_value`].
pub fn set_formula(text: &str, names: &[&str], formula: &str) -> Result<String, String> {
    let file = parse(text)?;
    let (mapping, name) = place(&file, names)?;
    let (scalar, flow) = scalar_of(&mapping, name, "formula")?;
    let style = match scalar.value().chars().next() {
        Some('"') => Style::Double,
        Some('\'') => Style::Single,
        _ => Style::Plain,
    };
    scalar.set_value(&encode_formula(formula, style, flow));
    Ok(file.to_string())
}

/// Add `name` at the end of its group, or, at the top level, after the
/// block of the top-level name `after`. The new lines take the indentation
/// of the keys already there.
///
/// # Errors
///
/// Returns why, when the group is not written as a block of keys, or
/// `after` is not a top-level name.
///
/// # Panics
///
/// Never: a group written as a block has at least one name.
pub fn add(
    text: &str,
    names: &[&str],
    new: New<'_>,
    after: Option<&str>,
) -> Result<String, String> {
    let file = parse(text)?;
    let (mapping, name) = place(&file, names)?;
    let top = names.len() == 1;
    if mapping.is_flow_style() {
        return Err("its group is written on one line ({...}); edit adds to a block".into());
    }
    if mapping.contains_key(name) {
        return Err(format!("{name} is already written there"));
    }
    let last = if top {
        let after = after.ok_or("a name at the top level needs `after`: the name it follows")?;
        mapping
            .find_entry_by_key(after)
            .ok_or_else(|| format!("{after} is not a name at the top level"))?
    } else {
        // An empty `g:` is null, not a group: `place` refused it
        mapping
            .entries()
            .last()
            .expect("a group written as a block has a name")
    };
    let pad = if top {
        String::new()
    } else {
        // The indentation of the group's first key
        let first = mapping
            .entries()
            .next()
            .map_or(0, |e| e.byte_range().start as usize);
        let line = text[..first].rfind('\n').map_or(0, |i| i + 1);
        text[line..first].to_string()
    };
    let inner = if top {
        "  ".to_string()
    } else {
        format!("{pad}{pad}")
    };
    let unit_line = |unit: Option<&str>| unit.map(|u| format!("{inner}unit: {}", unit_text(u)));
    let lines: Vec<String> = match new {
        New::Input { value, unit: None } => vec![format!("{pad}{name}: {value}")],
        New::Input { value, unit } => std::iter::once(format!("{pad}{name}:"))
            .chain(std::iter::once(format!("{inner}value: {value}")))
            .chain(unit_line(unit))
            .collect(),
        New::Formula { formula, unit } => std::iter::once(format!("{pad}{name}:"))
            .chain(std::iter::once(format!(
                "{inner}formula: {}",
                encode_formula(formula, Style::Double, false)
            )))
            .chain(unit_line(unit))
            .collect(),
        New::Column { formula } => vec![format!(
            "{pad}{name}: {}",
            encode_formula(formula, Style::Double, false)
        )],
    };
    let at = last.byte_range().end as usize;
    let added = if text[..at].ends_with('\n') {
        lines.join("\n") + "\n"
    } else {
        format!("\n{}", lines.join("\n"))
    };
    Ok(format!("{}{added}{}", &text[..at], &text[at..]))
}

/// Delete `name`: its key, the lines under it and its comment on the line.
/// A comment on the lines above it stays (it may be about what follows).
///
/// # Errors
///
/// Returns why, when `name` is not written where edit can find it, or is the
/// last name of its group, which would be left empty.
pub fn delete(text: &str, names: &[&str]) -> Result<String, String> {
    let file = parse(text)?;
    let (mapping, name) = place(&file, names)?;
    if names.len() > 1 && mapping.len() == 1 {
        return Err(format!(
            "{name} is the last name of its group, which would be left empty"
        ));
    }
    mapping
        .remove(name)
        .ok_or_else(|| format!("{name} is not written where edit can find it"))?;
    Ok(file.to_string())
}

/// A scalar of a file, a key or a value, and where it is written
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The keys from the top down to it, an item of a sequence as `#`; a
    /// key's ends with the key itself
    pub path: Vec<String>,
    /// Whether it is a key
    pub key: bool,
    /// Its bytes in the file
    pub range: Range<usize>,
    /// What it says, unquoted
    pub text: String,
    /// Whether it is in a flow mapping or sequence (`{...}`, `[...]`)
    pub flow: bool,
}

/// Every key and every scalar value of the file, in the file's order
///
/// # Errors
///
/// Returns why, when the file cannot be read as YAML or is not a mapping.
pub fn sites(text: &str) -> Result<Vec<Site>, String> {
    let file = parse(text)?;
    let top = file
        .document()
        .and_then(|d| d.as_mapping())
        .ok_or("it is not a mapping of names")?;
    let mut sites = Vec::new();
    walk(&YamlNode::Mapping(top), &mut Vec::new(), &mut sites);
    Ok(sites)
}

fn site(scalar: &Scalar, path: &[String], key: bool, flow: bool) -> Site {
    let range = scalar.byte_range();
    Site {
        path: path.to_vec(),
        key,
        range: range.start as usize..range.end as usize,
        text: scalar.as_string(),
        flow,
    }
}

fn walk(node: &YamlNode, path: &mut Vec<String>, sites: &mut Vec<Site>) {
    match node {
        YamlNode::Mapping(mapping) => {
            let flow = mapping.is_flow_style();
            for entry in mapping.entries() {
                let Some(YamlNode::Scalar(key)) = entry.key_node() else {
                    continue;
                };
                path.push(key.as_string());
                sites.push(site(&key, path, true, flow));
                // The parser gives every entry a value, empty where none is written
                let value = entry
                    .value_node()
                    .expect("an entry of a file that parses has a value");
                if let YamlNode::Scalar(scalar) = &value {
                    sites.push(site(scalar, path, false, flow));
                } else {
                    walk(&value, path, sites);
                }
                path.pop();
            }
        },
        YamlNode::Sequence(sequence) => {
            let flow = sequence.is_flow_style();
            path.push("#".to_string());
            for value in sequence.values() {
                if let YamlNode::Scalar(scalar) = &value {
                    sites.push(site(scalar, path, false, flow));
                } else {
                    walk(&value, path, sites);
                }
            }
            path.pop();
        },
        _ => {},
    }
}

/// `text` with each range replaced by its text; the ranges do not overlap
#[must_use]
pub fn splice(text: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (range, new) in edits {
        out.push_str(&text[at..range.start]);
        out.push_str(&new);
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// A name written in the quotes `raw` is written in (a name needs none)
#[must_use]
pub fn name_text(raw: &str, name: &str) -> String {
    match raw.chars().next() {
        Some(q @ ('"' | '\'')) => format!("{q}{name}{q}"),
        _ => name.to_string(),
    }
}

/// A formula's text, `text`, with some of its names changed to give `new`,
/// written as `raw` writes it.
///
/// In its quotes, and byte for byte the same outside the names. `None` when
/// `raw` is folded over several lines, as a block scalar or a long plain one.
#[must_use]
pub fn renamed_formula(raw: &str, text: &str, new: &str) -> Option<String> {
    match raw.chars().next() {
        Some('"') => Some(encode_formula(new, Style::Double, false)),
        Some('\'') => Some(encode_formula(new, Style::Single, false)),
        // A name is letters, digits, `_` and `.`, which a plain scalar
        // reads as written
        _ => (raw == text).then(|| new.to_string()),
    }
}

/// How the file writes a formula
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Plain,
    Single,
    Double,
}

/// A formula for YAML in the style the file uses for it: plain when that is
/// safe, 'single quotes', or "double quotes" (`text.js`'s `encodeFormula`).
pub(super) fn encode_formula(formula: &str, style: Style, flow: bool) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let comment = chars
        .windows(2)
        .any(|w| w[0].is_whitespace() && w[1] == '#');
    let plain_unsafe = formula.contains(": ")
        || comment
        || chars.first().is_some_and(|c| c.is_whitespace())
        || chars.last().is_some_and(|c| c.is_whitespace())
        || formula.contains(['"', '\'', '`', '\\'])
        || (flow && formula.contains([',', '{', '}', '[', ']']));
    match style {
        Style::Plain if !plain_unsafe => formula.to_string(),
        Style::Single => format!("'{}'", formula.replace('\'', "''")),
        _ => format!("\"{}\"", formula.replace('\\', "\\\\").replace('"', "\\\"")),
    }
}

/// A unit as YAML: plain when it is a word or a few, otherwise in double
/// quotes (`text.js`'s `yamlText`)
fn unit_text(unit: &str) -> String {
    let mut chars = unit.chars();
    let plain = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || " _./-".contains(c))
        && !unit.ends_with(' ');
    if plain {
        unit.to_string()
    } else {
        serde_json::Value::String(unit.to_string()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // studio/tests/text.mjs's files and cases, byte for byte (ADR-075,
    // decision 4)
    const FILE: &str = r#"_forge_version: "5.1.0"
# a comment at the top
year:
  margin:
    formula: "=profit / revenue"   # share of revenue
    unit: ratio
  flow: {formula: "=a + b", unit: CAD}   # flow
  flow_last: {unit: CAD, formula: =a + b}
  plain: =a + b # plain
  single: '=IF(a, "y", "n")'
  price: 16   # dollars
  hours:
    better: lower
    value: 32   # per day
  rate: {value: 0.03, unit: ratio}
  tiny: 1e-5   # an exponent
  loss: -3
  hex: 0x10
  small:
    value: 1.5E+3
  flow_exp: {value: 2.5e-4, unit: ratio}
months:
  dozens: "=a * 2"
  profit: =revenue - costs
"#;

    /// `FILE` with the line holding `needle` replaced by `line`
    fn with_line(needle: &str, line: &str) -> String {
        let mut lines: Vec<&str> = FILE.split('\n').collect();
        let at = lines.iter().position(|l| l.contains(needle)).unwrap();
        lines[at] = line;
        lines.join("\n")
    }

    /// Text that is not YAML, and a name that is not there, for each
    /// function that reads a file
    #[test]
    fn text_that_is_not_yaml_or_has_not_the_group_is_refused_with_the_reason() {
        let broken = "its YAML cannot be read (Unclosed flow sequence. Expected: ']' to close sequence. Found: end of input. Context: in flow sequence. Suggestion: Add ']' to close the array, or check for missing commas between elements)".to_string();
        let no_group = "z is not written as a group of names".to_string();
        let names: &[&str] = &["z", "b"];
        let new = || New::Input {
            value: "1",
            unit: None,
        };
        assert_eq!(set_formula("a: [", &["a"], "=1"), Err(broken.clone()));
        assert_eq!(set_formula("a: 1\n", names, "=1"), Err(no_group.clone()));
        assert_eq!(add("a: [", &["a"], new(), None), Err(broken.clone()));
        assert_eq!(add("a: 1\n", names, new(), None), Err(no_group.clone()));
        assert_eq!(delete("a: [", &["a"]), Err(broken.clone()));
        assert_eq!(delete("a: 1\n", names), Err(no_group));
        assert_eq!(sites("a: [").err(), Some(broken));
    }

    #[test]
    fn a_name_with_nothing_after_it_is_a_site_of_its_name_and_an_empty_value() {
        let seen = |text: &str| -> Vec<(Vec<String>, bool, String)> {
            sites(text)
                .unwrap()
                .into_iter()
                .map(|s| (s.path, s.key, s.text))
                .collect()
        };
        let path = |names: &[&str]| names.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(
            seen("a:\nb: 1\n"),
            vec![
                (path(&["a"]), true, "a".to_string()),
                (path(&["a"]), false, String::new()),
                (path(&["b"]), true, "b".to_string()),
                (path(&["b"]), false, "1".to_string()),
            ]
        );
        assert_eq!(
            seen("x: {a, b: 1}\n"),
            vec![
                (path(&["x"]), true, "x".to_string()),
                (path(&["x", "a"]), true, "a".to_string()),
                (path(&["x", "a"]), false, String::new()),
                (path(&["x", "b"]), true, "b".to_string()),
                (path(&["x", "b"]), false, "1".to_string()),
            ]
        );
    }

    #[test]
    fn a_formula_keeps_the_files_quotes_and_the_rest_of_its_line() {
        let cases = [
            (
                ["year", "margin"],
                "=profit / revenue * 100",
                "formula: \"=profit",
                "    formula: \"=profit / revenue * 100\"   # share of revenue",
            ),
            (
                ["year", "flow"],
                "=(a + b)",
                "flow: {",
                "  flow: {formula: \"=(a + b)\", unit: CAD}   # flow",
            ),
            (
                ["year", "flow_last"],
                "=a * b",
                "flow_last",
                "  flow_last: {unit: CAD, formula: =a * b}",
            ),
            (
                ["year", "flow_last"],
                "=MAX(a, b)",
                "flow_last",
                "  flow_last: {unit: CAD, formula: \"=MAX(a, b)\"}",
            ),
            (
                ["year", "plain"],
                "=a - b",
                "plain: =",
                "  plain: =a - b # plain",
            ),
            (
                ["year", "plain"],
                "=IF(a, \"x\", \"y\")",
                "plain: =",
                "  plain: \"=IF(a, \\\"x\\\", \\\"y\\\")\" # plain",
            ),
            (
                ["year", "plain"],
                "=a #b",
                "plain: =",
                "  plain: \"=a #b\" # plain",
            ),
            (
                ["year", "plain"],
                "=a\t#b",
                "plain: =",
                "  plain: \"=a\t#b\" # plain",
            ),
            (
                ["year", "single"],
                "=IF(a, \"it's\", \"n\")",
                "single:",
                "  single: '=IF(a, \"it''s\", \"n\")'",
            ),
            (
                ["months", "dozens"],
                "=a & \"\\\"",
                "dozens:",
                "  dozens: \"=a & \\\"\\\\\\\"\"",
            ),
            (
                ["months", "profit"],
                "=revenue - costs - 1",
                "profit: =revenue",
                "  profit: =revenue - costs - 1",
            ),
        ];
        for (names, formula, needle, line) in cases {
            assert_eq!(
                set_formula(FILE, &names, formula),
                Ok(with_line(needle, line)),
                "{formula}"
            );
        }
    }

    #[test]
    fn a_value_is_written_as_given_and_the_rest_of_its_line_stays() {
        let cases = [
            ("price", "17", "price:", "  price: 17   # dollars"),
            ("hours", "30", "value: 32", "    value: 30   # per day"),
            (
                "rate",
                "0.04",
                "rate:",
                "  rate: {value: 0.04, unit: ratio}",
            ),
            (
                "tiny",
                "0.00002",
                "tiny:",
                "  tiny: 0.00002   # an exponent",
            ),
            ("loss", "-4", "loss:", "  loss: -4"),
            ("hex", "17", "hex:", "  hex: 17"),
            ("small", "1600", "value: 1.5E+3", "    value: 1600"),
            (
                "flow_exp",
                "1e-7",
                "flow_exp:",
                "  flow_exp: {value: 1e-7, unit: ratio}",
            ),
        ];
        for (name, value, needle, line) in cases {
            assert_eq!(
                set_value(FILE, &["year", name], value),
                Ok(with_line(needle, line)),
                "{name}"
            );
        }
        // `value:` with nothing after it gets its space
        assert_eq!(
            set_value("a:\n  value:\n  unit: x\n", &["a"], "5"),
            Ok("a:\n  value: 5\n  unit: x\n".to_string())
        );
        // A top-level input
        assert_eq!(
            set_value("rate: 0.05 # r\n", &["rate"], "0.06"),
            Ok("rate: 0.06 # r\n".to_string())
        );
    }

    #[test]
    fn what_is_not_written_where_edit_can_find_it_is_said() {
        let missing = |name: &str, key: &str| {
            Err(format!(
                "{name} has no {key}: written where edit can change it"
            ))
        };
        assert_eq!(
            set_value(FILE, &["year", "nothing"], "1"),
            missing("nothing", "value")
        );
        assert_eq!(
            set_value(FILE, &["year", "margin"], "1"),
            missing("margin", "value")
        );
        assert_eq!(
            set_formula("t:\n  c: [1, 2]\n", &["t", "c"], "=1"),
            missing("c", "formula")
        );
        assert_eq!(
            set_value(FILE, &["nowhere", "price"], "1"),
            Err("nowhere is not written as a group of names".to_string())
        );
        assert_eq!(
            set_value(FILE, &[], "1"),
            Err("no name was given".to_string())
        );
        assert_eq!(
            set_value("- 1\n- 2\n", &["a"], "1"),
            Err("it is not a mapping of names".to_string())
        );
        assert_eq!(
            set_value("a: \"1\n", &["a"], "1"),
            Err("its YAML cannot be read (Unterminated quoted string)".to_string())
        );
        assert_eq!(
            set_value("a: 1\n---\na: 2\n", &["a"], "1"),
            Err("it holds several YAML documents, which edit does not change".to_string())
        );
    }

    // text.mjs's file for adding and deleting
    const TOP: &str = r#"_forge_version: "5.1.0"
rate: 0.05   # top level
growth:
  formula: "=rate * 2"

# a comment before the group
year:
  margin:
    formula: "=profit / revenue"   # share of revenue
  price: 16
  hours:
    value: 32

months:
  month: [1, 2, 3]
  # a comment above a column
  sales: "=month * 10"
  list:
  - 1
  - 2
  - 3
four:
    deep: 1
    other: 2
"#;

    /// `TOP` with `new` inserted after the line `after`
    fn inserted(after: &str, new: &[&str]) -> String {
        let mut lines: Vec<&str> = TOP.split('\n').collect();
        let at = lines.iter().position(|l| *l == after).unwrap() + 1;
        for (k, line) in new.iter().enumerate() {
            lines.insert(at + k, line);
        }
        lines.join("\n")
    }

    /// A name, what is added, `after`, the new lines, and the line they follow
    type AddCase<'a> = (
        &'a [&'a str],
        New<'a>,
        Option<&'a str>,
        &'a [&'a str],
        &'a str,
    );

    fn added_and_deleted(cases: &[AddCase<'_>]) {
        for &(names, new, after, lines, after_line) in cases {
            let added = add(TOP, names, new, after).unwrap();
            assert_eq!(added, inserted(after_line, lines), "{names:?}");
            assert_eq!(delete(&added, names), Ok(TOP.to_string()), "{names:?}");
        }
    }

    #[test]
    fn an_input_or_formula_is_added_in_the_files_style_and_deleted_back() {
        let input = |value| New::Input { value, unit: None };
        let cases: [AddCase<'_>; 5] = [
            (
                &["year", "tax"],
                input("0.15"),
                None,
                &["  tax: 0.15"],
                "    value: 32",
            ),
            (
                &["year", "fee"],
                New::Input {
                    value: "1e-7",
                    unit: Some("CAD"),
                },
                None,
                &["  fee:", "    value: 1e-7", "    unit: CAD"],
                "    value: 32",
            ),
            (
                &["year", "share"],
                New::Input {
                    value: "-3",
                    unit: Some("%"),
                },
                None,
                &["  share:", "    value: -3", "    unit: \"%\""],
                "    value: 32",
            ),
            (
                &["year", "profit_share"],
                New::Formula {
                    formula: "=profit / 2",
                    unit: None,
                },
                None,
                &["  profit_share:", "    formula: \"=profit / 2\""],
                "    value: 32",
            ),
            (
                &["year", "label"],
                New::Formula {
                    formula: "=IF(a, \"x\", \"y\")",
                    unit: Some("text"),
                },
                None,
                &[
                    "  label:",
                    "    formula: \"=IF(a, \\\"x\\\", \\\"y\\\")\"",
                    "    unit: text",
                ],
                "    value: 32",
            ),
        ];
        added_and_deleted(&cases);
    }

    #[test]
    fn a_column_or_a_name_at_the_top_level_is_added_and_deleted_back() {
        let input = |value| New::Input { value, unit: None };
        let cases: [AddCase<'_>; 5] = [
            (
                &["months", "double"],
                New::Column {
                    formula: "=sales * 2",
                },
                None,
                &["  double: \"=sales * 2\""],
                "  - 3",
            ),
            (
                &["four", "more"],
                input("3"),
                None,
                &["    more: 3"],
                "    other: 2",
            ),
            // A group indented by 4 indents what is under the new name by 4 more
            (
                &["four", "fee"],
                New::Input {
                    value: "2",
                    unit: Some("CAD"),
                },
                None,
                &["    fee:", "        value: 2", "        unit: CAD"],
                "    other: 2",
            ),
            (
                &["tax"],
                input("0.2"),
                Some("rate"),
                &["tax: 0.2"],
                "rate: 0.05   # top level",
            ),
            (
                &["x"],
                New::Formula {
                    formula: "=rate",
                    unit: None,
                },
                Some("growth"),
                &["x:", "  formula: \"=rate\""],
                "  formula: \"=rate * 2\"",
            ),
        ];
        added_and_deleted(&cases);
    }

    #[test]
    fn a_name_added_at_the_end_of_a_file_without_a_last_line_break_gets_one_before_it() {
        assert_eq!(
            add(
                "g:\n  a: 1",
                &["g", "b"],
                New::Input {
                    value: "2",
                    unit: None
                },
                None
            ),
            Ok("g:\n  a: 1\n  b: 2".to_string())
        );
    }

    #[test]
    fn an_addition_edit_cannot_place_is_refused() {
        let new = New::Input {
            value: "1",
            unit: None,
        };
        assert_eq!(
            add("g: {a: 1}\n", &["g", "b"], new, None),
            Err("its group is written on one line ({...}); edit adds to a block".to_string())
        );
        assert_eq!(
            add(TOP, &["x"], new, None),
            Err("a name at the top level needs `after`: the name it follows".to_string())
        );
        assert_eq!(
            add(TOP, &["x"], new, Some("nothing")),
            Err("nothing is not a name at the top level".to_string())
        );
        assert_eq!(
            add(TOP, &["year"], new, Some("rate")),
            Err("year is already written there".to_string())
        );
        assert_eq!(
            add("g: {}\nh: 1\n", &["g", "b"], new, None),
            Err("its group is written on one line ({...}); edit adds to a block".to_string())
        );
    }

    #[test]
    fn deleting_takes_its_lines_and_keeps_the_comment_above() {
        let without = |from: &str, to: &str| {
            let lines: Vec<&str> = TOP.split('\n').collect();
            let a = lines.iter().position(|l| l.contains(from)).unwrap();
            let b = lines.iter().position(|l| l.contains(to)).unwrap();
            let kept: Vec<&str> = lines
                .iter()
                .enumerate()
                .filter(|(k, _)| *k < a || *k > b)
                .map(|(_, l)| *l)
                .collect();
            kept.join("\n")
        };
        let cases: [(&[&str], &str, &str); 5] = [
            (&["year", "margin"], "margin:", "share of revenue"),
            (&["year", "hours"], "hours:", "value: 32"),
            (&["months", "sales"], "sales:", "sales:"),
            (&["months", "list"], "list:", "- 3"),
            (&["rate"], "rate:", "rate:"),
        ];
        for (names, from, to) in cases {
            assert_eq!(delete(TOP, names), Ok(without(from, to)), "{names:?}");
        }
        // In a flow mapping, the rest of the line stays as it was
        assert_eq!(
            delete(
                "p:\n  hold: {hold: =m.R, cut: =m.S}\n",
                &["p", "hold", "cut"]
            ),
            Ok("p:\n  hold: {hold: =m.R}\n".to_string())
        );
    }

    #[test]
    fn a_deletion_edit_cannot_make_is_refused() {
        assert_eq!(
            delete(TOP, &["year", "nothing"]),
            Err("nothing is not written where edit can find it".to_string())
        );
        assert_eq!(
            delete("g:\n  only: 1\nh: 2\n", &["g", "only"]),
            Err("only is the last name of its group, which would be left empty".to_string())
        );
    }

    #[test]
    fn names_in_any_alphabet_are_found() {
        let text = "hypothèses:\n  coût_unitaire: 3   # €\n假设:\n  单价: 5\n  总价: =假设.单价 * 2 # 合计\n";
        assert_eq!(
            set_value(text, &["hypothèses", "coût_unitaire"], "4"),
            Ok(text.replace("coût_unitaire: 3", "coût_unitaire: 4"))
        );
        assert_eq!(
            set_formula(text, &["假设", "总价"], "=假设.单价 * 3"),
            Ok(text.replace("* 2 #", "* 3 #"))
        );
        assert_eq!(
            add(
                text,
                &["假设", "数量"],
                New::Input {
                    value: "7",
                    unit: Some("个")
                },
                None
            ),
            Ok(format!("{text}  数量:\n    value: 7\n    unit: \"个\"\n"))
        );
    }

    #[test]
    fn every_key_and_value_is_found_where_it_is_written() {
        // A key that is not a scalar, and an alias, are not sites
        let text = "a: 1   # one\ng:\n  'x': \"=a + 1\"\n  f: {formula: =a, unit: CAD}\nl:\n  - p: |\n      =a\n  - [q, 'r']\n? [k]\n: 2\nm: &n 3\no: *n\n";
        let shown: Vec<String> = sites(text)
            .unwrap()
            .iter()
            .map(|s| {
                format!(
                    "{}{} {:?} {}{}",
                    if s.key { "key " } else { "" },
                    s.path.join("/"),
                    &text[s.range.clone()],
                    s.text.trim_end(),
                    if s.flow { " flow" } else { "" },
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                "key a \"a\" a",
                "a \"1\" 1",
                "key g \"g\" g",
                "key g/x \"'x'\" x",
                "g/x \"\\\"=a + 1\\\"\" =a + 1",
                "key g/f \"f\" f",
                "key g/f/formula \"formula\" formula flow",
                "g/f/formula \"=a\" =a flow",
                "key g/f/unit \"unit\" unit flow",
                "g/f/unit \"CAD\" CAD flow",
                "key l \"l\" l",
                "key l/#/p \"p\" p",
                "l/#/p \"|\\n      =a\\n\" =a",
                "l/#/# \"q\" q flow",
                "l/#/# \"'r'\" r flow",
                "key m \"m\" m",
                "m \"3\" 3",
                "key o \"o\" o",
            ]
        );
        assert_eq!(
            splice("abcdef", vec![(4..5, "E".into()), (0..2, "AB!".into())]),
            "AB!cdEf"
        );
        assert_eq!(
            sites("- 1\n"),
            Err("it is not a mapping of names".to_string())
        );
        assert_eq!(name_text("'a'", "b"), "'b'");
        assert_eq!(name_text("\"a\"", "b"), "\"b\"");
        assert_eq!(name_text("a", "b"), "b");
        let renamed = |raw| renamed_formula(raw, "=a + 'x'", "=bb + 'x'");
        assert_eq!(renamed("=a + 'x'").as_deref(), Some("=bb + 'x'"));
        assert_eq!(renamed("'=a + ''x'''").as_deref(), Some("'=bb + ''x'''"));
        assert_eq!(renamed("\"=a + 'x'\"").as_deref(), Some("\"=bb + 'x'\""));
        // Folded over several lines
        assert_eq!(renamed("=a +\n  'x'"), None);
        assert_eq!(renamed("|\n  =a + 'x'\n"), None);
    }

    #[test]
    fn a_unit_is_plain_only_when_yaml_reads_it_as_written() {
        assert_eq!(unit_text("CAD"), "CAD");
        assert_eq!(unit_text("km/h per day"), "km/h per day");
        assert_eq!(unit_text("trailing "), "\"trailing \"");
        assert_eq!(unit_text("%"), "\"%\"");
        assert_eq!(unit_text("a:b"), "\"a:b\"");
        assert_eq!(unit_text(""), "\"\"");
    }

    #[test]
    fn a_formula_is_plain_only_when_that_is_safe() {
        let plain = |f| encode_formula(f, Style::Plain, false);
        assert_eq!(plain("=a + b"), "=a + b");
        assert_eq!(plain("=a: b"), "\"=a: b\"");
        assert_eq!(plain(" =a"), "\" =a\"");
        assert_eq!(plain("=a "), "\"=a \"");
        assert_eq!(plain("=`a`"), "\"=`a`\"");
        assert_eq!(plain("=a,b"), "=a,b");
        assert_eq!(encode_formula("=a,b", Style::Plain, true), "\"=a,b\"");
        assert_eq!(encode_formula("=it's", Style::Single, false), "'=it''s'");
        assert_eq!(encode_formula("=a", Style::Double, false), "\"=a\"");
    }
}
