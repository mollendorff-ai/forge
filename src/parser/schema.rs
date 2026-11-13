//! JSON Schema validation for Forge YAML models
//!
//! Validates YAML against embedded JSON schemas (v1.0.0, v5.0.0, v5.1.0, v5.2.0, v5.3.0).

use crate::error::{ForgeError, ForgeResult};
use yaml_serde::Value;

/// Validate YAML against the appropriate Forge JSON Schema based on _`forge_version`
///
/// # Errors
///
/// Returns an error if the YAML is missing `_forge_version`, specifies an unsupported
/// version, or fails schema validation.
pub fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Extract the _forge_version to determine which schema to use
    let version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ForgeError::Validation(
                "Missing required field: _forge_version (the model format version). Must be \"1.0.0\", \"5.0.0\", \"5.1.0\", \"5.2.0\" or \"5.3.0\""
                    .to_string(),
            )
        })?;

    // Load the appropriate schema based on version
    let schema_str = match version {
        "1.0.0" => include_str!("../../schema/forge-v1.0.0.schema.json"),
        "5.0.0" => include_str!("../../schema/forge-v5.0.0.schema.json"),
        "5.1.0" => include_str!("../../schema/forge-v5.1.0.schema.json"),
        "5.2.0" => include_str!("../../schema/forge-v5.2.0.schema.json"),
        "5.3.0" => include_str!("../../schema/forge-v5.3.0.schema.json"),
        _ => {
            return Err(ForgeError::Validation(format!(
                "Unsupported _forge_version: '{version}'. Supported model formats: 1.0.0 (scalar-only), 5.0.0 (arrays/tables), 5.1.0 (5.0.0 plus the short form `price: 16`), 5.2.0 (5.1.0 plus period columns), 5.3.0 (5.2.0 plus empty cells). This is the model format version, not the Forge version: write `_forge_version: \"5.1.0\"` for a new model (`\"5.2.0\"` if a table has a period column), or start from `forge examples starter`"
            )));
        },
    };

    check_short_form(yaml, version)?;
    check_period_columns(yaml, version)?;
    check_empty_cells(yaml, version)?;
    // Tables in a 1.0.0 model: say so before the schema reports the shapes
    if version == "1.0.0" {
        validate_v1_0_0_no_tables(yaml)?;
    }

    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse schema: {e}")))?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value = serde_json::to_value(yaml)
        .map_err(|e| ForgeError::Validation(format!("Failed to convert YAML to JSON: {e}")))?;

    // Build the validator
    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| ForgeError::Validation(format!("Failed to compile schema: {e}")))?;

    // Validate
    if validator.validate(&json_value).is_err() {
        return Err(ForgeError::Validation(format!(
            "The model does not fit model format {version}:\n{}",
            super::schema_errors::describe_errors(
                validator.iter_errors(&json_value),
                &json_value,
                &schema_value
            )
        )));
    }

    Ok(())
}

/// A period column in a model of an earlier format: say which format has it,
/// before the schema reports the keys it does not know.
fn check_period_columns(yaml: &Value, version: &str) -> ForgeResult<()> {
    let (Some(root), false) = (yaml.as_mapping(), matches!(version, "5.2.0" | "5.3.0")) else {
        return Ok(());
    };
    for (key, value) in root {
        let (Some(table), Some(columns)) = (key.as_str(), value.as_mapping()) else {
            continue;
        };
        let period = columns.iter().find_map(|(name, column)| {
            column
                .as_mapping()
                .filter(|m| crate::parser::periods::is_period_column(m))
                .and(name.as_str())
        });
        if let Some(column) = period {
            return Err(ForgeError::Validation(format!(
                "'{table}.{column}' is a period column (start, count, unit), which model format \
                 5.2.0 adds. Set `_forge_version: \"5.2.0\"` (this file says {version})"
            )));
        }
    }
    Ok(())
}

/// An empty cell (null in a column of data) in a model of an earlier format:
/// say which format has it (ADR-059).
fn check_empty_cells(yaml: &Value, version: &str) -> ForgeResult<()> {
    let (Some(root), false) = (yaml.as_mapping(), version == "5.3.0") else {
        return Ok(());
    };
    for (key, value) in root {
        let (Some(table), Some(columns)) = (key.as_str(), value.as_mapping()) else {
            continue;
        };
        if !crate::parser::model::is_table(columns) {
            continue;
        }
        for (name, column) in columns {
            let values = match column {
                Value::Sequence(values) => values,
                Value::Mapping(rich) => match rich.get("value") {
                    Some(Value::Sequence(values)) => values,
                    _ => continue,
                },
                _ => continue,
            };
            if let Some(row) = values.iter().position(Value::is_null) {
                return Err(ForgeError::Validation(format!(
                    "'{table}.{}' row {} is empty (null), which model format 5.3.0 adds: an \
                     empty cell, skipped by aggregates and read as 0 by formulas, as in Excel \
                     (ADR-059). Set `_forge_version: \"5.3.0\"` (this file says {version}), or \
                     write 0 if the value is 0",
                    name.as_str().unwrap_or("?"),
                    row + 1
                )));
            }
        }
    }
    Ok(())
}

