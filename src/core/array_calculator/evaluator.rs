//! Formula evaluator for the array calculator
//!
//! Evaluates an AST to produce a result value. Supports both scalar and
//! array (row-wise) evaluation modes.

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
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Lambda { .. }, Value::Lambda { .. }) => false, // Lambdas don't compare
            _ => false,
        }
    }
}

impl Value {
    /// Try to convert to f64
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Text(s) => s.parse().ok(),
            Value::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            // Arrays in scalar context return their length
            Value::Array(arr) => Some(arr.len() as f64),
            Value::Lambda { .. } => None,
            Value::Null => None,
        }
    }

    /// Try to convert to string
    pub fn as_text(&self) -> String {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Null => String::new(),
            Value::Array(arr) => {
                let strs: Vec<String> = arr.iter().map(|v| v.as_text()).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Lambda { params, .. } => {
                format!("LAMBDA({})", params.join(", "))
            }
        }
    }

    /// Try to convert to boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            Value::Number(n) => Some(*n != 0.0),
            Value::Text(s) => {
                let upper = s.to_uppercase();
                if upper == "TRUE" || upper == "1" {
                    Some(true)
                } else if upper == "FALSE" || upper == "0" {
                    Some(false)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        self.as_bool().unwrap_or(false)
    }
}

/// Evaluation context containing variables and tables
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Scalar variables (name -> value)
    pub scalars: HashMap<String, Value>,
    /// Table data (table_name -> column_name -> values)
    pub tables: HashMap<String, HashMap<String, Vec<Value>>>,
    /// Scenarios (scenario_name -> variable_name -> value)
    pub scenarios: HashMap<String, HashMap<String, f64>>,
    /// Current row index for row-wise evaluation (None for scalar mode)
    pub current_row: Option<usize>,
    /// Number of rows in current table context
    pub row_count: Option<usize>,
}

impl EvalContext {
    /// Create a new empty context
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
    pub fn get_scalar(&self, name: &str) -> Option<&Value> {
        self.scalars.get(name)
    }

    /// Get a table column
    pub fn get_column(&self, table: &str, column: &str) -> Option<&Vec<Value>> {
        self.tables.get(table).and_then(|t| t.get(column))
    }

    /// Set to row-wise mode with given row index
    pub fn with_row(mut self, row: usize, count: usize) -> Self {
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
#[derive(Debug, Clone, PartialEq)]
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
pub fn evaluate(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::Number(n) => Ok(Value::Number(*n)),

        Expr::Text(s) => Ok(Value::Text(s.clone())),

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
                    .ok_or_else(|| EvalError::new(format!("Array index {} out of bounds", idx))),
                _ => Err(EvalError::new("Cannot index non-array value")),
            }
        }

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
                }
                _ => Err(EvalError::new("Cannot call non-lambda value")),
            }
        }

        Expr::BinaryOp { op, left, right } => {
            let left_val = evaluate(left, ctx)?;
            let right_val = evaluate(right, ctx)?;
            evaluate_binary_op(op, &left_val, &right_val)
        }

        Expr::UnaryOp { op, operand } => {
            let val = evaluate(operand, ctx)?;
            evaluate_unary_op(op, &val)
        }

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
        }
    }
}

/// Evaluate a reference (scalar or table.column)
fn evaluate_reference(reference: &Reference, ctx: &EvalContext) -> Result<Value, EvalError> {
    match reference {
        Reference::Scalar(name) => {
            let value = ctx
                .get_scalar(name)
                .cloned()
                .ok_or_else(|| EvalError::new(format!("Unknown variable: {}", name)))?;

            // In row-wise mode, if the value is an array, extract current row
            if let Some(row) = ctx.current_row {
                if let Value::Array(arr) = &value {
                    return arr
                        .get(row)
                        .cloned()
                        .ok_or_else(|| EvalError::new(format!("Row {} out of bounds", row)));
                }
            }
            Ok(value)
        }

        Reference::TableColumn { table, column } => {
            // First try as a section.scalar reference (e.g., thresholds.min_value)
            let scalar_key = format!("{}.{}", table, column);
            if let Some(value) = ctx.scalars.get(&scalar_key) {
                return Ok(value.clone());
            }

            // Fall back to table.column lookup
            let col = ctx
                .get_column(table, column)
                .ok_or_else(|| EvalError::new(format!("Unknown column: {}.{}", table, column)))?;

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
                    .ok_or_else(|| EvalError::new(format!("Row {} out of bounds", row)))
            } else {
                Ok(Value::Array(col.clone()))
            }
        }
    }
}

/// Evaluate a binary operation
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
        }
        "-" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l - r))
        }
        "*" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l * r))
        }
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
        }
        "^" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Number(l.powf(r)))
        }

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
        }
        ">" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l > r))
        }
        "<=" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l <= r))
        }
        ">=" => {
            let l = left
                .as_number()
                .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
            let r = right
                .as_number()
                .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
            Ok(Value::Boolean(l >= r))
        }

        _ => Err(EvalError::new(format!("Unknown operator: {}", op))),
    }
}

/// Check if two values are equal
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => (l - r).abs() < 1e-10,
        (Value::Text(l), Value::Text(r)) => l.to_lowercase() == r.to_lowercase(),
        (Value::Boolean(l), Value::Boolean(r)) => l == r,
        (Value::Null, Value::Null) => true,
        (Value::Array(l), Value::Array(r)) => {
            // Arrays are equal if same length and all elements equal
            if l.len() != r.len() {
                return false;
            }
            l.iter().zip(r.iter()).all(|(a, b)| values_equal(a, b))
        }
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
        }
        _ => Err(EvalError::new(format!("Unknown unary operator: {}", op))),
    }
}

