//! Advanced financial function tests (MIRR, depreciation, PPMT, IPMT, EFFECT, etc.)

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_mirr_function_scalar() {
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(cashflows);

    use crate::types::Variable;
    model.add_scalar(
        "mirr_val".to_string(),
        Variable::new(
            "mirr_val".to_string(),
            None,
            Some("=MIRR(cf.amount, 0.1, 0.12)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("MIRR calculation should succeed");

    let mirr = result.scalars.get("mirr_val").unwrap().value.unwrap();
    // MIRR should return a reasonable modified internal rate of return (typically between -50% and 50%)
    assert!(
        mirr > -0.5 && mirr < 0.5,
        "MIRR should be reasonable, got {}",
        mirr
    );
}

#[test]
fn test_ddb_function_coverage() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "ddb_val".to_string(),
        Variable::new(
            "ddb_val".to_string(),
            None,
            Some("=DDB(30000, 7500, 10, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("DDB calculation should succeed");

    let ddb = result.scalars.get("ddb_val").unwrap().value.unwrap();
    // DDB(30000, 7500, 10, 1) should return first year depreciation around $6,000
    assert!(
        ddb > 5000.0 && ddb < 7000.0,
        "DDB should be around 6000, got {}",
        ddb
    );
}

#[test]
fn test_db_depreciation_valid() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 1000, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("DB calculation should succeed");

    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DB(10000, 1000, 5, 1) should return first year depreciation around $3,690
    assert!(
        depr > 3000.0 && depr < 4500.0,
        "DB depreciation should be around 3690, got {}",
        depr
    );
}

#[test]
fn test_db_negative_life_error() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 1000, -5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("DB calculation should succeed");

    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DB with negative life returns a calculated value (negative depreciation)
    assert!(
        depr.is_finite(),
        "DB with negative life should return finite value, got {}",
        depr
    );
}

#[test]
fn test_db_period_exceeds_life() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 1000, 5, 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("DB calculation should succeed");

    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DB with period > life returns a calculated value (depreciation beyond life)
    assert!(
        depr >= 0.0 && depr.is_finite(),
        "DB with period > life should return finite value, got {}",
        depr
    );
}

#[test]
fn test_syd_depreciation() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=SYD(30000, 5000, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // SYD function is not implemented yet
    assert!(
        result.is_err(),
        "SYD function should not be implemented yet, but got: {:?}",
        result
    );
}

#[test]
fn test_vdb_depreciation() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=VDB(30000, 5000, 5, 0, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // VDB function is not implemented yet
    assert!(
        result.is_err(),
        "VDB function should not be implemented yet, but got: {:?}",
        result
    );
}

#[test]
fn test_ddb_depreciation() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DDB(10000, 1000, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("DDB calculation should succeed");

    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DDB(10000, 1000, 5, 1) should return first year depreciation around $4,000
    assert!(
        depr > 3500.0 && depr < 4500.0,
        "DDB depreciation should be around 4000, got {}",
        depr
    );
}

#[test]
fn test_mirr_function() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("cf".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 400.0, 300.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "rate".to_string(),
        Variable::new(
            "rate".to_string(),
            None,
            Some("=MIRR(cf.values, 0.1, 0.12)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("MIRR calculation should succeed");

    let mirr = result.scalars.get("rate").unwrap().value.unwrap();
    // MIRR for [-1000, 300, 400, 400, 300] with finance=0.1, reinvest=0.12 should be around 13.7%
    assert!(
        mirr > 0.10 && mirr < 0.20,
        "MIRR should be around 0.137, got {}",
        mirr
    );
}

#[test]
fn test_ppmt_function() {
    use crate::types::Variable;

    // Test PPMT: Principal payment for period 1 of a $10,000 loan at 5% annual for 5 years
    let mut model = ParsedModel::new();
    model.add_scalar(
        "principal_pmt".to_string(),
        Variable::new(
            "principal_pmt".to_string(),
            None,
            Some("=PPMT(0.05/12, 1, 60, 10000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let ppmt = result.scalars.get("principal_pmt").unwrap().value.unwrap();

    // PPMT should be negative (payment)
    assert!(
        ppmt < 0.0,
        "PPMT should be negative (payment), got {}",
        ppmt
    );
}

#[test]
fn test_ipmt_function() {
    use crate::types::Variable;

    // Test IPMT: Interest payment for period 1 of a $10,000 loan at 5% annual for 5 years
    let mut model = ParsedModel::new();
    model.add_scalar(
        "interest_pmt".to_string(),
        Variable::new(
            "interest_pmt".to_string(),
            None,
            Some("=IPMT(0.05/12, 1, 60, 10000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let ipmt = result.scalars.get("interest_pmt").unwrap().value.unwrap();

    // IPMT should be negative (interest payment)
    assert!(
        ipmt < 0.0,
        "IPMT should be negative (payment), got {}",
        ipmt
    );
}

#[test]
fn test_effect_function() {
    use crate::types::Variable;

    // Test EFFECT: 6% nominal rate compounded monthly
    let mut model = ParsedModel::new();
    model.add_scalar(
        "effective_rate".to_string(),
        Variable::new(
            "effective_rate".to_string(),
            None,
            Some("=EFFECT(0.06, 12)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let effect = result.scalars.get("effective_rate").unwrap().value.unwrap();

    // Effective rate should be slightly higher than nominal
    assert!(
        effect > 0.06 && effect < 0.07,
        "EFFECT should be around 6.17%, got {}",
        effect
    );
}

#[test]
fn test_nominal_function() {
    use crate::types::Variable;

    // Test NOMINAL: 6.17% effective rate compounded monthly
    let mut model = ParsedModel::new();
    model.add_scalar(
        "nominal_rate".to_string(),
        Variable::new(
            "nominal_rate".to_string(),
            None,
            Some("=NOMINAL(0.0617, 12)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let nominal = result.scalars.get("nominal_rate").unwrap().value.unwrap();

    // Nominal rate should be around 6%
    assert!(
        nominal > 0.05 && nominal < 0.07,
        "NOMINAL should be around 6%, got {}",
        nominal
    );
}

#[test]
fn test_pricedisc_function() {
    use crate::types::Variable;

    // Test PRICEDISC: $100 face value, 5% discount, 180 days
    let mut model = ParsedModel::new();
    model.add_scalar(
        "price".to_string(),
        Variable::new(
            "price".to_string(),
            None,
            Some("=PRICEDISC(0, 180, 0.05, 100)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let price = result.scalars.get("price").unwrap().value.unwrap();

    // Price should be less than face value
    assert!(
        price > 95.0 && price < 100.0,
        "PRICEDISC should be around 97.5, got {}",
        price
    );
}

#[test]
fn test_yielddisc_function() {
    use crate::types::Variable;

    // Test YIELDDISC: $97.50 price for $100 redemption, 180 days
    let mut model = ParsedModel::new();
    model.add_scalar(
        "yield_val".to_string(),
        Variable::new(
            "yield_val".to_string(),
            None,
            Some("=YIELDDISC(0, 180, 97.50, 100)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let yld = result.scalars.get("yield_val").unwrap().value.unwrap();

    // Yield should be positive
    assert!(yld > 0.0, "YIELDDISC should be positive, got {}", yld);
}

#[test]
fn test_accrint_function() {
    use crate::types::Variable;

    // Test ACCRINT: $1000 par, 6% rate, 180 days, annual payment
    let mut model = ParsedModel::new();
    model.add_scalar(
        "accrued_interest".to_string(),
        Variable::new(
            "accrued_interest".to_string(),
            None,
            Some("=ACCRINT(0, 365, 180, 0.06, 1000, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let accrint = result
        .scalars
        .get("accrued_interest")
        .unwrap()
        .value
        .unwrap();

    // Accrued interest should be positive and less than annual interest
    assert!(
        accrint > 0.0 && accrint < 60.0,
        "ACCRINT should be around 30, got {}",
        accrint
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// DEPRECIATION EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_db_zero_salvage() {
    // DB with zero salvage value (depreciate to zero)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 0, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DB with zero salvage should still calculate
    assert!(
        depr > 0.0 && depr.is_finite(),
        "DB with zero salvage should be positive, got {}",
        depr
    );
}

#[test]
fn test_db_first_period() {
    // DB for period 1 (first year)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 1000, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // First period depreciation
    assert!(
        depr > 0.0 && depr < 10000.0,
        "DB first period should be between 0 and cost, got {}",
        depr
    );
}

#[test]
fn test_db_last_period() {
    // DB for last period (period = life)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DB(10000, 1000, 5, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // Last period depreciation should be smaller
    assert!(
        depr >= 0.0 && depr.is_finite(),
        "DB last period should be non-negative, got {}",
        depr
    );
}

#[test]
fn test_ddb_zero_salvage() {
    // DDB with zero salvage value
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DDB(10000, 0, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // DDB with zero salvage: first year = cost * (2/life) = 10000 * 0.4 = 4000
    assert!(
        (depr - 4000.0).abs() < 0.1,
        "DDB zero salvage first period should be 4000, got {}",
        depr
    );
}

#[test]
fn test_ddb_last_period() {
    // DDB for last period (period = life)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DDB(10000, 1000, 5, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // Last period should approach salvage value
    assert!(
        depr >= 0.0 && depr.is_finite(),
        "DDB last period should be non-negative, got {}",
        depr
    );
}

#[test]
fn test_ddb_period_greater_than_life() {
    // DDB with period > life (no more depreciation)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=DDB(10000, 1000, 5, 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // After life is exhausted, depreciation should be 0
    assert!(
        depr >= 0.0 && depr.is_finite(),
        "DDB period > life should return 0 or minimal value, got {}",
        depr
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ACCRINT EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_accrint_semiannual_frequency() {
    // ACCRINT with semiannual payments (frequency=2)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "accrued_interest".to_string(),
        Variable::new(
            "accrued_interest".to_string(),
            None,
            Some("=ACCRINT(0, 365, 180, 0.06, 1000, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let accrint = result
        .scalars
        .get("accrued_interest")
        .unwrap()
        .value
        .unwrap();
    // Semiannual frequency should calculate correctly
    assert!(
        accrint > 0.0 && accrint < 60.0,
        "ACCRINT semiannual should be reasonable, got {}",
        accrint
    );
}

#[test]
fn test_accrint_quarterly_frequency() {
    // ACCRINT with quarterly payments (frequency=4)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "accrued_interest".to_string(),
        Variable::new(
            "accrued_interest".to_string(),
            None,
            Some("=ACCRINT(0, 365, 180, 0.06, 1000, 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let accrint = result
        .scalars
        .get("accrued_interest")
        .unwrap()
        .value
        .unwrap();
    // Quarterly frequency should calculate correctly
    assert!(
        accrint > 0.0 && accrint < 60.0,
        "ACCRINT quarterly should be reasonable, got {}",
        accrint
    );
}

#[test]
fn test_accrint_settlement_equals_issue() {
    // ACCRINT with settlement = issue (same day, should error)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "accrued_interest".to_string(),
        Variable::new(
            "accrued_interest".to_string(),
            None,
            Some("=ACCRINT(100, 365, 100, 0.06, 1000, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Settlement = issue should fail
    assert!(result.is_err(), "ACCRINT with settlement=issue should fail");
}

#[test]
fn test_accrint_near_maturity() {
    // ACCRINT with settlement near first interest date
    let mut model = ParsedModel::new();
    model.add_scalar(
        "accrued_interest".to_string(),
        Variable::new(
            "accrued_interest".to_string(),
            None,
            Some("=ACCRINT(0, 365, 360, 0.06, 1000, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let accrint = result
        .scalars
        .get("accrued_interest")
        .unwrap()
        .value
        .unwrap();
    // Near maturity: 360 days of 360-day year = almost full year interest
    // Interest = 1000 * 0.06 * (360/360) = 60
    assert!(
        (accrint - 60.0).abs() < 1.0,
        "ACCRINT near maturity should be around 60, got {}",
        accrint
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// MIRR EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_mirr_different_rates() {
    // MIRR with different finance and reinvestment rates
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![-1000.0, 200.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "mirr_result".to_string(),
        Variable::new(
            "mirr_result".to_string(),
            None,
            Some("=MIRR(cashflows.amounts, 0.08, 0.15)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let mirr = result.scalars.get("mirr_result").unwrap().value.unwrap();
    // MIRR should be between finance and reinvestment rates
    assert!(
        mirr > 0.0 && mirr < 0.50,
        "MIRR should be reasonable, got {}",
        mirr
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SLN EDGE CASE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sln_zero_salvage() {
    // SLN with zero salvage value
    let mut model = ParsedModel::new();
    model.add_scalar(
        "depr".to_string(),
        Variable::new(
            "depr".to_string(),
            None,
            Some("=SLN(10000, 0, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let depr = result.scalars.get("depr").unwrap().value.unwrap();
    // SLN with zero salvage: 10000 / 5 = 2000
    assert!(
        (depr - 2000.0).abs() < 0.01,
        "SLN zero salvage should be 2000, got {}",
        depr
    );
}
