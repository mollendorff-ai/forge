//! What Forge says about itself is what it does: its documents, workflows
//! and sources checked against its code and against what the `forge` binary
//! prints, so a doc, a count or a rule cannot drift from the engine.
//!
//! Each module is one kind of check: its code finds what the check reads,
//! and its tests, inline, assert that the repository passes. The checks that
//! run commands run the built binary, as a reader types them, like the
//! Gnumeric and R suites (ADR-057): the debug build, which the checks build
//! first so it is never older than the sources, or another named with
//! `FORGE_BIN`.
//!
//! ```text
//! cargo test -p mollendorff-forge-repo
//! ```

pub mod agent_rules;
pub mod agents_md;
pub mod architecture_docs;
pub mod conventions;
pub mod doc_counts;
pub mod doc_examples;
pub mod function_cases;
pub mod guide;
pub mod help_examples;
pub mod install_script;
pub mod no_skipping;
pub mod pipeline;
pub mod posts;
pub mod readme_commands;
pub mod skill;
pub mod workflows_doc;

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository's top folder.
#[must_use]
pub fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// A file of the repository, by its path from the top.
///
/// # Panics
///
/// Panics, naming the file, if it cannot be read.
#[must_use]
pub fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The Rust files under `dir`, at any depth, leaving out build output
/// (`target`) and installed packages (`node_modules`).
///
/// # Panics
///
/// Panics if `dir` or a folder in it cannot be read.
pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a folder entry").path();
        let name = path.file_name().unwrap_or_default();
        if path.is_dir() {
            if name != "target" && name != "node_modules" {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A line without its `//` comment, unless the `//` is inside a string (as
/// in a URL): a comment that names an attribute is not one.
#[must_use]
pub fn without_comment(line: &str) -> &str {
    let mut from = 0;
    while let Some(at) = line[from..].find("//") {
        let at = from + at;
        if line[..at].matches('"').count().is_multiple_of(2) {
            return &line[..at];
        }
        from = at + 2;
    }
    line
}

/// The `forge` binary the checks run: `FORGE_BIN` if it is set (a path from
/// the repository's top, or an absolute one), the debug build otherwise.
///
/// The debug build is built first, once a run, so the checks never run a
/// binary older than the sources: one built before 29b3729 wrote YAML into
/// `forge real-options -o results.json`, and `tables_for_example_files_match`
/// failed on it with "expected ident at line 1 column 2" though the code was
/// right. `FORGE_BIN` names a binary someone built for the run, as CI does.
///
/// # Panics
///
/// Panics if the build fails, or, saying how to build it, if there is no
/// such file.
#[must_use]
pub fn forge_bin() -> PathBuf {
    let path = std::env::var_os("FORGE_BIN").map_or_else(debug_bin, |bin| root().join(bin));
    assert!(
        path.is_file(),
        "no forge binary at {}: build it (cargo build --bin forge) or set FORGE_BIN",
        path.display()
    );
    path
}

/// The debug build of `forge`, built once a run so it is never older than
/// the sources.
///
/// # Panics
///
/// Panics if cargo cannot start or the build fails.
#[must_use]
pub fn debug_bin() -> PathBuf {
    static BUILT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let built = Command::new(env!("CARGO"))
            .args(["build", "--quiet", "--bin", "forge"])
            .current_dir(root())
            .status()
            .expect("cargo starts");
        assert!(built.success(), "cargo build --bin forge failed");
    });
    root().join("target/debug/forge")
}

/// What the binary did: its exit code and what it printed.
#[derive(Debug)]
pub struct Ran {
    /// The exit code, `None` if a signal ended it
    pub code: Option<i32>,
    /// Its standard output
    pub out: String,
    /// Its standard error
    pub err: String,
}

impl Ran {
    /// Whether it exited 0.
    #[must_use]
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// What it printed, for a command that must succeed.
    ///
    /// # Panics
    ///
    /// Panics, with what it wrote to standard error, if it did not exit 0.
    #[must_use]
    pub fn printed(self) -> String {
        assert!(self.success(), "{}{}", self.out, self.err);
        self.out
    }
}

/// `forge` with `args`, run from the repository's top as a reader runs it
/// there, without colour.
///
/// # Panics
///
/// Panics if the binary is missing or does not start.
#[must_use]
pub fn forge<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Ran {
    forge_in(root(), args)
}

/// `forge` with `args`, run from `dir`, without colour.
///
/// # Panics
///
/// Panics if the binary is missing or does not start.
#[must_use]
pub fn forge_in<S: AsRef<std::ffi::OsStr>>(dir: &Path, args: &[S]) -> Ran {
    let out = Command::new(forge_bin())
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("forge starts");
    Ran {
        code: out.status.code(),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_is_the_repository_with_its_manifest() {
        assert!(root().join("Cargo.toml").is_file());
        assert!(read("Cargo.toml").contains("name = \"mollendorff-forge\""));
    }

    /// A binary built before the sources it is checked against fails checks
    /// the code passes: one from before 29b3729 wrote YAML into
    /// `forge real-options -o results.json`.
    #[test]
    fn the_debug_binary_is_built_after_every_source_file() {
        let modified = |p: &Path| p.metadata().unwrap().modified().unwrap();
        let bin = modified(&debug_bin());
        let mut files = Vec::new();
        rust_files(&root().join("src"), &mut files);
        assert!(files.len() > 100, "{}", files.len());
        let newer: Vec<_> = files.iter().filter(|f| modified(f) > bin).collect();
        assert_eq!(newer, Vec::<&PathBuf>::new());
    }

    #[test]
    fn rust_files_finds_the_crates_own_sources() {
        let mut files = Vec::new();
        rust_files(&root().join("validation/repo"), &mut files);
        assert!(files.iter().any(|f| f.ends_with("src/lib.rs")), "{files:?}");
        assert!(files
            .iter()
            .all(|f| f.extension().is_some_and(|e| e == "rs")));
    }

    #[test]
    fn a_comment_is_cut_but_not_a_url() {
        assert_eq!(without_comment("let x = 1; // one"), "let x = 1; ");
        assert_eq!(
            without_comment("let u = \"https://x\"; // a"),
            "let u = \"https://x\"; "
        );
        assert_eq!(without_comment("no comment"), "no comment");
    }

    #[test]
    fn the_binary_prints_its_version_and_refuses_a_wrong_command() {
        let version = forge(&["--version"]).printed();
        assert!(version.starts_with("forge "), "{version}");
        let wrong = forge(&["nope"]);
        assert_eq!(wrong.code, Some(2));
        assert!(!wrong.success());
        assert!(
            wrong.err.contains("unrecognized subcommand"),
            "{}",
            wrong.err
        );
    }
}