/// Evaluate a function call
fn evaluate_function(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let upper_name = name.to_uppercase();

    match upper_name.as_str() {
        // ═══════════════════════════════════════════════════════════════════════
        // MATH FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "ABS" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ABS requires a number"))?;
            Ok(Value::Number(val.abs()))
        }

        "SQRT" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SQRT requires a number"))?;
            if val < 0.0 {
                Err(EvalError::new("SQRT of negative number"))
            } else {
                Ok(Value::Number(val.sqrt()))
            }
        }

        "ROUND" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUND requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            Ok(Value::Number((val * multiplier).round() / multiplier))
        }

        "ROUNDUP" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUNDUP requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            let sign = if val >= 0.0 { 1.0 } else { -1.0 };
            Ok(Value::Number(
                sign * (val.abs() * multiplier).ceil() / multiplier,
            ))
        }

        "ROUNDDOWN" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUNDDOWN requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            let sign = if val >= 0.0 { 1.0 } else { -1.0 };
            Ok(Value::Number(
                sign * (val.abs() * multiplier).floor() / multiplier,
            ))
        }

        "FLOOR" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("FLOOR requires a number"))?;
            let significance = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            if significance == 0.0 {
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number((val / significance).floor() * significance))
            }
        }

        "CEILING" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("CEILING requires a number"))?;
            let significance = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            if significance == 0.0 {
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number((val / significance).ceil() * significance))
            }
        }

        "MOD" => {
            require_args(&upper_name, args, 2)?;
            let num = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
            let divisor = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
            if divisor == 0.0 {
                Err(EvalError::new("MOD division by zero"))
            } else {
                Ok(Value::Number(num % divisor))
            }
        }

        "POWER" => {
            require_args(&upper_name, args, 2)?;
            let base = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            let exp = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            Ok(Value::Number(base.powf(exp)))
        }

        "EXP" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EXP requires a number"))?;
            Ok(Value::Number(val.exp()))
        }

        "LN" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LN requires a number"))?;
            if val <= 0.0 {
                Err(EvalError::new("LN of non-positive number"))
            } else {
                Ok(Value::Number(val.ln()))
            }
        }

        "LOG" | "LOG10" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LOG requires a number"))?;
            if val <= 0.0 {
                Err(EvalError::new("LOG of non-positive number"))
            } else {
                Ok(Value::Number(val.log10()))
            }
        }

        "INT" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("INT requires a number"))?;
            Ok(Value::Number(val.floor()))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // AGGREGATION FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "SUM" => {
            let values = collect_numeric_values(args, ctx)?;
            Ok(Value::Number(values.iter().sum()))
        }

        "PRODUCT" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number(values.iter().product()))
            }
        }

        "AVERAGE" | "AVG" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                Err(EvalError::new("AVERAGE of empty set"))
            } else {
                Ok(Value::Number(
                    values.iter().sum::<f64>() / values.len() as f64,
                ))
            }
        }

        "MIN" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MIN of empty set"))
        }

        "MAX" => {
            let values = collect_numeric_values(args, ctx)?;
            values
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .map(Value::Number)
                .ok_or_else(|| EvalError::new("MAX of empty set"))
        }

        "COUNT" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| v.as_number().is_some()).count();
                    }
                    Value::Number(_) => count += 1,
                    _ => {}
                }
            }
            Ok(Value::Number(count as f64))
        }

        "COUNTA" => {
            let mut count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        count += arr.iter().filter(|v| !matches!(v, Value::Null)).count();
                    }
                    Value::Null => {}
                    _ => count += 1,
                }
            }
            Ok(Value::Number(count as f64))
        }

        "MEDIAN" => {
            let mut values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("MEDIAN of empty set"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = values.len() / 2;
            if values.len() % 2 == 0 {
                Ok(Value::Number((values[mid - 1] + values[mid]) / 2.0))
            } else {
                Ok(Value::Number(values[mid]))
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // STATISTICAL FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "VAR" | "VAR.S" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.len() < 2 {
                return Err(EvalError::new("VAR requires at least 2 values"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            Ok(Value::Number(sum_sq / (values.len() - 1) as f64))
        }

        "VAR.P" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("VAR.P requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            Ok(Value::Number(sum_sq / values.len() as f64))
        }

        "STDEV" | "STDEV.S" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.len() < 2 {
                return Err(EvalError::new("STDEV requires at least 2 values"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / (values.len() - 1) as f64;
            Ok(Value::Number(variance.sqrt()))
        }

        "STDEV.P" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("STDEV.P requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / values.len() as f64;
            Ok(Value::Number(variance.sqrt()))
        }

        "PERCENTILE" => {
            require_args(&upper_name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("PERCENTILE of empty set"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PERCENTILE k must be a number"))?;
            if !(0.0..=1.0).contains(&k) {
                return Err(EvalError::new("PERCENTILE k must be between 0 and 1"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = values.len();
            if n == 1 {
                return Ok(Value::Number(values[0]));
            }
            let pos = k * (n - 1) as f64;
            let lower = pos.floor() as usize;
            let upper = pos.ceil() as usize;
            let frac = pos - lower as f64;
            if lower == upper {
                Ok(Value::Number(values[lower]))
            } else {
                Ok(Value::Number(
                    values[lower] * (1.0 - frac) + values[upper] * frac,
                ))
            }
        }

        "QUARTILE" => {
            require_args(&upper_name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("QUARTILE of empty set"));
            }
            let quart = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("QUARTILE quart must be a number"))?
                as i32;
            if !(0..=4).contains(&quart) {
                return Err(EvalError::new("QUARTILE quart must be 0, 1, 2, 3, or 4"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let k = quart as f64 / 4.0;
            let n = values.len();
            if n == 1 {
                return Ok(Value::Number(values[0]));
            }
            let pos = k * (n - 1) as f64;
            let lower = pos.floor() as usize;
            let upper = pos.ceil() as usize;
            let frac = pos - lower as f64;
            if lower == upper {
                Ok(Value::Number(values[lower]))
            } else {
                Ok(Value::Number(
                    values[lower] * (1.0 - frac) + values[upper] * frac,
                ))
            }
        }

        "CORREL" => {
            require_args(&upper_name, args, 2)?;
            let x_vals = collect_numeric_values(&args[..1], ctx)?;
            let y_vals = collect_numeric_values(&args[1..2], ctx)?;
            if x_vals.len() != y_vals.len() || x_vals.len() < 2 {
                return Err(EvalError::new(
                    "CORREL requires two arrays of equal length >= 2",
                ));
            }
            let n = x_vals.len() as f64;
            let x_mean = x_vals.iter().sum::<f64>() / n;
            let y_mean = y_vals.iter().sum::<f64>() / n;
            let mut cov = 0.0;
            let mut var_x = 0.0;
            let mut var_y = 0.0;
            for (x, y) in x_vals.iter().zip(y_vals.iter()) {
                let dx = x - x_mean;
                let dy = y - y_mean;
                cov += dx * dy;
                var_x += dx * dx;
                var_y += dy * dy;
            }
            if var_x == 0.0 || var_y == 0.0 {
                return Err(EvalError::new("CORREL: zero variance"));
            }
            Ok(Value::Number(cov / (var_x.sqrt() * var_y.sqrt())))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // CONDITIONAL AGGREGATION FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "SUMIF" => {
            require_args_range(&upper_name, args, 2, 3)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let sum_range_vals = if args.len() > 2 {
                collect_values_as_vec(&args[2], ctx)?
            } else {
                range_vals.clone()
            };
            let mut total = 0.0;
            for (i, val) in range_vals.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(sum_val) = sum_range_vals.get(i) {
                        if let Some(n) = sum_val.as_number() {
                            total += n;
                        }
                    }
                }
            }
            Ok(Value::Number(total))
        }

        "COUNTIF" => {
            require_args(&upper_name, args, 2)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let count = range_vals
                .iter()
                .filter(|v| matches_criteria(v, &criteria))
                .count();
            Ok(Value::Number(count as f64))
        }

        "AVERAGEIF" => {
            require_args_range(&upper_name, args, 2, 3)?;
            let range_vals = collect_values_as_vec(&args[0], ctx)?;
            let criteria = evaluate(&args[1], ctx)?;
            let avg_range_vals = if args.len() > 2 {
                collect_values_as_vec(&args[2], ctx)?
            } else {
                range_vals.clone()
            };
            let mut total = 0.0;
            let mut count = 0;
            for (i, val) in range_vals.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(avg_val) = avg_range_vals.get(i) {
                        if let Some(n) = avg_val.as_number() {
                            total += n;
                            count += 1;
                        }
                    }
                }
            }
            if count == 0 {
                Err(EvalError::new("AVERAGEIF: no matching values"))
            } else {
                Ok(Value::Number(total / count as f64))
            }
        }

        "SUMIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "SUMIFS requires sum_range, criteria_range1, criteria1, ...",
                ));
            }
            let sum_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; sum_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let total: f64 = sum_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .sum();
            Ok(Value::Number(total))
        }

        "COUNTIFS" => {
            if args.len() < 2 || !args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "COUNTIFS requires criteria_range1, criteria1, ...",
                ));
            }
            let first_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; first_range.len()];
            for pair in args.chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let count = matches.iter().filter(|&&m| m).count();
            Ok(Value::Number(count as f64))
        }

        "AVERAGEIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "AVERAGEIFS requires avg_range, criteria_range1, criteria1, ...",
                ));
            }
            let avg_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; avg_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = avg_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                Err(EvalError::new("AVERAGEIFS: no matching values"))
            } else {
                Ok(Value::Number(
                    matching.iter().sum::<f64>() / matching.len() as f64,
                ))
            }
        }

        "MAXIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "MAXIFS requires max_range, criteria_range1, criteria1, ...",
                ));
            }
            let max_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; max_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = max_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number(
                    matching.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                ))
            }
        }

        "MINIFS" => {
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "MINIFS requires min_range, criteria_range1, criteria1, ...",
                ));
            }
            let min_range = collect_values_as_vec(&args[0], ctx)?;
            let mut matches = vec![true; min_range.len()];
            for pair in args[1..].chunks(2) {
                let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
                let criteria = evaluate(&pair[1], ctx)?;
                for (i, val) in criteria_range.iter().enumerate() {
                    if i < matches.len() && !matches_criteria(val, &criteria) {
                        matches[i] = false;
                    }
                }
            }
            let matching: Vec<f64> = min_range
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < matches.len() && matches[*i])
                .filter_map(|(_, v)| v.as_number())
                .collect();
            if matching.is_empty() {
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number(
                    matching.iter().cloned().fold(f64::INFINITY, f64::min),
                ))
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // ARRAY FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "UNIQUE" => {
            require_args(&upper_name, args, 1)?;
            let values = collect_values_as_vec(&args[0], ctx)?;
            let mut seen = Vec::new();
            for v in values {
                let text = v.as_text();
                if !seen.iter().any(|s: &Value| s.as_text() == text) {
                    seen.push(v);
                }
            }
            Ok(Value::Array(seen))
        }

        "COUNTUNIQUE" => {
            require_args(&upper_name, args, 1)?;
            let values = collect_values_as_vec(&args[0], ctx)?;
            let mut seen = std::collections::HashSet::new();
            for v in values {
                seen.insert(v.as_text());
            }
            Ok(Value::Number(seen.len() as f64))
        }

        "SORT" => {
            require_args_range(&upper_name, args, 1, 2)?;
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
            Ok(Value::Array(
                values.into_iter().map(Value::Number).collect(),
            ))
        }

        "FILTER" => {
            require_args(&upper_name, args, 2)?;
            let data = collect_values_as_vec(&args[0], ctx)?;
            let criteria = collect_values_as_vec(&args[1], ctx)?;
            let filtered: Vec<Value> = data
                .into_iter()
                .zip(criteria.iter())
                .filter(|(_, c)| c.is_truthy())
                .map(|(v, _)| v)
                .collect();
            Ok(Value::Array(filtered))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // LOGICAL FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "IF" => {
            require_args_range(&upper_name, args, 2, 3)?;
            let condition = evaluate(&args[0], ctx)?;
            if condition.is_truthy() {
                evaluate(&args[1], ctx)
            } else if args.len() > 2 {
                evaluate(&args[2], ctx)
            } else {
                Ok(Value::Boolean(false))
            }
        }

        "AND" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if !val.is_truthy() {
                    return Ok(Value::Boolean(false));
                }
            }
            Ok(Value::Boolean(true))
        }

        "OR" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    return Ok(Value::Boolean(true));
                }
            }
            Ok(Value::Boolean(false))
        }

        "NOT" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Ok(Value::Boolean(!val.is_truthy()))
        }

        "IFERROR" => {
            require_args(&upper_name, args, 2)?;
            match evaluate(&args[0], ctx) {
                Ok(val) => Ok(val),
                Err(_) => evaluate(&args[1], ctx),
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // TEXT FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "CONCAT" | "CONCATENATE" => {
            let mut result = String::new();
            for arg in args {
                let val = evaluate(arg, ctx)?;
                result.push_str(&val.as_text());
            }
            Ok(Value::Text(result))
        }

        "UPPER" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Ok(Value::Text(val.as_text().to_uppercase()))
        }

        "LOWER" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Ok(Value::Text(val.as_text().to_lowercase()))
        }

        "TRIM" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Ok(Value::Text(val.as_text().trim().to_string()))
        }

        "LEN" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Ok(Value::Number(val.as_text().len() as f64))
        }

        "LEFT" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().take(n).collect();
            Ok(Value::Text(chars.into_iter().collect()))
        }

        "RIGHT" => {
            require_args_range(&upper_name, args, 1, 2)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let n = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize
            } else {
                1
            };
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(n);
            Ok(Value::Text(chars[start..].iter().collect()))
        }

        "MID" => {
            require_args(&upper_name, args, 3)?;
            let text = evaluate(&args[0], ctx)?.as_text();
            let start = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as usize;
            let length = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as usize;

            let chars: Vec<char> = text.chars().collect();
            // Excel MID is 1-indexed
            let start_idx = start.saturating_sub(1);
            let end_idx = (start_idx + length).min(chars.len());
            Ok(Value::Text(chars[start_idx..end_idx].iter().collect()))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // DATE FUNCTIONS (chrono-powered, supports YYYY-MM-DD and Excel serials)
        // ═══════════════════════════════════════════════════════════════════════
        "TODAY" => {
            use chrono::Local;
            let today = Local::now().date_naive();
            Ok(Value::Text(today.format("%Y-%m-%d").to_string()))
        }

        "YEAR" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Ok(Value::Number(date.year() as f64))
        }

        "MONTH" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Ok(Value::Number(date.month() as f64))
        }

        "DAY" => {
            require_args(&upper_name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Ok(Value::Number(date.day() as f64))
        }

        "DATE" => {
            require_args(&upper_name, args, 3)?;
            let year = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: year must be a number"))?
                as i32;
            let month = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: month must be a number"))?
                as i32;
            let day = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: day must be a number"))?
                as u32;

            // Handle month overflow/underflow (Excel-compatible behavior)
            // Month 0 = December of previous year, Month 13 = January of next year
            let total_months = (year * 12 + month - 1) as i32;
            let adj_year = total_months.div_euclid(12);
            let adj_month = (total_months.rem_euclid(12) + 1) as u32;

            use chrono::NaiveDate;
            let date = NaiveDate::from_ymd_opt(adj_year, adj_month, day).ok_or_else(|| {
                EvalError::new(format!(
                    "DATE: invalid date {}-{}-{}",
                    adj_year, adj_month, day
                ))
            })?;
            Ok(Value::Text(date.format("%Y-%m-%d").to_string()))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // LOOKUP FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "INDEX" => {
            require_args_range(&upper_name, args, 2, 3)?;

            // Evaluate array without row context to get full array
            let array_ctx = EvalContext {
                scalars: ctx.scalars.clone(),
                tables: ctx.tables.clone(),
                scenarios: ctx.scenarios.clone(),
                current_row: None,
                row_count: ctx.row_count,
            };
            let array = evaluate(&args[0], &array_ctx)?;
            let row_num = evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as i64;

            // INDEX is 1-based, row_num must be >= 1
            if row_num < 1 {
                return Err(EvalError::new(format!(
                    "INDEX: row_num {} must be >= 1",
                    row_num
                )));
            }

            match array {
                Value::Array(arr) => {
                    let idx = (row_num - 1) as usize;
                    arr.get(idx).cloned().ok_or_else(|| {
                        EvalError::new(format!("INDEX row {} out of bounds", row_num))
                    })
                }
                _ => Err(EvalError::new("INDEX requires an array")),
            }
        }

        "MATCH" => {
            require_args_range(&upper_name, args, 2, 3)?;

            // lookup_value uses current row context
            let lookup_value = evaluate(&args[0], ctx)?;

            // lookup_array needs full array
            let array_ctx = EvalContext {
                scalars: ctx.scalars.clone(),
                tables: ctx.tables.clone(),
                scenarios: ctx.scenarios.clone(),
                current_row: None,
                row_count: ctx.row_count,
            };
            let lookup_array = evaluate(&args[1], &array_ctx)?;

            let match_type = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(1.0) as i32
            } else {
                1
            };

            let arr = match lookup_array {
                Value::Array(a) => a,
                _ => return Err(EvalError::new("MATCH requires an array")),
            };

            let lookup_num = lookup_value.as_number();

            match match_type {
                0 => {
                    // Exact match
                    for (i, val) in arr.iter().enumerate() {
                        if values_equal(&lookup_value, val) {
                            return Ok(Value::Number((i + 1) as f64)); // 1-based
                        }
                    }
                    Err(EvalError::new("MATCH: value not found"))
                }
                1 => {
                    // Find largest value <= lookup_value (ascending order)
                    // For numeric: find largest number <= lookup
                    // For text: try exact match first, then alphabetical
                    if let Some(ln) = lookup_num {
                        let mut best_idx: Option<usize> = None;
                        let mut best_val: Option<f64> = None;
                        for (i, v) in arr.iter().enumerate() {
                            if let Some(vn) = v.as_number() {
                                if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                                    best_val = Some(vn);
                                    best_idx = Some(i);
                                }
                            }
                        }
                        best_idx
                            .map(|i| Value::Number((i + 1) as f64))
                            .ok_or_else(|| EvalError::new("MATCH: value not found"))
                    } else {
                        // For text, do exact match (case-insensitive)
                        let lookup_text = lookup_value.as_text().to_lowercase();
                        for (i, val) in arr.iter().enumerate() {
                            if val.as_text().to_lowercase() == lookup_text {
                                return Ok(Value::Number((i + 1) as f64));
                            }
                        }
                        Err(EvalError::new("MATCH: value not found"))
                    }
                }
                -1 => {
                    // Find smallest value >= lookup_value (descending order)
                    let mut best_idx: Option<usize> = None;
                    let mut best_val: Option<f64> = None;

                    if let Some(ln) = lookup_num {
                        for (i, v) in arr.iter().enumerate() {
                            if let Some(vn) = v.as_number() {
                                if vn >= ln && (best_val.is_none() || vn < best_val.unwrap()) {
                                    best_val = Some(vn);
                                    best_idx = Some(i);
                                }
                            }
                        }
                    }
                    best_idx
                        .map(|i| Value::Number((i + 1) as f64))
                        .ok_or_else(|| EvalError::new("MATCH: value not found"))
                }
                _ => Err(EvalError::new(format!(
                    "MATCH: invalid match_type {}",
                    match_type
                ))),
            }
        }

        "CHOOSE" => {
            require_args_range(&upper_name, args, 2, 255)?;
            let index = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("CHOOSE index must be a number"))?
                as usize;

            if index < 1 || index >= args.len() {
                Err(EvalError::new(format!(
                    "CHOOSE index {} out of range",
                    index
                )))
            } else {
                evaluate(&args[index], ctx)
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // FINANCIAL FUNCTIONS (simplified versions)
        // ═══════════════════════════════════════════════════════════════════════
        "PMT" => {
            require_args_range(&upper_name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pv = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let pmt_type = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            if rate == 0.0 {
                Ok(Value::Number(-(pv + fv) / nper))
            } else {
                let pmt = if pmt_type == 1 {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                        / (1.0 + rate)
                } else {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                };
                Ok(Value::Number(pmt))
            }
        }

        "FV" => {
            require_args_range(&upper_name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pmt = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let pv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if rate == 0.0 {
                Ok(Value::Number(-pv - pmt * nper))
            } else {
                let fv =
                    -pv * (1.0 + rate).powf(nper) - pmt * ((1.0 + rate).powf(nper) - 1.0) / rate;
                Ok(Value::Number(fv))
            }
        }

        "PV" => {
            require_args_range(&upper_name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pmt = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if rate == 0.0 {
                Ok(Value::Number(-fv - pmt * nper))
            } else {
                let pv =
                    (-fv - pmt * ((1.0 + rate).powf(nper) - 1.0) / rate) / (1.0 + rate).powf(nper);
                Ok(Value::Number(pv))
            }
        }

        "NPV" => {
            require_args_range(&upper_name, args, 2, 255)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPV rate must be a number"))?;

            let mut npv = 0.0;
            let mut period = 1;

            for arg in &args[1..] {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        for v in arr {
                            if let Some(n) = v.as_number() {
                                npv += n / (1.0 + rate).powi(period);
                                period += 1;
                            }
                        }
                    }
                    Value::Number(n) => {
                        npv += n / (1.0 + rate).powi(period);
                        period += 1;
                    }
                    _ => {}
                }
            }

            Ok(Value::Number(npv))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // FORGE-SPECIFIC FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "BREAKEVEN_UNITS" => {
            require_args(&upper_name, args, 3)?;
            let fixed_costs = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;
            let price_per_unit = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;
            let variable_cost_per_unit = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_UNITS requires numbers"))?;

            let contribution_margin = price_per_unit - variable_cost_per_unit;
            if contribution_margin <= 0.0 {
                Err(EvalError::new(
                    "unit_price must be greater than variable_cost",
                ))
            } else {
                Ok(Value::Number(fixed_costs / contribution_margin))
            }
        }

        "BREAKEVEN_REVENUE" => {
            require_args(&upper_name, args, 2)?;
            let fixed_costs = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_REVENUE requires numbers"))?;
            let contribution_margin_ratio = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("BREAKEVEN_REVENUE requires numbers"))?;

            if contribution_margin_ratio <= 0.0 || contribution_margin_ratio > 1.0 {
                Err(EvalError::new(
                    "contribution_margin_pct must be between 0 and 1 (exclusive of 0)",
                ))
            } else {
                Ok(Value::Number(fixed_costs / contribution_margin_ratio))
            }
        }

        "VARIANCE" => {
            require_args(&upper_name, args, 2)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE requires numbers"))?;
            Ok(Value::Number(actual - budget))
        }

        "VARIANCE_PCT" => {
            require_args(&upper_name, args, 2)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_PCT requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_PCT requires numbers"))?;
            if budget == 0.0 {
                return Err(EvalError::new("VARIANCE_PCT: budget cannot be zero"));
            }
            Ok(Value::Number((actual - budget) / budget))
        }

        "VARIANCE_STATUS" => {
            // VARIANCE_STATUS(actual, budget, [threshold_or_type])
            // Third arg: number = threshold (e.g., 0.10 = 10%), string "cost" = cost type
            // Returns: 1 = favorable, -1 = unfavorable, 0 = on budget (within threshold)
            require_args_range(&upper_name, args, 2, 3)?;
            let actual = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_STATUS requires numbers"))?;
            let budget = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("VARIANCE_STATUS requires numbers"))?;

            // Third arg can be threshold (number) or type ("cost")
            let (threshold, is_cost) = if args.len() > 2 {
                let third_val = evaluate(&args[2], ctx)?;
                match &third_val {
                    Value::Text(s) => (0.01, s.to_lowercase() == "cost"),
                    Value::Number(n) => (*n, false),
                    _ => (0.01, false),
                }
            } else {
                (0.01, false) // Default 1% tolerance, revenue type
            };

            if budget == 0.0 {
                return Ok(Value::Number(if actual > 0.0 {
                    1.0
                } else if actual < 0.0 {
                    -1.0
                } else {
                    0.0
                }));
            }

            let variance_pct = (actual - budget) / budget.abs();
            // Returns: 1.0 = favorable, -1.0 = unfavorable, 0.0 = on_budget
            let status = if variance_pct.abs() <= threshold {
                0.0 // on_budget
            } else if is_cost {
                // For costs: under budget = favorable, over budget = unfavorable
                if variance_pct < 0.0 {
                    1.0
                } else {
                    -1.0
                }
            } else {
                // For revenue: over budget = favorable, under budget = unfavorable
                if variance_pct > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            };
            Ok(Value::Number(status))
        }

        "SCENARIO" => {
            require_args(&upper_name, args, 2)?;
            // SCENARIO(scenario_name, variable_name) - lookup from scenarios
            let scenario_name = evaluate(&args[0], ctx)?.as_text();
            let var_name = evaluate(&args[1], ctx)?.as_text();

            if let Some(scenario) = ctx.scenarios.get(&scenario_name) {
                if let Some(&value) = scenario.get(&var_name) {
                    Ok(Value::Number(value))
                } else {
                    Err(EvalError::new(format!(
                        "Variable '{}' not found in scenario '{}'",
                        var_name, scenario_name
                    )))
                }
            } else {
                Err(EvalError::new(format!(
                    "Scenario '{}' not found",
                    scenario_name
                )))
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // FINANCIAL FUNCTIONS (EXTENDED)
        // ═══════════════════════════════════════════════════════════════════════
        "IRR" => {
            // Simplified IRR using Newton's method
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("IRR requires cash flows"));
            }
            let mut rate: f64 = 0.1; // Initial guess
            for _ in 0..100 {
                let mut npv: f64 = 0.0;
                let mut npv_deriv: f64 = 0.0;
                for (i, cf) in values.iter().enumerate() {
                    let factor = (1.0_f64 + rate).powi(i as i32);
                    npv += cf / factor;
                    if i > 0 {
                        npv_deriv -= (i as f64) * cf / (factor * (1.0 + rate));
                    }
                }
                if npv_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - npv / npv_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Ok(Value::Number(rate))
        }

        "XIRR" => {
            require_args(&upper_name, args, 2)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let dates_val = evaluate(&args[1], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(|v| v.as_number()).collect(),
                Value::Number(n) => vec![n],
                _ => return Err(EvalError::new("XIRR requires dates")),
            };
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XIRR: values and dates must have same length",
                ));
            }
            let base_date = dates[0];
            let year_fracs: Vec<f64> = dates.iter().map(|d| (d - base_date) / 365.0).collect();
            let mut rate: f64 = 0.1;
            for _ in 0..100 {
                let mut npv: f64 = 0.0;
                let mut npv_deriv: f64 = 0.0;
                for (i, cf) in values.iter().enumerate() {
                    let t = year_fracs[i];
                    let factor = (1.0_f64 + rate).powf(t);
                    npv += cf / factor;
                    if t != 0.0 {
                        npv_deriv -= t * cf / (factor * (1.0 + rate));
                    }
                }
                if npv_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - npv / npv_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Ok(Value::Number(rate))
        }

        "XNPV" => {
            require_args(&upper_name, args, 3)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("XNPV requires rate"))?;
            let values = collect_numeric_values(&args[1..2], ctx)?;
            let dates_val = evaluate(&args[2], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(|v| v.as_number()).collect(),
                Value::Number(n) => vec![n],
                _ => return Err(EvalError::new("XNPV requires dates")),
            };
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XNPV: values and dates must have same length",
                ));
            }
            let base_date = dates[0];
            let mut npv = 0.0;
            for (i, cf) in values.iter().enumerate() {
                let t = (dates[i] - base_date) / 365.0;
                npv += cf / (1.0 + rate).powf(t);
            }
            Ok(Value::Number(npv))
        }

        "NPER" => {
            require_args_range(&upper_name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires rate"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires present value"))?;
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let _type = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            if rate == 0.0 {
                Ok(Value::Number(-(pv + fv) / pmt))
            } else {
                let n = ((-fv * rate + pmt) / (pv * rate + pmt)).ln() / (1.0 + rate).ln();
                Ok(Value::Number(n))
            }
        }

        "RATE" => {
            require_args_range(&upper_name, args, 3, 6)?;
            let nper = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires nper"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires present value"))?;
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let _type = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let guess = if args.len() > 5 {
                evaluate(&args[5], ctx)?.as_number().unwrap_or(0.1)
            } else {
                0.1
            };

            // Newton-Raphson method
            let mut rate = guess;
            for _ in 0..100 {
                let f = pv * (1.0 + rate).powf(nper)
                    + pmt * ((1.0 + rate).powf(nper) - 1.0) / rate
                    + fv;
                let f_deriv = nper * pv * (1.0 + rate).powf(nper - 1.0)
                    + pmt
                        * (nper * rate * (1.0 + rate).powf(nper - 1.0) - (1.0 + rate).powf(nper)
                            + 1.0)
                        / (rate * rate);
                if f_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - f / f_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Ok(Value::Number(rate))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // DEPRECIATION FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "SLN" => {
            // Straight-line depreciation
            require_args(&upper_name, args, 3)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires life"))?;

            if life == 0.0 {
                return Err(EvalError::new("SLN: life cannot be zero"));
            }
            Ok(Value::Number((cost - salvage) / life))
        }

        "DB" => {
            // Fixed declining balance depreciation
            require_args_range(&upper_name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires period"))?;
            let month = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(12.0)
            } else {
                12.0
            };

            if life == 0.0 || cost == 0.0 {
                return Ok(Value::Number(0.0));
            }

            // Calculate rate: 1 - (salvage/cost)^(1/life), rounded to 3 decimals
            let rate = 1.0 - (salvage / cost).powf(1.0 / life);
            let rate = (rate * 1000.0).round() / 1000.0;

            let mut depreciation = 0.0;
            let mut remaining = cost;

            for p in 1..=(period as i32) {
                if p == 1 {
                    // First period adjusted for months
                    depreciation = cost * rate * month / 12.0;
                } else if p == (life as i32 + 1) {
                    // Last period gets remaining
                    depreciation = remaining * rate * (12.0 - month) / 12.0;
                } else {
                    depreciation = remaining * rate;
                }
                remaining -= depreciation;
            }

            Ok(Value::Number(depreciation))
        }

        "DDB" => {
            // Double declining balance depreciation
            require_args_range(&upper_name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires period"))?;
            let factor = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(2.0)
            } else {
                2.0
            };

            if life == 0.0 {
                return Err(EvalError::new("DDB: life cannot be zero"));
            }

            let rate = factor / life;
            let mut remaining = cost;
            let mut depreciation = 0.0;

            for _p in 1..=(period as i32) {
                depreciation = remaining * rate;
                // Don't depreciate below salvage
                if remaining - depreciation < salvage {
                    depreciation = remaining - salvage;
                }
                if depreciation < 0.0 {
                    depreciation = 0.0;
                }
                remaining -= depreciation;
            }

            Ok(Value::Number(depreciation))
        }

        "MIRR" => {
            // Modified internal rate of return
            require_args(&upper_name, args, 3)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let finance_rate = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MIRR requires finance rate"))?;
            let reinvest_rate = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MIRR requires reinvest rate"))?;

            if values.is_empty() {
                return Err(EvalError::new("MIRR requires cash flows"));
            }

            let n = values.len() as f64;

            // Separate negative and positive cash flows
            let mut npv_neg = 0.0;
            let mut npv_pos = 0.0;

            for (i, &cf) in values.iter().enumerate() {
                if cf < 0.0 {
                    npv_neg += cf / (1.0 + finance_rate).powi(i as i32);
                } else {
                    npv_pos += cf * (1.0 + reinvest_rate).powi((n - 1.0 - i as f64) as i32);
                }
            }

            if npv_neg == 0.0 || npv_pos == 0.0 {
                return Err(EvalError::new(
                    "MIRR requires both positive and negative cash flows",
                ));
            }

            // MIRR formula: (-FV_positive / PV_negative)^(1/n) - 1
            let mirr = (-npv_pos / npv_neg).powf(1.0 / (n - 1.0)) - 1.0;
            Ok(Value::Number(mirr))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // DATE FUNCTIONS (EXTENDED)
        // ═══════════════════════════════════════════════════════════════════════
        "DATEDIF" => {
            require_args(&upper_name, args, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let unit = evaluate(&args[2], ctx)?.as_text().to_uppercase();

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match unit.as_str() {
                "D" => (end_date - start_date).num_days() as f64,
                "M" => {
                    let years = end_date.year() - start_date.year();
                    let months = end_date.month() as i32 - start_date.month() as i32;
                    (years * 12 + months) as f64
                }
                "Y" => (end_date.year() - start_date.year()) as f64,
                "MD" => {
                    let mut day_diff = end_date.day() as i32 - start_date.day() as i32;
                    if day_diff < 0 {
                        day_diff += 30; // Approximate
                    }
                    day_diff as f64
                }
                "YM" => {
                    let mut month_diff = end_date.month() as i32 - start_date.month() as i32;
                    if month_diff < 0 {
                        month_diff += 12;
                    }
                    month_diff as f64
                }
                "YD" => {
                    let start_doy = start_date.ordinal() as i32;
                    let end_doy = end_date.ordinal() as i32;
                    let mut day_diff = end_doy - start_doy;
                    if day_diff < 0 {
                        day_diff += 365;
                    }
                    day_diff as f64
                }
                _ => return Err(EvalError::new(format!("DATEDIF: unknown unit '{}'", unit))),
            };
            Ok(Value::Number(result))
        }

        "NETWORKDAYS" => {
            require_args_range(&upper_name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            // Optional holidays array (ignored for simplicity)

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let mut count = 0;
            let mut current = start_date;
            while current <= end_date {
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
                    count += 1;
                }
                current = current.succ_opt().unwrap_or(current);
            }
            Ok(Value::Number(count as f64))
        }

        "YEARFRAC" => {
            require_args_range(&upper_name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let basis = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match basis {
                0 | 4 => {
                    // US 30/360 and European 30/360
                    let mut d1 = start_date.day() as i32;
                    let m1 = start_date.month() as i32;
                    let y1 = start_date.year() as i32;
                    let mut d2 = end_date.day() as i32;
                    let m2 = end_date.month() as i32;
                    let y2 = end_date.year() as i32;

                    // Adjust day counts per 30/360 convention
                    if d1 == 31 {
                        d1 = 30;
                    }
                    if d2 == 31 && (d1 >= 30 || basis == 4) {
                        d2 = 30;
                    }

                    let days_30_360 = ((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)) as f64;
                    days_30_360 / 360.0
                }
                1 => {
                    // Actual/actual
                    let days = (end_date - start_date).num_days() as f64;
                    let year = start_date.year();
                    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                    let year_days = if is_leap { 366.0 } else { 365.0 };
                    days / year_days
                }
                2 => {
                    // Actual/360
                    let days = (end_date - start_date).num_days() as f64;
                    days / 360.0
                }
                3 => {
                    // Actual/365
                    let days = (end_date - start_date).num_days() as f64;
                    days / 365.0
                }
                _ => return Err(EvalError::new(format!("YEARFRAC: unknown basis {}", basis))),
            };
            Ok(Value::Number(result))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // ADVANCED FUNCTIONS
        // ═══════════════════════════════════════════════════════════════════════
        "INDIRECT" => {
            require_args(&upper_name, args, 1)?;
            // INDIRECT returns the value of the reference - in our context, resolve it
            let ref_str = evaluate(&args[0], ctx)?.as_text();
            // Look up as a scalar
            if let Some(val) = ctx.scalars.get(&ref_str) {
                return Ok(val.clone());
            }
            // Try as table.column
            if ref_str.contains('.') {
                let parts: Vec<&str> = ref_str.splitn(2, '.').collect();
                if parts.len() == 2 {
                    if let Some(table) = ctx.tables.get(parts[0]) {
                        if let Some(col) = table.get(parts[1]) {
                            return Ok(Value::Array(col.clone()));
                        }
                    }
                }
            }
            Err(EvalError::new(format!(
                "INDIRECT: cannot resolve '{}'",
                ref_str
            )))
        }

        "LAMBDA" => {
            // LAMBDA(param1, param2, ..., body) - returns a lambda value
            // When called: LAMBDA(x, x * 2)(5) → 10
            if args.is_empty() {
                return Err(EvalError::new("LAMBDA requires at least a body"));
            }

            // All args except the last are parameter names
            let mut params = Vec::new();
            for i in 0..args.len() - 1 {
                // Parameters should be identifiers (references)
                match &args[i] {
                    Expr::Reference(Reference::Scalar(name)) => {
                        params.push(name.clone());
                    }
                    _ => {
                        return Err(EvalError::new(format!(
                            "LAMBDA parameter {} must be an identifier",
                            i + 1
                        )));
                    }
                }
            }

            // Last arg is the body
            let body = args.last().unwrap().clone();

            Ok(Value::Lambda {
                params,
                body: Box::new(body),
            })
        }
        "EDATE" => {
            use chrono::Months;

            require_args(&upper_name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EDATE requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            // Add/subtract months using chrono
            let result = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EDATE: Invalid date result"))?;

            Ok(Value::Text(result.format("%Y-%m-%d").to_string()))
        }

        "EOMONTH" => {
            use chrono::{Datelike, Months, NaiveDate};

            require_args(&upper_name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EOMONTH requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            // Add/subtract months
            let adjusted = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            // Get last day of that month
            let year = adjusted.year();
            let month = adjusted.month();
            let last_day = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).and_then(|d| d.pred_opt())
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            Ok(Value::Text(last_day.format("%Y-%m-%d").to_string()))
        }

        // ═══════════════════════════════════════════════════════════════════════
        // LOOKUP FUNCTIONS (EXTENDED)
        // ═══════════════════════════════════════════════════════════════════════
        "SWITCH" => {
            // SWITCH(expression, value1, result1, [value2, result2], ..., [default])
            if args.len() < 2 {
                return Err(EvalError::new("SWITCH requires at least 2 arguments"));
            }

            let expr_val = evaluate(&args[0], ctx)?;
            let remaining = &args[1..];

            // Process value/result pairs
            let mut i = 0;
            while i + 1 < remaining.len() {
                let check_val = evaluate(&remaining[i], ctx)?;
                if values_equal(&expr_val, &check_val) {
                    return evaluate(&remaining[i + 1], ctx);
                }
                i += 2;
            }

            // If odd number of remaining args, last is default
            if remaining.len() % 2 == 1 {
                evaluate(&remaining[remaining.len() - 1], ctx)
            } else {
                Err(EvalError::new("SWITCH: No match found"))
            }
        }

        "LET" => {
            // LET(name1, value1, [name2, value2, ...], calculation)
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "LET requires pairs of name/value plus a calculation",
                ));
            }

            // Create a new context with the LET bindings
            let mut new_ctx = ctx.clone();

            // Process name/value pairs
            let num_pairs = (args.len() - 1) / 2;
            for i in 0..num_pairs {
                let name_expr = &args[i * 2];
                let value_expr = &args[i * 2 + 1];

                // Get the variable name (must be an identifier)
                let name = match name_expr {
                    Expr::Reference(Reference::Scalar(n)) => n.clone(),
                    _ => return Err(EvalError::new("LET variable name must be an identifier")),
                };

                // Evaluate the value and store it
                let value = evaluate(value_expr, &new_ctx)?;
                new_ctx.scalars.insert(name, value);
            }

            // Evaluate the calculation expression with new context
            evaluate(&args[args.len() - 1], &new_ctx)
        }

        "XLOOKUP" => {
            // XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
            require_args_range(&upper_name, args, 3, 6)?;

            // lookup_value should use current row context
            let lookup_val = evaluate(&args[0], ctx)?;

            // lookup_array and return_array need full arrays, so evaluate without row context
            let array_ctx = EvalContext {
                scalars: ctx.scalars.clone(),
                tables: ctx.tables.clone(),
                scenarios: ctx.scenarios.clone(),
                current_row: None, // Disable row extraction
                row_count: ctx.row_count,
            };
            let lookup_arr = evaluate(&args[1], &array_ctx)?;
            let return_arr = evaluate(&args[2], &array_ctx)?;

            let if_not_found = if args.len() > 3 {
                Some(evaluate(&args[3], ctx)?)
            } else {
                None
            };
            let match_mode = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0 // Exact match
            };

            let lookup_values = match lookup_arr {
                Value::Array(arr) => arr,
                _ => return Err(EvalError::new("XLOOKUP lookup_array must be an array")),
            };

            let return_values = match return_arr {
                Value::Array(arr) => arr,
                _ => return Err(EvalError::new("XLOOKUP return_array must be an array")),
            };

            // Validate array lengths match
            if lookup_values.len() != return_values.len() {
                return Err(EvalError::new(format!(
                    "XLOOKUP: lookup_array ({}) and return_array ({}) must have same length",
                    lookup_values.len(),
                    return_values.len()
                )));
            }

            // Find match based on match_mode
            let idx = match match_mode {
                0 => {
                    // Exact match
                    lookup_values
                        .iter()
                        .position(|v| values_equal(v, &lookup_val))
                }
                -1 => {
                    // Exact match or next smaller
                    let mut best_idx: Option<usize> = None;
                    let mut best_val: Option<f64> = None;
                    let lookup_num = lookup_val.as_number();

                    for (i, v) in lookup_values.iter().enumerate() {
                        if values_equal(v, &lookup_val) {
                            return Ok(return_values.get(i).cloned().unwrap_or(Value::Null));
                        }
                        if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                            if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
                                best_val = Some(vn);
                                best_idx = Some(i);
                            }
                        }
                    }
                    best_idx
                }
                1 => {
                    // Exact match or next larger
                    let mut best_idx: Option<usize> = None;
                    let mut best_val: Option<f64> = None;
                    let lookup_num = lookup_val.as_number();

                    for (i, v) in lookup_values.iter().enumerate() {
                        if values_equal(v, &lookup_val) {
                            return Ok(return_values.get(i).cloned().unwrap_or(Value::Null));
                        }
                        if let (Some(ln), Some(vn)) = (lookup_num, v.as_number()) {
                            if vn >= ln && (best_val.is_none() || vn < best_val.unwrap()) {
                                best_val = Some(vn);
                                best_idx = Some(i);
                            }
                        }
                    }
                    best_idx
                }
                _ => {
                    return Err(EvalError::new(format!(
                        "XLOOKUP: invalid match_mode {}",
                        match_mode
                    )))
                }
            };

            match idx {
                Some(i) => Ok(return_values.get(i).cloned().unwrap_or(Value::Null)),
                None => {
                    if let Some(not_found) = if_not_found {
                        Ok(not_found)
                    } else {
                        Err(EvalError::new("XLOOKUP: No match found"))
                    }
                }
            }
        }

        // Unknown function
        _ => Err(EvalError::new(format!("Unknown function: {}", name))),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Require exact number of arguments
