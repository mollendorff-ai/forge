use crate::error::{ForgeError, ForgeResult};
use crate::types::{Include, ParsedYaml, Variable};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::Path;

/// Parse YAML and extract all variables with formulas (with includes support)
pub fn parse_yaml_file(path: &Path) -> ForgeResult<HashMap<String, Variable>> {
    let parsed = parse_yaml_with_includes(path)?;
    Ok(parsed.variables)
}

/// Parse YAML file with includes and return complete parsed data
pub fn parse_yaml_with_includes(path: &Path) -> ForgeResult<ParsedYaml> {
    let content = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;

    // Extract includes from the YAML
    let includes = extract_includes(&yaml)?;

    // Parse main file variables
    let mut all_variables = HashMap::new();
    extract_variables(&yaml, String::new(), None, &mut all_variables)?;

    // Parse variables from each included file
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for include in &includes {
        let include_path = base_dir.join(&include.file);
        let include_content = std::fs::read_to_string(&include_path)
            .map_err(|e| ForgeError::Parse(format!(
                "Failed to read included file '{}': {}",
                include.file,
                e
            )))?;
        let include_yaml: Value = serde_yaml::from_str(&include_content)
            .map_err(|e| ForgeError::Parse(format!(
                "Failed to parse included file '{}': {}",
                include.file,
                e
            )))?;

        // Extract variables with the alias
        extract_variables(&include_yaml, String::new(), Some(include.r#as.clone()), &mut all_variables)?;
    }

    Ok(ParsedYaml {
        includes,
        variables: all_variables,
    })
}

/// Extract includes from YAML
fn extract_includes(value: &Value) -> ForgeResult<Vec<Include>> {
    if let Value::Mapping(map) = value {
        if let Some(includes_val) = map.get("includes") {
            if let Value::Sequence(seq) = includes_val {
                let mut includes = Vec::new();
                for item in seq {
                    let include: Include = serde_yaml::from_value(item.clone())
                        .map_err(|e| ForgeError::Parse(format!("Invalid include format: {}", e)))?;
                    includes.push(include);
                }
                return Ok(includes);
            }
        }
    }
    Ok(Vec::new())
}

/// Recursively extract variables from YAML structure
fn extract_variables(
    value: &Value,
    path: String,
    alias: Option<String>,
    variables: &mut HashMap<String, Variable>,
) -> ForgeResult<()> {
    match value {
        Value::Mapping(map) => {
            // Skip the "includes" key - don't process it as a variable
            if path == "includes" {
                return Ok(());
            }

            // Check if this is a variable (has "value" key)
            if let Some(val) = map.get("value") {
                let formula = map.get("formula");

                // Build the full variable key with alias prefix if present
                let var_key = if let Some(ref a) = alias {
                    format!("@{}.{}", a, path)
                } else {
                    path.clone()
                };

                let var = Variable {
                    path: path.clone(),
                    value: val.as_f64(),
                    formula: formula.and_then(|f| f.as_str().map(|s| s.to_string())),
                    alias: alias.clone(),
                };

                // Include variables with formulas OR base variables with values
                if let Some(f) = &var.formula {
                    if f.starts_with('=') {
                        variables.insert(var_key, var);
                    }
                } else if var.value.is_some() {
                    // Base variable (no formula, but has a value)
                    variables.insert(var_key, var);
                }
            }

            // Recursively process all map entries
            for (key, val) in map {
                if let Some(key_str) = key.as_str() {
                    // Skip "includes" key
                    if path.is_empty() && key_str == "includes" {
                        continue;
                    }

                    let new_path = if path.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", path, key_str)
                    };
                    extract_variables(val, new_path, alias.clone(), variables)?;
                }
            }
        }
        Value::Sequence(seq) => {
            for (i, val) in seq.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                extract_variables(val, new_path, alias.clone(), variables)?;
            }
        }
        Value::Number(num) => {
            // Handle plain scalar numbers (e.g., louis_annual: 75000)
            if !path.is_empty() {
                if let Some(f64_val) = num.as_f64() {
                    // Build the full variable key with alias prefix if present
                    let var_key = if let Some(ref a) = alias {
                        format!("@{}.{}", a, path)
                    } else {
                        path.clone()
                    };

                    let var = Variable {
                        path: path.clone(),
                        value: Some(f64_val),
                        formula: None,
                        alias: alias.clone(),
                    };
                    variables.insert(var_key, var);
                }
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_formula() {
        let yaml = r#"
        gross_margin:
          value: 0.90
          formula: "=1 - platform_take_rate"
        "#;

        let parsed: Value = serde_yaml::from_str(yaml).unwrap();
        let mut variables = HashMap::new();
        extract_variables(&parsed, String::new(), None, &mut variables).unwrap();

        // Parser extracts both gross_margin and gross_margin.value
        assert_eq!(variables.len(), 2);
        assert!(variables.contains_key("gross_margin"));

        // Verify the formula is extracted
        let var = variables.get("gross_margin").unwrap();
        assert_eq!(var.formula, Some("=1 - platform_take_rate".to_string()));
        assert_eq!(var.value, Some(0.90));
    }

    #[test]
    fn test_extract_includes() {
        let yaml = r#"
        includes:
          - file: pricing.yaml
            as: pricing
          - file: costs.yaml
            as: costs

        revenue:
          value: 100
          formula: null
        "#;

        let parsed: Value = serde_yaml::from_str(yaml).unwrap();
        let includes = extract_includes(&parsed).unwrap();

        assert_eq!(includes.len(), 2);
        assert_eq!(includes[0].file, "pricing.yaml");
        assert_eq!(includes[0].r#as, "pricing");
        assert_eq!(includes[1].file, "costs.yaml");
        assert_eq!(includes[1].r#as, "costs");
    }

    #[test]
    fn test_no_includes() {
        let yaml = r#"
        revenue:
          value: 100
          formula: null
        "#;

        let parsed: Value = serde_yaml::from_str(yaml).unwrap();
        let includes = extract_includes(&parsed).unwrap();

        assert_eq!(includes.len(), 0);
    }

    #[test]
    fn test_variables_with_alias_prefix() {
        let yaml = r#"
        base_price:
          value: 100
          formula: null
        "#;

        let parsed: Value = serde_yaml::from_str(yaml).unwrap();
        let mut variables = HashMap::new();
        extract_variables(&parsed, String::new(), Some("pricing".to_string()), &mut variables).unwrap();

        // Should have @pricing prefix
        assert!(variables.contains_key("@pricing.base_price"));
        let var = variables.get("@pricing.base_price").unwrap();
        assert_eq!(var.value, Some(100.0));
        assert_eq!(var.alias, Some("pricing".to_string()));
    }

    #[test]
    fn test_includes_key_not_treated_as_variable() {
        let yaml = r#"
        includes:
          - file: pricing.yaml
            as: pricing

        revenue:
          value: 100
          formula: null
        "#;

        let parsed: Value = serde_yaml::from_str(yaml).unwrap();
        let mut variables = HashMap::new();
        extract_variables(&parsed, String::new(), None, &mut variables).unwrap();

        // Should not have 'includes' as a variable
        assert!(!variables.contains_key("includes"));
        // Should have revenue
        assert!(variables.contains_key("revenue"));
    }
}
