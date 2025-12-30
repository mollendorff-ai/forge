//! COUNTIF and COUNTIFS function implementations
//!
//! COUNTIF counts the number of cells in a range that meet a single criterion.
//! COUNTIFS counts cells that meet multiple criteria across multiple ranges.
//!
//! Syntax:
//! - COUNTIF(range, criteria)
//! - COUNTIFS(criteria_range1, criteria1, [criteria_range2, criteria2], ...)

use super::super::{
    collect_values_as_vec, evaluate, matches_criteria, require_args, EvalContext, EvalError, Expr,
    Value,
};

/// Evaluate COUNTIF function
pub fn eval_countif(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("COUNTIF", args, 2)?;
    let range_vals = collect_values_as_vec(&args[0], ctx)?;
    let criteria = evaluate(&args[1], ctx)?;
    let count = range_vals
        .iter()
        .filter(|v| matches_criteria(v, &criteria))
        .count();
    Ok(Value::Number(count as f64))
}

/// Evaluate COUNTIFS function
pub fn eval_countifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
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
    Ok(Value::Number(count as f64))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

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
        assert_eq!(
            eval("COUNTIF(t.scores, \">70\")", &ctx).unwrap(),
            Value::Number(3.0)
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
        model.add_scalar(
            "passing_count".to_string(),
            Variable::new(
                "passing_count".to_string(),
                None,
                Some("=COUNTIF(data.scores, \">=85\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(
            result.scalars.get("passing_count").unwrap().value,
            Some(4.0)
        );
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
        assert!((result.scalars.get("count_not_a").unwrap().value.unwrap() - 2.0).abs() < 0.01);
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
        let result = calculator.calculate_all().unwrap();
        assert_eq!(result.scalars.get("count_a").unwrap().value, Some(3.0));
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
        model.add_scalar(
            "count_result".to_string(),
            Variable::new(
                "count_result".to_string(),
                None,
                Some("=COUNTIFS(data.category, \"A\", data.value, \">20\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(result.scalars.get("count_result").unwrap().value, Some(2.0));
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
        assert!((result.scalars.get("active_a").unwrap().value.unwrap() - 2.0).abs() < 0.01);
    }

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
}
