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

mod aggregation;
mod criterion;
mod dates;
pub(crate) mod day_count;
mod financial;
mod logical;
mod lookup;
mod math;
pub(crate) use criterion::Criterion;
mod normal;
mod regression;
mod schedule;
mod statistical;
pub(crate) mod text;
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
    Array(Vec<Self>),
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
                // Then a date (YYYY-MM-DD), as its day number
                date_serial(s)
            },
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            // A one-element array is its element. A longer array has no single
            // numeric value; callers must aggregate it or report an error.
            Self::Array(arr) => match arr.as_slice() {
                [only] => only.as_number(),
                _ => None,
            },
            Self::Lambda { .. } | Self::Null => None,
        }
    }

    /// A value used as one number, where an empty cell is 0, as in Excel
    /// (`=A3 + 1`, `ROUND(A3, 0)`, ADR-059). Elements of a range keep
    /// [`Value::as_number`], which skips an empty cell, as aggregates must.
    #[must_use]
    pub fn as_cell_number(&self) -> Option<f64> {
        match self {
            Self::Null => Some(0.0),
            other => other.as_number(),
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
            Self::Array(arr) if arr.len() == 1 => arr[0].as_bool(),
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

/// A map by name with fast, fixed hashing: formulas look names up many
/// times per calculation, and the keys are the model's own names.
pub type NameMap<V> = HashMap<String, V, foldhash::fast::FixedState>;

/// Evaluation context containing variables and tables
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Scalar variables (name -> value)
    pub scalars: NameMap<Value>,
    /// Table data (`table_name` -> `column_name` -> values)
    pub tables: NameMap<NameMap<Vec<Value>>>,
    /// Scenarios (`scenario_name` -> `variable_name` -> value)
    pub scenarios: HashMap<String, HashMap<String, f64>>,
    /// Current row index for row-wise evaluation (None for scalar mode)
    pub current_row: Option<usize>,
    /// Number of rows in current table context
    pub row_count: Option<usize>,
    /// Rows back from `current_row` that references are read at: set by
    /// `PREV` while it evaluates its first argument (see [`EvalContext::row`])
    pub lag: std::cell::Cell<usize>,
    /// The table whose columns are being calculated together row by row
    /// (`PREV`): its columns may be shorter than the table while they grow
    pub growing_table: Option<String>,
    /// The arguments being read as whole columns (ADR-058), by the address of
    /// their node in the formula, and how many of them are being evaluated:
    /// while one is, the row is hidden (see [`EvalContext::row`])
    pub whole_args: std::cell::RefCell<Vec<usize>>,
    pub hidden_row: std::cell::Cell<usize>,
}

impl EvalContext {
    /// Create a new empty context
    #[must_use]
    pub fn new() -> Self {
        Self {
            scalars: NameMap::default(),
            tables: NameMap::default(),
            scenarios: HashMap::new(),
            current_row: None,
            row_count: None,
            lag: std::cell::Cell::new(0),
            growing_table: None,
            whole_args: std::cell::RefCell::new(Vec::new()),
            hidden_row: std::cell::Cell::new(0),
        }
    }

    /// The row a row formula is being evaluated in: the current row, or an
    /// earlier one inside `PREV`. `None` outside a row formula.
    #[must_use]
    pub fn row(&self) -> Option<usize> {
        if self.hidden_row.get() > 0 {
            return None;
        }
        self.current_row
            .map(|row| row.saturating_sub(self.lag.get()))
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
    // An argument read as a whole column: evaluated with the row hidden
    if ctx.hidden_row.get() == 0 && !ctx.whole_args.borrow().is_empty() {
        let address = std::ptr::from_ref(expr) as usize;
        if ctx.whole_args.borrow().contains(&address) {
            ctx.hidden_row.set(1);
            let result = evaluate_expr(expr, ctx);
            ctx.hidden_row.set(0);
            return result;
        }
    }
    evaluate_expr(expr, ctx)
}

/// Evaluate a function call, its whole-column arguments (ADR-058) with the
/// row hidden when it is called in a row formula
fn evaluate_call(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    if ctx.row().is_none() {
        return evaluate_function(name, args, ctx);
    }
    let upper = name.to_uppercase();
    if !super::whole_column::has_whole_column(&upper, args.len()) {
        return evaluate_function(name, args, ctx);
    }
    let before = ctx.whole_args.borrow().len();
    ctx.whole_args.borrow_mut().extend(
        args.iter()
            .enumerate()
            .filter(|(position, _)| {
                super::whole_column::is_whole_column(&upper, args.len(), *position)
            })
            .map(|(_, arg)| std::ptr::from_ref(arg) as usize),
    );
    let result = evaluate_function(name, args, ctx);
    ctx.whole_args.borrow_mut().truncate(before);
    result
}

fn evaluate_expr(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
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

        Expr::FunctionCall { name, args } => match evaluate_call(name, args, ctx)? {
            Value::Number(n) if n.is_infinite() => {
                Err(EvalError::new(format!("#NUM!: {name} result is too large")))
            },
            // Excel never returns NaN; it returns #NUM! when a function has no result
            Value::Number(n) if n.is_nan() => Err(EvalError::new(format!(
                "#NUM!: {name} has no result for these arguments"
            ))),
            value => Ok(value),
        },

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

thread_local! {
    /// `table.column` of the reference being looked up, reused so a lookup
    /// does not allocate
    static SCALAR_KEY: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Evaluate a reference (scalar or table.column)
fn evaluate_reference(reference: &Reference, ctx: &EvalContext) -> Result<Value, EvalError> {
    match reference {
        Reference::Scalar(name) => {
            let value = ctx
                .get_scalar(name)
                .ok_or_else(|| EvalError::new(format!("Unknown variable: {name}")))?;

            // In row-wise mode, if the value is an array, extract current row
            // (without copying the whole array first)
            if let Some(row) = ctx.row() {
                if let Value::Array(arr) = value {
                    return arr
                        .get(row)
                        .cloned()
                        .ok_or_else(|| EvalError::new(format!("Row {row} out of bounds")));
                }
            }
            Ok(value.clone())
        },

        Reference::TableColumn { table, column } => {
            // First try as a section.scalar reference (e.g., thresholds.min_value)
            let scalar = SCALAR_KEY.with(|key| {
                let mut key = key.borrow_mut();
                key.clear();
                key.push_str(table);
                key.push('.');
                key.push_str(column);
                ctx.scalars.get(key.as_str()).cloned()
            });
            if let Some(value) = scalar {
                return Ok(value);
            }

            let col = if let Some(namespace) = table.strip_prefix('@') {
                // A column of a table of an included file: @costs.prices.flour
                // (the included tables are in the context as `@costs.prices`)
                let included = column.split_once('.').and_then(|(inner, col)| {
                    SCALAR_KEY.with(|key| {
                        let mut key = key.borrow_mut();
                        key.clear();
                        key.push_str(table);
                        key.push('.');
                        key.push_str(inner);
                        ctx.tables.get(key.as_str()).and_then(|t| t.get(col))
                    })
                });
                included.ok_or_else(|| {
                    EvalError::new(format!(
                        "Unknown included value: {table}.{column} (needs an `_includes` entry \
                         with `as: {namespace}` whose file has a scalar `{column}`, or a \
                         table column `table.column`)"
                    ))
                })?
            } else {
                ctx.get_column(table, column)
                    .ok_or_else(|| EvalError::new(format!("Unknown column: {table}.{column}")))?
            };

            // In row-wise mode, validate row count matches and return single value
            if let Some(row) = ctx.row() {
                // Validate cross-table row count matches current context
                if let Some(expected_count) = ctx.row_count {
                    let growing = ctx.growing_table.as_deref() == Some(table.as_str());
                    if col.len() != expected_count && !growing {
                        return Err(EvalError::new(format!(
                            "Row count mismatch: {table}.{column} has {} rows but expected \
                             {expected_count}. A row formula reads the same row of another \
                             table; to read the whole column, give it to an aggregate as its \
                             range: SUM({table}.{column}) with one argument, or SUMIFS",
                            col.len(),
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

/// Evaluate a binary operation, element by element when an operand is an array.
///
/// Arrays of equal length pair up by position. A scalar or one-element array
/// applies to every element. Any other length mismatch is an error.
fn evaluate_binary_op(op: &str, left: &Value, right: &Value) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::Array(l), Value::Array(r)) if l.len() == r.len() => l
            .iter()
            .zip(r)
            .map(|(a, b)| evaluate_binary_op(op, a, b))
            .collect::<Result<_, _>>()
            .map(Value::Array),
        (Value::Array(l), Value::Array(r)) if r.len() == 1 => broadcast_left(op, l, &r[0]),
        (Value::Array(l), Value::Array(r)) if l.len() == 1 => broadcast_right(op, &l[0], r),
        (Value::Array(l), Value::Array(r)) => Err(EvalError::new(format!(
            "Array length mismatch for '{op}': {} and {}",
            l.len(),
            r.len()
        ))),
        (Value::Array(l), scalar) => broadcast_left(op, l, scalar),
        (scalar, Value::Array(r)) => broadcast_right(op, scalar, r),
        _ => evaluate_scalar_binary_op(op, left, right),
    }
}

/// Apply `op` between each element of `items` and a single right operand.
fn broadcast_left(op: &str, items: &[Value], right: &Value) -> Result<Value, EvalError> {
    items
        .iter()
        .map(|item| evaluate_binary_op(op, item, right))
        .collect::<Result<_, _>>()
        .map(Value::Array)
}

/// Apply `op` between a single left operand and each element of `items`.
fn broadcast_right(op: &str, left: &Value, items: &[Value]) -> Result<Value, EvalError> {
    items
        .iter()
        .map(|item| evaluate_binary_op(op, left, item))
        .collect::<Result<_, _>>()
        .map(Value::Array)
}

/// Evaluate a binary operation on two non-array values
// Arithmetic + comparison + logical operators require many match arms.
#[allow(clippy::too_many_lines)]
/// `base ^ exponent` as Excel computes it, for `^`, `POWER` and `POW`: 0^0
/// and a negative base with a fractional exponent are #NUM!, and 0 to a
/// negative power is #DIV/0!.
pub(crate) fn power(base: f64, exponent: f64) -> Result<Value, EvalError> {
    if base == 0.0 && exponent == 0.0 {
        return Err(EvalError::new("#NUM!: 0^0 is undefined"));
    }
    if base == 0.0 && exponent < 0.0 {
        return Err(EvalError::new("#DIV/0!: zero to a negative power"));
    }
    let result = base.powf(exponent);
    if result.is_nan() {
        return Err(EvalError::new(
            "#NUM!: negative base with a fractional exponent",
        ));
    }
    finite(result)
}

/// Excel returns #NUM! when arithmetic overflows; an f64 would quietly become inf.
fn finite(result: f64) -> Result<Value, EvalError> {
    if result.is_infinite() {
        Err(EvalError::new("#NUM!: result is too large"))
    } else {
        Ok(Value::Number(result))
    }
}

/// Both operands as numbers, or an error naming the side that is not one.
fn numbers(left: &Value, right: &Value) -> Result<(f64, f64), EvalError> {
    let l = left
        .as_number()
        .ok_or_else(|| EvalError::new("Left operand must be a number"))?;
    let r = right
        .as_number()
        .ok_or_else(|| EvalError::new("Right operand must be a number"))?;
    Ok((l, r))
}

fn evaluate_scalar_binary_op(op: &str, left: &Value, right: &Value) -> Result<Value, EvalError> {
    // An empty cell is 0 in arithmetic and comparison, and equal to both 0 and
    // "" (and to FALSE), as in Excel (ADR-059); `&` reads it as ""
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        let empty_equal = |other: &Value| match other {
            Value::Null => true,
            Value::Text(t) => t.is_empty(),
            Value::Boolean(b) => !b,
            other => other.as_number() == Some(0.0),
        };
        let other = if matches!(left, Value::Null) {
            right
        } else {
            left
        };
        match op {
            "=" => return Ok(Value::Boolean(empty_equal(other))),
            "<>" => return Ok(Value::Boolean(!empty_equal(other))),
            "&" => {},
            _ => {
                let zero = Value::Number(0.0);
                let l = if matches!(left, Value::Null) {
                    &zero
                } else {
                    left
                };
                let r = if matches!(right, Value::Null) {
                    &zero
                } else {
                    right
                };
                return evaluate_scalar_binary_op(op, l, r);
            },
        }
    }
    match op {
        // Arithmetic operators
        // `+` adds, as in Excel: a date is its day number and "5" is 5.
        // Text that is neither is #VALUE!; `&` joins text (ADR-054).
        "+" => {
            let (Some(l), Some(r)) = (left.as_number(), right.as_number()) else {
                return Err(EvalError::new(
                    "#VALUE!: + adds numbers; use & or CONCAT to join text",
                ));
            };
            finite(l + r)
        },
        "-" => {
            let (l, r) = numbers(left, right)?;
            finite(l - r)
        },
        "*" => {
            let (l, r) = numbers(left, right)?;
            finite(l * r)
        },
        "/" => {
            let (l, r) = numbers(left, right)?;
            if r == 0.0 {
                Err(EvalError::new("Division by zero"))
            } else {
                finite(l / r)
            }
        },
        "^" => {
            let (l, r) = numbers(left, right)?;
            power(l, r)
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
            let (l, r) = numbers(left, right)?;
            Ok(Value::Boolean(l < r))
        },
        ">" => {
            let (l, r) = numbers(left, right)?;
            Ok(Value::Boolean(l > r))
        },
        "<=" => {
            let (l, r) = numbers(left, right)?;
            Ok(Value::Boolean(l <= r))
        },
        ">=" => {
            let (l, r) = numbers(left, right)?;
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
        // A date is equal to its day number: `d = DATE(2026, 1, 31)` (ADR-054).
        // Other text is never equal to a number, as in Excel (`"5" = 5` is FALSE).
        (Value::Text(text), Value::Number(n)) | (Value::Number(n), Value::Text(text)) => {
            date_serial(text).is_some_and(|serial| (serial - n).abs() < 1e-10)
        },
        // Boolean vs Number is never equal: in Excel and Gnumeric `TRUE = 1`
        // and `FALSE = 0` are FALSE (arithmetic still coerces: TRUE + 1 = 2).
        _ => false,
    }
}

/// Evaluate a unary operation
fn evaluate_unary_op(op: &str, operand: &Value) -> Result<Value, EvalError> {
    if let Value::Array(items) = operand {
        return items
            .iter()
            .map(|item| evaluate_unary_op(op, item))
            .collect::<Result<_, _>>()
            .map(Value::Array);
    }
    match op {
        // -(an empty cell) is 0 (ADR-059)
        "-" if matches!(operand, Value::Null) => Ok(Value::Number(0.0)),
        "-" => {
            let n = operand
                .as_number()
                .ok_or_else(|| EvalError::new("Operand must be a number"))?;
            // 0 - n, so that -(0) is 0 and not -0, as Excel shows it
            Ok(Value::Number(0.0 - n))
        },
        _ => Err(EvalError::new(format!("Unknown unary operator: {op}"))),
    }
}

/// Evaluate a function call - dispatches to category-specific modules
fn evaluate_function(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let upper_name = name.to_uppercase();

    // ═══════════════════════════════════════════════════════════════════════════
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
    if let Some(result) = normal::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = schedule::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = regression::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = statistical::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }
    if let Some(result) = trig::try_evaluate(&upper_name, args, ctx)? {
        return Ok(result);
    }

    // ═══════════════════════════════════════════════════════════════════════════
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

/// Evaluate a scalar argument as a number, using `default` for a non-numeric value.
///
/// # Errors
///
/// Returns an error if evaluation fails or the argument is an array of more
/// than one value, which would otherwise be replaced by `default` unnoticed.
pub(crate) fn number_or(arg: &Expr, ctx: &EvalContext, default: f64) -> Result<f64, EvalError> {
    let value = evaluate(arg, ctx)?;
    if let Value::Array(items) = &value {
        if items.len() != 1 {
            return Err(EvalError::new(format!(
                "Expected a single value, got an array of {}",
                items.len()
            )));
        }
    }
    if matches!(value, Value::Null) {
        return Ok(0.0); // an empty cell given for an argument is 0 (ADR-059)
    }
    Ok(value.as_number().unwrap_or(default))
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
            // A date is a number wherever one is read, from a column (above)
            // or as a single value: `MAX(due, DATE(2026, 6, 1))` (ADR-054)
            Value::Text(text) => values.extend(date_serial(&text)),
            _ => {},
        }
    }

    Ok(values)
}

/// The day number of a date written `YYYY-MM-DD`: the days since 1899-12-30,
/// which is Excel's serial number for every date from 1900-03-01 on.
#[must_use]
pub fn date_serial(text: &str) -> Option<f64> {
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    #[allow(clippy::cast_precision_loss)] // a day count is far below 2^52
    Some((date - base).num_days() as f64)
}

/// The day number of a date Excel holds, written `YYYY-MM-DD`.
///
/// Four, two and two digits, from 1900-03-01 on: a date in a scalar, a date
/// column's cell in a workbook (ADR-054, ADR-060). `None` for any other text.
#[must_use]
pub fn excel_date_serial(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    let serial = date_serial(text).filter(|_| shaped)?;
    // 1900-03-01 is day 61: Excel counts a 29 February 1900 that never was
    (serial >= 61.0).then_some(serial)
}

/// The text of the date with day number `serial` (`2026-10-31` for 46326).
#[must_use]
pub fn serial_date_text(serial: f64) -> String {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)] // a day number, a whole number
    base.checked_add_signed(chrono::TimeDelta::days(serial as i64))
        .map_or_else(|| serial.to_string(), |d| d.format("%Y-%m-%d").to_string())
}

/// The number a criterion such as `>=2026-02-01` or `>50` compares with: a
/// number, or a date as its day number.
fn criterion_number(text: &str) -> Option<f64> {
    let text = text.trim();
    text.parse::<f64>().ok().or_else(|| date_serial(text))
}

/// Collect all values from an expression as a Vec<Value>
/// (used by array functions)
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
/// (used by conditional functions)
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
mod batch_tests;
#[cfg(test)]
mod date_tests;
#[cfg(test)]
mod empty_cell_tests;

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

    /// `POWER` and `POW` gave 1 for 0^0, where Excel and Forge's own `^` give
    /// #NUM!; all three now follow Excel.
    #[test]
    fn power_functions_match_the_operator() {
        let ctx = EvalContext::new();
        for (formula, error) in [
            ("0^0", "#NUM!"),
            ("POWER(0, 0)", "#NUM!"),
            ("POW(0, 0)", "#NUM!"),
            ("POWER(0, -1)", "#DIV/0!"),
            ("POW(-8, 1/3)", "#NUM!"),
        ] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains(error), "{formula}: {err}");
        }
        assert_eq!(eval("POWER(2, 10)", &ctx).unwrap(), Value::Number(1024.0));
        assert_eq!(eval("POW(-2, 3)", &ctx).unwrap(), Value::Number(-8.0));
    }

    /// `ADDRESS` as Excel computes it: R1C1 brackets on the relative part,
    /// `sheet_text`, and #VALUE! outside the sheet (it gave `$A$0`).
    #[test]
    fn address_matches_excel() {
        let ctx = EvalContext::new();
        for (formula, text) in [
            ("ADDRESS(2, 3)", "$C$2"),
            ("ADDRESS(2, 3, 2)", "C$2"),
            ("ADDRESS(2, 3, 2, FALSE)", "R2C[3]"),
            ("ADDRESS(2, 3, 3, FALSE)", "R[2]C3"),
            ("ADDRESS(2, 3, 4, FALSE)", "R[2]C[3]"),
            ("ADDRESS(1, 1, 1, TRUE, \"Sheet1\")", "Sheet1!$A$1"),
            ("ADDRESS(1, 1, 1, TRUE, \"My Sheet\")", "'My Sheet'!$A$1"),
            ("ADDRESS(2, 3, 1, FALSE, \"S\")", "S!R2C3"),
        ] {
            assert_eq!(
                eval(formula, &ctx).unwrap(),
                Value::Text(text.to_string()),
                "{formula}"
            );
        }
        for formula in ["ADDRESS(0, 1)", "ADDRESS(1048577, 1)", "ADDRESS(1, 1, 5)"] {
            assert!(eval(formula, &ctx).is_err(), "{formula}");
        }
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
        // Excel: 0^0 is #NUM!
        assert!(eval("0 ^ 0", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#NUM!"));
        // Excel: 0 to a negative power is #DIV/0!
        assert!(eval("0 ^ -1", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#DIV/0!"));
        // Excel: negative base, fractional exponent is #NUM!
        assert!(eval("(0 - 8) ^ 0.5", &ctx).is_err());
    }

    #[test]
    fn test_eval_overflow_is_num_error() {
        // Excel: arithmetic that overflows is #NUM!, never infinity
        let ctx = EvalContext::new();
        for formula in [
            "2 ^ 1024",
            "1E308 * 10",
            "1E308 + 1E308",
            "-1E308 - 1E308",
            "1E308 / 1E-10",
        ] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains("#NUM!"), "{formula}: {err}");
        }
        for formula in ["POWER(2, 1024)", "EXP(1000)", "SUM(1E308, 1E308)"] {
            let err = eval(formula, &ctx).unwrap_err().to_string();
            assert!(err.contains("#NUM!"), "{formula}: {err}");
        }
        assert_eq!(
            eval("2 ^ 1023", &ctx)
                .unwrap()
                .as_number()
                .map(f64::is_finite),
            Some(true)
        );
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
        // Excel and Gnumeric: a boolean never equals a number
        assert_eq!(eval("TRUE = 1", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("FALSE = 0", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("TRUE <> 1", &ctx).unwrap(), Value::Boolean(true));
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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
        ctx.tables
            .insert("orders".to_string(), table.into_iter().collect());

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
        // An array has no single number, so it must not become its length
        let arr = Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert_eq!(arr.as_number(), None);
        assert_eq!(
            Value::Array(vec![Value::Number(7.0)]).as_number(),
            Some(7.0)
        );
    }

    fn array_ctx() -> EvalContext {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        let column = |values: &[f64]| values.iter().copied().map(Value::Number).collect();
        table.insert("a".to_string(), column(&[1.0, 2.0, 3.0]));
        table.insert("b".to_string(), column(&[10.0, 20.0, 30.0]));
        table.insert("short".to_string(), column(&[1.0, 2.0]));
        table.insert("one".to_string(), column(&[5.0]));
        ctx.tables
            .insert("p".to_string(), table.into_iter().collect());
        ctx
    }

    #[test]
    fn test_array_arithmetic_inside_aggregates() {
        let ctx = array_ctx();
        for (formula, expected) in [
            ("SUM(p.a * p.b)", 140.0),
            ("SUM(p.a + 1)", 9.0),
            ("SUM(1 + p.a)", 9.0),
            ("SUM(-p.a)", -6.0),
            ("SUM(p.b / p.a)", 30.0),
            ("SUM(p.a ^ 2)", 14.0),
            ("SUM(p.a * p.one)", 30.0),
            ("AVERAGE(p.b / p.a)", 10.0),
            ("MAX(p.a + p.b)", 33.0),
            ("MIN(p.a * p.b)", 10.0),
            ("SUM((p.a > 1) * p.b)", 50.0),
        ] {
            assert_eq!(
                eval(formula, &ctx).unwrap(),
                Value::Number(expected),
                "{formula}"
            );
        }
    }

    #[test]
    fn test_array_arithmetic_length_mismatch_is_error() {
        let err = eval("SUM(p.a * p.short)", &array_ctx()).unwrap_err();
        assert!(err.message.contains("length mismatch"), "{}", err.message);
    }

    #[test]
    fn test_array_division_by_zero_element_is_error() {
        let err = eval("SUM(p.a / (p.a - 1))", &array_ctx()).unwrap_err();
        assert!(err.message.contains("zero"), "{}", err.message);
    }

    #[test]
    fn test_if_with_array_condition() {
        let ctx = array_ctx();
        assert_eq!(
            eval("SUM(IF(p.a > 1, p.b, 0))", &ctx).unwrap(),
            Value::Number(50.0)
        );
        assert_eq!(
            eval("SUM(IF(p.a > 1, 1, p.b))", &ctx).unwrap(),
            Value::Number(12.0)
        );
        // No else branch: false elements are skipped, not counted as zero
        assert_eq!(
            eval("AVERAGE(IF(p.a > 1, p.b))", &ctx).unwrap(),
            Value::Number(25.0)
        );
    }

    #[test]
    fn test_logical_functions_with_arrays() {
        let ctx = array_ctx();
        assert_eq!(eval("AND(p.a > 0)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("AND(p.a > 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("OR(p.a > 2)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("OR(p.a > 3)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("XOR(p.a > 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(
            eval("SUM(NOT(p.a > 1) * p.b)", &ctx).unwrap(),
            Value::Number(10.0)
        );
    }

    #[test]
    fn test_array_in_scalar_argument_is_error() {
        let ctx = array_ctx();
        for formula in ["PMT(p.a, 10, 1000)", "ROUND(1.5, p.a)", "TEXT(p.a, \"0\")"] {
            let err = eval(formula, &ctx).unwrap_err();
            assert!(
                err.message.contains("single value"),
                "{formula}: {}",
                err.message
            );
        }
        let err = eval("IFS(p.a > 1, 1, TRUE(), 2)", &ctx).unwrap_err();
        assert!(err.message.contains("single value"), "{}", err.message);
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
