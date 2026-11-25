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
}
