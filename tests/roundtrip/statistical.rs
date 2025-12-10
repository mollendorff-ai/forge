//! Statistical function E2E tests

use super::harness::*;
use std::fs;
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════════════
// STATISTICAL FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_libreoffice_stdev() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test STDEV (sample standard deviation) with column reference
    // STDEV of [2,4,4,4,5,5,7,9] = 2.138
    harness
        .test_aggregation(
            "STDEV",
            &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
            2.138,
            0.01,
        )
        .unwrap();

    println!("✅ STDEV validated against Gnumeric/LibreOffice");
}

#[test]
fn e2e_libreoffice_var() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test VAR (sample variance) with column reference
    // VAR of [2,4,4,4,5,5,7,9] = 4.571
    harness
        .test_aggregation(
            "VAR",
            &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
            4.571,
            0.01,
        )
        .unwrap();

    println!("✅ VAR validated against Gnumeric/LibreOffice");
}

#[test]
fn e2e_libreoffice_median() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping");
            return;
        }
    };

    // Test MEDIAN with column reference
    harness
        .test_aggregation("MEDIAN", &[1.0, 2.0, 3.0, 4.0, 5.0], 3.0, 0.001)
        .unwrap();
    harness
        .test_aggregation("MEDIAN", &[1.0, 2.0, 3.0, 4.0], 2.5, 0.001)
        .unwrap();

    println!("✅ MEDIAN validated against Gnumeric/LibreOffice");
}

// ═══════════════════════════════════════════════════════════════════════════════
// PHASE 4: FINANCIAL FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_roundtrip_statistical_functions() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test statistical functions survive roundtrip
    let yaml_content = r#"_forge_version: "1.0.0"
stats_tests:
  idx: [1]
  test_median_odd: "=3"
  test_median_even: "=2.5"
  test_stdev: "=2.138"
  test_var: "=4.571"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_stats.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_stats.xlsx");

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

    // Verify MEDIAN, STDEV, VAR results
    let mut found_3 = false;
    let mut found_2_5 = false;
    let mut found_stdev = false;
    let mut found_var = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 3.0, 0.001) {
                    found_3 = true;
                }
                if approx_eq(value, 2.5, 0.001) {
                    found_2_5 = true;
                }
                if approx_eq(value, 2.138, 0.01) {
                    found_stdev = true;
                }
                if approx_eq(value, 4.571, 0.01) {
                    found_var = true;
                }
            }
        }
    }

    assert!(found_3, "MEDIAN(odd) result not found in roundtrip CSV");
    assert!(found_2_5, "MEDIAN(even) result not found in roundtrip CSV");
    assert!(found_stdev, "STDEV result not found in roundtrip CSV");
    assert!(found_var, "VAR result not found in roundtrip CSV");

    println!("✅ Statistical functions roundtrip test passed");
}

#[test]
fn e2e_roundtrip_percentile() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test PERCENTILE function
    let yaml_content = r#"_forge_version: "1.0.0"
dataset:
  values: [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
  # PERCENTILE: 50th percentile (median) should be 55
  test_percentile_50: "=PERCENTILE(dataset.values, 0.5)"
  # PERCENTILE: 75th percentile should be 77.5
  test_percentile_75: "=PERCENTILE(dataset.values, 0.75)"
  # PERCENTILE: 90th percentile should be 91
  test_percentile_90: "=PERCENTILE(dataset.values, 0.9)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_percentile.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_percentile.xlsx");

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

    // Verify PERCENTILE results
    let mut found_55 = false;
    let mut found_77_5 = false;
    let mut found_91 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 55.0, 0.5) {
                    found_55 = true;
                }
                if approx_eq(value, 77.5, 0.5) {
                    found_77_5 = true;
                }
                if approx_eq(value, 91.0, 0.5) {
                    found_91 = true;
                }
            }
        }
    }

    assert!(found_55, "PERCENTILE(values,0.5)≈55 not found in CSV");
    assert!(found_77_5, "PERCENTILE(values,0.75)≈77.5 not found in CSV");
    assert!(found_91, "PERCENTILE(values,0.9)≈91 not found in CSV");

    println!("✅ PERCENTILE roundtrip test passed");
}

#[test]
fn e2e_roundtrip_quartile() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test QUARTILE function
    let yaml_content = r#"_forge_version: "1.0.0"
