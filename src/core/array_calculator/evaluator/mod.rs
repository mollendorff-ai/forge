//! Formula evaluator for the array calculator
//!
//! Evaluates an AST to produce a result value. Supports both scalar and
//! array (row-wise) evaluation modes.
//!
//! Function implementations are organized into submodules by category.

// Evaluator casts: array lengths and indices between f64, usize, and integer types.
// Values are bounded by practical worksheet sizes (well within f64 mantissa).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

// Demo modules (always included)
mod aggregation;
mod dates;
mod financial;
mod logical;
mod lookup;
mod math;
mod statistical;
mod text;
mod trig;

// Extended evaluator modules
mod advanced;
mod array;
mod conditional;
mod forge;
mod info;

use super::parser::{Expr, Reference};
use std::collections::HashMap;

/// Value type that can be returned from evaluation
#[derive(Debug, Clone)]
pub enum Value {
    /// A numeric value
    Number(f64),
    /// A text value
    Text(String),
    /// A boolean value
    Boolean(bool),
    /// An array of values (for table columns)
    Array(Vec<Value>),
    /// A lambda function value (parameter names, body expression)
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// Null/empty value
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Null, Self::Null) => true,
            _ => false, // Lambdas and mismatched types don't compare equal
        }
    }
}

impl Value {
    /// Try to convert to f64
    /// For text, tries numeric parsing first, then date parsing (YYYY-MM-DD -> Excel serial)
    ///
    /// # Panics
    ///
    /// Cannot panic in practice: the Excel epoch date (1899-12-30) is always valid.
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(s) => {
                // First try direct numeric parse
                if let Ok(n) = s.parse::<f64>() {
                    return Some(n);
                }
                // Then try date parsing (YYYY-MM-DD format → Excel serial)
                if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    // Excel serial: days since 1899-12-30
                    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                    let days = (date - base).num_days();
                    return Some(days as f64);
                }
                None
            },
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            // Arrays in scalar context return their length
            Self::Array(arr) => Some(arr.len() as f64),
            Self::Lambda { .. } | Self::Null => None,
        }
    }

    /// Try to convert to string
    pub fn as_text(&self) -> String {
        match self {
            Self::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            },
            Self::Text(s) => s.clone(),
            Self::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Self::Null => String::new(),
            Self::Array(arr) => {
                let strs: Vec<String> = arr.iter().map(Self::as_text).collect();
                format!("[{}]", strs.join(", "))
            },
            Self::Lambda { params, .. } => {
                format!("LAMBDA({})", params.join(", "))
            },
        }
    }

    /// Try to convert to boolean
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Number(n) => Some(*n != 0.0),
            Self::Text(s) => {
                let upper = s.to_uppercase();
                if upper == "TRUE" || upper == "1" {
                    Some(true)
                } else if upper == "FALSE" || upper == "0" {
                    Some(false)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Check if value is truthy
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        self.as_bool().unwrap_or(false)
    }
}

/// Evaluation context containing variables and tables
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Scalar variables (name -> value)
    pub scalars: HashMap<String, Value>,
    /// Table data (`table_name` -> `column_name` -> values)
    pub tables: HashMap<String, HashMap<String, Vec<Value>>>,
    /// Scenarios (`scenario_name` -> `variable_name` -> value)
    pub scenarios: HashMap<String, HashMap<String, f64>>,
    /// Current row index for row-wise evaluation (None for scalar mode)
    pub current_row: Option<usize>,
    /// Number of rows in current table context
    pub row_count: Option<usize>,
}

impl EvalContext {
    /// Create a new empty context
    #[must_use]
    pub fn new() -> Self {
        Self {
            scalars: HashMap::new(),
            tables: HashMap::new(),
            scenarios: HashMap::new(),
            current_row: None,
            row_count: None,
        }
    }

    /// Get a scalar value by name
    #[must_use]
    pub fn get_scalar(&self, name: &str) -> Option<&Value> {
        self.scalars.get(name)
    }

    /// Get a table column
    #[must_use]
    pub fn get_column(&self, table: &str, column: &str) -> Option<&Vec<Value>> {
        self.tables.get(table).and_then(|t| t.get(column))
    }

