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
        None => Ok(std::fs::read_to_string(path)?),
        Some(Some(text)) => Ok(text),
        Some(None) => Err(ForgeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not among the files given", path.display()),
        ))),
    }
}

/// Run `f` with the files in memory, if a call of [`with_files_in_memory`]
/// is running.
pub(super) fn with_current<R>(f: impl FnOnce(Option<&HashMap<String, String>>) -> R) -> R {
    FILES.with(|cell| f(cell.borrow().as_ref()))
}
