//! Inline model content for the MCP server and the REST API.
//!
//! A client can send a model as text instead of naming a file on the machine
//! that runs Forge. The text is written to a temporary directory, with any
//! included files, and the normal file-based commands run on that.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use tempfile::TempDir;

/// Resolve model input from either a file path or inline YAML content.
///
/// Shared by the MCP server and the REST API.
///
/// Returns `(PathBuf, Option<TempDir>)` — the `TempDir` guard must be kept
/// alive in the caller so the temp files are not cleaned up prematurely.
pub(crate) fn resolve_model_input(
    file_path: Option<&str>,
    content: Option<&str>,
    includes: Option<&HashMap<String, String>>,
) -> Result<(PathBuf, Option<TempDir>), String> {
    match (file_path, content) {
        (Some(_), Some(_)) => Err("Provide either file_path or content, not both".into()),
        (None, None) => Err("Either file_path or content is required".into()),
        (Some(path), None) => Ok((PathBuf::from(path), None)),
        (None, Some(yaml)) => {
            let tmp = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
            resolve_inline_includes(yaml, includes, tmp.path())?;
            std::fs::write(tmp.path().join("model.yaml"), yaml)
                .map_err(|e| format!("Failed to write temp model: {e}"))?;
            Ok((tmp.path().join("model.yaml"), Some(tmp)))
        },
    }
}

/// Reject include paths that could leave the temp directory.
///
/// Inline content is untrusted: it usually comes from whatever an AI agent
/// has read. Only plain relative paths are allowed, so `file` cannot name an
/// absolute path, a drive or a `..` component.
fn check_inline_include_path(file: &str) -> Result<(), String> {
    let path = Path::new(file);
    let is_plain_relative = !file.is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if is_plain_relative {
        Ok(())
    } else {
        Err(format!(
            "Include path '{file}' is not allowed with inline content. \
             Use a relative path without '..'."
        ))
    }
}

/// Check the paths of any `_includes` nested inside inline include content.
///
/// Nested includes are not written to the temp directory, but the parser
/// would still try to read them, so an unchecked path could read files
/// outside it. Content that does not parse is left for the parser to report.
fn check_nested_include_paths(include_content: &str) -> Result<(), String> {
    let Ok(parsed) = yaml_serde::from_str::<yaml_serde::Value>(include_content) else {
        return Ok(());
    };
    if let Some(yaml_serde::Value::Sequence(seq)) = parsed.get("_includes") {
        for file in seq
            .iter()
            .filter_map(|entry| entry.get("file"))
            .filter_map(yaml_serde::Value::as_str)
        {
            check_inline_include_path(file)?;
        }
    }
    Ok(())
}

/// Write inline include files into the temp directory.
///
/// Parses `_includes` entries from the YAML, matches each `as` namespace
/// to the `includes` map, and writes the content using the `file` field
/// as the filename (creating subdirectories if needed). Every path must stay
/// inside the temp directory.
fn resolve_inline_includes(
    yaml: &str,
    includes: Option<&HashMap<String, String>>,
    dir: &Path,
) -> Result<(), String> {
    // Quick check: does the YAML contain _includes?
    let parsed: yaml_serde::Value =
        yaml_serde::from_str(yaml).map_err(|e| format!("Invalid YAML: {e}"))?;

    let includes_seq = match parsed.get("_includes") {
        Some(yaml_serde::Value::Sequence(seq)) => seq,
        Some(_) => return Err("_includes must be a sequence".into()),
        None => return Ok(()), // No _includes — nothing to resolve
    };

    let map = includes.ok_or(
        "Model has _includes but no 'includes' map was provided. \
         Pass inline content for each included namespace.",
    )?;

    for entry in includes_seq {
        let file = entry
            .get("file")
            .and_then(yaml_serde::Value::as_str)
            .ok_or("Each _includes entry must have a 'file' field")?;

        let namespace = entry
            .get("as")
            .and_then(yaml_serde::Value::as_str)
            .ok_or_else(|| format!("Include '{file}' must have an 'as' field"))?;

        let include_content = map.get(namespace).ok_or_else(|| {
            format!(
                "No content provided for include namespace '{namespace}'. \
                 Add it to the 'includes' map."
            )
        })?;

        check_inline_include_path(file)?;
        check_nested_include_paths(include_content)?;

        let dest = dir.join(file);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir for include '{file}': {e}"))?;
        }
        std::fs::write(&dest, include_content)
            .map_err(|e| format!("Failed to write include '{file}': {e}"))?;
    }

    Ok(())
}
