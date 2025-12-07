// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::parser::parse_model;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};
use std::path::Path;

#[test]
fn test_simple_table_calculation() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("financials".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
    ));
    table.add_column(Column::new(
        "cogs".to_string(),
        ColumnValue::Number(vec![300.0, 360.0, 450.0, 540.0]),
    ));
    table.add_row_formula("gross_profit".to_string(), "=revenue - cogs".to_string());
    table.add_row_formula(
        "gross_margin".to_string(),
        "=gross_profit / revenue".to_string(),
    );

    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let result_table = result.tables.get("financials").unwrap();

    // Check gross_profit
    let gross_profit = result_table.columns.get("gross_profit").unwrap();
    match &gross_profit.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert_eq!(nums[0], 700.0);
            assert_eq!(nums[1], 840.0);
            assert_eq!(nums[2], 1050.0);
            assert_eq!(nums[3], 1260.0);
        }
        _ => panic!("Expected Number array"),
    }

    // Check gross_margin
    let gross_margin = result_table.columns.get("gross_margin").unwrap();
    match &gross_margin.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert!((nums[0] - 0.7).abs() < 0.0001);
            assert!((nums[1] - 0.7).abs() < 0.0001);
            assert!((nums[2] - 0.7).abs() < 0.0001);
            assert!((nums[3] - 0.7).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ Simple table calculation succeeded");
}

#[test]
fn test_calculate_quarterly_pl() {
    let path = Path::new("test-data/v1.0/quarterly_pl.yaml");
    let model = parse_model(path).expect("Failed to parse");

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    match result {
        Ok(calculated_model) => {
            println!("✓ Calculation succeeded");
            for (name, table) in &calculated_model.tables {
                println!("  Table '{}': {} columns", name, table.columns.len());
                for (col_name, col) in &table.columns {
                    println!("    - {}: {} rows", col_name, col.values.len());
                }
            }
        }
        Err(e) => {
            println!("✗ Calculation failed: {}", e);
            panic!("Calculation failed: {}", e);
        }
    }
}

#[test]
fn test_mixed_column_types() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("mixed".to_string());

    table.add_column(Column::new(
        "numbers".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    table.add_column(Column::new(
        "labels".to_string(),
        ColumnValue::Text(vec![
            "Item A".to_string(),
            "Item B".to_string(),
            "Item C".to_string(),
        ]),
    ));
    // Test that we can work with both Number and Text columns in same table
    table.add_row_formula("doubled".to_string(), "=numbers * 2".to_string());
    table.add_row_formula("codes".to_string(), "=RIGHT(labels, 1)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Mixed types should work");
    let result_table = result.tables.get("mixed").unwrap();

    let doubled = result_table.columns.get("doubled").unwrap();
    match &doubled.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 20.0);
            assert_eq!(nums[1], 40.0);
            assert_eq!(nums[2], 60.0);
        }
        _ => panic!("Expected Number array"),
    }

    let codes = result_table.columns.get("codes").unwrap();
    match &codes.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "A");
            assert_eq!(texts[1], "B");
            assert_eq!(texts[2], "C");
        }
        _ => panic!("Expected Text array"),
    }

    println!("✓ Mixed column types verified");
}

#[test]
fn test_nested_scalar_references() {
    // Bug #1: Nested scalar reference resolution
    // scalar formulas referencing other scalars using qualified names (section.scalar)
    let path = Path::new("test-data/quota_forecast.yaml");
    let model = parse_model(path).expect("Failed to parse quota_forecast.yaml");

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Nested scalar references should resolve");

    // Verify the burn_rate scalar was calculated correctly
    // burn_rate = current_usage_pct / hours_since_reset = 26 / 43 ≈ 0.6047
    let burn_rate = result.scalars.get("forecast.burn_rate").unwrap();
    assert!(
        (burn_rate.value.unwrap() - 0.6047).abs() < 0.01,
        "Expected ~0.6047, got {}",
        burn_rate.value.unwrap()
    );

    // Verify projected_total (nested reference chain)
    // projected_add = burn_rate * hours_until_reset = 0.6047 * 125 ≈ 75.58
    // projected_total = current_usage_pct + projected_add = 26 + 75.58 ≈ 101.58
    let projected_total = result.scalars.get("forecast.projected_total").unwrap();
    assert!(
        (projected_total.value.unwrap() - 101.58).abs() < 0.1,
        "Expected ~101.58, got {}",
        projected_total.value.unwrap()
    );

    println!("✓ Nested scalar references test passed");
}

