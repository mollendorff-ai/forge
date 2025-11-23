use crate::error::ForgeResult;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Update YAML file with calculated values
pub fn update_yaml_file(
    path: &Path,
    calculated_values: &HashMap<String, f64>,
) -> ForgeResult<()> {
    // Read original YAML
    let content = fs::read_to_string(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;

    // Update values
    for (var_path, calculated_value) in calculated_values {
        update_value_in_yaml(&mut yaml, var_path, *calculated_value);
    }

    // Write back to file
    let updated_content = serde_yaml::to_string(&yaml)?;
    fs::write(path, updated_content)?;

    Ok(())
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

    match yaml {
        Value::Mapping(map) => {
            // If this is the last part of the path
            if index == path_parts.len() - 1 {
                // Look for the value field to update
                if let Some(entry) = map.get_mut(&Value::String(current_part.to_string())) {
                    if let Value::Mapping(inner_map) = entry {
                        // Update the "value" field
                        if inner_map.contains_key(&Value::String("value".to_string())) {
                            inner_map.insert(
                                Value::String("value".to_string()),
                                Value::Number(serde_yaml::Number::from(new_value)),
                            );
                        }
                    }
                }
            } else {
                // Continue recursing
                if let Some(entry) = map.get_mut(&Value::String(current_part.to_string())) {
                    update_value_recursive(entry, path_parts, index + 1, new_value);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

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
}
