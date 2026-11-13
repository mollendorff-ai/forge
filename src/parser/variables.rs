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
                table.add_column(Column::new(col_name.to_string(), dates));
                continue;
            }
        }

        // Check for v4.0 rich column format: { value: [...], unit: "...", notes: "..." }
        if let Value::Mapping(col_map) = value {
            // Check if it has a 'value' key with an array (v4.0 rich format)
            if let Some(Value::Sequence(seq)) = col_map.get("value") {
                let column_value = parse_array_value(col_name, seq)?;
                let metadata = parse_metadata(col_map);
                let column = Column::with_metadata(col_name.to_string(), column_value, metadata);
                table.add_column(column);
                continue;
            }
            // Check if it has a 'formula' key (v4.0 rich formula format)
            if let Some(formula_val) = col_map.get("formula") {
                if let Some(formula_str) = formula_val.as_str() {
                    if formula_str.starts_with('=') {
                        // This is a row-wise formula with metadata
                        // TODO: Store formula metadata when we add formula metadata support
                        table.add_row_formula(col_name.to_string(), formula_str.to_string());
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
            Some(Value::String(text)) => scalar_date(text, &format!("{path}.value"))?,
            _ => None,
        };
        let val = date.or_else(|| map.get("value").and_then(yaml_serde::Value::as_f64));
        let formula = map
            .get("formula")
            .and_then(|f| f.as_str().map(std::string::ToString::to_string));

        // Extract v4.0 metadata fields
        let metadata = parse_metadata(map);
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
pub const SCALAR_KEYS: [&str; 8] = [
    "value",
    "formula",
    "unit",
    "notes",
    "source",
    "validation_status",
    "last_updated",
    "better",
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
            _ => false,
        };
        if bad {
            let expected = if key == "value" {
                "a number or a date (2026-10-31)"
            } else {
                "a formula starting with = (in quotes)"
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
    Err(ForgeError::Parse(message))
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

        if let Value::Mapping(map) = parsed {
            let table = parse_table("test_table", &map).unwrap();

            assert_eq!(table.name, "test_table");
            assert_eq!(table.columns.len(), 2);
            assert!(table.columns.contains_key("month"));
            assert!(table.columns.contains_key("revenue"));
            assert_eq!(table.row_count(), 3);
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_parse_table_with_formula() {
        let yaml = r"
    revenue: [100, 200, 300]
    expenses: [50, 100, 150]
    profit: '=revenue - expenses'
    ";
        let parsed: Value = yaml_serde::from_str(yaml).unwrap();

        if let Value::Mapping(map) = parsed {
            let table = parse_table("test_table", &map).unwrap();

            assert_eq!(table.columns.len(), 2);
            assert_eq!(table.row_formulas.len(), 1);
            assert!(table.row_formulas.contains_key("profit"));
            assert_eq!(
                table.row_formulas.get("profit").unwrap(),
                "=revenue - expenses"
            );
        } else {
            panic!("Expected mapping");
        }
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

        if let Some(price_val) = yaml.get("price") {
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
}
