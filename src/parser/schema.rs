//! JSON Schema validation for Forge YAML models
//!
//! Validates YAML against embedded JSON schemas (v1.0.0 and v5.0.0).

use crate::error::{ForgeError, ForgeResult};
use serde_yaml_ng::Value;

/// Validate YAML against the appropriate Forge JSON Schema based on _forge_version
pub fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Extract the _forge_version to determine which schema to use
    let version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ForgeError::Validation(
                "Missing required field: _forge_version. Must be \"1.0.0\" or \"5.0.0\""
                    .to_string(),
            )
        })?;

    // Load the appropriate schema based on version
    let schema_str = match version {
        "1.0.0" => include_str!("../../schema/forge-v1.0.0.schema.json"),
        "5.0.0" => include_str!("../../schema/forge-v5.0.0.schema.json"),
        _ => {
            return Err(ForgeError::Validation(format!(
                "Unsupported _forge_version: '{version}'. Supported versions: 1.0.0 (scalar-only for forge-demo), 5.0.0 (arrays/tables for enterprise)"
            )));
        },
    };

    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse schema: {e}")))?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value = serde_json::to_value(yaml)
        .map_err(|e| ForgeError::Validation(format!("Failed to convert YAML to JSON: {e}")))?;

    // Build the validator
    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| ForgeError::Validation(format!("Failed to compile schema: {e}")))?;

    // Validate
    if let Err(_error) = validator.validate(&json_value) {
        let error_messages: Vec<String> = validator
            .iter_errors(&json_value)
            .map(|e| format!("  - {e}"))
            .collect();
        return Err(ForgeError::Validation(format!(
            "Schema validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    // Additional runtime check for v1.0.0: NO tables/arrays allowed
    if version == "1.0.0" {
        validate_v1_0_0_no_tables(yaml)?;
    }

    Ok(())
}

/// Runtime validation: v1.0.0 models must NOT contain tables (arrays)
/// This provides a clear error message when users try to use enterprise features in forge-demo
pub fn validate_v1_0_0_no_tables(yaml: &Value) -> ForgeResult<()> {
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key.as_str().unwrap_or("");

            // Skip special keys (but error on enterprise features in v1.0.0)
            if key_str == "_forge_version" || key_str == "_name" || key_str == "scenarios" {
                continue;
            }

            // Block monte_carlo in v1.0.0 (enterprise feature)
            if key_str == "monte_carlo" {
                return Err(ForgeError::Validation(
                    "monte_carlo requires Forge Enterprise (v5.0.0+). \
                     This feature is not available in forge-demo. \
                     Upgrade to _forge_version: \"5.0.0\" to use Monte Carlo simulation."
                        .to_string(),
                ));
            }

            // Check if this is a table (mapping with arrays)
            if let Value::Mapping(inner_map) = value {
                // Skip if this is a scalar (has value/formula keys)
                if inner_map.contains_key("value") || inner_map.contains_key("formula") {
                    continue;
                }

                // Check if any child contains arrays (indicates a table)
                for (col_key, col_value) in inner_map {
                    let col_key_str = col_key.as_str().unwrap_or("");

                    // Check for direct array values (table columns)
                    if matches!(col_value, Value::Sequence(_)) {
                        return Err(ForgeError::Validation(format!(
                            "v1.0.0 models do not support tables/arrays. Found table '{key_str}' with array column '{col_key_str}'.\n\
                            \n\
                            v1.0.0 is for forge-demo and only supports scalar values.\n\
                            To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                            \n\
                            _forge_version: \"5.0.0\"\n\
                            \n\
                            Or convert your table to scalars using dot notation:\n\
                            {key_str}.{col_key_str}: {{ value: ..., formula: null }}\n\
                            {key_str}.{col_key_str}: {{ value: ..., formula: null }}"
                        )));
                    }

                    // Check for rich column format with array value
                    if let Value::Mapping(col_map) = col_value {
                        if let Some(Value::Sequence(_)) = col_map.get("value") {
                            return Err(ForgeError::Validation(format!(
                                "v1.0.0 models do not support tables/arrays. Found table '{key_str}' with array column '{col_key_str}' (rich format).\n\
                                \n\
                                v1.0.0 is for forge-demo and only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                                \n\
                                _forge_version: \"5.0.0\""
                            )));
                        }
                    }

                    // Check for row formulas (string starting with =)
                    if let Value::String(s) = col_value {
                        if s.starts_with('=') {
                            return Err(ForgeError::Validation(format!(
                                "v1.0.0 models do not support tables/arrays. Found table '{key_str}' with formula column '{col_key_str}'.\n\
                                \n\
                                v1.0.0 is for forge-demo and only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                                \n\
                                _forge_version: \"5.0.0\""
                            )));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_schema_validates_tornado_section() {
        // Verify schema accepts valid tornado configuration
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"_forge_version: "5.0.0"

price:
  value: 100

tornado:
  output: price
  inputs:
    - name: price
      low: 80
      high: 120
"#
        )
        .unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();
        let yaml: Value = serde_yaml_ng::from_str(&content).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(
            result.is_ok(),
            "Schema should accept valid tornado section: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_schema_validates_decision_tree_section() {
        // Verify schema accepts valid decision_tree configuration
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"_forge_version: "5.0.0"

decision_tree:
  name: "Simple Decision"
  root:
    type: decision
    branches:
      option_a:
        value: 100
      option_b:
        value: 200
"#
        )
        .unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();
        let yaml: Value = serde_yaml_ng::from_str(&content).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(
            result.is_ok(),
            "Schema should accept valid decision_tree section: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_v1_rejects_tables() {
        let yaml_str = r#"
_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = validate_v1_0_0_no_tables(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("v1.0.0"));
    }

    #[test]
    fn test_v1_allows_scalars() {
        let yaml_str = r#"
_forge_version: "1.0.0"
price:
  value: 100
  formula: null
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = validate_v1_0_0_no_tables(&yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_v1_rejects_monte_carlo() {
        let yaml_str = r#"
_forge_version: "1.0.0"
monte_carlo:
  iterations: 1000
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = validate_v1_0_0_no_tables(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("monte_carlo"));
    }

    #[test]
    fn test_missing_forge_version() {
        let yaml_str = r#"
price:
  value: 100
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("_forge_version"));
    }

    #[test]
    fn test_unsupported_version() {
        let yaml_str = r#"
_forge_version: "99.0.0"
price:
  value: 100
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}
