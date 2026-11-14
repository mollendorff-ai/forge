//! What a formula sees besides the context's own names: the short names of
//! its section (T1-SCALAR-SCALING), and a row hidden while a table argument
//! is read as a whole column.

use super::{evaluate_expr, EvalContext, EvalError, NameMap, Value};
use crate::core::array_calculator::parser::Expr;

/// The short names a formula of one section may use: those of its own
/// section first (`price` for `summary.price` in `summary`), then the ones
/// that name exactly one scalar in the model, from whichever section.
#[derive(Debug, Default)]
pub struct Aliases {
    /// The section's own scalars, by their last part
    pub own: std::rc::Rc<NameMap<String>>,
    /// Every last part that names one scalar only, and no top-level scalar
    pub unique: std::rc::Rc<NameMap<String>>,
}

impl Aliases {
    /// The full name `short` stands for
    #[must_use]
    pub fn get(&self, short: &str) -> Option<&String> {
        self.own.get(short).or_else(|| self.unique.get(short))
    }

    /// Every short name
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.own.keys().chain(self.unique.keys())
    }
}

/// Evaluate `expr` with the row hidden, so a column reads as a whole.
///
/// For the table argument of VLOOKUP, HLOOKUP and OFFSET. It used to be
/// evaluated in a copy of the context without a row, which copied every
/// scalar and table for each row of a row formula.
///
/// # Errors
///
/// Returns `EvalError` if `expr` cannot be evaluated.
pub fn evaluate_without_row(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    let before = ctx.hidden_row.get();
    ctx.hidden_row.set(before + 1);
    let result = evaluate_expr(expr, ctx);
    ctx.hidden_row.set(before);
    result
}
