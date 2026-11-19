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
/// Shared by the MCP server and the REST API. With a `root`, a path outside
/// it, or a model that includes a file outside it, is refused.
///
/// Returns `(PathBuf, Option<TempDir>)` — the `TempDir` guard must be kept
/// alive in the caller so the temp files are not cleaned up prematurely.
pub(crate) fn resolve_model_input(
    root: Option<&crate::path_root::Root>,
    file_path: Option<&str>,
    content: Option<&str>,
    includes: Option<&HashMap<String, String>>,
) -> Result<(PathBuf, Option<TempDir>), String> {
    match (file_path, content) {
        (Some(_), Some(_)) => Err("Provide either file_path or content, not both".into()),
        (None, None) => Err("Either file_path or content is required".into()),
        (Some(path), None) => match root {
            // `forge mcp --root`: the path and the files it includes
            Some(root) => Ok((root.model(path)?, None)),
            None => Ok((PathBuf::from(path), None)),
        },
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

/// Write inline include files into the temp directory.
///
/// Parses `_includes` entries from the YAML, matches each `as` namespace
/// to the `includes` map, and writes the content using the `file` field
/// as the filename (creating subdirectories if needed). An included file can
/// include others (`bagel_simulate.yaml` includes `costs_risk.yaml`, which
/// includes `costs.yaml`): they are written the same way, their `file`
/// relative to the file that names them, their content in the same map.
/// Every path must stay inside the temp directory.
fn resolve_inline_includes(
    yaml: &str,
    includes: Option<&HashMap<String, String>>,
    dir: &Path,
) -> Result<(), String> {
    let parsed: yaml_serde::Value =
        yaml_serde::from_str(yaml).map_err(|e| format!("Invalid YAML: {e}"))?;
    if parsed.get("_includes").is_none() {
        return Ok(());
    }
    let map = includes.ok_or(
        "Model has _includes but no 'includes' map was provided. \
         Pass inline content for each included namespace.",
    )?;
    write_includes(&parsed, map, dir, Path::new(""), &mut HashMap::new())
}

/// Write the files `model` includes, and the files those include in turn.
///
/// `from` is the folder of the file `model` is the text of, inside `dir`;
/// `written` holds the path each namespace has been written to, so that a
/// namespace is one file and a file that includes itself is written once.
fn write_includes(
    model: &yaml_serde::Value,
    map: &HashMap<String, String>,
    dir: &Path,
    from: &Path,
    written: &mut HashMap<String, PathBuf>,
) -> Result<(), String> {
    let entries = match model.get("_includes") {
        Some(yaml_serde::Value::Sequence(seq)) => seq,
        Some(_) => return Err("_includes must be a sequence".into()),
        None => return Ok(()),
    };

    for entry in entries {
        let file = entry
            .get("file")
            .and_then(yaml_serde::Value::as_str)
            .ok_or("Each _includes entry must have a 'file' field")?;

        let namespace = entry
            .get("as")
            .and_then(yaml_serde::Value::as_str)
            .ok_or_else(|| format!("Include '{file}' must have an 'as' field"))?;

        check_inline_include_path(file)?;
        let relative: PathBuf = from
            .join(file)
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect();

        let include_content = map.get(namespace).ok_or_else(|| {
            format!(
                "No content provided for include namespace '{namespace}'. \
                 Add it to the 'includes' map."
            )
        })?;

        match written.get(namespace) {
            Some(earlier) if *earlier == relative => continue,
            Some(earlier) => {
                return Err(format!(
                    "Include namespace '{namespace}' names two files, '{}' and '{}'. \
                     Each namespace in the 'includes' map is the content of one file.",
                    earlier.display(),
                    relative.display()
                ))
            },
            None => {},
        }
        written.insert(namespace.to_string(), relative.clone());

        let dest = dir.join(&relative);
        std::fs::create_dir_all(dest.parent().unwrap_or(dir))
            .map_err(|e| format!("Failed to create dir for include '{file}': {e}"))?;
        std::fs::write(&dest, include_content)
            .map_err(|e| format!("Failed to write include '{file}': {e}"))?;

        // Content that does not parse is left for the parser to report
        if let Ok(inner) = yaml_serde::from_str::<yaml_serde::Value>(include_content) {
            let folder = relative.parent().unwrap_or_else(|| Path::new(""));
            write_includes(&inner, map, dir, folder, written)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_model_input_file_path() {
        let (path, tmpdir) =
            resolve_model_input(None, Some("test-data/budget.yaml"), None, None).unwrap();
        assert_eq!(path, PathBuf::from("test-data/budget.yaml"));
        assert!(tmpdir.is_none());
    }

    #[test]
    fn test_resolve_model_input_content() {
        let (path, tmpdir) = resolve_model_input(
            None,
            None,
            Some("_forge_version: \"5.1.0\"\na:\n  x: 1\n"),
            None,
        )
        .unwrap();
        assert!(tmpdir.is_some());
        assert!(path.exists());
        assert!(path.ends_with("model.yaml"));
    }

    #[test]
    fn test_resolve_model_input_both_error() {
        let result = resolve_model_input(None, Some("file.yaml"), Some("content"), None);
        assert_eq!(
            result.unwrap_err(),
            "Provide either file_path or content, not both"
        );
    }

    #[test]
    fn test_resolve_model_input_neither_error() {
        let result = resolve_model_input(None, None, None, None);
        assert_eq!(
            result.unwrap_err(),
            "Either file_path or content is required"
        );
    }

    /// `includes` as text for each namespace
    fn map(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(namespace, text)| (namespace.to_string(), text.to_string()))
            .collect()
    }

    const BASE: &str = "_includes:\n  - file: a.yaml\n    as: a\n";

    #[test]
    fn a_file_an_included_file_includes_is_written_beside_it() {
        // examples/bagel/bagel_simulate.yaml includes costs_risk.yaml, which
        // includes costs.yaml: the agent names both, by their `as`
        let includes = map(&[
            ("a", "_includes:\n  - file: b.yaml\n    as: b\nx: 1\n"),
            ("b", "y: 2\n"),
        ]);
        let (path, tmp) = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap();
        let dir = path.parent().unwrap();
        assert!(tmp.is_some());
        assert_eq!(
            std::fs::read_to_string(dir.join("b.yaml")).unwrap(),
            "y: 2\n"
        );
        assert!(std::fs::read_to_string(dir.join("a.yaml"))
            .unwrap()
            .contains("x: 1"));
    }

    #[test]
    fn a_nested_include_is_relative_to_the_file_that_includes_it() {
        let top = "_includes:\n  - file: ./data/a.yaml\n    as: a\n";
        let includes = map(&[
            ("a", "_includes:\n  - file: ./b.yaml\n    as: b\n"),
            ("b", "y: 2\n"),
        ]);
        let (path, _tmp) = resolve_model_input(None, None, Some(top), Some(&includes)).unwrap();
        let dir = path.parent().unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("data/b.yaml")).unwrap(),
            "y: 2\n"
        );
        assert!(!dir.join("b.yaml").exists());
    }

    #[test]
    fn a_nested_include_without_content_is_named() {
        let includes = map(&[("a", "_includes:\n  - file: b.yaml\n    as: b\n")]);
        let err = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap_err();
        assert!(err.contains("namespace 'b'"), "{err}");
    }

    #[test]
    fn a_nested_include_cannot_leave_the_folder() {
        let includes = map(&[
            ("a", "_includes:\n  - file: ../b.yaml\n    as: b\n"),
            ("b", "y: 2\n"),
        ]);
        let err = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap_err();
        assert!(err.contains("not allowed"), "{err}");
    }

    #[test]
    fn one_namespace_cannot_be_two_files() {
        let includes = map(&[("a", "_includes:\n  - file: other.yaml\n    as: a\n")]);
        let err = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap_err();
        assert!(
            err.contains("namespace 'a'") && err.contains("a.yaml") && err.contains("other.yaml"),
            "{err}"
        );
    }

    #[test]
    fn a_file_included_twice_is_written_once() {
        // The same file by two routes (and a file that includes itself)
        let top = "_includes:\n  - file: a.yaml\n    as: a\n  - file: c.yaml\n    as: c\n";
        let includes = map(&[
            (
                "a",
                "_includes:\n  - file: ./c.yaml\n    as: c\n  - file: a.yaml\n    as: a\n",
            ),
            ("c", "z: 3\n"),
        ]);
        let (path, _tmp) = resolve_model_input(None, None, Some(top), Some(&includes)).unwrap();
        let dir = path.parent().unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("c.yaml")).unwrap(),
            "z: 3\n"
        );
    }

    #[test]
    fn an_included_file_that_is_not_yaml_is_left_to_the_parser() {
        let includes = map(&[("a", "key: [unclosed\n")]);
        let (path, _tmp) = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap();
        assert!(path.parent().unwrap().join("a.yaml").exists());
    }

    #[test]
    fn a_nested_includes_section_that_is_not_a_list_is_refused() {
        let includes = map(&[("a", "_includes: nope\n")]);
        let err = resolve_model_input(None, None, Some(BASE), Some(&includes)).unwrap_err();
        assert_eq!(err, "_includes must be a sequence");
    }

    #[test]
    fn an_include_without_a_file_or_a_name_is_refused() {
        let no_file = "_includes:\n  - as: a\n";
        let err = resolve_model_input(None, None, Some(no_file), Some(&map(&[]))).unwrap_err();
        assert_eq!(err, "Each _includes entry must have a 'file' field");
        let no_name = "_includes:\n  - file: a.yaml\n";
        let err = resolve_model_input(None, None, Some(no_name), Some(&map(&[]))).unwrap_err();
        assert_eq!(err, "Include 'a.yaml' must have an 'as' field");
    }

    #[test]
    fn an_include_that_cannot_be_written_says_which() {
        // `x` is a file where the second include wants a folder
        let top = "_includes:\n  - file: x\n    as: a\n  - file: x/y.yaml\n    as: b\n";
        let includes = map(&[("a", "k: 1\n"), ("b", "y: 2\n")]);
        let err = resolve_model_input(None, None, Some(top), Some(&includes)).unwrap_err();
        assert!(
            err.contains("Failed to create dir for include 'x/y.yaml'"),
            "{err}"
        );

        // `d` is a folder where the second include wants a file
        let top = "_includes:\n  - file: d/f.yaml\n    as: a\n  - file: d\n    as: b\n";
        let includes = map(&[("a", "k: 1\n"), ("b", "y: 2\n")]);
        let err = resolve_model_input(None, None, Some(top), Some(&includes)).unwrap_err();
        assert!(err.contains("Failed to write include 'd'"), "{err}");
    }
}
