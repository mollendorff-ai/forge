//! Math & Precision Functions (v1.1.0)
//! ROUND, ROUNDUP, ROUNDDOWN, CEILING, FLOOR, MOD, SQRT, POWER

use crate::error::{ForgeError, ForgeResult};

use super::ArrayCalculator;

impl ArrayCalculator {
    /// Evaluate ROUND function: ROUND(number, digits)
    pub(super) fn eval_round(&self, value: f64, digits: i32) -> f64 {
        let multiplier = 10_f64.powi(digits);
        (value * multiplier).round() / multiplier
    }

    /// Evaluate ROUNDUP function: ROUNDUP(number, digits)
    pub(super) fn eval_roundup(&self, value: f64, digits: i32) -> f64 {
        let multiplier = 10_f64.powi(digits);
        (value * multiplier).ceil() / multiplier
    }

    /// Evaluate ROUNDDOWN function: ROUNDDOWN(number, digits)
    pub(super) fn eval_rounddown(&self, value: f64, digits: i32) -> f64 {
        let multiplier = 10_f64.powi(digits);
        (value * multiplier).floor() / multiplier
    }

    /// Evaluate CEILING function: CEILING(number, significance)
    pub(super) fn eval_ceiling(&self, value: f64, significance: f64) -> f64 {
        if significance == 0.0 {
            return value;
        }
        (value / significance).ceil() * significance
    }

    /// Evaluate FLOOR function: FLOOR(number, significance)
    pub(super) fn eval_floor(&self, value: f64, significance: f64) -> f64 {
        if significance == 0.0 {
            return value;
        }
        (value / significance).floor() * significance
    }

    /// Evaluate MOD function: MOD(number, divisor)
    pub(super) fn eval_mod(&self, value: f64, divisor: f64) -> ForgeResult<f64> {
        if divisor == 0.0 {
            return Err(ForgeError::Eval("MOD: Division by zero".to_string()));
        }
        Ok(value % divisor)
    }

    /// Evaluate SQRT function: SQRT(number)
    pub(super) fn eval_sqrt(&self, value: f64) -> ForgeResult<f64> {
        if value < 0.0 {
            return Err(ForgeError::Eval(
                "SQRT: Cannot compute square root of negative number".to_string(),
            ));
        }
        Ok(value.sqrt())
    }

    /// Evaluate POWER function: POWER(number, exponent)
    pub(super) fn eval_power(&self, base: f64, exponent: f64) -> f64 {
        base.powf(exponent)
    }
}
