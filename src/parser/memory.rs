//! Files held in memory for the length of a call (ADR-055).
//!
//! In a browser there is no file system. [`with_files_in_memory`] runs a
//! function in which [`parse_model`](super::parse_model) and
//! [`read_model_text`] read the given files instead of the disk, so every
//! command's core function (`calculate_core`, `simulate_core`, ...) works
//! unchanged on a model held in memory. Outside such a call nothing changes:
//! files are read from disk.

use crate::error::{ForgeError, ForgeResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use super::includes::normalize;

thread_local! {
    static FILES: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// Run `f` with model files read from `files` instead of the disk.
///
/// The keys are paths relative to where the commands are pointed
/// (`model.yaml`, `costs.yaml`, `data/costs.yaml`), as `_includes` writes
/// them. Calls nest: the outer files come back when `f` returns.
pub fn with_files_in_memory<R, S: std::hash::BuildHasher>(
    files: HashMap<String, String, S>,
    f: impl FnOnce() -> R,
) -> R {
    let files = files
        .into_iter()
        .map(|(path, text)| (normalize(Path::new(&path)), text))
        .collect();
    let previous = FILES.with(|cell| cell.replace(Some(files)));
    let result = f();
    FILES.with(|cell| cell.replace(previous));
    result
}

/// Whether a call of [`with_files_in_memory`] is running on this thread.
#[must_use]
pub fn files_in_memory() -> bool {
    FILES.with(|cell| cell.borrow().is_some())
}

/// The text of a model file: from memory inside [`with_files_in_memory`],
/// from disk otherwise.
///
/// # Errors
///
/// Returns an I/O error if the file cannot be read, or is not among the
/// files in memory.
pub fn read_model_text(path: &Path) -> ForgeResult<String> {
    let in_memory = FILES.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|files| files.get(&normalize(path)).cloned())
    });
    match in_memory {
        None => read_from_disk(path),
        Some(Some(text)) => Ok(text),
        Some(None) => Err(ForgeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not among the files given", path.display()),
        ))),
    }
}

/// The text of a file on disk; the error names the file, as the system's
/// message ("No such file or directory") does not.
///
/// # Errors
///
/// Returns an I/O error, of the kind the system gave, that names `path`.
pub fn read_from_disk(path: &Path) -> ForgeResult<String> {
    std::fs::read_to_string(path).map_err(|e| {
        ForgeError::Io(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", path.display()),
        ))
    })
}

/// Run `f` with the files in memory, if a call of [`with_files_in_memory`]
/// is running.
pub(super) fn with_current<R>(f: impl FnOnce(Option<&HashMap<String, String>>) -> R) -> R {
    FILES.with(|cell| f(cell.borrow().as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string()))
            .collect()
    }

    /// Inside a call the files are the memory's, found by their normalized
    /// path, and a file that is not among them is `NotFound`, not a disk read.
    #[test]
    fn files_are_read_from_memory_and_a_missing_one_is_not_found() {
        assert!(!files_in_memory());
        let (inside, text, dotted, missing) = with_files_in_memory(
            files(&[("data/costs.yaml", "a: 1"), ("model.yaml", "b: 2")]),
            || {
                (
                    files_in_memory(),
                    read_model_text(Path::new("model.yaml")).unwrap(),
                    read_model_text(Path::new("./data/../data/costs.yaml")).unwrap(),
                    read_model_text(Path::new("other.yaml")),
                )
            },
        );
        assert!(inside);
        assert_eq!(text, "b: 2");
        assert_eq!(dotted, "a: 1");
        let error = missing.unwrap_err();
        assert_eq!(
            error.to_string(),
            "IO error: other.yaml is not among the files given"
        );
        assert!(!files_in_memory());
    }

    /// Outside a call the file is read from the disk; calls nest, and the
    /// outer files come back when the inner call returns.
    #[test]
    fn outside_a_call_the_disk_is_read_and_calls_nest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, "disk").unwrap();
        assert_eq!(read_model_text(&path).unwrap(), "disk");
        assert!(read_model_text(&dir.path().join("absent.yaml")).is_err());

        let (inner, outer_again) = with_files_in_memory(files(&[("m.yaml", "outer")]), || {
            let inner = with_files_in_memory(files(&[("m.yaml", "inner")]), || {
                read_model_text(Path::new("m.yaml")).unwrap()
            });
            (inner, read_model_text(Path::new("m.yaml")).unwrap())
        });
        assert_eq!(inner, "inner");
        assert_eq!(outer_again, "outer");
    }

    #[test]
    fn with_current_hands_over_the_files_or_nothing() {
        assert_eq!(with_current(|files| files.map(HashMap::len)), None);
        let count = with_files_in_memory(files(&[("a.yaml", ""), ("b.yaml", "")]), || {
            with_current(|files| files.map(HashMap::len))
        });
        assert_eq!(count, Some(2));
    }
}
