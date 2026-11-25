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

/// The tools `forge mcp` lists to a client that asks, as an agent asks: how
/// many.
///
/// # Panics
///
/// Panics if the server does not start or its answer is not a tool list.
#[must_use]
pub fn mcp_tool_count() -> usize {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut server = Command::new(crate::forge_bin())
        .arg("mcp")
        .current_dir(crate::root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("forge mcp starts");
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"doc_counts","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    ];
    let mut stdin = server.stdin.take().expect("a pipe in");
    for request in requests {
        writeln!(stdin, "{request}").expect("the server reads");
    }
    drop(stdin);
    let out = server
        .wait_with_output()
        .expect("the server ends at the end of its input");
    let listed = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|message| message["id"] == 2)
        .expect("an answer to tools/list");
    listed["result"]["tools"]
        .as_array()
        .expect("a list of tools")
        .len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read;

    #[test]
    fn every_function_count_written_anywhere_is_the_registry_count() {
        let (actual, _) = registry();
        let places = [
            "README.md",
            "docs/FUNCTIONS.md",
            "docs/FEATURES.md",
            "docs/MCP.md",
            "src/lib.rs",
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
    fn every_tool_count_written_in_the_docs_is_what_the_server_lists() {
        let actual = mcp_tool_count() as u64;
        let mut wrong = Vec::new();
        for path in ["README.md", "docs/MCP.md", "docs/WORKFLOWS.md"] {
            let text = read(path);
            for word in [" tools", " forge tools", " mcp tools"] {
                for n in numbers_before(&text, word) {
                    if n != actual {
                        wrong.push(format!("{path}: says {n}"));
                    }
                }
            }
        }
        wrong.sort();
        wrong.dedup();
        assert!(
            wrong.is_empty(),
            "forge mcp lists {actual} tools; fix: {wrong:#?}"
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

    /// Words and counts an evaluation found stale (2026-11-15): Forge's own docs
    /// make no unmeasured claim of maturity, say how many times Excel was
    /// checked, and give no unit-test count at all: a test count says nothing about
    /// quality and went stale with every merge (roadmap E-DOC-COUNTS).
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
        if row.contains(" pass, 0 fail (") || row.contains(" tests and ") {
            wrong.push("README.md: the unit-test row gives a count".into());
        }
        if read("VALIDATION.md").contains(" tests and 2 doctests") {
            wrong.push("VALIDATION.md: the unit-test row gives a count".into());
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
