//! Upgrade command - migrate YAML files to latest schema version

use crate::cli::console::outln;
use crate::cli::Console;
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
pub fn upgrade(
    c: &mut Console<'_>,
    file: &Path,
    dry_run: bool,
    target_version: &str,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Schema Upgrade".bold().green());
    outln!(c);
    outln!(c, "   File:    {}", file.display());
    outln!(c, "   Target:  v{target_version}");
    if dry_run {
        outln!(c, "   Mode:    {} (no files modified)", "DRY RUN".yellow());
    }
    outln!(c);

    // Track upgraded files to avoid circular processing
    let mut upgraded_files: HashSet<PathBuf> = HashSet::new();

    // Perform upgrade recursively
    let changes = upgrade_file_recursive(
        c,
        file,
        target_version,
        dry_run,
        verbose,
        &mut upgraded_files,
    )?;

    // Summary
    outln!(c);
    outln!(c, "{}", "═".repeat(70));
    outln!(c);
    if dry_run {
        outln!(
            c,
            "{} {} file(s) would be upgraded",
            "DRY RUN:".yellow().bold(),
            changes
        );
        outln!(c);
        outln!(c, "   Run without --dry-run to apply changes.");
    } else {
        outln!(
            c,
            "{} {} file(s) upgraded to v{}",
            "✅".green(),
            changes,
            target_version
        );
    }
    outln!(c);

    Ok(())
}

