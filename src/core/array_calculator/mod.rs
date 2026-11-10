#[cfg(test)]
mod calculation_order_tests;
mod dates;
#[cfg(test)]
mod error_location_tests;
pub mod evaluator;
#[cfg(test)]
mod include_reference_tests;
#[cfg(test)]
mod nan_input_tests;
pub mod parser;
mod plan;
mod recursive;
#[cfg(test)]
mod recursive_tests;
#[cfg(test)]
mod scalar_scope_tests;
pub mod tokenizer;

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};
use std::collections::HashMap;
use std::rc::Rc;

pub use plan::CalcPlan;

/// What a name used in a formula refers to
enum ScalarName {
    /// Exactly one scalar, by full name
    Found(String),
    /// Several scalars in different sections, sorted by full name
    Ambiguous(Vec<String>),
    /// No scalar
    Missing,
}

/// A cell with a formula: a whole table column (row formula) or a scalar.
/// Ordered by kind, then name, so iteration is deterministic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Cell {
    Column {
        table: String,
        column: String,
    },
    Scalar(String),
    /// Columns of one table that read each other through `PREV`, calculated
    /// together row by row, in this order within a row (`recursive`)
    Columns {
        table: String,
        columns: Vec<String>,
    },
}

impl Cell {
    fn label(&self) -> String {
        match self {
            Self::Column { table, column } => format!("{table}.{column}"),
            Self::Scalar(name) => name.clone(),
            Self::Columns { table, columns } => format!("{table}.{}", columns.join(", ")),
        }
    }

    /// The single cells this step calculates: itself, or each column of a
    /// group. Other cells name these as what they depend on.
    fn members(&self) -> Vec<Self> {
        match self {
            Self::Columns { table, columns } => columns
                .iter()
                .map(|column| Self::Column {
                    table: table.clone(),
                    column: column.clone(),
                })
                .collect(),
            other => vec![other.clone()],
        }
    }
}

/// A character that can be part of a name in a formula. `@` starts a value
/// from an included file (`@rates.fx.usd_eur`); keeping it in the word means
/// such a name never matches a cell of this model, even when a group here has
/// the same name as the include.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '@'
}

/// The names a formula mentions (`revenue`, `assumptions.rate`), without the
/// contents of string literals. Function names and numbers are included; they
/// match no cell, so they add no dependency.
fn formula_words(formula: &str) -> Vec<String> {
    let stripped = strip_string_literals(formula);
    let mut words: Vec<String> = Vec::new();
    for word in stripped.split(|c: char| !is_name_char(c)) {
        if !word.is_empty() && !words.iter().any(|w| w == word) {
            words.push(word.to_string());
        }
    }
    words
}

/// Name the column or scalar whose formula failed, and show the formula, so an
/// error points at the place to fix (`calc.x: Division by zero`). Only
/// evaluation errors are rewritten; other errors already say where they are.
fn locate_error(err: ForgeError, location: &str, formula: &str) -> ForgeError {
    match err {
        ForgeError::Eval(msg) => {
            // Row errors read "row 3: ..."; keep them on the location's line
            let sep = if msg.starts_with("row ") { " " } else { ": " };
            ForgeError::Eval(format!("{location}{sep}{msg} (formula: {formula})"))
        },
        other => other,
    }
}

/// The error for a calculation in which cells failed. One failure keeps its
/// own message; several are listed, followed by the cells that were not
/// calculated because they use a failed one.
fn failure_report(failures: &[String], skipped: &[(Cell, Cell)]) -> ForgeError {
    use std::fmt::Write as _;

    if failures.len() == 1 && skipped.is_empty() {
        return ForgeError::Eval(failures[0].clone());
    }
    let mut msg = if failures.len() == 1 {
        "1 formula failed:".to_string()
    } else {
        format!("{} formulas failed:", failures.len())
    };
    for failure in failures {
        msg.push_str("\n  - ");
        msg.push_str(failure);
    }
    if !skipped.is_empty() {
        msg.push_str("\n  Not calculated, because they use a failed formula:");
        for (cell, dep) in skipped {
            let _ = write!(msg, "\n  - {} (uses {})", cell.label(), dep.label());
        }
    }
    ForgeError::Eval(msg)
}

