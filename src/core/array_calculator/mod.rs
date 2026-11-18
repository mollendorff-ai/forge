mod errors;
pub mod evaluator;
mod includes;
mod names;
mod order;
pub mod parser;
mod plan;
mod recursive;
pub mod tokenizer;
pub mod whole_column;

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};
use std::collections::HashMap;
use std::rc::Rc;

use errors::{failure_report, locate_error, not_a_number};
use includes::IncludedTable;
use names::Sections;
use order::Cell;
pub use plan::CalcPlan;

/// Array-aware calculator for v1.0.0 models
/// Handles both row-wise (element-wise) and aggregation formulas
pub struct ArrayCalculator {
    model: ParsedModel,
    /// Scalars of included files as `@namespace.path`, calculated before the
    /// model (see [`ArrayCalculator::included_values`]).
    included: Vec<(String, evaluator::Value)>,
    /// Tables of included files as `@namespace.table`, with their calculated
    /// columns and the file they come from
    included_tables: Vec<IncludedTable>,
    /// Scalar names by their last part (`price` -> [`a.price`, `b.price`]),
    /// sorted. Calculation sets values, never names, so it is built once.
    by_last_part: std::cell::OnceCell<HashMap<String, Vec<String>>>,
    /// Each section's scalars by their last part, and the last parts that
    /// name one scalar only (see [`ArrayCalculator::short_names`]); built once
    sections: std::cell::OnceCell<Sections>,
    /// Every value a formula can read by its full name (scalars, included
    /// values, tables, scenarios), kept up to date as cells are calculated,
    /// so it is not rebuilt for each formula (see [`ArrayCalculator::evaluate_in_context`]).
    ctx: Option<evaluator::EvalContext>,
    /// Parsed formulas, order and name resolution, shared with other
    /// calculations of models with the same formulas
    plan: Rc<CalcPlan>,
    /// [`plan::fingerprint`] of the model as given, set by `calculate_all`
    fingerprint: u64,
    /// Position in the calculation order of the cell being calculated
    step: Option<usize>,
}

/// What a calculation of every cell left: the failures, each starting with
/// its cell's name; the cells not calculated because they use a failed one;
/// and every failed or skipped cell.
#[derive(Default)]
struct Outcome {
    failures: Vec<String>,
    skipped: Vec<(Cell, Cell)>,
    failed: std::collections::BTreeSet<Cell>,
}

impl Outcome {
    /// The error for the failures, both lists in name order
    fn report(mut self) -> ForgeError {
        self.failures.sort();
        self.skipped.sort_by_key(|(cell, _)| cell.label());
        failure_report(&self.failures, &self.skipped)
    }
}

impl ArrayCalculator {
    #[must_use]
    pub fn new(model: ParsedModel) -> Self {
        Self::with_plan(model, CalcPlan::new())
    }

    /// A calculator that reuses `plan`: what an earlier calculation of a model
    /// with the same names and formulas worked out (parsed formulas, the
    /// calculation order, name resolution) is not worked out again. Results
    /// are the same as with [`ArrayCalculator::new`].
    #[must_use]
    pub const fn with_plan(model: ParsedModel, plan: Rc<CalcPlan>) -> Self {
        Self {
            model,
            included: Vec::new(),
            included_tables: Vec::new(),
            by_last_part: std::cell::OnceCell::new(),
            sections: std::cell::OnceCell::new(),
            ctx: None,
            plan,
            fingerprint: 0,
            step: None,
        }
    }

    /// Calculate all formulas in the model
    /// Returns updated model with calculated values
    ///
    /// # Errors
    ///
    /// Returns an error if formula evaluation fails (circular dependency,
    /// unknown variable reference, or invalid expression).
    ///
    /// # Panics
    ///
    /// Panics if the model's table map is in an inconsistent state (should not
    /// occur with valid `ParsedModel` construction).
    pub fn calculate_all(mut self) -> ForgeResult<ParsedModel> {
        let outcome = self.calculate_cells()?;
        if outcome.failures.is_empty() {
            check_formula_formats(&self.model)?;
            return Ok(self.model);
        }
        Err(outcome.report())
    }

    /// Calculate the model for one scalar, `name`, and return its value.
    ///
    /// Every cell is calculated, as [`Self::calculate_all`] does, but a failed
    /// formula that `name` does not use does not fail the answer: a search
    /// that moves an input (goal seek, break-even, sensitivity) can pass
    /// through values where an output it does not ask for has no answer, such
    /// as an IRR with cash flows of one sign.
    ///
    /// # Errors
    ///
    /// Returns the calculation's report when `name` failed or uses a failed
    /// cell, or when the model cannot be calculated at all (a cycle, a table
    /// of uneven columns); and an error when `name` is not a scalar with a
    /// value.
    pub fn calculate_scalar(mut self, name: &str) -> ForgeResult<f64> {
        let outcome = self.calculate_cells()?;
        if outcome.failed.contains(&Cell::Scalar(name.to_string())) {
            return Err(outcome.report());
        }
        if outcome.failures.is_empty() {
            check_formula_formats(&self.model)?;
        }
        self.model
            .scalars
            .get(name)
            .ok_or_else(|| {
                ForgeError::Validation(format!("Output variable '{name}' not found in model"))
            })?
            .value
            .ok_or_else(|| ForgeError::Validation(format!("Output variable '{name}' has no value")))
    }

