//! Trigonometric function tests for ArrayCalculator
//! Complete coverage for all 11 trig functions
//!
//! These tests require the "full" feature flag as trig functions are enterprise-only.

#![cfg(not(feature = "demo"))]
#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════════════
// SIN - Sine function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sin_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "angles".to_string(),
        ColumnValue::Number(vec![0.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0]),
    ));
    table.add_row_formula("sine".to_string(), "=SIN(angles)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let sine = result_table.columns.get("sine").unwrap();
    match &sine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // sin(0) = 0
            assert!((nums[1] - 0.5).abs() < 0.0001); // sin(π/6) = 0.5
            assert!((nums[2] - 0.7071).abs() < 0.001); // sin(π/4) ≈ 0.7071
            assert!((nums[3] - 0.866).abs() < 0.001); // sin(π/3) ≈ 0.866
            assert!((nums[4] - 1.0).abs() < 0.0001); // sin(π/2) = 1
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_sin_negative_angles() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "angles".to_string(),
        ColumnValue::Number(vec![-PI / 2.0, -PI / 4.0, 0.0]),
    ));
    table.add_row_formula("sine".to_string(), "=SIN(angles)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let sine = result_table.columns.get("sine").unwrap();
    match &sine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - (-1.0)).abs() < 0.0001); // sin(-π/2) = -1
            assert!((nums[1] - (-0.7071)).abs() < 0.001); // sin(-π/4) ≈ -0.7071
            assert!((nums[2] - 0.0).abs() < 0.0001); // sin(0) = 0
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_sin_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "angle".to_string(),
        Variable::new("angle".to_string(), Some(PI / 2.0), None),
    );
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SIN(angle)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 1.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// COS - Cosine function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cos_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "angles".to_string(),
        ColumnValue::Number(vec![0.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0]),
    ));
    table.add_row_formula("cosine".to_string(), "=COS(angles)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let cosine = result_table.columns.get("cosine").unwrap();
    match &cosine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 1.0).abs() < 0.0001); // cos(0) = 1
            assert!((nums[1] - 0.866).abs() < 0.001); // cos(π/6) ≈ 0.866
            assert!((nums[2] - 0.7071).abs() < 0.001); // cos(π/4) ≈ 0.7071
            assert!((nums[3] - 0.5).abs() < 0.0001); // cos(π/3) = 0.5
            assert!((nums[4] - 0.0).abs() < 0.0001); // cos(π/2) ≈ 0
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_cos_pi() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COS(3.141592653589793)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - (-1.0)).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// TAN - Tangent function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_tan_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "angles".to_string(),
        ColumnValue::Number(vec![0.0, PI / 6.0, PI / 4.0, PI / 3.0]),
    ));
    table.add_row_formula("tangent".to_string(), "=TAN(angles)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let tangent = result_table.columns.get("tangent").unwrap();
    match &tangent.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // tan(0) = 0
            assert!((nums[1] - 0.577).abs() < 0.001); // tan(π/6) ≈ 0.577
            assert!((nums[2] - 1.0).abs() < 0.0001); // tan(π/4) = 1
            assert!((nums[3] - 1.732).abs() < 0.001); // tan(π/3) ≈ 1.732
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_tan_negative() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=TAN(-0.7853981633974483)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - (-1.0)).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// ASIN - Arcsine function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_asin_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 0.5, 1.0, -0.5, -1.0]),
    ));
    table.add_row_formula("arcsine".to_string(), "=ASIN(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let arcsine = result_table.columns.get("arcsine").unwrap();
    match &arcsine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // asin(0) = 0
            assert!((nums[1] - PI / 6.0).abs() < 0.0001); // asin(0.5) = π/6
            assert!((nums[2] - PI / 2.0).abs() < 0.0001); // asin(1) = π/2
            assert!((nums[3] - (-PI / 6.0)).abs() < 0.0001); // asin(-0.5) = -π/6
            assert!((nums[4] - (-PI / 2.0)).abs() < 0.0001); // asin(-1) = -π/2
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_asin_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=ASIN(0.5)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - PI / 6.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// ACOS - Arccosine function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_acos_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 0.5, 0.0, -0.5, -1.0]),
    ));
    table.add_row_formula("arccosine".to_string(), "=ACOS(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let arccosine = result_table.columns.get("arccosine").unwrap();
    match &arccosine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // acos(1) = 0
            assert!((nums[1] - PI / 3.0).abs() < 0.0001); // acos(0.5) = π/3
            assert!((nums[2] - PI / 2.0).abs() < 0.0001); // acos(0) = π/2
            assert!((nums[3] - 2.0 * PI / 3.0).abs() < 0.0001); // acos(-0.5) = 2π/3
            assert!((nums[4] - PI).abs() < 0.0001); // acos(-1) = π
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_acos_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=ACOS(0)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - PI / 2.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// ATAN - Arctangent function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_atan_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, -1.0, 1000.0, -1000.0]),
    ));
    table.add_row_formula("arctangent".to_string(), "=ATAN(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let arctangent = result_table.columns.get("arctangent").unwrap();
    match &arctangent.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // atan(0) = 0
            assert!((nums[1] - PI / 4.0).abs() < 0.0001); // atan(1) = π/4
            assert!((nums[2] - (-PI / 4.0)).abs() < 0.0001); // atan(-1) = -π/4
            assert!((nums[3] - PI / 2.0).abs() < 0.001); // atan(∞) ≈ π/2
            assert!((nums[4] - (-PI / 2.0)).abs() < 0.001); // atan(-∞) ≈ -π/2
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_atan_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=ATAN(1)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - PI / 4.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// SINH - Hyperbolic sine (Enterprise only)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_sinh_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, -1.0, 2.0]),
    ));
    table.add_row_formula("hyperbolic_sine".to_string(), "=SINH(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let hyperbolic_sine = result_table.columns.get("hyperbolic_sine").unwrap();
    match &hyperbolic_sine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // sinh(0) = 0
            assert!((nums[1] - 1.1752).abs() < 0.001); // sinh(1) ≈ 1.1752
            assert!((nums[2] - (-1.1752)).abs() < 0.001); // sinh(-1) ≈ -1.1752
            assert!((nums[3] - 3.6269).abs() < 0.001); // sinh(2) ≈ 3.6269
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_sinh_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SINH(1)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 1.1752).abs() < 0.001);
}