/// Say which cells form a dependency cycle, for example
/// "scalars calc.a, calc.b depend on each other".
fn describe_cycle(graph: &petgraph::graph::DiGraph<Cell, ()>) -> String {
    let mut cells: Vec<&Cell> = petgraph::algo::tarjan_scc(graph)
        .into_iter()
        .filter(|component| component.len() > 1 || graph.contains_edge(component[0], component[0]))
        .flatten()
        .filter_map(|idx| graph.node_weight(idx))
        .collect();
    cells.sort();
    cells.dedup();
    let kind = if cells.iter().all(|c| matches!(c, Cell::Scalar(_))) {
        "scalars"
    } else if cells.iter().all(|c| matches!(c, Cell::Column { .. })) {
        "columns"
    } else {
        "formulas"
    };
    let names: Vec<String> = cells.iter().map(|c| c.label()).collect();
    match names.as_slice() {
        [] => "formulas depend on each other".to_string(),
        [one] if matches!(cells[0], Cell::Column { .. }) => format!(
            "{one} refers to itself (to read the row above, write PREV(column, first_row_value))"
        ),
        [one] => format!("{one} refers to itself"),
        _ => format!("{kind} {} depend on each other", names.join(", ")),
    }
}

/// Strip string literals from a formula before extracting references.
/// This prevents content inside quotes from being parsed as column references.
/// e.g., =LEN("Hello") should not treat "Hello" as a column reference.
fn strip_string_literals(formula: &str) -> String {
    let mut result = String::with_capacity(formula.len());
    let mut in_string = false;
    let mut quote_char = '"';

    for c in formula.chars() {
        if !in_string && (c == '"' || c == '\'') {
            in_string = true;
            quote_char = c;
        } else if in_string && c == quote_char {
            in_string = false;
        } else if !in_string {
            result.push(c);
        }
        // Characters inside strings are skipped
    }

    result
}

/// What the included files give a formula: their scalars as
/// `@namespace.path` with their values, and their tables.
type Included = (Vec<(String, f64)>, Vec<IncludedTable>);

/// A table of an included file, calculated.
#[derive(Debug, Clone)]
struct IncludedTable {
    /// `@namespace.table`
    name: String,
    /// The file, as its `_includes` entry names it
    file: String,
    /// Each column's values
    columns: Vec<(String, Vec<evaluator::Value>)>,
}

