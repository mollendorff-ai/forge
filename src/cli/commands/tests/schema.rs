//! Schema tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_split_scalars_preserves_special_keys() {
    let yaml_str = r#"
_forge_version: "4.0.0"
_name: "test"
inputs:
  existing: {value: 1}
"#;
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // Special keys should be preserved
    assert!(yaml_map.contains_key(serde_yaml_ng::Value::String("_forge_version".to_string())));
    assert!(yaml_map.contains_key(serde_yaml_ng::Value::String("_name".to_string())));
}

#[test]
fn test_split_scalars_moves_value_only_to_inputs() {
    let yaml_str = r#"
_forge_version: "4.0.0"
my_input:
  value: 100
"#;
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // my_input should move to inputs
    assert!(!yaml_map.contains_key(serde_yaml_ng::Value::String("my_input".to_string())));
    let inputs = yaml_map.get(serde_yaml_ng::Value::String("inputs".to_string()));
    assert!(inputs.is_some());
}

#[test]
fn test_split_scalars_moves_formula_to_outputs() {
    let yaml_str = r#"
_forge_version: "4.0.0"
my_output:
  value: 200
  formula: "=x * 2"
"#;
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_str).unwrap();
    let yaml_map = yaml.as_mapping_mut().unwrap();

    split_scalars_to_inputs_outputs(yaml_map, false).unwrap();

    // my_output should move to outputs
    assert!(!yaml_map.contains_key(serde_yaml_ng::Value::String("my_output".to_string())));
    let outputs = yaml_map.get(serde_yaml_ng::Value::String("outputs".to_string()));
    assert!(outputs.is_some());
}
