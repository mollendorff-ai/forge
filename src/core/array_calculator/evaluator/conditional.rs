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
        }

        "COUNTIF" => {
            require_args(name, args, 2)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let count = range_vals
                .iter()
                .filter(|v| matches_criteria(v, &criteria))
                .count();
            Value::Number(count as f64)
        }

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
        }

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
        }

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
        }

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
        }

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
                Value::Number(matching.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
            }
        }

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
                Value::Number(matching.iter().cloned().fold(f64::INFINITY, f64::min))
            }
        }

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