    /// Set to row-wise mode with given row index
    #[must_use]
    pub const fn with_row(mut self, row: usize, count: usize) -> Self {
        self.current_row = Some(row);
        self.row_count = Some(count);
        self
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Error during evaluation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError {
    pub message: String,
}

impl EvalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Eval error: {}", self.message)
    }
}

impl std::error::Error for EvalError {}

/// Evaluate an expression in the given context
///
/// # Errors
///
/// Returns `EvalError` if the expression cannot be evaluated (e.g., unknown
/// function, type mismatch, or division by zero).
pub fn evaluate(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::Number(n) => Ok(Value::Number(*n)),

        Expr::Text(s) => Ok(Value::Text(s.clone())),

        Expr::Boolean(b) => Ok(Value::Boolean(*b)),

        Expr::Reference(reference) => evaluate_reference(reference, ctx),

        Expr::ArrayIndex { array, index } => {
            let arr_val = evaluate(array, ctx)?;
            let idx_val = evaluate(index, ctx)?;

            let idx = idx_val
                .as_number()
                .ok_or_else(|| EvalError::new("Array index must be a number"))?
                as usize;

            match arr_val {
                Value::Array(arr) => arr
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| EvalError::new(format!("Array index {idx} out of bounds"))),
                _ => Err(EvalError::new("Cannot index non-array value")),
            }
        },

        Expr::FunctionCall { name, args } => evaluate_function(name, args, ctx),

        Expr::CallResult { callable, args } => {
            // Evaluate the callable expression
            let callable_val = evaluate(callable, ctx)?;

            // It must be a Lambda
            match callable_val {
                Value::Lambda { params, body } => {
                    // Create new context with lambda parameters bound to arguments
                    if args.len() != params.len() {
                        return Err(EvalError::new(format!(
                            "Lambda expects {} arguments, got {}",
                            params.len(),
                            args.len()
                        )));
                    }

                    let mut new_ctx = ctx.clone();
                    for (param, arg_expr) in params.iter().zip(args.iter()) {
                        let value = evaluate(arg_expr, ctx)?;
                        new_ctx.scalars.insert(param.clone(), value);
                    }

                    // Evaluate the body with the new context
                    evaluate(&body, &new_ctx)
                },
                _ => Err(EvalError::new("Cannot call non-lambda value")),
            }
        },

        Expr::BinaryOp { op, left, right } => {
            let left_val = evaluate(left, ctx)?;
            let right_val = evaluate(right, ctx)?;
            evaluate_binary_op(op, &left_val, &right_val)
        },

        Expr::UnaryOp { op, operand } => {
            let val = evaluate(operand, ctx)?;
            evaluate_unary_op(op, &val)
        },

        Expr::Range { start, end } => {
            // Ranges are typically used within functions like INDIRECT
            // For now, return as text representation
            let start_val = evaluate(start, ctx)?;
            let end_val = evaluate(end, ctx)?;
            Ok(Value::Text(format!(
                "{}:{}",
                start_val.as_text(),
                end_val.as_text()
            )))
        },
    }
}

/// Evaluate a reference (scalar or table.column)
fn evaluate_reference(reference: &Reference, ctx: &EvalContext) -> Result<Value, EvalError> {
    match reference {
        Reference::Scalar(name) => {
            let value = ctx
                .get_scalar(name)
                .cloned()
                .ok_or_else(|| EvalError::new(format!("Unknown variable: {name}")))?;

            // In row-wise mode, if the value is an array, extract current row
            if let Some(row) = ctx.current_row {
                if let Value::Array(arr) = &value {
                    return arr
                        .get(row)
                        .cloned()
                        .ok_or_else(|| EvalError::new(format!("Row {row} out of bounds")));
                }
            }
            Ok(value)
        },

        Reference::TableColumn { table, column } => {
            // First try as a section.scalar reference (e.g., thresholds.min_value)
            let scalar_key = format!("{table}.{column}");
            if let Some(value) = ctx.scalars.get(&scalar_key) {
                return Ok(value.clone());
            }

            // Fall back to table.column lookup
            let col = ctx
                .get_column(table, column)
                .ok_or_else(|| EvalError::new(format!("Unknown column: {table}.{column}")))?;

            // In row-wise mode, validate row count matches and return single value
            if let Some(row) = ctx.current_row {
                // Validate cross-table row count matches current context
                if let Some(expected_count) = ctx.row_count {
                    if col.len() != expected_count {
                        return Err(EvalError::new(format!(
                            "Row count mismatch: {}.{} has {} rows but expected {}",
                            table,
                            column,
                            col.len(),
                            expected_count
                        )));
                    }
                }
                col.get(row)
                    .cloned()
                    .ok_or_else(|| EvalError::new(format!("Row {row} out of bounds")))
            } else {
                Ok(Value::Array(col.clone()))
            }
        },
    }
}

