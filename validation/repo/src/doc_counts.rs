//! What Forge says about itself cannot go stale (roadmap T1-CLAIMS).
//!
//! A count written in a doc, the help text or a comment is compared with what the
//! code has, and a count that differs fails here, with the text to write.
//!
//! The counts of Gnumeric cases and files are checked the same way in
//! `validation/gnumeric/src/runner.rs`, where the cases are loaded.

use std::collections::BTreeMap;

/// The functions `forge functions --json` lists: how many, and how many in
/// each category.
///
/// # Panics
///
/// Panics if the command fails or its JSON is not the function catalog.
#[must_use]
pub fn registry() -> (u64, BTreeMap<String, usize>) {
    let catalog: serde_json::Value =
        serde_json::from_str(&crate::forge(&["functions", "--json"]).printed())
            .expect("the catalog is JSON");
    let total = catalog["total"].as_u64().expect("a total");
    let by_category = catalog["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .map(|c| {
            let count = c["count"].as_u64().expect("a count");
            (
                c["name"].as_str().expect("a name").to_string(),
                usize::try_from(count).expect("a count that fits"),
            )
        })
        .collect();
    (total, by_category)
}

/// The numbers written just before `word` (case ignored) in `text`:
/// `203 functions` gives 203, `1,865 tests` gives 1865.
#[must_use]
pub fn numbers_before(text: &str, word: &str) -> Vec<u64> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(word) {
        let end = from + at;
        let digits: String = lower[..end]
            .trim_end()
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == ',')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .filter(|c| *c != ',')
            .collect();
        if let Ok(n) = digits.parse() {
            out.push(n);
        }
        from = end + word.len();
    }
    out
}

/// 1850 as `1,850`
#[must_use]
pub fn group(n: u64) -> String {
    format!("{},{:03}", n / 1000, n % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read;
    use std::path::Path;

    #[test]
    fn every_function_count_written_anywhere_is_the_registry_count() {
        let (actual, _) = registry();
        let places = [
            "README.md",
            "docs/FUNCTIONS.md",
            "docs/FEATURES.md",
            "docs/MCP.md",
            "src/main.rs",
            "src/cli/args.rs",
            "src/mcp/mod.rs",
            "src/mcp/server.rs",
            "Cargo.toml",
        ];
        let mut wrong = Vec::new();
        for path in places {
            let text = read(path);
            // "203 functions", "203 Excel-compatible functions", "203 supported functions"
            for word in [
                " functions",
                " excel-compatible functions",
                " supported functions",
                " supported excel-compatible functions",
                " excel-style functions",
            ] {
                for n in numbers_before(&text, word) {
                    // A category's own count (`## Math (22 functions)`) is checked below
                    let is_category = text
                        .lines()
                        .any(|l| l.starts_with("## ") && l.contains(&format!("({n} functions)")));
                    if n != actual && !is_category {
                        wrong.push(format!("{path}: says {n}"));
                    }
                }
            }
        }
        wrong.sort();
        wrong.dedup();
        assert!(
            wrong.is_empty(),
            "the registry has {actual} functions; fix: {wrong:#?}"
        );
    }

    #[test]
    fn the_function_doc_counts_each_category() {
        let doc = read("docs/FUNCTIONS.md");
        let (_, by_category) = registry();
        let mut wrong = Vec::new();
        for (category, count) in &by_category {
            let heading = format!("({count} functions)");
            let ok = doc.lines().any(|l| {
                l.starts_with("## ") && l.contains(category.as_str()) && l.contains(&heading)
            });
            if !ok {
                wrong.push(format!("{category}: the code has {count}"));
            }
        }
        assert!(
            wrong.is_empty(),
            "docs/FUNCTIONS.md headings are out of date: {wrong:#?}"
        );
        let categories = by_category.len() as u64;
        assert!(
            doc.contains(&format!("across {categories} categories")),
            "docs/FUNCTIONS.md must say `across {categories} categories`"
        );
    }

    /// The `#[test]` functions in this repository's Rust files (a `proptest!`
    /// block counts its tests by their `fn`).
    fn tests_in_the_code() -> u64 {
        fn walk(dir: &Path, count: &mut u64) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, count);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap_or_default();
                    *count += text
                        .lines()
                        .filter(|l| {
                            let l = l.trim_start();
                            l.starts_with("#[test]") || l.starts_with("#[tokio::test")
                        })
                        .count() as u64;
                }
            }
        }
        let root = crate::root();
        let mut count = 0;
        // The main crate's tests, and these checks of the repository
        walk(&root.join("src"), &mut count);
        walk(&root.join("validation/repo/src"), &mut count);
        count
    }

    #[test]
    fn the_test_count_in_the_docs_is_a_round_figure_just_under_the_real_one() {
        // The exact number changes with every pull request; the docs say "over
        // 1,850 tests", in steps of 50, and this fails when that is no longer so.
        let actual = tests_in_the_code();
        let floor = actual / 50 * 50;
        let mut wrong = Vec::new();
        for path in ["README.md", "VALIDATION.md"] {
            let text = read(path);
            let said = numbers_before(&text, " tests and 2 doctests");
            assert!(!said.is_empty(), "{path} must say `N tests and 2 doctests`");
            for n in said {
                if n != floor {
                    wrong.push(format!("{path}: says {n}"));
                }
            }
            assert!(
                text.contains(&format!("over {} tests and 2 doctests", group(floor))),
                "{path} must say `over {} tests and 2 doctests`",
                group(floor)
            );
        }
        assert!(
            wrong.is_empty(),
            "the code has {actual} tests; write `over {}`: {wrong:#?}",
            group(floor)
        );
    }

    /// Words and counts an evaluation found stale (2026-11-15): Forge's own docs
    /// make no unmeasured claim of maturity, say how many times Excel was
    /// checked, and give no dated unit-test count that the next commit outdates.
    #[test]
    fn the_docs_keep_no_stale_wording() {
        let mut wrong = Vec::new();
        let mut docs = vec!["README.md".to_string(), "VALIDATION.md".to_string()];
        for entry in std::fs::read_dir(crate::root().join("docs")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.extension().is_some_and(|x| x == "md") && name != "CHANGELOG_ARCHIVE.md" {
                docs.push(format!("docs/{name}"));
            }
        }
        for path in &docs {
            let text = read(path).to_lowercase();
            for word in ["battle-proven", "battle-tested", "production-grade"] {
                if text.contains(word) {
                    wrong.push(format!("{path}: says {word}"));
                }
            }
        }
        for path in ["VALIDATION.md", "docs/STABILITY.md"] {
            let text = read(path);
            for once in ["by hand, once", "by hand once"] {
                if text.contains(once) {
                    wrong.push(format!("{path}: says Excel was checked {once}"));
                }
            }
        }
        let readme = read("README.md");
        let row = readme
            .lines()
            .find(|l| l.starts_with("| Unit tests"))
            .expect("README has a unit-test row");
        if row.contains(" pass, 0 fail (") {
            wrong.push("README.md: the unit-test row gives a dated count".into());
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
