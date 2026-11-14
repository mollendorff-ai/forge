//! Rounding functions: ROUND, ROUNDUP, ROUNDDOWN, FLOOR, CEILING, MROUND, TRUNC, INT

// Rounding casts: f64 decimal-place counts to i32 (small bounded integers, typically 0..15).
#![allow(clippy::cast_possible_truncation)]

use super::super::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};
use crate::core::array_calculator::evaluator::number_or;

/// `x` to 15 significant digits, as Excel holds a number before it rounds:
/// 1.005 * 100 is 100.49999999999999 in binary and 100.5 in Excel, so
/// `ROUND(1.005, 2)` is 1.01 there.
fn excel_digits(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    format!("{x:.14e}").parse().unwrap_or(x)
}

/// `val` rounded to `decimals` places with `op` (round, ceil, floor) applied
/// to its magnitude, the sign kept: the rounding of ROUND, ROUNDUP,
/// ROUNDDOWN and TRUNC.
fn round_with(val: f64, decimals: f64, op: fn(f64) -> f64) -> f64 {
    let decimals = decimals.trunc().clamp(-400.0, 400.0) as i32;
    let magnitude = val.abs();
    let rounded = if decimals >= 0 {
        let multiplier = 10_f64.powi(decimals);
        let scaled = magnitude * multiplier;
        // Past 2^52 a double has no fraction left to round
        if !scaled.is_finite() || scaled >= 4_503_599_627_370_496.0 {
            return val;
        }
        op(excel_digits(scaled)) / multiplier
    } else {
        let divisor = 10_f64.powi(-decimals);
        op(excel_digits(magnitude / divisor)) * divisor
    };
    if val < 0.0 {
        -rounded
    } else {
        rounded
    }
}

/// The decimals argument of ROUND and the like, 0 when it is left out
fn decimals_arg(args: &[Expr], ctx: &EvalContext) -> Result<f64, EvalError> {
    if args.len() > 1 {
        number_or(&args[1], ctx, 0.0)
    } else {
        Ok(0.0)
    }
}

pub fn eval_round(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("ROUND", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ROUND requires a number"))?;
    Ok(Value::Number(round_with(
        val,
        decimals_arg(args, ctx)?,
        f64::round,
    )))
}

pub fn eval_roundup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("ROUNDUP", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ROUNDUP requires a number"))?;
    Ok(Value::Number(round_with(
        val,
        decimals_arg(args, ctx)?,
        f64::ceil,
    )))
}

pub fn eval_rounddown(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("ROUNDDOWN", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("ROUNDDOWN requires a number"))?;
    Ok(Value::Number(round_with(
        val,
        decimals_arg(args, ctx)?,
        f64::floor,
    )))
}

pub fn eval_floor(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("FLOOR", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("FLOOR requires a number"))?;
    let sig = if args.len() > 1 {
        number_or(&args[1], ctx, 1.0)?
    } else {
        1.0
    };
    if val == 0.0 {
        return Ok(Value::Number(0.0));
    }
    if sig == 0.0 {
        return Err(EvalError::new("#DIV/0!: FLOOR: the significance is 0"));
    }
    if val > 0.0 && sig < 0.0 {
        return Err(EvalError::new(
            "#NUM!: FLOOR: a positive number needs a positive significance",
        ));
    }
    Ok(Value::Number(excel_digits(val / sig).floor() * sig))
}

pub fn eval_ceiling(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("CEILING", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("CEILING requires a number"))?;
    let sig = if args.len() > 1 {
        number_or(&args[1], ctx, 1.0)?
    } else {
        1.0
    };
    if sig == 0.0 || val == 0.0 {
        return Ok(Value::Number(0.0));
    }
    if val > 0.0 && sig < 0.0 {
        return Err(EvalError::new(
            "#NUM!: CEILING: a positive number needs a positive significance",
        ));
    }
    Ok(Value::Number(excel_digits(val / sig).ceil() * sig))
}

/// MROUND(number, multiple): the nearest multiple, a half going away from 0
pub fn eval_mround(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("MROUND", args, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("MROUND requires a number"))?;
    let multiple = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("MROUND requires a multiple"))?;
    if multiple == 0.0 {
        return Ok(Value::Number(0.0));
    }
    if val * multiple < 0.0 {
        return Err(EvalError::new(
            "MROUND: the number and the multiple must have the same sign (#NUM!)",
        ));
    }
    Ok(Value::Number(
        excel_digits(val / multiple).round() * multiple,
    ))
}

pub fn eval_int(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("INT", args, 1)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("INT requires a number"))?;
    Ok(Value::Number(val.floor()))
}

