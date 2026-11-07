#[cfg(test)]
mod calculation_order_tests;
mod dates;
#[cfg(test)]
mod error_location_tests;
pub mod evaluator;
#[cfg(test)]
mod include_reference_tests;
pub mod parser;
#[cfg(test)]
mod scalar_scope_tests;
pub mod tokenizer;

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};

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
    Column { table: String, column: String },
    Scalar(String),
}

impl Cell {
    fn label(&self) -> String {
        match self {
            Self::Column { table, column } => format!("{table}.{column}"),
            Self::Scalar(name) => name.clone(),
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

/// Array-aware calculator for v1.0.0 models
/// Handles both row-wise (element-wise) and aggregation formulas
pub struct ArrayCalculator {
    model: ParsedModel,
    /// Scalars of included files as `@namespace.path`, calculated before the
    /// model (see [`ArrayCalculator::included_values`]).
    included: Vec<(String, f64)>,
}

impl ArrayCalculator {
    #[must_use]
    pub const fn new(model: ParsedModel) -> Self {
        Self {
            model,
            included: Vec::new(),
        }
    }

    /// The scalars of every included file, as `@namespace.path` with their
    /// values. Each included model is calculated first, with its own includes,
    /// so a value an included file defines by formula is available. Namespaces
    /// are taken in name order, so an error is the same on every run.
    fn included_values(&self) -> ForgeResult<Vec<(String, f64)>> {
        let mut namespaces: Vec<&String> = self.model.resolved_includes.keys().collect();
        namespaces.sort();
        let mut values = Vec::new();
        for namespace in namespaces {
            let resolved = &self.model.resolved_includes[namespace];
            let calculated = Self::new(resolved.model.clone())
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
        }
        Ok(values)
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
        self.included = self.included_values()?;

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
        for cell in self.calculation_order()? {
            let blocked_by = self
                .cell_dependencies(&cell)?
                .into_iter()
                .find(|dep| failed.contains(dep));
            if let Some(dep) = blocked_by {
                skipped.push((cell.clone(), dep));
                failed.insert(cell);
                continue;
            }
            let outcome = match &cell {
                Cell::Column { table, column } => self.calculate_column(table, column),
                Cell::Scalar(name) => self.calculate_scalar(name),
            };
            if let Err(e) = outcome {
                failures.push(match e {
                    ForgeError::Eval(msg) => msg,
                    other => other.to_string(),
                });
                failed.insert(cell);
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

    /// Every formula cell (table column or scalar) in dependency order.
    ///
    /// Nodes are added in name order, so the order, and which error is
    /// reported first, is the same on every run.
    fn calculation_order(&self) -> ForgeResult<Vec<Cell>> {
        use petgraph::algo::toposort;
        use petgraph::graph::DiGraph;
        use std::collections::BTreeMap;

        let mut graph: DiGraph<Cell, ()> = DiGraph::new();
        let mut index = BTreeMap::new();

        let mut table_names: Vec<&String> = self.model.tables.keys().collect();
        table_names.sort();
        for table_name in &table_names {
            let mut columns: Vec<&String> =
                self.model.tables[*table_name].row_formulas.keys().collect();
            columns.sort();
            for column in columns {
                let cell = Cell::Column {
                    table: (*table_name).clone(),
                    column: column.clone(),
                };
                index.insert(cell.clone(), graph.add_node(cell));
            }
        }
        let mut scalar_names: Vec<&String> = self
            .model
            .scalars
            .iter()
            .filter(|(_, var)| var.formula.is_some())
            .map(|(name, _)| name)
            .collect();
        scalar_names.sort();
        for name in scalar_names {
            let cell = Cell::Scalar(name.clone());
            index.insert(cell.clone(), graph.add_node(cell));
        }

        let cells: Vec<Cell> = index.keys().cloned().collect();
        for cell in &cells {
            for dep in self.cell_dependencies(cell)? {
                if let (Some(&from), Some(&to)) = (index.get(&dep), index.get(cell)) {
                    graph.add_edge(from, to, ());
                }
            }
        }

        let order = toposort(&graph, None)
            .map_err(|_| ForgeError::CircularDependency(describe_cycle(&graph)))?;
        Ok(order
            .into_iter()
            .filter_map(|idx| graph.node_weight(idx).cloned())
            .collect())
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
        let table = self.model.tables[table_name].clone();
        let formula = table.row_formulas[col_name].clone();
        if Self::is_aggregation_formula(&formula) {
            return Err(ForgeError::Eval(format!(
                "Table '{table_name}': Column '{col_name}' uses aggregation formula - aggregations should be in scalars section"
            )));
        }
        let result = self
            .evaluate_rowwise_formula_ast(&table, &formula)
            .map_err(|e| locate_error(e, &format!("{table_name}.{col_name}"), &formula))?;
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
        const ALWAYS_AGGREGATE: [&str; 17] = [
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
        let mut matches: Vec<String> = self
            .model
            .scalars
            .keys()
            .filter(|name| name.rsplit_once('.').is_some_and(|(_, last)| last == word))
            .cloned()
            .collect();
        matches.sort();
        match matches.len() {
            0 => ScalarName::Missing,
            1 => ScalarName::Found(matches.remove(0)),
            _ => ScalarName::Ambiguous(matches),
        }
    }

    /// Return an error if a bare name in `formula` could mean several scalars.
    fn check_ambiguous_names(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        let stripped = strip_string_literals(formula);
        for word in stripped.split(|c: char| !is_name_char(c)) {
            if word.is_empty() || word.contains('.') || table.columns.contains_key(word) {
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

    /// Build an evaluation context from the model state for a given table.
    ///
    /// `section` is the section of the scalar being calculated, if any; it
    /// decides which scalar a bare name refers to.
    fn build_eval_context(&self, table: &Table, section: Option<&str>) -> evaluator::EvalContext {
        use std::collections::{BTreeSet, HashMap};
        let mut ctx = evaluator::EvalContext::new();

        // Add all scalars to context under their full names
        for (name, scalar) in &self.model.scalars {
            if let Some(value) = scalar.value {
                ctx.scalars
                    .insert(name.clone(), evaluator::Value::Number(value));
            }
        }

        // Values from included files, under `@namespace.path`
        for (name, value) in &self.included {
            ctx.scalars
                .insert(name.clone(), evaluator::Value::Number(*value));
        }

        // Add short names (e.g., "price" for "summary.price") that resolve to
        // exactly one scalar, so formulas can reference them without prefix
        let short_names: BTreeSet<&str> = self
            .model
            .scalars
            .keys()
            .filter_map(|name| name.rsplit_once('.').map(|(_, last)| last))
            .collect();
        for short_name in short_names {
            if let ScalarName::Found(full) = self.resolve_scalar_name(short_name, section) {
                if let Some(value) = self.model.scalars.get(&full).and_then(|s| s.value) {
                    ctx.scalars
                        .insert(short_name.to_string(), evaluator::Value::Number(value));
                }
            }
        }

        // Add current table columns
        for (col_name, col) in &table.columns {
            let values: Vec<evaluator::Value> = match &col.values {
                ColumnValue::Number(nums) => {
                    nums.iter().map(|n| evaluator::Value::Number(*n)).collect()
                },
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
            };
            ctx.scalars
                .insert(col_name.clone(), evaluator::Value::Array(values));
        }

        // Add all tables to context
        for (table_name, tbl) in &self.model.tables {
            let mut table_data: HashMap<String, Vec<evaluator::Value>> = HashMap::new();
            for (col_name, col) in &tbl.columns {
                let values: Vec<evaluator::Value> = match &col.values {
                    ColumnValue::Number(nums) => {
                        nums.iter().map(|n| evaluator::Value::Number(*n)).collect()
                    },
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
                };
                table_data.insert(col_name.clone(), values);
            }
            ctx.tables.insert(table_name.clone(), table_data);
        }

        // Add scenarios to context
        for (scenario_name, scenario) in &self.model.scenarios {
            let mut overrides = HashMap::new();
            for (var_name, value) in &scenario.overrides {
                overrides.insert(var_name.clone(), *value);
            }
            ctx.scenarios.insert(scenario_name.clone(), overrides);
        }

        ctx.row_count = Some(table.row_count());
        ctx
    }

    /// Evaluate a row-wise formula using the AST evaluator
    fn evaluate_rowwise_formula_ast(
        &self,
        table: &Table,
        formula: &str,
    ) -> ForgeResult<ColumnValue> {
        let formula_str = formula.trim_start_matches('=').trim();
        let tokens = tokenizer::tokenize(formula_str)
            .map_err(|e| ForgeError::Eval(format!("syntax error: {}", e.message)))?;
        let ast = parser::parse(tokens)
            .map_err(|e| ForgeError::Eval(format!("syntax error: {}", e.message)))?;

        self.check_ambiguous_names(formula, None, table)?;
        let base_ctx = self.build_eval_context(table, None);
        let row_count = table.row_count();
        if row_count == 0 {
            return Err(ForgeError::Eval(
                "Cannot evaluate on empty table".to_string(),
            ));
        }

        // Evaluate first row to determine result type
        let first_ctx = base_ctx.clone().with_row(0, row_count);
        let first_result = evaluator::evaluate(&ast, &first_ctx)
            .map_err(|e| ForgeError::Eval(format!("row 1: {}", e.message)))?;

        // Determine column type from first result and evaluate all rows
        match &first_result {
            evaluator::Value::Text(_) => {
                let mut results: Vec<String> = Vec::with_capacity(row_count);
                results.push(first_result.as_text());
                for row_idx in 1..row_count {
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx).map_err(|e| {
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
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx).map_err(|e| {
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
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx).map_err(|e| {
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
    fn evaluate_scalar_formula_ast(&self, scalar_name: &str, formula: &str) -> ForgeResult<f64> {
        let formula_str = formula.trim_start_matches('=').trim();
        let tokens = tokenizer::tokenize(formula_str)
            .map_err(|e| ForgeError::Eval(format!("syntax error: {}", e.message)))?;
        let ast = parser::parse(tokens)
            .map_err(|e| ForgeError::Eval(format!("syntax error: {}", e.message)))?;

        let empty_table = Table::new("_scalar_context".to_string());
        let section = scalar_name.rsplit_once('.').map(|(section, _)| section);
        self.check_ambiguous_names(formula, section, &empty_table)?;
        let ctx = self.build_eval_context(&empty_table, section);
        let result = evaluator::evaluate(&ast, &ctx).map_err(|e| ForgeError::Eval(e.message))?;
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