/// Evaluate a binary operation
// Arithmetic + comparison + logical operators require many match arms.
#[allow(clippy::too_many_lines)]
fn evaluate_binary_op(op: &str, left: &Value, right: &Value) -> Result<Value, EvalError> {
    match op {
        // Arithmetic operators
        "+" => {
            // Handle text concatenation
            if matches!(left, Value::Text(_)) || matches!(right, Value::Text(_)) {
                Ok(Value::Text(format!(
                    "{}{}",
                    left.as_text(),
                    right.as_text()
                )))
            } else {
                let l = left
                    .as_number()
                    .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
                let r = right
                    .as_number()
                    .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
                Ok(Value::Number(l + r))
            }
        },
        "-" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l - r))
        },
        "*" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l * r))
        },
        "/" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            if r == 0.0 {
                Err(EvalError::new("Division by zero"))
            } else {
                Ok(Value::Number(l / r))
            }
        },
        "^" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l.powf(r)))
        },

        // String concatenation operator
        "&" => Ok(Value::Text(format!(
            "{}{}",
            left.as_text(),
            right.as_text()
        ))),

        // Comparison operators
        "=" => Ok(Value::Boolean(values_equal(left, right))),
        "<>" => Ok(Value::Boolean(!values_equal(left, right))),
        "<" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l < r))
        },
        ">" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l > r))
        },
        "<=" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l <= r))
        },
        ">=" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l >= r))
        },

        _ => Err(EvalError::new(format!("Unknown operator: {op}"))),
    }
}

/// Check if two values are equal
pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => (l - r).abs() < 1e-10,
        (Value::Text(l), Value::Text(r)) => l.to_lowercase() == r.to_lowercase(),
        (Value::Boolean(l), Value::Boolean(r)) => l == r,
        // Boolean vs Number: TRUE = 1, FALSE = 0 (Excel-compatible)
        (Value::Boolean(b), Value::Number(n)) | (Value::Number(n), Value::Boolean(b)) => {
            let bool_num = if *b { 1.0 } else { 0.0 };
            (bool_num - n).abs() < 1e-10
        },
        (Value::Null, Value::Null) => true,
        (Value::Array(l), Value::Array(r)) => {
            // Arrays are equal if same length and all elements equal
            if l.len() != r.len() {
                return false;
            }
            l.iter().zip(r.iter()).all(|(a, b)| values_equal(a, b))
        },
        // Single-element array compared with scalar
        (Value::Array(arr), other) if arr.len() == 1 => values_equal(&arr[0], other),
        (other, Value::Array(arr)) if arr.len() == 1 => values_equal(other, &arr[0]),
        _ => false,
    }
}

/// Evaluate a unary operation
fn evaluate_unary_op(op: &str, operand: &Value) -> Result<Value, EvalError> {
    match op {
        "-" => {
            let n = operand
                .as_number()
                .ok_or_else(|| EvalError::new("Operand must be a number"))?;
            Ok(Value::Number(-n))
        },
        _ => Err(EvalError::new(format!("Unknown unary operator: {op}"))),
    }
}

