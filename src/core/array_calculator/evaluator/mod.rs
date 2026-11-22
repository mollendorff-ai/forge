//! Formula evaluator for the array calculator
//!
//! Evaluates an AST to produce a result value. Supports both scalar and
//! array (row-wise) evaluation modes.
//!
//! Function implementations are organized into submodules by category.

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
pub(crate) use normal::standard_cdf;
mod regression;
mod schedule;
mod scope;
mod serial;
mod statistical;
pub(crate) mod text;
mod trig;
mod values;
pub(crate) mod wildcard;
pub use scope::{evaluate_without_row, Aliases};
pub use serial::{date_serial, excel_date_serial, serial_date_text};
pub use values::number_text;
pub(crate) use values::{is_na_error, paired_numbers};

mod operators;
mod value;
use operators::{evaluate_binary_op, evaluate_final_op, evaluate_unary_op};
pub(crate) use operators::{power, values_equal};
pub use value::Value;

// Extended evaluator modules
mod advanced;
mod array;
mod conditional;
mod forge;
mod info;

use super::parser::{Expr, Reference};
use num_traits::ToPrimitive;
use std::collections::HashMap;

/// A map by name with fast, fixed hashing: formulas look names up many
/// times per calculation, and the keys are the model's own names.
pub type NameMap<V> = HashMap<String, V, foldhash::fast::FixedState>;

/// Evaluation context containing variables and tables
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Scalar variables (name -> value)
    pub scalars: NameMap<Value>,
    /// Table data (`table_name` -> `column_name` -> values)
    pub tables: NameMap<NameMap<std::sync::Arc<Vec<Value>>>>,
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
    /// Short names (`price` for `summary.price`) the formula being evaluated
    /// may use, with the full name each stands for. A name in `scalars`
    /// comes first, so a column or a LAMBDA parameter laid over the context
    /// hides a short name, as it hid one laid there.
    pub aliases: Option<std::rc::Rc<Aliases>>,
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
            aliases: None,
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
        self.scalars.get(name).or_else(|| {
            let full = self.aliases.as_ref()?.get(name)?;
            self.scalars.get(full)
        })
    }

    /// Get a table column
    #[must_use]
    pub fn get_column(&self, table: &str, column: &str) -> Option<&std::sync::Arc<Vec<Value>>> {
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

/// Evaluate a cell's formula: a scalar's, or one row of a row formula.
///
/// When its last operation is a `+` or `-` that `body` (the formula's text
/// after `=`) does not wrap in parentheses, a result that nearly cancels is
/// 0, as in Excel (ADR-066); anything else is [`evaluate`].
///
/// # Errors
///
/// Returns `EvalError` as [`evaluate`] does.
pub fn evaluate_cell(expr: &Expr, body: &str, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::BinaryOp { op, left, right }
            if (op == "+" || op == "-") && !wrapped_in_parentheses(body) =>
        {
            let left_val = evaluate(left, ctx)?;
            let right_val = evaluate(right, ctx)?;
            evaluate_final_op(op, &left_val, &right_val)
        },
        _ => evaluate(expr, ctx),
    }
}

