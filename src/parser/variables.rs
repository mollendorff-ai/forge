//! Table and scalar variable parsing for Forge models
//!
//! Handles parsing of tables (with columns and row formulas) and scalar variables.

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, Metadata, Table, Variable};
use yaml_serde::Value;

use super::arrays::parse_array_value;

/// Parse a table from a YAML mapping (v4.0 enhanced with metadata)
///
/// # Errors
///
/// Returns an error if a column name is not a string, or column data has an invalid type or format.
pub fn parse_table(name: &str, map: &yaml_serde::Mapping) -> ForgeResult<Table> {
    let mut table = Table::new(name.to_string());

    for (key, value) in map {
        let col_name = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Column name must be a string".to_string()))?;

        // Skip _metadata table-level metadata (v4.0)
        if col_name == "_metadata" {
            continue;
        }

        // Check if this is a formula (string starting with =)
        if let Value::String(s) = value {
            if s.starts_with('=') {
                // This is a row-wise formula
                table.add_row_formula(col_name.to_string(), s.clone());
                continue;
            }
        }

        // A period column: {start: 2026-12, count: 300, unit: month} (ADR-053)
        if let Value::Mapping(col_map) = value {
            if super::periods::looks_like_period_column(col_map) {
                let dates = super::periods::period_column(name, col_name, col_map)?;
                table.add_column(Column::new(
                    col_name.to_string(),
                    crate::types::ColumnValue::Date(dates),
                ));
                continue;
            }
        }

        // Check for v4.0 rich column format: { value: [...], unit: "...", notes: "..." }
        if let Value::Mapping(col_map) = value {
            // Check if it has a 'value' key with an array (v4.0 rich format)
            if let Some(Value::Sequence(seq)) = col_map.get("value") {
                let column_value = parse_array_value(col_name, seq)?;
                let metadata = parse_metadata(col_map);
                let is_date = matches!(column_value, crate::types::ColumnValue::Date(_));
                check_format(&format!("{name}.{col_name}"), &metadata, Some(is_date))?;
                let column = Column::with_metadata(col_name.to_string(), column_value, metadata);
                table.add_column(column);
                continue;
            }
            // Check if it has a 'formula' key (v4.0 rich formula format)
            if let Some(formula_val) = col_map.get("formula") {
                if let Some(formula_str) = formula_val.as_str() {
                    if formula_str.starts_with('=') {
                        // A row formula with metadata, kept beside it (ADR-061)
                        table.add_row_formula(col_name.to_string(), formula_str.to_string());
                        let metadata = parse_metadata(col_map);
                        check_format(&format!("{name}.{col_name}"), &metadata, None)?;
                        if !metadata.is_empty() {
                            table
                                .formula_metadata
                                .insert(col_name.to_string(), metadata);
                        }
                        continue;
                    }
                }
            }
        }

        // Otherwise, it's a simple data column (array) - v1.0 format
        if let Value::Sequence(seq) = value {
            let column_value = parse_array_value(col_name, seq)?;
            let column = Column::new(col_name.to_string(), column_value);
            table.add_column(column);
        } else {
            return Err(ForgeError::Parse(format!(
                "Column '{col_name}' in table '{name}' must be an array or formula"
            )));
        }
    }

    Ok(table)
}

/// Parse a scalar variable (v4.0 enhanced with metadata)
///
/// # Errors
///
/// Returns an error if the value is not a mapping.
pub fn parse_scalar_variable(value: &Value, path: &str) -> ForgeResult<Variable> {
    if let Value::Mapping(map) = value {
        check_scalar_keys(map, path)?;
        let date = match map.get("value") {
            // `check_scalar_keys` has refused a text that looks like a date and is not one
            Some(Value::String(text)) => {
                scalar_date(text, &format!("{path}.value")).unwrap_or_default()
            },
            _ => None,
        };
        // A result that is not finite can only be a formula's, which an older
        // Forge wrote (`check_finite` refuses any other): it is calculated again
        let val = date
            .or_else(|| map.get("value").and_then(yaml_serde::Value::as_f64))
            .filter(|v| v.is_finite());
        let formula = map
            .get("formula")
            .and_then(|f| f.as_str().map(std::string::ToString::to_string));

        // Extract v4.0 metadata fields
        let metadata = parse_metadata(map);
        let is_date = formula.is_none().then_some(date.is_some());
        check_format(path, &metadata, is_date)?;
        if date.is_some() && formula.is_none() && metadata.unit.is_some() {
            return Err(ForgeError::Parse(format!(
                "'{path}' is a date, and a date has no unit: remove `unit:` (ADR-060)"
            )));
        }

        Ok(Variable {
            path: path.to_string(),
            value: val,
            formula,
            metadata,
            is_date: date.is_some(),
        })
    } else {
        Err(ForgeError::Parse(format!(
            "Expected mapping for scalar variable '{path}'"
        )))
    }
}

/// The keys a scalar's mapping may have.
pub const SCALAR_KEYS: [&str; 9] = [
    "value",
    "formula",
    "unit",
    "notes",
    "source",
    "validation_status",
    "last_updated",
    "better",
    "format",
];

