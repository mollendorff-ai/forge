#[cfg(feature = "full")]
use royalbit_forge::core::ArrayCalculator;
#[cfg(feature = "full")]
use royalbit_forge::types::{ParsedModel, Variable};
#[cfg(feature = "full")]
use std::f64::consts::PI;

#[cfg(feature = "full")]
fn eval_scalar(formula: &str) -> f64 {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some(format!("={}", formula))),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    result.scalars.get("result").unwrap().value.unwrap()
}

#[cfg(feature = "full")]
fn approx_eq(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

// ============================================================================
// SIN FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_sin_zero() {
    let result = eval_scalar("SIN(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "SIN(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_sin_pi_over_6() {
    let result = eval_scalar(&format!("SIN({})", PI / 6.0));
    assert!(approx_eq(result, 0.5, 1e-10), "SIN(PI/6) should be 0.5");
}

#[cfg(feature = "full")]
#[test]
fn test_sin_pi_over_4() {
    let result = eval_scalar(&format!("SIN({})", PI / 4.0));
    assert!(
        approx_eq(result, 2.0_f64.sqrt() / 2.0, 1e-10),
        "SIN(PI/4) should be sqrt(2)/2"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_sin_pi_over_2() {
    let result = eval_scalar(&format!("SIN({})", PI / 2.0));
    assert!(approx_eq(result, 1.0, 1e-10), "SIN(PI/2) should be 1");
}

#[cfg(feature = "full")]
#[test]
fn test_sin_pi() {
    let result = eval_scalar(&format!("SIN({})", PI));
    assert!(
        approx_eq(result, 0.0, 1e-10),
        "SIN(PI) should be 0 (approximately)"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_sin_negative() {
    let result = eval_scalar(&format!("SIN({})", -PI / 6.0));
    assert!(approx_eq(result, -0.5, 1e-10), "SIN(-PI/6) should be -0.5");
}

// ============================================================================
// COS FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_cos_zero() {
    let result = eval_scalar("COS(0)");
    assert!(approx_eq(result, 1.0, 1e-10), "COS(0) should be 1");
}

#[cfg(feature = "full")]
#[test]
fn test_cos_pi_over_3() {
    let result = eval_scalar(&format!("COS({})", PI / 3.0));
    assert!(approx_eq(result, 0.5, 1e-10), "COS(PI/3) should be 0.5");
}

#[cfg(feature = "full")]
#[test]
fn test_cos_pi_over_2() {
    let result = eval_scalar(&format!("COS({})", PI / 2.0));
    assert!(
        approx_eq(result, 0.0, 1e-10),
        "COS(PI/2) should be 0 (approximately)"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_cos_pi() {
    let result = eval_scalar(&format!("COS({})", PI));
    assert!(approx_eq(result, -1.0, 1e-10), "COS(PI) should be -1");
}

#[cfg(feature = "full")]
#[test]
fn test_cos_2pi() {
    let result = eval_scalar(&format!("COS({})", 2.0 * PI));
    assert!(approx_eq(result, 1.0, 1e-10), "COS(2*PI) should be 1");
}

#[cfg(feature = "full")]
#[test]
fn test_cos_negative() {
    let result = eval_scalar(&format!("COS({})", -PI / 3.0));
    assert!(approx_eq(result, 0.5, 1e-10), "COS(-PI/3) should be 0.5");
}

// ============================================================================
// TAN FUNCTION TESTS (5 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_tan_zero() {
    let result = eval_scalar("TAN(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "TAN(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_tan_pi_over_4() {
    let result = eval_scalar(&format!("TAN({})", PI / 4.0));
    assert!(approx_eq(result, 1.0, 1e-10), "TAN(PI/4) should be 1");
}

#[cfg(feature = "full")]
#[test]
fn test_tan_pi_over_6() {
    let result = eval_scalar(&format!("TAN({})", PI / 6.0));
    assert!(
        approx_eq(result, 1.0 / 3.0_f64.sqrt(), 1e-10),
        "TAN(PI/6) should be 1/sqrt(3)"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_tan_negative() {
    let result = eval_scalar(&format!("TAN({})", -PI / 4.0));
    assert!(approx_eq(result, -1.0, 1e-10), "TAN(-PI/4) should be -1");
}

#[cfg(feature = "full")]
#[test]
fn test_tan_pi() {
    let result = eval_scalar(&format!("TAN({})", PI));
    assert!(
        approx_eq(result, 0.0, 1e-10),
        "TAN(PI) should be approximately 0"
    );
}

// ============================================================================
// ASIN FUNCTION TESTS (7 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_asin_zero() {
    let result = eval_scalar("ASIN(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "ASIN(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_asin_half() {
    let result = eval_scalar("ASIN(0.5)");
    assert!(
        approx_eq(result, PI / 6.0, 1e-10),
        "ASIN(0.5) should be PI/6"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_asin_one() {
    let result = eval_scalar("ASIN(1)");
    assert!(approx_eq(result, PI / 2.0, 1e-10), "ASIN(1) should be PI/2");
}

#[cfg(feature = "full")]
#[test]
fn test_asin_negative_one() {
    let result = eval_scalar("ASIN(-1)");
    assert!(
        approx_eq(result, -PI / 2.0, 1e-10),
        "ASIN(-1) should be -PI/2"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_asin_negative_half() {
    let result = eval_scalar("ASIN(-0.5)");
    assert!(
        approx_eq(result, -PI / 6.0, 1e-10),
        "ASIN(-0.5) should be -PI/6"
    );
}

#[cfg(feature = "full")]
#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_asin_domain_error_greater_than_one() {
    let _result = eval_scalar("ASIN(1.5)");
    // This should panic in eval_scalar because ASIN returns an error
}

#[cfg(feature = "full")]
#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_asin_domain_error_less_than_negative_one() {
    let _result = eval_scalar("ASIN(-1.5)");
    // This should panic in eval_scalar because ASIN returns an error
}

// ============================================================================
// ACOS FUNCTION TESTS (7 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_acos_zero() {
    let result = eval_scalar("ACOS(0)");
    assert!(approx_eq(result, PI / 2.0, 1e-10), "ACOS(0) should be PI/2");
}

#[cfg(feature = "full")]
#[test]
fn test_acos_half() {
    let result = eval_scalar("ACOS(0.5)");
    assert!(
        approx_eq(result, PI / 3.0, 1e-10),
        "ACOS(0.5) should be PI/3"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_acos_one() {
    let result = eval_scalar("ACOS(1)");
    assert!(approx_eq(result, 0.0, 1e-10), "ACOS(1) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_acos_negative_one() {
    let result = eval_scalar("ACOS(-1)");
    assert!(approx_eq(result, PI, 1e-10), "ACOS(-1) should be PI");
}

#[cfg(feature = "full")]
#[test]
fn test_acos_negative_half() {
    let result = eval_scalar("ACOS(-0.5)");
    assert!(
        approx_eq(result, 2.0 * PI / 3.0, 1e-10),
        "ACOS(-0.5) should be 2*PI/3"
    );
}

#[cfg(feature = "full")]
#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_acos_domain_error_greater_than_one() {
    let _result = eval_scalar("ACOS(2.0)");
    // This should panic in eval_scalar because ACOS returns an error
}

#[cfg(feature = "full")]
#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_acos_domain_error_less_than_negative_one() {
    let _result = eval_scalar("ACOS(-2.0)");
    // This should panic in eval_scalar because ACOS returns an error
}

// ============================================================================
// ATAN FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_atan_zero() {
    let result = eval_scalar("ATAN(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "ATAN(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_atan_one() {
    let result = eval_scalar("ATAN(1)");
    assert!(approx_eq(result, PI / 4.0, 1e-10), "ATAN(1) should be PI/4");
}

#[cfg(feature = "full")]
#[test]
fn test_atan_negative_one() {
    let result = eval_scalar("ATAN(-1)");
    assert!(
        approx_eq(result, -PI / 4.0, 1e-10),
        "ATAN(-1) should be -PI/4"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_atan_sqrt_3() {
    let result = eval_scalar(&format!("ATAN({})", 3.0_f64.sqrt()));
    assert!(
        approx_eq(result, PI / 3.0, 1e-10),
        "ATAN(sqrt(3)) should be PI/3"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_atan_large_positive() {
    let result = eval_scalar("ATAN(1000000)");
    assert!(
        approx_eq(result, PI / 2.0, 1e-5),
        "ATAN(large positive) should approach PI/2"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_atan_large_negative() {
    let result = eval_scalar("ATAN(-1000000)");
    assert!(
        approx_eq(result, -PI / 2.0, 1e-5),
        "ATAN(large negative) should approach -PI/2"
    );
}

// ============================================================================
// SINH FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_sinh_zero() {
    let result = eval_scalar("SINH(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "SINH(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_sinh_one() {
    let result = eval_scalar("SINH(1)");
    let expected = (1.0_f64.exp() - (-1.0_f64).exp()) / 2.0;
    assert!(approx_eq(result, expected, 1e-10), "SINH(1) should match");
}

#[cfg(feature = "full")]
#[test]
fn test_sinh_negative_one() {
    let result = eval_scalar("SINH(-1)");
    let expected = ((-1.0_f64).exp() - 1.0_f64.exp()) / 2.0;
    assert!(approx_eq(result, expected, 1e-10), "SINH(-1) should match");
}

#[cfg(feature = "full")]
#[test]
fn test_sinh_large_positive() {
    let result = eval_scalar("SINH(5)");
    let expected = (5.0_f64.exp() - (-5.0_f64).exp()) / 2.0;
    assert!(
        approx_eq(result, expected, 1e-6),
        "SINH(5) should match expected value"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_sinh_large_negative() {
    let result = eval_scalar("SINH(-5)");
    let expected = ((-5.0_f64).exp() - 5.0_f64.exp()) / 2.0;
    assert!(
        approx_eq(result, expected, 1e-6),
        "SINH(-5) should match expected value"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_sinh_small_value() {
    let result = eval_scalar("SINH(0.1)");
    let expected = (0.1_f64.exp() - (-0.1_f64).exp()) / 2.0;
    assert!(approx_eq(result, expected, 1e-10), "SINH(0.1) should match");
}

// ============================================================================
// COSH FUNCTION TESTS (5 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_cosh_zero() {
    let result = eval_scalar("COSH(0)");
    assert!(approx_eq(result, 1.0, 1e-10), "COSH(0) should be 1");
}

#[cfg(feature = "full")]
#[test]
fn test_cosh_one() {
    let result = eval_scalar("COSH(1)");
    let expected = (1.0_f64.exp() + (-1.0_f64).exp()) / 2.0;
    assert!(approx_eq(result, expected, 1e-10), "COSH(1) should match");
}

#[cfg(feature = "full")]
#[test]
fn test_cosh_negative_one() {
    let result = eval_scalar("COSH(-1)");
    let expected = ((-1.0_f64).exp() + 1.0_f64.exp()) / 2.0;
    assert!(
        approx_eq(result, expected, 1e-10),
        "COSH(-1) should match (same as COSH(1))"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_cosh_large_value() {
    let result = eval_scalar("COSH(5)");
    let expected = (5.0_f64.exp() + (-5.0_f64).exp()) / 2.0;
    assert!(
        approx_eq(result, expected, 1e-6),
        "COSH(5) should match expected value"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_cosh_symmetry() {
    let pos_result = eval_scalar("COSH(2)");
    let neg_result = eval_scalar("COSH(-2)");
    assert!(
        approx_eq(pos_result, neg_result, 1e-10),
        "COSH should be symmetric: COSH(2) = COSH(-2)"
    );
}

// ============================================================================
// TANH FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_tanh_zero() {
    let result = eval_scalar("TANH(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "TANH(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_tanh_one() {
    let result = eval_scalar("TANH(1)");
    let expected = (1.0_f64.exp() - (-1.0_f64).exp()) / (1.0_f64.exp() + (-1.0_f64).exp());
    assert!(approx_eq(result, expected, 1e-10), "TANH(1) should match");
}

#[cfg(feature = "full")]
#[test]
fn test_tanh_negative_one() {
    let result = eval_scalar("TANH(-1)");
    let expected = ((-1.0_f64).exp() - 1.0_f64.exp()) / ((-1.0_f64).exp() + 1.0_f64.exp());
    assert!(approx_eq(result, expected, 1e-10), "TANH(-1) should match");
}

#[cfg(feature = "full")]
#[test]
fn test_tanh_large_positive_approaches_one() {
    let result = eval_scalar("TANH(10)");
    assert!(approx_eq(result, 1.0, 1e-8), "TANH(10) should approach 1");
}

#[cfg(feature = "full")]
#[test]
fn test_tanh_large_negative_approaches_negative_one() {
    let result = eval_scalar("TANH(-10)");
    assert!(
        approx_eq(result, -1.0, 1e-8),
        "TANH(-10) should approach -1"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_tanh_small_value() {
    let result = eval_scalar("TANH(0.1)");
    let expected = (0.1_f64.exp() - (-0.1_f64).exp()) / (0.1_f64.exp() + (-0.1_f64).exp());
    assert!(approx_eq(result, expected, 1e-10), "TANH(0.1) should match");
}

// ============================================================================
// RADIANS FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_radians_zero() {
    let result = eval_scalar("RADIANS(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "RADIANS(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_radians_90() {
    let result = eval_scalar("RADIANS(90)");
    assert!(
        approx_eq(result, PI / 2.0, 1e-10),
        "RADIANS(90) should be PI/2"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_radians_180() {
    let result = eval_scalar("RADIANS(180)");
    assert!(approx_eq(result, PI, 1e-10), "RADIANS(180) should be PI");
}

#[cfg(feature = "full")]
#[test]
fn test_radians_360() {
    let result = eval_scalar("RADIANS(360)");
    assert!(
        approx_eq(result, 2.0 * PI, 1e-10),
        "RADIANS(360) should be 2*PI"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_radians_negative() {
    let result = eval_scalar("RADIANS(-90)");
    assert!(
        approx_eq(result, -PI / 2.0, 1e-10),
        "RADIANS(-90) should be -PI/2"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_radians_45() {
    let result = eval_scalar("RADIANS(45)");
    assert!(
        approx_eq(result, PI / 4.0, 1e-10),
        "RADIANS(45) should be PI/4"
    );
}

// ============================================================================
// DEGREES FUNCTION TESTS (6 tests)
// ============================================================================

#[cfg(feature = "full")]
#[test]
fn test_degrees_zero() {
    let result = eval_scalar("DEGREES(0)");
    assert!(approx_eq(result, 0.0, 1e-10), "DEGREES(0) should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_degrees_pi_over_2() {
    let result = eval_scalar(&format!("DEGREES({})", PI / 2.0));
    assert!(approx_eq(result, 90.0, 1e-10), "DEGREES(PI/2) should be 90");
}

#[cfg(feature = "full")]
#[test]
fn test_degrees_pi() {
    let result = eval_scalar(&format!("DEGREES({})", PI));
    assert!(approx_eq(result, 180.0, 1e-10), "DEGREES(PI) should be 180");
}

#[cfg(feature = "full")]
#[test]
fn test_degrees_2pi() {
    let result = eval_scalar(&format!("DEGREES({})", 2.0 * PI));
    assert!(
        approx_eq(result, 360.0, 1e-10),
        "DEGREES(2*PI) should be 360"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_degrees_negative() {
    let result = eval_scalar(&format!("DEGREES({})", -PI / 2.0));
    assert!(
        approx_eq(result, -90.0, 1e-10),
        "DEGREES(-PI/2) should be -90"
    );
}

#[cfg(feature = "full")]
#[test]
fn test_degrees_pi_over_4() {
    let result = eval_scalar(&format!("DEGREES({})", PI / 4.0));
    assert!(approx_eq(result, 45.0, 1e-10), "DEGREES(PI/4) should be 45");
}
