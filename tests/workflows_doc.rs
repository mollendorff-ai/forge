//! docs/WORKFLOWS.md names Forge commands in its diagrams and examples; each
//! must be one the binary has, and each page it links to must exist.
//! A coverage build stubs `main` (ADR-006), so this runs the real binary only
//! outside coverage.

#![cfg(not(coverage))]

use std::path::Path;
use std::process::Command;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn every_command_the_workflows_page_names_exists() {
    let doc = std::fs::read_to_string(root().join("docs/WORKFLOWS.md")).unwrap();
    let help = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&help.stdout).to_string();
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
