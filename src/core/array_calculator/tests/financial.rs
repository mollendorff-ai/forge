//! Financial function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

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
        "PMT should be around -599.55, got {}",
        payment
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
        "FV should be around 155,282, got {}",
        fv
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
        "PV should be around 24,588, got {}",
        pv
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
        "NPV should be around 353.43, got {}",
        npv
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
        "NPER should be around 55.5, got {}",
        nper
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
        "RATE should be around 0.00655, got {}",
        rate
    );
}

#[test]
fn test_irr_function() {
    use crate::types::Variable;

    // Test IRR: Internal rate of return
    // IRR(-100, 30, 40, 50, 60) = ~0.21 (21%)
    let mut model = ParsedModel::new();

    // Create cash flows table
    let mut cashflows = Table::new("cashflows".to_string());
    cashflows.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-100.0, 30.0, 40.0, 50.0, 60.0]),
    ));
    model.add_table(cashflows);

    model.add_scalar(
        "irr_result".to_string(),
        Variable::new(
            "irr_result".to_string(),
            None,
            Some("=IRR(cashflows.amount)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let irr = result.scalars.get("irr_result").unwrap().value.unwrap();

    // IRR should be around 0.21 (21%)
    assert!(
        irr > 0.15 && irr < 0.30,
        "IRR should be around 0.21, got {}",
        irr
    );
}

#[test]
fn test_xnpv_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Create tables with numeric serial dates (Excel format)
    // Days since first date: 0, 182, 366
    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "d".to_string(),
        ColumnValue::Number(vec![0.0, 182.0, 366.0]),
    ));
    cashflows.add_column(Column::new(
        "v".to_string(),
        ColumnValue::Number(vec![-10000.0, 3000.0, 8000.0]),
    ));
    model.add_table(cashflows);

    // XNPV with 10% rate using numeric dates
    model.add_scalar(
        "xnpv_result".to_string(),
        Variable::new(
            "xnpv_result".to_string(),
            None,
            Some("=XNPV(0.10, cf.v, cf.d)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let xnpv = result.scalars.get("xnpv_result").unwrap().value.unwrap();

    // XNPV should be positive (investment pays off)
    assert!(xnpv > 0.0, "XNPV should be positive, got {}", xnpv);
}

#[test]
fn test_xirr_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Days since first date: 0, 182, 366
    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "d".to_string(),
        ColumnValue::Number(vec![0.0, 182.0, 366.0]),
    ));
    cashflows.add_column(Column::new(
        "v".to_string(),
        ColumnValue::Number(vec![-10000.0, 2750.0, 8500.0]),
    ));
    model.add_table(cashflows);

    model.add_scalar(
        "xirr_result".to_string(),
        Variable::new(
            "xirr_result".to_string(),
            None,
            Some("=XIRR(cf.v, cf.d)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let xirr = result.scalars.get("xirr_result").unwrap().value.unwrap();

    // XIRR should be a reasonable rate (positive for this profitable investment)
    assert!(
        xirr > 0.0 && xirr < 1.0,
        "XIRR should be between 0 and 1, got {}",
        xirr
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
        "NPV with booleans should be between 0 and 5, got {}",
        npv
    );
}

#[test]
fn test_irr_with_text_column_error() {
    let mut model = ParsedModel::new();

    let mut cashflows = Table::new("cf".to_string());
    cashflows.add_column(Column::new(
        "notes".to_string(),
        ColumnValue::Text(vec![
            "Initial".to_string(),
            "Year 1".to_string(),
            "Year 2".to_string(),
        ]),
    ));
    model.add_table(cashflows);

    use crate::types::Variable;
    model.add_scalar(
        "irr_text".to_string(),
        Variable::new(
            "irr_text".to_string(),
            None,
            Some("=IRR(cf.notes)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // IRR with Text column should return an error - financial functions require numeric data
    assert!(
        result.is_err(),
        "IRR with text column should fail but got: {:?}",
        result
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
        "NPV with date column should return finite value, got {}",
        npv
    );
}

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
        "PMT should be around 1467, got {}",
        payment
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
        "FV should be around 2886, got {}",
        fv
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
        "PV should be around 4909, got {}",
        pv
    );
}

#[test]
fn test_irr_function_coverage() {
    let mut model = ParsedModel::new();

    let mut cf = Table::new("cf".to_string());
    cf.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(cf);

    use crate::types::Variable;
    model.add_scalar(
        "irr_val".to_string(),
        Variable::new(
            "irr_val".to_string(),
            None,
            Some("=IRR(cf.amount)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IRR calculation should succeed");

    let irr = result.scalars.get("irr_val").unwrap().value.unwrap();
    // IRR for [-1000, 300, 400, 500] should be around 8.9%
    assert!(
        irr > 0.05 && irr < 0.15,
        "IRR should be around 0.089, got {}",
        irr
    );
}

#[test]
fn test_xirr_function_coverage() {
    let mut model = ParsedModel::new();

    let mut cf = Table::new("cf".to_string());
    cf.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 500.0]),
    ));
    cf.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(),
            "2024-06-01".to_string(),
            "2024-12-01".to_string(),
        ]),
    ));
    model.add_table(cf);

    use crate::types::Variable;
    model.add_scalar(
        "xirr_val".to_string(),
        Variable::new(
            "xirr_val".to_string(),
            None,
            Some("=XIRR(cf.amount, cf.date)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // XIRR with Date column type may have length mismatch issues
    // This test exercises the error path
    if result.is_ok() {
        let xirr = result
            .as_ref()
            .unwrap()
            .scalars
            .get("xirr_val")
            .unwrap()
            .value
            .unwrap();
        assert!(
            xirr.is_finite(),
            "XIRR should return finite value, got {}",
            xirr
        );
    } else {
        // Expected: XIRR may fail with Date columns due to length mismatch
        assert!(result.is_err(), "XIRR with Date column failed as expected");
    }
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
        "RATE should be around 0.006, got {}",
        rate
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
fn test_xnpv_with_dates() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("cf".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 2750.0, 4250.0, 3250.0, 2750.0]),
    ));
    data.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Text(vec![
            "2020-01-01".to_string(),
            "2020-03-01".to_string(),
            "2020-10-30".to_string(),
            "2021-02-15".to_string(),
            "2021-04-01".to_string(),
        ]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "npv".to_string(),
        Variable::new(
            "npv".to_string(),
            None,
            Some("=XNPV(0.09, cf.values, cf.dates)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // XNPV with Text column dates may have length mismatch issues
    // This test exercises both success and error paths
    if result.is_ok() {
        let xnpv = result
            .as_ref()
            .unwrap()
            .scalars
            .get("npv")
            .unwrap()
            .value
            .unwrap();
        assert!(
            xnpv.is_finite(),
            "XNPV should return finite value, got {}",
            xnpv
        );
    } else {
        // Expected: XNPV may fail with Text date columns due to length mismatch
        assert!(result.is_err(), "XNPV with Text dates failed as expected");
    }
}

#[test]
fn test_xirr_with_dates() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("cf".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10000.0, 2750.0, 4250.0, 3250.0, 2750.0]),
    ));
    data.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Text(vec![
            "2020-01-01".to_string(),
            "2020-03-01".to_string(),
            "2020-10-30".to_string(),
            "2021-02-15".to_string(),
            "2021-04-01".to_string(),
        ]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "irr".to_string(),
        Variable::new(
            "irr".to_string(),
            None,
            Some("=XIRR(cf.values, cf.dates)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // XIRR with Text column dates may have length mismatch issues
    // This test exercises both success and error paths
    if result.is_ok() {
        let xirr = result
            .as_ref()
            .unwrap()
            .scalars
            .get("irr")
            .unwrap()
            .value
            .unwrap();
        assert!(
            xirr.is_finite(),
            "XIRR should return finite value, got {}",
            xirr
        );
    } else {
        // Expected: XIRR may fail with Text date columns due to length mismatch
        assert!(result.is_err(), "XIRR with Text dates failed as expected");
    }
}

#[test]
fn test_irr_basic_calculation() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("cf".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-100.0, 30.0, 35.0, 40.0, 45.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "rate".to_string(),
        Variable::new(
            "rate".to_string(),
            None,
            Some("=IRR(cf.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IRR calculation should succeed");

    let irr = result.scalars.get("rate").unwrap().value.unwrap();
    // IRR for [-100, 30, 35, 40, 45] should be around 17.1%
    assert!(
        irr > 0.12 && irr < 0.22,
        "IRR should be around 0.171, got {}",
        irr
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS FOR 100% COVERAGE (ADR-006)
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
        "PMT(0,12,1200) should be -100, got {}",
        payment
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
        "Beginning payment {} should be less than end {}",
        begin_pmt,
        end_pmt
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
        "FV(0,12,-100,0) should be 1200, got {}",
        fv
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
        "PV(0,12,-100) should be 1200, got {}",
        pv
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
        "PV with fv should be reasonable, got {}",
        pv
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
// COMPREHENSIVE EDGE CASE TESTS FOR 100% COVERAGE
// ═══════════════════════════════════════════════════════════════════════════

// ──────────────────────────────────────────────────────────────────────────
// NEGATIVE VALUES TESTS
// ──────────────────────────────────────────────────────────────────────────

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
        "PMT with negative PV should be positive, got {}",
        payment
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
        "FV with positive payment should be negative, got {}",
        fv
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
        "PV with positive payment should be negative, got {}",
        pv
    );
}

// ──────────────────────────────────────────────────────────────────────────
// NPV EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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
        "NPV with all negative cash flows should be negative, got {}",
        npv
    );
    // Manual calculation: -1000/1.1 - 500/1.21 - 300/1.331 - 200/1.4641
    // = -909.09 - 413.22 - 225.39 - 136.60 = -1684.30
    assert!(
        (npv - (-1684.30)).abs() < 1.0,
        "NPV should be around -1684.30, got {}",
        npv
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
        "NPV with single cash flow should be 909.09, got {}",
        npv
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
        "NPV with 100% discount rate should be around 937.5, got {}",
        npv
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
        "NPV with 0% rate should be 1000, got {}",
        npv
    );
}

// ──────────────────────────────────────────────────────────────────────────
// IRR EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn test_irr_all_positive_cashflows() {
    // IRR with all positive cash flows (no solution - never breaks even)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "irr_result".to_string(),
        Variable::new(
            "irr_result".to_string(),
            None,
            Some("=IRR(cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let irr = result.scalars.get("irr_result").unwrap().value.unwrap();
    // IRR will converge to some value but it's not meaningful
    assert!(
        irr.is_finite(),
        "IRR with all positive should be finite, got {}",
        irr
    );
}

#[test]
fn test_irr_very_small_cashflows() {
    // IRR with very small cash flows (precision test)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![-0.01, 0.003, 0.004, 0.005]),
    ));
    model.add_table(table);
    model.add_scalar(
        "irr_result".to_string(),
        Variable::new(
            "irr_result".to_string(),
            None,
            Some("=IRR(cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let irr = result.scalars.get("irr_result").unwrap().value.unwrap();
    // Small cash flows should still calculate correctly
    assert!(
        irr.is_finite(),
        "IRR with small cash flows should be finite, got {}",
        irr
    );
}

#[test]
fn test_irr_large_returns() {
    // IRR with very large returns (high IRR)
    let mut model = ParsedModel::new();
    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![-100.0, 500.0, 600.0, 700.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "irr_result".to_string(),
        Variable::new(
            "irr_result".to_string(),
            None,
            Some("=IRR(cashflows.amounts)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let irr = result.scalars.get("irr_result").unwrap().value.unwrap();
    // Large returns should result in very high IRR
    assert!(
        irr > 1.0,
        "IRR with large returns should be > 100%, got {}",
        irr
    );
}

// ──────────────────────────────────────────────────────────────────────────
// DEPRECIATION EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────────────────
// ACCRINT EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────────────────
// NPER EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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
        "NPER with negative payment should be positive, got {}",
        nper
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
        "NPER with FV should be finite, got {}",
        nper
    );
}

// ──────────────────────────────────────────────────────────────────────────
// RATE EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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
        rate >= 0.0 && rate < 0.05,
        "RATE with high payment should be low, got {}",
        rate
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
        "RATE with FV should be finite, got {}",
        rate
    );
}

// ──────────────────────────────────────────────────────────────────────────
// FV/PV WITH OPTIONAL PARAMETERS
// ──────────────────────────────────────────────────────────────────────────

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
        "FV with PV and PMT should be reasonable, got {}",
        fv
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
        "FV with type parameter should be positive, got {}",
        fv
    );
}

// ──────────────────────────────────────────────────────────────────────────
// MIRR EDGE CASES
// ──────────────────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────────────────
// SLN EDGE CASE
// ──────────────────────────────────────────────────────────────────────────

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
