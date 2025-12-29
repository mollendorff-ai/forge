//! Conditional aggregation functions: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, etc.

use super::{
    collect_values_as_vec, evaluate, matches_criteria, require_args, require_args_range,
    EvalContext, EvalError, Expr, Value,
};

/// Try to evaluate a conditional aggregation function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SUMIF" => {
            require_args_range(name, args, 2, 3)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let sum_range_vals = if args.len() > 2 {
                collect_values_as_vec(&args[2], ctx)?
            } else {
                range_vals.clone()
            };
            let mut total = 0.0;
            for (i, val) in range_vals.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(sum_val) = sum_range_vals.get(i) {
                        if let Some(n) = sum_val.as_number() {
                            total += n;
                        }
                    }
                }
            }
            Value::Number(total)
        },

        "COUNTIF" => {
            require_args(name, args, 2)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let count = range_vals
                .iter()
                .filter(|v| matches_criteria(v, &criteria))
                .count();
            Value::Number(count as f64)
        },

        "AVERAGEIF" => {
            require_args_range(name, args, 2, 3)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let avg_range_vals = if args.len() > 2 {
                collect_values_as_vec(&args[2], ctx)?
            } else {
                range_vals.clone()
            };
            let mut total = 0.0;
            let mut count = 0;
            for (i, val) in range_vals.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(avg_val) = avg_range_vals.get(i) {
                        if let Some(n) = avg_val.as_number() {
                            total += n;
                            count += 1;
                        }
                    }
                }
            }
            if count == 0 {
                return Err(EvalError::new("AVERAGEIF: no matching values"));
            }
            Value::Number(total / count as f64)
        },

        "SUMIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "SUMIFS requires sum_range, criteria_range1, criteria1, ...",
                ));
            }
            let sum_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; sum_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let total: f64 = sum_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .sum();
            Value::Number(total)
        },

        "COUNTIFS" => {
            if args.len() < 2 || !args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "COUNTIFS requires criteria_range1, criteria1, ...",
                ));
            }
            let first_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; first_range.len()];
            for pair in args.chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let count = matches.iter().filter(|&&m| m).count();
            Value::Number(count as f64)
        },

        "AVERAGEIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "AVERAGEIFS requires avg_range, criteria_range1, criteria1, ...",
                ));
            }
            let avg_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; avg_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = avg_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                return Err(EvalError::new("AVERAGEIFS: no matching values"));
            }
            Value::Number(matching.iter().sum::<f64>() / matching.len() as f64)
        },

        "MAXIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "MAXIFS requires max_range, criteria_range1, criteria1, ...",
                ));
            }
            let max_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; max_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = max_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                Value::Number(0.0)
            } else {
                Value::Number(matching.iter().copied().fold(f64::NEG_INFINITY, f64::max))
            }
        },

        "MINIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "MINIFS requires min_range, criteria_range1, criteria1, ...",
                ));
            }
            let min_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; min_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = min_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                Value::Number(0.0)
            } else {
                Value::Number(matching.iter().copied().fold(f64::INFINITY, f64::min))
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
    fn test_sumif() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "category".to_string(),
            vec![
                Value::Text("A".to_string()),
                Value::Text("B".to_string()),
                Value::Text("A".to_string()),
            ],
        );
        table.insert(
            "amount".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // Sum amounts where category = "A"
        assert_eq!(
            eval("SUMIF(t.category, \"A\", t.amount)", &ctx).unwrap(),
            Value::Number(40.0)
        );
    }

    #[test]
    fn test_countif() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "scores".to_string(),
            vec![
                Value::Number(50.0),
                Value::Number(75.0),
                Value::Number(80.0),
                Value::Number(90.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // Count scores > 70
        assert_eq!(
            eval("COUNTIF(t.scores, \">70\")", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_averageif() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "scores".to_string(),
            vec![
                Value::Number(50.0),
                Value::Number(75.0),
                Value::Number(80.0),
                Value::Number(90.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // Average of scores >= 75
        let result = eval("AVERAGEIF(t.scores, \">=75\")", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 81.666).abs() < 0.01));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration tests (moved from tests/conditional.rs)
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_sumif_numeric_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 300.0, 50.0]),
        ));
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 1500.0, 3000.0, 500.0]),
        ));
        model.add_table(table);

        let high_revenue = Variable::new(
            "high_revenue".to_string(),
            None,
            Some("=SUMIF(sales.amount, \">100\", sales.revenue)".to_string()),
        );
        model.add_scalar("high_revenue".to_string(), high_revenue);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("high_revenue").unwrap().value,
            Some(6500.0)
        );
    }

    #[test]
    fn test_countif_numeric_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "scores".to_string(),
            ColumnValue::Number(vec![85.0, 92.0, 78.0, 95.0, 88.0, 72.0]),
        ));
        model.add_table(table);

        let passing_count = Variable::new(
            "passing_count".to_string(),
            None,
            Some("=COUNTIF(data.scores, \">=85\")".to_string()),
        );
        model.add_scalar("passing_count".to_string(), passing_count);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("passing_count").unwrap().value,
            Some(4.0)
        );
    }

    #[test]
    fn test_averageif_numeric_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("employees".to_string());
        table.add_column(Column::new(
            "years".to_string(),
            ColumnValue::Number(vec![2.0, 5.0, 3.0, 8.0, 1.0]),
        ));
        table.add_column(Column::new(
            "salary".to_string(),
            ColumnValue::Number(vec![50000.0, 75000.0, 60000.0, 95000.0, 45000.0]),
        ));
        model.add_table(table);

        let avg_senior_salary = Variable::new(
            "avg_senior_salary".to_string(),
            None,
            Some("=AVERAGEIF(employees.years, \">=3\", employees.salary)".to_string()),
        );
        model.add_scalar("avg_senior_salary".to_string(), avg_senior_salary);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        let expected = (75000.0 + 60000.0 + 95000.0) / 3.0;
        let actual = result
            .scalars
            .get("avg_senior_salary")
            .unwrap()
            .value
            .unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn test_sumifs_multiple_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "North".to_string(),
                "South".to_string(),
                "North".to_string(),
                "East".to_string(),
                "North".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 300.0, 250.0]),
        ));
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 1500.0, 3000.0, 2500.0]),
        ));
        model.add_table(table);

        let north_high_revenue = Variable::new(
            "north_high_revenue".to_string(),
            None,
            Some(
                "=SUMIFS(sales.revenue, sales.region, \"North\", sales.amount, \">=150\")"
                    .to_string(),
            ),
        );
        model.add_scalar("north_high_revenue".to_string(), north_high_revenue);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("north_high_revenue").unwrap().value,
            Some(4000.0)
        );
    }

    #[test]
    fn test_countifs_multiple_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "C".to_string(),
                "A".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);

        let count_result = Variable::new(
            "count_result".to_string(),
            None,
            Some("=COUNTIFS(data.category, \"A\", data.value, \">20\")".to_string()),
        );
        model.add_scalar("count_result".to_string(), count_result);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(result.scalars.get("count_result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_maxifs_multiple_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "North".to_string(),
                "South".to_string(),
                "North".to_string(),
                "North".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "quarter".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0]),
        ));
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 1500.0, 1800.0]),
        ));
        model.add_table(table);

        let max_result = Variable::new(
            "max_result".to_string(),
            None,
            Some(
                "=MAXIFS(sales.revenue, sales.region, \"North\", sales.quarter, \"2\")".to_string(),
            ),
        );
        model.add_scalar("max_result".to_string(), max_result);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("max_result").unwrap().value,
            Some(1800.0)
        );
    }

    #[test]
    fn test_minifs_multiple_criteria() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("inventory".to_string());
        table.add_column(Column::new(
            "product".to_string(),
            ColumnValue::Text(vec![
                "Widget".to_string(),
                "Gadget".to_string(),
                "Widget".to_string(),
                "Widget".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![100.0, 50.0, 75.0, 120.0]),
        ));
        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 15.0, 9.0, 11.0]),
        ));
        model.add_table(table);

        let min_result = Variable::new(
            "min_result".to_string(),
            None,
            Some(
                "=MINIFS(inventory.price, inventory.product, \"Widget\", inventory.quantity, \">=75\")"
                    .to_string(),
            ),
        );
        model.add_scalar("min_result".to_string(), min_result);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(result.scalars.get("min_result").unwrap().value, Some(9.0));
    }

    // ════════════════════════════════════════════════════════════════════════════
    // Additional tests moved from tests/conditional.rs
    // ════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_sumif_less_than_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "sum_le_30".to_string(),
            Variable::new(
                "sum_le_30".to_string(),
                None,
                Some("=SUMIF(data.values, \"<=30\", data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let sum = result.scalars.get("sum_le_30").unwrap().value.unwrap();
        assert!((sum - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_sumif_not_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 20.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "sum_ne_20".to_string(),
            Variable::new(
                "sum_ne_20".to_string(),
                None,
                Some("=SUMIF(data.values, \"<>20\", data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let sum = result.scalars.get("sum_ne_20").unwrap().value.unwrap();
        assert!((sum - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_sumif_less_than() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "sum_lt_30".to_string(),
            Variable::new(
                "sum_lt_30".to_string(),
                None,
                Some("=SUMIF(data.values, \"<30\", data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let sum = result.scalars.get("sum_lt_30").unwrap().value.unwrap();
        assert!((sum - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_sumif_equal_explicit() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 20.0, 50.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "sum_eq_20".to_string(),
            Variable::new(
                "sum_eq_20".to_string(),
                None,
                Some("=SUMIF(data.values, \"=20\", data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let sum = result.scalars.get("sum_eq_20").unwrap().value.unwrap();
        assert!((sum - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_countif_text_not_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("products".to_string());
        table.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "C".to_string(),
            ]),
        ));
        model.add_table(table);
        model.add_scalar(
            "count_not_a".to_string(),
            Variable::new(
                "count_not_a".to_string(),
                None,
                Some("=COUNTIF(products.category, \"<>A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let count = result.scalars.get("count_not_a").unwrap().value.unwrap();
        assert!((count - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_countif_text_with_equal_prefix() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("products".to_string());
        table.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Apple".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        model.add_table(table);
        model.add_scalar(
            "count_apple".to_string(),
            Variable::new(
                "count_apple".to_string(),
                None,
                Some("=COUNTIF(products.category, \"=Apple\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let count = result.scalars.get("count_apple").unwrap().value.unwrap();
        assert!((count - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_sumifs_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "North".to_string(),
                "South".to_string(),
                "North".to_string(),
                "South".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 300.0]),
        ));
        data.add_column(Column::new(
            "year".to_string(),
            ColumnValue::Number(vec![2024.0, 2024.0, 2023.0, 2024.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "north_2024".to_string(),
            Variable::new(
                "north_2024".to_string(),
                None,
                Some(
                    "=SUMIFS(sales.amount, sales.region, \"North\", sales.year, \"2024\")"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let sum = result.scalars.get("north_2024").unwrap().value.unwrap();
        assert!((sum - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_countifs_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "status".to_string(),
            ColumnValue::Text(vec![
                "active".to_string(),
                "active".to_string(),
                "inactive".to_string(),
                "active".to_string(),
            ]),
        ));
        model.add_table(data);
        model.add_scalar(
            "active_a".to_string(),
            Variable::new(
                "active_a".to_string(),
                None,
                Some(
                    "=COUNTIFS(products.category, \"A\", products.status, \"active\")".to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let count = result.scalars.get("active_a").unwrap().value.unwrap();
        assert!((count - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_averageifs_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("scores".to_string());
        data.add_column(Column::new(
            "grade".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "score".to_string(),
            ColumnValue::Number(vec![95.0, 85.0, 90.0, 88.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "avg_a".to_string(),
            Variable::new(
                "avg_a".to_string(),
                None,
                Some("=AVERAGEIFS(scores.score, scores.grade, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let avg = result.scalars.get("avg_a").unwrap().value.unwrap();
        assert!((avg - 91.0).abs() < 0.01);
    }

    #[test]
    fn test_sumif_scalar() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 50.0, 300.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "total_above_100".to_string(),
            Variable::new(
                "total_above_100".to_string(),
                None,
                Some("=SUMIF(sales.amount, \">100\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let total = result
            .scalars
            .get("total_above_100")
            .unwrap()
            .value
            .unwrap();
        assert!((total - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_countif_category_a() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count_a".to_string(),
            Variable::new(
                "count_a".to_string(),
                None,
                Some("=COUNTIF(products.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("count_a") {
            assert_eq!(scalar.value.unwrap(), 3.0);
        }
    }

    #[test]
    fn test_averageif_low_scores() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("scores".to_string());
        data.add_column(Column::new(
            "score".to_string(),
            ColumnValue::Number(vec![50.0, 75.0, 30.0, 90.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "avg_low".to_string(),
            Variable::new(
                "avg_low".to_string(),
                None,
                Some("=AVERAGEIF(scores.score, \"<60\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let avg = result.scalars.get("avg_low").unwrap().value.unwrap();
        assert!((avg - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_sumifs_region_and_amount() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "East".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 50.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "east_large".to_string(),
            Variable::new(
                "east_large".to_string(),
                None,
                Some(
                    "=SUMIFS(sales.amount, sales.region, \"East\", sales.amount, \">75\")"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("east_large") {
            assert_eq!(scalar.value.unwrap(), 250.0);
        }
    }

    #[test]
    fn test_maxifs_scalar() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 20.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "max_a_price".to_string(),
            Variable::new(
                "max_a_price".to_string(),
                None,
                Some("=MAXIFS(products.price, products.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("max_a_price") {
            assert_eq!(scalar.value.unwrap(), 30.0);
        }
    }

    #[test]
    fn test_minifs_criteria() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 20.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "min_a_price".to_string(),
            Variable::new(
                "min_a_price".to_string(),
                None,
                Some("=MINIFS(products.price, products.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        if let Some(scalar) = model.scalars.get("min_a_price") {
            assert_eq!(scalar.value.unwrap(), 10.0);
        }
    }

    #[test]
    fn test_sumif_with_range() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "east_total".to_string(),
            Variable::new(
                "east_total".to_string(),
                None,
                Some("=SUMIF(sales.region, \"East\", sales.amount)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let total = result.scalars.get("east_total").unwrap().value.unwrap();
        assert!((total - 250.0).abs() < 0.01);
    }

    #[test]
    fn test_countifs_function_v2() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("orders".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "East".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 50.0, 150.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTIFS(orders.region, \"East\", orders.amount, \">75\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let count = result.scalars.get("count").unwrap().value.unwrap();
        assert!((count - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_averageifs_function_v2() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec!["A".to_string(), "B".to_string(), "A".to_string()]),
        ));
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "avg_a".to_string(),
            Variable::new(
                "avg_a".to_string(),
                None,
                Some("=AVERAGEIFS(data.value, data.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let avg = result.scalars.get("avg_a").unwrap().value.unwrap();
        assert!((avg - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_sumifs_multi_criteria() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "West".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 250.0]),
        ));
        data.add_column(Column::new(
            "qty".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 15.0, 25.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "total".to_string(),
            Variable::new(
                "total".to_string(),
                None,
                Some(
                    "=SUMIFS(sales.amount, sales.region, \"East\", sales.qty, \">10\")".to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let total = result.scalars.get("total").unwrap().value.unwrap();
        assert!((total - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_countifs_multi_criteria() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("sales".to_string());
        data.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "East".to_string(),
                "West".to_string(),
                "East".to_string(),
                "West".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 150.0, 250.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTIFS(sales.region, \"East\", sales.amount, \">100\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let count = result.scalars.get("count").unwrap().value.unwrap();
        assert!((count - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_averageifs_text_criteria() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "B".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "avg".to_string(),
            Variable::new(
                "avg".to_string(),
                None,
                Some("=AVERAGEIFS(data.values, data.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let avg = result.scalars.get("avg").unwrap().value.unwrap();
        assert!((avg - 20.0).abs() < 0.01);
    }

    // IFS and SWITCH tests

    #[test]
    fn test_ifs_first_condition_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(10>5, 100, 5>10, 200, 1>0, 300)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[test]
    fn test_ifs_second_condition_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(5>10, 100, 10>5, 200, 1>0, 300)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_ifs_with_table_data() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("thresholds".to_string());
        table.add_column(Column::new(
            "low".to_string(),
            ColumnValue::Number(vec![10.0]),
        ));
        table.add_column(Column::new(
            "high".to_string(),
            ColumnValue::Number(vec![100.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "grade".to_string(),
            Variable::new(
                "grade".to_string(),
                None,
                Some(
                    "=IFS(85>=SUM(thresholds.high), 1, 85>=SUM(thresholds.low), 2, 1>0, 3)"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("grade").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_ifs_no_match_error() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(5>10, 100, 3>10, 200)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_ifs_with_final_true() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=IFS(5>10, 100, 3>10, 200, 1>0, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(999.0));
    }

    #[test]
    fn test_switch_match_first() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(1, 1, 10, 2, 20, 3, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_switch_match_middle() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(2, 1, 10, 2, 20, 3, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
    }

    #[test]
    fn test_switch_with_default() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(5, 1, 10, 2, 20, 3, 30, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(999.0));
    }

    #[test]
    fn test_switch_no_match_no_default() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SWITCH(5, 1, 10, 2, 20, 3, 30)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_switch_with_numeric_result() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "day".to_string(),
            Variable::new("day".to_string(), Some(2.0), None),
        );
        model.add_scalar(
            "day_code".to_string(),
            Variable::new(
                "day_code".to_string(),
                None,
                Some("=SWITCH(day, 1, 100, 2, 200, 3, 300, 999)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("day_code").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_switch_with_table_lookup() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("codes".to_string());
        table.add_column(Column::new(
            "status".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "priority".to_string(),
            Variable::new(
                "priority".to_string(),
                None,
                Some("=SWITCH(SUM(codes.status), 3, 1, 6, 2, 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("priority").unwrap().value, Some(2.0));
    }

    // ════════════════════════════════════════════════════════════════════════════
    // Edge case tests from tests/conditional_function_edge_cases.rs
    // ════════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumif_greater() {
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
                Some("=SUMIF(data.values, \">3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(9.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumif_less() {
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
                Some("=SUMIF(data.values, \"<3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumif_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 2.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIF(data.values, \"=2\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumif_all_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIF(data.values, \">0\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumif_none_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIF(data.values, \">10\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countif_greater() {
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
                Some("=COUNTIF(data.values, \">3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countif_less() {
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
                Some("=COUNTIF(data.values, \"<3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countif_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 2.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIF(data.values, \"=2\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countif_all_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIF(data.values, \">0\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countif_none_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIF(data.values, \">10\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_averageif_greater() {
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
                Some("=AVERAGEIF(data.values, \">3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.5));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_averageif_less() {
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
                Some("=AVERAGEIF(data.values, \"<3\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.5));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_averageif_all_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![2.0, 4.0, 6.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=AVERAGEIF(data.values, \">0\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_averageif_equal() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=AVERAGEIF(data.values, \"=10\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumifs_two_criteria() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        table.add_column(Column::new(
            "cat1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        table.add_column(Column::new(
            "cat2".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 6.0, 6.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIFS(data.values, data.cat1, \"=1\", data.cat2, \">5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumifs_all_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sumifs_none_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUMIFS(data.values, data.cat, \">5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countifs_two_criteria() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "cat1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        table.add_column(Column::new(
            "cat2".to_string(),
            ColumnValue::Number(vec![5.0, 6.0, 5.0, 6.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIFS(data.cat1, \"=1\", data.cat2, \">5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countifs_all_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIFS(data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_countifs_none_match() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTIFS(data.cat, \">5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_maxifs_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MAXIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_minifs_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MINIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_maxifs_all() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MAXIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_minifs_all() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MINIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }
}