/// Upgrade each file the model includes (`_includes`) that exists, and
/// return how many were changed.
fn upgrade_includes(
    c: &mut Console<'_>,
    file: &Path,
    yaml: &yaml_serde::Value,
    target_version: &str,
    dry_run: bool,
    verbose: bool,
    upgraded_files: &mut HashSet<PathBuf>,
) -> ForgeResult<usize> {
    let mut changes = 0;
    if let Some(yaml_serde::Value::Sequence(include_list)) = yaml.get("_includes") {
        for include in include_list {
            if let Some(include_file) = include.get("file").and_then(|f| f.as_str()) {
                let include_path = file.with_file_name(include_file);
                if include_path.exists() {
                    changes += upgrade_file_recursive(
                        c,
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
    Ok(changes)
}

/// Recursively upgrade a file and its includes.
///
/// # Errors
///
/// Returns an error if any file cannot be read, parsed, or written during upgrade.
pub fn upgrade_file_recursive(
    c: &mut Console<'_>,
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
            outln!(
                c,
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

    // First, recursively upgrade any included files
    let changes = upgrade_includes(
        c,
        file,
        &yaml,
        target_version,
        dry_run,
        verbose,
        upgraded_files,
    )?;

    // Get current version
    let current_version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    // Check if upgrade needed
    if current_version == target_version {
        if verbose {
            outln!(
                c,
                "   {} {} (already v{})",
                "✓".green(),
                file.display(),
                target_version
            );
        }
        return Ok(changes);
    }

    outln!(
        c,
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
    // 5.2.0 is 5.1.0 plus the period column (ADR-053): the same conversion,
    // then the version line. A 5.1.0 file only changes that line. 5.3.0 is
    // 5.2.0 plus the empty cell (ADR-059), and 5.4.0 is 5.3.0 plus the date as
    // a scalar (ADR-060), the same again.
    if matches!(target_version, "5.1.0" | "5.2.0" | "5.3.0" | "5.4.0") {
        let versions = (current_version, target_version);
        upgrade_to_short_form(c, file, &content, &yaml, versions, dry_run, verbose)?;
        return Ok(changes + 1);
    }
    if target_version != "5.0.0" {
        return Err(ForgeError::Validation(format!(
            "Unknown target format '{target_version}'; use 5.1.0 (default), 5.2.0, 5.3.0, 5.4.0 or 5.0.0"
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

    // 2. Split scalars into inputs/outputs (the target is 5.0.0 here)
    split_scalars_to_inputs_outputs(c, yaml_map, verbose);

    if !dry_run {
        // Preserve comments by writing a header
        let final_content = format!(
            "# Upgraded to Forge v{target_version} by 'forge upgrade'\n{}",
            yaml_text(&yaml)
        );
        let backup_path = back_up_and_write(file, &final_content)?;
        if verbose {
            outln!(
                c,
                "      {} Backup: {}",
                "📋".dimmed(),
                backup_path.display()
            );
        }
    }

    Ok(changes + 1)
}

/// Convert one file to model format 5.1.0, in place (ADR-051).
///
/// A 5.0.0 file, or a 1.0.0 file that is valid 5.x as written, is converted
/// as text so its comments are kept. Other 1.0.0 files go through the 5.0.0
/// restructuring first, which cannot keep comments; that is said on screen.
fn upgrade_to_short_form(
    c: &mut Console<'_>,
    file: &Path,
    content: &str,
    yaml: &yaml_serde::Value,
    (current_version, target_version): (&str, &str),
    dry_run: bool,
    verbose: bool,
) -> ForgeResult<()> {
    let rank = |v: &str| {
        ["5.1.0", "5.2.0", "5.3.0", "5.4.0"]
            .iter()
            .position(|known| *known == v)
    };
    if let (Some(from), Some(to)) = (rank(current_version), rank(target_version)) {
        if from > to {
            return Err(ForgeError::Validation(format!(
                "{}: format {current_version} is not converted back to {target_version} (a period \
                 column, an empty cell, a date as a scalar or a format: has no form in the earlier format)",
                file.display()
            )));
        }
    }
    let converted = match crate::writer::in_place::to_short_form(content) {
        // Already in the short form: only the version line changes
        _ if matches!(current_version, "5.1.0" | "5.2.0") => Ok(content.to_string()),
        Ok(text) => Ok(text),
        Err(_) if current_version != "5.0.0" => {
            outln!(
                c,
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
            split_scalars_to_inputs_outputs(c, map, verbose);
            crate::writer::in_place::to_short_form(&yaml_text(&yaml50))
        },
        Err(e) => Err(e),
    }
    .map_err(|e| {
        ForgeError::Validation(format!(
            "{}: not converted, the file is unchanged: {e}",
            file.display()
        ))
    })?;
    let converted = crate::writer::in_place::set_format_version(&converted, target_version);
    if !dry_run {
        back_up_and_write(file, &converted)?;
    }
    Ok(())
}

/// A YAML value as text. A value read from a file always serializes.
fn yaml_text(value: &yaml_serde::Value) -> String {
    yaml_serde::to_string(value).expect("a YAML value serializes")
}

/// Copy `file` to `<file>.yaml.bak`, then replace its content with `text`.
/// Returns the backup's path. Nothing is written if the copy fails.
fn back_up_and_write(file: &Path, text: &str) -> ForgeResult<PathBuf> {
    let backup_path = file.with_extension("yaml.bak");
    fs::copy(file, &backup_path)
        .and_then(|_| fs::write(file, text))
        .map_err(|e| {
            ForgeError::IO(format!(
                "Failed to back up and write {}: {e}",
                file.display()
            ))
        })?;
    Ok(backup_path)
}

/// Split scalars section into inputs and outputs based on formula presence.
///
/// # Errors
///
/// Returns an error if the output cannot be written.
pub fn split_scalars_to_inputs_outputs(
    c: &mut Console<'_>,
    yaml_map: &mut yaml_serde::Mapping,
    verbose: bool,
) {
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
                        outln!(
                            c,
                            "      {} {} → outputs (has formula)",
                            "📤".dimmed(),
                            key_str
                        );
                    }
                } else {
                    inputs.insert(key.clone(), value.clone());
                    if verbose {
                        outln!(
                            c,
                            "      {} {} → inputs (value only)",
                            "📥".dimmed(),
                            key_str
                        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;

    /// Upgrade `file` to `target`, giving the result and what was printed
    fn run(file: &Path, dry_run: bool, target: &str, verbose: bool) -> (ForgeResult<()>, String) {
        let (result, captured) = capture(|c| upgrade(c, file, dry_run, target, verbose));
        (result, captured.out)
    }

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn message(result: ForgeResult<()>) -> String {
        result.unwrap_err().to_string()
    }

    /// A 1.0.0 file with scalars written out in full, a section of them
    /// already moved, and things the split leaves alone
    const SCALARS: &str = "\
price:
  value: 10
cost:
  value: 4
  formula: \"=price * 0.4\"
empty_formula:
  value: 1
  formula: \"\"
null_formula:
  value: 2
  formula: null
inputs:
  old: {value: 1}
outputs:
  older: {value: 2, formula: \"=old\"}
scenarios:
  base: {}
_note: kept
plain: 5
no_value:
  other: 1
";

    #[test]
    fn upgrading_to_5_0_0_splits_scalars_into_inputs_and_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", SCALARS);
        let (result, out) = run(&file, false, "5.0.0", true);
        result.unwrap();
        for line in [
            "price → inputs (value only)",
            "cost → outputs (has formula)",
            "empty_formula → inputs",
            "null_formula → inputs",
            "Backup:",
            "1 file(s) upgraded to v5.0.0",
        ] {
            assert!(out.contains(line), "{line} in {out}");
        }
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("# Upgraded to Forge v5.0.0 by 'forge upgrade'\n"));
        let yaml: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
        assert_eq!(yaml["_forge_version"].as_str(), Some("5.0.0"));
        for name in ["price", "empty_formula", "null_formula", "old"] {
            assert!(yaml["inputs"].get(name).is_some(), "{name} in inputs");
        }
        for name in ["cost", "older"] {
            assert!(yaml["outputs"].get(name).is_some(), "{name} in outputs");
        }
        assert!(yaml.get("price").is_none());
        assert_eq!(yaml["plain"].as_i64(), Some(5));
        assert!(yaml["no_value"].get("other").is_some());
        assert_eq!(
            fs::read_to_string(file.with_extension("yaml.bak")).unwrap(),
            SCALARS
        );
    }

    #[test]
    fn upgrading_to_5_0_0_without_verbose_or_scalars_prints_less() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "plain: 5\ninputs: 3\noutputs: 4\n");
        let (result, out) = run(&file, false, "5.0.0", false);
        result.unwrap();
        assert!(
            !out.contains("Backup:") && !out.contains("→ inputs"),
            "{out}"
        );
        let yaml: yaml_serde::Value =
            yaml_serde::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert!(yaml.get("inputs").is_some_and(yaml_serde::Value::is_number));
    }

    #[test]
    fn a_dry_run_changes_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", SCALARS);
        let (result, out) = run(&file, true, "5.0.0", false);
        result.unwrap();
        assert!(out.contains("DRY RUN"), "{out}");
        assert!(out.contains("1 file(s) would be upgraded"), "{out}");
        assert_eq!(fs::read_to_string(&file).unwrap(), SCALARS);
        assert!(!file.with_extension("yaml.bak").exists());
    }

    #[test]
    fn a_target_format_that_does_not_exist_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "a: 1\n");
        let (result, _) = run(&file, false, "9.9.9", false);
        assert!(message(result).contains("Unknown target format '9.9.9'"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "a: 1\n");
    }

    #[test]
    fn a_file_that_is_not_a_mapping_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "- a\n- b\n");
        for target in ["5.0.0", "5.1.0"] {
            let (result, _) = run(&file, false, target, false);
            assert!(
                message(result).contains("Root must be a YAML mapping"),
                "{target}"
            );
        }
        assert_eq!(fs::read_to_string(&file).unwrap(), "- a\n- b\n");
    }

    #[test]
    fn a_file_that_is_missing_or_not_yaml_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let (result, _) = run(&dir.path().join("absent.yaml"), false, "5.1.0", false);
        assert!(message(result).contains("Failed to read"));
        let file = write(dir.path(), "bad.yaml", "a: [1, 2\n");
        let (result, _) = run(&file, false, "5.1.0", false);
        assert!(message(result).contains("Failed to parse"));
    }

    #[test]
    fn a_file_already_at_the_target_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "_forge_version: \"5.1.0\"\na: 1\n");
        let (result, out) = run(&file, false, "5.1.0", true);
        result.unwrap();
        assert!(out.contains("(already v5.1.0)"), "{out}");
        assert!(out.contains("0 file(s) upgraded"), "{out}");
        let (result, out) = run(&file, false, "5.1.0", false);
        result.unwrap();
        assert!(!out.contains("already"), "{out}");
    }

    #[test]
    fn includes_are_upgraded_before_the_file_that_names_them() {
        let dir = tempfile::tempdir().unwrap();
        let main = write(
            dir.path(),
            "main.yaml",
            "_includes:\n  - file: part.yaml\n    as: part\n  - file: absent.yaml\n    as: gone\n  - as: nofile\nx: {value: 1}\n",
        );
        let part = write(
            dir.path(),
            "part.yaml",
            "_includes:\n  - file: main.yaml\n    as: loop\ny: {value: 2}\n",
        );
        let (result, out) = run(&main, false, "5.0.0", true);
        result.unwrap();
        assert!(out.contains("(already processed)"), "{out}");
        assert!(out.contains("2 file(s) upgraded to v5.0.0"), "{out}");
        for file in [&main, &part] {
            let text = fs::read_to_string(file).unwrap();
            assert!(text.contains("_forge_version: 5.0.0"), "{text}");
            assert!(file.with_extension("yaml.bak").exists());
        }
        // Without verbose the loop is skipped silently
        let (result, out) = run(&main, true, "5.0.0", false);
        result.unwrap();
        assert!(!out.contains("already processed"), "{out}");
    }

    #[test]
    fn an_includes_entry_that_is_not_a_list_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "_includes: none\nz: {value: 1}\n");
        let (result, out) = run(&file, true, "5.0.0", false);
        result.unwrap();
        assert!(out.contains("1 file(s) would be upgraded"), "{out}");
    }

    #[test]
    fn a_later_format_is_not_converted_back() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "m.yaml", "_forge_version: \"5.2.0\"\na: 1\n");
        let (result, _) = run(&file, false, "5.1.0", false);
        assert!(message(result).contains("is not converted back to 5.1.0"));
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "_forge_version: \"5.2.0\"\na: 1\n"
        );
    }

    #[test]
    fn a_5_1_file_only_changes_its_version_line() {
        let dir = tempfile::tempdir().unwrap();
        let text = "# kept\n_forge_version: \"5.1.0\"\nprice: 10 # also kept\n";
        let file = write(dir.path(), "m.yaml", text);
        let (result, _) = run(&file, false, "5.4.0", false);
        result.unwrap();
        let upgraded = fs::read_to_string(&file).unwrap();
        assert_eq!(upgraded, text.replace("\"5.1.0\"", "\"5.4.0\""));
    }

    #[test]
    fn a_5_0_file_goes_to_the_short_form_keeping_comments() {
        let dir = tempfile::tempdir().unwrap();
        let text = "# kept\n_forge_version: \"5.0.0\"\ninputs:\n  price:\n    value: 10\n    formula: null\n";
        let file = write(dir.path(), "m.yaml", text);
        let (result, _) = run(&file, false, "5.1.0", false);
        result.unwrap();
        let upgraded = fs::read_to_string(&file).unwrap();
        assert!(
            upgraded.starts_with("# kept\n_forge_version: \"5.1.0\"\n"),
            "{upgraded}"
        );
        assert!(!upgraded.contains("formula: null"), "{upgraded}");
        assert_eq!(
            fs::read_to_string(file.with_extension("yaml.bak")).unwrap(),
            text
        );
    }

    #[test]
    fn a_backup_that_cannot_be_made_stops_the_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let old =
            "_forge_version: \"5.0.0\"\ninputs:\n  price:\n    value: 10\n    formula: null\n";
        for (target, text) in [("5.0.0", SCALARS), ("5.1.0", old)] {
            let file = write(dir.path(), "m.yaml", text);
            let bak = file.with_extension("yaml.bak");
            fs::create_dir(&bak).unwrap();
            let (result, _) = run(&file, false, target, false);
            assert!(
                message(result).contains("Failed to back up and write"),
                "{target}"
            );
            assert_eq!(fs::read_to_string(&file).unwrap(), text);
            fs::remove_dir(&bak).unwrap();
        }
    }

    #[test]
    fn a_1_0_file_with_scalars_is_restructured_and_the_loss_of_comments_is_said() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(
            dir.path(),
            "m.yaml",
            "# lost\nprice:\n  value: 10\ncost:\n  value: 4\n  formula: \"=price * 0.4\"\n",
        );
        let (result, out) = run(&file, false, "5.1.0", true);
        result.unwrap();
        assert!(out.contains("comments in"), "{out}");
        assert!(out.contains("cannot be kept"), "{out}");
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("_forge_version: \"5.1.0\""), "{text}");
        assert!(text.contains("price: 10"), "{text}");
        assert!(!text.contains("# lost"), "{text}");
    }

    #[test]
    fn a_file_that_does_not_convert_is_left_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in [
            ("a.yaml", "_forge_version: \"5.0.0\"\ny: 2\n"),
            ("b.yaml", SCALARS),
        ] {
            let file = write(dir.path(), name, text);
            let (result, _) = run(&file, false, "5.1.0", false);
            let message = message(result);
            assert!(
                message.contains("not converted, the file is unchanged"),
                "{message}"
            );
            assert_eq!(fs::read_to_string(&file).unwrap(), text);
            assert!(!file.with_extension("yaml.bak").exists());
        }
    }

    #[test]
    fn an_include_that_fails_stops_the_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let main = write(
            dir.path(),
            "main.yaml",
            "_includes:\n  - file: bad.yaml\n    as: b\n",
        );
        write(dir.path(), "bad.yaml", "a: [1, 2\n");
        let (result, _) = run(&main, false, "5.0.0", false);
        assert!(message(result).contains("Failed to parse"));
        assert!(!main.with_extension("yaml.bak").exists());
    }

    #[test]
    fn a_dry_run_of_the_short_form_changes_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let text =
            "_forge_version: \"5.0.0\"\ninputs:\n  price:\n    value: 10\n    formula: null\n";
        let file = write(dir.path(), "m.yaml", text);
        let (result, out) = run(&file, true, "5.1.0", false);
        result.unwrap();
        assert!(out.contains("1 file(s) would be upgraded"), "{out}");
        assert_eq!(fs::read_to_string(&file).unwrap(), text);
        assert!(!file.with_extension("yaml.bak").exists());
    }
}
