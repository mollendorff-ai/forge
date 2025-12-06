//! Formula evaluator for the array calculator
//!
//! Evaluates an AST to produce a result value. Supports both scalar and
//! array (row-wise) evaluation modes.

use super::parser::{Expr, Reference};
use std::collections::HashMap;

/// Value type that can be returned from evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A numeric value
    Number(f64),
    /// A text value
    Text(String),
    /// A boolean value
    Boolean(bool),
    /// An array of values (for table columns)
    Array(Vec<Value>),
    /// Null/empty value
    Null,
}

impl Value {
    /// Try to convert to f64
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Text(s) => s.parse().ok(),
            Value::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
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

            // In row-wise mode, return single value; otherwise return full array
            // Note: XLOOKUP etc. handle arrays directly from their column args
            if let Some(row) = ctx.current_row {
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
                    let mut best_idx: Option<usize> = None;
                    let mut best_val: Option<f64> = None;

                    if let Some(ln) = lookup_num {
                        for (i, v) in arr.iter().enumerate() {
                            if let Some(vn) = v.as_number() {
                                if vn <= ln && (best_val.is_none() || vn > best_val.unwrap()) {
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
                    "BREAKEVEN_UNITS: contribution margin must be positive",
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

            if contribution_margin_ratio <= 0.0 {
                Err(EvalError::new(
                    "BREAKEVEN_REVENUE: contribution margin ratio must be positive",
                ))
            } else {
                Ok(Value::Number(fixed_costs / contribution_margin_ratio))
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // DATE FUNCTIONS (EXTENDED)
        // ═══════════════════════════════════════════════════════════════════════
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
