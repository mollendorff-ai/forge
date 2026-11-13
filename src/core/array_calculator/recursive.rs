//! Columns that read the row above (`PREV`, ADR-053).
//!
//! `opening: "=PREV(closing, loan.principal)"` and `closing: "=opening -
//! paid"` read each other, but never in the same row: `opening` reads the
//! `closing` of the row above. So the two are calculated together, row by row,
//! instead of one whole column after the other.
//!
//! A dependency is *lagged* when a formula reads a column of its own table
//! only inside the first argument of `PREV`. The calculation order is found
//! in three steps:
//!
//! 1. Without the lagged dependencies the cells must have an order, as
//!    before: a cycle there is a circular dependency.
//! 2. With them, cells that reach each other form a group. A group must be
//!    columns of one table (a cycle through a scalar or another table needs a
//!    whole column before its first row exists).
//! 3. Each group is one step of the order, calculated row by row, its columns
//!    in the order of step 1 within each row.
//!
//! A model without `PREV` has no lagged dependency and no group, and gets the
//! order it always had.

use super::evaluator::{self, Value};
use super::parser::{Expr, Reference};
use super::{describe_cycle, locate_error, ArrayCalculator, Cell};
use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue};
use petgraph::algo::{tarjan_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// The names a formula reads in its own row, and the names it reads only in
/// an earlier row (inside the first argument of `PREV`).
fn names_by_row(ast: &Expr) -> (HashSet<String>, HashSet<String>) {
    fn walk(expr: &Expr, lagged: bool, same: &mut HashSet<String>, earlier: &mut HashSet<String>) {
        match expr {
            Expr::Reference(reference) => {
                let name = match reference {
                    Reference::Scalar(name) => name.clone(),
                    Reference::TableColumn { table, column } => format!("{table}.{column}"),
                };
                if lagged {
                    earlier.insert(name);
                } else {
                    same.insert(name);
                }
            },
            Expr::FunctionCall { name, args } => {
                let is_prev = name.eq_ignore_ascii_case("PREV");
                for (i, arg) in args.iter().enumerate() {
                    walk(arg, lagged || (is_prev && i == 0), same, earlier);
                }
            },
            Expr::CallResult { callable, args } => {
                walk(callable, lagged, same, earlier);
                for arg in args {
                    walk(arg, lagged, same, earlier);
                }
            },
            Expr::ArrayIndex { array, index } => {
                walk(array, lagged, same, earlier);
                walk(index, lagged, same, earlier);
            },
            Expr::BinaryOp { left, right, .. } => {
                walk(left, lagged, same, earlier);
                walk(right, lagged, same, earlier);
            },
            Expr::Range { start, end } => {
                walk(start, lagged, same, earlier);
                walk(end, lagged, same, earlier);
            },
            Expr::UnaryOp { operand, .. } => walk(operand, lagged, same, earlier),
            Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) => {},
        }
    }
    let (mut same, mut earlier) = (HashSet::new(), HashSet::new());
    walk(ast, false, &mut same, &mut earlier);
    earlier.retain(|name| !same.contains(name));
    (same, earlier)
}

/// One column of numbers, text or booleans from the values of its rows; the
/// first row decides which, as for any row formula.
fn column_from(values: &[Value]) -> Result<ColumnValue, String> {
    match values.first() {
        Some(Value::Text(_)) => Ok(ColumnValue::Text(
            values.iter().map(Value::as_text).collect(),
        )),
        Some(Value::Boolean(_)) => Ok(ColumnValue::Boolean(
            values
                .iter()
                .map(|v| v.as_bool().unwrap_or(false))
                .collect(),
        )),
        _ => values
            .iter()
            .enumerate()
            .map(|(row, value)| {
                value
                    .as_number()
                    .ok_or_else(|| format!("row {}: result is not a number", row + 1))
            })
            .collect::<Result<Vec<f64>, String>>()
            .map(ColumnValue::Number),
    }
}

/// Cells and which reads which (an edge goes from the cell read).
type Graph = DiGraph<Cell, ()>;

/// One column's name, `table.column`, formula and parsed formula.
type RowFormula = (String, String, String, std::rc::Rc<Expr>);