// ═══════════════════════════════════════════════════════════════════════════
// COSH - Hyperbolic cosine (Enterprise only)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_cosh_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, -1.0, 2.0]),
    ));
    table.add_row_formula("hyperbolic_cosine".to_string(), "=COSH(values)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let hyperbolic_cosine = result_table.columns.get("hyperbolic_cosine").unwrap();
    match &hyperbolic_cosine.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 1.0).abs() < 0.0001); // cosh(0) = 1
            assert!((nums[1] - 1.5431).abs() < 0.001); // cosh(1) ≈ 1.5431
            assert!((nums[2] - 1.5431).abs() < 0.001); // cosh(-1) ≈ 1.5431 (even)
            assert!((nums[3] - 3.7622).abs() < 0.001); // cosh(2) ≈ 3.7622
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_cosh_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=COSH(0)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 1.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// TANH - Hyperbolic tangent (Enterprise only)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_tanh_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, -1.0, 10.0]),
    ));
    table.add_row_formula(
        "hyperbolic_tangent".to_string(),
        "=TANH(values)".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let hyperbolic_tangent = result_table.columns.get("hyperbolic_tangent").unwrap();
    match &hyperbolic_tangent.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // tanh(0) = 0
            assert!((nums[1] - 0.7616).abs() < 0.001); // tanh(1) ≈ 0.7616
            assert!((nums[2] - (-0.7616)).abs() < 0.001); // tanh(-1) ≈ -0.7616
            assert!((nums[3] - 1.0).abs() < 0.0001); // tanh(10) ≈ 1
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_tanh_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TANH(1)".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 0.7616).abs() < 0.001);
}

