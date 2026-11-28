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

    /// What `text` of the file `path` says about the number of functions that
    /// is not `actual`: "203 functions", "203 Excel-compatible functions", "203
    /// supported functions". A category's own count (`## Math (22 functions)`)
    /// is checked apart.
    fn wrong_function_counts(path: &str, text: &str, actual: u64) -> Vec<String> {
        let mut wrong = Vec::new();
        for word in [
            " functions",
            " excel-compatible functions",
            " supported functions",
            " supported excel-compatible functions",
            " excel-style functions",
        ] {
            for n in numbers_before(text, word) {
                let is_category = text
                    .lines()
                    .any(|l| l.starts_with("## ") && l.contains(&format!("({n} functions)")));
                if n != actual && !is_category {
                    wrong.push(format!("{path}: says {n}"));
                }
            }
        }
        wrong
    }

    /// What `text` of the file `path` says about the number of tools that is
    /// not `actual`
    fn wrong_tool_counts(path: &str, text: &str, actual: u64) -> Vec<String> {
        let mut wrong = Vec::new();
        for word in [" tools", " forge tools", " mcp tools"] {
            for n in numbers_before(text, word) {
                if n != actual {
                    wrong.push(format!("{path}: says {n}"));
                }
            }
        }
        wrong
    }

    /// The categories whose heading in `doc` does not give the count of the
    /// code
    fn stale_categories(doc: &str, by_category: &BTreeMap<String, usize>) -> Vec<String> {
        let mut wrong = Vec::new();
        for (category, count) in by_category {
            let heading = format!("({count} functions)");
            let ok = doc.lines().any(|l| {
                l.starts_with("## ") && l.contains(category.as_str()) && l.contains(&heading)
            });
            if !ok {
                wrong.push(format!("{category}: the code has {count}"));
            }
        }
        wrong
    }

    /// The stale wording of `docs` (path and text): an unmeasured claim of
    /// maturity; of `excel_docs`, how Excel was checked; of the two rows,
    /// a unit-test count
    fn stale_wording(
        docs: &[(String, String)],
        excel_docs: &[(String, String)],
        readme: &str,
        validation: &str,
    ) -> Vec<String> {
        let mut wrong = Vec::new();
        for (path, text) in docs {
            let text = text.to_lowercase();
            for word in ["battle-proven", "battle-tested", "production-grade"] {
                if text.contains(word) {
                    wrong.push(format!("{path}: says {word}"));
                }
            }
        }
        for (path, text) in excel_docs {
            for once in ["by hand, once", "by hand once"] {
                if text.contains(once) {
                    wrong.push(format!("{path}: says Excel was checked {once}"));
                }
            }
        }
        let row = readme
            .lines()
            .find(|l| l.starts_with("| Unit tests"))
            .expect("README has a unit-test row");
        if row.contains(" pass, 0 fail (") || row.contains(" tests and ") {
            wrong.push("README.md: the unit-test row gives a count".into());
        }
        if validation.contains(" tests and 2 doctests") {
            wrong.push("VALIDATION.md: the unit-test row gives a count".into());
        }
        wrong
    }

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
        let mut wrong: Vec<String> = places
            .iter()
            .flat_map(|path| wrong_function_counts(path, &read(path), actual))
            .collect();
        wrong.sort();
        wrong.dedup();
        assert!(
            wrong.is_empty(),
            "the registry has {actual} functions; fix: {wrong:#?}"
        );
    }

    #[test]
    fn a_function_count_that_is_not_the_registrys_is_named_unless_it_is_a_category() {
        let text = "It has 5 functions, 7 Excel-compatible functions and 9 functions.\n\
                    ## Math (9 functions)\n";
        assert_eq!(wrong_function_counts("a.md", text, 7), ["a.md: says 5"]);
    }

    #[test]
    fn every_tool_count_written_in_the_docs_is_what_the_server_lists() {
        let actual = mcp_tool_count() as u64;
        let mut wrong: Vec<String> = ["README.md", "docs/MCP.md", "docs/WORKFLOWS.md"]
            .iter()
            .flat_map(|path| wrong_tool_counts(path, &read(path), actual))
            .collect();
        wrong.sort();
        wrong.dedup();
        assert!(
            wrong.is_empty(),
            "forge mcp lists {actual} tools; fix: {wrong:#?}"
        );
    }

    #[test]
    fn a_tool_count_that_is_not_the_servers_is_named() {
        assert_eq!(
            wrong_tool_counts("a.md", "23 tools, 21 MCP tools", 23),
            ["a.md: says 21"]
        );
    }

    #[test]
    fn the_function_doc_counts_each_category() {
        let doc = read("docs/FUNCTIONS.md");
        let (_, by_category) = registry();
        let wrong = stale_categories(&doc, &by_category);
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

    #[test]
    fn a_category_heading_with_another_count_is_named() {
        let by_category = BTreeMap::from([("Math".to_string(), 22), ("Text".to_string(), 3)]);
        assert_eq!(
            stale_categories(
                "## Math (22 functions)\n## Text (4 functions)\n",
                &by_category
            ),
            ["Text: the code has 3"]
        );
    }

    /// Words and counts an evaluation found stale (2026-11-15): Forge's own docs
    /// make no unmeasured claim of maturity, say how many times Excel was
    /// checked, and give no unit-test count at all: a test count says nothing about
    /// quality and went stale with every merge (roadmap E-DOC-COUNTS).
    #[test]
    fn the_docs_keep_no_stale_wording() {
        let mut docs = vec!["README.md".to_string(), "VALIDATION.md".to_string()];
        for entry in std::fs::read_dir(crate::root().join("docs")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.extension().is_some_and(|x| x == "md") && name != "CHANGELOG_ARCHIVE.md" {
                docs.push(format!("docs/{name}"));
            }
        }
        let read_all = |paths: &[String]| -> Vec<(String, String)> {
            paths
                .iter()
                .map(|path| (path.clone(), read(path)))
                .collect()
        };
        let excel = ["VALIDATION.md".to_string(), "docs/STABILITY.md".to_string()];
        let wrong = stale_wording(
            &read_all(&docs),
            &read_all(&excel),
            &read("README.md"),
            &read("VALIDATION.md"),
        );
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn stale_wording_is_named_in_each_place_it_is_looked_for() {
        let doc = |text: &str| vec![("a.md".to_string(), text.to_string())];
        let row = "| Unit tests | 1,865 tests and 2 doctests |\n";
        let wrong = stale_wording(
            &doc("A Battle-Tested engine"),
            &doc("checked by hand, once"),
            row,
            "1 tests and 2 doctests",
        );
        assert_eq!(
            wrong,
            [
                "a.md: says battle-tested",
                "a.md: says Excel was checked by hand, once",
                "README.md: the unit-test row gives a count",
                "VALIDATION.md: the unit-test row gives a count",
            ]
        );
        assert_eq!(
            stale_wording(&doc("fine"), &doc("fine"), "| Unit tests | many |\n", ""),
            Vec::<String>::new()
        );
    }
}