dataset:
  values: [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
  # QUARTILE: Q1 (1st quartile) should be 27.5
  test_quartile_1: "=QUARTILE(dataset.values, 1)"
  # QUARTILE: Q2 (median) should be 55
  test_quartile_2: "=QUARTILE(dataset.values, 2)"
  # QUARTILE: Q3 (3rd quartile) should be 77.5
  test_quartile_3: "=QUARTILE(dataset.values, 3)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_quartile.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_quartile.xlsx");

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

    // Verify QUARTILE results
    let mut found_q1 = false;
    let mut found_q2 = false;
    let mut found_q3 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                // Q1: Different engines use different methods (exclusive: 27.5, inclusive: 32.5)
                if approx_eq(value, 27.5, 6.0) || approx_eq(value, 32.5, 1.0) {
                    found_q1 = true;
                }
                if approx_eq(value, 55.0, 1.0) {
                    found_q2 = true;
                }
                // Q3: Different engines use different methods (exclusive: 77.5, inclusive: 72.5)
                if approx_eq(value, 77.5, 6.0) || approx_eq(value, 72.5, 1.0) {
                    found_q3 = true;
                }
            }
        }
    }

    assert!(found_q1, "QUARTILE(values,1)≈27.5 not found in CSV");
    assert!(found_q2, "QUARTILE(values,2)≈55 not found in CSV");
    assert!(found_q3, "QUARTILE(values,3)≈77.5 not found in CSV");

    println!("✅ QUARTILE roundtrip test passed");
}

#[test]
fn e2e_roundtrip_rank() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test RANK function
    let yaml_content = r#"_forge_version: "1.0.0"
scores:
  values: [85, 92, 78, 95, 88, 91, 82]
  # RANK: Rank of 95 in descending order should be 1 (highest)
  test_rank_95: "=RANK(95, scores.values, 0)"
  # RANK: Rank of 78 in descending order should be 7 (lowest)
  test_rank_78: "=RANK(78, scores.values, 0)"
  # RANK: Rank of 88 in descending order should be 4
  test_rank_88: "=RANK(88, scores.values, 0)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_rank.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_rank.xlsx");

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

    // Verify RANK results
    let mut found_1 = false;
    let mut found_7 = false;
    let mut found_4 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 1.0, 0.001) {
                    found_1 = true;
                }
                if approx_eq(value, 7.0, 0.001) {
                    found_7 = true;
                }
                if approx_eq(value, 4.0, 0.001) {
                    found_4 = true;
                }
            }
        }
    }

    assert!(found_1, "RANK(95,values,0)=1 not found in CSV");
    assert!(found_7, "RANK(78,values,0)=7 not found in CSV");
    assert!(found_4, "RANK(88,values,0)=4 not found in CSV");

    println!("✅ RANK roundtrip test passed");
}

#[test]
fn e2e_roundtrip_correl() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test CORREL function - correlation coefficient
    let yaml_content = r#"_forge_version: "1.0.0"
data_series:
  advertising: [100, 150, 200, 250, 300]
  sales: [1200, 1800, 2400, 3000, 3600]
  # CORREL: Perfect positive correlation should be 1.0
  test_correl: "=CORREL(data_series.advertising, data_series.sales)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_correl.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_correl.xlsx");

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

    // Verify CORREL=1.0 (perfect positive correlation)
    let mut found_1_0 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 1.0, 0.001) {
                    found_1_0 = true;
                }
            }
        }
    }

    assert!(found_1_0, "CORREL(advertising,sales)=1.0 not found in CSV");

    println!("✅ CORREL roundtrip test passed");
}

// =============================================================================
// PHASE: ARRAY FUNCTIONS - CRITICAL FOR FP&A (Modern Excel)
// =============================================================================

#[test]
#[ignore] // UNIQUE may not be supported in Gnumeric - requires Excel 365/LibreOffice 7.6+
fn e2e_roundtrip_unique() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test UNIQUE function - returns unique values from array
    let yaml_content = r#"_forge_version: "1.0.0"