/// Clear messages for the two likely mistakes with the 5.1.0 short form
/// (ADR-051), before the schema reports them as an unmatched `anyOf`.
///
/// # Errors
///
/// Returns an error if a pre-5.1.0 model uses a plain number as a scalar, or
/// if a group of scalars has a bare formula string.
fn check_short_form(yaml: &Value, version: &str) -> ForgeResult<()> {
    const SECTIONS: [&str; 9] = [
        "_forge_version",
        "_name",
        "_includes",
        "scenarios",
        "monte_carlo",
        "tornado",
        "decision_tree",
        "real_options",
        "bootstrap",
    ];
    let Value::Mapping(root) = yaml else {
        return Ok(());
    };
    for (key, value) in root {
        let Some(key) = key.as_str() else { continue };
        if SECTIONS.contains(&key) || key == "bayesian_network" {
            continue;
        }
        // `price: {value: 100}` is one scalar, not a group with an entry `value`
        if let Value::Mapping(m) = value {
            if (m.contains_key("value") || m.contains_key("formula"))
                && !crate::parser::model::is_table(m)
            {
                continue;
            }
        }
        let numbers: Vec<String> = match value {
            Value::Number(_) => vec![key.to_string()],
            Value::Mapping(group) if !crate::parser::model::is_table(group) => group
                .iter()
                .filter(|(_, v)| matches!(v, Value::Number(_)))
                .filter_map(|(k, _)| k.as_str().map(|k| format!("{key}.{k}")))
                .collect(),
            _ => Vec::new(),
        };
        if !matches!(version, "5.1.0" | "5.2.0" | "5.3.0") {
            if let Some(first) = numbers.first() {
                return Err(ForgeError::Validation(format!(
                    "'{first}' is a plain number, which is the short form of model format 5.1.0. \
                     Set `_forge_version: \"5.1.0\"`, or write it as `{{value: ..., formula: null}}` for {version}"
                )));
            }
        }
        if let Value::Mapping(group) = value {
            let is_scalar_group = !crate::parser::model::is_table(group)
                && group.values().any(|v| {
                    matches!(v, Value::Number(_))
                        || matches!(v, Value::Mapping(m) if m.contains_key("value") || m.contains_key("formula"))
                });
            if is_scalar_group {
                for (k, v) in group {
                    if let (Some(k), Value::String(f)) = (k.as_str(), v) {
                        if f.starts_with('=') {
                            return Err(ForgeError::Validation(format!(
                                "'{key}.{k}' is a formula in a group of scalars; write it as \
                                 `{k}: {{formula: \"{f}\"}}` so its result has a place in the file \
                                 (a bare formula string is a column, and belongs in a table)"
                            )));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Runtime validation: v1.0.0 models must NOT contain tables (arrays)
/// This provides a clear error message when users try to use v5.0.0 features in a v1.0.0 model
///
/// # Errors
///
/// Returns an error if the v1.0.0 model contains tables, arrays, or `monte_carlo` sections.
pub fn validate_v1_0_0_no_tables(yaml: &Value) -> ForgeResult<()> {
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key.as_str().unwrap_or("");

            // Skip special keys
            if key_str == "_forge_version" || key_str == "_name" || key_str == "scenarios" {
                continue;
            }

            // Block monte_carlo in v1.0.0 (requires v5.0.0)
            if key_str == "monte_carlo" {
                return Err(ForgeError::Validation(
                    "monte_carlo requires v5.0.0+. \
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
                            v1.0.0 only supports scalar values.\n\
                            To use tables/arrays, upgrade to v5.0.0:\n\
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
                                v1.0.0 only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0:\n\
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
                                v1.0.0 only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0:\n\
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
        let yaml: Value = yaml_serde::from_str(&content).unwrap();
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
        let yaml: Value = yaml_serde::from_str(&content).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(
            result.is_ok(),
            "Schema should accept valid decision_tree section: {:?}",
            result.err()
        );
    }

    fn validate_str(yaml: &str) -> ForgeResult<()> {
        validate_against_schema(&yaml_serde::from_str(yaml).unwrap())
    }

    #[test]
    fn test_schema_accepts_row_formula_starting_with_aggregate_name() {
        // "=MAX(" matched both RowFormula and AggregationFormula, so `oneOf` rejected it
        let result = validate_str(
            r#"
_forge_version: "5.0.0"
t:
  ebit: [100, -50]
  tax: "=MAX(0, ebit * 0.25)"
  total: "=SUM(ebit)"
"#,
        );
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn test_schema_accepts_date_and_formula_arrays() {
        // Date strings and formula strings also match TextArray
        let result = validate_str(
            r#"
_forge_version: "5.0.0"
t:
  month: ["2025-01-01", "2025-02-01"]
  x: [1, 2]
  doubled: ["=x * 2", "=x * 2"]
"#,
        );
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn test_schema_accepts_command_sections() {
        let result = validate_str(
            r#"
_forge_version: "5.0.0"
price:
  value: 100
real_options:
  method: binomial
bootstrap:
  iterations: 1000
  data: [1, 2, 3]
bayesian_network:
  name: "Example"
"#,
        );
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn test_schema_still_rejects_invalid_column() {
        let result = validate_str(
            r#"
_forge_version: "5.0.0"
t:
  x: [1, "two", true]
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_v1_rejects_tables() {
        let yaml_str = r#"
_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
"#;
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
        let result = validate_v1_0_0_no_tables(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("monte_carlo"));
    }

    #[test]
    fn test_missing_forge_version() {
        let yaml_str = r"
price:
  value: 100
";
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
        let result = validate_against_schema(&yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}
