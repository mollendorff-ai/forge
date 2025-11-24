use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
use jsonschema::JSONSchema;
use serde_yaml::Value;

/// Parse a Forge model file (v1.0.0 array format) and return a ParsedModel.
///
/// This is the main entry point for parsing Forge YAML files.
///
/// # Arguments
/// * `path` - Path to the Forge YAML file
///
/// # Returns
/// * `Ok(ParsedModel)` - Successfully parsed model with tables and scalars
/// * `Err(ForgeError)` - Parse error with detailed context
///
/// # Example
/// ```no_run
/// use royalbit_forge::parser::parse_model;
/// use std::path::Path;
///
/// let model = parse_model(Path::new("model.yaml"))?;
/// println!("Tables: {}", model.tables.len());
/// # Ok::<(), royalbit_forge::error::ForgeError>(())
/// ```
pub fn parse_model(path: &std::path::Path) -> ForgeResult<ParsedModel> {
    let content = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;

    parse_v1_model(&yaml)
}

/// Parse v1.0.0 array model
fn parse_v1_model(yaml: &Value) -> ForgeResult<ParsedModel> {
    // Optionally validate against JSON Schema if available
    if let Err(e) = validate_against_schema(yaml) {
        // Schema validation is optional - warn but continue
        eprintln!("Warning: Schema validation failed: {}", e);
    }

    let mut model = ParsedModel::new();

    // Parse each top-level key as either a table or scalar
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key
                .as_str()
                .ok_or_else(|| ForgeError::Parse("Table name must be a string".to_string()))?;

            // Skip special keys
            if key_str == "_forge_version" {
                continue;
            }

            // Check if this is a table (mapping with arrays) or scalar (mapping with value/formula)
            if let Value::Mapping(inner_map) = value {
                // Check if it has {value, formula} pattern (scalar)
                if inner_map.contains_key("value") || inner_map.contains_key("formula") {
                    // This is a scalar variable
                    let variable = parse_scalar_variable(value, key_str)?;
                    model.add_scalar(key_str.to_string(), variable);
                } else if is_nested_scalar_section(inner_map) {
                    // This is a section containing nested scalars (e.g., summary.total)
                    parse_nested_scalars(key_str, inner_map, &mut model)?;
                } else {
                    // This is a table - parse it
                    let table = parse_table(key_str, inner_map)?;
                    model.add_table(table);
                }
            }
        }
    }

    // Validate all tables
    for (name, table) in &model.tables {
        table
            .validate_lengths()
            .map_err(|e| ForgeError::Validation(format!("Table '{}': {}", name, e)))?;
    }

    Ok(model)
}