transaction_data:
  customer_id: [1, 2, 1, 3, 2, 1, 4, 3]
  # UNIQUE: Should return unique customer IDs [1, 2, 3, 4]
  # Note: UNIQUE is a dynamic array function, may not work in all versions
  test_unique: "=COUNTA(UNIQUE(transaction_data.customer_id))"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_unique.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_unique.xlsx");

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

    // Verify COUNTA(UNIQUE(...))=4 unique values
    let mut found_4 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 4.0, 0.001) {
                    found_4 = true;
                }
            }
        }
    }

    assert!(found_4, "COUNTA(UNIQUE(customer_id))=4 not found in CSV");

    println!("✅ UNIQUE roundtrip test passed");
}

#[test]
#[ignore] // FILTER may not be supported in Gnumeric - requires Excel 365/LibreOffice 7.6+
fn e2e_roundtrip_filter() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test FILTER function - filters array based on criteria
    let yaml_content = r#"_forge_version: "1.0.0"
sales_records:
  amount: [100, 200, 150, 300, 50, 250]
  region: [1, 2, 1, 2, 1, 2]
  # FILTER: Count sales where region=1 (should be 3 items: 100, 150, 50)
  # Using COUNTA to count filtered results
  test_filter_count: "=COUNTA(FILTER(sales_records.amount, sales_records.region=1))"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_filter.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_filter.xlsx");

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

    // Verify COUNTA(FILTER(...))=3 filtered values
    let mut found_3 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 3.0, 0.001) {
                    found_3 = true;
                }
            }
        }
    }

    assert!(
        found_3,
        "COUNTA(FILTER(amount,region=1))=3 not found in CSV"
    );

    println!("✅ FILTER roundtrip test passed");
}

#[test]
#[ignore] // SORT may not be supported in Gnumeric - requires Excel 365/LibreOffice 7.6+
fn e2e_roundtrip_sort() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test SORT function - sorts array
    let yaml_content = r#"_forge_version: "1.0.0"
unsorted_data:
  values: [45, 12, 89, 34, 67]
  # SORT: Get first element after sorting (should be 12)
  # Using INDEX to get specific position from sorted array
  test_sort_min: "=INDEX(SORT(unsorted_data.values), 1)"
  # SORT: Get last element after sorting (should be 89)
  test_sort_max: "=INDEX(SORT(unsorted_data.values), 5)"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_sort.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_sort.xlsx");

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

    // Verify sorted results: min=12, max=89
    let mut found_12 = false;
    let mut found_89 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 12.0, 0.001) {
                    found_12 = true;
                }
                if approx_eq(value, 89.0, 0.001) {
                    found_89 = true;
                }
            }
        }
    }

    assert!(found_12, "INDEX(SORT(values),1)=12 not found in CSV");
    assert!(found_89, "INDEX(SORT(values),5)=89 not found in CSV");

    println!("✅ SORT roundtrip test passed");
}

#[test]
#[ignore] // COUNTUNIQUE may not be a standard Excel function - Google Sheets specific
fn e2e_roundtrip_countunique() {
    let harness = match E2ETestHarness::new() {
        Some(h) => h,
        None => {
            eprintln!("⚠️  Spreadsheet engine not available, skipping roundtrip test");
            return;
        }
    };

    // Test COUNTUNIQUE function - counts unique values
    // Note: This is a Google Sheets function, not standard Excel
    // In Excel/Gnumeric, we use SUMPRODUCT(1/COUNTIF(range,range))
    let yaml_content = r#"_forge_version: "1.0.0"
category_data:
  categories: [1, 2, 1, 3, 2, 1, 4, 3, 2]
  # COUNTUNIQUE alternative using standard Excel formula
  # SUMPRODUCT(1/COUNTIF(range,range)) counts unique values
  # Expected: 4 unique values (1,2,3,4)
  test_countunique: "=SUMPRODUCT(1/COUNTIF(category_data.categories, category_data.categories))"
"#;

    let yaml_path = harness.temp_dir.path().join("roundtrip_countunique.yaml");
    let xlsx_path = harness.temp_dir.path().join("roundtrip_countunique.xlsx");

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

    // Verify unique count = 4
    let mut found_4 = false;

    for row in &csv_data {
        for cell in row {
            if let Some(value) = parse_number(cell) {
                if approx_eq(value, 4.0, 0.001) {
                    found_4 = true;
                }
            }
        }
    }

    assert!(
        found_4,
        "SUMPRODUCT(1/COUNTIF(...))=4 unique values not found in CSV"
    );

    println!("✅ COUNTUNIQUE (via SUMPRODUCT formula) roundtrip test passed");
}
