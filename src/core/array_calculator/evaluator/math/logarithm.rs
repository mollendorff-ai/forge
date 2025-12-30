//! Logarithmic and exponential functions: EXP, LN, LOG10 (DEMO)
//! Enterprise: LOG

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// Evaluate EXP function - returns e raised to power of x
pub fn eval_exp(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("EXP", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("EXP requires a number"))?;
    Ok(Value::Number(val.exp()))
}

/// Evaluate LN function - returns natural logarithm
pub fn eval_ln(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("LN", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("LN requires a number"))?;
    if val <= 0.0 {
        return Err(EvalError::new("LN of non-positive number"));
    }
    Ok(Value::Number(val.ln()))
}

/// Evaluate LOG10 function - returns base-10 logarithm
pub fn eval_log10(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("LOG10", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("LOG10 requires a number"))?;
    if val <= 0.0 {
        return Err(EvalError::new("LOG10 of non-positive number"));
    }
    Ok(Value::Number(val.log10()))
}

/// Evaluate LOG function - returns base-10 logarithm (Enterprise)
pub fn eval_log(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("LOG", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new("LOG requires a number"))?;
    if val <= 0.0 {
        return Err(EvalError::new("LOG of non-positive number"));
    }
    Ok(Value::Number(val.log10()))
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};

    // Imports for enterprise integration tests
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_exp_ln_log10() {
        let ctx = EvalContext::new();
        let exp_result = eval("EXP(1)", &ctx).unwrap();
        assert!(matches!(exp_result, Value::Number(n) if (n - std::f64::consts::E).abs() < 0.0001));
        let ln_result = eval("LN(2.718281828)", &ctx).unwrap();
        assert!(matches!(ln_result, Value::Number(n) if (n - 1.0).abs() < 0.0001));
        assert_eq!(eval("LOG10(100)", &ctx).unwrap(), Value::Number(2.0));
        assert_eq!(eval("LOG10(1)", &ctx).unwrap(), Value::Number(0.0));
        assert_eq!(eval("LOG10(10)", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_ln_errors() {
        let ctx = EvalContext::new();
        assert!(eval("LN(0)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("non-positive"));
        assert!(eval("LN(-1)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("non-positive"));
    }

    #[test]
    fn test_log10_errors() {
        let ctx = EvalContext::new();
        assert!(eval("LOG10(0)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("non-positive"));
        assert!(eval("LOG10(-5)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("non-positive"));
    }

    #[test]
    fn test_log_alias() {
        let ctx = EvalContext::new();
        assert_eq!(eval("LOG(100)", &ctx).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_exp_ln_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![0.0, 1.0, 2.0]),
        ));
        table.add_row_formula("exp_x".to_string(), "=EXP(x)".to_string());
        model.add_table(table);

        let result = ArrayCalculator::new(model)
            .calculate_all()
            .expect("Should calculate");
        if let ColumnValue::Number(vals) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("exp_x")
            .unwrap()
            .values
        {
            assert!((vals[0] - 1.0).abs() < 0.001);
            assert!((vals[1] - std::f64::consts::E).abs() < 0.001);
        }
    }

    #[test]
    fn test_exp_ln_scalars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "exp1".to_string(),
            Variable::new("exp1".to_string(), None, Some("=EXP(1)".to_string())),
        );
        model.add_scalar(
            "lne".to_string(),
            Variable::new("lne".to_string(), None, Some("=LN(2.718)".to_string())),
        );
        model.add_scalar(
            "log1000".to_string(),
            Variable::new(
                "log1000".to_string(),
                None,
                Some("=LOG10(1000)".to_string()),
            ),
        );

        let result = ArrayCalculator::new(model)
            .calculate_all()
            .expect("Should calculate");
        assert!(
            (result.scalars.get("exp1").unwrap().value.unwrap() - std::f64::consts::E).abs()
                < 0.001
        );
        assert!((result.scalars.get("lne").unwrap().value.unwrap() - 1.0).abs() < 0.01);
        assert!((result.scalars.get("log1000").unwrap().value.unwrap() - 3.0).abs() < 0.001);
    }
}
