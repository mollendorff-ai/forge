//! The shared context a formula is evaluated in, with the names of its table
//! and its section laid over it for that formula only.

use super::{column_values, evaluator, ArrayCalculator};
use crate::types::Table;
use std::rc::Rc;

/// The number of each column of `table`, as the export lays it out: in model
/// order ([`Table::column_names`]), the first being 1 (column A).
pub(super) fn column_numbers(table: &Table) -> evaluator::NameMap<usize> {
    table
        .column_names()
        .into_iter()
        .zip(1..)
        .map(|(name, number)| (name.clone(), number))
        .collect()
}

/// The shared context with a formula's names laid over it, and what to put
/// back when the formula is done.
pub(super) struct LaidContext {
    pub(super) ctx: evaluator::EvalContext,
    pub(super) saved: Vec<(String, Option<evaluator::Value>)>,
    pub(super) aliases: Option<Rc<evaluator::Aliases>>,
}

impl ArrayCalculator {
    /// The shared context with what a formula of `table` or `section` sees
    /// laid over it: [`Self::evaluate_in_context`] without the formula, so
    /// that there is one of it whatever the formula returns.
    pub(super) fn lay_context(
        &mut self,
        table: Option<&str>,
        section: Option<&str>,
    ) -> LaidContext {
        let mut ctx = self.ctx.take().unwrap_or_else(|| self.base_context());
        // Each name laid over the shared context, with what it held before: a
        // name laid twice is put back to its first value last
        let mut saved: Vec<(String, Option<evaluator::Value>)> = Vec::new();
        let mut lay = |ctx: &mut evaluator::EvalContext, name: String, value: evaluator::Value| {
            let previous = ctx.scalars.insert(name.clone(), value);
            saved.push((name, previous));
        };

        // The shared context holds each calculated scalar under its full name
        // (a full name is never another short name's key, so this reads the
        // scalar's own value)
        let short_names = self.short_names(section);
        for (short_name, full) in &short_names.shadowing {
            if let Some(value) = self.model_value(&ctx, full) {
                lay(&mut ctx, short_name.clone(), value);
            }
        }
        let aliases = ctx.aliases.replace(Rc::clone(&short_names.aliases));
        // A row formula sets its column; any other formula has none
        ctx.formula_table = table.map(str::to_string);
        ctx.formula_column = None;
        let table = table.and_then(|name| self.model.tables.get(name));
        if let Some(table) = table {
            for (col_name, col) in &table.columns {
                lay(
                    &mut ctx,
                    col_name.clone(),
                    evaluator::Value::Array(column_values(&col.values).into()),
                );
            }
        }
        ctx.current_row = None;
        ctx.row_count = Some(table.map_or(0, Table::row_count));
        LaidContext {
            ctx,
            saved,
            aliases,
        }
    }

    /// The shared context as it was before [`Self::lay_context`].
    pub(super) fn restore_context(&mut self, laid: LaidContext) {
        let LaidContext {
            mut ctx,
            saved,
            aliases,
        } = laid;
        ctx.aliases = aliases;
        for (name, previous) in saved.into_iter().rev() {
            match previous {
                Some(value) => ctx.scalars.insert(name, value),
                None => ctx.scalars.remove(&name),
            };
        }
        self.ctx = Some(ctx);
    }
}