fn require_args(func: &str, args: &[Expr], count: usize) -> Result<(), EvalError> {
    if args.len() != count {
        Err(EvalError::new(format!(
            "{} requires {} argument(s), got {}",
            func,
            count,
            args.len()
        )))
    } else {
        Ok(())
    }
}

/// Require arguments in range
fn require_args_range(func: &str, args: &[Expr], min: usize, max: usize) -> Result<(), EvalError> {
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
fn collect_numeric_values(args: &[Expr], ctx: &EvalContext) -> Result<Vec<f64>, EvalError> {
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
            }
            Value::Number(n) => values.push(n),
            _ => {}
        }
    }

    Ok(values)
}

/// Parse a Value into a NaiveDate (supports YYYY-MM-DD strings and Excel serial numbers)
fn parse_date_value(val: &Value) -> Result<chrono::NaiveDate, EvalError> {
    use chrono::NaiveDate;

    match val {
        Value::Text(s) => {
            // Try YYYY-MM-DD format
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| EvalError::new(format!("Invalid date format: '{}'", s)))
        }
        Value::Number(n) => {
            // Excel serial number (days since 1899-12-30)
            // Note: Excel incorrectly treats 1900 as a leap year, we handle this
            let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            let days = *n as i64;
            base.checked_add_days(chrono::Days::new(days as u64))
                .ok_or_else(|| EvalError::new(format!("Invalid Excel date serial: {}", n)))
        }
        _ => Err(EvalError::new("Expected date string or serial number")),
    }
}

