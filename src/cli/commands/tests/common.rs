//! Common test utilities for CLI commands tests

use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test YAML file in the given temp directory
pub fn create_test_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}
