//! What Forge says about itself cannot go stale (roadmap T1-CLAIMS): a count
//! written in a doc, the help text or a comment is compared with what the
//! code has, and a count that differs fails here, with the text to write.
//!
//! The counts of Gnumeric cases and files are checked the same way in
//! `validation/gnumeric/tests/doc_counts.rs`, where the cases are loaded.

use mollendorff_forge::functions::registry::all_functions;
use std::path::Path;

fn read(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The numbers written just before `word` (case ignored) in `text`:
/// `203 functions` gives 203, `1,865 tests` gives 1865.
fn numbers_before(text: &str, word: &str) -> Vec<u64> {
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

#[test]
fn every_function_count_written_anywhere_is_the_registry_count() {
    let actual = all_functions().count() as u64;
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
    let mut by_category = std::collections::BTreeMap::<String, usize>::new();
    for f in all_functions() {
        *by_category.entry(f.category.to_string()).or_default() += 1;
    }
    let mut wrong = Vec::new();
    for (category, count) in &by_category {
        let heading = format!("({count} functions)");
        let ok = doc
            .lines()
            .any(|l| l.starts_with("## ") && l.contains(category.as_str()) && l.contains(&heading));
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut count = 0;
    walk(&root.join("src"), &mut count);
    walk(&root.join("tests"), &mut count);
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

/// 1850 as `1,850`
fn group(n: u64) -> String {
    format!("{},{:03}", n / 1000, n % 1000)
}

/// Words and counts an evaluation found stale (2026-11-15): Forge's own docs
/// make no unmeasured claim of maturity, say how many times Excel was
/// checked, and give no dated unit-test count that the next commit outdates.
#[test]
fn the_docs_keep_no_stale_wording() {
    let mut wrong = Vec::new();
    let mut docs = vec!["README.md".to_string(), "VALIDATION.md".to_string()];
    for entry in std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs")).unwrap() {
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
