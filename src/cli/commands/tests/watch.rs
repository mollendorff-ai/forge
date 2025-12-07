//! Watch tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_watch_file_not_found() {
    let result = watch(PathBuf::from("/nonexistent.yaml"), false, false);
    assert!(result.is_err());
}