    /// Calculate every cell of the model in order. A failing cell does not
    /// stop the calculation: every cell that does not use it is still
    /// calculated, so all errors are reported at once. A failed cell gets no
    /// value, so nothing can read a number from it.
    ///
    /// # Errors
    ///
    /// Returns an error when the model cannot be calculated at all: an
    /// included file that fails, uneven columns in a table with row formulas,
    /// or a cycle.
    fn calculate_cells(&mut self) -> ForgeResult<Outcome> {
        self.fingerprint = plan::fingerprint(&self.model);
        (self.included, self.included_tables) = self.included_values()?;

        // A formula scalar's stored value (from the file, an import or an
        // earlier run) is never read: it is unknown until it is recalculated.
        // Callers that set a value on purpose (scenarios, sensitivity,
        // simulation) clear the formula when they do.
        for var in self.model.scalars.values_mut() {
            if var.formula.is_some() {
                var.value = None;
                var.is_date = false;
            }
        }

        // Row formulas need aligned columns
        let mut table_names: Vec<String> = self.model.tables.keys().cloned().collect();
        table_names.sort();
        for table_name in &table_names {
            let table = &self.model.tables[table_name];
            if !table.row_formulas.is_empty() {
                if let Err(e) = table.validate_lengths() {
                    return Err(ForgeError::Eval(format!("Table '{table_name}': {e}")));
                }
            }
        }

        // One order for table columns and scalars together, so a column that
        // uses a calculated scalar, and a scalar that sums a calculated
        // column, both see the current value.
        let mut outcome = Outcome::default();
        let order = self.planned_order()?;
        for (step, (cell, dependencies)) in order.iter().enumerate() {
            self.step = Some(step);
            let blocked_by = dependencies
                .iter()
                .find(|dep| outcome.failed.contains(*dep));
            if let Some(dep) = blocked_by {
                outcome.skipped.push((cell.clone(), dep.clone()));
                outcome.failed.extend(cell.members());
                continue;
            }
            let result = match cell {
                Cell::Column { table, column } => self.calculate_column(table, column),
                Cell::Scalar(name) => self.calculate_scalar_cell(name),
                Cell::Columns { table, columns } => self.calculate_columns(table, columns),
            };
            if let Err(e) = result {
                outcome.failures.push(match e {
                    ForgeError::Eval(msg) => msg,
                    other => other.to_string(),
                });
                outcome.failed.extend(cell.members());
            }
        }
        Ok(outcome)
    }

    /// Calculate one row-formula column and store it in its table
    fn calculate_column(&mut self, table_name: &str, col_name: &str) -> ForgeResult<()> {
        let formula = self.model.tables[table_name].row_formulas[col_name].clone();
        let result = self
            .evaluate_rowwise_formula_ast(table_name, &formula)
            .map_err(|e| locate_error(e, &format!("{table_name}.{col_name}"), &formula))?;
        if let Some(ctx) = self.ctx.as_mut() {
            ctx.tables
                .entry(table_name.to_string())
                .or_default()
                .insert(col_name.to_string(), column_values(&result).into());
        }
        if let Some(stored) = self.model.tables.get_mut(table_name) {
            stored.add_column(Column::new(col_name.to_string(), result));
        }
        Ok(())
    }

