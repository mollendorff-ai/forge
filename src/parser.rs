use crate::error::{ForgeError, ForgeResult};
use crate::types::Variable;
use serde_yaml::Value;
use std::collections::HashMap;

/// Parse YAML and extract all variables with formulas
pub fn parse_yaml_file(path: &std::path::Path) -> ForgeResult<HashMap<String, Variable>> {
    let content = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;

    let mut variables = HashMap::new();
    extract_variables(&yaml, String::new(), &mut variables)?;

    Ok(variables)
}

/// Recursively extract variables from YAML structure
fn extract_variables(
    value: &Value,
    path: String,
    variables: &mut HashMap<String, Variable>,
) -> ForgeResult<()> {
    match value {
        Value::Mapping(map) => {
            // Check if this is a variable (has "value" key)
            if let Some(val) = map.get("value") {
                let formula = map.get("formula");
                let var = Variable {
                    path: path.clone(),
                    value: val.as_f64(),
                    formula: formula.and_then(|f| f.as_str().map(|s| s.to_string())),
                };

                // Include variables with formulas OR base variables with values
                if let Some(f) = &var.formula {
                    if f.starts_with('=') {
                        variables.insert(path.clone(), var);
                    }
                } else if var.value.is_some() {
                    // Base variable (no formula, but has a value)
                    variables.insert(path.clone(), var);
                }
            }

            // Recursively process all map entries
            for (key, val) in map {
                if let Some(key_str) = key.as_str() {
                    let new_path = if path.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", path, key_str)
                    };
                    extract_variables(val, new_path, variables)?;
                }
            }
        }
        Value::Sequence(seq) => {
            for (i, val) in seq.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                extract_variables(val, new_path, variables)?;
            }
        }
        Value::Number(num) => {
            // Handle plain scalar numbers (e.g., louis_annual: 75000)
            if !path.is_empty() {
                if let Some(f64_val) = num.as_f64() {
                    let var = Variable {
                        path: path.clone(),
                        value: Some(f64_val),
                        formula: None,
                    };
                    variables.insert(path, var);
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
        extract_variables(&parsed, String::new(), &mut variables).unwrap();

        // Parser extracts both gross_margin and gross_margin.value
        assert_eq!(variables.len(), 2);
        assert!(variables.contains_key("gross_margin"));

        // Verify the formula is extracted
        let var = variables.get("gross_margin").unwrap();
        assert_eq!(var.formula, Some("=1 - platform_take_rate".to_string()));
        assert_eq!(var.value, Some(0.90));
    }
}
