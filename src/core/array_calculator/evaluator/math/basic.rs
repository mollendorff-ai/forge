//! Basic math functions: ABS, SIGN, SQRT, POWER, MOD, PI (DEMO), POW, E (Enterprise)

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

pub fn eval_abs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("ABS", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("ABS requires a number"))?;
    Ok(Value::Number(val.abs()))
}

pub fn eval_sqrt(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("SQRT", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("SQRT requires a number"))?;
    if val < 0.0 {
        return Err(EvalError::new("SQRT of negative number"));
    }
    Ok(Value::Number(val.sqrt()))
}

pub fn eval_mod(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("MOD", args, 2)?;
    let num = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
    let divisor = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
    if divisor == 0.0 {
        return Err(EvalError::new("MOD division by zero"));
    }
    Ok(Value::Number(num - divisor * (num / divisor).floor()))
}

pub fn eval_power(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("POWER", args, 2)?;
    let base = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
    let exp = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
    Ok(Value::Number(base.powf(exp)))
}

pub fn eval_sign(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("SIGN", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
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
        .as_number()
        .ok_or_else(|| EvalError::new("POW requires numbers"))?;
    let exp = evaluate(&args[1], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("POW requires numbers"))?;
    Ok(Value::Number(base.powf(exp)))
}

pub fn eval_e(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("E", args, 0)?;
    let _ = ctx;
    Ok(Value::Number(std::f64::consts::E))
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
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
        if let ColumnValue::Number(nums) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("s")
            .unwrap()
            .values
        {
            assert_eq!(nums, &[2.0, 3.0, 4.0]);
        }
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
        if let ColumnValue::Number(nums) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("a")
            .unwrap()
            .values
        {
            assert_eq!(nums, &[10.0, 5.0, 3.0]);
        }
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
                < 0.000001
        );
        assert!(
            (result.scalars.get("e").unwrap().value.unwrap() - std::f64::consts::E).abs()
                < 0.000001
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
}
