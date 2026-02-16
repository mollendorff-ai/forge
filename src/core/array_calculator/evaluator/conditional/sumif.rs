//! SUMIF and SUMIFS function implementations
//!
//! SUMIF sums values in a range that meet a single criterion.
//! SUMIFS sums values that meet multiple criteria across multiple ranges.
//!
//! Syntax:
//! - SUMIF(range, criteria, [`sum_range`])
//! - `SUMIFS(sum_range`, `criteria_range1`, criteria1, [`criteria_range2`, criteria2], ...)

use super::super::{
    collect_values_as_vec, evaluate, matches_criteria, require_args_range, EvalContext, EvalError,
    Expr, Value,
};

/// Evaluate SUMIF function
pub fn eval_sumif(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("SUMIF", args, 2, 3)?;
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
    Ok(Value::Number(total))
}

/// Evaluate SUMIFS function
pub fn eval_sumifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
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
    Ok(Value::Number(total))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
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
        assert_eq!(
            eval("SUMIF(t.category, \"A\", t.amount)", &ctx).unwrap(),
            Value::Number(40.0)
        );
    }

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
        model.add_scalar(
            "high_revenue".to_string(),
            Variable::new(
                "high_revenue".to_string(),
                None,
                Some("=SUMIF(sales.amount, \">100\", sales.revenue)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(
            result.scalars.get("high_revenue").unwrap().value,
            Some(6500.0)
        );
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
        assert!((result.scalars.get("east_total").unwrap().value.unwrap() - 250.0).abs() < 0.01);
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
        model.add_scalar(
            "north_high_revenue".to_string(),
            Variable::new(
                "north_high_revenue".to_string(),
                None,
                Some(
                    "=SUMIFS(sales.revenue, sales.region, \"North\", sales.amount, \">=150\")"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(
            result.scalars.get("north_high_revenue").unwrap().value,
            Some(4000.0)
        );
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
        assert!((result.scalars.get("total").unwrap().value.unwrap() - 150.0).abs() < 0.01);
    }

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
}