/// Refuse what reading a mapping as one scalar would otherwise drop without a
/// word: a key it does not have (`formla`, or a second scalar beside `value:`),
/// a `value` that is not a number and a `formula` that is not text.
///
/// # Errors
///
/// Returns an error naming the scalar and the key.
pub fn check_scalar_keys(map: &yaml_serde::Mapping, path: &str) -> ForgeResult<()> {
    for (key, value) in map {
        let Some(key) = key.as_str() else {
            return Err(ForgeError::Parse(format!(
                "'{path}': a key of a scalar must be text"
            )));
        };
        if !SCALAR_KEYS.contains(&key) {
            let hint = crate::error::closest(key, &SCALAR_KEYS)
                .map_or_else(String::new, |k| format!(" Did you mean '{k}'?"));
            let nested = if matches!(value, Value::Mapping(_)) {
                format!(
                    " ('{path}' has `value:` or `formula:`, so it is read as one scalar, \
                     and '{key}' cannot be a scalar inside it)"
                )
            } else {
                String::new()
            };
            return Err(ForgeError::Parse(format!(
                "'{path}.{key}': a scalar has no key '{key}'.{hint}{nested} \
                 A scalar's keys are {}",
                SCALAR_KEYS.join(", ")
            )));
        }
        let bad = match key {
            "value" => match value {
                Value::Number(_) | Value::Null => false,
                Value::String(text) => scalar_date(text, &format!("{path}.value"))?.is_none(),
                _ => true,
            },
            "formula" => !matches!(value, Value::String(_) | Value::Null),
            "format" => !matches!(value, Value::String(_)),
            _ => false,
        };
        if bad {
            let expected = match key {
                "value" => "a number or a date (2026-10-31)",
                "format" => "an Excel number format in quotes, such as \"#,##0.00\"",
                _ => "a formula starting with = (in quotes)",
            };
            return Err(ForgeError::Parse(format!(
                "'{path}.{key}' must be {expected}, not {}",
                describe(value)
            )));
        }
    }
    Ok(())
}

