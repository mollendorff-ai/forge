// Enterprise-only: Gnumeric E2E tests for enterprise functions
#![cfg(all(feature = "full", feature = "e2e-gnumeric"))]
// Allow approximate constants - we're testing Excel formula results, not Rust math
#![allow(clippy::approx_constant)]

//! Spreadsheet Engine E2E Validation Tests
//!
//! Validates Forge calculations against battle-proven spreadsheet engines
//! (Gnumeric/LibreOffice) with decades of use and millions of users.
//!
//! # Why External Validation?
//! We don't know if Forge hallucinated formulas. External spreadsheet engines
//! provide validation from something REAL and battle-tested.
//!
//! # How It Works
//! 1. Forge exports XLSX with formulas
//! 2. ssconvert (Gnumeric) recalculates and exports to CSV
//! 3. Rust test compares values - any mismatch = we have a bug
//!
//! # Running Tests
//! ```bash
//! cargo test --features e2e-gnumeric
//! ```
//!
//! # Requirements
//! - gnumeric installed (`ssconvert --version`) - preferred, properly recalculates
//! - OR LibreOffice installed (`libreoffice --version`) - fallback
//! - Tests skip gracefully if neither found
//!
//! # Coverage Exclusion (ADR-006)
//! These tests are skipped during coverage runs.

#![cfg(all(feature = "e2e-gnumeric", not(coverage)))]

use std::fs;
use std::process::Command;

// Import the roundtrip test modules
mod roundtrip;

// Re-export for use in tests
use roundtrip::harness::{forge_binary, E2ETestHarness};

// ═══════════════════════════════════════════════════════════════════════════════
// INFRASTRUCTURE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn spreadsheet_engine_detection_works() {
    let engine = require_gnumeric!();
    println!("✅ {} detected: {}", engine.name(), engine.version());
    // Either gnumeric or LibreOffice is fine
    assert!(
        engine.version().contains("gnumeric")
            || engine.version().contains("Gnumeric")
            || engine.version().contains("LibreOffice")
            || engine.name().contains("Gnumeric")
    );
}

#[test]
fn gnumeric_conversion_works() {
    let lo = require_gnumeric!();

    // Create a simple test XLSX using Forge (simplified format)
    let yaml_content = r#"_forge_version: "5.0.0"
test_data:
  row: [1]
  test_sum: "=1+2+3"
"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_path = temp_dir.path().join("test.yaml");
    let xlsx_path = temp_dir.path().join("test.xlsx");

    fs::write(&yaml_path, yaml_content).unwrap();

    // Export using Forge
    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_path)
        .arg(&xlsx_path)
        .output()
        .expect("Failed to run forge export");

    assert!(
        output.status.success(),
        "Forge export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Convert to CSV using Gnumeric
    let csv_path = lo.xlsx_to_csv(&xlsx_path, temp_dir.path()).unwrap();
    assert!(csv_path.exists(), "CSV file should exist");

    let csv_content = fs::read_to_string(&csv_path).unwrap();
    println!("CSV content:\n{}", csv_content);

    // The CSV should contain calculated values
    assert!(!csv_content.is_empty(), "CSV should not be empty");
}