/// Evaluate `formulas` row by row, in the order given within each row, each
/// column growing by one value per row so the next row can read it.
fn rows_together(
    ctx: &mut evaluator::EvalContext,
    table_name: &str,
    formulas: &[RowFormula],
    row_count: usize,
) -> ForgeResult<Vec<(String, ColumnValue)>> {
    // Each column grows by one value per row, under its bare name and
    // as `table.column`; what the bare name held before comes back
    let before: Vec<Option<Value>> = formulas
        .iter()
        .map(|(column, ..)| {
            ctx.tables
                .entry(table_name.to_string())
                .or_default()
                .insert(column.clone(), Vec::with_capacity(row_count));
            ctx.scalars
                .insert(column.clone(), Value::Array(Vec::with_capacity(row_count)))
        })
        .collect();
    ctx.row_count = Some(row_count);
    ctx.growing_table = Some(table_name.to_string());
    let mut failure = None;
    'rows: for row in 0..row_count {
        ctx.current_row = Some(row);
        for (column, place, formula, ast) in formulas {
            match evaluator::evaluate(ast, ctx) {
                Ok(value) => {
                    if let Some(Value::Array(values)) = ctx.scalars.get_mut(column) {
                        values.push(value.clone());
                    }
                    if let Some(values) = ctx
                        .tables
                        .get_mut(table_name)
                        .and_then(|t| t.get_mut(column))
                    {
                        values.push(value);
                    }
                },
                Err(e) => {
                    let hint = if e.message.contains("out of bounds") {
                        ". A column calculated with PREV is known only up to the row \
                         being calculated: read it in the same row after it is \
                         calculated, or in the row above through PREV"
                    } else {
                        ""
                    };
                    failure = Some(locate_error(
                        ForgeError::Eval(format!("row {}: {}{hint}", row + 1, e.message)),
                        place,
                        formula,
                    ));
                    break 'rows;
                },
            }
        }
    }
    ctx.growing_table = None;
    let mut results = Vec::with_capacity(formulas.len());
    for ((column, place, formula, _), held) in formulas.iter().zip(before) {
        let values = match ctx.scalars.remove(column) {
            Some(Value::Array(values)) => values,
            _ => Vec::new(),
        };
        if let Some(held) = held {
            ctx.scalars.insert(column.clone(), held);
        }
        if failure.is_some() {
            if let Some(table) = ctx.tables.get_mut(table_name) {
                table.remove(column);
            }
            continue;
        }
        match column_from(&values) {
            Ok(values) => results.push((column.clone(), values)),
            Err(e) => failure = Some(locate_error(ForgeError::Eval(e), place, formula)),
        }
    }
    failure.map_or(Ok(results), Err)
}

impl ArrayCalculator {
    /// The columns of its own table that the formula of `table.column` reads
    /// only in an earlier row.
    fn lagged_dependencies(&self, table: &str, column: &str) -> Vec<Cell> {
        let own = &self.model.tables[table];
        let Ok(ast) = self.parsed_by_text(&own.row_formulas[column]) else {
            // A formula that does not parse fails when it is calculated
            return Vec::new();
        };
        let (_, earlier) = names_by_row(&ast);
        let mut cells: Vec<Cell> = earlier
            .iter()
            .filter_map(|name| {
                let bare = name
                    .strip_prefix(table)
                    .and_then(|rest| rest.strip_prefix('.'))
                    .unwrap_or(name);
                own.row_formulas.contains_key(bare).then(|| Cell::Column {
                    table: table.to_string(),
                    column: bare.to_string(),
                })
            })
            .collect();
        cells.sort();
        cells
    }

    /// The cells as a graph of every dependency, the same without the lagged
    /// ones, and whether there is a lagged one. Nodes are the cells in order.
    fn dependency_graphs(&self, cells: &BTreeSet<Cell>) -> ForgeResult<(Graph, Graph, bool)> {
        let mut full: DiGraph<Cell, ()> = DiGraph::new();
        let mut same_row: DiGraph<Cell, ()> = DiGraph::new();
        let mut index: BTreeMap<&Cell, NodeIndex> = BTreeMap::new();
        for cell in cells {
            let node = full.add_node(cell.clone());
            same_row.add_node(cell.clone());
            index.insert(cell, node);
        }
        let mut any_lagged = false;
        for cell in cells {
            let lagged = match cell {
                Cell::Column { table, column } => self.lagged_dependencies(table, column),
                _ => Vec::new(),
            };
            for dep in self.cell_dependencies(cell)? {
                if let (Some(&from), Some(&to)) = (index.get(&dep), index.get(cell)) {
                    full.add_edge(from, to, ());
                    if lagged.contains(&dep) {
                        any_lagged = true;
                    } else {
                        same_row.add_edge(from, to, ());
                    }
                }
            }
        }
        Ok((full, same_row, any_lagged))
    }

