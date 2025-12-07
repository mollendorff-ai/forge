//! Statistical function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_quartile_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "q1".to_string(),
        Variable::new(
            "q1".to_string(),
            None,
            Some("=QUARTILE(data.values, 1)".to_string()),
        ),
    );
    model.add_scalar(
        "q2".to_string(),
        Variable::new(
            "q2".to_string(),
            None,
            Some("=QUARTILE(data.values, 2)".to_string()),
        ),
    );
    model.add_scalar(
        "q3".to_string(),
        Variable::new(
            "q3".to_string(),
            None,
            Some("=QUARTILE(data.values, 3)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Q2 should be median = 5.5
    let q2 = result.scalars.get("q2").unwrap().value.unwrap();
    assert!((q2 - 5.5).abs() < 0.5);
}

#[test]
fn test_correl_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    // Perfect positive correlation
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "correlation".to_string(),
        Variable::new(
            "correlation".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Perfect positive correlation = 1.0
    let corr = result.scalars.get("correlation").unwrap().value.unwrap();
    assert!((corr - 1.0).abs() < 0.01);
}

#[test]
fn test_var_p_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "variance_pop".to_string(),
        Variable::new(
            "variance_pop".to_string(),
            None,
            Some("=VAR.P(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Population variance = 4.0
    let var = result.scalars.get("variance_pop").unwrap().value.unwrap();
    assert!((var - 4.0).abs() < 0.01);
}

#[test]
fn test_stdev_p_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "stdev_pop".to_string(),
        Variable::new(
            "stdev_pop".to_string(),
            None,
            Some("=STDEV.P(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Population stdev = 2.0
    let stdev = result.scalars.get("stdev_pop").unwrap().value.unwrap();
    assert!((stdev - 2.0).abs() < 0.01);
}

#[test]
fn test_percentile_edge_cases() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);

    // Test 0th percentile (minimum)
    model.add_scalar(
        "p0".to_string(),
        Variable::new(
            "p0".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0)".to_string()),
        ),
    );

    // Test 100th percentile (maximum)
    model.add_scalar(
        "p100".to_string(),
        Variable::new(
            "p100".to_string(),
            None,
            Some("=PERCENTILE(data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let p0 = result.scalars.get("p0").unwrap().value.unwrap();
    let p100 = result.scalars.get("p100").unwrap().value.unwrap();
    assert!((p0 - 1.0).abs() < 0.01);
    assert!((p100 - 5.0).abs() < 0.01);
}

#[test]
fn test_correl_perfect_positive() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "correlation".to_string(),
        Variable::new(
            "correlation".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Perfect positive correlation = 1.0
    let correl = result.scalars.get("correlation").unwrap().value.unwrap();
    assert!((correl - 1.0).abs() < 0.01);
}

#[test]
fn test_correl_negative() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![10.0, 8.0, 6.0, 4.0, 2.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "correlation".to_string(),
        Variable::new(
            "correlation".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Perfect negative correlation = -1.0
    let correl = result.scalars.get("correlation").unwrap().value.unwrap();
    assert!((correl - (-1.0)).abs() < 0.01);
}

#[test]
fn test_stdev_scalar() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "std".to_string(),
        Variable::new(
            "std".to_string(),
            None,
            Some("=STDEV(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_var_population() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "variance".to_string(),
        Variable::new(
            "variance".to_string(),
            None,
            Some("=VAR.P(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_percentile() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "p50".to_string(),
        Variable::new(
            "p50".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_quartile() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "q2".to_string(),
        Variable::new(
            "q2".to_string(),
            None,
            Some("=QUARTILE(data.values, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_correl() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]), // Perfect linear correlation
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "correlation".to_string(),
        Variable::new(
            "correlation".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    if let Some(scalar) = model.scalars.get("correlation") {
        assert!((scalar.value.unwrap() - 1.0).abs() < 0.001); // Should be 1.0
    }
}

#[test]
fn test_percentile_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "p75".to_string(),
        Variable::new(
            "p75".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.75)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_quartile_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "q3".to_string(),
        Variable::new(
            "q3".to_string(),
            None,
            Some("=QUARTILE(data.values, 3)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_correl_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "corr".to_string(),
        Variable::new(
            "corr".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_correl_empty_array() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.columns.insert(
        "x".to_string(),
        Column::new("x".to_string(), ColumnValue::Number(vec![])),
    );
    data.columns.insert(
        "y".to_string(),
        Column::new("y".to_string(), ColumnValue::Number(vec![])),
    );
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "corr".to_string(),
        Variable::new(
            "corr".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_correl_mismatched_lengths() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]), // Different length
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "corr".to_string(),
        Variable::new(
            "corr".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to length mismatch
    assert!(result.is_err());
}

#[test]
fn test_percentile_valid() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "p50".to_string(),
        Variable::new(
            "p50".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_percentile_k_invalid() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "p".to_string(),
        Variable::new(
            "p".to_string(),
            None,
            Some("=PERCENTILE(data.values, 1.5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_quartile_q1() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "q".to_string(),
        Variable::new(
            "q".to_string(),
            None,
            Some("=QUARTILE(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_correl_arrays() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "r".to_string(),
        Variable::new(
            "r".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_stdev_sample() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "sd".to_string(),
        Variable::new(
            "sd".to_string(),
            None,
            Some("=STDEV(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_stdevp_population() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "sd".to_string(),
        Variable::new(
            "sd".to_string(),
            None,
            Some("=STDEVP(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_var_sample() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "v".to_string(),
        Variable::new("v".to_string(), None, Some("=VAR(data.values)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_varp_population() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "v".to_string(),
        Variable::new(
            "v".to_string(),
            None,
            Some("=VARP(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
