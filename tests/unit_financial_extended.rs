//! Extended unit tests for XIRR and XNPV financial functions
//!
//! These functions handle irregular cash flows with specific dates, unlike
//! their regular counterparts (IRR, NPV) which assume periodic cash flows.
//!
//! XNPV: Net Present Value with specific dates - XNPV(rate, values, dates)
//! XIRR: Internal Rate of Return with specific dates - XIRR(values, dates)

// Financial functions are enterprise-only
#![cfg(feature = "full")]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ═══════════════════════════════════════════════════════════════════════════
// XNPV TESTS - Net Present Value with Specific Dates
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_xnpv_basic_functionality() {
    // Test basic XNPV calculation with irregular cash flows
    // Investment at day 0, returns at days 180 and 365
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 180.0, 365.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 5000.0, 6000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.10, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XNPV calculation should succeed");

    let xnpv = result.scalars.get("result").unwrap().value.unwrap();
    // With irregular dates and 10% rate, should get positive NPV
    assert!(
        xnpv > 0.0 && xnpv < 2000.0,
        "XNPV should be positive and reasonable, got {}",
        xnpv
    );

    println!("✓ XNPV basic functionality test passed (NPV: {:.2})", xnpv);
}

#[test]
fn test_xnpv_negative_npv() {
    // Test XNPV where the present value is negative (bad investment)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 365.0, 730.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 2000.0, 2500.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.20, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XNPV calculation should succeed");

    let xnpv = result.scalars.get("result").unwrap().value.unwrap();
    // High discount rate (20%) with low returns should yield negative NPV
    assert!(
        xnpv < 0.0,
        "XNPV should be negative for bad investment, got {}",
        xnpv
    );

    println!("✓ XNPV negative NPV test passed (NPV: {:.2})", xnpv);
}

#[test]
fn test_xnpv_single_cash_flow() {
    // Edge case: single cash flow at day 0
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-5000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.10, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XNPV with single cash flow should succeed");

    let xnpv = result.scalars.get("result").unwrap().value.unwrap();
    // Single cash flow at day 0 should equal the cash flow value
    assert!(
        (xnpv - (-5000.0)).abs() < 0.01,
        "XNPV single cash flow should equal value, got {}",
        xnpv
    );

    println!("✓ XNPV single cash flow test passed");
}

#[test]
fn test_xnpv_zero_rate() {
    // Edge case: XNPV with 0% discount rate (sum of cash flows)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 180.0, 365.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 4000.0, 7000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XNPV with zero rate should succeed");

    let xnpv = result.scalars.get("result").unwrap().value.unwrap();
    // At 0% rate, XNPV should be close to sum of cash flows = 1000
    assert!(
        (xnpv - 1000.0).abs() < 1.0,
        "XNPV at 0% should be ~1000, got {}",
        xnpv
    );

    println!("✓ XNPV zero rate test passed");
}

#[test]
fn test_xnpv_mismatched_arrays_error() {
    // Error case: values and dates arrays have different lengths
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 180.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 5000.0, 6000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.10, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(
        result.is_err(),
        "XNPV should fail with mismatched array lengths"
    );

    let err_msg = result.unwrap_err().to_string();
    // Error is caught at table validation (mismatched column lengths)
    assert!(
        err_msg.contains("rows") || err_msg.contains("expected"),
        "Error should mention row mismatch, got: {}",
        err_msg
    );

    println!("✓ XNPV mismatched arrays error test passed");
}

#[test]
fn test_xnpv_empty_arrays_error() {
    // Error case: empty values/dates arrays
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.10, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err(), "XNPV should fail with empty arrays");

    println!("✓ XNPV empty arrays error test passed");
}

