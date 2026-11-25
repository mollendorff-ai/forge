//! A table's column of values, changed with `set_value` and a list, one value
//! for each row (ADR-075).
//!
//! `products.price` with `value: [20, 45]` writes each row whose value
//! changes and leaves the others' bytes as they are (`20.00` stays `20.00`),
//! in the list's own style, flow or block, comments kept. A column of
//! numbers, of dates or of text is changed, each text in the quotes its row
//! has (`text::set_text_column`); a column's rows are not added or removed
//! (every column of a table has as many), and a column of true and false or
//! with an empty cell is refused, saying so.

use super::{text, value_text, Change, Target, Value};
use crate::types::ColumnValue;

/// The new text of the target's file, its column of values set to the
/// change's list
pub(super) fn set(
    text: &str,
    target: &Target<'_>,
    names: &[&str],
    change: &Change,
) -> Result<String, String> {
    let (table, column) = target
        .name
        .rsplit_once('.')
        .expect("a column is named with its table");
    let values = &target.model.tables[table].columns[column].values;
    let other = match values {
        ColumnValue::Number(_) | ColumnValue::Date(_) | ColumnValue::Text(_) => None,
        ColumnValue::Boolean(_) => Some("true and false"),
        ColumnValue::Blanks(_) => Some("numbers with an empty cell"),
    };
    if let Some(other) = other {
        return Err(format!(
            "it is a column of {other}; edit changes a column of numbers, of dates or of text"
        ));
    }
    let items = items(change.value.as_ref(), values.len())?;
    if matches!(values, ColumnValue::Text(_)) {
        let texts = items
            .iter()
            .map(|item| match item {
                Value::Text(text) => Ok(text.clone()),
                _ => Err(
                    "it is a column of text: its value is a list of texts ([\"Basic\", \"Plus\"])"
                        .to_string(),
                ),
            })
            .collect::<Result<Vec<_>, _>>()?;
        return text::set_text_column(text, names, &texts);
    }
    let texts = items
        .iter()
        .map(|item| value_text(Some(item)))
        .collect::<Result<Vec<_>, _>>()?;
    text::set_column(text, names, &texts)
}

