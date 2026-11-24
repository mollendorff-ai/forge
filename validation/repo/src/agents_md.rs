//! `AGENTS.md` holds the rules every coding agent must follow; `CLAUDE.md`
//! imports it and repeats none of them (roadmap A11-AGENTS-MD, ADR-078).
//!
//! Most coding agents read `AGENTS.md`; Claude Code reads `CLAUDE.md`, and
//! imports a file with a line that is `@` and its path (Claude Code's memory
//! page, read 2026-11-24). If the rules were in both files, a change to one
//! would leave the other saying something else, so this check fails when
//! the import goes, or when a line of a rule appears in both files.

/// The shortest line counted as a rule: a heading or a table border is not.
const MIN_RULE_LEN: usize = 40;

/// Whether `claude` imports `AGENTS.md`: a line of its own, outside a code
/// block and a code span, that is `@AGENTS.md`.
#[must_use]
pub fn imports_agents_md(claude: &str) -> bool {
    let mut fenced = false;
    for line in claude.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if !fenced && line.trim() == "@AGENTS.md" {
            return true;
        }
    }
    false
}

/// The lines of `agents` that are long enough to be a rule and that
/// `claude` has too, each once.
#[must_use]
pub fn repeated_rules(agents: &str, claude: &str) -> Vec<String> {
    let mut repeated: Vec<String> = Vec::new();
    for line in agents.lines().map(str::trim) {
        let again = line.len() >= MIN_RULE_LEN && claude.lines().any(|c| c.trim() == line);
        if again && !repeated.iter().any(|r| r == line) {
            repeated.push(line.to_string());
        }
    }
    repeated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read;

    const RULE: &str = "- **Formatting.** `cargo fmt --all -- --check` passes, with no skipping.";

    #[test]
    fn a_line_that_is_the_import_imports() {
        assert!(imports_agents_md("@AGENTS.md\n"));
        assert!(imports_agents_md("# Claude\n\n  @AGENTS.md  \nMore\n"));
    }

    #[test]
    fn a_mention_or_a_code_block_does_not_import() {
        assert!(!imports_agents_md("See @AGENTS.md for the rules\n"));
        assert!(!imports_agents_md("Write `@AGENTS.md` here\n"));
        assert!(!imports_agents_md("```\n@AGENTS.md\n```\n"));
        assert!(!imports_agents_md(""));
    }

    #[test]
    fn a_rule_in_both_files_is_found_once() {
        let agents = format!("# Rules\n\n{RULE}\n- Short\n{RULE}\n");
        let claude = format!("@AGENTS.md\n{RULE}\n  {RULE}\n- Short\n");
        assert_eq!(repeated_rules(&agents, &claude), [RULE]);
    }

    #[test]
    fn a_rule_in_one_file_only_is_not_repeated() {
        assert!(repeated_rules(&format!("{RULE}\n"), "@AGENTS.md\n").is_empty());
    }

    #[test]
    fn claude_md_imports_agents_md_and_repeats_none_of_it() {
        let agents = read("AGENTS.md");
        let claude = read("CLAUDE.md");
        assert!(imports_agents_md(&claude), "CLAUDE.md must say @AGENTS.md");
        assert_eq!(repeated_rules(&agents, &claude), Vec::<String>::new());
    }

    #[test]
    fn agents_md_holds_the_rules() {
        let agents = read("AGENTS.md");
        for rule in [
            "100% test coverage",
            "Pedantic linting",
            "Formatting",
            "Tests are inline",
            "No file over 1,500 lines",
        ] {
            assert!(agents.contains(rule), "AGENTS.md lacks: {rule}");
        }
    }
}
