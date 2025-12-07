//! Break_Even tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_break_even_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "break_even.yaml",
        r#"_forge_version: "1.0.0"
price:
  value: 100
  formula: null
cost:
  value: 60
  formula: null
units:
  value: 100
  formula: null
fixed_costs:
  value: 2000
  formula: null
profit:
  value: null
  formula: "=(price - cost) * units - fixed_costs"
"#,
    );

    // Find units where profit = 0
    let result = break_even(
        yaml,
        "profit".to_string(),
        "units".to_string(),
        Some(1.0),
        Some(200.0),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_break_even_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "break_even_verbose.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
costs:
  value: 1200
  formula: null
margin_pct:
  value: 0.20
  formula: null
net:
  value: null
  formula: "=revenue * margin_pct - costs * 0.1"
"#,
    );

    let result = break_even(
        yaml,
        "net".to_string(),
        "revenue".to_string(),
        Some(100.0),
        Some(10000.0),
        true,
    );
    assert!(result.is_ok());
}
