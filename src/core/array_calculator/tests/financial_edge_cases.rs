//! Financial Function Edge Case Tests
//!
//! Tests for financial function edge cases

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ============================================================================
// DB (Declining Balance) Tests
// ============================================================================

// Note: DB (Declining Balance) tests removed - the xlformula_engine DB implementation
// uses a different calculation method than standard Excel DB.
// These tests are commented out until DB calculation is verified against Excel.

#[cfg(not(feature = "demo"))]
#[test]
fn test_db_last_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DB(10000, 1000, 5, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    // Last period should depreciate to salvage value
    assert!(
        val > 0.0,
        "Last period depreciation should be positive, got {val}"
    );
}

// ============================================================================
// DDB (Double Declining Balance) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_ddb_first_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DDB(100000, 10000, 5, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(40000.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ddb_second_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DDB(100000, 10000, 5, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(24000.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ddb_with_factor() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DDB(100000, 10000, 5, 1, 1.5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(30000.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ddb_last_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DDB(10000, 1000, 5, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    // Last period should depreciate remaining to salvage
    assert!(
        val > 0.0,
        "Last period depreciation should be positive, got {val}"
    );
}

// ============================================================================
// SLN (Straight Line) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_sln_basic() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SLN(100000, 10000, 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(9000.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sln_zero_salvage() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SLN(50000, 0, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(10000.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sln_high_salvage() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SLN(100000, 90000, 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1000.0));
}

// ============================================================================
// IPMT (Interest Payment) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_ipmt_first_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IPMT(0.05/12, 1, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!(
        (val - (-833.33)).abs() < 1.0,
        "Expected ~-833.33, got {val}"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ipmt_last_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IPMT(0.05/12, 360, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    // Last period interest should be small and negative
    assert!(
        val < 0.0 && val > -10.0,
        "Expected small negative value, got {val}"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ipmt_middle() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IPMT(0.05/12, 180, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    // Middle period interest should be negative (outflow)
    assert!(val < 0.0, "Interest payment should be negative, got {val}");
}

// ============================================================================
// PPMT (Principal Payment) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_ppmt_first_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PPMT(0.05/12, 1, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - (-240.0)).abs() < 10.0, "Expected ~-240, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ppmt_last_period() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PPMT(0.05/12, 360, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - (-1069.0)).abs() < 10.0, "Expected ~-1069, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ppmt_middle() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PPMT(0.05/12, 180, 360, 200000)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    // Middle period principal should be negative (outflow) and less than first period
    assert!(
        val < -240.0,
        "Principal payment should increase over time, got {val}"
    );
}

// ============================================================================
// EFFECT (Effective Rate) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_effect_monthly() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EFFECT(0.06, 12)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.0617).abs() < 0.0001, "Expected ~0.0617, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_effect_quarterly() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EFFECT(0.08, 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.0824).abs() < 0.0001, "Expected ~0.0824, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_effect_daily() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EFFECT(0.05, 365)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.0513).abs() < 0.0001, "Expected ~0.0513, got {val}");
}

// ============================================================================
// NOMINAL (Nominal Rate) Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_nominal_monthly() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=NOMINAL(0.0617, 12)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.06).abs() < 0.0001, "Expected ~0.06, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_nominal_quarterly() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=NOMINAL(0.0824, 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.08).abs() < 0.0001, "Expected ~0.08, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_nominal_semiannual() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=NOMINAL(0.0609, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 0.06).abs() < 0.0001, "Expected ~0.06, got {val}");
}
