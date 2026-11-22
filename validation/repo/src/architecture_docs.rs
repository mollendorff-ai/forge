//! The architecture pages in `docs/architecture` cannot point at what is gone.
//!
//! Every link and anchor in them resolves, every repository path named by the
//! pages that describe today's code exists, every ADR is in the index, and
//! every module of `src/lib.rs` is in the module map (roadmap T1-CLAIMS).
//!
//! The ADRs are records of a decision at their date, so a path an ADR names
//! is not checked; their links are.

use std::path::Path;

/// The folders a repository path named in a page starts with.
const TOP: [&str; 10] = [
    "src/",
    "forge-wasm/",
    "studio/",
    "validation/",
    "schema/",
    "examples/",
    "editors/",
    "scripts/",
    "test-data/",
    ".github/",
];

/// `doc` without its fenced code blocks (diagrams, YAML, shell).
#[must_use]
pub fn prose(doc: &str) -> String {
    let mut out = String::new();
    let mut fenced = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if !fenced {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The targets of the Markdown links `[text](target)` in `doc`'s prose,
/// outside code spans, that are files of this repository (no scheme), each
/// with its `#anchor` if it has one.
#[must_use]
pub fn local_links(doc: &str) -> Vec<String> {
    let text: String = prose(doc).split('`').step_by(2).collect();
    let mut links = Vec::new();
    for (at, _) in text.match_indices("](") {
        let rest = &text[at + 2..];
        let Some(end) = rest.find(')') else {
            continue;
        };
        let target = &rest[..end];
        if !target.is_empty() && !target.contains(':') {
            links.push(target.to_string());
        }
    }
    links
}

/// The anchor GitHub gives a heading: lower case, letters, digits, `-` and
/// `_` kept, each space a `-`, everything else dropped.
#[must_use]
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// The anchors of `doc`'s headings.
#[must_use]
pub fn anchors(doc: &str) -> Vec<String> {
    prose(doc)
        .lines()
        .filter_map(|line| {
            let text = line.trim_start_matches('#');
            (text.len() < line.len() && text.starts_with(' ')).then(|| slug(text))
        })
        .collect()
}

/// The repository paths `doc` names in code spans (`src/core/mod.rs`),
/// each once. A span with a placeholder (`<family>`, `*`, `5.N.0`) or a
/// space is not a path.
#[must_use]
pub fn named_paths(doc: &str) -> Vec<String> {
    let text = prose(doc);
    let mut paths: Vec<String> = Vec::new();
    for span in text.split('`').skip(1).step_by(2) {
        let placeholder = span.contains(|c: char| c.is_whitespace() || "*<>{}".contains(c))
            || span.contains("N.0");
        if !placeholder
            && TOP.iter().any(|top| span.starts_with(top))
            && !paths.iter().any(|p| p == span)
        {
            paths.push(span.to_string());
        }
    }
    paths
}

/// What is wrong with the link `target` in the page at `page`: `None` when
/// its file exists and, if it names an anchor in a Markdown file, that file
/// has the heading.
#[must_use]
pub fn broken_link(page: &Path, target: &str) -> Option<String> {
    let (file, anchor) = target
        .split_once('#')
        .map_or((target, None), |(file, anchor)| (file, Some(anchor)));
    let dir = page.parent().unwrap_or(page);
    let path = if file.is_empty() {
        page.to_path_buf()
    } else {
        dir.join(file)
    };
    if !path.exists() {
        return Some(format!("{target}: no such file"));
    }
    let anchor = anchor?;
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if anchors(&text).iter().any(|a| a == anchor) {
        None
    } else {
        Some(format!("{target}: no heading with that anchor"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, root};

    /// The Markdown files of `docs/architecture`, by name.
    fn pages() -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(root().join("docs/architecture"))
            .expect("docs/architecture")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| {
                Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .collect();
        names.sort();
        names
    }

    /// The pages that describe the code as it is today: the numbered pages
    /// and the index, not the ADRs.
    fn current_pages() -> Vec<String> {
        pages()
            .into_iter()
            .filter(|name| !name.starts_with("ADR-"))
            .collect()
    }

    #[test]
    fn prose_leaves_out_fenced_blocks() {
        let doc = "a\n```mermaid\nb[\"x\"](y)\n```\nc\n";
        assert_eq!(prose(doc), "a\nc\n");
    }

    #[test]
    fn links_are_local_targets_outside_code_blocks() {
        let doc = "[a](01-X.md#part) [b](https://example.com) [c](../../README.md)\n\
                   `#,##0;[Red](#,##0)` [empty]() a ](without an end\n\
                   ```text\n[d](gone.md)\n```\n";
        assert_eq!(local_links(doc), ["01-X.md#part", "../../README.md"]);
    }

    #[test]
    fn slugs_are_githubs() {
        assert_eq!(slug("01 - Components"), "01---components");
        assert_eq!(
            slug(" PREV: columns that read the row above"),
            "prev-columns-that-read-the-row-above"
        );
        assert_eq!(slug("Add a `forge` command"), "add-a-forge-command");
        assert_eq!(slug("Coûts_unitaires"), "coûts_unitaires");
    }

    #[test]
    fn anchors_are_headings_outside_code_blocks() {
        let doc = "# Title\n## Two words\n#not a heading\n```\n# comment\n```\n";
        assert_eq!(anchors(doc), ["title", "two-words"]);
    }

    #[test]
    fn named_paths_are_concrete_repository_paths() {
        let doc = "`src/lib.rs`, `src/lib.rs` again, `src/mcp/server/*_tools.rs`, \
                   `evaluator/<family>`, `schema/forge-v5.N.0.schema.json`, `cargo test`, \
                   `core::precision`, `.github/workflows/ci.yml`\n```\n`src/gone.rs`\n```\n";
        assert_eq!(named_paths(doc), ["src/lib.rs", ".github/workflows/ci.yml"]);
    }

    #[test]
    fn a_link_is_broken_by_a_missing_file_or_anchor() {
        let page = root().join("docs/architecture/README.md");
        assert_eq!(broken_link(&page, "00-OVERVIEW.md"), None);
        assert_eq!(broken_link(&page, "00-OVERVIEW.md#principles"), None);
        assert_eq!(broken_link(&page, "#pages"), None);
        assert_eq!(broken_link(&page, "../../src/"), None);
        assert_eq!(
            broken_link(&page, "99-GONE.md"),
            Some("99-GONE.md: no such file".to_string())
        );
        assert_eq!(
            broken_link(&page, "00-OVERVIEW.md#gone"),
            Some("00-OVERVIEW.md#gone: no heading with that anchor".to_string())
        );
    }

    #[test]
    fn every_link_in_the_architecture_pages_resolves() {
        let mut broken = Vec::new();
        for name in pages() {
            let page = root().join("docs/architecture").join(&name);
            for target in local_links(&read(&format!("docs/architecture/{name}"))) {
                if let Some(why) = broken_link(&page, &target) {
                    broken.push(format!("{name}: {why}"));
                }
            }
        }
        assert!(broken.is_empty(), "broken links: {broken:#?}");
    }

    #[test]
    fn every_path_the_current_pages_name_exists() {
        let mut missing = Vec::new();
        let mut named = 0;
        for name in current_pages() {
            for path in named_paths(&read(&format!("docs/architecture/{name}"))) {
                named += 1;
                if !root().join(&path).exists() {
                    missing.push(format!("{name}: {path}"));
                }
            }
        }
        assert!(named > 100, "the pages name the code they describe");
        assert!(missing.is_empty(), "paths that do not exist: {missing:#?}");
    }

    #[test]
    fn every_adr_is_in_the_index() {
        let index = read("docs/architecture/README.md");
        let missing: Vec<String> = pages()
            .into_iter()
            .filter(|name| name.starts_with("ADR-") && !index.contains(&format!("]({name})")))
            .collect();
        assert!(
            missing.is_empty(),
            "ADRs missing from the index: {missing:?}"
        );
    }

    #[test]
    fn every_module_of_the_crate_is_in_the_module_map() {
        let map = read("docs/architecture/01-COMPONENT-ARCHITECTURE.md");
        let missing: Vec<String> = read("src/lib.rs")
            .lines()
            .filter_map(|line| line.strip_prefix("pub mod "))
            .map(|rest| rest.trim_end_matches(';').to_string())
            .filter(|module| !map.contains(&format!("| `{module}` |")))
            .collect();
        assert!(
            missing.is_empty(),
            "modules missing from the module map in 01-COMPONENT-ARCHITECTURE.md: {missing:?}"
        );
    }
}