/// Collect all values from an expression as a Vec<Value>
fn collect_values_as_vec(expr: &Expr, ctx: &EvalContext) -> Result<Vec<Value>, EvalError> {
    let val = evaluate(expr, ctx)?;
    match val {
        Value::Array(arr) => Ok(arr),
        other => Ok(vec![other]),
    }
}

/// Check if a value matches a criteria (supports comparisons like ">50", "<=100", "<>0", "=text")
fn matches_criteria(val: &Value, criteria: &Value) -> bool {
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

// Re-export chrono types used in function implementations
use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::parser::parse;
    use crate::core::array_calculator::tokenizer::tokenize;

    fn eval(formula: &str, ctx: &EvalContext) -> Result<Value, EvalError> {
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
    fn test_eval_comparison() {
        let ctx = EvalContext::new();
        assert_eq!(eval("5 > 3", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 < 3", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("5 = 5", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("5 <> 3", &ctx).unwrap(), Value::Boolean(true));
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
    fn test_eval_math_functions() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ABS(-5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("SQRT(16)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUND(3.567, 2)", &ctx).unwrap(), Value::Number(3.57));
        assert_eq!(eval("FLOOR(3.7)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("CEILING(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("MOD(10, 3)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("POWER(2, 3)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_eval_aggregation_with_scalars() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SUM(1, 2, 3)", &ctx).unwrap(), Value::Number(6.0));
        assert_eq!(eval("AVERAGE(2, 4, 6)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("MIN(5, 2, 8)", &ctx).unwrap(), Value::Number(2.0));
        assert_eq!(eval("MAX(5, 2, 8)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_eval_aggregation_with_array() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert("data".to_string(), table);

        assert_eq!(eval("SUM(data.values)", &ctx).unwrap(), Value::Number(60.0));
        assert_eq!(
            eval("AVERAGE(data.values)", &ctx).unwrap(),
            Value::Number(20.0)
        );
    }

    #[test]
    fn test_eval_if() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("IF(5 > 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("yes".to_string())
        );
        assert_eq!(
            eval("IF(5 < 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("no".to_string())
        );
    }

    #[test]
    fn test_eval_logical() {
        let ctx = EvalContext::new();
        assert_eq!(eval("AND(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("AND(1, 0, 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("OR(0, 0, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_eval_text_functions() {
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
    fn test_eval_index_function() {
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

        // INDEX is 1-based
        assert_eq!(eval("INDEX(t.col, 1)", &ctx).unwrap(), Value::Number(10.0));
        assert_eq!(eval("INDEX(t.col, 2)", &ctx).unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_eval_choose() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("CHOOSE(1, \"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("a".to_string())
        );
        assert_eq!(
            eval("CHOOSE(2, \"a\", \"b\", \"c\")", &ctx).unwrap(),
            Value::Text("b".to_string())
        );
    }

    #[test]
    fn test_eval_nested_functions() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("ROUND(SQRT(2), 2)", &ctx).unwrap(),
            Value::Number(1.41)
        );
        assert_eq!(
            eval("ABS(MIN(-5, -10, -3))", &ctx).unwrap(),
            Value::Number(10.0)
        );
    }

    #[test]
    fn test_eval_financial() {
        let ctx = EvalContext::new();
        // PMT for $100,000 loan at 5% for 30 years
        let pmt = eval("PMT(0.05/12, 360, 100000)", &ctx).unwrap();
        assert!(matches!(pmt, Value::Number(n) if (n + 536.82).abs() < 0.01));
    }

    #[test]
    fn test_eval_breakeven() {
        let ctx = EvalContext::new();
        // Fixed costs: 10000, price: 50, variable cost: 30
        // Breakeven units = 10000 / (50 - 30) = 500
        assert_eq!(
            eval("BREAKEVEN_UNITS(10000, 50, 30)", &ctx).unwrap(),
            Value::Number(500.0)
        );

        // Fixed costs: 10000, contribution margin ratio: 0.4
        // Breakeven revenue = 10000 / 0.4 = 25000
        assert_eq!(
            eval("BREAKEVEN_REVENUE(10000, 0.4)", &ctx).unwrap(),
            Value::Number(25000.0)
        );
    }

    #[test]
    fn test_eval_iferror() {
        let ctx = EvalContext::new();
        // Division by zero returns the fallback value
        assert_eq!(eval("IFERROR(1/0, 0)", &ctx).unwrap(), Value::Number(0.0));
        // No error returns the original value
        assert_eq!(eval("IFERROR(10/2, 0)", &ctx).unwrap(), Value::Number(5.0));
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

    #[test]
    fn test_eval_median() {
        let ctx = EvalContext::new();
        assert_eq!(eval("MEDIAN(1, 3, 5)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(
            eval("MEDIAN(1, 2, 3, 4)", &ctx).unwrap(),
            Value::Number(2.5)
        );
    }

    #[test]
    fn test_eval_count() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "values".to_string(),
            vec![
                Value::Number(1.0),
                Value::Text("text".to_string()),
                Value::Number(3.0),
                Value::Null,
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(eval("COUNT(t.values)", &ctx).unwrap(), Value::Number(2.0));
        assert_eq!(eval("COUNTA(t.values)", &ctx).unwrap(), Value::Number(3.0));
    }
}