/// Validate YAML against the Forge v1.0.0 JSON Schema
fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Load the JSON Schema from the embedded schema file
    let schema_str = include_str!("../../schema/forge-v1.0.schema.json");
    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse schema: {}", e)))?;

    // Compile the schema
    let compiled_schema = JSONSchema::compile(&schema_value)
        .map_err(|e| ForgeError::Validation(format!("Failed to compile schema: {}", e)))?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value = serde_json::to_value(yaml)
        .map_err(|e| ForgeError::Validation(format!("Failed to convert YAML to JSON: {}", e)))?;

    // Validate
    if let Err(errors) = compiled_schema.validate(&json_value) {
        let error_messages: Vec<String> = errors.map(|e| format!("  - {}", e)).collect();
        return Err(ForgeError::Validation(format!(
            "Schema validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    Ok(())
}

/// Check if a mapping contains nested scalar sections (e.g., summary.total)
fn is_nested_scalar_section(map: &serde_yaml::Mapping) -> bool {
    // Check if all children are mappings with {value, formula} pattern
    for (_key, value) in map {
        if let Value::Mapping(child_map) = value {
            // Check if this child has value or formula keys
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                return true;
            }
        }
    }
    false
}

/// Parse nested scalar variables (e.g., summary.total, summary.average)
fn parse_nested_scalars(
    parent_key: &str,
    map: &serde_yaml::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (key, value) in map {
        let key_str = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scalar name must be a string".to_string()))?;

        if let Value::Mapping(child_map) = value {
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                // This is a scalar variable
                let full_path = format!("{}.{}", parent_key, key_str);
                let variable = parse_scalar_variable(value, &full_path)?;
                model.add_scalar(full_path.clone(), variable);
            }
        }
    }
    Ok(())
}

/// Parse a table from a YAML mapping
fn parse_table(name: &str, map: &serde_yaml::Mapping) -> ForgeResult<Table> {
    let mut table = Table::new(name.to_string());

    for (key, value) in map {
        let col_name = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Column name must be a string".to_string()))?;

        // Check if this is a formula (string starting with =)
        if let Value::String(s) = value {
            if s.starts_with('=') {
                // This is a row-wise formula
                table.add_row_formula(col_name.to_string(), s.clone());
                continue;
            }
        }

        // Otherwise, it's a data column (array)
        if let Value::Sequence(seq) = value {
            let column_value = parse_array_value(col_name, seq)?;
            let column = Column::new(col_name.to_string(), column_value);
            table.add_column(column);
        } else {
            return Err(ForgeError::Parse(format!(
                "Column '{}' in table '{}' must be an array or formula",
                col_name, name
            )));
        }
    }

    Ok(table)
}

/// Parse a scalar variable
fn parse_scalar_variable(value: &Value, path: &str) -> ForgeResult<Variable> {
    if let Value::Mapping(map) = value {
        let val = map.get("value").and_then(|v| v.as_f64());
        let formula = map
            .get("formula")
            .and_then(|f| f.as_str().map(std::string::ToString::to_string));

        Ok(Variable {
            path: path.to_string(),
            value: val,
            formula,
        })
    } else {
        Err(ForgeError::Parse(format!(
            "Expected mapping for scalar variable '{}'",
            path
        )))
    }
}

/// Parse a YAML array into a typed ColumnValue
fn parse_array_value(col_name: &str, seq: &[Value]) -> ForgeResult<ColumnValue> {
    if seq.is_empty() {
        return Err(ForgeError::Parse(format!(
            "Column '{}' cannot be empty",
            col_name
        )));
    }

    // Detect the type from the first element
    let array_type = detect_array_type(&seq[0])?;

    match array_type {
        "Number" => {
            let mut numbers = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            numbers.push(f);
                        } else {
                            return Err(ForgeError::Parse(format!(
                                "Column '{}' row {}: Invalid number format",
                                col_name, i
                            )));
                        }
                    }
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Number, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Number(numbers))
        }
        "Text" => {
            let mut texts = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => texts.push(s.clone()),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Text, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Text(texts))
        }
        "Date" => {
            let mut dates = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => {
                        // Validate date format (YYYY-MM or YYYY-MM-DD)
                        if !is_valid_date_format(s) {
                            return Err(ForgeError::Parse(format!(
                                "Column '{}' row {}: Invalid date format '{}' (expected YYYY-MM or YYYY-MM-DD)",
                                col_name, i, s
                            )));
                        }
                        dates.push(s.clone());
                    }
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Date, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Date(dates))
        }
        "Boolean" => {
            let mut bools = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Bool(b) => bools.push(*b),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Boolean, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Boolean(bools))
        }
        _ => Err(ForgeError::Parse(format!(
            "Column '{}': Unsupported array type '{}'",
            col_name, array_type
        ))),
    }
}

/// Detect the type of a YAML value
fn detect_array_type(val: &Value) -> ForgeResult<&'static str> {
    match val {
        Value::Number(_) => Ok("Number"),
        Value::String(s) => {
            // Check if it's a date string
            if is_valid_date_format(s) {
                Ok("Date")
            } else {
                Ok("Text")
            }
        }
        Value::Bool(_) => Ok("Boolean"),
        _ => Err(ForgeError::Parse(format!(
            "Unsupported array element type: {}",
            type_name(val)
        ))),
    }
}

