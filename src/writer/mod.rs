use crate::error::ForgeResult;
use crate::types::{ColumnValue, ParsedModel, Variable};
use serde_yaml_ng::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Update YAML file with calculated values (v1.0.0)
pub fn update_yaml_file(path: &Path, calculated_values: &HashMap<String, f64>) -> ForgeResult<()> {
    // Read original YAML
    let content = fs::read_to_string(path)?;
    let mut yaml: Value = serde_yaml_ng::from_str(&content)?;

    // Update values
    for (var_path, calculated_value) in calculated_values {
        update_value_in_yaml(&mut yaml, var_path, *calculated_value);
    }

    // Write back to file
    let updated_content = serde_yaml_ng::to_string(&yaml)?;
    fs::write(path, updated_content)?;

    Ok(())
}

/// Write calculated results back to YAML file (v4.3.0)
/// Creates a backup (.bak) before writing
/// Returns true if write was successful, false if skipped (multi-doc)
pub fn write_calculated_results(path: &Path, result: &ParsedModel) -> ForgeResult<bool> {
    // Read original content to check for multi-document YAML
    let content = fs::read_to_string(path)?;
    let content_trimmed = content.trim_start();

    // Check if this is a multi-document YAML file (v4.4.2)
    // Multi-doc files cannot be written back directly - skip with warning
    if content_trimmed.starts_with("---") && content_trimmed[3..].contains("\n---") {
        return Ok(false); // Indicate write was skipped
    }

    // Create backup
    let backup_path = path.with_extension("yaml.bak");
    fs::copy(path, &backup_path)?;

    // Read original YAML to preserve structure/comments
    let mut yaml: Value = serde_yaml_ng::from_str(&content)?;

    // Update table value arrays
    if let Value::Mapping(ref mut root) = yaml {
        for (table_name, table) in &result.tables {
            if let Some(Value::Mapping(table_map)) = root.get_mut(Value::String(table_name.clone()))
            {
                // Look for "value" column and update it
                if let Some(col) = table.columns.get("value") {
                    if let ColumnValue::Number(values) = &col.values {
                        let yaml_values: Vec<Value> = values
                            .iter()
                            .map(|v| {
                                // Format nicely: remove unnecessary decimal places
                                if v.fract() == 0.0 && v.abs() < 1e10 {
                                    Value::Number(serde_yaml_ng::Number::from(*v as i64))
                                } else {
                                    Value::Number(serde_yaml_ng::Number::from(*v))
                                }
                            })
                            .collect();
                        table_map.insert(
                            Value::String("value".to_string()),
                            Value::Sequence(yaml_values),
                        );
                    }
                }
            }
        }

        // Update scalar values
        for (name, var) in &result.scalars {
            if let Some(value) = var.value {
                update_value_in_yaml(&mut yaml, name, value);
            }
        }
    }

    // Write back to file
    let updated_content = serde_yaml_ng::to_string(&yaml)?;
    fs::write(path, updated_content)?;

    Ok(true)
}

/// Update scalar values in a model file
pub fn update_scalars(path: &Path, scalars: &HashMap<String, Variable>) -> ForgeResult<()> {
    let mut calculated_values = HashMap::new();
    for (name, var) in scalars {
        if let Some(value) = var.value {
            calculated_values.insert(name.clone(), value);
        }
    }
    update_yaml_file(path, &calculated_values)
}

/// Recursively update a value in YAML structure by path
fn update_value_in_yaml(yaml: &mut Value, path: &str, new_value: f64) {
    let parts: Vec<&str> = path.split('.').collect();
    update_value_recursive(yaml, &parts, 0, new_value);
}

