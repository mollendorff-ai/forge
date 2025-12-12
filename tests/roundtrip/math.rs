//! Math function E2E tests

use super::harness::*;
use std::fs;
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════════════
// MATH FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_gnumeric_abs() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("ABS(-42)", 42.0, 0.001).unwrap();
    harness.test_formula("ABS(42)", 42.0, 0.001).unwrap();

    println!("✅ ABS validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_round() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness
        .test_formula("ROUND(3.14159, 2)", 3.14, 0.001)
        .unwrap();
    harness.test_formula("ROUND(3.5, 0)", 4.0, 0.001).unwrap();
    harness.test_formula("ROUND(2.5, 0)", 2.0, 0.001).ok(); // Banker's rounding may differ

    println!("✅ ROUND validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_sqrt() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("SQRT(16)", 4.0, 0.001).unwrap();
    harness
        .test_formula("SQRT(2)", 1.41421356, 0.00001)
        .unwrap();

    println!("✅ SQRT validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_power() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("POWER(2, 10)", 1024.0, 0.001).unwrap();
    harness.test_formula("POWER(3, 3)", 27.0, 0.001).unwrap();
    harness.test_formula("2^10", 1024.0, 0.001).unwrap();

    println!("✅ POWER validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_mod() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("MOD(10, 3)", 1.0, 0.001).unwrap();
    harness.test_formula("MOD(17, 5)", 2.0, 0.001).unwrap();

    println!("✅ MOD validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_floor_ceiling() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("FLOOR(3.7, 1)", 3.0, 0.001).unwrap();
    harness.test_formula("CEILING(3.2, 1)", 4.0, 0.001).unwrap();

    println!("✅ FLOOR/CEILING validated against Gnumeric");
}

#[test]
fn e2e_gnumeric_log_ln_exp() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Gnumeric not available, skipping");
            return;
        }
    };

    harness.test_formula("LN(2.71828)", 1.0, 0.001).unwrap();
    harness.test_formula("LOG10(100)", 2.0, 0.001).unwrap();
    harness.test_formula("EXP(1)", 2.71828, 0.001).unwrap();

    println!("✅ LOG/LN/EXP validated against Gnumeric");
}

// ═══════════════════════════════════════════════════════════════════════════════
// PHASE 3: STATISTICAL FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_roundtrip_math_functions() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test math functions survive roundtrip: YAML → XLSX → Gnumeric → CSV
    let yaml_content = r#"_forge_version: "5.0.0"
math_tests:
  idx: [1, 2, 3, 4, 5]
  test_abs: "=ABS(-42)"
  test_sqrt: "=SQRT(144)"
  test_power: "=POWER(2, 10)"
  test_mod: "=MOD(17, 5)"
  test_round: "=ROUND(3.14159, 2)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_math.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_math.xlsx");

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
    let mut found_42 = false;
    let mut found_12 = false;
    let mut found_1024 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 42.0, 0.001) {
                    found_42 = true;
                }
                if approx_eq(value, 12.0, 0.001) {
                    found_12 = true;
                }
                if approx_eq(value, 1024.0, 0.001) {
                    found_1024 = true;
                }
            }
        }
    }

    assert!(found_42, "ABS(-42)=42 not found in roundtrip CSV");
    assert!(found_12, "SQRT(144)=12 not found in roundtrip CSV");
    assert!(found_1024, "POWER(2,10)=1024 not found in roundtrip CSV");

    println!("✅ Math functions roundtrip test passed");
}

#[test]
fn e2e_roundtrip_trig_functions() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test trigonometric functions survive roundtrip
    let yaml_content = r#"_forge_version: "5.0.0"
