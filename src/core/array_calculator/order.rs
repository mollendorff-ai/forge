//! What each formula reads, and the order the cells are calculated in: a
//! column or a scalar after everything it reads, a cycle refused.

use super::names::ScalarName;
use super::{plan, ArrayCalculator};
use crate::error::ForgeResult;
use std::rc::Rc;

/// A cell with a formula: a whole table column (row formula) or a scalar.
/// Ordered by kind, then name, so iteration is deterministic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Cell {
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
    pub(super) fn label(&self) -> String {
        match self {
            Self::Column { table, column } => format!("{table}.{column}"),
            Self::Scalar(name) => name.clone(),
            Self::Columns { table, columns } => format!("{table}.{}", columns.join(", ")),
        }
    }

    /// The single cells this step calculates: itself, or each column of a
    /// group. Other cells name these as what they depend on.
    pub(super) fn members(&self) -> Vec<Self> {
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
pub(super) fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '@'
}

/// The names a formula mentions (`revenue`, `assumptions.rate`), without the
/// contents of string literals. Function names and numbers are included; they
/// match no cell, so they add no dependency.
pub(super) fn formula_words(formula: &str) -> Vec<String> {
    let stripped = strip_string_literals(formula);
    let mut words: Vec<String> = Vec::new();
    for word in stripped.split(|c: char| !is_name_char(c)) {
        if !word.is_empty() && !words.iter().any(|w| w == word) {
            words.push(word.to_string());
        }
    }
    words
}

/// Strip string literals from a formula before extracting references.
/// This prevents content inside quotes from being parsed as column references.
/// e.g., =LEN("Hello") should not treat "Hello" as a column reference.
pub(super) fn strip_string_literals(formula: &str) -> String {
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

impl ArrayCalculator {
    /// [`ArrayCalculator::calculation_order`] with each cell's dependencies,
    /// from the plan when a model with the same formulas was ordered before.
    pub(super) fn planned_order(&self) -> ForgeResult<plan::Order> {
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
    pub(super) fn cell_dependencies(&self, cell: &Cell) -> ForgeResult<Vec<Cell>> {
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
    pub(super) fn qualified_column(&self, word: &str) -> Option<Cell> {
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

    /// Extract scalar dependencies from a formula, using the same name
    /// resolution as evaluation (`resolve_scalar_name`)
    // Returns Result for consistency with other dependency extraction methods.
    #[allow(clippy::unnecessary_wraps)]
    pub(super) fn extract_scalar_dependencies(
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
mod tests {
    //! Tests that table columns and scalars are calculated in one dependency order,
    //! and that a formula scalar's stored value is never read

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    fn scalar(model: &mut ParsedModel, name: &str, value: Option<f64>, formula: Option<&str>) {
        model.add_scalar(
            name.to_string(),
            Variable::new(name.to_string(), value, formula.map(str::to_string)),
        );
    }

    fn table(name: &str, data: &[(&str, Vec<f64>)], formulas: &[(&str, &str)]) -> Table {
        let mut table = Table::new(name.to_string());
        for (column, values) in data {
            table.add_column(Column::new(
                (*column).to_string(),
                ColumnValue::Number(values.clone()),
            ));
        }
        for (column, formula) in formulas {
            table.add_row_formula((*column).to_string(), (*formula).to_string());
        }
        table
    }

    fn numbers(model: &ParsedModel, table: &str, column: &str) -> Vec<f64> {
        match &model.tables[table].columns[column].values {
            ColumnValue::Number(v) => v.clone(),
            other => panic!("{table}.{column} is not numeric: {other:?}"),
        }
    }

    fn value(model: &ParsedModel, name: &str) -> f64 {
        model.scalars[name].value.unwrap()
    }

    #[test]
    fn test_column_uses_recalculated_scalar_not_stored_value() {
        // The stored 999 is stale (from the file, an import or an earlier run)
        let mut model = ParsedModel::new();
        scalar(&mut model, "assumptions.base", Some(10.0), None);
        scalar(
            &mut model,
            "assumptions.double",
            Some(999.0),
            Some("=assumptions.base * 2"),
        );
        model.add_table(table(
            "t",
            &[("x", vec![1.0, 2.0, 3.0])],
            &[("y", "=x * assumptions.double")],
        ));

        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(numbers(&result, "t", "y"), vec![20.0, 40.0, 60.0]);
        assert!((value(&result, "assumptions.double") - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_column_uses_calculated_scalar_without_stored_value() {
        let mut model = ParsedModel::new();
        scalar(&mut model, "assumptions.base", Some(10.0), None);
        scalar(
            &mut model,
            "assumptions.double",
            None,
            Some("=assumptions.base * 2"),
        );
        model.add_table(table(
            "t",
            &[("x", vec![1.0, 2.0])],
            &[("y", "=x * double")],
        ));

        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(numbers(&result, "t", "y"), vec![20.0, 40.0]);
    }

    #[test]
    fn test_share_of_total_is_not_a_cycle() {
        // share reads total; total sums an input column of the same table
        let mut model = ParsedModel::new();
        model.add_table(table(
            "t",
            &[("revenue", vec![100.0, 300.0])],
            &[("share", "=revenue / summary.total")],
        ));
        scalar(&mut model, "summary.total", None, Some("=SUM(t.revenue)"));
        scalar(&mut model, "summary.top", None, Some("=MAX(t.share)"));

        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(numbers(&result, "t", "share"), vec![0.25, 0.75]);
        assert!((value(&result, "summary.total") - 400.0).abs() < f64::EPSILON);
        assert!((value(&result, "summary.top") - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_column_in_one_table_uses_calculated_column_of_another() {
        let mut model = ParsedModel::new();
        model.add_table(table("a", &[("x", vec![1.0, 2.0])], &[("y", "=x * 10")]));
        model.add_table(table("b", &[("z", vec![3.0, 4.0])], &[("w", "=z + a.y")]));

        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(numbers(&result, "b", "w"), vec![13.0, 24.0]);
    }

    #[test]
    fn test_cycle_through_a_column_and_a_scalar_is_named() {
        let mut model = ParsedModel::new();
        model.add_table(table("t", &[("x", vec![1.0, 2.0])], &[("y", "=x * s.k")]));
        scalar(&mut model, "s.k", None, Some("=SUM(t.y)"));

        let err = ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("formulas t.y, s.k depend on each other"),
            "{err}"
        );
    }

    #[test]
    fn test_stored_value_of_a_failing_formula_is_not_used() {
        // Before, a stale stored value could stand in for a formula that was never
        // recalculated; now the formula must calculate or the model fails
        let mut model = ParsedModel::new();
        scalar(&mut model, "s.bad", Some(5.0), Some("=1 / 0"));
        scalar(&mut model, "s.uses_bad", None, Some("=s.bad + 1"));

        let err = ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert!(err.contains("s.bad: Division by zero"), "{err}");
    }

    #[test]
    fn test_order_is_the_same_on_every_run() {
        // Several independent failures: the same one is reported every time
        let build = || {
            let mut model = ParsedModel::new();
            for name in ["s.a", "s.b", "s.c", "s.d"] {
                scalar(&mut model, name, None, Some("=1 / 0"));
            }
            model
        };
        let first = ArrayCalculator::new(build())
            .calculate_all()
            .unwrap_err()
            .to_string();
        for _ in 0..20 {
            let again = ArrayCalculator::new(build())
                .calculate_all()
                .unwrap_err()
                .to_string();
            assert_eq!(first, again);
        }
    }
}
