//! Text functions: CONCAT, UPPER, LOWER, TRIM, LEN, LEFT, RIGHT, MID

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a text function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "CONCAT" | "CONCATENATE" => {
            let mut result = String::new();
            for arg in args {
                let val = evaluate(arg, ctx)?;
                result.push_str(&val.as_text());
            }
            Value::Text(result)
        }

        "UPPER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_uppercase())
        }

        "LOWER" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().to_lowercase())
        }

        "TRIM" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Text(val.as_text().trim().to_string())
        }

        "LEN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Number(val.as_text().len() as f64)
        }

        "LEFT" => {
            require_args_range(name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().take(n).collect();
            Value::Text(chars.into_iter().collect())
        }

        "RIGHT" => {
            require_args_range(name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(n);
            Value::Text(chars[start..].iter().collect())
        }

        "MID" => {
            require_args(name, args, 3)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let start = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize;
            let length = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as usize;

            let chars: Vec<char> = text.chars().collect();
            // Excel MID is 1-indexed
            let start_idx = start.saturating_sub(1);
            let end_idx = (start_idx + length).min(chars.len());
            Value::Text(chars[start_idx..end_idx].iter().collect())
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    #[test]
    fn test_text_functions() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("UPPER(\"hello\")", &ctx).unwrap(),
            Value::Text("HELLO".to_string())
        );
        assert_eq!(
            eval("LOWER(\"HELLO\")", &ctx).unwrap(),
            Value::Text("hello".to_string())
        );
        assert_eq!(
            eval("CONCAT(\"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("abc".to_string())
        );
        assert_eq!(eval("LEN(\"hello\")", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(
            eval("LEFT(\"hello\", 2)", &ctx).unwrap(),
            Value::Text("he".to_string())
        );
        assert_eq!(
            eval("RIGHT(\"hello\", 2)", &ctx).unwrap(),
            Value::Text("lo".to_string())
        );
        assert_eq!(
            eval("MID(\"hello\", 2, 3)", &ctx).unwrap(),
            Value::Text("ell".to_string())
        );
    }

    #[test]
    fn test_trim() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("TRIM(\"  hello  \")", &ctx).unwrap(),
            Value::Text("hello".to_string())
        );
    }
}