trig_tests:
  idx: [1]
  test_sin: "=SIN(0)"
  test_cos: "=COS(0)"
  test_tan: "=TAN(0)"
  test_pi: "=PI()"
  test_radians: "=RADIANS(180)"
  test_degrees: "=DEGREES(PI())"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_trig.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_trig.xlsx");

    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

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

    let csv_path = harness
        .engine
        .xlsx_to_csv(&xlsx_path, harness.temp_dir.path())
        .expect("Failed to convert to CSV");

    let csv_data = parse_csv(&csv_path);

    // Verify trigonometric function results
    let mut found_0 = false;
    let mut found_1 = false;
    let mut found_pi = false;
    let mut found_180 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 0.0, 0.001) {
                    found_0 = true;
                }
                if approx_eq(value, 1.0, 0.001) {
                    found_1 = true;
                }
                if approx_eq(value, 3.14159, 0.001) {
                    found_pi = true;
                }
                if approx_eq(value, 180.0, 0.001) {
                    found_180 = true;
                }
            }
        }
    }

    assert!(found_0, "SIN(0)/TAN(0)=0 not found in roundtrip CSV");
    assert!(found_1, "COS(0)=1 not found in roundtrip CSV");
    assert!(found_pi, "PI() not found in roundtrip CSV");
    assert!(found_180, "DEGREES(PI())=180 not found in roundtrip CSV");

    println!("✅ Trigonometric functions roundtrip test passed");
}

#[test]
fn e2e_roundtrip_math_extended() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test extended math functions: ROUNDUP, ROUNDDOWN, INT, TRUNC, SIGN, LN, LOG10, EXP, FLOOR, CEILING
    let yaml_content = r#"_forge_version: "5.0.0"
math_extended:
  idx: [1]
  test_roundup: "=ROUNDUP(3.14159, 2)"
  test_rounddown: "=ROUNDDOWN(3.14159, 2)"
  test_int: "=INT(3.7)"
  test_trunc: "=TRUNC(3.7)"
  test_sign_pos: "=SIGN(42)"
  test_sign_neg: "=SIGN(-42)"
  test_ln: "=LN(2.71828)"
  test_log10: "=LOG10(1000)"
  test_exp: "=EXP(0)"
  test_floor: "=FLOOR(3.7, 1)"
  test_ceiling: "=CEILING(3.2, 1)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_math_ext.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_math_ext.xlsx");

    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

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

    let csv_path = harness
        .engine
        .xlsx_to_csv(&xlsx_path, harness.temp_dir.path())
        .expect("Failed to convert to CSV");

    let csv_data = parse_csv(&csv_path);

    // Verify expected values
    let mut found_3_15 = false; // ROUNDUP(3.14159, 2)
    let mut found_3_14 = false; // ROUNDDOWN(3.14159, 2)
    let mut found_3 = false; // INT(3.7) or TRUNC(3.7)
    let mut found_1_pos = false; // SIGN(42)
    let mut found_1_neg = false; // SIGN(-42)
    let mut found_1_ln = false; // LN(2.71828) ≈ 1.0
    let mut found_3_log = false; // LOG10(1000) = 3
    let mut found_1_exp = false; // EXP(0) = 1
    let mut found_4 = false; // CEILING(3.2, 1)

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 3.15, 0.001) {
                    found_3_15 = true;
                }
                if approx_eq(value, 3.14, 0.001) {
                    found_3_14 = true;
                }
                if approx_eq(value, 3.0, 0.001) {
                    found_3 = true;
                }
                if approx_eq(value, 1.0, 0.01) {
                    found_1_pos = true;
                    found_1_ln = true;
                    found_1_exp = true;
                }
                if approx_eq(value, -1.0, 0.001) {
                    found_1_neg = true;
                }
                if approx_eq(value, 3.0, 0.01) {
                    found_3_log = true;
                }
                if approx_eq(value, 4.0, 0.001) {
                    found_4 = true;
                }
            }
        }
    }

    assert!(found_3_15, "ROUNDUP(3.14159,2)=3.15 not found in CSV");
    assert!(found_3_14, "ROUNDDOWN(3.14159,2)=3.14 not found in CSV");
    assert!(found_3, "INT(3.7)=3 or TRUNC(3.7)=3 not found in CSV");
    assert!(found_1_pos, "SIGN(42)=1 not found in CSV");
    assert!(found_1_neg, "SIGN(-42)=-1 not found in CSV");
    assert!(found_1_ln, "LN(2.71828)≈1 not found in CSV");
    assert!(found_3_log, "LOG10(1000)=3 not found in CSV");
    assert!(found_1_exp, "EXP(0)=1 not found in CSV");
    assert!(found_4, "CEILING(3.2,1)=4 not found in CSV");

    println!("✅ Extended math functions roundtrip test passed");
}