/// Check if a string is a valid date format (YYYY-MM or YYYY-MM-DD)
fn is_valid_date_format(s: &str) -> bool {
    // YYYY-MM format
    if s.len() == 7 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit());
        }
    }
    // YYYY-MM-DD format
    if s.len() == 10 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 3 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit())
                && parts[2].len() == 2
                && parts[2].chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

/// Get the type name of a YAML value for error messages
fn type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Sequence(_) => "Array",
        Value::Mapping(_) => "Mapping",
        Value::Tagged(_) => "Tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // v1.0.0 Array Model Tests
    // =========================================================================

    #[test]
    fn test_parse_number_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::Number(1.into()),
            Value::Number(2.into()),
            Value::Number(3.into()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums, vec![1.0, 2.0, 3.0]);
            }
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_parse_text_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("A".to_string()),
            Value::String("B".to_string()),
            Value::String("C".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts, vec!["A", "B", "C"]);
            }
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_parse_date_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()),
            Value::String("2025-02".to_string()),
            Value::String("2025-03".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Date(dates) => {
                assert_eq!(dates, vec!["2025-01", "2025-02", "2025-03"]);
            }
            _ => panic!("Expected Date array"),
        }
    }

    #[test]
    fn test_parse_boolean_array() {
        let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools, vec![true, false, true]);
            }
            _ => panic!("Expected Boolean array"),
        }
    }

    #[test]
    fn test_mixed_type_array_error() {
        let yaml_seq: Vec<Value> = vec![Value::Number(1.into()), Value::String("text".to_string())];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Expected Number, found String"));
    }

    #[test]
    fn test_empty_array_error() {
        let yaml_seq: Vec<Value> = vec![];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cannot be empty"));
    }

    #[test]
    fn test_invalid_date_format_error() {
        // Mix valid date format with invalid - should fail
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()), // Valid date
            Value::String("2025-1".to_string()),  // Invalid: needs zero padding
        ];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid date format"));
    }

    #[test]
    fn test_valid_date_formats() {
        // YYYY-MM format
        assert!(is_valid_date_format("2025-01"));
        assert!(is_valid_date_format("2025-12"));

        // YYYY-MM-DD format
        assert!(is_valid_date_format("2025-01-15"));
        assert!(is_valid_date_format("2025-12-31"));

        // Invalid formats
        assert!(!is_valid_date_format("2025-1"));
        assert!(!is_valid_date_format("2025-1-1"));
        assert!(!is_valid_date_format("25-01-01"));
        assert!(!is_valid_date_format("not-a-date"));
    }

    #[test]
    fn test_parse_table_with_arrays() {
        let yaml = r"
        month: ['Jan', 'Feb', 'Mar']
        revenue: [100, 200, 300]
        ";
        let parsed: Value = serde_yaml::from_str(yaml).unwrap();

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
        let parsed: Value = serde_yaml::from_str(yaml).unwrap();

        if let Value::Mapping(map) = parsed {
            let table = parse_table("test_table", &map).unwrap();

            assert_eq!(table.columns.len(), 2); // Only data columns
            assert_eq!(table.row_formulas.len(), 1); // One formula
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
            ColumnValue::Number(vec![4.0, 5.0]), // Different length!
        ));

        let result = table.validate_lengths();
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        // HashMap iteration order is not guaranteed, so error could mention either column
        assert!(err_msg.contains("col1") || err_msg.contains("col2"));
        assert!(err_msg.contains("2 rows"));
        assert!(err_msg.contains("3 rows"));
    }

    #[test]
    fn test_parse_v1_model_simple() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "1.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.2"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert!(result.tables.contains_key("sales"));

        let sales_table = result.tables.get("sales").unwrap();
        assert_eq!(sales_table.columns.len(), 2); // month, revenue
        assert_eq!(sales_table.row_formulas.len(), 1); // profit
    }

    #[test]
    fn test_parse_v1_model_with_scalars() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "1.0.0"

data:
  values: [1, 2, 3]

summary:
  total:
    value: null
    formula: "=SUM(data.values)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.scalars.len(), 1);
        assert!(result.scalars.contains_key("summary.total"));
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
}
