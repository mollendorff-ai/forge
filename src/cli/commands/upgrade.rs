//! Upgrade command - migrate YAML files to latest schema version

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Check if a YAML file needs schema upgrade (v5.3.0)
/// Returns Some(current_version) if upgrade needed, None otherwise
/// Skips multi-doc files (not supported for auto-upgrade yet)
pub fn needs_schema_upgrade(file: &Path) -> ForgeResult<Option<String>> {
    let content = fs::read_to_string(file)
        .map_err(|e| ForgeError::IO(format!("Failed to read {}: {}", file.display(), e)))?;

    // Skip multi-doc files (contain "---" separator after first line)
    let is_multi_doc = content.lines().skip(1).any(|line| line.trim() == "---");
    if is_multi_doc {
        return Ok(None);
    }

    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
        .map_err(|e| ForgeError::Parse(format!("Failed to parse {}: {}", file.display(), e)))?;

    let version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    // Current schema version is 5.0.0
    if version == "5.0.0" {
        Ok(None)
    } else {
        Ok(Some(version.to_string()))
    }
}

/// Auto-upgrade schema for calculate command (v5.3.0)
pub fn auto_upgrade_schema(file: &Path, verbose: bool) -> ForgeResult<()> {
    let mut upgraded_files = HashSet::new();
    upgrade_file_recursive(file, "5.0.0", false, verbose, &mut upgraded_files)?;
    Ok(())
}

/// Execute the upgrade command - migrate YAML files to latest schema version
pub fn upgrade(
    file: PathBuf,
    dry_run: bool,
    target_version: String,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Schema Upgrade".bold().green());
    println!();
    println!("   File:    {}", file.display());
    println!("   Target:  v{}", target_version);
    if dry_run {
        println!("   Mode:    {} (no files modified)", "DRY RUN".yellow());
    }
    println!();

    // Track upgraded files to avoid circular processing
    let mut upgraded_files: HashSet<PathBuf> = HashSet::new();

    // Perform upgrade recursively
    let changes = upgrade_file_recursive(
        &file,
        &target_version,
        dry_run,
        verbose,
        &mut upgraded_files,
    )?;

    // Summary
    println!();
    println!("{}", "═".repeat(70));
    println!();
    if dry_run {
        println!(
            "{} {} file(s) would be upgraded",
            "DRY RUN:".yellow().bold(),
            changes
        );
        println!();
        println!("   Run without --dry-run to apply changes.");
    } else {
        println!(
            "{} {} file(s) upgraded to v{}",
            "✅".green(),
            changes,
            target_version
        );
    }
    println!();

    Ok(())
}

/// Recursively upgrade a file and its includes
pub fn upgrade_file_recursive(
    file: &Path,
    target_version: &str,
    dry_run: bool,
    verbose: bool,
    upgraded_files: &mut HashSet<PathBuf>,
) -> ForgeResult<usize> {
    // Canonicalize path to handle relative paths
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

    // Skip if already processed (circular include protection)
    if upgraded_files.contains(&canonical) {
        if verbose {
            println!(
                "   {} {} (already processed)",
                "⏭️".dimmed(),
                file.display()
            );
        }
        return Ok(0);
    }
    upgraded_files.insert(canonical.clone());

    // Read and parse the file
    let content = fs::read_to_string(file)
        .map_err(|e| ForgeError::IO(format!("Failed to read {}: {}", file.display(), e)))?;

    // Parse as YAML Value to manipulate
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
        .map_err(|e| ForgeError::Parse(format!("Failed to parse {}: {}", file.display(), e)))?;

    let mut changes = 0;

    // First, recursively upgrade any included files
    if let Some(serde_yaml_ng::Value::Sequence(include_list)) = yaml.get("_includes").cloned() {
        let parent_dir = file.parent().unwrap_or(Path::new("."));
        for include in include_list {
            if let Some(include_file) = include.get("file").and_then(|f| f.as_str()) {
                let include_path = parent_dir.join(include_file);
                if include_path.exists() {
                    changes += upgrade_file_recursive(
                        &include_path,
                        target_version,
                        dry_run,
                        verbose,
                        upgraded_files,
                    )?;
                }
            }
        }
    }

    // Get current version
    let current_version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    // Check if upgrade needed
    if current_version == target_version {
        if verbose {
            println!(
                "   {} {} (already v{})",
                "✓".green(),
                file.display(),
                target_version
            );
        }
        return Ok(changes);
    }

    println!(
        "   {} {} (v{} → v{})",
        if dry_run {
            "→".yellow()
        } else {
            "↑".cyan()
        },
        file.display(),
        current_version,
        target_version
    );

    // Perform transformations
    let yaml_map = yaml
        .as_mapping_mut()
        .ok_or_else(|| ForgeError::Parse("Root must be a YAML mapping".to_string()))?;

    // 1. Update _forge_version
    yaml_map.insert(
        serde_yaml_ng::Value::String("_forge_version".to_string()),
        serde_yaml_ng::Value::String(target_version.to_string()),
    );

    // 2. Split scalars into inputs/outputs if upgrading to 5.0.0
    if target_version == "5.0.0" {
        split_scalars_to_inputs_outputs(yaml_map, verbose)?;
    }

    if !dry_run {
        // Create backup
        let backup_path = file.with_extension("yaml.bak");
        fs::copy(file, &backup_path)
            .map_err(|e| ForgeError::IO(format!("Failed to create backup: {}", e)))?;
        if verbose {
            println!("      {} Backup: {}", "📋".dimmed(), backup_path.display());
        }

        // Write upgraded content
        let upgraded_content = serde_yaml_ng::to_string(&yaml)
            .map_err(|e| ForgeError::IO(format!("Failed to serialize YAML: {}", e)))?;

        // Preserve comments by writing a header
        let final_content = format!(
            "# Upgraded to Forge v{} by 'forge upgrade'\n{}",
            target_version, upgraded_content
        );

        fs::write(file, final_content)
            .map_err(|e| ForgeError::IO(format!("Failed to write {}: {}", file.display(), e)))?;
    }

    Ok(changes + 1)
}

