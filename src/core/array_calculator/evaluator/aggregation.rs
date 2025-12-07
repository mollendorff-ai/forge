//! Aggregation functions: SUM, AVERAGE, COUNT, MIN, MAX, PRODUCT, MEDIAN

use super::{collect_numeric_values, evaluate, EvalContext, EvalError, Expr, Value};

/// Try to evaluate an aggregation function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SUM" => {
            let values = collect_numeric_values(args, ctx)?;
            Value::Number(values.iter().sum())
        }

        "PRODUCT" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                Value::Number(0.0)
            } else {
                Value::Number(values.iter().product())
            }
        }

        "AVERAGE" | "AVG" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("AVERAGE of empty set"));
            }
            Value::Number(values.iter().sum::<f64>() / values.len() as f64)
        }

        "MIN" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MIN of empty set"))?
        }

        "MAX" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MAX of empty set"))?
        }

        "COUNT" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| v.as_number().is_some()).count();
                    }
                    Value::Number(_) => count += 1,
                    _ => {}
                }
            }
            Value::Number(count as f64)
        }

        "COUNTA" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| !matches!(v, Value::Null)).count();
                    }
                    Value::Null => {}
                    _ => count += 1,
                }
            }
            Value::Number(count as f64)
        }

        "MEDIAN" => {
            let mut values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("MEDIAN of empty set"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = values.len() / 2;
            if values.len() % 2 == 0 {
                Value::Number((values[mid - 1] + values[mid]) / 2.0)
            } else {
                Value::Number(values[mid])
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
    fn test_median() {
        let ctx = EvalContext::new();
        assert_eq!(eval("MEDIAN(1, 3, 5)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(
            eval("MEDIAN(1, 2, 3, 4)", &ctx).unwrap(),
            Value::Number(2.5)
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
        assert_eq!(eval("COUNTA(t.values)", &ctx).unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_product() {
        let ctx = EvalContext::new();
        assert_eq!(eval("PRODUCT(2, 3, 4)", &ctx).unwrap(), Value::Number(24.0));
    }
}