    /// Every formula cell (table column or scalar) in dependency order, the
    /// columns that read each other through `PREV` as one step.
    ///
    /// Nodes are added in name order, so the order, and which error is
    /// reported first, is the same on every run.
    pub(super) fn calculation_order(&self) -> ForgeResult<Vec<Cell>> {
        let mut cells: BTreeSet<Cell> = BTreeSet::new();
        for (table_name, table) in &self.model.tables {
            for column in table.row_formulas.keys() {
                cells.insert(Cell::Column {
                    table: table_name.clone(),
                    column: column.clone(),
                });
            }
        }
        for (name, var) in &self.model.scalars {
            if var.formula.is_some() {
                cells.insert(Cell::Scalar(name.clone()));
            }
        }

        let (full, same_row, any_lagged) = self.dependency_graphs(&cells)?;
        let index: BTreeMap<&Cell, NodeIndex> = cells.iter().zip(full.node_indices()).collect();

        let within_a_row = toposort(&same_row, None)
            .map_err(|_| ForgeError::CircularDependency(describe_cycle(&same_row)))?;
        if !any_lagged {
            return Ok(within_a_row
                .into_iter()
                .filter_map(|idx| same_row.node_weight(idx).cloned())
                .collect());
        }
        let position: BTreeMap<NodeIndex, usize> = within_a_row
            .iter()
            .enumerate()
            .map(|(at, &node)| (node, at))
            .collect();

        // Cells that reach each other through a lagged dependency
        let mut group_of: BTreeMap<NodeIndex, usize> = BTreeMap::new();
        let mut groups: Vec<Cell> = Vec::new();
        for mut component in tarjan_scc(&full) {
            if component.len() == 1 && !full.contains_edge(component[0], component[0]) {
                continue;
            }
            component.sort_by_key(|node| position[node]);
            let mut columns = Vec::new();
            let mut tables = BTreeSet::new();
            for &node in &component {
                if let Cell::Column { table, column } = &full[node] {
                    tables.insert(table.clone());
                    columns.push(column.clone());
                }
            }
            if columns.len() != component.len() || tables.len() != 1 {
                let mut names: Vec<String> = component.iter().map(|&n| full[n].label()).collect();
                names.sort();
                return Err(ForgeError::CircularDependency(format!(
                    "{} depend on each other. PREV reads the row above, so it breaks a cycle \
                     only among columns of one table; a scalar or another table in the cycle \
                     needs a whole column first",
                    names.join(", ")
                )));
            }
            for &node in &component {
                group_of.insert(node, groups.len());
            }
            groups.push(Cell::Columns {
                table: tables.into_iter().next().unwrap_or_default(),
                columns,
            });
        }

        // The same graph with each group as one node, nodes in name order
        let mut steps: DiGraph<Cell, ()> = DiGraph::new();
        let mut step_of: BTreeMap<NodeIndex, NodeIndex> = BTreeMap::new();
        let mut group_step: BTreeMap<usize, NodeIndex> = BTreeMap::new();
        for cell in &cells {
            let node = index[cell];
            let step = match group_of.get(&node) {
                Some(&group) => *group_step
                    .entry(group)
                    .or_insert_with(|| steps.add_node(groups[group].clone())),
                None => steps.add_node(cell.clone()),
            };
            step_of.insert(node, step);
        }
        for edge in full.raw_edges() {
            let (from, to) = (step_of[&edge.source()], step_of[&edge.target()]);
            if from != to {
                steps.update_edge(from, to, ());
            }
        }
        let order = toposort(&steps, None)
            .map_err(|_| ForgeError::CircularDependency(describe_cycle(&steps)))?;
        Ok(order
            .into_iter()
            .filter_map(|idx| steps.node_weight(idx).cloned())
            .collect())
    }

    /// Calculate columns of one table that read each other through `PREV`:
    /// row by row, the columns in the order given within each row.
    pub(super) fn calculate_columns(
        &mut self,
        table_name: &str,
        columns: &[String],
    ) -> ForgeResult<()> {
        let table = &self.model.tables[table_name];
        let mut formulas = Vec::with_capacity(columns.len());
        for column in columns {
            let formula = table.row_formulas[column].clone();
            let place = format!("{table_name}.{column}");
            let ast = self
                .parsed_by_text(&formula)
                .and_then(|ast| {
                    self.check_ambiguous_names_by_text(&formula, None, table)?;
                    Ok(ast)
                })
                .map_err(|e| locate_error(e, &place, &formula))?;
            // A whole column of the group does not exist until its last row (ADR-058)
            let whole = super::whole_column::whole_column_names(&ast);
            if let Some(name) = columns.iter().find(|c| {
                whole.contains(c.as_str()) || whole.contains(&format!("{table_name}.{c}"))
            }) {
                return Err(locate_error(
                    ForgeError::Eval(format!(
                        "reads all of {table_name}.{name}, which is calculated row by row with \
                         this column through PREV, so it needs a whole column first"
                    )),
                    &place,
                    &formula,
                ));
            }
            formulas.push((column.clone(), place, formula, ast));
        }
        let row_count = table.row_count();
        if row_count == 0 {
            let (_, place, formula, _) = &formulas[0];
            return Err(locate_error(
                ForgeError::Eval("Cannot evaluate on empty table".to_string()),
                place,
                formula,
            ));
        }

        let results = self.evaluate_in_context(Some(table_name), None, |ctx| {
            rows_together(ctx, table_name, &formulas, row_count)
        })?;

        if let Some(stored) = self.model.tables.get_mut(table_name) {
            for (column, values) in results {
                stored.add_column(Column::new(column, values));
            }
        }
        Ok(())
    }
}
