//! `install.sh` downloads the archives the release workflow publishes.
//!
//! And the README's prompt for an AI runs what the script installs. A target or
//! a path renamed on one side and not the other would leave the one-line
//! install failing for everyone who copies it.

/// The one-line install the README and every prompt give
pub const INSTALL_LINE: &str = "curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples";

/// The prompts of a page: the body of each of its `text` code blocks.
#[must_use]
pub fn prompts(doc: &str) -> Vec<&str> {
    doc.split("```text\n")
        .skip(1)
        .map(|p| p.split("```").next().unwrap_or(""))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, root};

    #[test]
    fn prompts_are_the_text_blocks() {
        let doc = "intro\n```text\none\n```\nmore\n```text\ntwo\n```\n";
        assert_eq!(prompts(doc), ["one\n", "two\n"]);
        assert_eq!(prompts("no blocks"), [] as [&str; 0]);
    }

    #[test]
    fn install_sh_asks_for_archives_the_release_publishes() {
        let script = read("install.sh");
        let release = read(".github/workflows/release.yml");
        for target in ["unknown-linux-musl", "apple-darwin"] {
            assert!(
                script.contains(&format!("-{target}\"")),
                "install.sh: {target}"
            );
            for arch in ["x86_64", "aarch64"] {
                let full = format!("{arch}-{target}");
                assert!(release.contains(&full), "release.yml publishes no {full}");
            }
        }
        assert!(script.contains("archive=\"forge-$target.tar.gz\""));
        assert!(release.contains("forge-${{ matrix.target }}.tar.gz"));
        assert!(release.contains("checksums.txt"));
    }

    #[test]
    fn the_readme_installs_and_runs_what_exists() {
        let readme = read("README.md");
        let line = INSTALL_LINE;
        // In the prompt for an AI and in the quick start
        assert_eq!(
            readme.matches(line).count(),
            2,
            "README must give the install line twice"
        );
        assert!(readme.contains("I will keep asking you questions"));
        // The prompt runs the bagel example the script copies from examples/
        assert!(readme.contains("forge calculate forge-examples/bagel/bagel.yaml --dry-run"));
        let root = root();
        for file in ["examples/bagel/bagel.yaml", "examples/bagel/costs.yaml"] {
            assert!(root.join(file).exists(), "{file}");
        }
        assert!(read("install.sh").contains("forge-examples"));
    }

    #[test]
    fn every_prompt_in_docs_installs_checks_and_ends_with_the_yaml() {
        let doc = read("docs/PROMPTS.md");
        let line = INSTALL_LINE;
        let prompts = prompts(&doc);
        assert_eq!(prompts.len(), 6, "docs/PROMPTS.md has six prompts");
        for (i, prompt) in prompts.iter().enumerate() {
            let n = i + 1;
            assert!(
                prompt.contains("mollendorff-ai/forge"),
                "prompt {n} names the repository"
            );
            assert!(prompt.contains(line), "prompt {n} installs with install.sh");
            assert!(
                prompt.contains("print every YAML file"),
                "prompt {n} ends with the YAML"
            );
            assert!(
                prompt.contains("do not invent any"),
                "prompt {n} says what to do without a sandbox"
            );
            assert!(
                prompt.contains("yourself") || prompt.contains("your own code"),
                "prompt {n} asks for a check"
            );
            assert!(
                prompt.contains("I will keep asking you questions"),
                "prompt {n} says the conversation goes on"
            );
        }
        // Every example file a prompt names exists
        let root = root();
        for file in [
            "examples/household/budget.yaml",
            "examples/household/actual.yaml",
            "examples/bagel/bagel.yaml",
            "examples/bagel/bagel_simulate.yaml",
            "examples/bagel/second_shop.yaml",
            "examples/bagel/wait_option.yaml",
            "examples/starter.yaml",
        ] {
            assert!(root.join(file).exists(), "{file}");
        }
        for (name, path) in [
            (
                "household/budget.yaml",
                "forge-examples/household/budget.yaml",
            ),
            ("bagel_simulate.yaml", "bagel_simulate.yaml"),
            ("second_shop.yaml", "second_shop.yaml"),
            ("wait_option.yaml", "wait_option.yaml"),
        ] {
            assert!(doc.contains(path), "docs/PROMPTS.md names {name}");
        }
    }
}