/// Whether text is written like a date or a month (`2026-10-31`, `2026-10`,
/// `2026-1-5`): four digits, a dash and one or two digits, once or twice.
#[must_use]
pub fn looks_like_date(text: &str) -> bool {
    let mut parts = text.split('-');
    let year = parts
        .next()
        .is_some_and(|y| y.len() == 4 && y.bytes().all(|b| b.is_ascii_digit()));
    let rest: Vec<&str> = parts.collect();
    year && (1..=2).contains(&rest.len())
        && rest
            .iter()
            .all(|p| (1..=2).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

/// A scalar written as a date (ADR-060): its day number. `Ok(None)` for text
/// not written like a date.
///
/// # Errors
///
/// Returns an error naming `path` for a month, a date that does not exist and
/// a date before 1900-03-01, saying which form is wanted.
pub fn scalar_date(text: &str, path: &str) -> ForgeResult<Option<f64>> {
    date_or_message(text, path).map_err(ForgeError::Parse)
}

/// `scalar_date` with the reason as plain text.
pub(super) fn date_or_message(text: &str, path: &str) -> Result<Option<f64>, String> {
    if !looks_like_date(text) {
        return Ok(None);
    }
    if let Some(serial) = crate::core::array_calculator::evaluator::excel_date_serial(text) {
        return Ok(Some(serial));
    }
    let message = if text.matches('-').count() == 1 {
        format!(
            "'{path}' is the month {text}, and a date in a scalar is a day: write {text}-01 for its \
             first day, or use EOMONTH for its last (ADR-060)"
        )
    } else if chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_err() {
        format!("'{path}' is not a date: {text} is no day of the calendar (a date is written 2026-10-31)")
    } else if text.len() != 10 {
        format!(
            "'{path}': write the date with two digits for the month and the day, as in 2026-01-05"
        )
    } else {
        format!(
            "'{path}' is {text}, and a date in a scalar is from 1900-03-01 on, the first day \
             Excel's day numbers count right"
        )
    };
    Err(message)
}

/// A YAML value in a few words, for an error message.
#[must_use]
pub fn describe(value: &Value) -> String {
    match value {
        Value::Null => "empty".to_string(),
        Value::Bool(b) => format!("{b} (true or false)"),
        Value::Number(n) => format!("the number {n}"),
        Value::String(s) => format!("the text \"{s}\""),
        Value::Sequence(_) => "a list".to_string(),
        Value::Mapping(_) => "a mapping".to_string(),
        Value::Tagged(_) => "a tagged value".to_string(),
    }
}

/// Extract metadata fields from a YAML mapping (v4.0)
#[must_use]
pub fn parse_metadata(map: &yaml_serde::Mapping) -> Metadata {
    Metadata {
        unit: map
            .get("unit")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        notes: map
            .get("notes")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        source: map
            .get("source")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        validation_status: map
            .get("validation_status")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        last_updated: map
            .get("last_updated")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        better: map
            .get("better")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        format: map
            .get("format")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
    }
}

/// A `format:` must be a code Forge reads, a date code on a date and a
/// number code on a number (ADR-061). `is_date` is `None` where the value is
/// a formula's, known only once calculated.
///
/// # Errors
///
/// Returns an error naming `path` and what Forge does not read.
pub fn check_format(path: &str, metadata: &Metadata, is_date: Option<bool>) -> ForgeResult<()> {
    use crate::core::array_calculator::evaluator::text::number_format::{check_format, FormatKind};
    let Some(format) = &metadata.format else {
        return Ok(());
    };
    let kind = check_format(format).map_err(|why| {
        ForgeError::Parse(format!(
            "'{path}.format': {}",
            why.trim_start_matches("TEXT: ")
        ))
    })?;
    match (kind, is_date) {
        (FormatKind::Date, Some(false)) => Err(ForgeError::Parse(format!(
            "'{path}.format': \"{format}\" is a date format, and '{path}' is a number"
        ))),
        (FormatKind::Number, Some(true)) => Err(ForgeError::Parse(format!(
            "'{path}.format': \"{format}\" is a number format, and '{path}' is a date"
        ))),
        _ => Ok(()),
    }
}

/// Check if a mapping contains nested scalar sections (e.g., summary.total)
/// Returns false for v4.0 rich table columns (where value is an array)
#[must_use]
pub fn is_nested_scalar_section(map: &yaml_serde::Mapping) -> bool {
    // A plain number child makes it a group of scalars (5.1.0 short form), and
    // so does a date (5.4.0, ADR-060)
    if map.values().any(|v| match v {
        Value::Number(_) => true,
        Value::String(text) => looks_like_date(text),
        _ => false,
    }) {
        return true;
    }
    // Check if children are mappings with {value, formula} pattern where value is a scalar (not array)
    for (_key, value) in map {
        if let Value::Mapping(child_map) = value {
            // Check if this child has value or formula keys
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                // If value is an array, this is a v4.0 rich table column, not a scalar
                if let Some(val) = child_map.get("value") {
                    if matches!(val, Value::Sequence(_)) {
                        return false; // v4.0 rich table column
                    }
                }
                return true; // Nested scalar section
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    fn table_of(text: &str) -> ForgeResult<Table> {
        parse_table("t", &yaml_serde::from_str(text).unwrap())
    }

    fn scalar_of(text: &str) -> ForgeResult<Variable> {
        parse_scalar_variable(&yaml_serde::from_str(text).unwrap(), "x")
    }

    fn table_error(text: &str) -> String {
        table_of(text).err().unwrap().to_string()
    }

    fn scalar_error(text: &str) -> String {
        scalar_of(text).err().unwrap().to_string()
    }

    #[test]
    fn table_level_metadata_is_not_a_column() {
        let table = table_of("_metadata: {x: 1}\na: [1, 2]\n").unwrap();
        assert_eq!(table.columns.len(), 1);
    }

    #[test]
    fn a_period_column_that_is_not_one_is_refused_with_its_reason() {
        assert_eq!(
            table_error("d: {start: \"2026-13\", count: 3, unit: month}\n"),
            "Parse error: 't.d': start must be a month (2026-12), a day (2026-12-15) or a year (2026)"
        );
        assert_eq!(
            table_error("d: {start: \"2026-01\", count: 0, unit: month}\n"),
            "Parse error: 't.d': count must be a whole number from 1 to 1048575"
        );
    }

    #[test]
    fn a_rich_column_is_read_and_its_format_is_checked_against_its_values() {
        assert_eq!(
            table_error("a: {value: [1, \"x\"]}\n"),
            "Parse error: Column 'a' row 1: Expected Number, found String"
        );
        assert_eq!(
            table_error("a: {value: [1, 2], format: \"yyyy\"}\n"),
            "Parse error: 't.a.format': \"yyyy\" is a date format, and 't.a' is a number"
        );
        assert_eq!(
            table_error("a: {value: [\"2026-01-31\"], format: \"#,##0\"}\n"),
            "Parse error: 't.a.format': \"#,##0\" is a number format, and 't.a' is a date"
        );
        assert_eq!(
            table_error("a: {value: [1, 2], format: \"bogus\"}\n"),
            "Parse error: 't.a.format': the format \"bogus\" is not one Forge reads (write the text b in quotes)"
        );
    }

    #[test]
    fn a_row_formula_keeps_its_metadata_only_when_it_has_some() {
        let plain = table_of("a: {formula: \"=b + 1\"}\n").unwrap();
        assert_eq!(plain.row_formulas["a"], "=b + 1");
        assert!(plain.formula_metadata.is_empty());
        let with_unit = table_of("a: {formula: \"=b + 1\", unit: CAD}\n").unwrap();
        assert_eq!(with_unit.formula_metadata["a"].unit.as_deref(), Some("CAD"));
        assert_eq!(
            table_error("a: {formula: \"=b + 1\", format: \"nope\"}\n"),
            "Parse error: 't.a.format': the format \"nope\" is not one Forge reads (write the text n in quotes)"
        );
    }

    #[test]
    fn a_column_that_is_neither_a_list_nor_a_formula_is_refused() {
        let reason = "Parse error: Column 'a' in table 't' must be an array or formula";
        assert_eq!(table_error("a: {formula: x}\n"), reason);
        assert_eq!(table_error("a: 5\n"), reason);
        assert_eq!(table_error("a: text\n"), reason);
        assert_eq!(table_error("a: {formula: 5}\n"), reason);
        assert_eq!(
            table_error("a: [1, x]\n"),
            "Parse error: Column 'a' row 1: Expected Number, found String"
        );
    }

    #[test]
    fn what_makes_a_scalar_is_checked_key_by_key() {
        let key = "Parse error: 'x.value' must be a number or a date (2026-10-31), not ";
        for (text, found) in [
            ("{value: [1]}", "a list"),
            ("{value: true}", "true (true or false)"),
            ("{value: text}", "the text \"text\""),
            ("{value: {a: 1}}", "a mapping"),
        ] {
            assert_eq!(scalar_error(text), format!("{key}{found}"));
        }
        assert_eq!(
            scalar_error("{formula: 5}"),
            "Parse error: 'x.formula' must be a formula starting with = (in quotes), not the number 5"
        );
        assert_eq!(
            scalar_error("{formula: [1]}"),
            "Parse error: 'x.formula' must be a formula starting with = (in quotes), not a list"
        );
        assert_eq!(
            scalar_error("{format: 5}"),
            "Parse error: 'x.format' must be an Excel number format in quotes, such as \"#,##0.00\", not the number 5"
        );
        assert_eq!(
            scalar_error("{1: 2}"),
            "Parse error: 'x': a key of a scalar must be text"
        );
        assert_eq!(
            scalar_error("{value: \"2026-02-30\"}"),
            "Parse error: 'x.value' is not a date: 2026-02-30 is no day of the calendar (a date is written 2026-10-31)"
        );
        assert_eq!(
            scalar_of("{value: null, formula: null}").unwrap().value,
            None
        );
    }

    #[test]
    fn a_key_a_scalar_does_not_have_is_named_with_its_close_match() {
        let keys = "A scalar's keys are value, formula, unit, notes, source, validation_status, last_updated, better, format";
        assert_eq!(
            scalar_error("{value: 1, nested: {a: 1}}"),
            format!("Parse error: 'x.nested': a scalar has no key 'nested'. ('x' has `value:` or `formula:`, so it is read as one scalar, and 'nested' cannot be a scalar inside it) {keys}")
        );
    }

    #[test]
    fn a_date_scalar_has_no_unit_and_no_number_format() {
        assert_eq!(
            scalar_error("{value: \"2026-10-31\", unit: CAD}"),
            "Parse error: 'x' is a date, and a date has no unit: remove `unit:` (ADR-060)"
        );
        assert_eq!(
            scalar_error("{value: \"2026-10-31\", format: \"#,##0\"}"),
            "Parse error: 'x.format': \"#,##0\" is a number format, and 'x' is a date"
        );
        assert_eq!(
            scalar_error("{value: 1, unit: CAD, format: \"yyyy\"}"),
            "Parse error: 'x.format': \"yyyy\" is a date format, and 'x' is a number"
        );
    }

    #[test]
    fn a_date_that_cannot_be_a_scalar_says_which_rule_it_breaks() {
        let reason = |text: &str| scalar_date(text, "p").err().unwrap().to_string();
        assert_eq!(
            reason("2026-10"),
            "Parse error: 'p' is the month 2026-10, and a date in a scalar is a day: write 2026-10-01 for its first day, or use EOMONTH for its last (ADR-060)"
        );
        assert_eq!(
            reason("2026-1-5"),
            "Parse error: 'p': write the date with two digits for the month and the day, as in 2026-01-05"
        );
        assert_eq!(
            reason("1899-01-01"),
            "Parse error: 'p' is 1899-01-01, and a date in a scalar is from 1900-03-01 on, the first day Excel's day numbers count right"
        );
    }

    #[test]
    fn a_value_is_described_in_a_few_words() {
        let tagged: Value = yaml_serde::from_str("!t 1").unwrap();
        for (value, words) in [
            (Value::Null, "empty"),
            (Value::Bool(false), "false (true or false)"),
            (Value::from(2.5), "the number 2.5"),
            (Value::from("a"), "the text \"a\""),
            (Value::Sequence(vec![]), "a list"),
            (Value::Mapping(yaml_serde::Mapping::new()), "a mapping"),
            (tagged, "a tagged value"),
        ] {
            assert_eq!(describe(&value), words);
        }
    }

    /// Reading a mapping as one scalar refuses what it would otherwise drop.
    #[test]
    fn a_scalar_refuses_keys_and_values_it_would_drop() {
        let read = |text: &str| {
            let value: Value = yaml_serde::from_str(text).unwrap();
            parse_scalar_variable(&value, "loan.payment").map_err(|e| e.to_string())
        };
        assert!(read("value: 3\nformula: \"=a\"\nunit: CAD\nbetter: lower\n").is_ok());
        assert!(read("value: null\nformula: null\n").is_ok());
        let err = read("formla: \"=a\"\n").unwrap_err();
        assert!(
            err.contains("'loan.payment.formla'") && err.contains("Did you mean 'formula'?"),
            "{err}"
        );
        let err = read("value: 3\ndoubled:\n  formula: \"=value * 2\"\n").unwrap_err();
        assert!(err.contains("read as one scalar"), "{err}");
        let err = read("value: high\n").unwrap_err();
        assert!(
            err.contains(
                "'loan.payment.value' must be a number or a date (2026-10-31), not the text \"high\""
            ),
            "{err}"
        );
        let err = read("formula: 3\n").unwrap_err();
        assert!(
            err.contains("'loan.payment.formula' must be a formula"),
            "{err}"
        );
    }

    use super::*;
    use crate::types::ColumnValue;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_table_with_arrays() {
        let yaml = r"
    month: ['Jan', 'Feb', 'Mar']
    revenue: [100, 200, 300]
    ";
        let parsed: Value = yaml_serde::from_str(yaml).unwrap();

        let map = parsed.as_mapping().unwrap();
        let table = parse_table("test_table", map).unwrap();

        assert_eq!(table.name, "test_table");
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns.contains_key("month"));
        assert!(table.columns.contains_key("revenue"));
        assert_eq!(table.row_count(), 3);
    }

    #[test]
    fn test_parse_table_with_formula() {
        let yaml = r"
    revenue: [100, 200, 300]
    expenses: [50, 100, 150]
    profit: '=revenue - expenses'
    ";
        let parsed: Value = yaml_serde::from_str(yaml).unwrap();

        let map = parsed.as_mapping().unwrap();
        let table = parse_table("test_table", map).unwrap();

        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.row_formulas.len(), 1);
        assert!(table.row_formulas.contains_key("profit"));
        assert_eq!(
            table.row_formulas.get("profit").unwrap(),
            "=revenue - expenses"
        );
    }

    #[test]
    fn test_table_validate_lengths_ok() {
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "col1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "col2".to_string(),
            ColumnValue::Number(vec![4.0, 5.0, 6.0]),
        ));

        assert!(table.validate_lengths().is_ok());
    }

    #[test]
    fn test_table_validate_lengths_error() {
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "col1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "col2".to_string(),
            ColumnValue::Number(vec![4.0, 5.0]),
        ));

        let result = table.validate_lengths();
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("col1") || err_msg.contains("col2"));
        assert!(err_msg.contains("2 rows"));
        assert!(err_msg.contains("3 rows"));
    }

    #[test]
    fn test_column_value_type_name() {
        let num_col = ColumnValue::Number(vec![1.0]);
        let text_col = ColumnValue::Text(vec!["A".to_string()]);
        let date_col = ColumnValue::Date(vec!["2025-01".to_string()]);
        let bool_col = ColumnValue::Boolean(vec![true]);

        assert_eq!(num_col.type_name(), "Number");
        assert_eq!(text_col.type_name(), "Text");
        assert_eq!(date_col.type_name(), "Date");
        assert_eq!(bool_col.type_name(), "Boolean");
    }

    #[test]
    fn test_column_value_len() {
        let col = ColumnValue::Number(vec![1.0, 2.0, 3.0]);
        assert_eq!(col.len(), 3);
        assert!(!col.is_empty());

        let empty_col = ColumnValue::Number(vec![]);
        assert_eq!(empty_col.len(), 0);
        assert!(empty_col.is_empty());
    }

    #[test]
    fn test_parse_v4_scalar_with_metadata() {
        let yaml_content = r#"
_forge_version: "5.0.0"

price:
  value: 100
  formula: null
  unit: "CAD"
  notes: "Base price per unit"
  source: "market_research.yaml"
  validation_status: "VALIDATED"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let yaml: Value = yaml_serde::from_str(&content).unwrap();

        let price_val = &yaml["price"];
        let price = parse_scalar_variable(price_val, "price").unwrap();
        assert_eq!(price.value, Some(100.0));
        assert!(price.formula.is_none());
        assert_eq!(price.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            price.metadata.notes,
            Some("Base price per unit".to_string())
        );
        assert_eq!(
            price.metadata.source,
            Some("market_research.yaml".to_string())
        );
        assert_eq!(
            price.metadata.validation_status,
            Some("VALIDATED".to_string())
        );
    }

    #[test]
    fn test_parse_scalar_variable_not_mapping() {
        let val = Value::String("not a mapping".to_string());
        let result = parse_scalar_variable(&val, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected mapping"));
    }

    #[test]
    fn test_metadata_last_updated() {
        let mut map = yaml_serde::Mapping::new();
        map.insert(
            Value::String("last_updated".to_string()),
            Value::String("2025-01-01".to_string()),
        );
        let metadata = parse_metadata(&map);
        assert_eq!(metadata.last_updated, Some("2025-01-01".to_string()));
    }

    #[test]
    fn test_is_nested_scalar_section_false_for_v4_rich_column() {
        let mut map = yaml_serde::Mapping::new();
        let mut child = yaml_serde::Mapping::new();
        child.insert(
            Value::String("value".to_string()),
            Value::Sequence(vec![Value::Number(1.into())]),
        );
        map.insert(Value::String("col".to_string()), Value::Mapping(child));
        assert!(!is_nested_scalar_section(&map));
    }

    #[test]
    fn test_is_nested_scalar_section_true_for_scalar() {
        let mut map = yaml_serde::Mapping::new();
        let mut child = yaml_serde::Mapping::new();
        child.insert(Value::String("value".to_string()), Value::Number(42.into()));
        map.insert(Value::String("total".to_string()), Value::Mapping(child));
        assert!(is_nested_scalar_section(&map));
    }

    #[test]
    fn test_is_nested_scalar_section_empty_child() {
        let mut map = yaml_serde::Mapping::new();
        map.insert(
            Value::String("empty".to_string()),
            Value::Mapping(yaml_serde::Mapping::new()),
        );
        assert!(!is_nested_scalar_section(&map));
    }

    #[test]
    fn test_parse_table_column_scalar_not_array() {
        let mut map = yaml_serde::Mapping::new();
        map.insert(Value::String("col".to_string()), Value::Number(42.into()));
        let result = parse_table("test", &map);
        assert!(result.is_err());
    }

    mod scalar_dates {
        //! A date as a scalar (ADR-060): written `2026-10-31` in model format 5.4.0,
        //! read as a cell of a date column is, printed and written back as a date,
        //! text in JSON, a date cell in the exported workbook and a date again after
        //! `forge import`. `DATE` gives a date. Formats before 5.4.0 refuse a date as a
        //! scalar, and the commands that vary an input refuse a date input.

        use crate::cli::{calculate_core, export_core, goal_seek_core, import_core};
        use crate::cli::{compare_core, sensitivity_core};
        use std::path::{Path, PathBuf};

        const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  last_actual: 2026-10-31
  start:
    value: 2026-04-01
    notes: "the first day of the fiscal year"
  rate: 0.1
months:
  month: {start: 2026-04, count: 19, unit: month}
  after: "=IF(month > inputs.last_actual, 1, 0)"
  year_end: "=DATE(YEAR(month), 12, 31)"
out:
  year:
    formula: "=YEAR(inputs.last_actual)"
  plus_one:
    formula: "=inputs.last_actual + 1"
  days:
    formula: "=inputs.last_actual - inputs.start"
  open_months:
    formula: "=COUNTIF(months.month, \">\" & inputs.last_actual)"
  same_day:
    formula: "=IF(inputs.last_actual = DATE(2026, 10, 31), 1, 0)"
  left:
    formula: "=VALUE(LEFT(inputs.last_actual, 4))"
  is_number:
    formula: "=IF(ISNUMBER(inputs.last_actual), 1, 0)"
  next_year:
    formula: "=EOMONTH(inputs.last_actual, 12)"
  close:
    formula: "=inputs.last_actual"
  made:
    formula: "=DATE(2026, 10, 31)"
  grown:
    formula: "=100 * (1 + rate)"
scenarios:
  later:
    last_actual: 2026-11-30
"#;

        fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }

        #[test]
        fn a_date_scalar_is_read_as_a_cell_of_a_date_column() {
            let dir = tempfile::tempdir().unwrap();
            let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
            let number = |name: &str| result.scalars[name].unwrap();
            // Where a number is read, the day number (2026-10-31 is 46326)
            assert_exact!(number("out.year"), 2026.0);
            assert_exact!(number("out.plus_one"), 46327.0);
            assert_exact!(number("out.days"), 213.0);
            // November 2026 to October 2027
            assert_exact!(number("out.open_months"), 12.0);
            assert_exact!(number("out.same_day"), 1.0);
            // Where text is read, the date's text, as for a column (ADR-054)
            assert_exact!(number("out.left"), 2026.0);
            assert_exact!(number("out.is_number"), 0.0);
            // A row formula reads it too
            let after = &result.tables["months"].values["after"];
            assert_eq!(after.iter().filter(|v| v.as_f64() == Some(1.0)).count(), 12);

            // Dates, and formulas whose value is a date, are shown as dates
            for (name, date) in [
                ("inputs.last_actual", "2026-10-31"),
                ("inputs.start", "2026-04-01"),
                ("out.next_year", "2027-10-31"),
                ("out.close", "2026-10-31"),
                ("out.made", "2026-10-31"),
            ] {
                assert_eq!(result.shown(name).as_deref(), Some(date), "{name}");
            }
            // A date plus days is a number, and so is a number
            assert_eq!(result.shown("out.plus_one").as_deref(), Some("46327"));
            assert_eq!(result.shown("out.grown").as_deref(), Some("110"));
            // DATE gives a date in a column too
            assert_eq!(result.tables["months"].values["year_end"][0], "2026-12-31");
        }

        #[test]
        fn json_gives_a_date_as_its_text() {
            let dir = tempfile::tempdir().unwrap();
            let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
            let json = serde_json::to_value(&result).unwrap();
            assert_eq!(json["scalars"]["inputs.last_actual"], "2026-10-31");
            assert_eq!(json["scalars"]["out.next_year"], "2027-10-31");
            assert_eq!(json["scalars"]["out.year"], 2026.0);
            assert!(json.get("dates").is_none(), "{json}");
            // And reads back to the same result
            let back: crate::cli::commands::results::CalculationResult =
                serde_json::from_value(json).unwrap();
            assert_eq!(back.scalars["inputs.last_actual"], Some(46326.0));
            assert!(back.dates.contains("out.next_year"));
            assert!(!back.dates.contains("out.year"));
        }

        #[test]
        fn calculate_writes_a_date_back_as_a_date_in_5_4_0_and_as_its_day_number_before() {
            let dir = tempfile::tempdir().unwrap();
            let new = write(
                dir.path(),
                "new.yaml",
                "_forge_version: \"5.4.0\"\nend:\n  formula: \"=DATE(2026, 10, 31)\"\n",
            );
            calculate_core(&new, false, None).unwrap();
            let text = std::fs::read_to_string(&new).unwrap();
            assert!(text.contains("  value: 2026-10-31\n"), "{text}");
            // It reads back as the date, and a second run changes nothing
            let again = calculate_core(&new, false, None).unwrap();
            assert!(!again.file_updated);

            let old = write(
                dir.path(),
                "old.yaml",
                "_forge_version: \"5.3.0\"\nend:\n  formula: \"=DATE(2026, 10, 31)\"\n",
            );
            let result = calculate_core(&old, false, None).unwrap();
            let text = std::fs::read_to_string(&old).unwrap();
            assert!(text.contains("  value: 46326\n"), "{text}");
            // A date all the same where it is shown
            assert_eq!(result.shown("end").as_deref(), Some("2026-10-31"));
        }

        #[test]
        fn a_date_is_refused_where_it_cannot_be() {
            let dir = tempfile::tempdir().unwrap();
            let error = |text: &str| {
                calculate_core(&write(dir.path(), "bad.yaml", text), true, None)
                    .unwrap_err()
                    .to_string()
            };
            let older = error("_forge_version: \"5.3.0\"\ninputs:\n  last_actual: 2026-10-31\n");
            assert!(
                older.contains("model format 5.4.0") && older.contains("5.3.0"),
                "{older}"
            );
            let month = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 2026-10\n");
            assert!(
                month.contains("the month 2026-10") && month.contains("2026-10-01"),
                "{month}"
            );
            let early = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 1900-01-15\n");
            assert!(early.contains("1900-03-01"), "{early}");
            let impossible = error("_forge_version: \"5.4.0\"\ninputs:\n  last: 2026-02-30\n");
            assert!(
                impossible.contains("no day of the calendar"),
                "{impossible}"
            );
            let unit = error(
                "_forge_version: \"5.4.0\"\ninputs:\n  last:\n    value: 2026-10-31\n    unit: CAD\n",
            );
            assert!(unit.contains("a date has no unit"), "{unit}");
        }

        #[test]
        fn a_scenario_gives_a_date_input_another_date() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let later = calculate_core(&path, true, Some("later")).unwrap();
            assert_eq!(
                later.shown("inputs.last_actual").as_deref(),
                Some("2026-11-30")
            );
            assert_eq!(later.scalars["out.open_months"], Some(11.0));
            let compared = compare_core(&path, &["later".to_string()], false).unwrap();
            assert!(compared.dates.contains("inputs.last_actual"));
            assert!(!compared.dates.contains("out.year"));

            // A number for a date input is refused
            let wrong = MODEL.replace("last_actual: 2026-11-30", "last_actual: 46356");
            let error = calculate_core(&write(dir.path(), "w.yaml", &wrong), true, Some("later"))
                .unwrap_err()
                .to_string();
            assert!(error.contains("is a date"), "{error}");
        }

        #[test]
        fn the_commands_that_vary_an_input_refuse_a_date() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let sensitivity = sensitivity_core(
                &path,
                "inputs.last_actual",
                "46300,46400,50",
                None,
                None,
                "out.days",
                false,
            )
            .unwrap_err()
            .to_string();
            assert!(
                sensitivity.contains("is a date") && sensitivity.contains("sensitivity"),
                "{sensitivity}"
            );
            let seek = goal_seek_core(
                &path,
                "out.days",
                100.0,
                "inputs.last_actual",
                (None, None),
                0.001,
                false,
            )
            .unwrap_err()
            .to_string();
            assert!(seek.contains("is a date"), "{seek}");
            // A number input beside it is varied as before
            assert!(sensitivity_core(
                &path,
                "inputs.rate",
                "0.1,0.2,0.1",
                None,
                None,
                "out.grown",
                false
            )
            .is_ok());
        }

        #[test]
        fn export_writes_a_date_cell_and_import_reads_it_back() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let xlsx = dir.path().join("m.xlsx");
            let exported = export_core(&path, &xlsx).unwrap();
            // Every formula is written live (scenarios are not exported, and say so)
            assert!(
                !exported
                    .warnings
                    .iter()
                    .any(|w| w.contains("written as a value")),
                "{:?}",
                exported.warnings
            );
            let back = dir.path().join("back.yaml");
            import_core(&xlsx, &back, false, false).unwrap();
            let text = std::fs::read_to_string(&back).unwrap();
            assert!(text.starts_with("_forge_version: \"5.4.0\""), "{text}");
            assert!(text.contains("last_actual: 2026-10-31"), "{text}");
            assert!(text.contains("start: 2026-04-01"), "{text}");
            // The formulas come back as written, and give the same values
            assert!(
                text.contains("=VALUE(LEFT(inputs.last_actual, 4))"),
                "{text}"
            );
            assert!(text.contains("\">\" & inputs.last_actual"), "{text}");
            let original = calculate_core(&path, true, None).unwrap();
            let again = calculate_core(&back, true, None).unwrap();
            for (name, value) in &original.scalars {
                assert_eq!(again.scalars.get(name), Some(value), "{name}");
                assert_eq!(again.shown(name), original.shown(name), "{name}");
            }
        }
    }

    mod number_formats {
        //! `format:` beside `unit:` (ADR-061, model format 5.4.0): an Excel number
        //! format that changes how a number is shown in the workbook, `forge report`
        //! and JSON's `formatted`, and never its value. Forge accepts only codes it
        //! writes as Excel does; an earlier format refuses `format:`.

        use crate::cli::{calculate_core, export_core, import_core};
        use std::path::{Path, PathBuf};

        const MODEL: &str = r##"_forge_version: "5.4.0"
year:
  price: {value: 16, format: "$#,##0.00"}
  close: {value: 2026-10-31, format: "mmm yyyy"}
  margin:
    formula: "=-0.1234"
    format: "0.0%;(0.0%)"
months:
  units: {value: [1200, 1500, 0], format: "#,##0"}
  revenue:
    formula: "=units * year.price * -1"
    format: "#,##0;(#,##0);\"-\""
    unit: CAD
    notes: "a formula column keeps its metadata"
"##;

        fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }

        #[test]
        fn a_format_changes_how_a_number_is_shown_and_never_its_value() {
            let dir = tempfile::tempdir().unwrap();
            let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
            // The values are what they were
            assert_eq!(result.scalars["year.price"], Some(16.0));
            assert_eq!(result.scalars["year.margin"], Some(-0.1234));
            assert_eq!(
                result.tables["months"].values["revenue"],
                [-19200.0, -24000.0, 0.0]
            );
            // JSON shows them as their formats do, beside the values
            assert_eq!(result.formatted["year.price"], "$16.00");
            assert_eq!(result.formatted["year.close"], "Oct 2026");
            assert_eq!(result.formatted["year.margin"], "(12.3%)");
            assert_eq!(
                result.tables["months"].formatted["units"],
                ["1,200", "1,500", "0"]
            );
            assert_eq!(
                result.tables["months"].formatted["revenue"],
                ["(19,200)", "(24,000)", "-"]
            );
            let json = serde_json::to_value(&result).unwrap();
            assert_eq!(json["scalars"]["year.price"], 16.0);
            assert_eq!(json["formatted"]["year.price"], "$16.00");
        }

        #[test]
        fn a_model_without_format_has_no_formatted_in_json() {
            let dir = tempfile::tempdir().unwrap();
            let plain = "_forge_version: \"5.1.0\"\nyear:\n  price: 16\nt:\n  x: [1, 2]\n";
            let result = calculate_core(&write(dir.path(), "p.yaml", plain), true, None).unwrap();
            let json = serde_json::to_value(&result).unwrap();
            assert!(json.get("formatted").is_none(), "{json}");
            assert!(json["tables"]["t"].get("formatted").is_none(), "{json}");
        }

        #[test]
        fn a_format_forge_does_not_read_or_on_the_wrong_value_is_refused() {
            let dir = tempfile::tempdir().unwrap();
            let error = |text: &str| {
                calculate_core(&write(dir.path(), "bad.yaml", text), true, None)
                    .unwrap_err()
                    .to_string()
            };
            let older =
                error("_forge_version: \"5.3.0\"\ny:\n  price: {value: 16, format: \"0.00\"}\n");
            assert!(older.contains("model format 5.4.0"), "{older}");
            let condition =
                error("_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"[>10]0\"}\n");
            assert!(
                condition.contains("'y.price.format'") && condition.contains("only a colour"),
                "{condition}"
            );
            let fourth =
                error("_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"0;0;0;@\"}\n");
            assert!(fourth.contains("fourth section"), "{fourth}");
            let date_on_number = error(
                "_forge_version: \"5.4.0\"\ny:\n  price: {value: 16, format: \"mmm yyyy\"}\n",
            );
            assert!(
                date_on_number.contains("is a date format"),
                "{date_on_number}"
            );
            let number_on_date = error(
                "_forge_version: \"5.4.0\"\ny:\n  close: {value: 2026-10-31, format: \"0.00\"}\n",
            );
            assert!(
                number_on_date.contains("is a number format"),
                "{number_on_date}"
            );
            // A formula's value is known once calculated
            let formula =
                error("_forge_version: \"5.4.0\"\ny:\n  end:\n    formula: \"=DATE(2026, 1, 31)\"\n    format: \"0.00\"\n");
            assert!(
                formula.contains("the value of 'y.end' is a date"),
                "{formula}"
            );
        }

        #[test]
        fn export_writes_the_format_and_import_reads_it_back() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let xlsx = dir.path().join("m.xlsx");
            export_core(&path, &xlsx).unwrap();
            let back = dir.path().join("back.yaml");
            let imported = import_core(&xlsx, &back, false, false).unwrap();
            assert_eq!(imported.warnings, Vec::<String>::new());
            let text = std::fs::read_to_string(&back).unwrap();
            assert!(text.starts_with("_forge_version: \"5.4.0\""), "{text}");
            for format in [
                "$#,##0.00",
                "mmm yyyy",
                "0.0%;(0.0%)",
                "#,##0",
                "#,##0;(#,##0);\"-\"",
            ] {
                assert!(text.contains(format), "{format} in {text}");
            }
            // The same values and the same text
            let original = calculate_core(&path, true, None).unwrap();
            let again = calculate_core(&back, true, None).unwrap();
            assert_eq!(again.scalars, original.scalars);
            assert_eq!(again.formatted, original.formatted);
            assert_eq!(
                again.tables["months"].formatted,
                original.tables["months"].formatted
            );
        }

        #[test]
        fn a_file_without_format_exports_as_before() {
            let dir = tempfile::tempdir().unwrap();
            let plain = "_forge_version: \"5.1.0\"\ny:\n  price: 16\n  total:\n    formula: \"=price * 2\"\nt:\n  x: [1, 2]\n  y: \"=x * 2\"\n";
            let path = write(dir.path(), "p.yaml", plain);
            let xlsx = dir.path().join("p.xlsx");
            export_core(&path, &xlsx).unwrap();
            let back = dir.path().join("back.yaml");
            import_core(&xlsx, &back, false, false).unwrap();
            let text = std::fs::read_to_string(&back).unwrap();
            assert!(!text.contains("format"), "{text}");
            assert!(text.starts_with("_forge_version: \"5.1.0\""), "{text}");
        }

        #[test]
        fn text_reads_the_formats_format_added() {
            let dir = tempfile::tempdir().unwrap();
            let model = r##"_forge_version: "5.1.0"
t:
  v: [1234.567, -1234.567, 0]
  negatives: "=TEXT(v, \"#,##0;(#,##0)\")"
  millions: "=TEXT(v * 1000, \"0.0,,\"\"M\"\"\")"
  aligned: "=TEXT(v, \"#,##0_);(#,##0)\")"
"##;
            let result = calculate_core(&write(dir.path(), "t.yaml", model), true, None).unwrap();
            let column = |name: &str| result.tables["t"].values[name].clone();
            assert_eq!(column("negatives"), ["1,235", "(1,235)", "0"]);
            assert_eq!(column("millions"), ["1.2M", "-1.2M", "0.0M"]);
            assert_eq!(column("aligned"), ["1,235 ", "(1,235)", "0 "]);
        }
    }
}
