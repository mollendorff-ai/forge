//! `install.sh` downloads the archives the release workflow publishes, and
//! the README's prompt for an AI runs what the script installs. A target or
//! a path renamed on one side and not the other would leave the one-line
//! install failing for everyone who copies it.

use std::path::Path;

fn read(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
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
    let line = "curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples";
    // In the prompt for an AI and in the quick start
    assert_eq!(
        readme.matches(line).count(),
        2,
        "README must give the install line twice"
    );
    // The prompt runs the bagel example the script copies from examples/
    assert!(readme.contains("forge calculate forge-examples/bagel/bagel.yaml --dry-run"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in ["examples/bagel/bagel.yaml", "examples/bagel/costs.yaml"] {
        assert!(root.join(file).exists(), "{file}");
    }
    assert!(read("install.sh").contains("forge-examples"));
}

#[test]
fn every_prompt_in_docs_installs_checks_and_ends_with_the_yaml() {
    let doc = read("docs/PROMPTS.md");
    let line = "curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples";
    let prompts: Vec<&str> = doc
        .split("```text\n")
        .skip(1)
        .map(|p| p.split("```").next().unwrap_or(""))
        .collect();
    assert_eq!(prompts.len(), 5, "docs/PROMPTS.md has five prompts");
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
    }
    // Every example file a prompt names exists
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
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
