//! Aggregation functions: SUM, AVERAGE, COUNT, MIN, MAX, PRODUCT, MEDIAN
//!
//! DEMO functions (5): SUM, AVERAGE, MIN, MAX, COUNT
//! ENTERPRISE functions: PRODUCT, COUNTA, MEDIAN

use super::{collect_numeric_values, evaluate, EvalContext, EvalError, Expr, Value};

/// Try to evaluate an aggregation function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "SUM" => {
            let values = collect_numeric_values(args, ctx)?;
            Value::Number(values.iter().sum())
        },

        "AVERAGE" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("AVERAGE of empty set"));
            }
            Value::Number(values.iter().sum::<f64>() / values.len() as f64)
        },

        "MIN" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MIN of empty set"))?
        },

        "MAX" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MAX of empty set"))?
        },

        "COUNT" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| v.as_number().is_some()).count();
                    },
                    Value::Number(_) => count += 1,
                    _ => {},
                }
            }
            Value::Number(count as f64)
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "AVG" => {
            // Alias for AVERAGE (enterprise only)
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("AVG of empty set"));
            }
            Value::Number(values.iter().sum::<f64>() / values.len() as f64)
        },

        #[cfg(not(feature = "demo"))]
        "PRODUCT" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                Value::Number(0.0)
            } else {
                Value::Number(values.iter().product())
            }
        },

        #[cfg(not(feature = "demo"))]
        "COUNTA" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| !matches!(v, Value::Null)).count();
                    },
                    Value::Null => {},
                    _ => count += 1,
                }
            }
            Value::Number(count as f64)
        },

        #[cfg(not(feature = "demo"))]
        "MEDIAN" => {
            let mut values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("MEDIAN of empty set"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = values.len() / 2;
            if values.len() % 2 == 0 {
                Value::Number(f64::midpoint(values[mid - 1], values[mid]))
            } else {
                Value::Number(values[mid])
            }
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_aggregation_with_scalars() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SUM(1, 2, 3)", &ctx).unwrap(), Value::Number(6.0));
        assert_eq!(eval("AVERAGE(2, 4, 6)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("MIN(5, 2, 8)", &ctx).unwrap(), Value::Number(2.0));
        assert_eq!(eval("MAX(5, 2, 8)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_aggregation_with_array() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("data".to_string(), table);

        assert_eq!(eval("SUM(data.values)", &ctx).unwrap(), Value::Number(60.0));
        assert_eq!(
            eval("AVERAGE(data.values)", &ctx).unwrap(),
            Value::Number(20.0)
        );
    }

    #[test]
    fn test_count() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(1.0),
                Value::Text("text".to_string()),
                Value::Number(3.0),
                Value::Null,
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COUNT(t.values)", &ctx).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_aggregation_edge_cases() {
        let ctx = EvalContext::new();
        // SUM(1, 2, 3) = 6
        assert_eq!(eval("SUM(1, 2, 3)", &ctx).unwrap(), Value::Number(6.0));
        // AVERAGE(2, 4, 6) = 4
        assert_eq!(eval("AVERAGE(2, 4, 6)", &ctx).unwrap(), Value::Number(4.0));
        // MIN(5, 3, 8, 1) = 1
        assert_eq!(eval("MIN(5, 3, 8, 1)", &ctx).unwrap(), Value::Number(1.0));
        // MAX(5, 3, 8, 1) = 8
        assert_eq!(eval("MAX(5, 3, 8, 1)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_count_edge_cases() {
        let ctx = EvalContext::new();
        // COUNT(1, 2, 3, 4, 5) = 5
        assert_eq!(
            eval("COUNT(1, 2, 3, 4, 5)", &ctx).unwrap(),
            Value::Number(5.0)
        );
    }

    #[test]
    fn test_sum_empty() {
        let ctx = EvalContext::new();
        // SUM() = 0 (empty)
        assert_eq!(eval("SUM()", &ctx).unwrap(), Value::Number(0.0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median() {
        let ctx = EvalContext::new();
        assert_eq!(eval("MEDIAN(1, 3, 5)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(
            eval("MEDIAN(1, 2, 3, 4)", &ctx).unwrap(),
            Value::Number(2.5)
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_counta() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(1.0),
                Value::Text("text".to_string()),
                Value::Number(3.0),
                Value::Null,
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COUNTA(t.values)", &ctx).unwrap(), Value::Number(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_product() {
        let ctx = EvalContext::new();
        assert_eq!(eval("PRODUCT(2, 3, 4)", &ctx).unwrap(), Value::Number(24.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests (moved from tests/aggregation.rs)
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_aggregation_sum() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
        ));
        model.add_table(table);

        let total_revenue = Variable::new(
            "total_revenue".to_string(),
            None,
            Some("=SUM(sales.revenue)".to_string()),
        );
        model.add_scalar("total_revenue".to_string(), total_revenue);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let total = result.scalars.get("total_revenue").unwrap();
        assert_eq!(total.value, Some(1000.0));
    }

    #[test]
    fn test_aggregation_average() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("metrics".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(table);

        let avg_value = Variable::new(
            "avg_value".to_string(),
            None,
            Some("=AVERAGE(metrics.values)".to_string()),
        );
        model.add_scalar("avg_value".to_string(), avg_value);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        let avg = result.scalars.get("avg_value").unwrap();
        assert_eq!(avg.value, Some(25.0));
    }

    #[test]
    fn test_aggregation_max_min() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![15.0, 42.0, 8.0, 23.0]),
        ));
        model.add_table(table);

        let max_value = Variable::new(
            "max_value".to_string(),
            None,
            Some("=MAX(data.values)".to_string()),
        );
        model.add_scalar("max_value".to_string(), max_value);

        let min_value = Variable::new(
            "min_value".to_string(),
            None,
            Some("=MIN(data.values)".to_string()),
        );
        model.add_scalar("min_value".to_string(), min_value);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(result.scalars.get("max_value").unwrap().value, Some(42.0));
        assert_eq!(result.scalars.get("min_value").unwrap().value, Some(8.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 30.0, 20.0, 40.0, 50.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "median_val".to_string(),
            Variable::new(
                "median_val".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let median = result.scalars.get("median_val").unwrap().value.unwrap();
        assert_eq!(median, 30.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median_even_count() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "median_val".to_string(),
            Variable::new(
                "median_val".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let median = result.scalars.get("median_val").unwrap().value.unwrap();
        assert_eq!(median, 25.0);
    }

    #[test]
    fn test_count_function() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(table);

        model.add_scalar(
            "cnt".to_string(),
            Variable::new(
                "cnt".to_string(),
                None,
                Some("=COUNT(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let cnt = result.scalars.get("cnt").unwrap().value.unwrap();
        assert!((cnt - 5.0).abs() < 0.01);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_counta_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "items".to_string(),
            ColumnValue::Text(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
        ));
        model.add_table(data);

        model.add_scalar(
            "cnt".to_string(),
            Variable::new(
                "cnt".to_string(),
                None,
                Some("=COUNTA(data.items)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count = result.scalars.get("cnt").unwrap().value.unwrap();
        assert_eq!(count, 3.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_product_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![2.0, 3.0, 4.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "prod".to_string(),
            Variable::new(
                "prod".to_string(),
                None,
                Some("=PRODUCT(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let product = result.scalars.get("prod").unwrap().value.unwrap();
        assert_eq!(product, 24.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_large_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 70.0, 20.0, 90.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "largest".to_string(),
            Variable::new(
                "largest".to_string(),
                None,
                Some("=LARGE(data.values, 1)".to_string()),
            ),
        );
        model.add_scalar(
            "second_largest".to_string(),
            Variable::new(
                "second_largest".to_string(),
                None,
                Some("=LARGE(data.values, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert_eq!(result.scalars.get("largest").unwrap().value.unwrap(), 90.0);
        assert_eq!(
            result.scalars.get("second_largest").unwrap().value.unwrap(),
            70.0
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_small_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 70.0, 20.0, 90.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "smallest".to_string(),
            Variable::new(
                "smallest".to_string(),
                None,
                Some("=SMALL(data.values, 1)".to_string()),
            ),
        );
        model.add_scalar(
            "second_smallest".to_string(),
            Variable::new(
                "second_smallest".to_string(),
                None,
                Some("=SMALL(data.values, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert_eq!(result.scalars.get("smallest").unwrap().value.unwrap(), 10.0);
        assert_eq!(
            result
                .scalars
                .get("second_smallest")
                .unwrap()
                .value
                .unwrap(),
            20.0
        );
    }

    #[test]
    fn test_sum_aggregation_simple() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some("=SUM(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let total = result.scalars.get("total").unwrap().value.unwrap();
        assert_eq!(total, 60.0);
    }

    #[test]
    fn test_sum_empty_table() {
        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        model.add_table(table);

        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some("=SUM(empty.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should handle missing column gracefully
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median_odd_count() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 1.0, 9.0, 3.0, 7.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "med".to_string(),
            Variable::new(
                "med".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("med") {
            assert_eq!(scalar.value.unwrap(), 5.0);
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median_even_array_count() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "med".to_string(),
            Variable::new(
                "med".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("med") {
            assert_eq!(scalar.value.unwrap(), 2.5); // (2+3)/2
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_avg_aggregation_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "avg_val".to_string(),
            Variable::new(
                "avg_val".to_string(),
                None,
                Some("=AVG(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Values: [10, 20, 30, 40] - average = 25.0
        let avg = result.scalars.get("avg_val").unwrap().value.unwrap();
        assert_eq!(avg, 25.0);
    }

    #[test]
    fn test_max_aggregation_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "max_val".to_string(),
            Variable::new(
                "max_val".to_string(),
                None,
                Some("=MAX(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_min_aggregation_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "min_val".to_string(),
            Variable::new(
                "min_val".to_string(),
                None,
                Some("=MIN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_median_aggregation_scalar() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 3.0, 5.0, 7.0, 9.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "med_val".to_string(),
            Variable::new(
                "med_val".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_empty_array_median() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.columns.insert(
            "values".to_string(),
            Column::new("values".to_string(), ColumnValue::Number(vec![])),
        );
        model.add_table(data);

        model.add_scalar(
            "med".to_string(),
            Variable::new(
                "med".to_string(),
                None,
                Some("=MEDIAN(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        // Empty array median: verify behavior is consistent
        // Either it errors (mathematically undefined) OR returns 0
        match result {
            Ok(model) => {
                let median = model.scalars.get("med").unwrap().value;
                // If it succeeds, median of empty array should be 0 or None
                assert!(
                    median.is_none() || median == Some(0.0),
                    "Empty array median should be None or 0, got {median:?}"
                );
            },
            Err(_) => {
                // Error is acceptable for empty array median (undefined)
            },
        }
    }

    #[test]
    fn test_sumproduct_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "qty".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        data.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![5.0, 10.0, 15.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some("=SUMPRODUCT(data.qty, data.price)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        // SUMPRODUCT is not implemented in base version
        // Verify it returns an error for unknown function
        assert!(
            result.is_err(),
            "SUMPRODUCT should return error (not implemented)"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("SUMPRODUCT") || err.to_string().contains("Unknown function"),
            "Error should mention SUMPRODUCT or Unknown function, got: {err}"
        );
    }

    #[test]
    fn test_count_function_v2() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "cnt".to_string(),
            Variable::new(
                "cnt".to_string(),
                None,
                Some("=COUNT(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_counta_with_empty_strings() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Text(vec!["a".to_string(), String::new(), "b".to_string()]),
        ));
        model.add_table(data);

        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTA(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // COUNTA: ["a", "", "b"] - counts all values (including empty strings)
        // Standard Excel behavior: COUNTA counts all non-blank cells, treating "" as having data
        let count = result.scalars.get("count").unwrap().value.unwrap();
        assert_eq!(count, 3.0);
    }

    #[test]
    fn test_countblank_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Text(vec![
                "a".to_string(),
                String::new(),
                "b".to_string(),
                String::new(),
            ]),
        ));
        model.add_table(data);

        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTBLANK(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        // COUNTBLANK is not implemented
        // Verify it returns an error for unknown function
        assert!(
            result.is_err(),
            "COUNTBLANK should return error (not implemented)"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("COUNTBLANK") || err.to_string().contains("Unknown function"),
            "Error should mention COUNTBLANK or Unknown function, got: {err}"
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0, 20.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "unique_count".to_string(),
            Variable::new(
                "unique_count".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Values: [10, 20, 10, 30, 20, 40] - unique count = 4 (10, 20, 30, 40)
        let count = result.scalars.get("unique_count").unwrap().value.unwrap();
        assert_eq!(count, 4.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_rank_eq_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "scores".to_string(),
            ColumnValue::Number(vec![85.0, 90.0, 78.0, 92.0, 88.0]),
        ));
        model.add_table(data);

        // Test RANK.EQ with descending order (default)
        model.add_scalar(
            "rank_92_desc".to_string(),
            Variable::new(
                "rank_92_desc".to_string(),
                None,
                Some("=RANK.EQ(92, data.scores)".to_string()),
            ),
        );
        model.add_scalar(
            "rank_90_desc".to_string(),
            Variable::new(
                "rank_90_desc".to_string(),
                None,
                Some("=RANK.EQ(90, data.scores, 0)".to_string()),
            ),
        );

        // Test RANK with ascending order
        model.add_scalar(
            "rank_78_asc".to_string(),
            Variable::new(
                "rank_78_asc".to_string(),
                None,
                Some("=RANK(78, data.scores, 1)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // Descending: [92, 90, 88, 85, 78] - ranks: 1, 2, 3, 4, 5
        assert_eq!(
            result.scalars.get("rank_92_desc").unwrap().value.unwrap(),
            1.0
        );
        assert_eq!(
            result.scalars.get("rank_90_desc").unwrap().value.unwrap(),
            2.0
        );

        // Ascending: [78, 85, 88, 90, 92] - 78 is rank 1
        assert_eq!(
            result.scalars.get("rank_78_asc").unwrap().value.unwrap(),
            1.0
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_maxifs_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 300.0, 250.0]),
        ));
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "quarter".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0, 2.0]),
        ));
        model.add_table(data);

        // Max amount where region=East
        model.add_scalar(
            "max_east".to_string(),
            Variable::new(
                "max_east".to_string(),
                None,
                Some("=MAXIFS(sales.amount, sales.region, \"East\")".to_string()),
            ),
        );

        // Max amount where region=East AND quarter=2
        model.add_scalar(
            "max_east_q2".to_string(),
            Variable::new(
                "max_east_q2".to_string(),
                None,
                Some("=MAXIFS(sales.amount, sales.region, \"East\", sales.quarter, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // East amounts: [100, 150, 250] - max = 250
        assert_eq!(
            result.scalars.get("max_east").unwrap().value.unwrap(),
            250.0
        );

        // East + Q2 amounts: [150, 250] - max = 250
        assert_eq!(
            result.scalars.get("max_east_q2").unwrap().value.unwrap(),
            250.0
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_minifs_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 300.0, 250.0]),
        ));
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "quarter".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0, 2.0]),
        ));
        model.add_table(data);

        // Min amount where region=West
        model.add_scalar(
            "min_west".to_string(),
            Variable::new(
                "min_west".to_string(),
                None,
                Some("=MINIFS(sales.amount, sales.region, \"West\")".to_string()),
            ),
        );

        // Min amount where region=East AND quarter=2
        model.add_scalar(
            "min_east_q2".to_string(),
            Variable::new(
                "min_east_q2".to_string(),
                None,
                Some("=MINIFS(sales.amount, sales.region, \"East\", sales.quarter, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // West amounts: [200, 300] - min = 200
        assert_eq!(
            result.scalars.get("min_west").unwrap().value.unwrap(),
            200.0
        );

        // East + Q2 amounts: [150, 250] - min = 150
        assert_eq!(
            result.scalars.get("min_east_q2").unwrap().value.unwrap(),
            150.0
        );
    }
}
