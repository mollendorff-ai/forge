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
        }

        "COUNTUNIQUE" => {
            require_args(name, args, 1)?;
            let values = collect_values_as_vec(&args[0], ctx)?;
            let mut seen = std::collections::HashSet::new();
            for v in values {
                seen.insert(v.as_text());
            }
            Value::Number(seen.len() as f64)
        }

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
        }

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
        }

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
        }

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
}
