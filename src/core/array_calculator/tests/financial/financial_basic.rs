//! Basic financial function tests (PMT, FV, PV, NPV, NPER, RATE)

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_pmt_function() {
    use crate::types::Variable;

    // Test PMT: Monthly payment for $100,000 loan at 6% annual for 30 years
    // PMT(0.005, 360, 100000) = -599.55 (monthly payment)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "monthly_rate".to_string(),
        Variable::new("monthly_rate".to_string(), Some(0.005), None), // 6% annual / 12 months
    );
    model.add_scalar(
        "periods".to_string(),
        Variable::new("periods".to_string(), Some(360.0), None), // 30 years * 12 months
    );
    model.add_scalar(
        "loan_amount".to_string(),
        Variable::new("loan_amount".to_string(), Some(100000.0), None),
    );
    model.add_scalar(
        "payment".to_string(),
        Variable::new(
            "payment".to_string(),
            None,
            Some("=PMT(monthly_rate, periods, loan_amount)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let payment = result.scalars.get("payment").unwrap().value.unwrap();

    // PMT should be around -599.55
    assert!(
        (payment - (-599.55)).abs() < 0.1,
        "PMT should be around -599.55, got {payment}"
    );
}

#[test]
fn test_fv_function() {
    use crate::types::Variable;

    // Test FV: Future value of $1000/month at 5% annual for 10 years
    // FV(0.05/12, 120, -1000) = ~155,282
    let mut model = ParsedModel::new();
    model.add_scalar(
        "future_value".to_string(),
        Variable::new(
            "future_value".to_string(),
            None,
            Some("=FV(0.004166667, 120, -1000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let fv = result.scalars.get("future_value").unwrap().value.unwrap();

    // FV should be around 155,282
    assert!(
        fv > 155000.0 && fv < 156000.0,
        "FV should be around 155,282, got {fv}"
    );
}

#[test]
fn test_pv_function() {
    use crate::types::Variable;

    // Test PV: Present value of $500/month for 5 years at 8% annual
    // PV(0.08/12, 60, -500) = ~24,588
    let mut model = ParsedModel::new();
    model.add_scalar(
        "present_value".to_string(),
        Variable::new(
            "present_value".to_string(),
            None,
            Some("=PV(0.006666667, 60, -500)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let pv = result.scalars.get("present_value").unwrap().value.unwrap();

    // PV should be around 24,588
    assert!(
        pv > 24000.0 && pv < 25000.0,
        "PV should be around 24,588, got {pv}"
    );
}

#[test]
fn test_npv_function() {
    use crate::types::Variable;

    // Test NPV: Net present value of cash flows (Excel-style: all values discounted from period 1)
    // NPV(0.10, -1000, 300, 400, 500, 600) = ~353.43
    // Note: Excel's NPV discounts ALL values starting from period 1
    // For traditional investment NPV where initial investment is at period 0:
    // Use: =initial_investment + NPV(rate, future_cash_flows)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "npv_result".to_string(),
        Variable::new(
            "npv_result".to_string(),
            None,
            Some("=NPV(0.10, -1000, 300, 400, 500, 600)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let npv = result.scalars.get("npv_result").unwrap().value.unwrap();

    // NPV should be around 353.43 (Excel-style calculation)
    assert!(
        (npv - 353.43).abs() < 1.0,
        "NPV should be around 353.43, got {npv}"
    );
}

#[test]
fn test_nper_function() {
    use crate::types::Variable;

    // Test NPER: How many months to pay off $10,000 at 5% with $200/month
    // NPER(0.05/12, -200, 10000) = ~55.5 months
    let mut model = ParsedModel::new();
    model.add_scalar(
        "num_periods".to_string(),
        Variable::new(
            "num_periods".to_string(),
            None,
            Some("=NPER(0.004166667, -200, 10000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let nper = result.scalars.get("num_periods").unwrap().value.unwrap();

    // NPER should be around 55.5
    assert!(
        nper > 50.0 && nper < 60.0,
        "NPER should be around 55.5, got {nper}"
    );
}

#[test]
fn test_rate_function() {
    use crate::types::Variable;

    // Test RATE: What rate pays off $10,000 in 60 months at $200/month?
    // RATE(60, -200, 10000) = ~0.00655 (monthly), ~7.9% annual
    let mut model = ParsedModel::new();
    model.add_scalar(
        "interest_rate".to_string(),
        Variable::new(
            "interest_rate".to_string(),
            None,
            Some("=RATE(60, -200, 10000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let rate = result.scalars.get("interest_rate").unwrap().value.unwrap();

    // Monthly rate should be around 0.00655
    assert!(
        rate > 0.005 && rate < 0.01,
        "RATE should be around 0.00655, got {rate}"
    );
}

#[test]
fn test_npv_with_negative_cashflows() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("cashflows".to_string());
    // Investment (negative) followed by returns
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "discount_rate".to_string(),
        Variable::new("discount_rate".to_string(), Some(0.10), None),
    );
    model.add_scalar(
        "net_pv".to_string(),
        Variable::new(
            "net_pv".to_string(),
            None,
            Some("=NPV(discount_rate, cashflows.amounts)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let npv = result.scalars.get("net_pv").unwrap().value.unwrap();
    // NPV should be calculated (positive or negative depending on discount rate)
    assert!(npv.is_finite());
}

#[test]
fn test_npv_calculation() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("cashflows".to_string());
    data.add_column(Column::new(
        "cf".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "npv".to_string(),
        Variable::new(
            "npv".to_string(),
            None,
            Some("=NPV(0.1, cashflows.cf)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // NPV calculation should succeed
    assert!(result.is_ok());
}

#[test]
fn test_pmt_calculation() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    // PMT(rate, nper, pv)
    model.add_scalar(
        "payment".to_string(),
        Variable::new(
            "payment".to_string(),
            None,
            Some("=PMT(0.05/12, 360, 200000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_fv_calculation() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    // FV(rate, nper, pmt, pv)
    model.add_scalar(
        "future_value".to_string(),
        Variable::new(
            "future_value".to_string(),
            None,
            Some("=FV(0.05, 10, -100, -1000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_pv_calculation() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    // PV(rate, nper, pmt)
    model.add_scalar(
        "present_value".to_string(),
        Variable::new(
            "present_value".to_string(),
            None,
            Some("=PV(0.08, 20, 500)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_npv_with_boolean_column() {
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "paid".to_string(),
        ColumnValue::Boolean(vec![true, false, true, true]),
    ));
    model.add_table(cashflows);

    use crate::types::Variable;
    model.add_scalar(
        "npv_bool".to_string(),
        Variable::new(
            "npv_bool".to_string(),
            None,
            Some("=NPV(0.1, cf.paid)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should calculate NPV with booleans");

    let npv = result.scalars.get("npv_bool").unwrap().value.unwrap();
    // Booleans convert to 1.0 and 0.0, so NPV should be calculated
    // NPV(0.1, [1.0, 0.0, 1.0, 1.0]) with all discounted
    assert!(
        npv > 0.0 && npv < 5.0,
        "NPV with booleans should be between 0 and 5, got {npv}"
    );
}

#[test]
fn test_npv_with_date_column_error() {
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(),
            "2024-06-01".to_string(),
            "2024-12-01".to_string(),
        ]),
    ));
    model.add_table(cashflows);

    use crate::types::Variable;
    model.add_scalar(
        "npv_date".to_string(),
        Variable::new(
            "npv_date".to_string(),
            None,
            Some("=NPV(0.1, cf.dates)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("NPV with date column should succeed");

    let npv = result.scalars.get("npv_date").unwrap().value.unwrap();
    // NPV with Date column converts dates to numeric values (likely 0.0 or based on date parsing)
    assert!(
        npv.is_finite(),
        "NPV with date column should return finite value, got {npv}"
    );
}

#[test]
fn test_pmt_function_coverage() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "payment".to_string(),
        Variable::new(
            "payment".to_string(),
            None,
            Some("=PMT(0.08/12, 360, -200000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("PMT calculation should succeed");

    let payment = result.scalars.get("payment").unwrap().value.unwrap();
    // PMT(0.08/12, 360, -200000) should return monthly payment around $1,467
    assert!(
        (payment - 1467.0).abs() < 10.0,
        "PMT should be around 1467, got {payment}"
    );
}

#[test]
fn test_fv_function_coverage() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "future_val".to_string(),
        Variable::new(
            "future_val".to_string(),
            None,
            Some("=FV(0.05, 10, -100, -1000)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("FV calculation should succeed");

    let fv = result.scalars.get("future_val").unwrap().value.unwrap();
    // FV(0.05, 10, -100, -1000) should return future value around $2,886
    assert!(
        fv > 2500.0 && fv < 3500.0,
        "FV should be around 2886, got {fv}"
    );
}

#[test]
fn test_pv_function_coverage() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "present_val".to_string(),
        Variable::new(
            "present_val".to_string(),
            None,
            Some("=PV(0.08, 20, -500)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("PV calculation should succeed");

    let pv = result.scalars.get("present_val").unwrap().value.unwrap();
    // PV(0.08, 20, -500) should return present value around $4,909
    assert!(
        pv > 4500.0 && pv < 5500.0,
        "PV should be around 4909, got {pv}"
    );
}

#[test]
fn test_rate_basic() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "r".to_string(),
        Variable::new(
            "r".to_string(),
            None,
            Some("=RATE(60, -1000, 50000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("RATE calculation should succeed");

    let rate = result.scalars.get("r").unwrap().value.unwrap();
    // RATE(60, -1000, 50000) should return a positive interest rate around 0.6%
    assert!(
        rate > 0.0 && rate < 0.02,
        "RATE should be around 0.006, got {rate}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS FOR PMT, FV, PV, NPER, RATE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_pmt_zero_rate() {
    // PMT with rate=0: simple division of principal over periods
    let mut model = ParsedModel::new();
    model.add_scalar(
        "payment".to_string(),
        Variable::new(
            "payment".to_string(),
            None,
            Some("=PMT(0, 12, 1200)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let payment = result.scalars.get("payment").unwrap().value.unwrap();
    assert!(
        (payment - (-100.0)).abs() < 0.01,
        "PMT(0,12,1200) should be -100, got {payment}"
    );
}

#[test]
fn test_pmt_type_beginning_of_period() {
    // PMT with type=1 (payment at beginning of period)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "payment_end".to_string(),
        Variable::new(
            "payment_end".to_string(),
            None,
            Some("=PMT(0.01, 12, 1000, 0, 0)".to_string()),
        ),
    );
    model.add_scalar(
        "payment_begin".to_string(),
        Variable::new(
            "payment_begin".to_string(),
            None,
            Some("=PMT(0.01, 12, 1000, 0, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let end_pmt = result.scalars.get("payment_end").unwrap().value.unwrap();
    let begin_pmt = result.scalars.get("payment_begin").unwrap().value.unwrap();
    assert!(
        begin_pmt.abs() < end_pmt.abs(),
        "Beginning payment {begin_pmt} should be less than end {end_pmt}"
    );
}

#[test]
fn test_fv_zero_rate() {
    // FV with rate=0: simple sum of payments
    let mut model = ParsedModel::new();
    model.add_scalar(
        "future_value".to_string(),
        Variable::new(
            "future_value".to_string(),
            None,
            Some("=FV(0, 12, -100, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let fv = result.scalars.get("future_value").unwrap().value.unwrap();
    assert!(
        (fv - 1200.0).abs() < 0.01,
        "FV(0,12,-100,0) should be 1200, got {fv}"
    );
}

#[test]
fn test_pv_zero_rate() {
    // PV with rate=0: simple sum of payments
    let mut model = ParsedModel::new();
    model.add_scalar(
        "present_value".to_string(),
        Variable::new(
            "present_value".to_string(),
            None,
            Some("=PV(0, 12, -100)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let pv = result.scalars.get("present_value").unwrap().value.unwrap();
    assert!(
        (pv - 1200.0).abs() < 0.01,
        "PV(0,12,-100) should be 1200, got {pv}"
    );
}

#[test]
fn test_pv_with_future_value() {
    // PV with optional fv argument
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pv_with_fv".to_string(),
        Variable::new(
            "pv_with_fv".to_string(),
            None,
            Some("=PV(0.01, 12, -100, 500)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let pv = result.scalars.get("pv_with_fv").unwrap().value.unwrap();
    assert!(
        pv > 500.0 && pv < 2000.0,
        "PV with fv should be reasonable, got {pv}"
    );
}

#[test]
fn test_pmt_negative_present_value() {
    // PMT with negative PV (investment vs loan)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "payment".to_string(),
        Variable::new(
            "payment".to_string(),
            None,
            Some("=PMT(0.01, 12, -1000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let payment = result.scalars.get("payment").unwrap().value.unwrap();
    // Negative PV means we're receiving money, payment should be positive (paying back)
    assert!(
        payment > 0.0,
        "PMT with negative PV should be positive, got {payment}"
    );
}

#[test]
fn test_fv_negative_payment() {
    // FV with negative payment (receiving vs paying)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "future_value".to_string(),
        Variable::new(
            "future_value".to_string(),
            None,
            Some("=FV(0.05, 10, 100)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let fv = result.scalars.get("future_value").unwrap().value.unwrap();
    // Positive payment (receiving) should result in negative FV (debt)
    assert!(
        fv < 0.0,
        "FV with positive payment should be negative, got {fv}"
    );
}

#[test]
fn test_pv_negative_payment() {
    // PV with positive payment (receiving money each period)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "present_value".to_string(),
        Variable::new(
            "present_value".to_string(),
            None,
            Some("=PV(0.05, 10, 100)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let pv = result.scalars.get("present_value").unwrap().value.unwrap();
    // Positive payment should result in negative PV
    assert!(
        pv < 0.0,
        "PV with positive payment should be negative, got {pv}"
    );
}

#[test]
fn test_nper_negative_payment() {
    // NPER with negative payment (paying off debt)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "periods".to_string(),
        Variable::new(
            "periods".to_string(),
            None,
            Some("=NPER(0.01, -100, 1000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let nper = result.scalars.get("periods").unwrap().value.unwrap();
    // Should calculate number of periods to pay off
    assert!(
        nper > 0.0 && nper < 20.0,
        "NPER with negative payment should be positive, got {nper}"
    );
}

#[test]
fn test_nper_with_future_value() {
    // NPER with future value parameter
    let mut model = ParsedModel::new();
    model.add_scalar(
        "periods".to_string(),
        Variable::new(
            "periods".to_string(),
            None,
            Some("=NPER(0.05, -100, 1000, 500)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let nper = result.scalars.get("periods").unwrap().value.unwrap();
    // FV affects the number of periods
    assert!(
        nper.is_finite(),
        "NPER with FV should be finite, got {nper}"
    );
}

#[test]
fn test_rate_high_payment() {
    // RATE with very high payment relative to PV (low interest rate)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "rate".to_string(),
        Variable::new(
            "rate".to_string(),
            None,
            Some("=RATE(12, -90, 1000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let rate = result.scalars.get("rate").unwrap().value.unwrap();
    // High payment means lower interest rate
    assert!(
        (0.0..0.05).contains(&rate),
        "RATE with high payment should be low, got {rate}"
    );
}

#[test]
fn test_rate_with_future_value() {
    // RATE with future value
    let mut model = ParsedModel::new();
    model.add_scalar(
        "rate".to_string(),
        Variable::new(
            "rate".to_string(),
            None,
            Some("=RATE(12, -100, 1000, 200)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let rate = result.scalars.get("rate").unwrap().value.unwrap();
    // Should converge to a rate
    assert!(
        rate.is_finite(),
        "RATE with FV should be finite, got {rate}"
    );
}

#[test]
fn test_fv_with_present_value() {
    // FV with both payment and present value
    let mut model = ParsedModel::new();
    model.add_scalar(
        "future_value".to_string(),
        Variable::new(
            "future_value".to_string(),
            None,
            Some("=FV(0.05, 10, -100, -1000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let fv = result.scalars.get("future_value").unwrap().value.unwrap();
    // FV should account for both PV and payments
    assert!(
        fv > 2000.0 && fv < 3000.0,
        "FV with PV and PMT should be reasonable, got {fv}"
    );
}

#[test]
fn test_fv_type_beginning_of_period() {
    // FV with type parameter (5th argument)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "fv_with_type".to_string(),
        Variable::new(
            "fv_with_type".to_string(),
            None,
            Some("=FV(0.05, 10, -100, 0, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let fv = result.scalars.get("fv_with_type").unwrap().value.unwrap();
    // FV with 5 parameters should calculate correctly
    assert!(
        fv > 0.0 && fv.is_finite(),
        "FV with type parameter should be positive, got {fv}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// NPV EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_npv_all_negative_cashflows() {
    // NPV with all negative cash flows (all costs, no returns)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![-1000.0, -500.0, -300.0, -200.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "npv_result".to_string(),
        Variable::new(
            "npv_result".to_string(),
            None,
            Some("=NPV(0.10, cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let npv = result.scalars.get("npv_result").unwrap().value.unwrap();
    // All negative cash flows should result in negative NPV
    assert!(
        npv < 0.0,
        "NPV with all negative cash flows should be negative, got {npv}"
    );
    // Manual calculation: -1000/1.1 - 500/1.21 - 300/1.331 - 200/1.4641
    // = -909.09 - 413.22 - 225.39 - 136.60 = -1684.30
    assert!(
        (npv - (-1684.30)).abs() < 1.0,
        "NPV should be around -1684.30, got {npv}"
    );
}

#[test]
fn test_npv_single_cashflow() {
    // NPV with single cash flow
    let mut model = ParsedModel::new();
    model.add_scalar(
        "npv_result".to_string(),
        Variable::new(
            "npv_result".to_string(),
            None,
            Some("=NPV(0.10, 1000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let npv = result.scalars.get("npv_result").unwrap().value.unwrap();
    // Single cash flow: 1000 / 1.1 = 909.09
    assert!(
        (npv - 909.09).abs() < 0.1,
        "NPV with single cash flow should be 909.09, got {npv}"
    );
}

#[test]
fn test_npv_very_high_discount_rate() {
    // NPV with very high discount rate (100%)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![1000.0, 1000.0, 1000.0, 1000.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "npv_result".to_string(),
        Variable::new(
            "npv_result".to_string(),
            None,
            Some("=NPV(1.0, cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let npv = result.scalars.get("npv_result").unwrap().value.unwrap();
    // With 100% discount rate: 1000/2 + 1000/4 + 1000/8 + 1000/16
    // = 500 + 250 + 125 + 62.5 = 937.5
    assert!(
        (npv - 937.5).abs() < 1.0,
        "NPV with 100% discount rate should be around 937.5, got {npv}"
    );
}

#[test]
fn test_npv_zero_discount_rate() {
    // NPV with zero discount rate (simple sum)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "npv_result".to_string(),
        Variable::new(
            "npv_result".to_string(),
            None,
            Some("=NPV(0, cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let npv = result.scalars.get("npv_result").unwrap().value.unwrap();
    // With 0% discount rate, NPV = sum of all cash flows
    assert!(
        (npv - 1000.0).abs() < 0.01,
        "NPV with 0% rate should be 1000, got {npv}"
    );
}
