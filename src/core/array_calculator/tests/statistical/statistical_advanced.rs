//! Advanced statistical function tests
//!
//! Tests for CORREL, RANK, LARGE, SMALL, and comprehensive edge cases

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS - 100% COVERAGE MANDATORY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_stdev_single_element_should_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "stdev".to_string(),
        Variable::new(
            "stdev".to_string(),
            None,
            Some("=STDEV(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // STDEV requires at least 2 values (sample stdev undefined for n=1)
    assert!(
        result.is_err(),
        "STDEV of single element should error (sample stdev requires n>=2)"
    );
}

#[test]
fn test_var_single_element_should_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "var".to_string(),
        Variable::new(
            "var".to_string(),
            None,
            Some("=VAR(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // VAR requires at least 2 values (sample variance undefined for n=1)
    assert!(
        result.is_err(),
        "VAR of single element should error (sample variance requires n>=2)"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_median_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "median".to_string(),
        Variable::new(
            "median".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Median of single element [42] is 42
    let median = result.scalars.get("median").unwrap().value.unwrap();
    assert_eq!(median, 42.0);
}

#[test]
fn test_average_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "avg".to_string(),
        Variable::new(
            "avg".to_string(),
            None,
            Some("=AVERAGE(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Average of single element [42] is 42
    let avg = result.scalars.get("avg").unwrap().value.unwrap();
    assert_eq!(avg, 42.0);
}

#[test]
fn test_stdev_identical_values_is_zero() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    // All values identical: [5, 5, 5, 5]
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "stdev".to_string(),
        Variable::new(
            "stdev".to_string(),
            None,
            Some("=STDEV(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // STDEV of identical values should be 0 (no variation)
    let stdev = result.scalars.get("stdev").unwrap().value.unwrap();
    assert_eq!(stdev, 0.0, "STDEV of identical values must be exactly 0");
}

#[test]
fn test_var_identical_values_is_zero() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    // All values identical: [5, 5, 5, 5]
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "var".to_string(),
        Variable::new(
            "var".to_string(),
            None,
            Some("=VAR(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // VAR of identical values should be 0 (no variation)
    let var = result.scalars.get("var").unwrap().value.unwrap();
    assert_eq!(var, 0.0, "VAR of identical values must be exactly 0");
}

#[test]
fn test_percentile_as_median() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");

    // 50th percentile = median of [1,2,3,4,5,6,7,8,9] = 5
    let p50 = result.scalars.get("p50").unwrap().value.unwrap();
    assert_eq!(p50, 5.0, "PERCENTILE(0.5) must equal median");
}

#[test]
fn test_quartile_min_q0() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "q0".to_string(),
        Variable::new(
            "q0".to_string(),
            None,
            Some("=QUARTILE(data.values, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // QUARTILE(0) = minimum = 10
    let q0 = result.scalars.get("q0").unwrap().value.unwrap();
    assert_eq!(q0, 10.0, "QUARTILE(0) must equal minimum value");
}

#[test]
fn test_quartile_max_q4() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "q4".to_string(),
        Variable::new(
            "q4".to_string(),
            None,
            Some("=QUARTILE(data.values, 4)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // QUARTILE(4) = maximum = 80
    let q4 = result.scalars.get("q4").unwrap().value.unwrap();
    assert_eq!(q4, 80.0, "QUARTILE(4) must equal maximum value");
}

#[test]
fn test_quartile_all_quartiles() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "q0".to_string(),
        Variable::new(
            "q0".to_string(),
            None,
            Some("=QUARTILE(data.values, 0)".to_string()),
        ),
    );
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
    model.add_scalar(
        "q4".to_string(),
        Variable::new(
            "q4".to_string(),
            None,
            Some("=QUARTILE(data.values, 4)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Verify all quartiles
    let q0 = result.scalars.get("q0").unwrap().value.unwrap();
    let q1 = result.scalars.get("q1").unwrap().value.unwrap();
    let q2 = result.scalars.get("q2").unwrap().value.unwrap();
    let q3 = result.scalars.get("q3").unwrap().value.unwrap();
    let q4 = result.scalars.get("q4").unwrap().value.unwrap();

    assert_eq!(q0, 1.0, "Q0 must be minimum");
    assert_eq!(q1, 3.0, "Q1 must be 25th percentile");
    assert_eq!(q2, 5.0, "Q2 must be median");
    assert_eq!(q3, 7.0, "Q3 must be 75th percentile");
    assert_eq!(q4, 9.0, "Q4 must be maximum");

    // Verify quartiles are monotonically increasing
    assert!(q0 <= q1 && q1 <= q2 && q2 <= q3 && q3 <= q4);
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_rank_with_ties() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    // Values with ties: [85, 90, 85, 92, 90]
    data.add_column(Column::new(
        "scores".to_string(),
        ColumnValue::Number(vec![85.0, 90.0, 85.0, 92.0, 90.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Test RANK with tied values (descending order)
    model.add_scalar(
        "rank_92".to_string(),
        Variable::new(
            "rank_92".to_string(),
            None,
            Some("=RANK(92, data.scores)".to_string()),
        ),
    );
    model.add_scalar(
        "rank_90".to_string(),
        Variable::new(
            "rank_90".to_string(),
            None,
            Some("=RANK(90, data.scores)".to_string()),
        ),
    );
    model.add_scalar(
        "rank_85".to_string(),
        Variable::new(
            "rank_85".to_string(),
            None,
            Some("=RANK(85, data.scores)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Descending sorted: [92, 90, 90, 85, 85]
    // 92 = rank 1, 90 = rank 2 (first occurrence), 85 = rank 4 (first occurrence)
    let rank_92 = result.scalars.get("rank_92").unwrap().value.unwrap();
    let rank_90 = result.scalars.get("rank_90").unwrap().value.unwrap();
    let rank_85 = result.scalars.get("rank_85").unwrap().value.unwrap();

    assert_eq!(rank_92, 1.0, "92 is highest, rank 1");
    assert_eq!(rank_90, 2.0, "90 is second highest (first occurrence)");
    assert_eq!(
        rank_85, 4.0,
        "85 is fourth (first occurrence after two 90s)"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_rank_ascending_vs_descending() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 50.0, 30.0, 70.0, 20.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Test value 30 in both ascending and descending order
    model.add_scalar(
        "rank_desc".to_string(),
        Variable::new(
            "rank_desc".to_string(),
            None,
            Some("=RANK(30, data.values, 0)".to_string()),
        ),
    );
    model.add_scalar(
        "rank_asc".to_string(),
        Variable::new(
            "rank_asc".to_string(),
            None,
            Some("=RANK(30, data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Descending: [70, 50, 30, 20, 10] - 30 is rank 3
    // Ascending: [10, 20, 30, 50, 70] - 30 is rank 3
    let rank_desc = result.scalars.get("rank_desc").unwrap().value.unwrap();
    let rank_asc = result.scalars.get("rank_asc").unwrap().value.unwrap();

    assert_eq!(rank_desc, 3.0, "30 is 3rd largest (descending)");
    assert_eq!(rank_asc, 3.0, "30 is 3rd smallest (ascending)");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_rank_value_not_in_array() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Try to rank a value (99) that doesn't exist in the array
    model.add_scalar(
        "rank".to_string(),
        Variable::new(
            "rank".to_string(),
            None,
            Some("=RANK(99, data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // RANK should error when value is not found in array
    assert!(
        result.is_err(),
        "RANK should error when value not found in array"
    );
}

#[test]
fn test_correl_no_correlation() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    // No correlation: x increases, y is random
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![5.0, 2.0, 8.0, 1.0, 6.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");

    // Weak/no correlation - verify r is close to 0
    let corr = result.scalars.get("corr").unwrap().value.unwrap();
    assert!(
        corr.abs() < 0.5,
        "Correlation should be weak (close to 0), got {corr}"
    );
}

#[test]
fn test_correl_constant_values_should_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    // Constant x values (zero variance)
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0, 5.0]),
    ));
    data.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
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

    // CORREL should error when one array has zero variance
    assert!(
        result.is_err(),
        "CORREL should error with constant values (zero variance)"
    );
}

#[test]
fn test_percentile_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");

    // PERCENTILE of single element should return that element
    let p50 = result.scalars.get("p50").unwrap().value.unwrap();
    assert_eq!(p50, 42.0, "PERCENTILE of single element [42] must be 42");
}

#[test]
fn test_quartile_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");

    // QUARTILE of single element should return that element
    let q2 = result.scalars.get("q2").unwrap().value.unwrap();
    assert_eq!(q2, 42.0, "QUARTILE of single element [42] must be 42");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "largest".to_string(),
        Variable::new(
            "largest".to_string(),
            None,
            Some("=LARGE(data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // LARGE(1) of single element [42] is 42
    let largest = result.scalars.get("largest").unwrap().value.unwrap();
    assert_eq!(largest, 42.0, "LARGE(1) of single element [42] must be 42");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_small_single_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "smallest".to_string(),
        Variable::new(
            "smallest".to_string(),
            None,
            Some("=SMALL(data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // SMALL(1) of single element [42] is 42
    let smallest = result.scalars.get("smallest").unwrap().value.unwrap();
    assert_eq!(smallest, 42.0, "SMALL(1) of single element [42] must be 42");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_k_out_of_bounds() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Try k=5 when array only has 3 elements
    model.add_scalar(
        "large".to_string(),
        Variable::new(
            "large".to_string(),
            None,
            Some("=LARGE(data.values, 5)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // LARGE should error when k > array length
    assert!(
        result.is_err(),
        "LARGE should error when k is greater than array length"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_small_k_out_of_bounds() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Try k=0 (invalid)
    model.add_scalar(
        "small".to_string(),
        Variable::new(
            "small".to_string(),
            None,
            Some("=SMALL(data.values, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // SMALL should error when k=0
    assert!(result.is_err(), "SMALL should error when k=0");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_k_zero_should_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    // Try k=0 (invalid - must be >= 1)
    model.add_scalar(
        "large".to_string(),
        Variable::new(
            "large".to_string(),
            None,
            Some("=LARGE(data.values, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // LARGE should error when k=0
    assert!(
        result.is_err(),
        "LARGE should error when k=0 (k must be >= 1)"
    );
}
