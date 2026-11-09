//! The conventions in `.asimov/project.yaml` that a test can check
//! (roadmap T1-PROJECT-TRUTH: each convention is enforced or removed).

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

/// "No file > 1,500 lines"
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
            let compact: String = text.split_whitespace().collect();
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