/// Whether a formula's text is wrapped in one pair of parentheses: `(a+b-c)`,
/// not `(a+b)-c`. Parentheses inside a text in quotes do not count.
#[must_use]
pub fn wrapped_in_parentheses(body: &str) -> bool {
    let body = body.trim().trim_start_matches('=').trim();
    if !body.starts_with('(') {
        return false;
    }
    let mut depth = 0_usize;
    let mut in_text = false;
    for (at, c) in body.char_indices() {
        match c {
            '"' => in_text = !in_text,
            '(' if !in_text => depth += 1,
            ')' if !in_text => {
                depth = depth.saturating_sub(1);
                // The first parenthesis closes: the formula is wrapped only if
                // nothing follows
                if depth == 0 {
                    return body[at + 1..].trim().is_empty();
                }
            },
            _ => {},
        }
    }
    false
}

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

        // An array constant (ADR-064): its items are literals
        Expr::Array { items, .. } => items
            .iter()
            .map(|item| evaluate_expr(item, ctx))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| Value::Array(items.into())),

        Expr::Reference(reference) => evaluate_reference(reference, ctx),

        Expr::ArrayIndex { array, index } => {
            let arr_val = evaluate(array, ctx)?;
            let idx_val = evaluate(index, ctx)?;

            let idx = idx_val
                .as_number()
                .ok_or_else(|| EvalError::new("Array index must be a number"))?;

            // A place from 0, cut to a whole number; a negative one is none
            match arr_val {
                Value::Array(arr) => idx
                    .to_usize()
                    .and_then(|at| arr.get(at))
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
                ctx.get_column(table, column).ok_or_else(|| {
                    // A group of scalars is not a table: `a.price` with only
                    // `a.prce` in the model is a misspelled name.
                    let wanted = format!("{table}.{column}");
                    if ctx.tables.contains_key(table.as_str()) {
                        return EvalError::new(format!("Unknown column: {wanted}"));
                    }
                    let names: Vec<&str> = ctx
                        .scalars
                        .keys()
                        .chain(ctx.aliases.iter().flat_map(|a| a.keys()))
                        .map(String::as_str)
                        .collect();
                    let hint = crate::error::closest(&wanted, &names)
                        .map(|c| format!(" Did you mean '{c}'?"))
                        .unwrap_or_default();
                    EvalError::new(format!("Unknown name: {wanted}.{hint}"))
                })?
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
                // A count, not a copy (T1-COLUMN-COPIES)
                Ok(Value::Array(std::sync::Arc::clone(col)))
            }
        },
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
    Ok(collect_numbers_and_dates(args, ctx)?.0)
}

/// Whether a value is a date Excel holds: date text from 1900-03-01 on.
pub(crate) fn is_a_date(value: &Value) -> bool {
    matches!(value, Value::Text(text) if excel_date_serial(text).is_some())
}

/// The numbers of `args` as `collect_numeric_values` reads them, and whether
/// they are all dates: at least one, and none a plain number (ADR-060
/// addendum). `MAX`, `MIN`, `LARGE` and `SMALL` of dates give a date.
pub(crate) fn collect_numbers_and_dates(
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<(Vec<f64>, bool), EvalError> {
    let mut values = Vec::new();
    let (mut dates, mut plain) = (false, false);

    for arg in args {
        let val = evaluate(arg, ctx)?;
        match val {
            Value::Array(arr) => {
                for v in arr.iter() {
                    if let Some(n) = v.as_number() {
                        values.push(n);
                        if is_a_date(v) {
                            dates = true;
                        } else {
                            plain = true;
                        }
                    }
                }
            },
            Value::Number(n) => {
                values.push(n);
                plain = true;
            },
            // A date is a number wherever one is read, from a column (above)
            // or as a single value: `MAX(due, DATE(2026, 6, 1))` (ADR-054)
            Value::Text(ref text) => {
                if let Some(n) = date_serial(text) {
                    values.push(n);
                    if is_a_date(&val) {
                        dates = true;
                    } else {
                        plain = true;
                    }
                }
            },
            _ => {},
        }
    }

    Ok((values, dates && !plain))
}

/// `number`, shown as the date it is when `is_date` says it came from dates.
pub(crate) fn number_or_date(number: f64, is_date: bool) -> Value {
    if is_date {
        Value::Text(serial_date_text(number))
    } else {
        Value::Number(number)
    }
}

/// The number a criterion such as `>=2026-02-01` or `>50` compares with: a
/// number, or a date as its day number.
fn criterion_number(text: &str) -> Option<f64> {
    let text = text.trim();
    text.parse::<f64>().ok().or_else(|| date_serial(text))
}

/// Collect all values from an expression as a Vec<Value>
/// (used by array functions)
/// A column comes back shared, not copied (T1-COLUMN-COPIES).
pub(crate) fn collect_values_as_vec(
    expr: &Expr,
    ctx: &EvalContext,
) -> Result<std::sync::Arc<Vec<Value>>, EvalError> {
    let val = evaluate(expr, ctx)?;
    match val {
        Value::Array(arr) => Ok(arr),
        other => Ok(std::sync::Arc::new(vec![other])),
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
            // Whole days on from it: a negative serial is no date
            let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            n.to_u64()
                .and_then(|days| base.checked_add_days(chrono::Days::new(days)))
                .ok_or_else(|| EvalError::new(format!("Invalid Excel date serial: {n}")))
        },
        _ => Err(EvalError::new("Expected date string or serial number")),
    }
}

