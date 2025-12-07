//! End-to-end tests for forge CLI
//!
//! # Coverage Exclusion (ADR-006)
//! These tests are skipped during coverage runs because the binaries are
//! stubbed to empty main() functions. Run without coverage for full testing.

// Skip all e2e tests during coverage builds (ADR-006)
// The binaries have stubbed main() functions that exit immediately
#![cfg(not(coverage))]

use std::path::PathBuf;
use std::process::Command;

fn forge_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("forge");

    if !path.exists() {
        path.pop();
        path.pop();
        path.push("debug");
        path.push("forge");
    }

    path
}

fn test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data");
    path.push(filename);
    path
}

// ========== Basic Validation Tests ==========

#[test]
fn e2e_watch_nonexistent_file_fails() {
    let file = test_data_path("this_file_does_not_exist.yaml");

    let output = Command::new(forge_binary())
        .arg("watch")
        .arg(&file)
        .output()
        .expect("Failed to execute watch");

    assert!(
        !output.status.success(),
        "Watch should fail for nonexistent file"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("not found") || combined.contains("File not found"),
        "Should report file not found, got: {combined}"
    );
}

#[test]
fn e2e_watch_help_shows_usage() {
    let output = Command::new(forge_binary())
        .arg("watch")
        .arg("--help")
        .output()
        .expect("Failed to execute watch --help");

    assert!(output.status.success(), "watch --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Watch") || stdout.contains("watch"),
        "Should show watch help, got: {stdout}"
    );

    assert!(
        stdout.contains("--validate") || stdout.contains("validate"),
        "Should show --validate option, got: {stdout}"
    );
}
