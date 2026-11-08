//! Upgrade command - migrate YAML files to latest schema version

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Execute the upgrade command - migrate YAML files to latest schema version.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or written,
/// or if any included files fail during recursive upgrade.
pub fn upgrade(file: &Path, dry_run: bool, target_version: &str, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Schema Upgrade".bold().green());
    println!();
    println!("   File:    {}", file.display());
    println!("   Target:  v{target_version}");
    if dry_run {
        println!("   Mode:    {} (no files modified)", "DRY RUN".yellow());
    }
    println!();

    // Track upgraded files to avoid circular processing
    let mut upgraded_files: HashSet<PathBuf> = HashSet::new();

    // Perform upgrade recursively
    let changes =
        upgrade_file_recursive(file, target_version, dry_run, verbose, &mut upgraded_files)?;

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

/// Recursively upgrade a file and its includes.
///
/// # Errors
///
/// Returns an error if any file cannot be read, parsed, or written during upgrade.
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
    upgraded_files.insert(canonical);

    // Read and parse the file
    let content = fs::read_to_string(file)
        .map_err(|e| ForgeError::IO(format!("Failed to read {}: {}", file.display(), e)))?;

    // Parse as YAML Value to manipulate
    let mut yaml: yaml_serde::Value = yaml_serde::from_str(&content)
        .map_err(|e| ForgeError::Parse(format!("Failed to parse {}: {}", file.display(), e)))?;

    let mut changes = 0;

    // First, recursively upgrade any included files
    if let Some(yaml_serde::Value::Sequence(include_list)) = yaml.get("_includes").cloned() {
        let parent_dir = file.parent().unwrap_or_else(|| Path::new("."));
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

    // 5.0.0 -> 5.1.0: the short form, edited in place (ADR-051), comments kept
    if target_version == "5.1.0" {
        upgrade_to_short_form(file, &content, &yaml, current_version, dry_run, verbose)?;
        return Ok(changes + 1);
    }
    if target_version != "5.0.0" {
        return Err(ForgeError::Validation(format!(
            "Unknown target format '{target_version}'; use 5.1.0 (default) or 5.0.0"
        )));
    }

    // Perform transformations
    let yaml_map = yaml
        .as_mapping_mut()
        .ok_or_else(|| ForgeError::Parse("Root must be a YAML mapping".to_string()))?;

    // 1. Update _forge_version
    yaml_map.insert(
        yaml_serde::Value::String("_forge_version".to_string()),
        yaml_serde::Value::String(target_version.to_string()),
    );

    // 2. Split scalars into inputs/outputs if upgrading to 5.0.0
    if target_version == "5.0.0" {
        split_scalars_to_inputs_outputs(yaml_map, verbose);
    }

    if !dry_run {
        // Create backup
        let backup_path = file.with_extension("yaml.bak");
        fs::copy(file, &backup_path)
            .map_err(|e| ForgeError::IO(format!("Failed to create backup: {e}")))?;
        if verbose {
            println!("      {} Backup: {}", "📋".dimmed(), backup_path.display());
        }

        // Write upgraded content
        let upgraded_content = yaml_serde::to_string(&yaml)
            .map_err(|e| ForgeError::IO(format!("Failed to serialize YAML: {e}")))?;

        // Preserve comments by writing a header
        let final_content =
            format!("# Upgraded to Forge v{target_version} by 'forge upgrade'\n{upgraded_content}");

        fs::write(file, final_content)
            .map_err(|e| ForgeError::IO(format!("Failed to write {}: {}", file.display(), e)))?;
    }

    Ok(changes + 1)
}

/// Convert one file to model format 5.1.0, in place (ADR-051).
///
/// A 5.0.0 file, or a 1.0.0 file that is valid 5.x as written, is converted
/// as text so its comments are kept. Other 1.0.0 files go through the 5.0.0
/// restructuring first, which cannot keep comments; that is said on screen.
fn upgrade_to_short_form(
    file: &Path,
    content: &str,
    yaml: &yaml_serde::Value,
    current_version: &str,
    dry_run: bool,
    verbose: bool,
) -> ForgeResult<()> {
    let converted = match crate::writer::in_place::to_short_form(content) {
        Ok(text) => Ok(text),
        Err(_) if current_version != "5.0.0" => {
            println!(
                "      {} comments in {} cannot be kept: it needs the 1.0.0 to 5.0.0 restructuring",
                "⚠️".yellow(),
                file.display()
            );
            let mut yaml50 = yaml.clone();
            let map = yaml50
                .as_mapping_mut()
                .ok_or_else(|| ForgeError::Parse("Root must be a YAML mapping".to_string()))?;
            map.insert(
                yaml_serde::Value::String("_forge_version".to_string()),
                yaml_serde::Value::String("5.0.0".to_string()),
            );
            split_scalars_to_inputs_outputs(map, verbose);
            let text = yaml_serde::to_string(&yaml50)
                .map_err(|e| ForgeError::IO(format!("Failed to serialize YAML: {e}")))?;
            crate::writer::in_place::to_short_form(&text)
        },
        Err(e) => Err(e),
    }
    .map_err(|e| {
        ForgeError::Validation(format!(
            "{}: not converted, the file is unchanged: {e}",
            file.display()
        ))
    })?;
    if !dry_run {
        let backup_path = file.with_extension("yaml.bak");
        fs::copy(file, &backup_path)
            .map_err(|e| ForgeError::IO(format!("Failed to create backup: {e}")))?;
        fs::write(file, converted)
            .map_err(|e| ForgeError::IO(format!("Failed to write {}: {}", file.display(), e)))?;
    }
    Ok(())
}

/// Split scalars section into inputs and outputs based on formula presence.
pub fn split_scalars_to_inputs_outputs(yaml_map: &mut yaml_serde::Mapping, verbose: bool) {
    // Check if there's a top-level scalars-like structure (not in a table)
    // In v4.x, scalars are scattered at root level or in sections
    // We need to identify them and split into inputs/outputs

    let mut inputs: yaml_serde::Mapping = yaml_serde::Mapping::new();
    let mut outputs: yaml_serde::Mapping = yaml_serde::Mapping::new();
    let mut keys_to_remove: Vec<yaml_serde::Value> = Vec::new();

    // Preserve existing inputs/outputs if they exist
    if let Some(existing_inputs) = yaml_map.get(yaml_serde::Value::String("inputs".to_string())) {
        if let Some(map) = existing_inputs.as_mapping() {
            inputs = map.clone();
        }
    }
    if let Some(existing_outputs) = yaml_map.get(yaml_serde::Value::String("outputs".to_string())) {
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
            let value_key = yaml_serde::Value::String("value".to_string());
            let formula_key = yaml_serde::Value::String("formula".to_string());
            if mapping.contains_key(&value_key) {
                let has_formula = mapping.contains_key(&formula_key)
                    && mapping
                        .get(&formula_key)
                        .is_some_and(|f| !f.is_null() && f.as_str().is_some_and(|s| !s.is_empty()));

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
            yaml_serde::Value::String("inputs".to_string()),
            yaml_serde::Value::Mapping(inputs),
        );
    }
    if !outputs.is_empty() {
        yaml_map.insert(
            yaml_serde::Value::String("outputs".to_string()),
            yaml_serde::Value::Mapping(outputs),
        );
    }
}