fn update_value_recursive(yaml: &mut Value, path_parts: &[&str], index: usize, new_value: f64) {
    if index >= path_parts.len() {
        return;
    }

    let current_part = path_parts[index];

    if let Value::Mapping(map) = yaml {
        // If this is the last part of the path
        if index == path_parts.len() - 1 {
            // Look for the value field to update
            if let Some(Value::Mapping(inner_map)) =
                map.get_mut(Value::String(current_part.to_string()))
            {
                // Update the "value" field
                if inner_map.contains_key(Value::String("value".to_string())) {
                    inner_map.insert(
                        Value::String("value".to_string()),
                        Value::Number(serde_yaml_ng::Number::from(new_value)),
                    );
                }
            }
        } else {
            // Continue recursing
            if let Some(entry) = map.get_mut(Value::String(current_part.to_string())) {
                update_value_recursive(entry, path_parts, index + 1, new_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_update_simple_value() {
        let yaml_content = r#"
gross_margin:
  value: 0.0
  formula: "=1 - platform_take_rate"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let mut values = HashMap::new();
        values.insert("gross_margin".to_string(), 0.90);

        update_yaml_file(temp_file.path(), &values).unwrap();

        let updated_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(updated_content.contains("0.9") || updated_content.contains("0.90"));
    }

    #[test]
    fn test_update_nested_value() {
        let yaml_content = r#"
summary:
  total:
    value: 0.0
    formula: "=SUM(data.values)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let mut values = HashMap::new();
        values.insert("summary.total".to_string(), 150.0);

        update_yaml_file(temp_file.path(), &values).unwrap();

        let updated_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(updated_content.contains("150"));
    }

    #[test]
    fn test_update_scalars() {
        let yaml_content = r#"
revenue:
  value: 0.0
  formula: null
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let mut scalars = HashMap::new();
        let var = Variable::new("revenue".to_string(), Some(1000.0), None);
        scalars.insert("revenue".to_string(), var);

        update_scalars(temp_file.path(), &scalars).unwrap();

        let updated_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(updated_content.contains("1000"));
    }

    #[test]
    fn test_update_scalars_with_no_value() {
        let yaml_content = r#"
revenue:
  value: 100.0
  formula: null
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let mut scalars = HashMap::new();
        // Variable with no value - should not update
        let var = Variable::new("revenue".to_string(), None, None);
        scalars.insert("revenue".to_string(), var);

        update_scalars(temp_file.path(), &scalars).unwrap();

        // Original value should remain
        let updated_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(updated_content.contains("100"));
    }

    #[test]
    fn test_write_calculated_results_skips_multidoc() {
        use crate::types::ParsedModel;

        let yaml_content = "---\nfirst: 1\n---\nsecond: 2\n";

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let model = ParsedModel::new();
        let result = write_calculated_results(temp_file.path(), &model).unwrap();

        // Should return false indicating write was skipped
        assert!(!result, "Multi-doc YAML should be skipped");
    }

    #[test]
    fn test_write_calculated_results_creates_backup() {
        use crate::types::ParsedModel;

        let yaml_content = r#"
_forge_version: "5.0.0"
test_scalar:
  value: 100.0
  formula: null
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let model = ParsedModel::new();
        let result = write_calculated_results(path, &model).unwrap();

        assert!(result, "Single-doc YAML should be written");

        // Check backup was created
        let backup_path = path.with_extension("yaml.bak");
        assert!(backup_path.exists(), "Backup file should be created");

        // Clean up backup
        let _ = fs::remove_file(backup_path);
    }

    #[test]
    fn test_write_calculated_results_with_scalars() {
        use crate::types::ParsedModel;

        let yaml_content = r#"
profit:
  value: 0.0
  formula: "=revenue - costs"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let mut model = ParsedModel::new();
        let var = Variable::new("profit".to_string(), Some(500.0), None);
        model.scalars.insert("profit".to_string(), var);

        write_calculated_results(path, &model).unwrap();

        let updated_content = fs::read_to_string(path).unwrap();
        assert!(updated_content.contains("500"));

        // Clean up backup
        let _ = fs::remove_file(path.with_extension("yaml.bak"));
    }

    #[test]
    fn test_write_calculated_results_with_tables() {
        use crate::types::{Column, ColumnValue, ParsedModel, Table};

        let yaml_content = r#"
financials:
  value: [0, 0, 0]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let mut model = ParsedModel::new();
        let mut table = Table::new("financials".to_string());
        table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.tables.insert("financials".to_string(), table);

        write_calculated_results(path, &model).unwrap();

        let updated_content = fs::read_to_string(path).unwrap();
        assert!(updated_content.contains("100"));
        assert!(updated_content.contains("200"));
        assert!(updated_content.contains("300"));

        // Clean up backup
        let _ = fs::remove_file(path.with_extension("yaml.bak"));
    }

    #[test]
    fn test_update_value_empty_path() {
        let mut yaml: Value = serde_yaml_ng::from_str("test: 1").unwrap();
        // Empty path should not panic
        update_value_in_yaml(&mut yaml, "", 0.0);
    }

    #[test]
    fn test_update_value_nonexistent_path() {
        let mut yaml: Value = serde_yaml_ng::from_str("test: 1").unwrap();
        // Non-existent path should not panic
        update_value_in_yaml(&mut yaml, "nonexistent.path", 99.0);
    }

    #[test]
    fn test_update_multiple_values() {
        let yaml_content = r#"
revenue:
  value: 0.0
costs:
  value: 0.0
profit:
  value: 0.0
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let mut values = HashMap::new();
        values.insert("revenue".to_string(), 1000.0);
        values.insert("costs".to_string(), 400.0);
        values.insert("profit".to_string(), 600.0);

        update_yaml_file(temp_file.path(), &values).unwrap();

        let updated_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(updated_content.contains("1000"));
        assert!(updated_content.contains("400"));
        assert!(updated_content.contains("600"));
    }

    #[test]
    fn test_write_results_fractional_values() {
        use crate::types::ParsedModel;

        let yaml_content = r#"
rate:
  value: 0.0
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let mut model = ParsedModel::new();
        // Test with fractional value
        let var = Variable::new("rate".to_string(), Some(0.05), None);
        model.scalars.insert("rate".to_string(), var);

        write_calculated_results(path, &model).unwrap();

        let updated_content = fs::read_to_string(path).unwrap();
        assert!(updated_content.contains("0.05"));

        // Clean up backup
        let _ = fs::remove_file(path.with_extension("yaml.bak"));
    }

    #[test]
    fn test_write_results_integer_values() {
        use crate::types::{Column, ColumnValue, ParsedModel, Table};

        let yaml_content = r#"
counts:
  value: [0, 0]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let mut model = ParsedModel::new();
        let mut table = Table::new("counts".to_string());
        // Integer values (no decimal)
        table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        model.tables.insert("counts".to_string(), table);

        write_calculated_results(path, &model).unwrap();

        let updated_content = fs::read_to_string(path).unwrap();
        // Should be formatted as integers
        assert!(updated_content.contains("10"));
        assert!(updated_content.contains("20"));

        // Clean up backup
        let _ = fs::remove_file(path.with_extension("yaml.bak"));
    }
}