#[test]
fn e2e_gnumeric_comprehensive_validation() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping comprehensive test");
            return;
        }
    };

    println!("\n══════════════════════════════════════════════════════════════");
    println!("  Gnumeric E2E Validation - Comprehensive Test");
    println!(
        "  Engine: {} ({})",
        harness.engine.name(),
        harness.engine.version()
    );
    println!("══════════════════════════════════════════════════════════════\n");

    let mut passed = 0;
    let mut failed = 0;

    // List of formulas to test with expected results
    // Covers 50+ functions validated against Gnumeric/LibreOffice
    let tests: Vec<(&str, f64, f64)> = vec![
        // === MATH (15 functions) ===
        ("ABS(-5)", 5.0, 0.001),
        ("SQRT(144)", 12.0, 0.001),
        ("POWER(2, 8)", 256.0, 0.001),
        ("MOD(17, 5)", 2.0, 0.001),
        ("ROUND(3.14159, 2)", 3.14, 0.001),
        ("ROUNDUP(3.14159, 2)", 3.15, 0.001),
        ("ROUNDDOWN(3.14159, 2)", 3.14, 0.001),
        ("FLOOR(3.7, 1)", 3.0, 0.001),
        ("CEILING(3.2, 1)", 4.0, 0.001),
        ("INT(3.7)", 3.0, 0.001),
        ("TRUNC(3.7)", 3.0, 0.001),
        ("SIGN(-42)", -1.0, 0.001),
        ("LN(2.71828)", 1.0, 0.01),
        ("LOG10(1000)", 3.0, 0.001),
        ("EXP(0)", 1.0, 0.001),
        // === TRIGONOMETRY (7 functions) ===
        ("PI()", 3.14159, 0.0001),
        ("SIN(0)", 0.0, 0.001),
        ("COS(0)", 1.0, 0.001),
        ("TAN(0)", 0.0, 0.001),
        ("RADIANS(180)", 3.14159, 0.0001),
        ("DEGREES(PI())", 180.0, 0.001),
        ("SIN(PI()/2)", 1.0, 0.001),
        // === LOGICAL (5 functions) ===
        ("IF(10>5, 1, 0)", 1.0, 0.001),
        ("IF(10<5, 1, 0)", 0.0, 0.001),
        ("IFERROR(1/0, -1)", -1.0, 0.001),
        ("IF(AND(1>0, 2>1), 1, 0)", 1.0, 0.001),
        ("IF(OR(1<0, 2>1), 1, 0)", 1.0, 0.001),
        // === FINANCIAL (8 functions) ===
        ("PMT(0.05/12, 60, 10000)", -188.71, 1.0),
        ("PV(0.08/12, 60, -1000)", 49318.43, 10.0),
        ("FV(0.05/12, 120, -100, 0)", 15528.23, 10.0),
        ("NPV(0.1, 3000, 4200, 6800)", 11308.20, 10.0),
        ("NPER(0.06/12, -200, 10000)", 57.68, 1.0),
        ("SLN(30000, 7500, 10)", 2250.0, 1.0),
        ("DDB(1000000, 100000, 6, 1)", 333333.33, 10.0),
        ("RATE(48, -500, 20000)", 0.0077, 0.001),
        // === DATE (7 functions) ===
        ("YEAR(DATE(2025, 6, 15))", 2025.0, 0.001),
        ("MONTH(DATE(2025, 6, 15))", 6.0, 0.001),
        ("DAY(DATE(2025, 6, 15))", 15.0, 0.001),
        ("WEEKDAY(DATE(2025, 12, 7))", 1.0, 0.001),
        ("HOUR(0.5)", 12.0, 0.001),
        ("MONTH(EDATE(DATE(2024, 1, 15), 3))", 4.0, 0.001),
        ("DATE(2024, 12, 31) - DATE(2024, 1, 1)", 365.0, 0.001),
        // === INFORMATION (2 functions) ===
        ("IF(ISEVEN(4), 1, 0)", 1.0, 0.001),
        ("IF(ISODD(5), 1, 0)", 1.0, 0.001),
    ];

    for (formula, expected, tolerance) in tests {
        match harness.test_formula(formula, expected, tolerance) {
            Ok(()) => {
                println!("  ✅ {} = {}", formula, expected);
                passed += 1;
            }
            Err(e) => {
                println!("  ❌ {} - {}", formula, e);
                failed += 1;
            }
        }
    }

    println!("\n══════════════════════════════════════════════════════════════");
    println!("  Results: {} passed, {} failed", passed, failed);
    println!("══════════════════════════════════════════════════════════════\n");

    assert_eq!(
        failed, 0,
        "Some formulas failed validation against Gnumeric"
    );
}