#[cfg(test)]
mod tests {

    mod wrapped {
        //! A formula wrapped in one pair of parentheses has no last `+` or
        //! `-` of its own, as Excel reads it (ADR-066)

        use super::super::wrapped_in_parentheses;

        #[test]
        fn only_one_pair_around_the_whole_formula_wraps_it() {
            for wrapped in ["=(a+b-c)", "((a+b-c))", " = ( a - b ) ", "(\"(\" & a)"] {
                assert!(wrapped_in_parentheses(wrapped), "{wrapped}");
            }
            for not in [
                "=a+b-c",
                "=(a+b)-c",
                "=(a)+(b)",
                "=a-(b+c)",
                "=(\")\" & a)-b",
                "=(a+b",
                "=)a(",
                "",
            ] {
                assert!(!wrapped_in_parentheses(not), "{not}");
            }
        }
    }
    use super::*;
    use crate::core::array_calculator::parser::parse;
    use crate::core::array_calculator::tokenizer::tokenize;

    pub fn eval(formula: &str, ctx: &EvalContext) -> Result<Value, EvalError> {
        let tokens = tokenize(formula).map_err(|e| EvalError::new(e.message))?;
        let ast = parse(tokens).map_err(|e| EvalError::new(e.message))?;
        evaluate(&ast, ctx)
    }

    // Helpers for the tests of every module of the evaluator, which reach
    // them as `evaluator::tests`

