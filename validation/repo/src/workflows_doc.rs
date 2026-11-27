//! docs/WORKFLOWS.md names Forge commands in its diagrams and examples; each
//! must be one the binary has, and each page it links to must exist.

/// The commands a text names as `forge <command>`, each once, in order.
#[must_use]
pub fn commands_named(doc: &str) -> Vec<String> {
    let mut named = Vec::new();
    for (i, _) in doc.match_indices("forge ") {
        let word: String = doc[i + 6..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .collect();
        if !word.is_empty() && !named.contains(&word) {
            named.push(word);
        }
    }
    named
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forge, read, root};

    #[test]
    fn commands_are_named_once_each_and_a_name_is_not_one() {
        assert_eq!(
            commands_named(
                "forge calculate, then forge report; forge calculate. Forge, one binary"
            ),
            ["calculate", "report"]
        );
    }

    #[test]
    fn every_command_the_workflows_page_names_exists() {
        let doc = read("docs/WORKFLOWS.md");
        let help = forge(&["--help"]).printed();
        let named = commands_named(&doc);
        for word in [
            "calculate",
            "validate",
            "report",
            "export",
            "variance",
            "mcp",
            "serve",
        ] {
            assert!(
                named.contains(&word.to_string()),
                "the page names forge {word}"
            );
        }
        for word in &named {
            // "forge validate" and the like; "Forge, one binary" is not a command
            assert!(
                help.contains(&format!("  {word} ")),
                "docs/WORKFLOWS.md names `forge {word}`, which `forge --help` does not list"
            );
        }
        for page in ["docs/PROMPTS.md", "docs/MCP.md"] {
            assert!(root().join(page).exists(), "{page}");
        }
        assert!(
            doc.contains("```mermaid"),
            "the diagrams are Mermaid, which GitHub draws"
        );
    }

    #[test]
    fn a_page_only_change_runs_the_repository_checks() {
        // push and pull_request each list the paths that start CI
        let ci = read(".github/workflows/ci.yml");
        let head = ci.split("\nconcurrency:").next().unwrap_or_default();
        for path in ["docs/**", "**/*.md", "skills/**"] {
            assert_eq!(
                head.matches(&format!("      - '{path}'\n")).count(),
                2,
                "{path} is in the filter of both push and pull_request"
            );
        }
    }

    /// The actions `text` of the workflow `path` uses without pinning them to
    /// a commit
    fn unpinned_actions(path: &str, text: &str) -> Vec<String> {
        let mut wrong = Vec::new();
        for line in text.lines() {
            let Some(used) = line
                .trim_start()
                .strip_prefix("- uses: ")
                .or_else(|| line.trim_start().strip_prefix("uses: "))
            else {
                continue;
            };
            let used = used.split_whitespace().next().unwrap_or_default();
            let pinned = used.rsplit_once('@').is_some_and(|(_, sha)| {
                sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())
            });
            if !used.starts_with("./") && !pinned {
                wrong.push(format!("{path}: {used}"));
            }
        }
        wrong
    }

    /// An action is pinned to a commit
    /// (a tag can be moved), so that what runs is what was reviewed.
    #[test]
    fn every_action_is_pinned_to_a_commit() {
        let mut wrong = Vec::new();
        let dir = root().join(".github/workflows");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            wrong.extend(unpinned_actions(&path.display().to_string(), &text));
        }
        assert!(
            wrong.is_empty(),
            "actions not pinned to a commit: {wrong:#?}"
        );
    }

    #[test]
    fn an_action_on_a_tag_is_named() {
        let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
        let text = format!(
            "steps:\n  - uses: actions/checkout@{sha} # v7\n  - uses: actions/cache@v4\n    \
             uses: ./local\n  - run: echo uses: x\n"
        );
        assert_eq!(
            unpinned_actions("ci.yml", &text),
            ["ci.yml: actions/cache@v4"]
        );
    }

    #[test]
    fn the_scorecard_runs_weekly_and_on_main_and_the_readme_shows_its_badge() {
        let workflow = read(".github/workflows/scorecard.yml");
        for needed in [
            "schedule:",
            "branches: [ main ]",
            "publish_results: true",
            "permissions: read-all",
        ] {
            assert!(workflow.contains(needed), "scorecard.yml has `{needed}`");
        }
        assert!(read("README.md")
            .contains("https://api.scorecard.dev/projects/github.com/mollendorff-ai/forge/badge"));
    }

    #[test]
    fn codeql_analyses_the_rust_and_the_workflows_on_pull_requests_and_weekly() {
        let workflow = read(".github/workflows/codeql.yml");
        for needed in [
            "pull_request:",
            "schedule:",
            "language: [ rust, actions ]",
            "github/codeql-action/init@",
            "github/codeql-action/analyze@",
            "permissions: read-all",
        ] {
            assert!(workflow.contains(needed), "codeql.yml has `{needed}`");
        }
    }
}
