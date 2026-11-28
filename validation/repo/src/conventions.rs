//! The conventions in `.asimov/project.yaml` that a test can check
//! (roadmap T1-PROJECT-TRUTH: each convention is enforced or removed).

use crate::{root, rust_files};
use std::path::PathBuf;

/// Every Rust file of every crate: the main crate, forge-wasm and the
/// validation suites
///
/// # Panics
///
/// Panics if a file cannot be read.
#[must_use]
pub fn sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    for dir in ["src", "forge-wasm", "validation"] {
        rust_files(&root().join(dir), &mut files);
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::without_comment;
    use std::path::Path;

    /// The file `path` as a complaint when its text is over 1,500 lines
    fn too_long(path: &Path, text: &str) -> Option<String> {
        let lines = text.lines().count();
        (lines > 1500).then(|| format!("{}: {lines} lines", path.display()))
    }

    /// The file `path` as a complaint when its code (comments left out)
    /// unwraps a `partial_cmp`
    fn unwraps_partial_cmp(path: &Path, text: &str) -> Option<String> {
        let compact: String = text
            .lines()
            .map(without_comment)
            .flat_map(str::split_whitespace)
            .collect();
        let uses = compact
            .match_indices("partial_cmp(")
            .filter(|(i, _)| {
                compact[*i..]
                    .find(')')
                    .is_some_and(|end| compact[*i + end..].starts_with(").unwrap()"))
            })
            .count();
        (uses > 0).then(|| format!("{}: {uses}", path.display()))
    }

    /// "No file > 1,500 lines", tests included: a longer file is split into
    /// modules by what it does, never emptied of its tests
    #[test]
    fn no_source_file_is_longer_than_1500_lines() {
        let long: Vec<String> = sources()
            .into_iter()
            .filter_map(|(path, text)| too_long(&path, &text))
            .collect();
        assert!(long.is_empty(), "split these files: {long:#?}");
    }

    #[test]
    fn a_file_is_too_long_from_1501_lines() {
        let path = Path::new("a.rs");
        assert_eq!(too_long(path, &"x\n".repeat(1500)), None);
        assert_eq!(
            too_long(path, &"x\n".repeat(1501)),
            Some("a.rs: 1501 lines".to_string())
        );
    }

    /// "Sorting and comparing f64 uses `total_cmp`, never `partial_cmp().unwrap()`":
    /// a NaN would panic.
    #[test]
    fn no_partial_cmp_unwrap() {
        let found: Vec<String> = sources()
            .into_iter()
            .filter_map(|(path, text)| unwraps_partial_cmp(&path, &text))
            .collect();
        assert!(found.is_empty(), "use total_cmp: {found:#?}");
    }

    #[test]
    fn a_partial_cmp_unwrap_is_found_in_code_and_not_in_a_comment() {
        // Written in two pieces: this file is read by the check it tests
        let path = Path::new("a.rs");
        let sort = concat!("v.sort_by(|a, b| a.partial_cmp", "(b).unwrap());");
        let comment = concat!("// a.partial_cmp", "(b).unwrap()");
        let handled = concat!("a.partial_cmp", "(b).unwrap_or(x)");
        assert_eq!(unwraps_partial_cmp(path, sort), Some("a.rs: 1".to_string()));
        assert_eq!(unwraps_partial_cmp(path, comment), None);
        assert_eq!(unwraps_partial_cmp(path, handled), None);
    }
}
