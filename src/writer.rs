use crate::error::ForgeResult;
use crate::types::{ParsedYaml, Variable};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Update all YAML files (main + includes) with calculated values - Excel-style
pub fn update_all_yaml_files(
    main_file_path: &Path,
    parsed: &ParsedYaml,
    calculated_values: &HashMap<String, f64>,
    all_variables: &HashMap<String, Variable>,
) -> ForgeResult<()> {
    // Group calculated values by source file
    let mut values_by_file: HashMap<Option<String>, HashMap<String, f64>> = HashMap::new();

    for (var_name, &calculated_value) in calculated_values {
        if let Some(var) = all_variables.get(var_name) {
            let file_key = var.alias.clone();
            values_by_file
                .entry(file_key)
                .or_insert_with(HashMap::new)
                .insert(var.path.clone(), calculated_value);
        }
    }

    // Update main file (variables with alias = None)
    if let Some(main_values) = values_by_file.get(&None) {
        update_yaml_file(main_file_path, main_values)?;
    }

    // Update each included file
    let base_dir = main_file_path.parent().unwrap_or_else(|| Path::new("."));
    for include in &parsed.includes {
        let file_key = Some(include.r#as.clone());
        if let Some(include_values) = values_by_file.get(&file_key) {
            let include_path = base_dir.join(&include.file);
            update_yaml_file(&include_path, include_values)?;
        }
    }

    Ok(())
}

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
    use crate::types::Include;
    use tempfile::{NamedTempFile, tempdir};
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

    #[test]
    fn test_update_all_yaml_files_with_includes() {
        let temp_dir = tempdir().unwrap();

        // Create main file
        let main_path = temp_dir.path().join("main.yaml");
        fs::write(&main_path, r#"
includes:
  - file: included.yaml
    as: inc

total:
  value: 0.0
  formula: "=@inc.base * 2"
"#).unwrap();

        // Create included file
        let included_path = temp_dir.path().join("included.yaml");
        fs::write(&included_path, r#"
base:
  value: 0.0
  formula: "=10 * 5"
"#).unwrap();

        // Set up test data
        let mut all_variables = HashMap::new();
        all_variables.insert("total".to_string(), Variable {
            path: "total".to_string(),
            value: Some(0.0),
            formula: Some("=@inc.base * 2".to_string()),
            alias: None,
        });
        all_variables.insert("@inc.base".to_string(), Variable {
            path: "base".to_string(),
            value: Some(0.0),
            formula: Some("=10 * 5".to_string()),
            alias: Some("inc".to_string()),
        });

        let mut calculated_values = HashMap::new();
        calculated_values.insert("total".to_string(), 100.0);
        calculated_values.insert("@inc.base".to_string(), 50.0);

        let parsed = ParsedYaml {
            includes: vec![Include {
                file: "included.yaml".to_string(),
                r#as: "inc".to_string(),
            }],
            variables: all_variables.clone(),
        };

        // Update all files
        update_all_yaml_files(&main_path, &parsed, &calculated_values, &all_variables).unwrap();

        // Verify main file was updated
        let main_content = fs::read_to_string(&main_path).unwrap();
        assert!(main_content.contains("100"));

        // Verify included file was updated
        let included_content = fs::read_to_string(&included_path).unwrap();
        assert!(included_content.contains("50"));
    }

    #[test]
    fn test_update_all_files_with_multiple_includes() {
        let temp_dir = tempdir().unwrap();

        // Create main file
        let main_path = temp_dir.path().join("main.yaml");
        fs::write(&main_path, r#"
includes:
  - file: pricing.yaml
    as: pricing
  - file: costs.yaml
    as: costs

margin:
  value: 0.0
  formula: "=@pricing.price - @costs.cost"
"#).unwrap();

        // Create pricing file
        let pricing_path = temp_dir.path().join("pricing.yaml");
        fs::write(&pricing_path, r#"
price:
  value: 0.0
  formula: "=100 * 1.2"
"#).unwrap();

        // Create costs file
        let costs_path = temp_dir.path().join("costs.yaml");
        fs::write(&costs_path, r#"
cost:
  value: 0.0
  formula: "=50 + 10"
"#).unwrap();

        // Set up test data
        let mut all_variables = HashMap::new();
        all_variables.insert("margin".to_string(), Variable {
            path: "margin".to_string(),
            value: Some(0.0),
            formula: Some("=@pricing.price - @costs.cost".to_string()),
            alias: None,
        });
        all_variables.insert("@pricing.price".to_string(), Variable {
            path: "price".to_string(),
            value: Some(0.0),
            formula: Some("=100 * 1.2".to_string()),
            alias: Some("pricing".to_string()),
        });
        all_variables.insert("@costs.cost".to_string(), Variable {
            path: "cost".to_string(),
            value: Some(0.0),
            formula: Some("=50 + 10".to_string()),
            alias: Some("costs".to_string()),
        });

        let mut calculated_values = HashMap::new();
        calculated_values.insert("margin".to_string(), 60.0);
        calculated_values.insert("@pricing.price".to_string(), 120.0);
        calculated_values.insert("@costs.cost".to_string(), 60.0);

        let parsed = ParsedYaml {
            includes: vec![
                Include {
                    file: "pricing.yaml".to_string(),
                    r#as: "pricing".to_string(),
                },
                Include {
                    file: "costs.yaml".to_string(),
                    r#as: "costs".to_string(),
                },
            ],
            variables: all_variables.clone(),
        };

        // Update all files
        update_all_yaml_files(&main_path, &parsed, &calculated_values, &all_variables).unwrap();

        // Verify all files were updated
        let main_content = fs::read_to_string(&main_path).unwrap();
        assert!(main_content.contains("60"));

        let pricing_content = fs::read_to_string(&pricing_path).unwrap();
        assert!(pricing_content.contains("120"));

        let costs_content = fs::read_to_string(&costs_path).unwrap();
        assert!(costs_content.contains("60"));
    }
}
