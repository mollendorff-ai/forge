//! Edit a model's YAML text in place (ADR-051).
//!
//! `forge calculate` and `forge upgrade` change a few values in a file the
//! author wrote by hand. Serializing the whole document again would delete
//! comments, quotes and blank lines, so these functions edit the text itself,
//! line by line, and then read the result back to check that nothing else
//! changed. If a scalar is written in a layout they do not handle, they return
//! an error and the caller leaves the file alone.

use yaml_serde::Value;

/// A scalar's place in the file: `["year", "profit"]` or `["total"]`.
pub type ScalarPath = Vec<String>;

/// Set the `value:` of each scalar, adding it after `formula:` when missing.
///
/// Returns the new text. Every other byte is kept.
///
/// # Errors
///
/// Returns an error if a scalar cannot be found, is written in a layout this
/// editor does not handle, or if the edited text does not read back as the
/// original with only those values changed.
pub fn set_values(text: &str, values: &[(ScalarPath, f64)]) -> Result<String, String> {
    let mut lines: Vec<String> = split_lines(text);
    for (path, value) in values {
        set_one(&mut lines, path, &format_number(*value))?;
    }
    let edited = lines.concat();
    verify_values(text, &edited, values)?;
    Ok(edited)
}

/// Rewrite a 5.0.0 model in the 5.1.0 short form (ADR-051).
///
/// - `name: {value: 16, formula: null}` (block or flow, no metadata) becomes `name: 16`
/// - `formula: null` lines are removed from inputs that keep metadata
/// - `value: null` lines are removed from formula scalars
/// - `_forge_version` becomes `"5.1.0"`
///
/// Comments on the removed lines are lost; every other line is kept as typed.
///
/// # Errors
///
/// Returns an error if the converted text does not parse to the same scalars,
/// tables and settings as the original.
pub fn to_short_form(text: &str) -> Result<String, String> {
    let lines = split_lines(text);
    let scalar_sections = scalar_sections(text);
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut section = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if indent(line) == 0 && is_content(line) {
            section = key_of(line).map(|(k, _)| k).unwrap_or_default();
        }
        if indent(line) == 0 && key_of(line).is_some_and(|(k, _)| k == "_forge_version") {
            out.push(replace_after_colon(line, "\"5.1.0\""));
            i += 1;
            continue;
        }
        // Only scalars change: a top-level scalar, or a direct child of a
        // group of scalars. Tables and the analysis sections are kept as typed.
        let depth_ok = match scalar_sections.get(&section) {
            Some(ScalarSection::Scalar) => indent(line) == 0,
            Some(ScalarSection::Group(child_indent)) => indent(line) == *child_indent,
            None => false,
        };
        let in_scalar_block = match scalar_sections.get(&section) {
            Some(ScalarSection::Scalar) => indent(line) > 0,
            Some(ScalarSection::Group(child_indent)) => indent(line) > *child_indent,
            None => false,
        };
        if !depth_ok && !in_scalar_block {
            out.push(line.clone());
            i += 1;
            continue;
        }
        // A key with a block mapping of only `value:` (and `formula: null`)
        if let Some((name, rest)) = key_of(line).filter(|_| depth_ok) {
            let rest_trim = strip_comment(rest).trim();
            if rest_trim.is_empty() {
                let end = block_end(&lines, i);
                let children: Vec<usize> =
                    (i + 1..end).filter(|&j| is_content(&lines[j])).collect();
                let keys: Vec<(String, String)> = children
                    .iter()
                    .filter_map(|&j| {
                        key_of(&lines[j]).map(|(k, r)| (k, strip_comment(r).trim().to_string()))
                    })
                    .collect();
                // `value: <number>` alone, or with `formula: null`: an input
                let has_number = keys.iter().any(|(k, r)| k == "value" && is_plain_number(r));
                let only_value_and_null_formula = has_number
                    && (keys.len() == 1
                        || (keys.len() == 2
                            && keys.iter().any(|(k, r)| k == "formula" && is_null(r))));
                if only_value_and_null_formula
                    && children
                        .iter()
                        .all(|&j| indent(&lines[j]) == indent(&lines[children[0]]))
                {
                    let value = keys
                        .iter()
                        .find(|(k, _)| k == "value")
                        .map_or("", |(_, v)| v.as_str());
                    let ending = line_ending(line);
                    let head = &line[..line.len() - ending.len()];
                    let comment = comment_of(head);
                    let key_part = &head[..head.find(':').map_or(head.len(), |c| c + 1)];
                    let comment = if comment.is_empty() {
                        String::new()
                    } else {
                        format!("  {comment}")
                    };
                    out.push(format!("{key_part} {value}{comment}{ending}"));
                    // keep comment-only and blank lines that were inside the block
                    for inner in &lines[i + 1..end] {
                        if !is_content(inner) {
                            out.push(inner.clone());
                        }
                    }
                    let _ = name;
                    i = end;
                    continue;
                }
            } else if rest_trim.starts_with('{') && rest_trim.ends_with('}') {
                if let Some(short) = flow_input_to_number(rest_trim) {
                    out.push(replace_after_colon(line, &short));
                    i += 1;
                    continue;
                }
            }
        }
        // Drop `formula: null` and `value: null` lines inside scalar blocks
        if in_scalar_block {
            if let Some((k, r)) = key_of(line) {
                let r = strip_comment(r).trim().to_string();
                if (k == "formula" || k == "value") && is_null(&r) && comment_of(line).is_empty() {
                    i += 1;
                    continue;
                }
            }
        }
        out.push(line.clone());
        i += 1;
    }
    let converted = out.concat();
    verify_same_model(text, &converted)?;
    Ok(converted)
}

