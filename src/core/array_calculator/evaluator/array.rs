//! Array functions: UNIQUE, COUNTUNIQUE, SORT, FILTER, SEQUENCE, RANDARRAY

use super::{
    collect_numeric_values, collect_values_as_vec, evaluate, require_args, require_args_range,
    EvalContext, EvalError, Expr, Value,
};
use rand::Rng;

/// Try to evaluate an array function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "UNIQUE" => {
            require_args(name, args, 1)?;
            let values = collect_values_as_vec(&args[0], ctx)?;
            let mut seen = Vec::new();
            for v in values {
                let text = v.as_text();
                if !seen.iter().any(|s: &Value| s.as_text() == text) {
                    seen.push(v);
                }
            }
            Value::Array(seen)
        },

        "COUNTUNIQUE" => {
            require_args(name, args, 1)?;
            let values = collect_values_as_vec(&args[0], ctx)?;
            let mut seen = std::collections::HashSet::new();
            for v in values {
                seen.insert(v.as_text());
            }
            Value::Number(seen.len() as f64)
        },

        "SORT" => {
            require_args_range(name, args, 1, 2)?;
            let mut values = collect_numeric_values(args, ctx)?;
            let descending = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) < 0.0
            } else {
                false
            };
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if descending {
                values.reverse();
            }
            Value::Array(values.into_iter().map(Value::Number).collect())
        },

        "FILTER" => {
            require_args(name, args, 2)?;
            let data = collect_values_as_vec(&args[0], ctx)?;
            let criteria = collect_values_as_vec(&args[1], ctx)?;
            let filtered: Vec<Value> = data
                .into_iter()
                .zip(criteria.iter())
                .filter(|(_, c)| c.is_truthy())
                .map(|(v, _)| v)
                .collect();
            Value::Array(filtered)
        },

        "SEQUENCE" => {
            // SEQUENCE(rows, [columns], [start], [step])
            require_args_range(name, args, 1, 4)?;
            let rows = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SEQUENCE: rows must be a number"))?
                as usize;
            let cols = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let start = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            let step = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };

            // Generate sequence
            let total = rows * cols;
            let values: Vec<Value> = (0..total)
                .map(|i| Value::Number(start + (i as f64) * step))
                .collect();
            Value::Array(values)
        },

        "RANDARRAY" => {
            // RANDARRAY([rows], [columns], [min], [max], [whole_number])
            require_args_range(name, args, 0, 5)?;
            let rows = if !args.is_empty() {
                evaluate(&args[0], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let cols = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let min = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let max = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            let whole_number = if args.len() > 4 {
                evaluate(&args[4], ctx)?.is_truthy()
            } else {
                false
            };

            let mut rng = rand::rng();
            let total = rows * cols;
            let values: Vec<Value> = if whole_number {
                (0..total)
                    .map(|_| {
                        Value::Number(
                            rng.random_range(min.floor() as i64..=max.floor() as i64) as f64
                        )
                    })
                    .collect()
            } else {
                (0..total)
                    .map(|_| Value::Number(rng.random_range(min..max)))
                    .collect()
            };
            Value::Array(values)
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
    fn test_unique() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Text("A".to_string()),
                Value::Text("B".to_string()),
                Value::Text("A".to_string()),
                Value::Text("C".to_string()),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        let result = eval("UNIQUE(t.data)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3); // A, B, C
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_countunique() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Text("A".to_string()),
                Value::Text("B".to_string()),
                Value::Text("A".to_string()),
                Value::Text("C".to_string()),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(
            eval("COUNTUNIQUE(t.data)", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_sort() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(3.0),
                Value::Number(1.0),
                Value::Number(4.0),
                Value::Number(1.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        let result = eval("SORT(t.data)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], Value::Number(1.0));
            assert_eq!(arr[4], Value::Number(5.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_sequence() {
        let ctx = EvalContext::new();

        // SEQUENCE(5) = [1, 2, 3, 4, 5]
        let result = eval("SEQUENCE(5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], Value::Number(1.0));
            assert_eq!(arr[4], Value::Number(5.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_sequence_with_params() {
        let ctx = EvalContext::new();

        // SEQUENCE(3, 1, 10, 5) = [10, 15, 20]
        let result = eval("SEQUENCE(3, 1, 10, 5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(10.0));
            assert_eq!(arr[1], Value::Number(15.0));
            assert_eq!(arr[2], Value::Number(20.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_randarray() {
        let ctx = EvalContext::new();

        // RANDARRAY(5) = 5 random numbers between 0 and 1
        let result = eval("RANDARRAY(5)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            for v in arr {
                if let Value::Number(n) = v {
                    assert!((0.0..1.0).contains(&n));
                } else {
                    panic!("Expected number");
                }
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_randarray_with_range() {
        let ctx = EvalContext::new();

        // RANDARRAY(3, 1, 1, 10, TRUE()) = 3 whole numbers between 1 and 10
        let result = eval("RANDARRAY(3, 1, 1, 10, TRUE())", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            for v in arr {
                if let Value::Number(n) = v {
                    assert!((1.0..=10.0).contains(&n));
                    assert_eq!(n.fract(), 0.0); // whole number
                } else {
                    panic!("Expected number");
                }
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_filter() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        // Create data array: [10, 20, 30, 40, 50]
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
                Value::Number(40.0),
                Value::Number(50.0),
            ],
        );

        // Create filter array: [true, false, true, false, true]
        table.insert(
            "include".to_string(),
            vec![
                Value::Boolean(true),
                Value::Boolean(false),
                Value::Boolean(true),
                Value::Boolean(false),
                Value::Boolean(true),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // FILTER should return [10, 30, 50]
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(10.0));
            assert_eq!(arr[1], Value::Number(30.0));
            assert_eq!(arr[2], Value::Number(50.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_filter_all_false() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        table.insert(
            "values".to_string(),
            vec![Value::Number(10.0), Value::Number(20.0)],
        );
        table.insert(
            "include".to_string(),
            vec![Value::Boolean(false), Value::Boolean(false)],
        );
        ctx.tables.insert("t".to_string(), table);

        // FILTER should return empty array
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_filter_all_true() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();

        table.insert(
            "values".to_string(),
            vec![
                Value::Number(100.0),
                Value::Number(200.0),
                Value::Number(300.0),
            ],
        );
        table.insert(
            "include".to_string(),
            vec![
                Value::Boolean(true),
                Value::Boolean(true),
                Value::Boolean(true),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // FILTER should return all values
        let result = eval("FILTER(t.values, t.include)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(100.0));
            assert_eq!(arr[1], Value::Number(200.0));
            assert_eq!(arr[2], Value::Number(300.0));
        } else {
            panic!("Expected array");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests (moved from tests/array.rs)
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_countunique_function() {
        let mut model = ParsedModel::new();

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Apple".to_string(),
                "Orange".to_string(),
                "Banana".to_string(),
            ]),
        ));
        sales.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0, 20.0]),
        ));
        model.add_table(sales);

        model.add_scalar(
            "unique_products".to_string(),
            Variable::new(
                "unique_products".to_string(),
                None,
                Some("=COUNTUNIQUE(sales.product)".to_string()),
            ),
        );

        model.add_scalar(
            "unique_quantities".to_string(),
            Variable::new(
                "unique_quantities".to_string(),
                None,
                Some("=COUNTUNIQUE(sales.quantity)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_products = result
            .scalars
            .get("unique_products")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(
            unique_products, 3.0,
            "Should have 3 unique products, got {unique_products}"
        );

        let unique_quantities = result
            .scalars
            .get("unique_quantities")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(
            unique_quantities, 3.0,
            "Should have 3 unique quantities, got {unique_quantities}"
        );
    }

    #[test]
    fn test_unique_function_as_count() {
        let mut model = ParsedModel::new();

        let mut flags = Table::new("flags".to_string());
        flags.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false, true, true, false]),
        ));
        model.add_table(flags);

        model.add_scalar(
            "unique_flags".to_string(),
            Variable::new(
                "unique_flags".to_string(),
                None,
                Some("=UNIQUE(flags.active)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_flags = result.scalars.get("unique_flags").unwrap().value.unwrap();
        assert_eq!(
            unique_flags, 2.0,
            "Should have 2 unique boolean values (true, false), got {unique_flags}"
        );
    }

    #[test]
    fn test_filter_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 25.0, 5.0, 30.0]),
        ));
        data.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Boolean(vec![true, true, false, true]),
        ));
        model.add_table(data);

        model.add_scalar(
            "filtered_sum".to_string(),
            Variable::new(
                "filtered_sum".to_string(),
                None,
                Some("=SUM(FILTER(data.value, data.include))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let filtered_sum = result.scalars.get("filtered_sum").unwrap().value.unwrap();
        assert_eq!(filtered_sum, 65.0, "SUM(FILTER(...)) should return 65.0");
    }

    #[test]
    fn test_sort_function_coverage() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![30.0, 10.0, 20.0, 40.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "min_value".to_string(),
            Variable::new(
                "min_value".to_string(),
                None,
                Some("=MIN(SORT(data.values))".to_string()),
            ),
        );
        model.add_scalar(
            "max_value".to_string(),
            Variable::new(
                "max_value".to_string(),
                None,
                Some("=MAX(SORT(data.values))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let min_val = result.scalars.get("min_value").unwrap().value.unwrap();
        assert_eq!(min_val, 10.0, "MIN(SORT(...)) should return 10.0");

        let max_val = result.scalars.get("max_value").unwrap().value.unwrap();
        assert_eq!(max_val, 40.0, "MAX(SORT(...)) should return 40.0");
    }

    #[test]
    fn test_sequence_function_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_sum".to_string(),
            Variable::new(
                "seq_sum".to_string(),
                None,
                Some("=SUM(SEQUENCE(5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_val = result.scalars.get("seq_sum").unwrap().value.unwrap();
        assert_eq!(sum_val, 15.0, "SUM(SEQUENCE(5)) should return 15.0");
    }

    #[test]
    fn test_sequence_function_with_start_step() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_custom".to_string(),
            Variable::new(
                "seq_custom".to_string(),
                None,
                Some("=SUM(SEQUENCE(4, 1, 10, 5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_val = result.scalars.get("seq_custom").unwrap().value.unwrap();
        assert_eq!(
            sum_val, 70.0,
            "SUM(SEQUENCE(4, 1, 10, 5)) should return 70.0"
        );
    }

    #[test]
    fn test_randarray_function_basic() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "rand_count".to_string(),
            Variable::new(
                "rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(5))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result.scalars.get("rand_count").unwrap().value.unwrap();
        assert_eq!(count_val, 5.0, "COUNT(RANDARRAY(5)) should return 5.0");
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Edge case tests (moved from tests/array_function_edge_cases.rs)
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(15.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_count() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(SEQUENCE(10))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_with_start() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5, 1, 10))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(60.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_with_step() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(5, 1, 1, 2))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(25.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_unique_all_same() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_unique_all_different() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_ascending_first() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(SORT(data.values), 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_filter_greater() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![0.0, 0.0, 0.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(9.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_all_same() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_all_diff() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    // ════════════════════════════════════════════════════════════════════════════
    // Additional tests moved from tests/array.rs
    // ════════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_with_dates() {
        let mut model = ParsedModel::new();

        let mut events = Table::new("events".to_string());
        events.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2024-01-15".to_string(),
                "2024-01-16".to_string(),
                "2024-01-15".to_string(),
                "2024-01-17".to_string(),
            ]),
        ));
        model.add_table(events);

        model.add_scalar(
            "unique_dates".to_string(),
            Variable::new(
                "unique_dates".to_string(),
                None,
                Some("=COUNTUNIQUE(events.date)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_dates = result.scalars.get("unique_dates").unwrap().value.unwrap();
        assert_eq!(unique_dates, 3.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_edge_cases() {
        let mut model = ParsedModel::new();

        let mut single = Table::new("single".to_string());
        single.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![42.0]),
        ));
        model.add_table(single);

        let mut same = Table::new("same".to_string());
        same.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(same);

        let mut different = Table::new("different".to_string());
        different.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(different);

        let mut floats = Table::new("floats".to_string());
        floats.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0]),
        ));
        model.add_table(floats);

        model.add_scalar(
            "single_unique".to_string(),
            Variable::new(
                "single_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(single.value)".to_string()),
            ),
        );

        model.add_scalar(
            "same_unique".to_string(),
            Variable::new(
                "same_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(same.value)".to_string()),
            ),
        );

        model.add_scalar(
            "different_unique".to_string(),
            Variable::new(
                "different_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(different.value)".to_string()),
            ),
        );

        model.add_scalar(
            "floats_unique".to_string(),
            Variable::new(
                "floats_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(floats.value)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        assert_eq!(
            result.scalars.get("single_unique").unwrap().value.unwrap(),
            1.0
        );
        assert_eq!(
            result.scalars.get("same_unique").unwrap().value.unwrap(),
            1.0
        );
        assert_eq!(
            result
                .scalars
                .get("different_unique")
                .unwrap()
                .value
                .unwrap(),
            5.0
        );
        assert_eq!(
            result.scalars.get("floats_unique").unwrap().value.unwrap(),
            2.0
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_empty_text_values() {
        let mut model = ParsedModel::new();

        let mut mixed = Table::new("mixed".to_string());
        mixed.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                String::new(),
                "Alice".to_string(),
                String::new(),
                "Bob".to_string(),
                "Alice".to_string(),
            ]),
        ));
        model.add_table(mixed);

        model.add_scalar(
            "unique_names".to_string(),
            Variable::new(
                "unique_names".to_string(),
                None,
                Some("=COUNTUNIQUE(mixed.name)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_names = result.scalars.get("unique_names").unwrap().value.unwrap();
        assert_eq!(unique_names, 3.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_in_expression() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "C".to_string(),
            ]),
        ));
        model.add_table(data);

        model.add_scalar(
            "unique_times_10".to_string(),
            Variable::new(
                "unique_times_10".to_string(),
                None,
                Some("=COUNTUNIQUE(data.category) * 10".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let result_val = result
            .scalars
            .get("unique_times_10")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(result_val, 30.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_numbers() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 1.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "unique".to_string(),
            Variable::new(
                "unique".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let unique = result.scalars.get("unique").unwrap().value.unwrap();
        assert!((unique - 3.0).abs() < 0.01);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_numbers_basic() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_rows_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=ROWS(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_filter_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        data.add_column(Column::new(
            "flags".to_string(),
            ColumnValue::Boolean(vec![true, false, true, false, true]),
        ));
        model.add_table(data);
        model.add_scalar(
            "sum".to_string(),
            Variable::new(
                "sum".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.flags))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let sum_result = result.scalars.get("sum").unwrap().value.unwrap();
        assert_eq!(sum_result, 9.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_unique_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_and_min() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "min".to_string(),
            Variable::new(
                "min".to_string(),
                None,
                Some("=MIN(SORT(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let min_val = result.scalars.get("min").unwrap().value.unwrap();
        assert_eq!(min_val, 1.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_function_zero_start() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "seq_max".to_string(),
            Variable::new(
                "seq_max".to_string(),
                None,
                Some("=MAX(SEQUENCE(3, 1, 0, 10))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let max_val = result.scalars.get("seq_max").unwrap().value.unwrap();
        assert_eq!(max_val, 20.0);
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_randarray_function_range() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "rand_count".to_string(),
            Variable::new(
                "rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );
        model.add_scalar(
            "rand_min".to_string(),
            Variable::new(
                "rand_min".to_string(),
                None,
                Some("=MIN(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );
        model.add_scalar(
            "rand_max".to_string(),
            Variable::new(
                "rand_max".to_string(),
                None,
                Some("=MAX(RANDARRAY(10, 1, 1, 10, TRUE()))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result.scalars.get("rand_count").unwrap().value.unwrap();
        assert_eq!(count_val, 10.0);

        let min_val = result.scalars.get("rand_min").unwrap().value.unwrap();
        assert!((1.0..=10.0).contains(&min_val));

        let max_val = result.scalars.get("rand_max").unwrap().value.unwrap();
        assert!((1.0..=10.0).contains(&max_val));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_randarray_function_larger_count() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "large_rand_count".to_string(),
            Variable::new(
                "large_rand_count".to_string(),
                None,
                Some("=COUNT(RANDARRAY(100))".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let count_val = result
            .scalars
            .get("large_rand_count")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(count_val, 100.0);
    }

    // ════════════════════════════════════════════════════════════════════════════
    // Additional tests moved from tests/array_function_edge_cases.rs
    // ════════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sequence_single() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(SEQUENCE(1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_unique_some_dups() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_unique_sum() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_ascending_last() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(SORT(data.values), 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sort_preserves_count() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 3.0, 1.0, 4.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(SORT(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_filter_less() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 0.0, 0.0, 0.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_filter_count() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![0.0, 0.0, 1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_filter_first_only() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        table.add_column(Column::new(
            "include".to_string(),
            ColumnValue::Number(vec![1.0, 0.0, 0.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(FILTER(data.values, data.include))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countunique_mixed() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }
}
