//! Which arguments of a function are whole columns in a row formula (ADR-058).
//!
//! In a row formula a name is the value in the current row, except in an
//! argument that Excel reads as a range: there it is the whole column, of
//! another table (`months.revenue`) or of the row's own (`revenue`). So
//! `=SUMIFS(months.revenue, months.fiscal_quarter, quarter)` sums the months
//! of the row's quarter, and `=revenue / SUM(revenue)` is a share of the total.
//!
//! This one table is read by the evaluator (which hides the row while it
//! evaluates such an argument), by the calculation order (`PREV` groups) and by
//! the export, which writes such an argument as an absolute range.

use super::parser::{Expr, Reference};
use std::collections::HashSet;

/// Whether argument `position` (0-based) of `function` (upper case), called
/// with `arg_count` arguments, is read as a whole column in a row formula.
#[must_use]
// Grouped as ADR-058's table is, by what the arguments mean
#[allow(clippy::match_same_arms)]
pub fn is_whole_column(function: &str, arg_count: usize, position: usize) -> bool {
    match function {
        // With several arguments these work row by row: `=MAX(0, ebit)`
        "SUM" | "AVERAGE" | "AVG" | "MAX" | "MIN" | "COUNT" | "MEDIAN" | "MODE" | "MODE.SNGL"
        | "GEOMEAN" => arg_count == 1,
        // range, criterion, [sum range]
        "SUMIF" | "COUNTIF" | "AVERAGEIF" => position == 0 || position == 2,
        // sum range, then pairs of criteria range and criterion
        "SUMIFS" | "AVERAGEIFS" | "MAXIFS" | "MINIFS" => {
            position == 0 || !position.is_multiple_of(2)
        },
        // pairs of criteria range and criterion
        "COUNTIFS" => position.is_multiple_of(2),
        "COUNTBLANK" => position == 0,
        "SUMPRODUCT" | "VAR" | "VAR.S" | "VAR.P" | "STDEV" | "STDEV.S" | "STDEV.P" => true,
        // the values, then k or the quartile (a different one in each row)
        "PERCENTILE" | "PERCENTILE.INC" | "PERCENTILE.EXC" | "QUARTILE" | "QUARTILE.INC"
        | "QUARTILE.EXC" => position == 0,
        "CORREL" | "SLOPE" | "INTERCEPT" | "RSQ" | "COVARIANCE.S" | "COVARIANCE.P" => position <= 1,
        // x, then the known y and x values
        "FORECAST" | "FORECAST.LINEAR" => position == 1 || position == 2,
        // the lookups that always read whole columns
        "INDEX" | "ROWS" => position == 0,
        "MATCH" | "XMATCH" => position == 1,
        "XLOOKUP" => position == 1 || position == 2,
        _ => false,
    }
}

/// Whether some argument of `function` is a whole column with `arg_count` arguments.
#[must_use]
pub fn has_whole_column(function: &str, arg_count: usize) -> bool {
    (0..arg_count).any(|position| is_whole_column(function, arg_count, position))
}

/// The names a formula reads as whole columns: `revenue` and `months.revenue`
/// as written, inside any argument that [`is_whole_column`].
#[must_use]
pub fn whole_column_names(ast: &Expr) -> HashSet<String> {
    fn walk(expr: &Expr, whole: bool, names: &mut HashSet<String>) {
        match expr {
            Expr::Reference(reference) => {
                if whole {
                    names.insert(match reference {
                        Reference::Scalar(name) => name.clone(),
                        Reference::TableColumn { table, column } => format!("{table}.{column}"),
                    });
                }
            },
            Expr::FunctionCall { name, args } => {
                let upper = name.to_uppercase();
                for (position, arg) in args.iter().enumerate() {
                    let inside = whole || is_whole_column(&upper, args.len(), position);
                    walk(arg, inside, names);
                }
            },
            Expr::CallResult { callable, args } => {
                walk(callable, whole, names);
                for arg in args {
                    walk(arg, whole, names);
                }
            },
            Expr::ArrayIndex { array, index } => {
                walk(array, whole, names);
                walk(index, whole, names);
            },
            Expr::BinaryOp { left, right, .. } => {
                walk(left, whole, names);
                walk(right, whole, names);
            },
            Expr::Range { start, end } => {
                walk(start, whole, names);
                walk(end, whole, names);
            },
            Expr::UnaryOp { operand, .. } => walk(operand, whole, names),
            Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) => {},
        }
    }
    let mut names = HashSet::new();
    walk(ast, false, &mut names);
    names
}

#[cfg(test)]
mod tests {
    use super::is_whole_column;

    #[test]
    fn criteria_are_the_rows_value_and_ranges_are_whole() {
        // SUMIFS(sum, range1, crit1, range2, crit2)
        let sumifs: Vec<bool> = (0..5).map(|p| is_whole_column("SUMIFS", 5, p)).collect();
        assert_eq!(sumifs, [true, true, false, true, false]);
        // COUNTIFS(range1, crit1, range2, crit2)
        let countifs: Vec<bool> = (0..4).map(|p| is_whole_column("COUNTIFS", 4, p)).collect();
        assert_eq!(countifs, [true, false, true, false]);
        let sumif: Vec<bool> = (0..3).map(|p| is_whole_column("SUMIF", 3, p)).collect();
        assert_eq!(sumif, [true, false, true]);
        assert!(is_whole_column("PERCENTILE.INC", 2, 0));
        assert!(!is_whole_column("PERCENTILE.INC", 2, 1));
        assert!(!is_whole_column("FORECAST", 3, 0));
        assert!(is_whole_column("FORECAST", 3, 2));
    }

    #[test]
    fn one_argument_aggregates_and_several_work_row_by_row() {
        assert!(is_whole_column("SUM", 1, 0));
        assert!(!is_whole_column("SUM", 2, 0));
        assert!(!is_whole_column("MAX", 2, 1));
        assert!(is_whole_column("GEOMEAN", 1, 0));
        // Functions not in the table keep reading the row's value
        assert!(!is_whole_column("NPV", 2, 1));
        assert!(!is_whole_column("IF", 3, 0));
    }
}