/// Split scalars section into inputs and outputs based on formula presence
pub fn split_scalars_to_inputs_outputs(
    yaml_map: &mut serde_yaml_ng::Mapping,
    verbose: bool,
) -> ForgeResult<()> {
    // Check if there's a top-level scalars-like structure (not in a table)
    // In v4.x, scalars are scattered at root level or in sections
    // We need to identify them and split into inputs/outputs

    let mut inputs: serde_yaml_ng::Mapping = serde_yaml_ng::Mapping::new();
    let mut outputs: serde_yaml_ng::Mapping = serde_yaml_ng::Mapping::new();
    let mut keys_to_remove: Vec<serde_yaml_ng::Value> = Vec::new();

    // Preserve existing inputs/outputs if they exist
    if let Some(existing_inputs) = yaml_map.get(serde_yaml_ng::Value::String("inputs".to_string()))
    {
        if let Some(map) = existing_inputs.as_mapping() {
            inputs = map.clone();
        }
    }
    if let Some(existing_outputs) =
        yaml_map.get(serde_yaml_ng::Value::String("outputs".to_string()))
    {
        if let Some(map) = existing_outputs.as_mapping() {
            outputs = map.clone();
        }
    }

    // Look for scalar-like entries at root level
    // These are mappings with 'value' and optionally 'formula' keys
    for (key, value) in yaml_map.iter() {
        let key_str = key.as_str().unwrap_or("");

        // Skip special keys and existing sections
        if key_str.starts_with('_')
            || key_str == "inputs"
            || key_str == "outputs"
            || key_str == "scenarios"
        {
            continue;
        }

        // Check if this looks like a scalar (has 'value' key)
        if let Some(mapping) = value.as_mapping() {
            let value_key = serde_yaml_ng::Value::String("value".to_string());
            let formula_key = serde_yaml_ng::Value::String("formula".to_string());
            if mapping.contains_key(&value_key) {
                let has_formula = mapping.contains_key(&formula_key)
                    && mapping
                        .get(&formula_key)
                        .map(|f| !f.is_null() && f.as_str().map(|s| !s.is_empty()).unwrap_or(false))
                        .unwrap_or(false);

                if has_formula {
                    outputs.insert(key.clone(), value.clone());
                    if verbose {
                        println!(
                            "      {} {} → outputs (has formula)",
                            "📤".dimmed(),
                            key_str
                        );
                    }
                } else {
                    inputs.insert(key.clone(), value.clone());
                    if verbose {
                        println!("      {} {} → inputs (value only)", "📥".dimmed(), key_str);
                    }
                }
                keys_to_remove.push(key.clone());
            }
        }
    }

    // Remove moved keys
    for key in keys_to_remove {
        yaml_map.remove(&key);
    }

    // Add inputs and outputs sections if they have content
    if !inputs.is_empty() {
        yaml_map.insert(
            serde_yaml_ng::Value::String("inputs".to_string()),
            serde_yaml_ng::Value::Mapping(inputs),
        );
    }
    if !outputs.is_empty() {
        yaml_map.insert(
            serde_yaml_ng::Value::String("outputs".to_string()),
            serde_yaml_ng::Value::Mapping(outputs),
        );
    }

    Ok(())
}

#[cfg(test)]
mod auto_upgrade_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_needs_schema_upgrade_old_version() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("old.yaml");
        std::fs::write(
            &yaml_path,
            r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
"#,
        )
        .unwrap();

        let result = needs_schema_upgrade(&yaml_path).unwrap();
        assert_eq!(result, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_needs_schema_upgrade_v4_version() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("v4.yaml");
        std::fs::write(
            &yaml_path,
            r#"_forge_version: "4.0.0"
x:
  value: 10
  formula: null
"#,
        )
        .unwrap();

        let result = needs_schema_upgrade(&yaml_path).unwrap();
        assert_eq!(result, Some("4.0.0".to_string()));
    }

    #[test]
    fn test_needs_schema_upgrade_current_version() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("current.yaml");
        std::fs::write(
            &yaml_path,
            r#"_forge_version: "5.0.0"
x:
  value: 10
  formula: null
"#,
        )
        .unwrap();

        let result = needs_schema_upgrade(&yaml_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_needs_schema_upgrade_skips_multi_doc() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("multi.yaml");
        std::fs::write(
            &yaml_path,
            r#"---
_forge_version: "1.0.0"
x:
  value: 10
---
_forge_version: "1.0.0"
y:
  value: 20
"#,
        )
        .unwrap();

        // Multi-doc should return None (skip upgrade)
        let result = needs_schema_upgrade(&yaml_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_needs_schema_upgrade_no_version() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("noversion.yaml");
        std::fs::write(
            &yaml_path,
            r#"x:
  value: 10
  formula: null
"#,
        )
        .unwrap();

        // No version defaults to "1.0.0"
        let result = needs_schema_upgrade(&yaml_path).unwrap();
        assert_eq!(result, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_auto_upgrade_schema_upgrades_file() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("upgrade_me.yaml");
        std::fs::write(
            &yaml_path,
            r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
"#,
        )
        .unwrap();

        let result = auto_upgrade_schema(&yaml_path, false);
        assert!(result.is_ok());

        // Verify upgrade happened
        let content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(content.contains("5.0.0"));
    }
}
