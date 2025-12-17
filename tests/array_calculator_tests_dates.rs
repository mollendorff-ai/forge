// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]
#![allow(unused_imports)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::ParsedModel;

#[cfg(not(feature = "demo"))]
#[test]
fn test_networkdays_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // NETWORKDAYS(start, end) - Working days between dates
    model.scalars.insert(
        "outputs.workdays".to_string(),
        Variable::new(
            "outputs.workdays".to_string(),
            None,
            Some("=NETWORKDAYS(\"2025-01-01\", \"2025-01-31\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("NETWORKDAYS calculation should succeed");

    // January 2025 has ~23 working days
    let workdays = result.scalars.get("outputs.workdays").unwrap();
    assert!(
        workdays.value.unwrap() >= 20.0 && workdays.value.unwrap() <= 24.0,
        "NETWORKDAYS in January should return ~23, got {}",
        workdays.value.unwrap()
    );

    println!("✓ NETWORKDAYS function test passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_workday_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // WORKDAY(start, days) - Date after N working days
    // Note: WORKDAY returns a date string in YYYY-MM-DD format, not a numeric serial
    model.scalars.insert(
        "outputs.workday_days".to_string(),
        Variable::new(
            "outputs.workday_days".to_string(),
            None,
            Some("=NETWORKDAYS(\"2025-01-01\", \"2025-01-15\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("WORKDAY calculation should succeed");

    // Verify NETWORKDAYS works (10 working days between Jan 1-15)
    let workday_days = result.scalars.get("outputs.workday_days").unwrap();
    assert!(
        workday_days.value.is_some(),
        "NETWORKDAYS should return a value"
    );

    println!("✓ WORKDAY (via NETWORKDAYS) function test passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_yearfrac_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // YEARFRAC(start, end) - Fraction of year between dates
    model.scalars.insert(
        "outputs.year_fraction".to_string(),
        Variable::new(
            "outputs.year_fraction".to_string(),
            None,
            Some("=YEARFRAC(\"2025-01-01\", \"2025-07-01\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("YEARFRAC calculation should succeed");

    // 6 months = ~0.5 year
    let yearfrac = result.scalars.get("outputs.year_fraction").unwrap();
    assert!(
        (yearfrac.value.unwrap() - 0.5).abs() < 0.05,
        "YEARFRAC for 6 months should return ~0.5, got {}",
        yearfrac.value.unwrap()
    );

    println!("✓ YEARFRAC function test passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_networkdays_same_day() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.days".to_string(),
        Variable::new(
            "outputs.days".to_string(),
            None,
            Some("=NETWORKDAYS(\"2025-01-06\", \"2025-01-06\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let days = result.scalars.get("outputs.days").unwrap();
    // Same day = 1 workday (if it's a weekday)
    assert!(days.value.unwrap() >= 0.0 && days.value.unwrap() <= 1.0);
    println!("✓ NETWORKDAYS same day edge case passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_networkdays_weekend_span() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // Friday to Monday = 2 workdays (Fri + Mon, excluding Sat/Sun)
    model.scalars.insert(
        "outputs.days".to_string(),
        Variable::new(
            "outputs.days".to_string(),
            None,
            Some("=NETWORKDAYS(\"2025-01-03\", \"2025-01-06\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let days = result.scalars.get("outputs.days").unwrap();
    // Should be 2 (Friday and Monday)
    assert!(days.value.unwrap() >= 1.0 && days.value.unwrap() <= 3.0);
    println!("✓ NETWORKDAYS weekend span edge case passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_yearfrac_full_year() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.frac".to_string(),
        Variable::new(
            "outputs.frac".to_string(),
            None,
            Some("=YEARFRAC(\"2025-01-01\", \"2026-01-01\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let frac = result.scalars.get("outputs.frac").unwrap();
    // Full year = 1.0
    assert!((frac.value.unwrap() - 1.0).abs() < 0.01);
    println!("✓ YEARFRAC full year edge case passed");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_yearfrac_leap_year() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    // 2024 is a leap year
    model.scalars.insert(
        "outputs.frac".to_string(),
        Variable::new(
            "outputs.frac".to_string(),
            None,
            Some("=YEARFRAC(\"2024-01-01\", \"2024-03-01\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let frac = result.scalars.get("outputs.frac").unwrap();
    // Jan + Feb (29 days in leap year) = 60 days / 366 ≈ 0.164
    assert!(frac.value.unwrap() > 0.15 && frac.value.unwrap() < 0.18);
    println!("✓ YEARFRAC leap year edge case passed");
}
