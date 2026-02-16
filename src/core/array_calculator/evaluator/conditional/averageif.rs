//! AVERAGEIF and AVERAGEIFS function implementations
//!
//! AVERAGEIF calculates the average of values in a range that meet a single criterion.
//! AVERAGEIFS calculates the average of values that meet multiple criteria.
//!
//! Syntax:
//! - AVERAGEIF(range, criteria, [`average_range`])
//! - `AVERAGEIFS(average_range`, `criteria_range1`, criteria1, [`criteria_range2`, criteria2], ...)

// AVERAGEIF casts: matching count (usize) to f64 for average computation.
#![allow(clippy::cast_precision_loss)]

use super::super::{
    collect_values_as_vec, evaluate, matches_criteria, require_args_range, EvalContext, EvalError,
    Expr, Value,
};

/// Evaluate AVERAGEIF function
pub fn eval_averageif(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("AVERAGEIF", args, 2, 3)?;
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
    Ok(Value::Number(total / f64::from(count)))
}

/// Evaluate AVERAGEIFS function
pub fn eval_averageifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
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
    Ok(Value::Number(
        matching.iter().sum::<f64>() / matching.len() as f64,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

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
        let result = eval("AVERAGEIF(t.scores, \">=75\")", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 81.666).abs() < 0.01));
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
        model.add_scalar(
            "avg_senior_salary".to_string(),
            Variable::new(
                "avg_senior_salary".to_string(),
                None,
                Some("=AVERAGEIF(employees.years, \">=3\", employees.salary)".to_string()),
            ),
        );
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
        assert!((result.scalars.get("avg_a").unwrap().value.unwrap() - 91.0).abs() < 0.01);
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
        assert!((result.scalars.get("avg").unwrap().value.unwrap() - 20.0).abs() < 0.01);
    }

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
}