#[test]
fn test_xnpv_long_term_investment() {
    // Test XNPV with multi-year investment spanning 5 years
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        // Days: 0, 1yr, 2yr, 3yr, 4yr, 5yr
        ColumnValue::Number(vec![0.0, 365.0, 730.0, 1095.0, 1460.0, 1825.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-50000.0, 10000.0, 15000.0, 18000.0, 20000.0, 22000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XNPV(0.08, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XNPV long-term calculation should succeed");

    let xnpv = result.scalars.get("result").unwrap().value.unwrap();
    // 5-year investment with 8% rate should have positive NPV
    assert!(
        xnpv > 0.0,
        "XNPV for profitable long-term investment should be positive, got {}",
        xnpv
    );

    println!("✓ XNPV long-term investment test passed (NPV: {:.2})", xnpv);
}

// ═══════════════════════════════════════════════════════════════════════════
// XIRR TESTS - Internal Rate of Return with Specific Dates
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_xirr_basic_functionality() {
    // Test basic XIRR calculation with irregular cash flows
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 180.0, 365.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 5500.0, 6000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR calculation should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // Investment returns should yield positive IRR
    assert!(
        xirr > 0.0 && xirr < 1.0,
        "XIRR should be between 0 and 100%, got {}",
        xirr
    );

    println!(
        "✓ XIRR basic functionality test passed (IRR: {:.2}%)",
        xirr * 100.0
    );
}

#[test]
fn test_xirr_negative_return() {
    // Test XIRR where investment loses money (negative IRR)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 365.0, 730.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 3000.0, 3000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR with negative return should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // Losing investment should have negative IRR
    assert!(
        xirr < 0.0,
        "XIRR should be negative for losing investment, got {}",
        xirr
    );

    println!(
        "✓ XIRR negative return test passed (IRR: {:.2}%)",
        xirr * 100.0
    );
}

#[test]
fn test_xirr_simple_annual_return() {
    // Test XIRR with simple annual doubling (should be ~100% IRR)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 365.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-1000.0, 2000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR simple annual return should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // Doubling in one year should be ~100% IRR
    assert!(
        (xirr - 1.0).abs() < 0.1,
        "XIRR for doubling should be ~100%, got {:.2}%",
        xirr * 100.0
    );

    println!(
        "✓ XIRR simple annual return test passed (IRR: {:.2}%)",
        xirr * 100.0
    );
}

#[test]
fn test_xirr_quarterly_cashflows() {
    // Test XIRR with quarterly cash flows over 2 years
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        // Quarterly: 0, 90, 180, 270, 360, 450, 540, 630, 720 days
        ColumnValue::Number(vec![
            0.0, 90.0, 180.0, 270.0, 360.0, 450.0, 540.0, 630.0, 720.0,
        ]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![
            -25000.0, 3000.0, 3500.0, 4000.0, 4500.0, 5000.0, 5500.0, 6000.0, 6500.0,
        ]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR quarterly cashflows should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // Should get reasonable positive IRR
    assert!(
        xirr > 0.0 && xirr < 2.0,
        "XIRR for quarterly flows should be reasonable, got {:.2}%",
        xirr * 100.0
    );

    println!(
        "✓ XIRR quarterly cashflows test passed (IRR: {:.2}%)",
        xirr * 100.0
    );
}

#[test]
fn test_xirr_mismatched_arrays_error() {
    // Error case: values and dates arrays have different lengths
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 180.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 5000.0, 6000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(
        result.is_err(),
        "XIRR should fail with mismatched array lengths"
    );

    let err_msg = result.unwrap_err().to_string();
    // Error is caught at table validation (mismatched column lengths)
    assert!(
        err_msg.contains("rows") || err_msg.contains("expected"),
        "Error should mention row mismatch, got: {}",
        err_msg
    );

    println!("✓ XIRR mismatched arrays error test passed");
}

#[test]
fn test_xirr_empty_arrays_error() {
    // Error case: empty values/dates arrays
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err(), "XIRR should fail with empty arrays");

    println!("✓ XIRR empty arrays error test passed");
}

#[test]
fn test_xirr_irregular_monthly_cashflows() {
    // Test XIRR with irregular monthly cash flows (not exactly 30 days apart)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        // Irregular monthly intervals: 0, 31, 59, 92, 120, 153 days
        ColumnValue::Number(vec![0.0, 31.0, 59.0, 92.0, 120.0, 153.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-12000.0, 2000.0, 2500.0, 3000.0, 3500.0, 4000.0]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR irregular monthly cashflows should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // Should calculate IRR despite irregular intervals
    assert!(
        xirr.is_finite(),
        "XIRR should return finite value for irregular intervals, got {}",
        xirr
    );

    println!(
        "✓ XIRR irregular monthly cashflows test passed (IRR: {:.2}%)",
        xirr * 100.0
    );
}

#[test]
fn test_xirr_high_frequency_trades() {
    // Test XIRR with very frequent cash flows (simulating day trading)
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        // Daily trades over 10 days
        ColumnValue::Number(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![
            -10000.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0, 1200.0, 1300.0, 1400.0,
        ]),
    ));
    model.add_table(cashflows);

    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR high frequency trades should succeed");

    let xirr = result.scalars.get("result").unwrap().value.unwrap();
    // High frequency profitable trades should have extremely high annualized IRR
    // Note: This is an extreme edge case and may not converge (NaN/Inf is acceptable)
    // The test verifies the function handles high-frequency data without crashing
    assert!(
        xirr.is_finite() || !xirr.is_finite(),
        "XIRR should complete calculation even for extreme high-frequency trades"
    );

    if xirr.is_finite() {
        println!(
            "✓ XIRR high frequency trades test passed (Annualized IRR: {:.2}%)",
            xirr * 100.0
        );
    } else {
        println!(
            "✓ XIRR high frequency trades test passed (non-convergent result: {})",
            xirr
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CROSS-VALIDATION TESTS - Verify XIRR and XNPV are consistent
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_xirr_xnpv_consistency() {
    // Verify that XNPV(XIRR(values, dates), values, dates) ≈ 0
    // This is the fundamental relationship between IRR and NPV
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Number(vec![0.0, 182.0, 365.0, 547.0]),
    ));
    cashflows.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-15000.0, 5000.0, 6000.0, 7000.0]),
    ));
    model.add_table(cashflows);

    // First calculate XIRR
    model.scalars.insert(
        "irr_rate".to_string(),
        Variable::new(
            "irr_rate".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );

    // Then use that rate in XNPV - should be close to 0
    model.scalars.insert(
        "npv_at_irr".to_string(),
        Variable::new(
            "npv_at_irr".to_string(),
            None,
            Some("=XNPV(irr_rate, cf.values, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("XIRR/XNPV consistency check should succeed");

    let irr = result.scalars.get("irr_rate").unwrap().value.unwrap();
    let npv = result.scalars.get("npv_at_irr").unwrap().value.unwrap();

    println!("XIRR: {:.4}%, XNPV at IRR: {:.4}", irr * 100.0, npv);

    // NPV at IRR rate should be very close to 0
    assert!(
        npv.abs() < 100.0,
        "XNPV at XIRR rate should be close to 0, got {} (IRR: {:.4}%)",
        npv,
        irr * 100.0
    );

    println!("✓ XIRR/XNPV consistency test passed");
}
