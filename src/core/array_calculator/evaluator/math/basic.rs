//! Basic math functions: ABS, SIGN, SQRT, POWER, MOD, PI, POW, E

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

pub fn eval_abs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("ABS", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ABS requires a number"))?;
    Ok(Value::Number(val.abs()))
}

pub fn eval_sqrt(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("SQRT", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("SQRT requires a number"))?;
    if val < 0.0 {
        return Err(EvalError::new("SQRT of negative number"));
    }
    Ok(Value::Number(val.sqrt()))
}

pub fn eval_mod(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("MOD", args, 2)?;
    let num = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
    let divisor = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
    if divisor == 0.0 {
        return Err(EvalError::new("MOD division by zero"));
    }
    // The exact remainder of the two binary values, with the divisor's sign:
    // Microsoft's n - d * INT(n / d) in exact arithmetic. Computing that
    // formula in floating point gave MOD(7.3, 0.1) = -5.8E-16 (a negative
    // result for a positive divisor) and MOD(1E15, 1E-12) = 0.0068, larger
    // than the divisor.
    let remainder = num % divisor;
    Ok(Value::Number(
        if remainder != 0.0 && (remainder < 0.0) != (divisor < 0.0) {
            remainder + divisor
        } else {
            remainder
        },
    ))
}

pub fn eval_power(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("POWER", args, 2)?;
    let base = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
    let exp = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
    super::super::power(base, exp)
}

pub fn eval_sign(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("SIGN", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("SIGN requires a number"))?;
    Ok(Value::Number(if val > 0.0 {
        1.0
    } else if val < 0.0 {
        -1.0
    } else {
        0.0
    }))
}

pub fn eval_pi(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("PI", args, 0)?;
    let _ = ctx;
    Ok(Value::Number(std::f64::consts::PI))
}

pub fn eval_pow(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("POW", args, 2)?;
    let base = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("POW requires numbers"))?;
    let exp = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("POW requires numbers"))?;
    super::super::power(base, exp)
}