pub fn eval_trunc(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("TRUNC", args, 1, 2)?;
    let val = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("TRUNC requires a number"))?;
    Ok(Value::Number(round_with(
        val,
        decimals_arg(args, ctx)?,
        f64::floor,
    )))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_round() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROUND(3.567, 2)", &ctx).unwrap(), Value::Number(3.57));
        assert_eq!(eval("ROUND(3.567)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUND(2.5, 0)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("ROUND(-2.5, 0)", &ctx).unwrap(), Value::Number(-3.0));
        assert_eq!(
            eval("ROUND(1234.5, -2)", &ctx).unwrap(),
            Value::Number(1200.0)
        );
    }

    #[test]
    fn test_roundup_rounddown() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROUNDUP(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUNDUP(-3.2)", &ctx).unwrap(), Value::Number(-4.0));
        assert_eq!(eval("ROUNDDOWN(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("ROUNDDOWN(-3.9)", &ctx).unwrap(), Value::Number(-3.0));
    }

    #[test]
    fn test_floor_ceiling() {
        let ctx = EvalContext::new();
        assert_eq!(eval("FLOOR(3.7)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("FLOOR(17, 5)", &ctx).unwrap(), Value::Number(15.0));
        // Excel: FLOOR with a significance of 0 is #DIV/0!, CEILING's is 0
        assert!(eval("FLOOR(3.5, 0)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#DIV/0!"));
        assert_eq!(eval("CEILING(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("CEILING(13, 5)", &ctx).unwrap(), Value::Number(15.0));
        assert_eq!(eval("CEILING(3.5, 0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_int_trunc() {
        let ctx = EvalContext::new();
        assert_eq!(eval("INT(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("INT(-5.9)", &ctx).unwrap(), Value::Number(-6.0));
        assert_eq!(eval("TRUNC(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("TRUNC(-3.9)", &ctx).unwrap(), Value::Number(-3.0));
        assert_eq!(eval("TRUNC(3.567, 2)", &ctx).unwrap(), Value::Number(3.56));
    }

    #[test]
    fn test_round_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![1.456, 2.789, 3.123]),
        ));
        table.add_row_formula("r".to_string(), "=ROUND(v, 1)".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        if let ColumnValue::Number(nums) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("r")
            .unwrap()
            .values
        {
            assert_eq!(nums, &[1.5, 2.8, 3.1]);
        }
    }

    #[test]
    fn test_ceiling_floor_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![1.1, 2.3, 4.7]),
        ));
        table.add_row_formula("c".to_string(), "=CEILING(v, 1)".to_string());
        table.add_row_formula("f".to_string(), "=FLOOR(v, 1)".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        if let ColumnValue::Number(nums) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("c")
            .unwrap()
            .values
        {
            assert_eq!(nums, &[2.0, 3.0, 5.0]);
        }
        if let ColumnValue::Number(nums) = &result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("f")
            .unwrap()
            .values
        {
            assert_eq!(nums, &[1.0, 2.0, 4.0]);
        }
    }

    #[test]
    fn test_rounding_scalars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "v".to_string(),
            Variable::new("v".to_string(), Some(3.567), None),
        );
        model.add_scalar(
            "r".to_string(),
            Variable::new("r".to_string(), None, Some("=ROUND(v, 2)".to_string())),
        );
        model.add_scalar(
            "u".to_string(),
            Variable::new("u".to_string(), None, Some("=ROUNDUP(v, 1)".to_string())),
        );
        model.add_scalar(
            "d".to_string(),
            Variable::new("d".to_string(), None, Some("=ROUNDDOWN(v, 1)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!((result.scalars.get("r").unwrap().value.unwrap() - 3.57).abs() < 0.01);
        assert!((result.scalars.get("u").unwrap().value.unwrap() - 3.6).abs() < 0.01);
        assert!((result.scalars.get("d").unwrap().value.unwrap() - 3.5).abs() < 0.01);
    }

    #[test]
    fn test_negative_precision() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "r1".to_string(),
            Variable::new(
                "r1".to_string(),
                None,
                Some("=ROUND(1234.5, -2)".to_string()),
            ),
        );
        model.add_scalar(
            "r2".to_string(),
            Variable::new("r2".to_string(), None, Some("=ROUND(1250, -2)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(result.scalars.get("r1").unwrap().value, Some(1200.0));
        assert_eq!(result.scalars.get("r2").unwrap().value, Some(1300.0));
    }

    #[test]
    fn test_int_trunc_scalars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "ip".to_string(),
            Variable::new("ip".to_string(), None, Some("=INT(5.9)".to_string())),
        );
        model.add_scalar(
            "in".to_string(),
            Variable::new("in".to_string(), None, Some("=INT(-5.9)".to_string())),
        );
        model.add_scalar(
            "tp".to_string(),
            Variable::new("tp".to_string(), None, Some("=TRUNC(5.9)".to_string())),
        );
        model.add_scalar(
            "tn".to_string(),
            Variable::new("tn".to_string(), None, Some("=TRUNC(-5.9)".to_string())),
        );
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(result.scalars.get("ip").unwrap().value, Some(5.0));
        assert_eq!(result.scalars.get("in").unwrap().value, Some(-6.0));
        assert_eq!(result.scalars.get("tp").unwrap().value, Some(5.0));
        assert_eq!(result.scalars.get("tn").unwrap().value, Some(-5.0));
    }
}
