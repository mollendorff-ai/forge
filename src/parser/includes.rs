//! Include file resolution for Forge models (v4.0)
//!
//! Handles parsing and resolution of _includes sections for cross-file references.

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Include, ParsedModel, ResolvedInclude};
use serde_yaml_ng::Value;
use std::collections::HashSet;
use std::path::Path;

use super::model::parse_v1_model;

/// Resolve all includes in a model, loading and parsing referenced files.
/// Detects circular dependencies.
pub fn resolve_includes(
    model: &mut ParsedModel,
    base_path: &Path,
    visited: &mut HashSet<std::path::PathBuf>,
) -> ForgeResult<()> {
    let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));

    // Check for circular dependency
    let canonical = base_path
        .canonicalize()
        .unwrap_or_else(|_| base_path.to_path_buf());
    if visited.contains(&canonical) {
        return Err(ForgeError::Parse(format!(
            "Circular dependency detected: {} is already included",
            base_path.display()
        )));
    }
    visited.insert(canonical);

    // Process each include
    for include in model.includes.clone() {
        let include_path = base_dir.join(&include.file);

        if !include_path.exists() {
            return Err(ForgeError::Parse(format!(
                "Included file not found: {} (referenced as '{}')",
                include_path.display(),
                include.file
            )));
        }

        // Parse the included file
        let content = std::fs::read_to_string(&include_path)?;
        let yaml: Value = serde_yaml_ng::from_str(&content)?;
        let mut included_model = parse_v1_model(&yaml)?;

        // Recursively resolve includes in the included file
        if !included_model.includes.is_empty() {
            resolve_includes(&mut included_model, &include_path, visited)?;
        }

        // Store resolved include
        let resolved = ResolvedInclude {
            include: include.clone(),
            resolved_path: include_path.canonicalize().unwrap_or(include_path),
            model: included_model,
        };
        model
            .resolved_includes
            .insert(include.namespace.clone(), resolved);
    }

    Ok(())
}

/// Parse _includes section from YAML (v4.0 cross-file references)
///
/// Expected format:
/// ```yaml
/// _includes:
///   - file: "data_sources.yaml"
///     as: "sources"
///   - file: "pricing.yaml"
///     as: "pricing"
/// ```
pub fn parse_includes(includes_seq: &[Value], model: &mut ParsedModel) -> ForgeResult<()> {
    for include_val in includes_seq {
        if let Value::Mapping(include_map) = include_val {
            // Extract 'file' field (required)
            let file = include_map
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ForgeError::Parse("Include must have a 'file' field".to_string()))?
                .to_string();

            // Extract 'as' field (required - the namespace alias)
            let namespace = include_map
                .get("as")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ForgeError::Parse(format!(
                        "Include '{file}' must have an 'as' field for the namespace"
                    ))
                })?
                .to_string();

            model.add_include(Include::new(file, namespace));
        } else {
            return Err(ForgeError::Parse(
                "Each include must be a mapping with 'file' and 'as' fields".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_parse_includes_section() {
        let temp_dir = TempDir::new().unwrap();

        let included_path = temp_dir.path().join("external.yaml");
        std::fs::write(
            &included_path,
            r#"
_forge_version: "5.0.0"
ext_data:
  values: [10, 20, 30]
"#,
        )
        .unwrap();

        let main_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "external.yaml"
    as: "ext"
main_data:
  values: [1, 2, 3]
"#
        .to_string();

        let main_path = temp_dir.path().join("main.yaml");
        std::fs::write(&main_path, main_content).unwrap();

        let content = std::fs::read_to_string(&main_path).unwrap();
        let yaml: Value = serde_yaml_ng::from_str(&content).unwrap();
        let mut model = parse_v1_model(&yaml).unwrap();

        resolve_includes(&mut model, &main_path, &mut HashSet::new()).unwrap();

        assert!(model.tables.contains_key("main_data"));
        assert!(model.resolved_includes.contains_key("ext"));
    }

    #[test]
    fn test_parse_includes_missing_file() {
        let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "nonexistent.yaml"
    as: "ext"
data:
  values: [1, 2, 3]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let yaml: Value = serde_yaml_ng::from_str(&content).unwrap();
        let mut model = parse_v1_model(&yaml).unwrap();

        let result = resolve_includes(&mut model, temp_file.path(), &mut HashSet::new());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("nonexistent"));
    }

    #[test]
    fn test_parse_includes_missing_as_field() {
        let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - file: "external.yaml"
data:
  values: [1, 2, 3]
"#;

        let yaml: Value = serde_yaml_ng::from_str(yaml_content).unwrap();

        // Try to parse the includes section
        if let Some(Value::Sequence(includes_seq)) = yaml.get("_includes") {
            let mut model = ParsedModel::new();
            let result = parse_includes(includes_seq, &mut model);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_includes_invalid_format() {
        let yaml_content = r#"
_forge_version: "5.0.0"
_includes:
  - "just a string, not a mapping"
data:
  values: [1, 2, 3]
"#;

        let yaml: Value = serde_yaml_ng::from_str(yaml_content).unwrap();

        // Try to parse the includes section
        if let Some(Value::Sequence(includes_seq)) = yaml.get("_includes") {
            let mut model = ParsedModel::new();
            let result = parse_includes(includes_seq, &mut model);
            assert!(result.is_err());
        }
    }
}
