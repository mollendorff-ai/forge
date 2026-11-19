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

    /// "No file > 1,500 lines", tests included: a longer file is split into
    /// modules by what it does, never emptied of its tests
    #[test]
    fn no_source_file_is_longer_than_1500_lines() {
        let long: Vec<String> = sources()
            .into_iter()
            .filter_map(|(path, text)| {
                let lines = text.lines().count();
                (lines > 1500).then(|| format!("{}: {lines} lines", path.display()))
            })
            .collect();
        assert!(long.is_empty(), "split these files: {long:#?}");
    }

    /// "Sorting and comparing f64 uses `total_cmp`, never `partial_cmp().unwrap()`":
    /// a NaN would panic.
    #[test]
    fn no_partial_cmp_unwrap() {
        let found: Vec<String> = sources()
            .into_iter()
            .filter_map(|(path, text)| {
                // Code only: a comment that quotes the pattern does not use it
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
            })
            .collect();
        assert!(found.is_empty(), "use total_cmp: {found:#?}");
    }
}