pub fn eval_e(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("E", args, 0)?;
    let _ = ctx;
    Ok(Value::Number(std::f64::consts::E))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_abs() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ABS(-5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("ABS(5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("ABS(0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_sqrt() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SQRT(16)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("SQRT(0)", &ctx).unwrap(), Value::Number(0.0));
        assert!(eval("SQRT(-4)", &ctx).is_err());
    }

    #[test]
    fn test_mod() {
        let ctx = EvalContext::new();
        assert_eq!(eval("MOD(10, 3)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("MOD(-5, 3)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("MOD(5, -3)", &ctx).unwrap(), Value::Number(-1.0));
        assert!(eval("MOD(10, 0)", &ctx).is_err());
    }

    #[test]
    fn test_power() {
        let ctx = EvalContext::new();
        assert_eq!(eval("POWER(2, 3)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_sign() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SIGN(5)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("SIGN(-5)", &ctx).unwrap(), Value::Number(-1.0));
        assert_eq!(eval("SIGN(0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_pi() {
        let ctx = EvalContext::new();
        let result = eval("PI()", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - std::f64::consts::PI).abs() < 0.0001));
    }

    #[test]
    fn test_pow_and_e() {
        let ctx = EvalContext::new();
        assert_eq!(eval("POW(2, 3)", &ctx).unwrap(), Value::Number(8.0));
        let e = eval("E()", &ctx).unwrap();
        assert!(matches!(e, Value::Number(n) if (n - std::f64::consts::E).abs() < 0.0001));
    }

    #[test]
    fn test_sqrt_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![4.0, 9.0, 16.0]),
        ));
        table.add_row_formula("s".to_string(), "=SQRT(v)".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        let nums = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("s")
            .unwrap()
            .values
            .numbers()
            .unwrap();
        let nums = nums.as_ref();
        assert_eq!(nums, &[2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_abs_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![-10.0, 5.0, -3.0]),
        ));
        table.add_row_formula("a".to_string(), "=ABS(v)".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        let nums = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("a")
            .unwrap()
            .values
            .numbers()
            .unwrap();
        let nums = nums.as_ref();
        assert_eq!(nums, &[10.0, 5.0, 3.0]);
    }

    #[test]
    fn test_scalar_formulas() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "base".to_string(),
            Variable::new("base".to_string(), Some(2.0), None),
        );
        model.add_scalar(
            "sq".to_string(),
            Variable::new("sq".to_string(), None, Some("=POWER(base, 2)".to_string())),
        );
        model.add_scalar(
            "rt".to_string(),
            Variable::new("rt".to_string(), None, Some("=SQRT(sq)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!((result.scalars.get("sq").unwrap().value.unwrap() - 4.0).abs() < 0.01);
        assert!((result.scalars.get("rt").unwrap().value.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_power_operator() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "p".to_string(),
            Variable::new("p".to_string(), None, Some("=2^10".to_string())),
        );
        model.add_scalar(
            "n".to_string(),
            Variable::new("n".to_string(), None, Some("=2^(-1)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(result.scalars.get("p").unwrap().value, Some(1024.0));
        assert_eq!(result.scalars.get("n").unwrap().value, Some(0.5));
    }

    #[test]
    fn test_double_triple_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "d".to_string(),
            Variable::new("d".to_string(), None, Some("=--5".to_string())),
        );
        model.add_scalar(
            "t".to_string(),
            Variable::new("t".to_string(), None, Some("=---5".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(result.scalars.get("d").unwrap().value, Some(5.0));
        assert_eq!(result.scalars.get("t").unwrap().value, Some(-5.0));
    }

    #[test]
    fn test_mod_edge_cases() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pp".to_string(),
            Variable::new("pp".to_string(), None, Some("=MOD(5, 3)".to_string())),
        );
        model.add_scalar(
            "np".to_string(),
            Variable::new("np".to_string(), None, Some("=MOD(-5, 3)".to_string())),
        );
        model.add_scalar(
            "pn".to_string(),
            Variable::new("pn".to_string(), None, Some("=MOD(5, -3)".to_string())),
        );
        model.add_scalar(
            "nn".to_string(),
            Variable::new("nn".to_string(), None, Some("=MOD(-5, -3)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(result.scalars.get("pp").unwrap().value, Some(2.0));
        assert_eq!(result.scalars.get("np").unwrap().value, Some(1.0));
        assert_eq!(result.scalars.get("pn").unwrap().value, Some(-1.0));
        assert_eq!(result.scalars.get("nn").unwrap().value, Some(-2.0));
    }

    #[test]
    fn test_pi_e_integration() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "pi".to_string(),
            Variable::new("pi".to_string(), None, Some("=PI()".to_string())),
        );
        model.add_scalar(
            "e".to_string(),
            Variable::new("e".to_string(), None, Some("=E()".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!(
            (result.scalars.get("pi").unwrap().value.unwrap() - std::f64::consts::PI).abs()
                < 0.000_001
        );
        assert!(
            (result.scalars.get("e").unwrap().value.unwrap() - std::f64::consts::E).abs()
                < 0.000_001
        );
    }

    #[test]
    fn test_pow_integration() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "p".to_string(),
            Variable::new("p".to_string(), None, Some("=POW(2, 8)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!((result.scalars.get("p").unwrap().value.unwrap() - 256.0).abs() < 0.0001);
    }

    /// Fixes for what the generated cases found (ADR-062) that the engines cannot
    /// check: answers Forge gives on its own, with the reason, and the exact
    /// values where every engine loses digits.
    mod generated_cases {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn mod_is_the_exact_remainder() {
            // n - d * INT(n / d) in floating point gave -5.8E-16 and 0.0068 (above d)
            let r = h::number("MOD(7.3, 0.1)");
            assert!(r > 0.0 && r < 0.1, "{r}");
            let r = h::number("MOD(1E15, 1E-12)");
            assert!((0.0..1e-12).contains(&r), "{r}");
            h::is_exactly("MOD(-7, 3)", 2.0);
            h::is_exactly("MOD(7, -3)", -2.0);
        }
    }

    /// An error in any argument is passed on, and text where a number is
    /// needed is refused; PI and E take no argument
    #[test]
    fn each_argument_is_checked() {
        use super::super::super::tests::{each_argument, fails_with};
        for (name, message) in [
            ("ABS", "ABS requires a number"),
            ("SQRT", "SQRT requires a number"),
            ("SIGN", "SIGN requires a number"),
        ] {
            each_argument(name, &[("4", Some(message))]);
        }
        for name in ["MOD", "POWER", "POW"] {
            let message = format!("{name} requires numbers");
            each_argument(name, &[("7", Some(&message)), ("2", Some(&message))]);
        }
        fails_with("PI(1)", "PI requires 0 argument(s), got 1");
        fails_with("E(1)", "E requires 0 argument(s), got 1");
    }
}