/// Evaluate a function call - dispatches to category-specific modules
fn evaluate_function(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let upper_name = name.to_uppercase();

    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    if let Some(result) = math::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = aggregation::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = logical::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = text::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = dates::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = lookup::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = financial::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = statistical::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = trig::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE FUNCTIONS (only in full build)
    // ═══════════════════════════════════════════════════════════════════════════
    {
        if let Some(result) = conditional::try_evaluate(&upper_name, args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = array::try_evaluate(&upper_name, args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = info::try_evaluate(&upper_name, args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = forge::try_evaluate(&upper_name, args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = advanced::try_evaluate(&upper_name, args, ctx)? {
            return Ok(result);
        }
    }

    Err(EvalError::new(format!("Unknown function: {name}")))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SHARED HELPER FUNCTIONS (used by submodules)
// ═══════════════════════════════════════════════════════════════════════════════

/// Require exact number of arguments
pub(crate) fn require_args(func: &str, args: &[Expr], count: usize) -> Result<(), EvalError> {
    if args.len() == count {
        Ok(())
    } else {
        Err(EvalError::new(format!(
            "{} requires {} argument(s), got {}",
            func,
            count,
            args.len()
        )))
    }
}

/// Require arguments in range
pub(crate) fn require_args_range(
    func: &str,
    args: &[Expr],
    min: usize,
    max: usize,
) -> Result<(), EvalError> {
    if args.len() < min || args.len() > max {
        Err(EvalError::new(format!(
            "{} requires {}-{} arguments, got {}",
            func,
            min,
            max,
            args.len()
        )))
    } else {
        Ok(())
    }
}

/// Collect all numeric values from arguments (handles arrays)
pub(crate) fn collect_numeric_values(
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Vec<f64>, EvalError> {
    let mut values = Vec::new();

    for arg in args {
        let val = evaluate(arg, ctx)?;
        match val {
            Value::Array(arr) => {
                for v in arr {
                    if let Some(n) = v.as_number() {
                        values.push(n);
                    }
                }
            },
            Value::Number(n) => values.push(n),
            _ => {},
        }
    }

    Ok(values)
}

/// Collect all values from an expression as a Vec<Value>
/// (Enterprise only - used by array functions)
pub(crate) fn collect_values_as_vec(
    expr: &Expr,
    ctx: &EvalContext,
) -> Result<Vec<Value>, EvalError> {
    let val = evaluate(expr, ctx)?;
    match val {
        Value::Array(arr) => Ok(arr),
        other => Ok(vec![other]),
    }
}

/// Check if a value matches a criteria (supports comparisons like ">50", "<=100", "<>0", "=text")
/// (Enterprise only - used by conditional functions)
pub(crate) fn matches_criteria(val: &Value, criteria: &Value) -> bool {
    let criteria_str = criteria.as_text();

    // Handle comparison operators
    if let Some(stripped) = criteria_str.strip_prefix(">=") {
        if let (Some(v), Ok(c)) = (val.as_number(), stripped.trim().parse::<f64>()) {
            return v >= c;
        }
    } else if let Some(stripped) = criteria_str.strip_prefix("<=") {
        if let (Some(v), Ok(c)) = (val.as_number(), stripped.trim().parse::<f64>()) {
            return v <= c;
        }
    } else if let Some(stripped) = criteria_str
        .strip_prefix("<>")
        .or_else(|| criteria_str.strip_prefix("!="))
    {
        let crit_val = stripped.trim();
        if let (Some(v), Ok(c)) = (val.as_number(), crit_val.parse::<f64>()) {
            return (v - c).abs() > f64::EPSILON;
        }
        return val.as_text() != crit_val;
    } else if let Some(stripped) = criteria_str.strip_prefix('>') {
        if let (Some(v), Ok(c)) = (val.as_number(), stripped.trim().parse::<f64>()) {
            return v > c;
        }
    } else if let Some(stripped) = criteria_str.strip_prefix('<') {
        if let (Some(v), Ok(c)) = (val.as_number(), stripped.trim().parse::<f64>()) {
            return v < c;
        }
    } else if let Some(stripped) = criteria_str.strip_prefix('=') {
        let crit_val = stripped.trim();
        if let (Some(v), Ok(c)) = (val.as_number(), crit_val.parse::<f64>()) {
            return (v - c).abs() < f64::EPSILON;
        }
        return val.as_text().eq_ignore_ascii_case(crit_val);
    }

    // Direct comparison (numeric or text)
    if let (Some(v), Some(c)) = (val.as_number(), criteria.as_number()) {
        return (v - c).abs() < f64::EPSILON;
    }

    // Text comparison (case-insensitive)
    val.as_text().eq_ignore_ascii_case(&criteria_str)
}

/// Parse a Value into a `NaiveDate` (supports YYYY-MM-DD strings and Excel serial numbers)
pub(crate) fn parse_date_value(val: &Value) -> Result<chrono::NaiveDate, EvalError> {
    use chrono::NaiveDate;

    match val {
        Value::Text(s) => {
            // Try YYYY-MM-DD format
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| EvalError::new(format!("Invalid date format: '{s}'")))
        },
        Value::Number(n) => {
            // Excel serial number (days since 1899-12-30)
            // Note: Excel incorrectly treats 1900 as a leap year, we handle this
            let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            let days = *n as i64;
            base.checked_add_days(chrono::Days::new(days as u64))
                .ok_or_else(|| EvalError::new(format!("Invalid Excel date serial: {n}")))
        },
        _ => Err(EvalError::new("Expected date string or serial number")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::parser::parse;
    use crate::core::array_calculator::tokenizer::tokenize;

    pub fn eval(formula: &str, ctx: &EvalContext) -> Result<Value, EvalError> {
        let tokens = tokenize(formula).map_err(|e| EvalError::new(e.message))?;
        let ast = parse(tokens).map_err(|e| EvalError::new(e.message))?;
        evaluate(&ast, ctx)
    }

    #[test]
    fn test_eval_number() {
        let ctx = EvalContext::new();
        let result = eval("42", &ctx).unwrap();
        assert_eq!(result, Value::Number(42.0));
    }

    #[test]
    fn test_eval_arithmetic() {
        let ctx = EvalContext::new();
        assert_eq!(eval("2 + 3", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("10 - 4", &ctx).unwrap(), Value::Number(6.0));
        assert_eq!(eval("3 * 4", &ctx).unwrap(), Value::Number(12.0));
        assert_eq!(eval("15 / 3", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("2 ^ 3", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_eval_power_zero_to_zero() {
        let ctx = EvalContext::new();
        // 0^0 = 1 (Excel convention)
        assert_eq!(eval("0 ^ 0", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_eval_precedence() {
        let ctx = EvalContext::new();
        assert_eq!(eval("2 + 3 * 4", &ctx).unwrap(), Value::Number(14.0));
        assert_eq!(eval("(2 + 3) * 4", &ctx).unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_eval_unary_minus() {
        let ctx = EvalContext::new();
        assert_eq!(eval("-5", &ctx).unwrap(), Value::Number(-5.0));
        assert_eq!(eval("10 + -3", &ctx).unwrap(), Value::Number(7.0));
    }

    #[test]
    fn test_eval_double_negative() {
        let ctx = EvalContext::new();
        // --5 = 5
        assert_eq!(eval("--5", &ctx).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_eval_triple_negative() {
        let ctx = EvalContext::new();
        // ---5 = -5
        assert_eq!(eval("---5", &ctx).unwrap(), Value::Number(-5.0));
    }

    #[test]
    fn test_eval_comparison() {
        let ctx = EvalContext::new();
        assert_eq!(eval("5 > 3", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 < 3", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("5 = 5", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 <> 3", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_boolean_comparison() {
        let ctx = EvalContext::new();
        // TRUE > FALSE = true (TRUE is 1, FALSE is 0)
        assert_eq!(eval("TRUE > FALSE", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_boolean_number_equality() {
        let ctx = EvalContext::new();
        // TRUE = 1 = true (Excel-compatible type coercion)
        assert_eq!(eval("TRUE = 1", &ctx).unwrap(), Value::Boolean(true));
        // FALSE = 0 = true
        assert_eq!(eval("FALSE = 0", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_string_equality() {
        let ctx = EvalContext::new();
        // "ABC" = "ABC" = true (exact string match)
        assert_eq!(
            eval("\"ABC\" = \"ABC\"", &ctx).unwrap(),
            Value::Boolean(true)
        );
        // Note: Forge uses case-insensitive string comparison (differs from Excel)
        // In Excel: "ABC" = "abc" would be false (case sensitive)
        // In Forge: "ABC" = "abc" is true (case insensitive)
        assert_eq!(
            eval("\"ABC\" = \"abc\"", &ctx).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_eval_floating_point_precision() {
        let ctx = EvalContext::new();
        // Note: Forge uses epsilon comparison (1e-10) for floating point equality
        // 0.1 + 0.2 = 0.3 = true (difference is ~5.5e-17, less than epsilon)
        // In strict floating point: this would be false
        // Forge's epsilon comparison makes this true to avoid precision issues
        assert_eq!(eval("0.1 + 0.2 = 0.3", &ctx).unwrap(), Value::Boolean(true));
        // ROUND also ensures equality
        assert_eq!(
            eval("ROUND(0.1 + 0.2, 1) = 0.3", &ctx).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_eval_boolean_addition() {
        let ctx = EvalContext::new();
        // TRUE + 1 = 2 (TRUE coerces to 1)
        assert_eq!(eval("TRUE + 1", &ctx).unwrap(), Value::Number(2.0));
        // FALSE + 1 = 1 (FALSE coerces to 0)
        assert_eq!(eval("FALSE + 1", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_eval_boolean_multiplication() {
        let ctx = EvalContext::new();
        // TRUE * 5 = 5 (TRUE coerces to 1)
        assert_eq!(eval("TRUE * 5", &ctx).unwrap(), Value::Number(5.0));
        // FALSE * 5 = 0 (FALSE coerces to 0)
        assert_eq!(eval("FALSE * 5", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_eval_scalar_reference() {
        let mut ctx = EvalContext::new();
        ctx.scalars
            .insert("price".to_string(), Value::Number(100.0));
        ctx.scalars.insert("tax".to_string(), Value::Number(0.08));

        assert_eq!(eval("price", &ctx).unwrap(), Value::Number(100.0));
        assert_eq!(
            eval("price * (1 + tax)", &ctx).unwrap(),
            Value::Number(108.0)
        );
    }

    #[test]
    fn test_eval_array_index() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "col".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("t.col[0]", &ctx).unwrap(), Value::Number(10.0));
        assert_eq!(eval("t.col[2]", &ctx).unwrap(), Value::Number(30.0));
    }

    #[test]
    fn test_eval_row_wise() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "quantity".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        table.insert(
            "price".to_string(),
            vec![Value::Number(5.0), Value::Number(6.0), Value::Number(7.0)],
        );
        ctx.tables.insert("orders".to_string(), table);

        // In row-wise mode, table.column returns single value
        let row_ctx = ctx.clone().with_row(0, 3);
        assert_eq!(
            eval("orders.quantity * orders.price", &row_ctx).unwrap(),
            Value::Number(50.0)
        );

        let row_ctx = ctx.clone().with_row(1, 3);
        assert_eq!(
            eval("orders.quantity * orders.price", &row_ctx).unwrap(),
            Value::Number(120.0)
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VALUE ENUM EDGE CASES (ADR-006 coverage)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_value_array_equality() {
        let arr1 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let arr2 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let arr3 = Value::Array(vec![Value::Number(1.0)]);
        assert_eq!(arr1, arr2);
        assert_ne!(arr1, arr3);
    }

    #[test]
    fn test_value_null_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_ne!(Value::Null, Value::Number(0.0));
    }

    #[test]
    fn test_value_lambda_never_equal() {
        use crate::core::array_calculator::parser::Expr;
        let lambda1 = Value::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(Expr::Number(1.0)),
        };
        let lambda2 = Value::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(Expr::Number(1.0)),
        };
        // Lambdas never compare equal
        assert_ne!(lambda1, lambda2);
    }

    #[test]
    fn test_value_array_as_number() {
        // Array as_number returns length
        let arr = Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert_eq!(arr.as_number(), Some(3.0));
    }

    #[test]
    fn test_value_lambda_as_number() {
        use crate::core::array_calculator::parser::Expr;
        let lambda = Value::Lambda {
            params: vec![],
            body: Box::new(Expr::Number(42.0)),
        };
        assert_eq!(lambda.as_number(), None);
    }

    #[test]
    fn test_value_null_as_number() {
        assert_eq!(Value::Null.as_number(), None);
    }

    #[test]
    fn test_value_date_string_as_number() {
        // Date string converts to Excel serial number
        let date = Value::Text("2024-01-01".to_string());
        // Excel serial: days since 1899-12-30
        // 2024-01-01 is 45292 days from 1899-12-30
        assert_eq!(date.as_number(), Some(45292.0));

        // Invalid date string returns None
        let invalid = Value::Text("not-a-date".to_string());
        assert_eq!(invalid.as_number(), None);

        // Numeric string still works
        let num = Value::Text("42.5".to_string());
        assert_eq!(num.as_number(), Some(42.5));
    }

    #[test]
    fn test_value_mixed_type_inequality() {
        // Different types should not be equal
        assert_ne!(Value::Number(1.0), Value::Text("1".to_string()));
        assert_ne!(Value::Boolean(true), Value::Number(1.0));
        assert_ne!(Value::Array(vec![]), Value::Null);
    }
}