/// The change's `value`: a list, one value for each of the column's `rows`
fn items(value: Option<&Value>, rows: usize) -> Result<&[Value], String> {
    let Some(Value::List(items)) = value else {
        return Err(format!(
            "it is a table's column of {rows} rows: its value is a list, one for each row ([20, 45])"
        ));
    };
    if items.len() != rows {
        return Err(format!(
            "the column has {rows} rows and the list has {}: edit changes a column's values, not its rows",
            items.len()
        ));
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::super::{edit_files, Op};
    use super::*;
    use std::collections::{BTreeMap, HashMap};

    const MODEL: &str = "_forge_version: \"5.1.0\"
products:
  name: [\"Basic\", \"Plus\"]   # two products
  price: [20.00, 40]
  units:
    - 200   # the busy one
    - 120
  launched: [2026-01-31, \"2026-03-01\"]
  region:
    - North   # plain
    - 'South' # single quotes
  online: [true, false]
  revenue: \"=price * units\"
year:
  revenue: {formula: \"=SUM(products.revenue)\"}
";

    fn set_column(path: &str, value: Value) -> Result<String, String> {
        let change = Change {
            op: Op::SetValue,
            path: path.into(),
            value: Some(value),
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: None,
        };
        let files = HashMap::from([("model.yaml".to_string(), MODEL.to_string())]);
        edit_files(&files, "model.yaml", &[change])
            // No file in the answer: none changed
            .map(|changed: BTreeMap<String, String>| {
                changed
                    .get("model.yaml")
                    .map_or_else(|| MODEL.to_string(), Clone::clone)
            })
            .map_err(|e| e.to_string())
    }

    fn list(items: &[Value]) -> Value {
        Value::List(items.to_vec())
    }

    fn text(t: &str) -> Value {
        Value::Text(t.into())
    }

    #[test]
    fn only_the_rows_that_change_are_written_in_the_lists_style() {
        // A flow list: 20.00 is 20, so it stays as it is written
        assert_eq!(
            set_column(
                "products.price",
                list(&[Value::Number(20.0), Value::Number(45.0)])
            ),
            Ok(MODEL.replace("[20.00, 40]", "[20.00, 45]"))
        );
        // A block list, its comment kept; text is written as given
        assert_eq!(
            set_column("products.units", list(&[text("250"), text("1.2e2")])),
            Ok(MODEL.replace("- 200   #", "- 250   #"))
        );
        // A column of dates, in the quotes each row has
        assert_eq!(
            set_column(
                "products.launched",
                list(&[text("2026-02-28"), text("2026-04-01")])
            ),
            Ok(MODEL.replace(
                "[2026-01-31, \"2026-03-01\"]",
                "[2026-02-28, \"2026-04-01\"]"
            ))
        );
        // The same values: nothing to write
        assert_eq!(
            set_column("products.price", list(&[text("20"), text("40")])),
            Ok(MODEL.to_string())
        );
    }

    #[test]
    fn a_column_of_text_keeps_the_quotes_each_row_has() {
        let set = |path: &str, texts: &[&str]| {
            set_column(
                path,
                list(&texts.iter().map(|t| text(t)).collect::<Vec<_>>()),
            )
        };
        // Double quotes, escaped as they need; the comment kept
        assert_eq!(
            set("products.name", &["Basic", "Plus \"Pro\" \\ 2"]),
            Ok(MODEL.replace(
                "[\"Basic\", \"Plus\"]   #",
                "[\"Basic\", \"Plus \\\"Pro\\\" \\\\ 2\"]   #"
            ))
        );
        // Plain stays plain; single quotes double a quote inside
        assert_eq!(
            set("products.region", &["Zürich-Nord", "O'Brien's"]),
            Ok(MODEL
                .replace("- North   #", "- Zürich-Nord   #")
                .replace("- 'South' #", "- 'O''Brien''s' #"))
        );
        assert_eq!(
            set("products.region", &["假设.单价", "South"]),
            Ok(MODEL.replace("- North   #", "- 假设.单价   #"))
        );
        // A plain row whose text YAML would read as something else, or not
        // as written, is double-quoted
        for (new, written) in [
            ("true", "\"true\""),
            ("Yes", "\"Yes\""),
            ("12", "\"12\""),
            ("", "\"\""),
            ("a: b", "\"a: b\""),
            ("a #b", "\"a #b\""),
            ("[x]", "\"[x]\""),
            ("trailing ", "\"trailing \""),
        ] {
            assert_eq!(
                set("products.region", &[new, "South"]),
                Ok(MODEL.replace("- North   #", &format!("- {written}   #"))),
                "{new}"
            );
        }
        // A line break: double quotes, the only ones that escape it
        assert_eq!(
            set("products.region", &["North", "two\nlines"]),
            Ok(MODEL.replace("- 'South' #", "- \"two\\nlines\" #"))
        );
        // The same texts: nothing to write
        assert_eq!(
            set("products.name", &["Basic", "Plus"]),
            Ok(MODEL.to_string())
        );
    }

    #[test]
    fn a_column_edit_cannot_change_is_refused_saying_why() {
        let refused = |path: &str, value: Value, why: &str| {
            let message = set_column(path, value).unwrap_err();
            assert!(
                message.contains(&format!("set_value {path}: {why}")),
                "{message}"
            );
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        let two = list(&[text("1"), text("2")]);
        refused(
            "products.online",
            two.clone(),
            "it is a column of true and false; edit changes a column of numbers, of dates or of text",
        );
        refused(
            "products.name",
            list(&[text("Basic"), Value::Number(2.0)]),
            "it is a column of text: its value is a list of texts ([\"Basic\", \"Plus\"])",
        );
        // Numbers for dates: the model's format refuses it
        refused(
            "products.launched",
            list(&[text("1"), text("2")]),
            "Forge refuses the model with this change",
        );
        // Texts that are all dates: Forge would read a column of dates
        refused(
            "products.name",
            list(&[text("2026-01-31"), text("2026-02-28")]),
            "Forge reads products.name in the new text as a column of dates and not of text",
        );
        refused(
            "products.price",
            Value::Number(1.0),
            "it is a table's column of 2 rows: its value is a list, one for each row ([20, 45])",
        );
        refused(
            "products.price",
            list(&[text("1")]),
            "the column has 2 rows and the list has 1: edit changes a column's values, not its rows",
        );
        refused(
            "products.price",
            list(&[text("1"), list(&[text("2")])]),
            "a list of values is for a table's column; this takes one",
        );
        refused(
            "products.price",
            list(&[text("1"), text("two")]),
            "\"two\" is not written as a number or a date",
        );
        // A date where the other rows are numbers: Forge reads no column
        refused(
            "products.price",
            list(&[text("1"), text("2026-01-31")]),
            "Forge refuses the model with this change",
        );
        // A row Forge reads as another number than the text given
        refused(
            "products.price",
            list(&[text("1"), text("0x10")]),
            "Forge reads products.price in the new text differently from what was asked",
        );
        // A list for a scalar, and a column with a formula
        refused("year.revenue", two.clone(), "it is a formula, not an input");
        refused("products.revenue", two, "it is a formula, not an input");
    }

    #[test]
    fn a_column_with_an_empty_cell_is_refused() {
        let model = "_forge_version: \"5.3.0\"\nt:\n  x: [1, null]\n  y: \"=x * 2\"\n";
        let change = Change {
            op: Op::SetValue,
            path: "t.x".into(),
            value: Some(list(&[text("1"), text("2")])),
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: None,
        };
        let files = HashMap::from([("m.yaml".to_string(), model.to_string())]);
        let message = edit_files(&files, "m.yaml", &[change])
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("it is a column of numbers with an empty cell"),
            "{message}"
        );
    }

    /// Every column of text of every example: each row's text changed, plain
    /// and then in words a plain row cannot hold, and changed back to the
    /// file as it was, byte for byte
    #[test]
    fn every_column_of_text_of_every_example_is_changed_and_back() {
        let mut changed = 0;
        let mut lookups = 0;
        for (dir, files) in super::super::tests::example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = super::super::check::read(&files, main).unwrap_or_default();
                for (t, table) in &model.tables {
                    for (c, column) in &table.columns {
                        let ColumnValue::Text(texts) = &column.values else {
                            continue;
                        };
                        let place = format!("{}/{main} {t}.{c}", dir.display());
                        let set = |files: &HashMap<String, String>, texts: Vec<Value>| {
                            let change = Change {
                                op: Op::SetValue,
                                path: format!("{t}.{c}"),
                                value: Some(Value::List(texts)),
                                formula: None,
                                unit: None,
                                after: None,
                                new_name: None,
                                plays: None,
                            };
                            edit_files(files, main, &[change]).map_err(|e| e.to_string())
                        };
                        let read = |files: &HashMap<String, String>| {
                            super::super::check::read(files, main)
                                .map(|m| super::super::check::snapshot(&m))
                        };
                        for form in ["{} (2)", "'{}': \"#2\""] {
                            let new = texts.iter().map(|t| text(&form.replace("{}", t))).collect();
                            let once = match set(&files, new) {
                                Ok(once) => once,
                                // The fund's ledger finds its deals and fees
                                // by their text: a changed name no longer
                                // matches, and Forge refuses the model
                                Err(e) => {
                                    assert!(
                                        place.contains("fund.yaml")
                                            && e.contains("XLOOKUP: no match found"),
                                        "{place}: {e}"
                                    );
                                    lookups += 1;
                                    continue;
                                },
                            };
                            let mut after = files.clone();
                            after.extend(once);
                            let back = set(&after, texts.iter().map(|t| text(t)).collect());
                            assert!(back.is_ok(), "{place}: {back:?}");
                            after.extend(back.unwrap_or_default());
                            // A plain row that took double quotes keeps them:
                            // the model is the same, its bytes are not
                            let same = if form.contains('#') {
                                read(&after) == read(&files)
                            } else {
                                after == files
                            };
                            assert!(same, "{place}: not back as it was");
                            changed += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(lookups, 8);
        // The starter's products and the vineyards' varieties, both ways
        assert!(changed >= 8, "{changed} changed");
    }
}