/// Where scalars live in a model: a top-level scalar, or a group of scalars
/// whose children are at the given indent.
enum ScalarSection {
    Scalar,
    Group(usize),
}

/// Top-level keys that hold scalars (not tables, not analysis sections).
const NOT_SCALARS: [&str; 11] = [
    "_forge_version",
    "_name",
    "_includes",
    "scenarios",
    "monte_carlo",
    "tornado",
    "decision_tree",
    "real_options",
    "bootstrap",
    "bayesian_network",
    "_metadata",
];

fn scalar_sections(text: &str) -> std::collections::HashMap<String, ScalarSection> {
    let mut out = std::collections::HashMap::new();
    let Ok(Value::Mapping(root)) = yaml_serde::from_str::<Value>(text) else {
        return out;
    };
    let lines = split_lines(text);
    for (key, value) in &root {
        let Some(key) = key.as_str() else { continue };
        if NOT_SCALARS.contains(&key) {
            continue;
        }
        match value {
            Value::Mapping(m) if m.contains_key("value") || m.contains_key("formula") => {
                if !crate::parser::model::is_table(m) {
                    out.insert(key.to_string(), ScalarSection::Scalar);
                }
            },
            Value::Mapping(m) if !crate::parser::model::is_table(m) => {
                let is_group = m.values().any(|v| {
                    matches!(v, Value::Number(_))
                        || matches!(v, Value::Mapping(c) if c.contains_key("value") || c.contains_key("formula"))
                });
                if is_group {
                    let start = lines
                        .iter()
                        .position(|l| indent(l) == 0 && key_of(l).is_some_and(|(k, _)| k == key));
                    if let Some(start) = start {
                        let child = (start + 1..lines.len())
                            .find(|&j| is_content(&lines[j]))
                            .map(|j| indent(&lines[j]));
                        if let Some(child) = child.filter(|c| *c > 0) {
                            out.insert(key.to_string(), ScalarSection::Group(child));
                        }
                    }
                }
            },
            _ => {},
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Finding and editing a scalar
// ─────────────────────────────────────────────────────────────────────────────

fn set_one(lines: &mut Vec<String>, path: &[String], value: &str) -> Result<(), String> {
    let name = path.join(".");
    let scalar =
        find_path(lines, path).ok_or_else(|| format!("'{name}' was not found in the file"))?;
    let line = lines[scalar].clone();
    let (_, rest) = key_of(&line).ok_or_else(|| format!("'{name}' is not a key"))?;
    let rest_trim = strip_comment(rest).trim().to_string();

    if rest_trim.is_empty() {
        // Block mapping: change `value:` or add it after `formula:`
        let end = block_end(lines, scalar);
        let children: Vec<usize> = (scalar + 1..end)
            .filter(|&j| is_content(&lines[j]))
            .collect();
        let Some(&first) = children.first() else {
            return Err(format!("'{name}' has an empty block"));
        };
        let child_indent = indent(&lines[first]);
        let direct: Vec<usize> = children
            .iter()
            .copied()
            .filter(|&j| indent(&lines[j]) == child_indent)
            .collect();
        let key_line = |key: &str| {
            direct
                .iter()
                .copied()
                .find(|&j| key_of(&lines[j]).is_some_and(|(k, _)| k == key))
        };
        if let Some(v) = key_line("value") {
            let current = key_of(&lines[v]).map(|(_, r)| strip_comment(r).trim().to_string());
            if !current.is_some_and(|c| same_number(&c, value)) {
                lines[v] = replace_after_colon(&lines[v], value);
            }
            return Ok(());
        }
        let formula = key_line("formula");
        let after = match formula {
            Some(f) if is_single_line_value(&lines[f]) => f,
            _ => *children.last().unwrap_or(&first),
        };
        let ending = line_ending(&lines[after]).to_string();
        let ending = if ending.is_empty() {
            "\n".to_string()
        } else {
            ending
        };
        if line_ending(&lines[after]).is_empty() {
            lines[after].push_str(&ending);
        }
        lines.insert(
            after + 1,
            format!("{}value: {value}{ending}", " ".repeat(child_indent)),
        );
        return Ok(());
    }

    if rest_trim.starts_with('{') && rest_trim.ends_with('}') {
        let edited = set_flow_value(&rest_trim, value)
            .ok_or_else(|| format!("'{name}' uses a flow mapping Forge cannot edit"))?;
        lines[scalar] = replace_after_colon(&line, &edited);
        return Ok(());
    }

    Err(format!(
        "'{name}' is not written as a mapping with `formula:`, so its result has nowhere to go"
    ))
}

/// The line index of the key at `path`, starting from the top level.
fn find_path(lines: &[String], path: &[String]) -> Option<usize> {
    let mut start = 0;
    let mut end = lines.len();
    let mut want_indent = Some(0);
    let mut found = None;
    for part in path {
        let child_indent = match want_indent {
            Some(i) => i,
            None => (start..end)
                .find(|&j| is_content(&lines[j]))
                .map(|j| indent(&lines[j]))?,
        };
        let idx = (start..end).find(|&j| {
            is_content(&lines[j])
                && indent(&lines[j]) == child_indent
                && key_of(&lines[j]).is_some_and(|(k, _)| k == *part)
        })?;
        found = Some(idx);
        start = idx + 1;
        end = block_end(lines, idx);
        want_indent = None;
    }
    found
}

/// One past the last content line of the block under the key at `i`.
fn block_end(lines: &[String], i: usize) -> usize {
    let base = indent(&lines[i]);
    let mut last = i;
    for (j, line) in lines.iter().enumerate().skip(i + 1) {
        if !is_content(line) {
            continue;
        }
        if indent(line) <= base {
            break;
        }
        last = j;
    }
    last + 1
}

/// Change the `value` entry of a one-line flow mapping, or add it.
fn set_flow_value(flow: &str, value: &str) -> Option<String> {
    let inner = &flow[1..flow.len() - 1];
    let mut entries = split_flow(inner)?;
    let mut set = false;
    for entry in &mut entries {
        let trimmed = entry.trim_start();
        if trimmed.starts_with("value:") || trimmed.starts_with("\"value\":") {
            let current = trimmed.split_once(':').map_or("", |(_, v)| v.trim());
            if !same_number(current, value) {
                let lead = &entry[..entry.len() - trimmed.len()];
                *entry = format!("{lead}value: {value}");
            }
            set = true;
        }
    }
    if !set {
        entries.push(format!(" value: {value}"));
    }
    let body = entries.join(",");
    let body = body.trim_end();
    let close = if inner.ends_with(' ') { " }" } else { "}" };
    Some(format!("{{{body}{close}"))
}

/// `{value: 16, formula: null}` as `16`; `None` if it has anything else.
fn flow_input_to_number(flow: &str) -> Option<String> {
    let entries = split_flow(&flow[1..flow.len() - 1])?;
    let mut value = None;
    let mut formula_null = false;
    for entry in &entries {
        let (k, v) = entry.split_once(':')?;
        let (k, v) = (k.trim(), v.trim());
        match k {
            "value" if is_plain_number(v) => value = Some(v.to_string()),
            "formula" if is_null(v) => formula_null = true,
            _ => return None,
        }
    }
    if entries.len() == 1 || (entries.len() == 2 && formula_null) {
        value
    } else {
        None
    }
}

/// Split the inside of a flow mapping at top-level commas.
fn split_flow(inner: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0i32;
    for c in inner.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => {
                quote = None;
                current.push(c);
            },
            (None, '"' | '\'') => {
                quote = Some(c);
                current.push(c);
            },
            (None, '{' | '[') => {
                depth += 1;
                current.push(c);
            },
            (None, '}' | ']') => {
                depth -= 1;
                current.push(c);
            },
            (None, ',') if depth == 0 => out.push(std::mem::take(&mut current)),
            (_, c) => current.push(c),
        }
    }
    if quote.is_some() || depth != 0 {
        return None;
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    Some(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// Line helpers
// ─────────────────────────────────────────────────────────────────────────────

fn split_lines(text: &str) -> Vec<String> {
    text.split_inclusive('\n').map(str::to_string).collect()
}

fn line_ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_content(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && !t.starts_with('#') && t != "---" && t != "..."
}

/// The key and the text after its colon, for a `key: ...` line.
fn key_of(line: &str) -> Option<(String, &str)> {
    let t = line.trim_start_matches(' ');
    let t = t.trim_end_matches(['\n', '\r']);
    if t.starts_with('#') || t.starts_with('-') {
        return None;
    }
    let (key, rest) = if let Some(q) = t.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let close = t[1..].find(q)? + 1;
        let after = &t[close + 1..];
        let rest = after.strip_prefix(':')?;
        (t[1..close].to_string(), rest)
    } else {
        let colon = t.find(": ").or_else(|| t.strip_suffix(':').map(str::len))?;
        (t[..colon].trim().to_string(), &t[colon + 1..])
    };
    Some((key, rest))
}

/// The text before a ` #` comment, outside quotes.
fn strip_comment(s: &str) -> &str {
    let mut quote: Option<char> = None;
    let bytes: Vec<char> = s.chars().collect();
    let mut byte_pos = 0;
    for (i, c) in bytes.iter().enumerate() {
        match (quote, *c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(*c),
            (None, '#') if i == 0 || bytes[i - 1].is_whitespace() => return &s[..byte_pos],
            _ => {},
        }
        byte_pos += c.len_utf8();
    }
    s
}

/// The ` # comment` at the end of a line, if any (without the line ending).
fn comment_of(line: &str) -> String {
    let body = line.trim_end_matches(['\n', '\r']);
    let before = strip_comment(body);
    body[before.len()..].trim().to_string()
}

/// Replace what follows `key:` on a line, keeping indent, comment and ending.
fn replace_after_colon(line: &str, new_value: &str) -> String {
    let ending = line_ending(line);
    let body = &line[..line.len() - ending.len()];
    let lead = indent(body);
    let t = &body[lead..];
    let colon = if t.starts_with('"') || t.starts_with('\'') {
        let q = t.chars().next().unwrap_or('"');
        t[1..].find(q).map_or(0, |c| c + 2)
    } else {
        t.find(':').unwrap_or(0)
    };
    let key = &body[..=(lead + colon)];
    let comment = comment_of(body);
    let comment = if comment.is_empty() {
        String::new()
    } else {
        format!("  {comment}")
    };
    format!("{key} {new_value}{comment}{ending}")
}

fn is_single_line_value(line: &str) -> bool {
    key_of(line).is_some_and(|(_, r)| {
        let r = strip_comment(r).trim();
        !r.is_empty() && !r.starts_with('|') && !r.starts_with('>')
    })
}

/// Two YAML scalars that are the same number (`16` and `16.0`).
#[allow(clippy::float_cmp)] // exactly the same number, not a close one
fn same_number(a: &str, b: &str) -> bool {
    match (
        yaml_serde::from_str::<f64>(a),
        yaml_serde::from_str::<f64>(b),
    ) {
        (Ok(x), Ok(y)) => x == y || (x.is_nan() && y.is_nan()),
        _ => false,
    }
}

fn is_null(s: &str) -> bool {
    matches!(s, "null" | "~" | "Null" | "NULL")
}

fn is_plain_number(s: &str) -> bool {
    !s.is_empty() && s.parse::<f64>().is_ok() && !s.contains(['"', '\''])
}

/// A number as YAML that reads back to the same `f64`.
#[must_use]
pub fn format_number(v: f64) -> String {
    if v.is_nan() {
        return ".nan".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { ".inf" } else { "-.inf" }.to_string();
    }
    #[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let plain = format!("{v}");
    if plain.len() <= 24 {
        return plain;
    }
    // Very large or very small: scientific with a decimal point, which YAML
    // 1.1 and 1.2 both read as a float.
    let sci = format!("{v:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.0")
    };
    let exp = exp.parse::<i32>().unwrap_or(0);
    format!("{mantissa}e{exp:+}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Checks
// ─────────────────────────────────────────────────────────────────────────────

fn parse(text: &str) -> Result<Value, String> {
    yaml_serde::from_str(text).map_err(|e| format!("the edited file does not parse: {e}"))
}

/// The edited text must equal the original with only these values set.
fn verify_values(original: &str, edited: &str, values: &[(ScalarPath, f64)]) -> Result<(), String> {
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
        map.insert(Value::from("value"), Value::from(*value));
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
fn verify_same_model(original: &str, converted: &str) -> Result<(), String> {
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
    if format!("{:?}", a.scenarios.len()) != format!("{:?}", b.scenarios.len()) {
        return Err("the converted file has different scenarios".to_string());
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
        let normalized = if NOT_SCALARS.contains(&name) {
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

    fn paths(items: &[(&str, f64)]) -> Vec<(ScalarPath, f64)> {
        items
            .iter()
            .map(|(p, v)| (p.split('.').map(str::to_string).collect(), *v))
            .collect()
    }

    #[test]
    fn adds_value_after_formula_and_keeps_everything_else() {
        let text = "_forge_version: \"5.1.0\"\n# My plan. Keep this.\nassumptions:\n  price: 16   # per dozen\n\nyear:\n  revenue:\n    formula: \"=assumptions.price * 100\"  # yearly\n    unit: CAD\n";
        let out = set_values(text, &paths(&[("year.revenue", 1600.0)])).unwrap();
        assert_eq!(
            out,
            "_forge_version: \"5.1.0\"\n# My plan. Keep this.\nassumptions:\n  price: 16   # per dozen\n\nyear:\n  revenue:\n    formula: \"=assumptions.price * 100\"  # yearly\n    value: 1600\n    unit: CAD\n"
        );
    }

    #[test]
    fn replaces_an_existing_value_and_keeps_its_comment() {
        let text = "_forge_version: \"5.0.0\"\nout:\n  total:\n    value: null   # filled by forge\n    formula: \"=1 + 1\"\n";
        let out = set_values(text, &paths(&[("out.total", 2.5)])).unwrap();
        assert_eq!(
            out,
            "_forge_version: \"5.0.0\"\nout:\n  total:\n    value: 2.5  # filled by forge\n    formula: \"=1 + 1\"\n"
        );
    }

    #[test]
    fn edits_flow_mappings() {
        let text = "x:\n  a: {formula: \"=1, 2\"}\n  b: {value: null, formula: \"=3\"}\n";
        let out = set_values(text, &paths(&[("x.a", 1.0), ("x.b", 3.0)])).unwrap();
        assert_eq!(
            out,
            "x:\n  a: {formula: \"=1, 2\", value: 1}\n  b: {value: 3, formula: \"=3\"}\n"
        );
    }

    #[test]
    fn top_level_scalar() {
        let text = "total:\n  formula: \"=2 * 3\"\n";
        assert_eq!(
            set_values(text, &paths(&[("total", 6.0)])).unwrap(),
            "total:\n  formula: \"=2 * 3\"\n  value: 6\n"
        );
    }

    #[test]
    fn a_bare_number_has_nowhere_to_put_a_result() {
        let err = set_values("x:\n  a: 5\n", &paths(&[("x.a", 6.0)])).unwrap_err();
        assert!(err.contains("nowhere to go"), "{err}");
    }

    #[test]
    #[allow(clippy::float_cmp)] // exact round trip is the point
    fn numbers_read_back_exactly() {
        for v in [
            0.1,
            1.0 / 3.0,
            -42.0,
            1e-9,
            6.02e23,
            1e15,
            -0.0,
            123_456.789,
        ] {
            let s = format_number(v);
            let back: f64 = yaml_serde::from_str(&s).unwrap();
            assert!(
                back.to_bits() == v.to_bits() || back == v,
                "{v} -> {s} -> {back}"
            );
        }
    }

    #[test]
    fn short_form_leaves_analysis_sections_and_tables_alone() {
        let text = "_forge_version: \"5.0.0\"\ndecision_tree:\n  name: t\n  root:\n    type: decision\n    branches:\n      stay:\n        value: 0\n      go:\n        cost: 5\n        value: 10\nsales:\n  amount:\n    value: [1, 2]\n    unit: CAD\n";
        let out = to_short_form(text).unwrap();
        assert_eq!(out, text.replace("\"5.0.0\"", "\"5.1.0\""));
    }

    #[test]
    fn short_form_conversion_keeps_comments_and_meaning() {
        let text = "_forge_version: \"5.0.0\"\n# inputs\nassumptions:\n  price:   # per dozen\n    value: 16\n    formula: null\n  rate: {value: 0.03, formula: null}\n  rent:\n    value: 4800\n    formula: null\n    unit: CAD\nyear:\n  total:\n    value: null\n    formula: \"=assumptions.price * 2\"\n";
        let out = to_short_form(text).unwrap();
        assert_eq!(
            out,
            "_forge_version: \"5.1.0\"\n# inputs\nassumptions:\n  price: 16  # per dozen\n  rate: 0.03\n  rent:\n    value: 4800\n    unit: CAD\nyear:\n  total:\n    formula: \"=assumptions.price * 2\"\n"
        );
    }
}