    /// A context with one table, `t`, of these columns
    pub fn table(columns: &[(&str, Vec<Value>)]) -> EvalContext {
        let mut ctx = EvalContext::new();
        let table: HashMap<String, Vec<Value>> = columns
            .iter()
            .map(|(name, values)| ((*name).to_string(), values.clone()))
            .collect();
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );
        ctx
    }

    pub fn numbers(values: &[f64]) -> Vec<Value> {
        values.iter().map(|v| Value::Number(*v)).collect()
    }

    pub fn texts(items: &[&str]) -> Vec<Value> {
        items
            .iter()
            .map(|s| Value::Text((*s).to_string()))
            .collect()
    }

    pub fn number_in(formula: &str, ctx: &EvalContext) -> f64 {
        match eval(formula, ctx) {
            Ok(Value::Number(n)) => n,
            other => panic!("{formula}: {other:?}"),
        }
    }

    pub fn number(formula: &str) -> f64 {
        number_in(formula, &EvalContext::new())
    }

    pub fn text_in(formula: &str, ctx: &EvalContext) -> String {
        match eval(formula, ctx) {
            Ok(Value::Text(t)) => t,
            other => panic!("{formula}: {other:?}"),
        }
    }

    pub fn text(formula: &str) -> String {
        text_in(formula, &EvalContext::new())
    }

    pub fn truth(formula: &str) -> bool {
        match eval(formula, &EvalContext::new()) {
            Ok(Value::Boolean(b)) => b,
            other => panic!("{formula}: {other:?}"),
        }
    }

    /// The formula is an error, any error
    pub fn fails(formula: &str) {
        assert!(eval(formula, &EvalContext::new()).is_err(), "{formula}");
    }

    /// The formula is an error whose message holds `code`
    pub fn fails_in(formula: &str, ctx: &EvalContext, code: &str) {
        match eval(formula, ctx) {
            Err(e) => assert!(e.to_string().contains(code), "{formula}: {e}"),
            Ok(v) => panic!("{formula} gave {v:?}, not {code}"),
        }
    }

    pub fn fails_with(formula: &str, code: &str) {
        fails_in(formula, &EvalContext::new(), code);
    }

    pub fn error_in(formula: &str, ctx: &EvalContext) -> String {
        eval(formula, ctx).unwrap_err().to_string()
    }

    /// Within 1e-9 of `b`, relative to it once it is past 1
    pub fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    /// The formula's number is `value` to the last bit: an exact answer,
    /// checked without comparing floats with `==`
    pub fn is_exactly(formula: &str, value: f64) {
        is_exactly_in(formula, &EvalContext::new(), value);
    }

    pub fn is_exactly_in(formula: &str, ctx: &EvalContext, value: f64) {
        let got = number_in(formula, ctx);
        assert!(
            got.to_bits() == value.to_bits(),
            "{formula}: {got}, not exactly {value}"
        );
    }

    /// Each argument of `name(args)`, a call that gives an answer: an unknown
    /// name in its place is reported as unknown, the error passed on as it
    /// is, and where the argument has a message, text in its place is
    /// refused with an error that starts with that message
    pub fn each_argument(name: &str, args: &[(&str, Option<&str>)]) {
        let call = |at: usize, with: &str| {
            let given: Vec<&str> = (0..)
                .zip(args)
                .map(|(i, (arg, _))| if i == at { with } else { *arg })
                .collect();
            format!("{name}({})", given.join(", "))
        };
        let whole = call(usize::MAX, "");
        assert!(eval(&whole, &EvalContext::new()).is_ok(), "{whole} fails");
        for (at, (_, refused)) in (0..).zip(args) {
            let unknown = call(at, "nope");
            assert_eq!(
                eval(&unknown, &EvalContext::new()).unwrap_err().message,
                "Unknown variable: nope",
                "{unknown}"
            );
            if let Some(message) = refused {
                let text = call(at, "\"x\"");
                let error = eval(&text, &EvalContext::new()).unwrap_err().message;
                assert!(error.starts_with(message), "{text}: {error}");
            }
        }
    }

    pub fn sweep_is_exactly(formula: &str, value: f64) {
        is_exactly_in(formula, &sweep_table(), value);
    }

    /// The formula's number within 1e-14 of the exact value, relative to it
    /// once it is past 1
    pub fn exactly(formula: &str, exact: f64) {
        let got = number(formula);
        assert!(
            (got - exact).abs() <= 1e-14 * exact.abs().max(1.0),
            "{formula}: {got}, exact {exact}"
        );
    }

    /// The table of the sweeps against Gnumeric and `LibreOffice`
    /// (`validation/sweep`): names `t.nm`, prices `t.pr`, lookups `t.lu` and
    /// a column with an empty text, `t.s`
    pub fn sweep_table() -> EvalContext {
        table(&[
            (
                "nm",
                texts(&["apple", "Banana", "cherry", "apple pie", "date", "b*"]),
            ),
            ("pr", numbers(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0])),
            ("lu", numbers(&[4.0, 2.0, 9.0, 2.0, 7.0, 4.0])),
            ("s", texts(&["a", "", "b", "c"])),
        ])
    }

    pub fn sweep_number(formula: &str) -> f64 {
        number_in(formula, &sweep_table())
    }

    pub fn sweep_text(formula: &str) -> String {
        text_in(formula, &sweep_table())
    }

    pub fn sweep_fails(formula: &str, code: &str) {
        fails_in(formula, &sweep_table(), code);
    }

    /// The table of ADR-059: `t.x` is 10, empty, 30, empty, 0; `t.b` is a
    /// column of only empty cells; `blank` is one empty cell
    pub fn empty_cells() -> EvalContext {
        let n = Value::Number;
        let mut ctx = table(&[
            (
                "x",
                vec![n(10.0), Value::Null, n(30.0), Value::Null, n(0.0)],
            ),
            ("b", vec![Value::Null; 5]),
        ]);
        ctx.scalars.insert("blank".into(), Value::Null);
        ctx
    }

    #[test]
    fn test_eval_number() {
        let ctx = EvalContext::new();
        let result = eval("42", &ctx).unwrap();
        assert_eq!(result, Value::Number(42.0));
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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "orders".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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

    pub fn array_ctx() -> EvalContext {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        let column = |values: &[f64]| {
            values
                .iter()
                .copied()
                .map(Value::Number)
                .collect::<Vec<_>>()
        };
        table.insert("a".to_string(), column(&[1.0, 2.0, 3.0]));
        table.insert("b".to_string(), column(&[10.0, 20.0, 30.0]));
        table.insert("short".to_string(), column(&[1.0, 2.0]));
        table.insert("one".to_string(), column(&[5.0]));
        ctx.tables.insert(
            "p".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );
        ctx
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

    /// A date is a date (ADR-054): the text `2026-01-31` in a model, and its day
    /// number wherever a formula reads a number. `+` adds and `&` joins, as in
    /// Excel. Each result here is what Gnumeric and `LibreOffice` give for the
    /// exported formula over a real date (forge-e2e-gnumeric,
    /// `edge_real_dates.yaml`).
    mod dates_as_dates {
        use crate::core::array_calculator::evaluator::tests as h;

        /// `MAX(d, DATE(2026, 6, 1))` ignored the date `d` of a row, and
        /// `COUNT(d, e, n)` counted only the number.
        #[test]
        fn a_single_date_counts_as_a_number_in_aggregates() {
            // MAX and MIN of dates give a date (ADR-060 addendum)
            assert_eq!(
                h::eval(
                    "MAX(\"2026-12-01\", DATE(2026, 6, 1))",
                    &crate::core::array_calculator::evaluator::EvalContext::new()
                ),
                Ok(crate::core::array_calculator::evaluator::Value::Text(
                    "2026-12-01".to_string()
                ))
            );
            assert_eq!(
                h::eval(
                    "MIN(\"2026-01-31\", \"2026-02-01\")",
                    &crate::core::array_calculator::evaluator::EvalContext::new()
                ),
                Ok(crate::core::array_calculator::evaluator::Value::Text(
                    "2026-01-31".to_string()
                ))
            );
            assert!((h::number("SUM(\"2026-01-31\", 1)") - 46054.0).abs() < 1e-12);
            assert!((h::number("AVERAGE(\"2026-01-31\", \"2026-02-02\")") - 46054.0).abs() < 1e-12);
            assert!((h::number("COUNT(\"2026-01-31\", 1, \"x\")") - 2.0).abs() < 1e-12);
            // Text that is not a date is still not a number here
            assert!((h::number("SUM(\"x\", 1)") - 1.0).abs() < 1e-12);
        }
    }

    /// An empty cell (ADR-059): the table of the ADR, measured in Gnumeric 1.12
    /// and `LibreOffice` 26.2 on 2026-11-12, which agree on every value here.
    /// `x` is 10, empty, 30, empty, 0; `blank` is one empty cell; `b` is a column
    /// of empty cells.
    mod empty_cells {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::Value;

        #[test]
        fn aggregates_skip_an_empty_cell_and_count_a_zero() {
            let ctx = h::empty_cells();
            for (formula, expected) in [
                ("SUM(t.x)", 40.0),
                ("COUNT(t.x)", 3.0),
                ("COUNTA(t.x)", 3.0),
                ("COUNTBLANK(t.x)", 2.0),
                ("AVERAGE(t.x)", 40.0 / 3.0),
                ("MEDIAN(t.x)", 10.0),
                ("MIN(t.x)", 0.0),
                ("MAX(t.x)", 30.0),
                ("PRODUCT(t.x)", 0.0),
                ("LARGE(t.x, 3)", 0.0),
                ("SMALL(t.x, 1)", 0.0),
                ("RANK(30, t.x)", 1.0),
                ("PERCENTILE(t.x, 0.5)", 10.0),
                ("SUMPRODUCT(t.x, t.x)", 1000.0),
                ("COUNTIF(t.x, \"\")", 2.0),
                ("COUNTIF(t.x, \"=\")", 2.0),
                ("COUNTIF(t.x, \"<>\")", 3.0),
                ("COUNTIF(t.x, 0)", 1.0),
                ("COUNTIFS(t.x, \"\")", 2.0),
                ("SUMIF(t.x, \">=0\")", 40.0),
                ("AVERAGEIF(t.x, \">=0\")", 40.0 / 3.0),
                // 10 / 1.1 + 30 / 1.1^2 + 0 / 1.1^3: the empty cells are skipped
                ("NPV(0.1, t.x)", 10.0 / 1.1 + 30.0 / 1.21),
            ] {
                let n = h::number_in(formula, &ctx);
                assert!((n - expected).abs() < 1e-12, "{formula}: {n}");
            }
            // STDEV of 10, 30, 0, the empty cells left out: R gives sd(c(10, 30, 0))
            let n = h::number_in("STDEV(t.x)", &ctx);
            assert!((n - 15.275_252_316_519_467).abs() < 1e-12, "{n}");
        }

        #[test]
        fn a_column_of_only_empty_cells() {
            let ctx = h::empty_cells();
            for formula in ["SUM(t.b)", "COUNT(t.b)", "MAX(t.b)", "MIN(t.b)"] {
                // 0, and not -0 or an error
                let value = h::eval(formula, &ctx).unwrap();
                assert!(
                    matches!(value, Value::Number(n) if n == 0.0 && n.is_sign_positive()),
                    "{formula}: {value:?}"
                );
            }
            assert!(h::eval("AVERAGE(t.b)", &ctx).is_err());
            assert_eq!(
                h::eval("COUNTBLANK(t.b)", &ctx).unwrap(),
                Value::Number(5.0)
            );
        }
    }

    /// An index from 0 is cut to a whole number; a negative one is no place,
    /// where the cast made it 0. A negative day number is no date.
    #[test]
    fn an_index_and_a_serial_are_whole_and_not_negative() {
        let ctx = table(&[("col", numbers(&[10.0, 20.0, 30.0]))]);
        is_exactly_in("t.col[1.9]", &ctx, 20.0);
        is_exactly_in("t.col[-0.5]", &ctx, 10.0);
        assert!(error_in("t.col[-1]", &ctx).contains("out of bounds"));
        assert!(error_in("t.col[1E+20]", &ctx).contains("out of bounds"));
        is_exactly("YEAR(0.5)", 1899.0);
        is_exactly("YEAR(45000.9)", 2023.0);
        fails_with("YEAR(-1)", "Invalid Excel date serial");
        fails_with("YEAR(1E+20)", "Invalid Excel date serial");
    }

    /// Text is no index
    #[test]
    fn an_index_that_is_no_number_fails() {
        let ctx = table(&[("col", numbers(&[10.0]))]);
        assert!(error_in("t.col[\"x\"]", &ctx).contains("must be a number"));
    }

    /// An error in a part of an expression is passed on as it is
    #[test]
    fn an_error_in_a_part_is_passed_on() {
        let ctx = table(&[("col", numbers(&[10.0, 20.0]))]);
        for formula in [
            "nope[0]",
            "t.col[nope]",
            "(1 + nope)(1)",
            "LAMBDA(x, x * 2)(nope)",
            "-nope",
            "nope:1",
            "1:nope",
            "FILTER(nope, TRUE)",
        ] {
            assert_eq!(
                eval(formula, &ctx).unwrap_err().message,
                "Unknown variable: nope",
                "{formula}"
            );
        }
    }

    /// What is not an array cannot be indexed, what is not a LAMBDA cannot
    /// be called, and a LAMBDA takes as many arguments as it names
    #[test]
    fn what_cannot_be_indexed_or_called_is_refused() {
        let ctx = EvalContext::new();
        let refused = |formula| eval(formula, &ctx).unwrap_err().message;
        assert_eq!(refused("5[0]"), "Cannot index non-array value");
        assert_eq!(refused("(5)(1)"), "Cannot call non-lambda value");
        assert_eq!(
            refused("LAMBDA(x, x * 2)(1, 2)"),
            "Lambda expects 1 arguments, got 2"
        );
    }

    /// A range is its two ends as text
    #[test]
    fn a_range_is_its_ends_as_text() {
        let ctx = table(&[("col", numbers(&[10.0, 20.0]))]);
        assert_eq!(eval("2:3", &ctx), Ok(Value::Text("2:3".to_string())));
    }

    /// A row past the end of a column, or of an array scalar, is out of
    /// bounds; with no row count to check against, a column's row is read
    #[test]
    fn a_row_past_the_end_is_out_of_bounds() {
        let mut ctx = table(&[("col", numbers(&[10.0, 20.0]))]);
        ctx.scalars
            .insert("x".into(), Value::Array(numbers(&[1.0]).into()));
        let ctx = ctx.with_row(5, 2);
        let refused = |formula| eval(formula, &ctx).unwrap_err().message;
        assert_eq!(refused("x"), "Row 5 out of bounds");
        assert_eq!(refused("t.col"), "Row 5 out of bounds");
        let mut ctx = table(&[("col", numbers(&[10.0, 20.0]))]);
        ctx.current_row = Some(1);
        assert_eq!(eval("t.col", &ctx), Ok(Value::Number(20.0)));
    }

    /// A name that is neither in the context nor one of its short names is
    /// unknown; the default context has no names
    #[test]
    fn a_name_in_no_scope_is_unknown() {
        let mut ctx = EvalContext::default();
        assert_eq!(
            eval("nope", &ctx).unwrap_err().message,
            "Unknown variable: nope"
        );
        ctx.aliases = Some(std::rc::Rc::new(scope::Aliases::default()));
        assert_eq!(
            eval("nope", &ctx).unwrap_err().message,
            "Unknown variable: nope"
        );
    }

    /// A number argument given as an array of one value is that value, and
    /// an empty cell is 0 (ADR-059)
    #[test]
    fn a_number_from_one_value_or_an_empty_cell() {
        is_exactly("PMT({0}, 10, -1000)", 100.0);
        is_exactly_in("PMT(blank, 10, -1000)", &empty_cells(), 100.0);
    }

    /// Date text before 1900-03-01, a date Excel does not hold, is read as a
    /// plain number, so the answer is one too
    #[test]
    fn early_date_text_is_a_plain_number() {
        assert_eq!(
            eval("MAX(\"1900-01-05\", \"1900-01-03\")", &EvalContext::new()),
            Ok(Value::Number(6.0))
        );
    }

    /// A date is text or a number; anything else is refused
    #[test]
    fn a_date_is_text_or_a_number() {
        assert_eq!(
            parse_date_value(&Value::Boolean(true)).unwrap_err().message,
            "Expected date string or serial number"
        );
    }

    /// The tests' own helpers: a formula that does not tokenize or parse is
    /// an error, and each helper refuses an answer of the wrong kind
    mod helpers {
        use super::*;

        #[test]
        fn a_formula_that_does_not_read_is_an_error() {
            let ctx = EvalContext::new();
            assert_eq!(
                eval("\"open", &ctx).unwrap_err().message,
                tokenize("\"open").unwrap_err().message
            );
            assert_eq!(
                eval("1 +", &ctx).unwrap_err().message,
                parse(tokenize("1 +").unwrap()).unwrap_err().message
            );
        }

        #[test]
        #[should_panic(expected = "\"a\": Ok(Text(\"a\"))")]
        fn number_refuses_text() {
            number("\"a\"");
        }

        #[test]
        #[should_panic(expected = "1: Ok(Number(1.0))")]
        fn text_refuses_a_number() {
            text("1");
        }

        #[test]
        #[should_panic(expected = "1: Ok(Number(1.0))")]
        fn truth_refuses_a_number() {
            truth("1");
        }

        #[test]
        #[should_panic(expected = "1 gave Number(1.0), not #NUM!")]
        fn fails_with_refuses_an_answer() {
            fails_with("1", "#NUM!");
        }
    }
}
