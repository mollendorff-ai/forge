//! Random functions: RAND, RANDBETWEEN

use super::super::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};
use crate::core::precision;

use num_traits::ToPrimitive;
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
        .as_cell_number()
        .ok_or_else(|| EvalError::new("RANDBETWEEN requires numbers"))?;
    let top = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("RANDBETWEEN requires numbers"))?;

    if bottom > top {
        return Err(EvalError::new("RANDBETWEEN: bottom must be <= top"));
    }

    // The whole numbers from bottom to top, which an i64 must hold
    let Some((bottom_int, top_int)) = bottom.ceil().to_i64().zip(top.floor().to_i64()) else {
        return Err(EvalError::new(format!(
            "#NUM!: RANDBETWEEN: {bottom} to {top} is past the whole numbers it counts"
        )));
    };

    if bottom_int > top_int {
        return Err(EvalError::new("RANDBETWEEN: no integers in range"));
    }

    let mut rng = rand::rng();
    let result = rng.random_range(bottom_int..=top_int);
    Ok(Value::Number(precision::whole(result)))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::{eval, number_in};
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_rand() {
        let ctx = EvalContext::new();
        for _ in 0..10 {
            let n = number_in("RAND()", &ctx);
            assert!((0.0..1.0).contains(&n), "{n}");
        }
    }

    #[test]
    fn test_randbetween() {
        let ctx = EvalContext::new();
        for _ in 0..10 {
            let n = number_in("RANDBETWEEN(1, 10)", &ctx);
            assert!((1.0..=10.0).contains(&n), "{n}");
            assert!(n.fract().abs() < f64::EPSILON, "{n}");
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

    /// The whole numbers from bottom (rounded up) to top (rounded down);
    /// none in an i64 is #NUM!
    #[test]
    fn randbetween_draws_whole_numbers_an_i64_holds() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("RANDBETWEEN(2.5, 3.5)", 3.0);
        h::fails_with("RANDBETWEEN(2.5, 2.7)", "no integers");
        h::fails_with("RANDBETWEEN(1, 1E+20)", "#NUM!");
        h::fails_with("RANDBETWEEN(-1E+20, 1)", "#NUM!");
    }

    #[test]
    fn each_argument_is_checked() {
        use super::super::super::tests::{each_argument, fails_with};
        fails_with("RAND(1)", "RAND requires 0 argument(s), got 1");
        fails_with(
            "RANDBETWEEN(1)",
            "RANDBETWEEN requires 2-2 arguments, got 1",
        );
        each_argument(
            "RANDBETWEEN",
            &[
                ("1", Some("RANDBETWEEN requires numbers")),
                ("2", Some("RANDBETWEEN requires numbers")),
            ],
        );
    }
}