// ═══════════════════════════════════════════════════════════════════════════
// RADIANS - Degrees to radians conversion (Enterprise only)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_radians_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "degrees".to_string(),
        ColumnValue::Number(vec![0.0, 45.0, 90.0, 180.0, 360.0]),
    ));
    table.add_row_formula("radians".to_string(), "=RADIANS(degrees)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let radians = result_table.columns.get("radians").unwrap();
    match &radians.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // 0° = 0 rad
            assert!((nums[1] - PI / 4.0).abs() < 0.0001); // 45° = π/4 rad
            assert!((nums[2] - PI / 2.0).abs() < 0.0001); // 90° = π/2 rad
            assert!((nums[3] - PI).abs() < 0.0001); // 180° = π rad
            assert!((nums[4] - 2.0 * PI).abs() < 0.0001); // 360° = 2π rad
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_radians_negative() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=RADIANS(-90)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - (-PI / 2.0)).abs() < 0.0001);
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_radians_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=RADIANS(180)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - PI).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// DEGREES - Radians to degrees conversion (Enterprise only)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_degrees_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "radians".to_string(),
        ColumnValue::Number(vec![0.0, PI / 4.0, PI / 2.0, PI, 2.0 * PI]),
    ));
    table.add_row_formula("degrees".to_string(), "=DEGREES(radians)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let result_table = result.tables.get("data").unwrap();

    let degrees = result_table.columns.get("degrees").unwrap();
    match &degrees.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 0.0).abs() < 0.0001); // 0 rad = 0°
            assert!((nums[1] - 45.0).abs() < 0.0001); // π/4 rad = 45°
            assert!((nums[2] - 90.0).abs() < 0.0001); // π/2 rad = 90°
            assert!((nums[3] - 180.0).abs() < 0.0001); // π rad = 180°
            assert!((nums[4] - 360.0).abs() < 0.0001); // 2π rad = 360°
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_degrees_negative() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DEGREES(-1.5707963267948966)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - (-90.0)).abs() < 0.0001);
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_degrees_scalar() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DEGREES(3.141592653589793)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("result").unwrap().value.unwrap() - 180.0).abs() < 0.0001);
}

// ═══════════════════════════════════════════════════════════════════════════
// Combined trig identity tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_trig_identity_sin_cos() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "angle".to_string(),
        Variable::new("angle".to_string(), Some(0.5), None),
    );
    model.add_scalar(
        "sin_val".to_string(),
        Variable::new("sin_val".to_string(), None, Some("=SIN(angle)".to_string())),
    );
    model.add_scalar(
        "cos_val".to_string(),
        Variable::new("cos_val".to_string(), None, Some("=COS(angle)".to_string())),
    );
    model.add_scalar(
        "identity".to_string(),
        Variable::new(
            "identity".to_string(),
            None,
            Some("=POWER(sin_val, 2) + POWER(cos_val, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // sin²(x) + cos²(x) = 1
    assert!((result.scalars.get("identity").unwrap().value.unwrap() - 1.0).abs() < 0.0001);
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_radians_degrees_roundtrip() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "original".to_string(),
        Variable::new("original".to_string(), Some(45.0), None),
    );
    model.add_scalar(
        "to_rad".to_string(),
        Variable::new(
            "to_rad".to_string(),
            None,
            Some("=RADIANS(original)".to_string()),
        ),
    );
    model.add_scalar(
        "back_to_deg".to_string(),
        Variable::new(
            "back_to_deg".to_string(),
            None,
            Some("=DEGREES(to_rad)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("back_to_deg").unwrap().value.unwrap() - 45.0).abs() < 0.0001);
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_sinh_cosh_identity() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "x".to_string(),
        Variable::new("x".to_string(), Some(2.0), None),
    );
    model.add_scalar(
        "cosh_val".to_string(),
        Variable::new("cosh_val".to_string(), None, Some("=COSH(x)".to_string())),
    );
    model.add_scalar(
        "sinh_val".to_string(),
        Variable::new("sinh_val".to_string(), None, Some("=SINH(x)".to_string())),
    );
    model.add_scalar(
        "identity".to_string(),
        Variable::new(
            "identity".to_string(),
            None,
            Some("=POWER(cosh_val, 2) - POWER(sinh_val, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // cosh²(x) - sinh²(x) = 1
    assert!((result.scalars.get("identity").unwrap().value.unwrap() - 1.0).abs() < 0.0001);
}