    /// Calculate one scalar formula and store its value
    fn calculate_scalar_cell(&mut self, scalar_name: &str) -> ForgeResult<()> {
        let Some(formula) = self
            .model
            .scalars
            .get(scalar_name)
            .and_then(|v| v.formula.clone())
        else {
            return Ok(());
        };
        let (value, is_date) = self
            .evaluate_scalar_formula_ast(scalar_name, &formula)
            .map_err(|e| locate_error(e, scalar_name, &formula))?;
        if let Some(var) = self.model.scalars.get_mut(scalar_name) {
            var.value = Some(value);
            var.is_date = is_date;
            if let (Some(ctx), Some(held)) = (self.ctx.as_mut(), scalar_value(var)) {
                ctx.scalars.insert(scalar_name.to_string(), held);
            }
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AST-BASED FORMULA EVALUATION (v5.2.0 - Parser Architecture Refactor)
    // ═══════════════════════════════════════════════════════════════════════════

    /// The values every formula can read by their full name: scalars,
    /// values of included files (`@namespace.path`), tables and scenarios.
    fn base_context(&self) -> evaluator::EvalContext {
        let mut ctx = evaluator::EvalContext::new();
        for (name, scalar) in &self.model.scalars {
            if let Some(value) = scalar_value(scalar) {
                ctx.scalars.insert(name.clone(), value);
            }
        }
        for (name, value) in &self.included {
            ctx.scalars.insert(name.clone(), value.clone());
        }
        for table in &self.included_tables {
            ctx.tables.insert(
                table.name.clone(),
                table
                    .columns
                    .iter()
                    .map(|(name, values)| (name.clone(), values.clone().into()))
                    .collect(),
            );
        }
        for (table_name, tbl) in &self.model.tables {
            let columns = tbl
                .columns
                .iter()
                .map(|(col_name, col)| (col_name.clone(), column_values(&col.values).into()))
                .collect();
            ctx.tables.insert(table_name.clone(), columns);
        }
        for (scenario_name, scenario) in &self.model.scenarios {
            ctx.scenarios.insert(
                scenario_name.clone(),
                scenario.overrides.clone().into_iter().collect(),
            );
        }
        ctx
    }

    /// The value of scalar `full`, read from the shared context, which mirrors
    /// the model's scalar values.
    fn model_value(&self, ctx: &evaluator::EvalContext, full: &str) -> Option<evaluator::Value> {
        debug_assert_eq!(
            ctx.scalars.get(full).and_then(evaluator::Value::as_number),
            self.model.scalars.get(full).and_then(|s| s.value),
            "context and model disagree on {full}"
        );
        ctx.scalars.get(full).cloned()
    }

    /// Evaluate with the context a formula of `table` (row formulas) or of
    /// section `section` (scalar formulas) sees: every value by its full name,
    /// then the short names (`price` for `summary.price`) that resolve to
    /// exactly one scalar from that section, then the columns of `table` by
    /// their own names. The columns, and the few short names that hide a name
    /// of their own, are laid over the shared context for this formula only
    /// and removed afterwards; the other short names are read through
    /// `aliases`, so a formula does not pay for every name in the model.
    fn evaluate_in_context<R>(
        &mut self,
        table: Option<&str>,
        section: Option<&str>,
        evaluate: impl FnOnce(&mut evaluator::EvalContext) -> R,
    ) -> R {
        let mut ctx = self.ctx.take().unwrap_or_else(|| self.base_context());
        // Each name laid over the shared context, with what it held before
        let mut saved: Vec<(String, Option<evaluator::Value>)> = Vec::new();
        let mut laid: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut lay = |ctx: &mut evaluator::EvalContext, name: String, value: evaluator::Value| {
            let previous = ctx.scalars.insert(name.clone(), value);
            if laid.insert(name.clone()) {
                saved.push((name, previous));
            }
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
        let previous_aliases = ctx.aliases.replace(Rc::clone(&short_names.aliases));
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

        let result = evaluate(&mut ctx);

        ctx.aliases = previous_aliases;
        for (name, previous) in saved.into_iter().rev() {
            match previous {
                Some(value) => ctx.scalars.insert(name, value),
                None => ctx.scalars.remove(&name),
            };
        }
        self.ctx = Some(ctx);
        result
    }

    /// What the current step of the order worked out in an earlier
    /// calculation (`get`), or else `compute` it and keep it (`set`).
    fn step_cached<T: Clone>(
        &self,
        get: impl Fn(&plan::Step) -> Option<T>,
        set: impl FnOnce(&mut plan::Step, T),
        compute: impl FnOnce() -> ForgeResult<T>,
    ) -> ForgeResult<T> {
        let Some(index) = self.step else {
            return compute();
        };
        let fingerprint = self.fingerprint;
        if let Some(found) = self
            .plan
            .structure(fingerprint, |s| s.steps.get(index).and_then(&get))
        {
            return Ok(found);
        }
        let value = compute()?;
        self.plan.structure(fingerprint, |s| {
            if let Some(step) = s.steps.get_mut(index) {
                set(step, value.clone());
            }
        });
        Ok(value)
    }

    /// A formula's syntax tree, parsed once per plan.
    fn parsed(&self, formula: &str) -> ForgeResult<Rc<parser::Expr>> {
        self.step_cached(
            |step| step.ast.clone(),
            |step, ast| step.ast = Some(ast),
            || self.parsed_by_text(formula),
        )
    }

    fn parsed_by_text(&self, formula: &str) -> ForgeResult<Rc<parser::Expr>> {
        let body = formula.trim_start_matches('=').trim();
        self.plan
            .ast(body, || {
                let tokens = tokenizer::tokenize(body)
                    .map_err(|e| format!("syntax error: {}", e.message))?;
                parser::parse(tokens).map_err(|e| format!("syntax error: {}", e.message))
            })
            .map_err(ForgeError::Eval)
    }

    /// Evaluate a row-wise formula using the AST evaluator
    fn evaluate_rowwise_formula_ast(
        &mut self,
        table_name: &str,
        formula: &str,
    ) -> ForgeResult<ColumnValue> {
        let table = &self.model.tables[table_name];
        let ast = self.parsed(formula)?;

        self.check_ambiguous_names(formula, None, table)?;
        let row_count = table.row_count();
        if row_count == 0 {
            return Err(ForgeError::Eval(
                "Cannot evaluate on empty table".to_string(),
            ));
        }
        self.evaluate_in_context(Some(table_name), None, |row_ctx| {
            Self::evaluate_rows(&ast, row_ctx, row_count)
        })
        .map_err(|e| self.explain_included_mismatch(e, table_name))
    }

    /// Evaluate a row formula in each of `row_count` rows. Only the row of
    /// the context changes (evaluation reads the context, never changes it).
    fn evaluate_rows(
        ast: &parser::Expr,
        row_ctx: &mut evaluator::EvalContext,
        row_count: usize,
    ) -> ForgeResult<ColumnValue> {
        row_ctx.current_row = Some(0);
        row_ctx.row_count = Some(row_count);
        let first_result = evaluator::evaluate(ast, row_ctx)
            .map_err(|e| ForgeError::Eval(format!("row 1: {}", e.message)))?;

        // Determine column type from first result and evaluate all rows
        match &first_result {
            evaluator::Value::Text(_) => {
                let mut results: Vec<String> = Vec::with_capacity(row_count);
                results.push(first_result.as_text());
                for row_idx in 1..row_count {
                    row_ctx.current_row = Some(row_idx);
                    let result = evaluator::evaluate(ast, row_ctx).map_err(|e| {
                        ForgeError::Eval(format!("row {}: {}", row_idx + 1, e.message))
                    })?;
                    results.push(result.as_text());
                }
                Ok(ColumnValue::Text(results))
            },
            evaluator::Value::Boolean(_) => {
                let mut results: Vec<bool> = Vec::with_capacity(row_count);
                results.push(first_result.as_bool().unwrap_or(false));
                for row_idx in 1..row_count {
                    row_ctx.current_row = Some(row_idx);
                    let result = evaluator::evaluate(ast, row_ctx).map_err(|e| {
                        ForgeError::Eval(format!("row {}: {}", row_idx + 1, e.message))
                    })?;
                    results.push(result.as_bool().unwrap_or(false));
                }
                Ok(ColumnValue::Boolean(results))
            },
            _ => {
                // Default to numeric
                let mut results: Vec<f64> = Vec::with_capacity(row_count);
                // A formula never gives an empty cell: it gives 0 (ADR-059)
                let first_num = first_result
                    .as_cell_number()
                    .ok_or_else(|| not_a_number(1, &first_result))?;
                results.push(first_num);
                for row_idx in 1..row_count {
                    row_ctx.current_row = Some(row_idx);
                    let result = evaluator::evaluate(ast, row_ctx).map_err(|e| {
                        ForgeError::Eval(format!("row {}: {}", row_idx + 1, e.message))
                    })?;
                    let value = result
                        .as_cell_number()
                        .ok_or_else(|| not_a_number(row_idx + 1, &result))?;
                    results.push(value);
                }
                Ok(ColumnValue::Number(results))
            },
        }
    }

    /// Evaluate the formula of scalar `scalar_name` using the AST evaluator:
    /// its value, and whether that is a date (ADR-060)
    fn evaluate_scalar_formula_ast(
        &mut self,
        scalar_name: &str,
        formula: &str,
    ) -> ForgeResult<(f64, bool)> {
        let ast = self.parsed(formula)?;

        let empty_table = Table::new("_scalar_context".to_string());
        let section = scalar_name.rsplit_once('.').map(|(section, _)| section);
        self.check_ambiguous_names(formula, section, &empty_table)?;
        let result = self
            .evaluate_in_context(None, section, |ctx| evaluator::evaluate(&ast, ctx))
            .map_err(|e| ForgeError::Eval(e.message))?;
        if let evaluator::Value::Array(items) = &result {
            if items.len() != 1 {
                return Err(ForgeError::Eval(format!(
                    "Scalar formula '{formula}' returns {} values. \
                     Aggregate it with a function such as SUM or INDEX.",
                    items.len()
                )));
            }
        }
        // A date, as a column of dates holds one (ADR-060)
        let single = match &result {
            evaluator::Value::Array(items) => items.first().unwrap_or(&result),
            other => other,
        };
        if let evaluator::Value::Text(text) = single {
            if let Some(serial) = evaluator::excel_date_serial(text) {
                return Ok((serial, true));
            }
        }
        // A formula never gives an empty cell: it gives 0 (ADR-059)
        result
            .as_cell_number()
            .map(|n| (n, false))
            .ok_or_else(|| ForgeError::Eval("result is not a number".to_string()))
    }
}

/// A column's values as the evaluator reads them (dates as text).
/// A formula's `format:` must be a date code where its value is a date and a
/// number code where it is a number (ADR-061); its value is known only once
/// calculated.
fn check_formula_formats(model: &ParsedModel) -> ForgeResult<()> {
    use evaluator::text::number_format::{check_format, FormatKind};
    let mismatch = |path: &str, format: &str, is_date: bool| -> ForgeResult<()> {
        let kind = check_format(format).map_err(ForgeError::Eval)?;
        let (wrong, value) = match (kind, is_date) {
            (FormatKind::Date, false) => ("a date format", "a number"),
            (FormatKind::Number, true) => ("a number format", "a date"),
            _ => return Ok(()),
        };
        Err(ForgeError::Eval(format!(
            "'{path}.format': \"{format}\" is {wrong}, and the value of '{path}' is {value}"
        )))
    };
    for (path, var) in &model.scalars {
        if let (Some(format), Some(_), Some(_)) = (&var.metadata.format, &var.formula, var.value) {
            mismatch(path, format, var.is_date)?;
        }
    }
    for (name, table) in &model.tables {
        for (column, metadata) in &table.formula_metadata {
            let (Some(format), Some(col)) = (&metadata.format, table.columns.get(column)) else {
                continue;
            };
            let is_date = match &col.values {
                ColumnValue::Date(_) => true,
                ColumnValue::Text(v) => {
                    !v.is_empty() && v.iter().all(|t| evaluator::excel_date_serial(t).is_some())
                },
                _ => false,
            };
            mismatch(&format!("{name}.{column}"), format, is_date)?;
        }
    }
    Ok(())
}

/// A scalar's value as a formula reads it: a date as its text, as a cell of
/// a date column is (ADR-060), a number as itself.
fn scalar_value(scalar: &crate::types::Variable) -> Option<evaluator::Value> {
    scalar.date_text().map_or_else(
        || scalar.value.map(evaluator::Value::Number),
        |date| Some(evaluator::Value::Text(date)),
    )
}

fn column_values(values: &ColumnValue) -> Vec<evaluator::Value> {
    match values {
        ColumnValue::Number(nums) => nums.iter().map(|n| evaluator::Value::Number(*n)).collect(),
        // An empty cell is Null to the evaluator (ADR-059)
        ColumnValue::Blanks(nums) => nums
            .iter()
            .map(|n| n.map_or(evaluator::Value::Null, evaluator::Value::Number))
            .collect(),
        ColumnValue::Text(texts) => texts
            .iter()
            .map(|s| evaluator::Value::Text(s.clone()))
            .collect(),
        ColumnValue::Boolean(bools) => bools
            .iter()
            .map(|b| evaluator::Value::Boolean(*b))
            .collect(),
        ColumnValue::Date(dates) => dates
            .iter()
            .map(|d| evaluator::Value::Text(d.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod aggregation_classifier_tests {
    use super::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table};

    fn calculate_column(formula: &str) -> Result<ColumnValue, String> {
        let mut model = ParsedModel::new();
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![3.0, -2.0, 5.0]),
        ));
        table.add_row_formula("result".to_string(), formula.to_string());
        model.add_table(table);
        let calculated = ArrayCalculator::new(model)
            .calculate_all()
            .map_err(|e| e.to_string())?;
        Ok(calculated.tables["t"].columns["result"].values.clone())
    }

    #[test]
    fn test_max_with_several_arguments_is_row_wise() {
        assert_eq!(
            calculate_column("=MAX(0, x)").unwrap(),
            ColumnValue::Number(vec![3.0, 0.0, 5.0])
        );
        assert_eq!(
            calculate_column("=MIN(x, 4)").unwrap(),
            ColumnValue::Number(vec![3.0, -2.0, 4.0])
        );
    }

    #[test]
    fn test_single_argument_aggregate_in_table_reads_the_whole_column() {
        // x is 3, -2, 5 (ADR-058): by hand, the sum is 6 and the positives 8
        for (formula, expected) in [
            ("=SUM(x)", vec![6.0, 6.0, 6.0]),
            ("=MAX(x)", vec![5.0, 5.0, 5.0]),
            ("=x / SUM(x)", vec![0.5, -2.0 / 6.0, 5.0 / 6.0]),
            ("=SUMIF(x, \">0\", x)", vec![8.0, 8.0, 8.0]),
            ("=COUNTIF(x, \">\" & x) + 1", vec![2.0, 3.0, 1.0]),
        ] {
            assert_eq!(
                calculate_column(formula).unwrap(),
                ColumnValue::Number(expected),
                "{formula}"
            );
        }
    }

    #[test]
    fn test_a_column_that_aggregates_itself_is_circular() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
        table.add_row_formula("total".to_string(), "=SUM(total)".to_string());
        model.add_table(table);
        let err = ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert!(err.contains("t.total refers to itself"), "{err}");
    }

    #[test]
    fn test_several_arguments_of_another_length_say_how_to_aggregate() {
        let mut model = ParsedModel::new();
        let mut other = Table::new("o".to_string());
        other.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        model.add_table(other);
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_row_formula("m".to_string(), "=MAX(0, o.v)".to_string());
        model.add_table(table);
        let err = ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert!(err.contains("Row count mismatch"), "{err}");
        assert!(err.contains("SUM(o.v) with one argument"), "{err}");
    }

    #[test]
    fn test_function_name_inside_text_is_not_an_aggregate() {
        assert!(calculate_column("=IF(x > 0, \"SUM(\", \"no\")").is_ok());
    }
}

#[cfg(test)]
mod nan_inputs {
    //! A NaN in the data (`.nan` in YAML) must give an answer or an error, never a panic.

    use super::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_nan_in_a_column_never_panics() {
        let functions = [
            "MEDIAN",
            "MIN",
            "MAX",
            "SMALL",
            "LARGE",
            "PERCENTILE",
            "QUARTILE",
            "RANK",
            "SORT",
            "MODE",
            "STDEV",
            "VAR",
        ];
        for name in functions {
            let mut model = ParsedModel::new();
            let mut table = Table::new("t".to_string());
            table.add_column(Column::new(
                "x".to_string(),
                ColumnValue::Number(vec![3.0, f64::NAN, 2.0, 1.0]),
            ));
            model.add_table(table);
            let formula = match name {
                "SMALL" | "LARGE" | "PERCENTILE" | "QUARTILE" => format!("={name}(t.x, 1)"),
                "RANK" => "=RANK(2, t.x)".to_string(),
                "SORT" => "=SUM(SORT(t.x))".to_string(),
                _ => format!("={name}(t.x)"),
            };
            model.add_scalar(
                "out.v".to_string(),
                Variable::new("out.v".to_string(), None, Some(formula)),
            );
            // Ok or Err are both fine; a panic fails the test.
            let _ = ArrayCalculator::new(model).calculate_all();
        }
    }
}

#[cfg(test)]
mod tests {

    mod one_scalar {
        //! `calculate_scalar` answers for one scalar: a failed formula that the
        //! scalar does not use does not fail it, as it fails `calculate_all`.

        use super::super::ArrayCalculator;
        use crate::parser::parse_single_document_yaml;
        use std::path::Path;

        fn calculator(text: &str) -> ArrayCalculator {
            let model = parse_single_document_yaml(
                &format!("_forge_version: \"5.4.0\"\n{text}"),
                Path::new("m.yaml"),
            )
            .unwrap();
            ArrayCalculator::new(model)
        }

        const MODEL: &str = "s:\n  x: 4\n  y: {formula: \"=x * 2\"}\n  bad: {formula: \"=1 / 0\"}\n  uses_bad: {formula: \"=bad + y\"}\n";

        #[test]
        fn a_failure_the_scalar_does_not_use_is_left_out() {
            let y = calculator(MODEL).calculate_scalar("s.y").unwrap();
            assert!((y - 8.0).abs() <= 0.0, "{y}");
            // calculate_all still fails on it
            let err = calculator(MODEL).calculate_all().unwrap_err().to_string();
            assert!(err.contains("s.bad"), "{err}");
        }

        #[test]
        fn a_failure_the_scalar_uses_fails_it_with_the_report() {
            for name in ["s.bad", "s.uses_bad"] {
                let err = calculator(MODEL)
                    .calculate_scalar(name)
                    .unwrap_err()
                    .to_string();
                assert!(err.contains("s.bad: "), "{name}: {err}");
            }
            let err = calculator(MODEL)
                .calculate_scalar("s.uses_bad")
                .unwrap_err()
                .to_string();
            assert!(err.contains("s.uses_bad (uses s.bad)"), "{err}");
        }

        #[test]
        fn a_scalar_that_is_not_there_or_has_no_value_is_an_error() {
            let err = calculator(MODEL).calculate_scalar("s.nothing").unwrap_err();
            assert!(err.to_string().contains("'s.nothing' not found"), "{err}");
            let err = calculator("s:\n  empty: {value: null}\n")
                .calculate_scalar("s.empty")
                .unwrap_err();
            assert!(err.to_string().contains("'s.empty' has no value"), "{err}");
        }

        #[test]
        fn a_model_that_cannot_be_calculated_is_an_error() {
            let err = calculator("s:\n  a: {formula: \"=b\"}\n  b: {formula: \"=a\"}\n")
                .calculate_scalar("s.a")
                .unwrap_err();
            assert!(err.to_string().contains("depend on each other"), "{err}");
            // With every formula calculated, the formats are checked as by
            // calculate_all
            let err = calculator("s:\n  d: {formula: \"=1 + 1\", format: \"yyyy-mm-dd\"}\n")
                .calculate_scalar("s.d")
                .unwrap_err();
            assert!(err.to_string().contains("is a date format"), "{err}");
        }
    }

    mod column_copies {
        //! A row formula that reads a whole column copied it for every row, so the
        //! time grew with the square of the rows for the copies alone: 16,000 rows of
        //! `=INDEX(d.v, MATCH(k, d.k, 0))` took 5.2 s, and `SUMIF` 6.3 s (release).
        //! A column is now shared, not copied. Roadmap T1-COLUMN-COPIES.
        //!
        //! Each row still searches the column, as in Excel, so the time still grows
        //! with the square of the rows; the copies were most of it.

        use crate::cli::calculate_core;
        use crate::core::precision;
        use std::fmt::Write as _;
        use std::time::{Duration, Instant};

        const ROWS: usize = 16_000;

        /// A table `d` of keys and values, and a table `q` whose row formula reads
        /// both columns of `d` whole in every row.
        fn model(formula: &str) -> String {
            let list = |f: &dyn Fn(usize) -> usize| {
                (0..ROWS)
                    .map(|i| f(i).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let mut text = String::from("_forge_version: \"5.1.0\"\nd:\n");
            writeln!(text, "  k: [{}]", list(&|i| i)).unwrap();
            writeln!(text, "  v: [{}]", list(&|i| i * 2)).unwrap();
            // Keys in a scattered order, every one of d.k once
            writeln!(text, "q:\n  k: [{}]", list(&|i| (i * 7919) % ROWS)).unwrap();
            writeln!(text, "  r: \"{formula}\"").unwrap();
            text
        }

        /// Under 1 s in a release build. A debug build is too slow to tell the copies
        /// from the search (2 s here against 13 s before, and CI runners vary), so it
        /// checks only the results; the Validation workflow runs this in release.
        fn in_time(took: Duration, limit: Duration, what: &str) {
            if !cfg!(debug_assertions) {
                assert!(took < limit, "16,000 rows of {what} took {took:?}");
            }
        }

        /// The time, and the results of `q.r`.
        fn run(formula: &str) -> (Duration, Vec<f64>) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("lookup.yaml");
            std::fs::write(&path, model(formula)).unwrap();
            let start = Instant::now();
            let result = calculate_core(&path, true, None).unwrap();
            let took = start.elapsed();
            let values = result.tables["q"].values["r"]
                .iter()
                .map(|v| serde_json::to_value(v).unwrap().as_f64().unwrap())
                .collect();
            (took, values)
        }

        /// Row i of q looks up key (i * 7919) % ROWS, whose value is twice the key.
        fn expected() -> Vec<f64> {
            (0..ROWS)
                .map(|i| precision::count((i * 7919) % ROWS * 2))
                .collect()
        }

        #[test]
        fn sixteen_thousand_rows_of_index_match_calculate_in_time() {
            let (took, values) = run("=INDEX(d.v, MATCH(k, d.k, 0))");
            in_time(took, Duration::from_secs(1), "INDEX/MATCH");
            assert_eq!(values, expected());
        }

        #[test]
        fn sixteen_thousand_rows_of_sumif_calculate_in_time() {
            // SUMIF reads its ranges another way (collect_values_as_vec)
            let (took, values) = run("=SUMIF(d.k, k, d.v)");
            in_time(took, Duration::from_secs(2), "SUMIF");
            assert_eq!(values, expected());
        }
    }

    mod formula_depth {
        //! A formula nested too deep is refused with a message, not calculated until
        //! the stack overflows (roadmap T1-DEPTH). Before the limit, a formula of
        //! 10,000 parentheses, or of 100,000 terms added, ended `forge calculate`
        //! with a stack overflow (exit 134), and a `forge mcp` server died on the
        //! same formula in a call, taking its session with it.

        use crate::cli::calculate_core;
        use crate::core::array_calculator::parser::{parse, MAX_DEPTH};
        use crate::core::array_calculator::tokenizer::tokenize;
        use crate::core::precision;

        fn model(formula: &str) -> String {
            format!("_forge_version: \"5.1.0\"\nm:\n  x:\n    formula: \"={formula}\"\n")
        }

        fn accepted(formula: &str) -> bool {
            parse(tokenize(formula).unwrap()).is_ok()
        }

        /// What `m.x` calculates to, in a file of its own
        fn calculated(formula: &str) -> Result<f64, String> {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, model(formula)).unwrap();
            calculate_core(&path, true, None)
                .map(|result| result.scalars["m.x"].unwrap())
                .map_err(|e| e.to_string())
        }

        /// The longest chain `1+1+...` inside parentheses that is accepted as the
        /// first operand of a longer one, and the longest longer one: the deepest
        /// tree the limit lets through, nearly twice `MAX_DEPTH`.
        fn deepest() -> (String, f64) {
            let chain = |n: usize| vec!["1"; n].join("+");
            let inner = (1..=MAX_DEPTH)
                .take_while(|&n| accepted(&format!("({})+1", chain(n))))
                .last()
                .unwrap();
            let outer = (1..=MAX_DEPTH)
                .take_while(|&n| accepted(&format!("({}){}", chain(inner), "+1".repeat(n))))
                .last()
                .unwrap();
            (
                format!("({}){}", chain(inner), "+1".repeat(outer)),
                precision::count(inner + outer),
            )
        }

        #[test]
        fn a_formula_at_the_limit_calculates() {
            let pairs = MAX_DEPTH - 1;
            let parentheses = format!("{}7{}", "(".repeat(pairs), ")".repeat(pairs));
            assert_eq!(calculated(&parentheses), Ok(7.0));
            // The sign inside is one more level, so one call fewer
            let calls = format!("{}-3{}", "ABS(".repeat(pairs - 1), ")".repeat(pairs - 1));
            assert_eq!(calculated(&calls), Ok(3.0));
            let signs = format!("{}5", "-".repeat(pairs));
            assert_eq!(calculated(&signs), Ok(-5.0));
            let sum = vec!["1"; MAX_DEPTH].join("+");
            assert_eq!(calculated(&sum), Ok(precision::count(MAX_DEPTH)));
        }

        #[test]
        fn the_deepest_tree_the_limit_lets_through_calculates() {
            let (formula, value) = deepest();
            assert!(
                value > precision::count(MAX_DEPTH),
                "{value}: not the worst case"
            );
            assert_eq!(calculated(&formula), Ok(value));
        }

        #[test]
        fn a_formula_one_level_past_the_limit_is_refused_with_the_cell() {
            let pairs = MAX_DEPTH;
            let formula = format!("{}7{}", "(".repeat(pairs), ")".repeat(pairs));
            let message = calculated(&formula).unwrap_err();
            assert!(message.contains("m.x"), "{message}");
            assert!(
                message.contains(&format!("nested more than {MAX_DEPTH} levels")),
                "{message}"
            );
            let sum = vec!["1"; MAX_DEPTH + 1].join("+");
            assert!(calculated(&sum).is_err());
        }

        #[test]
        fn a_hundred_thousand_levels_are_refused() {
            let n = 100_000;
            for formula in [
                format!("{}1{}", "(".repeat(n), ")".repeat(n)),
                format!("{}1{}", "ABS(".repeat(n), ")".repeat(n)),
                format!("{}1", "-".repeat(n)),
                vec!["2"; n].join("^"),
                vec!["1"; n].join("+"),
                vec!["1"; n].join("&"),
                vec!["1"; n].join("="),
                format!("LAMBDA(q, q){}", "(1)".repeat(n)),
            ] {
                let message = calculated(&formula).unwrap_err();
                assert!(message.contains("nested more than"), "{message}");
            }
        }

        /// The command line and the MCP server, which ran out of stack before
        #[cfg(feature = "cli")]
        mod command_line_and_mcp {
            use super::{deepest, model, MAX_DEPTH};
            use crate::cli::forge;
            use crate::mcp::Session;

            const N: usize = 100_000;

            fn deep() -> String {
                format!("{}1{}", "(".repeat(N), ")".repeat(N))
            }

            #[test]
            fn forge_calculate_exits_with_an_error_not_a_stack_overflow() {
                let dir = tempfile::tempdir().unwrap();
                for (name, formula) in [
                    ("parentheses", deep()),
                    ("sum", vec!["1"; N].join("+")),
                    ("signs", format!("{}1", "-".repeat(N))),
                ] {
                    let path = dir.path().join(format!("{name}.yaml"));
                    std::fs::write(&path, model(&formula)).unwrap();
                    let ran = forge(&["calculate", "--dry-run", path.to_str().unwrap()]);
                    assert_eq!(ran.code, 1, "{name}: {}", ran.err);
                    assert!(ran.err.contains("m.x"), "{name}: {}", ran.err);
                    assert!(
                        ran.err
                            .contains(&format!("nested more than {MAX_DEPTH} levels")),
                        "{name}: {}",
                        ran.err
                    );
                }
            }

            /// The answers of the MCP server to `forge_calculate` on each model, in
            /// order, with the server still serving after the last
            fn answers(models: &[String]) -> Vec<String> {
                let mut session = Session::start(None);
                let answers = (1..)
                    .zip(models)
                    .map(|(id, content)| {
                        session.call(
                            id,
                            "forge_calculate",
                            &serde_json::json!({"content": content, "dry_run": true}),
                        )
                    })
                    .collect();
                assert!(session.is_serving(), "the server is alive");
                answers
            }

            #[test]
            fn forge_mcp_answers_a_deep_formula_with_an_error_and_the_next_call() {
                let fine = model("1 + 2");
                let (worst, value) = deepest();
                let answers = answers(&[
                    model(&deep()),
                    fine,
                    model(&vec!["1"; N].join("+")),
                    model(&worst),
                ]);
                assert!(answers[0].contains("nested more than"), "{}", answers[0]);
                assert!(answers[1].contains("\\\"m.x\\\":3.0"), "{}", answers[1]);
                assert!(answers[2].contains("nested more than"), "{}", answers[2]);
                let expected = format!("\\\"m.x\\\":{value:.1}");
                assert!(answers[3].contains(&expected), "{}", answers[3]);
            }
        }
    }
}
