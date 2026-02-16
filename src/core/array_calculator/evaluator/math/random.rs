//! Random functions: RAND, RANDBETWEEN

// Random casts: f64 bounds to i64 (RANDBETWEEN integer bounds).
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use super::super::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

use rand::RngExt;

/// Evaluate RAND function - returns a random number between 0 and 1
pub fn eval_rand(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("RAND", args, 0)?;
    let _ = ctx;
    let mut rng = rand::rng();
    Ok(Value::Number(rng.random::<f64>()))
}

/// Evaluate RANDBETWEEN function - returns a random integer between two values
pub fn eval_randbetween(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("RANDBETWEEN", args, 2, 2)?;
    let bottom = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("RANDBETWEEN requires numbers"))?;
    let top = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("RANDBETWEEN requires numbers"))?;

    if bottom > top {
        return Err(EvalError::new("RANDBETWEEN: bottom must be <= top"));
    }

    let bottom_int = bottom.ceil() as i64;
    let top_int = top.floor() as i64;

    if bottom_int > top_int {
        return Err(EvalError::new("RANDBETWEEN: no integers in range"));
    }

    let mut rng = rand::rng();
    let result = rng.random_range(bottom_int..=top_int);
    Ok(Value::Number(result as f64))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_rand() {
        let ctx = EvalContext::new();
        for _ in 0..10 {
            let result = eval("RAND()", &ctx).unwrap();
            if let Value::Number(n) = result {
                assert!((0.0..1.0).contains(&n));
            }
        }
    }

    #[test]
    fn test_randbetween() {
        let ctx = EvalContext::new();
        for _ in 0..10 {
            let result = eval("RANDBETWEEN(1, 10)", &ctx).unwrap();
            if let Value::Number(n) = result {
                assert!((1.0..=10.0).contains(&n));
                assert!(n.fract() == 0.0);
            }
        }
    }

    #[test]
    fn test_randbetween_same_values() {
        let ctx = EvalContext::new();
        assert_eq!(eval("RANDBETWEEN(5, 5)", &ctx).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_randbetween_invalid_range() {
        let ctx = EvalContext::new();
        assert!(eval("RANDBETWEEN(10, 1)", &ctx).is_err());
    }

    #[test]
    fn test_rand_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "random".to_string(),
            Variable::new("random".to_string(), None, Some("=RAND()".to_string())),
        );

        let result = ArrayCalculator::new(model)
            .calculate_all()
            .expect("Should calculate");
        let val = result.scalars.get("random").unwrap().value.unwrap();
        assert!((0.0..1.0).contains(&val));
    }

    #[test]
    fn test_randbetween_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "random".to_string(),
            Variable::new(
                "random".to_string(),
                None,
                Some("=RANDBETWEEN(1, 100)".to_string()),
            ),
        );

        let result = ArrayCalculator::new(model)
            .calculate_all()
            .expect("Should calculate");
        let val = result.scalars.get("random").unwrap().value.unwrap();
        assert!((1.0..=100.0).contains(&val));
    }
}
