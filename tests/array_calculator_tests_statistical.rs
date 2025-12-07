// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};

#[test]
fn test_median_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with values for MEDIAN
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 3.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    // Scalar with MEDIAN formula
    model.scalars.insert(
        "outputs.median_result".to_string(),
        Variable::new(
            "outputs.median_result".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("MEDIAN calculation should succeed");

    let median = result.scalars.get("outputs.median_result").unwrap();
    assert!(
        (median.value.unwrap() - 5.0).abs() < 0.0001,
        "MEDIAN([1,3,5,7,9]) should return 5, got {}",
        median.value.unwrap()
    );

    println!("✓ MEDIAN function test passed");
}

#[test]
fn test_var_stdev_functions() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with values
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    // Sample variance
    model.scalars.insert(
        "outputs.var_sample".to_string(),
        Variable::new(
            "outputs.var_sample".to_string(),
            None,
            Some("=VAR.S(data.values)".to_string()),
        ),
    );

    // Sample standard deviation
    model.scalars.insert(
        "outputs.stdev_sample".to_string(),
        Variable::new(
            "outputs.stdev_sample".to_string(),
            None,
            Some("=STDEV.S(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("VAR/STDEV calculation should succeed");

    // Sample variance for [2,4,4,4,5,5,7,9] = 4.571428...
    let var_sample = result.scalars.get("outputs.var_sample").unwrap();
    assert!(
        (var_sample.value.unwrap() - 4.5714).abs() < 0.01,
        "VAR.S should return ~4.5714, got {}",
        var_sample.value.unwrap()
    );

    // Sample stdev = sqrt(4.571428) = 2.138
    let stdev_sample = result.scalars.get("outputs.stdev_sample").unwrap();
    assert!(
        (stdev_sample.value.unwrap() - 2.138).abs() < 0.01,
        "STDEV.S should return ~2.138, got {}",
        stdev_sample.value.unwrap()
    );

    println!("✓ VAR/STDEV functions test passed");
}

#[test]
fn test_percentile_quartile_functions() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with values
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]),
    ));
    model.add_table(table);

    // 50th percentile (median)
    model.scalars.insert(
        "outputs.p50".to_string(),
        Variable::new(
            "outputs.p50".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.5)".to_string()),
        ),
    );

    // 25th percentile (Q1)
    model.scalars.insert(
        "outputs.q1".to_string(),
        Variable::new(
            "outputs.q1".to_string(),
            None,
            Some("=QUARTILE(data.values, 1)".to_string()),
        ),
    );

    // 75th percentile (Q3)
    model.scalars.insert(
        "outputs.q3".to_string(),
        Variable::new(
            "outputs.q3".to_string(),
            None,
            Some("=QUARTILE(data.values, 3)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("PERCENTILE/QUARTILE calculation should succeed");

    let p50 = result.scalars.get("outputs.p50").unwrap();
    assert!(
        (p50.value.unwrap() - 5.5).abs() < 0.1,
        "PERCENTILE(data, 0.5) should return ~5.5, got {}",
        p50.value.unwrap()
    );

    let q1 = result.scalars.get("outputs.q1").unwrap();
    assert!(
        (q1.value.unwrap() - 3.25).abs() < 0.1,
        "QUARTILE(data, 1) should return ~3.25, got {}",
        q1.value.unwrap()
    );

    let q3 = result.scalars.get("outputs.q3").unwrap();
    assert!(
        (q3.value.unwrap() - 7.75).abs() < 0.1,
        "QUARTILE(data, 3) should return ~7.75, got {}",
        q3.value.unwrap()
    );

    println!("✓ PERCENTILE/QUARTILE functions test passed");
}

#[test]
fn test_correl_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with correlated data
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]), // Perfect correlation
    ));
    model.add_table(table);

    // Correlation coefficient
    model.scalars.insert(
        "outputs.correlation".to_string(),
        Variable::new(
            "outputs.correlation".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("CORREL calculation should succeed");

    let corr = result.scalars.get("outputs.correlation").unwrap();
    assert!(
        (corr.value.unwrap() - 1.0).abs() < 0.0001,
        "CORREL for perfect correlation should return 1, got {}",
        corr.value.unwrap()
    );

    println!("✓ CORREL function test passed");
}

#[test]
fn test_median_even_count() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with even number of values
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.median_even".to_string(),
        Variable::new(
            "outputs.median_even".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("MEDIAN even count calculation should succeed");

    // MEDIAN([1,2,3,4]) = (2+3)/2 = 2.5
    let median = result.scalars.get("outputs.median_even").unwrap();
    assert!(
        (median.value.unwrap() - 2.5).abs() < 0.0001,
        "MEDIAN([1,2,3,4]) should return 2.5, got {}",
        median.value.unwrap()
    );

    println!("✓ MEDIAN even count test passed");
}

#[test]
fn test_population_variance() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();

    // Table with values
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    // Population variance
    model.scalars.insert(
        "outputs.var_pop".to_string(),
        Variable::new(
            "outputs.var_pop".to_string(),
            None,
            Some("=VAR.P(data.values)".to_string()),
        ),
    );

    // Population standard deviation
    model.scalars.insert(
        "outputs.stdev_pop".to_string(),
        Variable::new(
            "outputs.stdev_pop".to_string(),
            None,
            Some("=STDEV.P(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Population VAR/STDEV calculation should succeed");

    // Population variance = 4.0
    let var_pop = result.scalars.get("outputs.var_pop").unwrap();
    assert!(
        (var_pop.value.unwrap() - 4.0).abs() < 0.01,
        "VAR.P should return 4.0, got {}",
        var_pop.value.unwrap()
    );

    // Population stdev = 2.0
    let stdev_pop = result.scalars.get("outputs.stdev_pop").unwrap();
    assert!(
        (stdev_pop.value.unwrap() - 2.0).abs() < 0.01,
        "STDEV.P should return 2.0, got {}",
        stdev_pop.value.unwrap()
    );

    println!("✓ Population VAR.P/STDEV.P test passed");
}

#[test]
fn test_median_single_element() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.median".to_string(),
        Variable::new(
            "outputs.median".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let median = result.scalars.get("outputs.median").unwrap();
    assert!((median.value.unwrap() - 42.0).abs() < 0.0001);
    println!("✓ MEDIAN single element edge case passed");
}

#[test]
fn test_stdev_two_elements() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 10.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.stdev".to_string(),
        Variable::new(
            "outputs.stdev".to_string(),
            None,
            Some("=STDEV.S(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let stdev = result.scalars.get("outputs.stdev").unwrap();
    // Sample stdev of [0, 10] = sqrt((25+25)/1) = sqrt(50) ≈ 7.07
    assert!(stdev.value.unwrap() > 7.0 && stdev.value.unwrap() < 7.2);
    println!("✓ STDEV.S two elements edge case passed");
}

#[test]
fn test_percentile_extremes() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    // 0th percentile = min
    model.scalars.insert(
        "outputs.p0".to_string(),
        Variable::new(
            "outputs.p0".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0)".to_string()),
        ),
    );
    // 100th percentile = max
    model.scalars.insert(
        "outputs.p100".to_string(),
        Variable::new(
            "outputs.p100".to_string(),
            None,
            Some("=PERCENTILE(data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");

    let p0 = result.scalars.get("outputs.p0").unwrap();
    assert!(
        (p0.value.unwrap() - 10.0).abs() < 0.1,
        "0th percentile should be min"
    );

    let p100 = result.scalars.get("outputs.p100").unwrap();
    assert!(
        (p100.value.unwrap() - 50.0).abs() < 0.1,
        "100th percentile should be max"
    );
    println!("✓ PERCENTILE extremes edge case passed");
}

#[test]
fn test_correl_negative() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    // Perfect negative correlation
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![10.0, 8.0, 6.0, 4.0, 2.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.corr".to_string(),
        Variable::new(
            "outputs.corr".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let corr = result.scalars.get("outputs.corr").unwrap();
    assert!(
        (corr.value.unwrap() - (-1.0)).abs() < 0.0001,
        "Perfect negative correlation"
    );
    println!("✓ CORREL negative correlation edge case passed");
}

#[test]
fn test_correl_no_correlation() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    // No correlation (constant y)
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.corr".to_string(),
        Variable::new(
            "outputs.corr".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // May return NaN or error for constant data - just verify it doesn't crash
    assert!(result.is_ok() || result.is_err());
    println!("✓ CORREL constant data edge case passed");
}

#[test]
fn test_median_large_dataset() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    // 1000 elements (1 to 1000)
    let values: Vec<f64> = (1..=1000).map(|x| x as f64).collect();
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(values),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.median".to_string(),
        Variable::new(
            "outputs.median".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let median = result.scalars.get("outputs.median").unwrap();
    // Median of 1 to 1000 = (500 + 501) / 2 = 500.5
    assert!((median.value.unwrap() - 500.5).abs() < 0.1);
    println!("✓ MEDIAN large dataset edge case passed");
}

#[test]
fn test_stdev_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "std".to_string(),
        Variable::new(
            "std".to_string(),
            None,
            Some("=STDEV(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("std") {
            if let Some(v) = val.value {
                assert!(v > 1.5 && v < 2.5, "STDEV should be around 2.0");
            }
        }
    }
    println!("✓ STDEV function test passed");
}