/// Array-aware calculator for v1.0.0 models
/// Handles both row-wise (element-wise) and aggregation formulas
pub struct ArrayCalculator {
    model: ParsedModel,
    /// Scalars of included files as `@namespace.path`, calculated before the
    /// model (see [`ArrayCalculator::included_values`]).
    included: Vec<(String, f64)>,
    /// Tables of included files as `@namespace.table`, with their calculated
    /// columns and the file they come from
    included_tables: Vec<IncludedTable>,
    /// Scalar names by their last part (`price` -> [`a.price`, `b.price`]),
    /// sorted. Calculation sets values, never names, so it is built once.
    by_last_part: std::cell::OnceCell<HashMap<String, Vec<String>>>,
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
            ctx: None,
            plan,
            fingerprint: 0,
            step: None,
        }
    }

    /// The scalars of every included file, as `@namespace.path` with their
    /// values, and its tables as `@namespace.table`. Each included model is
    /// calculated first, with its own includes, so a value or column an
    /// included file defines by formula is available. Namespaces are taken in
    /// name order, so an error is the same on every run.
    fn included_values(&self) -> ForgeResult<Included> {
        let mut namespaces: Vec<&String> = self.model.resolved_includes.keys().collect();
        namespaces.sort();
        let mut values = Vec::new();
        let mut tables = Vec::new();
        for namespace in namespaces {
            let resolved = &self.model.resolved_includes[namespace];
            let calculated = Self::with_plan(resolved.model.clone(), self.plan.include(namespace))
                .calculate_all()
                .map_err(|e| {
                    let reason = match e {
                        ForgeError::Eval(msg) => msg,
                        other => other.to_string(),
                    };
                    ForgeError::Eval(format!(
                        "included file '{}' (as {namespace}): {reason}",
                        resolved.include.file
                    ))
                })?;
            for (path, var) in &calculated.scalars {
                if let Some(value) = var.value {
                    values.push((format!("@{namespace}.{path}"), value));
                }
            }
            for (name, table) in &calculated.tables {
                tables.push(IncludedTable {
                    name: format!("@{namespace}.{name}"),
                    file: resolved.include.file.clone(),
                    columns: table
                        .columns
                        .iter()
                        .map(|(col, c)| (col.clone(), column_values(&c.values)))
                        .collect(),
                });
            }
        }
        Ok((values, tables))
    }

    /// The evaluator's row count mismatch for a column of an included table,
    /// said with both files: a row formula pairs the rows of the two tables
    /// one by one. Any other error is returned as it is.
    fn explain_included_mismatch(&self, err: ForgeError, table_name: &str) -> ForgeError {
        let ForgeError::Eval(msg) = &err else {
            return err;
        };
        let Some(at) = msg.find("Row count mismatch: @") else {
            return err;
        };
        let rest = &msg[at + "Row count mismatch: ".len()..];
        let parsed = rest.split_once(" has ").and_then(|(word, counts)| {
            let (rows, expected) = counts.split_once(" rows but expected ")?;
            let expected: String = expected.chars().take_while(char::is_ascii_digit).collect();
            Some((
                word,
                rows.parse::<usize>().ok()?,
                expected.parse::<usize>().ok()?,
            ))
        });
        let Some((word, rows, expected)) = parsed else {
            return err;
        };
        let file = word
            .strip_prefix('@')
            .and_then(|r| r.rsplit_once('.'))
            .and_then(|(table, _)| {
                let table = format!("@{table}");
                self.included_tables.iter().find(|t| t.name == table)
            })
            .map_or_else(|| "an included file".to_string(), |t| t.file.clone());
        let here = self
            .model
            .source
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or_else(
                || "this model".to_string(),
                |f| f.to_string_lossy().into_owned(),
            );
        ForgeError::Eval(format!(
            "{word} has {rows} rows (in {file}), but table '{table_name}' has {expected} \
             (in {here}); a row formula pairs their rows one by one, so they must have as \
             many. To read one row whatever the lengths, use INDEX({word}, n); for a total, \
             put SUM({word}) in a scalar and use that"
        ))
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
        self.fingerprint = plan::fingerprint(&self.model);
        (self.included, self.included_tables) = self.included_values()?;

        // A formula scalar's stored value (from the file, an import or an
        // earlier run) is never read: it is unknown until it is recalculated.
        // Callers that set a value on purpose (scenarios, sensitivity,
        // simulation) clear the formula when they do.
        for var in self.model.scalars.values_mut() {
            if var.formula.is_some() {
                var.value = None;
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
        //
        // A failing cell does not stop the calculation: every cell that does
        // not use it is still calculated, so all errors are reported at once.
        // A failed cell gets no value, so nothing can read a number from it,
        // and the model as a whole still fails.
        let mut failures: Vec<String> = Vec::new();
        let mut skipped: Vec<(Cell, Cell)> = Vec::new();
        let mut failed: std::collections::BTreeSet<Cell> = std::collections::BTreeSet::new();
        let order = self.planned_order()?;
        for (step, (cell, dependencies)) in order.iter().enumerate() {
            self.step = Some(step);
            let blocked_by = dependencies.iter().find(|dep| failed.contains(*dep));
            if let Some(dep) = blocked_by {
                skipped.push((cell.clone(), dep.clone()));
                failed.extend(cell.members());
                continue;
            }
            let outcome = match cell {
                Cell::Column { table, column } => self.calculate_column(table, column),
                Cell::Scalar(name) => self.calculate_scalar(name),
                Cell::Columns { table, columns } => self.calculate_columns(table, columns),
            };
            if let Err(e) = outcome {
                failures.push(match e {
                    ForgeError::Eval(msg) => msg,
                    other => other.to_string(),
                });
                failed.extend(cell.members());
            }
        }

        if failures.is_empty() {
            return Ok(self.model);
        }
        // Each failure starts with its cell's name; list both in name order
        failures.sort();
        skipped.sort_by_key(|(cell, _)| cell.label());
        Err(failure_report(&failures, &skipped))
    }

    /// [`ArrayCalculator::calculation_order`] with each cell's dependencies,
    /// from the plan when a model with the same formulas was ordered before.
    fn planned_order(&self) -> ForgeResult<plan::Order> {
        if let Some(order) = self.plan.structure(self.fingerprint, |s| s.order.clone()) {
            return Ok(order);
        }
        let order = self
            .calculation_order()?
            .into_iter()
            .map(|cell| {
                let deps = self.cell_dependencies(&cell)?;
                Ok((cell, deps))
            })
            .collect::<ForgeResult<Vec<_>>>()?;
        let order = Rc::new(order);
        self.plan.structure(self.fingerprint, |s| {
            s.steps = vec![plan::Step::default(); order.len()];
            s.order = Some(Rc::clone(&order));
        });
        Ok(order)
    }

    /// The formula cells that `cell`'s formula reads. Names that are data
    /// (an input column, a scalar without a formula) need no edge and are
    /// left out.
    fn cell_dependencies(&self, cell: &Cell) -> ForgeResult<Vec<Cell>> {
        let mut deps = Vec::new();
        match cell {
            Cell::Column { table, column } => {
                let own = &self.model.tables[table];
                let formula = &own.row_formulas[column];
                for word in formula_words(formula) {
                    if let Some(dep) = self.qualified_column(&word) {
                        deps.push(dep);
                    } else if !word.contains('.') && own.row_formulas.contains_key(&word) {
                        // A bare name is a column of this table first, as in
                        // evaluation (columns shadow scalar short names)
                        deps.push(Cell::Column {
                            table: table.clone(),
                            column: word.clone(),
                        });
                    } else if !word.contains('.') && own.columns.contains_key(&word) {
                        // An input column
                    } else if let ScalarName::Found(full) = self.resolve_scalar_name(&word, None) {
                        deps.push(Cell::Scalar(full));
                    }
                }
            },
            Cell::Columns { .. } => {
                let members = cell.members();
                for member in &members {
                    for dep in self.cell_dependencies(member)? {
                        if !members.contains(&dep) && !deps.contains(&dep) {
                            deps.push(dep);
                        }
                    }
                }
            },
            Cell::Scalar(name) => {
                let formula = self.model.scalars[name].formula.as_deref().unwrap_or("");
                for full in self.extract_scalar_dependencies(formula, name)? {
                    deps.push(Cell::Scalar(full));
                }
                for word in formula_words(formula) {
                    if let Some(dep) = self.qualified_column(&word) {
                        deps.push(dep);
                    }
                }
            },
        }
        Ok(deps)
    }

    /// `table.column` where the column has a row formula
    fn qualified_column(&self, word: &str) -> Option<Cell> {
        let (table, column) = word.split_once('.')?;
        self.model
            .tables
            .get(table)
            .filter(|t| t.row_formulas.contains_key(column))
            .map(|_| Cell::Column {
                table: table.to_string(),
                column: column.to_string(),
            })
    }

    /// Calculate one row-formula column and store it in its table
    fn calculate_column(&mut self, table_name: &str, col_name: &str) -> ForgeResult<()> {
        let formula = self.model.tables[table_name].row_formulas[col_name].clone();
        if self.step_cached(
            |step| step.aggregates,
            |step, aggregates| step.aggregates = Some(aggregates),
            || {
                Ok(self
                    .plan
                    .aggregation(&formula, || Self::is_aggregation_formula(&formula)))
            },
        )? {
            return Err(ForgeError::Eval(format!(
                "Table '{table_name}': Column '{col_name}' uses aggregation formula - aggregations should be in scalars section"
            )));
        }
        let result = self
            .evaluate_rowwise_formula_ast(table_name, &formula)
            .map_err(|e| locate_error(e, &format!("{table_name}.{col_name}"), &formula))?;
        if let Some(ctx) = self.ctx.as_mut() {
            ctx.tables
                .entry(table_name.to_string())
                .or_default()
                .insert(col_name.to_string(), column_values(&result));
        }
        if let Some(stored) = self.model.tables.get_mut(table_name) {
            stored.add_column(Column::new(col_name.to_string(), result));
        }
        Ok(())
    }

    /// Calculate one scalar formula and store its value
    fn calculate_scalar(&mut self, scalar_name: &str) -> ForgeResult<()> {
        let Some(formula) = self
            .model
            .scalars
            .get(scalar_name)
            .and_then(|v| v.formula.clone())
        else {
            return Ok(());
        };
        let value = self
            .evaluate_scalar_formula_ast(scalar_name, &formula)
            .map_err(|e| locate_error(e, scalar_name, &formula))?;
        if let Some(var) = self.model.scalars.get_mut(scalar_name) {
            var.value = Some(value);
        }
        if let Some(ctx) = self.ctx.as_mut() {
            ctx.scalars
                .insert(scalar_name.to_string(), evaluator::Value::Number(value));
        }
        Ok(())
    }

    /// Check if a formula is an aggregation (returns scalar)
    fn is_aggregation_formula(formula: &str) -> bool {
        let body = formula.trim_start_matches('=').trim();
        match tokenizer::tokenize(body).map(parser::parse) {
            Ok(Ok(ast)) => Self::expr_has_aggregation(&ast),
            // Unparseable formulas fail later with a parse error
            _ => false,
        }
    }

    /// Whether an expression aggregates a whole column.
    ///
    /// `SUM`, `AVERAGE`, `MAX`, `MIN`, `COUNT` and `MEDIAN` aggregate only with
    /// a single argument: with several, a row formula such as `=MAX(0, ebit)`
    /// works row by row, as it does in Excel. The other functions listed always
    /// aggregate.
    fn expr_has_aggregation(expr: &parser::Expr) -> bool {
        use parser::Expr;
        const ROW_WISE_WITH_SEVERAL_ARGS: [&str; 7] =
            ["SUM", "AVERAGE", "AVG", "MAX", "MIN", "COUNT", "MEDIAN"];
        const ALWAYS_AGGREGATE: [&str; 18] = [
            "SUMPRODUCT",
            "SUMIF",
            "COUNTIF",
            "AVERAGEIF",
            "SUMIFS",
            "COUNTIFS",
            "AVERAGEIFS",
            "MAXIFS",
            "MINIFS",
            "VAR",
            "VAR.S",
            "VAR.P",
            "STDEV",
            "STDEV.S",
            "STDEV.P",
            "PERCENTILE",
            "QUARTILE",
            "CORREL",
        ];
        match expr {
            Expr::FunctionCall { name, args } => {
                let upper = name.to_uppercase();
                let aggregates = ALWAYS_AGGREGATE.contains(&upper.as_str())
                    || (ROW_WISE_WITH_SEVERAL_ARGS.contains(&upper.as_str()) && args.len() == 1);
                aggregates || args.iter().any(Self::expr_has_aggregation)
            },
            Expr::CallResult { callable, args } => {
                Self::expr_has_aggregation(callable) || args.iter().any(Self::expr_has_aggregation)
            },
            Expr::ArrayIndex { array, index } => {
                Self::expr_has_aggregation(array) || Self::expr_has_aggregation(index)
            },
            Expr::BinaryOp { left, right, .. } => {
                Self::expr_has_aggregation(left) || Self::expr_has_aggregation(right)
            },
            Expr::Range { start, end } => {
                Self::expr_has_aggregation(start) || Self::expr_has_aggregation(end)
            },
            Expr::UnaryOp { operand, .. } => Self::expr_has_aggregation(operand),
            Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) | Expr::Reference(_) => false,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AST-BASED FORMULA EVALUATION (v5.2.0 - Parser Architecture Refactor)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Resolve a name used in a formula to a full scalar name.
    ///
    /// A bare name such as `revenue` in a formula of section `budget` means
    /// `budget.revenue` if that exists, then a top-level scalar `revenue`, then
    /// the only scalar in any section with that last part. Dotted names must
    /// match exactly. Dependency ordering and evaluation both use this, so they
    /// agree, and the result never depends on hash map order.
    fn resolve_scalar_name(&self, word: &str, section: Option<&str>) -> ScalarName {
        if let Some(section) = section {
            if !word.contains('.') {
                let scoped = format!("{section}.{word}");
                if self.model.scalars.contains_key(&scoped) {
                    return ScalarName::Found(scoped);
                }
            }
        }
        if self.model.scalars.contains_key(word) {
            return ScalarName::Found(word.to_string());
        }
        if word.contains('.') {
            return ScalarName::Missing;
        }
        let by_last_part = self.by_last_part.get_or_init(|| {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for name in self.model.scalars.keys() {
                if let Some((_, last)) = name.rsplit_once('.') {
                    map.entry(last.to_string()).or_default().push(name.clone());
                }
            }
            for names in map.values_mut() {
                names.sort();
            }
            map
        });
        match by_last_part.get(word).map(Vec::as_slice) {
            None | Some([]) => ScalarName::Missing,
            Some([only]) => ScalarName::Found(only.clone()),
            Some(several) => ScalarName::Ambiguous(several.to_vec()),
        }
    }

    /// Return an error if a bare name in `formula` could mean several scalars.
    fn check_ambiguous_names(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        self.step_cached(
            |step| step.unambiguous.then_some(()),
            |step, ()| step.unambiguous = true,
            || self.check_ambiguous_names_by_text(formula, section, table),
        )
    }

    fn check_ambiguous_names_by_text(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        let key = (
            formula.to_string(),
            section.map(str::to_string),
            Some(table.name.clone()),
        );
        if self
            .plan
            .structure(self.fingerprint, |s| s.unambiguous.contains(&key))
        {
            return Ok(());
        }
        self.find_ambiguous_names(formula, section, table)?;
        self.plan
            .structure(self.fingerprint, |s| s.unambiguous.insert(key));
        Ok(())
    }

    fn find_ambiguous_names(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        let stripped = strip_string_literals(formula);
        for word in stripped.split(|c: char| !is_name_char(c)) {
            // A bare name is a column of the table first: a data column, or a
            // formula column (which may not be calculated yet, when the two
            // are calculated together row by row)
            if word.is_empty()
                || word.contains('.')
                || table.columns.contains_key(word)
                || table.row_formulas.contains_key(word)
            {
                continue;
            }
            if let ScalarName::Ambiguous(matches) = self.resolve_scalar_name(word, section) {
                return Err(ForgeError::Eval(format!(
                    "Name '{word}' in formula '{formula}' is ambiguous: it matches {}. \
                     Use the full name.",
                    matches.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// The values every formula can read by their full name: scalars,
    /// values of included files (`@namespace.path`), tables and scenarios.
    fn base_context(&self) -> evaluator::EvalContext {
        let mut ctx = evaluator::EvalContext::new();
        for (name, scalar) in &self.model.scalars {
            if let Some(value) = scalar.value {
                ctx.scalars
                    .insert(name.clone(), evaluator::Value::Number(value));
            }
        }
        for (name, value) in &self.included {
            ctx.scalars
                .insert(name.clone(), evaluator::Value::Number(*value));
        }
        for table in &self.included_tables {
            ctx.tables
                .insert(table.name.clone(), table.columns.iter().cloned().collect());
        }
        for (table_name, tbl) in &self.model.tables {
            let columns = tbl
                .columns
                .iter()
                .map(|(col_name, col)| (col_name.clone(), column_values(&col.values)))
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
    fn model_value(&self, ctx: &evaluator::EvalContext, full: &str) -> Option<f64> {
        debug_assert_eq!(
            ctx.scalars.get(full).and_then(evaluator::Value::as_number),
            self.model.scalars.get(full).and_then(|s| s.value),
            "context and model disagree on {full}"
        );
        ctx.scalars.get(full).and_then(evaluator::Value::as_number)
    }

    /// Each short name (`price` for `summary.price`) that resolves to exactly
    /// one scalar from `section`, with that scalar's full name, in name order.
    fn short_names(&self, section: Option<&str>) -> Rc<Vec<(String, String)>> {
        let key = section.map(str::to_string);
        if let Some(found) = self
            .plan
            .structure(self.fingerprint, |s| s.short_names.get(&key).cloned())
        {
            return found;
        }
        let short_names: std::collections::BTreeSet<&str> = self
            .model
            .scalars
            .keys()
            .filter_map(|name| name.rsplit_once('.').map(|(_, last)| last))
            .collect();
        let resolved: Vec<(String, String)> = short_names
            .into_iter()
            .filter_map(|short| match self.resolve_scalar_name(short, section) {
                ScalarName::Found(full) => Some((short.to_string(), full)),
                _ => None,
            })
            .collect();
        let resolved = Rc::new(resolved);
        self.plan.structure(self.fingerprint, |s| {
            s.short_names.insert(key, Rc::clone(&resolved));
        });
        resolved
    }

    /// Evaluate with the context a formula of `table` (row formulas) or of
    /// section `section` (scalar formulas) sees: every value by its full name,
    /// then the short names (`price` for `summary.price`) that resolve to
    /// exactly one scalar from that section, then the columns of `table` by
    /// their own names. The short names and columns are laid over the shared
    /// context for this formula only and removed afterwards.
    fn evaluate_in_context<R>(
        &mut self,
        table: Option<&str>,
        section: Option<&str>,
        evaluate: impl FnOnce(&mut evaluator::EvalContext) -> R,
    ) -> R {
        let mut ctx = self.ctx.take().unwrap_or_else(|| self.base_context());
        // Each name laid over the shared context, with what it held before
        let mut saved: Vec<(String, Option<evaluator::Value>)> = Vec::new();
        let mut lay = |ctx: &mut evaluator::EvalContext, name: String, value: evaluator::Value| {
            let previous = ctx.scalars.insert(name.clone(), value);
            if !saved.iter().any(|(laid, _)| *laid == name) {
                saved.push((name, previous));
            }
        };

        // The shared context holds each calculated scalar under its full name
        // (a full name is never another short name's key, so this reads the
        // scalar's own value)
        for (short_name, full) in self.short_names(section).iter() {
            if let Some(value) = self.model_value(&ctx, full) {
                lay(
                    &mut ctx,
                    short_name.clone(),
                    evaluator::Value::Number(value),
                );
            }
        }
        let table = table.and_then(|name| self.model.tables.get(name));
        if let Some(table) = table {
            for (col_name, col) in &table.columns {
                lay(
                    &mut ctx,
                    col_name.clone(),
                    evaluator::Value::Array(column_values(&col.values)),
                );
            }
        }
        ctx.current_row = None;
        ctx.row_count = Some(table.map_or(0, Table::row_count));

        let result = evaluate(&mut ctx);

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
                let first_num = first_result
                    .as_number()
                    .ok_or_else(|| ForgeError::Eval("row 1: result is not a number".to_string()))?;
                results.push(first_num);
                for row_idx in 1..row_count {
                    row_ctx.current_row = Some(row_idx);
                    let result = evaluator::evaluate(ast, row_ctx).map_err(|e| {
                        ForgeError::Eval(format!("row {}: {}", row_idx + 1, e.message))
                    })?;
                    let value = result.as_number().ok_or_else(|| {
                        ForgeError::Eval(format!("row {}: result is not a number", row_idx + 1))
                    })?;
                    results.push(value);
                }
                Ok(ColumnValue::Number(results))
            },
        }
    }

    /// Evaluate the formula of scalar `scalar_name` using the AST evaluator
    fn evaluate_scalar_formula_ast(
        &mut self,
        scalar_name: &str,
        formula: &str,
    ) -> ForgeResult<f64> {
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
        result
            .as_number()
            .ok_or_else(|| ForgeError::Eval("result is not a number".to_string()))
    }

    /// Extract scalar dependencies from a formula, using the same name
    /// resolution as evaluation (`resolve_scalar_name`)
    // Returns Result for consistency with other dependency extraction methods.
    #[allow(clippy::unnecessary_wraps)]
    fn extract_scalar_dependencies(
        &self,
        formula: &str,
        scalar_name: &str,
    ) -> ForgeResult<Vec<String>> {
        let mut deps = Vec::new();

        let section = scalar_name.rsplit_once('.').map(|(section, _)| section);
        let stripped = strip_string_literals(formula);
        for word in stripped.split(|c: char| !is_name_char(c)) {
            if word.is_empty() {
                continue;
            }
            if let ScalarName::Found(full) = self.resolve_scalar_name(word, section) {
                if !deps.contains(&full) {
                    deps.push(full);
                }
            }
            // Anything else is a table.column reference, a function name or a
            // literal, so no dependency edge is needed
        }

        Ok(deps)
    }
}

/// A column's values as the evaluator reads them (dates as text).
fn column_values(values: &ColumnValue) -> Vec<evaluator::Value> {
    match values {
        ColumnValue::Number(nums) => nums.iter().map(|n| evaluator::Value::Number(*n)).collect(),
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
    fn test_single_argument_aggregate_in_table_is_rejected() {
        for formula in ["=SUM(x)", "=MAX(x)", "=x / SUM(x)", "=SUMIF(x, \">0\", x)"] {
            let err = calculate_column(formula).unwrap_err();
            assert!(err.contains("aggregation"), "{formula}: {err}");
        }
    }

    #[test]
    fn test_function_name_inside_text_is_not_an_aggregate() {
        assert!(calculate_column("=IF(x > 0, \"SUM(\", \"no\")").is_ok());
    }
}
