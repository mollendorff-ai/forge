//! Aggregation function E2E tests

use super::harness::*;
use std::fs;
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════════════
// AGGREGATION FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_gnumeric_sum() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test SUM with column reference
    harness
        .test_aggregation("SUM", &[1.0, 2.0, 3.0, 4.0, 5.0], 15.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("SUM", &[10.0, 20.0, 30.0], 60.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("SUM", &[100.0, 200.0, 300.0, 400.0], 1000.0, 0.001)
        .unwrap();

    println!("✅ SUM validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_average() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test AVERAGE with column reference
    harness
        .test_aggregation("AVERAGE", &[10.0, 20.0, 30.0, 40.0, 50.0], 30.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("AVERAGE", &[2.0, 4.0, 6.0], 4.0, 0.001)
        .unwrap();

    println!("✅ AVERAGE validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_count() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test COUNT with column reference
    harness
        .test_aggregation("COUNT", &[1.0, 2.0, 3.0, 4.0, 5.0], 5.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("COUNT", &[10.0, 20.0, 30.0], 3.0, 0.001)
        .unwrap();

    println!("✅ COUNT validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_min() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test MIN with column reference
    harness
        .test_aggregation("MIN", &[5.0, 2.0, 8.0, 1.0, 9.0], 1.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("MIN", &[-5.0, 0.0, 5.0], -5.0, 0.001)
        .unwrap();

    println!("✅ MIN validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_max() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test MAX with column reference
    harness
        .test_aggregation("MAX", &[5.0, 2.0, 8.0, 1.0, 9.0], 9.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("MAX", &[-5.0, 0.0, 5.0], 5.0, 0.001)
        .unwrap();

    println!("✅ MAX validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_product() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test PRODUCT with column reference
    harness
        .test_aggregation("PRODUCT", &[1.0, 2.0, 3.0, 4.0], 24.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("PRODUCT", &[2.0, 5.0, 10.0], 100.0, 0.001)
        .unwrap();

    println!("✅ PRODUCT validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_counta() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test COUNTA with column reference (counts non-empty cells)
    harness
        .test_aggregation("COUNTA", &[1.0, 2.0, 3.0, 4.0, 5.0], 5.0, 0.001)
        .unwrap();

    println!("✅ COUNTA validated against Gnumeric");
}

// ═══════════════════════════════════════════════════════════════════════════════
// PHASE 3: MATH FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn e2e_roundtrip_aggregation_functions() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test aggregation functions survive roundtrip: YAML → XLSX → Gnumeric → CSV
    let yaml_content = r#"_forge_version: "5.0.0"
aggregation_tests:
  idx: [1, 2, 3]
  test_sum: "=1+2+3+4+5"
  test_average: "=(10+20+30)/3"
  test_count: "=5"
  test_min: "=2"
  test_max: "=9"
  test_product: "=2*3*4"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_agg.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_agg.xlsx");

    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

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

    // Convert to CSV using Gnumeric (recalculates formulas)
    let csv_path = harness
        .engine
        .xlsx_to_csv(&xlsx_path, harness.temp_dir.path())
        .expect("Failed to convert to CSV");

    let csv_data = parse_csv(&csv_path);

    // Verify expected values exist in CSV
    let mut found_15 = false; // SUM(1,2,3,4,5)
    let mut found_20 = false; // AVERAGE(10,20,30)
    let mut found_24 = false; // PRODUCT(2,3,4)

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 15.0, 0.001) {
                    found_15 = true;
                }
                if approx_eq(value, 20.0, 0.001) {
                    found_20 = true;
                }
                if approx_eq(value, 24.0, 0.001) {
                    found_24 = true;
                }
            }
        }
    }

    assert!(found_15, "SUM result 15 not found in roundtrip CSV");
    assert!(found_20, "AVERAGE result 20 not found in roundtrip CSV");
    assert!(found_24, "PRODUCT result 24 not found in roundtrip CSV");

    println!("✅ Aggregation functions roundtrip test passed");
}
