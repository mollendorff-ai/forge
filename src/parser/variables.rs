//! Table and scalar variable parsing for Forge models
//!
//! Handles parsing of tables (with columns and row formulas) and scalar variables.

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, Metadata, Table, Variable};
use serde_yaml_ng::Value;

use super::arrays::parse_array_value;

/// Parse a table from a YAML mapping (v4.0 enhanced with metadata)
pub fn parse_table(name: &str, map: &serde_yaml_ng::Mapping) -> ForgeResult<Table> {
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
pub fn parse_scalar_variable(value: &Value, path: &str) -> ForgeResult<Variable> {
    if let Value::Mapping(map) = value {
        let val = map.get("value").and_then(serde_yaml_ng::Value::as_f64);
        let formula = map
            .get("formula")
            .and_then(|f| f.as_str().map(std::string::ToString::to_string));

        // Extract v4.0 metadata fields
        let metadata = parse_metadata(map);

        Ok(Variable {
            path: path.to_string(),
            value: val,
            formula,
            metadata,
        })
    } else {
        Err(ForgeError::Parse(format!(
            "Expected mapping for scalar variable '{path}'"
        )))
    }
}

/// Extract metadata fields from a YAML mapping (v4.0)
pub fn parse_metadata(map: &serde_yaml_ng::Mapping) -> Metadata {
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
    }
}

/// Check if a mapping contains nested scalar sections (e.g., summary.total)
/// Returns false for v4.0 rich table columns (where value is an array)
pub fn is_nested_scalar_section(map: &serde_yaml_ng::Mapping) -> bool {
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
        let parsed: Value = serde_yaml_ng::from_str(yaml).unwrap();

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
        let parsed: Value = serde_yaml_ng::from_str(yaml).unwrap();

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
        let yaml: Value = serde_yaml_ng::from_str(&content).unwrap();

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
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(
            Value::String("last_updated".to_string()),
            Value::String("2025-01-01".to_string()),
        );
        let metadata = parse_metadata(&map);
        assert_eq!(metadata.last_updated, Some("2025-01-01".to_string()));
    }

    #[test]
    fn test_is_nested_scalar_section_false_for_v4_rich_column() {
        let mut map = serde_yaml_ng::Mapping::new();
        let mut child = serde_yaml_ng::Mapping::new();
        child.insert(
            Value::String("value".to_string()),
            Value::Sequence(vec![Value::Number(1.into())]),
        );
        map.insert(Value::String("col".to_string()), Value::Mapping(child));
        assert!(!is_nested_scalar_section(&map));
    }

    #[test]
    fn test_is_nested_scalar_section_true_for_scalar() {
        let mut map = serde_yaml_ng::Mapping::new();
        let mut child = serde_yaml_ng::Mapping::new();
        child.insert(Value::String("value".to_string()), Value::Number(42.into()));
        map.insert(Value::String("total".to_string()), Value::Mapping(child));
        assert!(is_nested_scalar_section(&map));
    }

    #[test]
    fn test_is_nested_scalar_section_empty_child() {
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(
            Value::String("empty".to_string()),
            Value::Mapping(serde_yaml_ng::Mapping::new()),
        );
        assert!(!is_nested_scalar_section(&map));
    }

    #[test]
    fn test_parse_table_column_scalar_not_array() {
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(Value::String("col".to_string()), Value::Number(42.into()));
        let result = parse_table("test", &map);
        assert!(result.is_err());
    }
}
