//! `forge --version` says which commit a build came from (Session Prompt part
//! 1, "A build of main says forge 10.0.0-rc.11, the same as the release").
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::process::Command;

fn version() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn the_version_starts_with_the_crate_version_and_a_stamp_is_a_commit() {
    let text = version();
    let plain = format!("forge {}", env!("CARGO_PKG_VERSION"));
    assert!(text.starts_with(&plain), "{text}");
    let rest = text[plain.len()..].trim();
    if !rest.is_empty() {
        // `(a8ec1db)` or `(a8ec1db, modified)`
        let inner = rest
            .strip_prefix('(')
            .and_then(|r| r.strip_suffix(')'))
            .expect(&text);
        let commit = inner.strip_suffix(", modified").unwrap_or(inner);
        assert_eq!(commit.len(), 7, "{text}");
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()), "{text}");
    }
}