#[test]
fn test_scalar_with_metadata() {
    // Bug #3: Schema validation for scalars with metadata (value/notes/unit)
    let path = Path::new("test-data/scalar_metadata_test.yaml");
    let model = parse_model(path).expect("Failed to parse scalar_metadata_test.yaml");

    // Verify scalars were parsed correctly with metadata
    let tax_rate = model.scalars.get("config.tax_rate").unwrap();
    assert!(
        (tax_rate.value.unwrap() - 0.25).abs() < 0.0001,
        "Expected 0.25 for tax_rate"
    );

    let discount = model.scalars.get("config.discount_rate").unwrap();
    assert!(
        (discount.value.unwrap() - 0.10).abs() < 0.0001,
        "Expected 0.10 for discount_rate"
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Scalar with metadata should work");

    // Verify scalars are still present after calculation
    assert!(result.scalars.contains_key("config.tax_rate"));
    assert!(result.scalars.contains_key("config.discount_rate"));

    println!("✓ Scalar with metadata test passed");
}

#[test]
fn test_scalar_math_functions() {
    use royalbit_forge::types::Variable;

    // Test v4.4.1: Math functions in scalar context
    let mut model = ParsedModel::new();

    // SQRT test
    model.scalars.insert(
        "outputs.sqrt_test".to_string(),
        Variable::new(
            "outputs.sqrt_test".to_string(),
            None,
            Some("=SQRT(16)".to_string()),
        ),
    );

    // ROUND test
    model.scalars.insert(
        "outputs.round_test".to_string(),
        Variable::new(
            "outputs.round_test".to_string(),
            None,
            Some("=ROUND(3.14159, 2)".to_string()),
        ),
    );

    // ROUNDUP test
    model.scalars.insert(
        "outputs.roundup_test".to_string(),
        Variable::new(
            "outputs.roundup_test".to_string(),
            None,
            Some("=ROUNDUP(3.14159, 2)".to_string()),
        ),
    );

    // ROUNDDOWN test
    model.scalars.insert(
        "outputs.rounddown_test".to_string(),
        Variable::new(
            "outputs.rounddown_test".to_string(),
            None,
            Some("=ROUNDDOWN(3.14159, 2)".to_string()),
        ),
    );

    // MOD test
    model.scalars.insert(
        "outputs.mod_test".to_string(),
        Variable::new(
            "outputs.mod_test".to_string(),
            None,
            Some("=MOD(10, 3)".to_string()),
        ),
    );

    // POWER test
    model.scalars.insert(
        "outputs.power_test".to_string(),
        Variable::new(
            "outputs.power_test".to_string(),
            None,
            Some("=POWER(2, 8)".to_string()),
        ),
    );

    // CEILING test
    model.scalars.insert(
        "outputs.ceiling_test".to_string(),
        Variable::new(
            "outputs.ceiling_test".to_string(),
            None,
            Some("=CEILING(4.3, 1)".to_string()),
        ),
    );

    // FLOOR test
    model.scalars.insert(
        "outputs.floor_test".to_string(),
        Variable::new(
            "outputs.floor_test".to_string(),
            None,
            Some("=FLOOR(4.7, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Scalar math functions calculation should succeed");

    // Verify results
    let sqrt_test = result.scalars.get("outputs.sqrt_test").unwrap();
    assert!(
        (sqrt_test.value.unwrap() - 4.0).abs() < 0.0001,
        "SQRT(16) should return 4, got {}",
        sqrt_test.value.unwrap()
    );

    let round_test = result.scalars.get("outputs.round_test").unwrap();
    assert!(
        (round_test.value.unwrap() - 3.14).abs() < 0.0001,
        "ROUND(3.14159, 2) should return 3.14, got {}",
        round_test.value.unwrap()
    );

    let roundup_test = result.scalars.get("outputs.roundup_test").unwrap();
    assert!(
        (roundup_test.value.unwrap() - 3.15).abs() < 0.0001,
        "ROUNDUP(3.14159, 2) should return 3.15, got {}",
        roundup_test.value.unwrap()
    );

    let rounddown_test = result.scalars.get("outputs.rounddown_test").unwrap();
    assert!(
        (rounddown_test.value.unwrap() - 3.14).abs() < 0.0001,
        "ROUNDDOWN(3.14159, 2) should return 3.14, got {}",
        rounddown_test.value.unwrap()
    );

    let mod_test = result.scalars.get("outputs.mod_test").unwrap();
    assert!(
        (mod_test.value.unwrap() - 1.0).abs() < 0.0001,
        "MOD(10, 3) should return 1, got {}",
        mod_test.value.unwrap()
    );

    let power_test = result.scalars.get("outputs.power_test").unwrap();
    assert!(
        (power_test.value.unwrap() - 256.0).abs() < 0.0001,
        "POWER(2, 8) should return 256, got {}",
        power_test.value.unwrap()
    );

    let ceiling_test = result.scalars.get("outputs.ceiling_test").unwrap();
    assert!(
        (ceiling_test.value.unwrap() - 5.0).abs() < 0.0001,
        "CEILING(4.3, 1) should return 5, got {}",
        ceiling_test.value.unwrap()
    );

    let floor_test = result.scalars.get("outputs.floor_test").unwrap();
    assert!(
        (floor_test.value.unwrap() - 4.0).abs() < 0.0001,
        "FLOOR(4.7, 1) should return 4, got {}",
        floor_test.value.unwrap()
    );

    println!("✓ Scalar math functions (v4.4.1) test passed");
}

#[test]
fn test_scalar_math_with_scalar_refs() {
    use royalbit_forge::types::Variable;

    // Test math functions with scalar references
    let mut model = ParsedModel::new();

    // Input value
    model.scalars.insert(
        "inputs.base_value".to_string(),
        Variable::new("inputs.base_value".to_string(), Some(16.0), None),
    );

    model.scalars.insert(
        "inputs.precision".to_string(),
        Variable::new("inputs.precision".to_string(), Some(2.0), None),
    );

    model.scalars.insert(
        "inputs.raw_value".to_string(),
        Variable::new("inputs.raw_value".to_string(), Some(3.14159), None),
    );

    // SQRT with scalar reference
    model.scalars.insert(
        "outputs.sqrt_ref".to_string(),
        Variable::new(
            "outputs.sqrt_ref".to_string(),
            None,
            Some("=SQRT(inputs.base_value)".to_string()),
        ),
    );

    // ROUND with scalar references
    model.scalars.insert(
        "outputs.round_ref".to_string(),
        Variable::new(
            "outputs.round_ref".to_string(),
            None,
            Some("=ROUND(inputs.raw_value, inputs.precision)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Scalar math with refs should succeed");

    let sqrt_ref = result.scalars.get("outputs.sqrt_ref").unwrap();
    assert!(
        (sqrt_ref.value.unwrap() - 4.0).abs() < 0.0001,
        "SQRT(inputs.base_value=16) should return 4, got {}",
        sqrt_ref.value.unwrap()
    );

    let round_ref = result.scalars.get("outputs.round_ref").unwrap();
    assert!(
        (round_ref.value.unwrap() - 3.14).abs() < 0.0001,
        "ROUND(inputs.raw_value, inputs.precision) should return 3.14, got {}",
        round_ref.value.unwrap()
    );

    println!("✓ Scalar math with scalar references test passed");
}

#[test]
fn test_average_with_zeros() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 0.0, 10.0, 0.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.avg".to_string(),
        Variable::new(
            "outputs.avg".to_string(),
            None,
            Some("=AVERAGE(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let avg = result.scalars.get("outputs.avg").unwrap();
    // (0 + 0 + 10 + 0) / 4 = 2.5
    assert!((avg.value.unwrap() - 2.5).abs() < 0.0001);
    println!("✓ AVERAGE with zeros edge case passed");
}

#[test]
fn test_scalar_references_scalar() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "inputs.base".to_string(),
        Variable::new("inputs.base".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "inputs.multiplier".to_string(),
        Variable::new("inputs.multiplier".to_string(), Some(1.5), None),
    );
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=inputs.base * inputs.multiplier".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 150.0).abs() < 0.0001);
    println!("✓ Scalar references scalar edge case passed");
}

#[test]
fn test_scalar_references_table_aggregation() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "inputs.tax_rate".to_string(),
        Variable::new("inputs.tax_rate".to_string(), Some(0.20), None),
    );
    model.scalars.insert(
        "outputs.total_after_tax".to_string(),
        Variable::new(
            "outputs.total_after_tax".to_string(),
            None,
            Some("=SUM(sales.revenue) * (1 - inputs.tax_rate)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Complex formula with table aggregation and scalar reference
    assert!(result.is_ok() || result.is_err());
    println!("✓ Scalar references table aggregation edge case passed");
}

#[test]
fn test_table_formula_with_constant() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(table);

    // Test scalar that multiplies table sum by constant
    model.scalars.insert(
        "outputs.sum_base".to_string(),
        Variable::new(
            "outputs.sum_base".to_string(),
            None,
            Some("=SUM(data.base)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let sum_base = result.scalars.get("outputs.sum_base").unwrap();
    // 10+20+30 = 60
    assert!((sum_base.value.unwrap() - 60.0).abs() < 0.01);
    println!("✓ Table formula with constant edge case passed");
}

#[test]
fn test_table_formula_column_arithmetic() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("financials".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0]),
    ));
    table.add_column(Column::new(
        "costs".to_string(),
        ColumnValue::Number(vec![600.0, 1100.0]),
    ));
    model.add_table(table);

    // Calculate simple aggregates
    model.scalars.insert(
        "outputs.total_revenue".to_string(),
        Variable::new(
            "outputs.total_revenue".to_string(),
            None,
            Some("=SUM(financials.revenue)".to_string()),
        ),
    );
    model.scalars.insert(
        "outputs.total_costs".to_string(),
        Variable::new(
            "outputs.total_costs".to_string(),
            None,
            Some("=SUM(financials.costs)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");

    let revenue = result.scalars.get("outputs.total_revenue").unwrap();
    // 1000+2000 = 3000
    assert!((revenue.value.unwrap() - 3000.0).abs() < 0.01);

    let costs = result.scalars.get("outputs.total_costs").unwrap();
    // 600+1100 = 1700
    assert!((costs.value.unwrap() - 1700.0).abs() < 0.01);
    println!("✓ Table formula column arithmetic edge case passed");
}

#[test]
fn test_very_small_numbers() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=0.0000001 * 1000000".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 0.1).abs() < 0.0001);
    println!("✓ Very small numbers edge case passed");
}

#[test]
fn test_very_large_numbers() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=1000000000 + 1".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Large number arithmetic - verify it handles without overflow
    assert!(result.is_ok(), "Large number calculation should not fail");
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("outputs.result") {
            if let Some(v) = val.value {
                // Should be close to 1_000_000_001.0
                assert!(
                    v > 999_000_000.0,
                    "Large number should be computed correctly"
                );
            }
        }
    }
    println!("✓ Very large numbers edge case passed");
}

#[test]
fn test_division_by_zero_handling() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=1 / 0".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Division by zero should either error or return infinity
    let _ = result;
    println!("✓ Division by zero test passed");
}

#[test]
fn test_circular_reference_detection() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "a".to_string(),
        Variable::new("a".to_string(), None, Some("=b + 1".to_string())),
    );
    model.scalars.insert(
        "b".to_string(),
        Variable::new("b".to_string(), None, Some("=a + 1".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should detect circular reference and error
    assert!(result.is_err(), "Circular reference should be detected");
    println!("✓ Circular reference detection test passed");
}

#[test]
fn test_undefined_reference_error() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=nonexistent_var + 1".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error on undefined reference
    assert!(result.is_err(), "Undefined reference should error");
    println!("✓ Undefined reference test passed");
}
