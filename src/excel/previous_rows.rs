//! A column whose first rows differ from the rest, read back as the `PREV`
//! that was written (roadmap T3-IMPORT-PREV, ADR-053).
//!
//! `forge export` writes `PREV(value, first, back)` as `first` in the first
//! `back` rows and as `value` read `back` rows up in the others. Read back
//! with [`super::workbook_refs::ReverseRefs::to_forge_reading_above`], the
//! later rows give one formula with a marker `PREV(column, 0, back)` for each
//! cell read from a row above, and the first row gives the same formula with
//! the first values in their places. Lined up, the two give the `PREV` back:
//! `PREV(closing, 0) + x` and `loan.principal + x` are
//! `PREV(closing, loan.principal) + x`.
//!
//! The result is kept only if it gives every first row exactly as the
//! workbook holds it.

use crate::core::array_calculator::parser::{parse, Expr};
use crate::core::array_calculator::tokenizer::tokenize;

fn parsed(formula: &str) -> Option<Expr> {
    parse(tokenize(formula.trim_start_matches('=')).ok()?).ok()
}

/// How many rows back a marker reads: `PREV(column, 0)` is 1,
/// `PREV(column, 0, n)` is n.
fn marker_back(expr: &Expr) -> Option<usize> {
    let Expr::FunctionCall { name, args } = expr else {
        return None;
    };
    if name != "PREV" {
        return None;
    }
    match args.as_slice() {
        [_, Expr::Number(zero)] if *zero == 0.0 => Some(1),
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        [_, Expr::Number(zero), Expr::Number(back)]
            if *zero == 0.0 && *back >= 1.0 && back.fract() == 0.0 =>
        {
            Some(*back as usize)
        },
        _ => None,
    }
}

fn children(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::FunctionCall { args, .. } => args.iter().collect(),
        Expr::CallResult { callable, args } => {
            std::iter::once(callable.as_ref()).chain(args).collect()
        },
        Expr::BinaryOp { left, right, .. } => vec![left, right],
        Expr::UnaryOp { operand, .. } => vec![operand],
        Expr::ArrayIndex { array, index } => vec![array, index],
        Expr::Range { start, end } => vec![start, end],
        Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) | Expr::Reference(_) => Vec::new(),
    }
}

/// The rows back of every marker in `expr`.
fn backs(expr: &Expr, out: &mut Vec<usize>) {
    match marker_back(expr) {
        Some(back) => out.push(back),
        None => children(expr).into_iter().for_each(|c| backs(c, out)),
    }
}

/// Whether every name `expr` reads is read from a row above (inside a
/// marker). A name read in the row itself (`amount` in `PREV(balance, 0) +
/// amount`) would be read a row up too if the part became one `PREV`, and so
/// would `ROW()`.
fn reads_only_above(expr: &Expr) -> bool {
    if marker_back(expr).is_some() {
        return true;
    }
    match expr {
        Expr::Reference(_) => false,
        Expr::FunctionCall { name, .. } if name == "ROW" => false,
        _ => children(expr).into_iter().all(reads_only_above),
    }
}

/// `expr` with every marker replaced by the column it reads.
fn unmarked(expr: &Expr) -> Expr {
    if marker_back(expr).is_some() {
        if let Expr::FunctionCall { args, .. } = expr {
            return args[0].clone();
        }
    }
    rebuilt(expr, children(expr).into_iter().map(unmarked).collect())
}

/// `expr` with its children replaced, in order.
fn rebuilt(expr: &Expr, mut new: Vec<Expr>) -> Expr {
    let mut next = || Box::new(new.remove(0));
    match expr {
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name: name.clone(),
            args: (0..args.len()).map(|_| *next()).collect(),
        },
        Expr::CallResult { args, .. } => Expr::CallResult {
            callable: next(),
            args: (0..args.len()).map(|_| *next()).collect(),
        },
        Expr::BinaryOp { op, .. } => Expr::BinaryOp {
            op: op.clone(),
            left: next(),
            right: next(),
        },
        Expr::UnaryOp { op, .. } => Expr::UnaryOp {
            op: op.clone(),
            operand: next(),
        },
        Expr::ArrayIndex { .. } => Expr::ArrayIndex {
            array: next(),
            index: next(),
        },
        Expr::Range { .. } => Expr::Range {
            start: next(),
            end: next(),
        },
        leaf => leaf.clone(),
    }
}

/// Whether two nodes are the same apart from their children.
fn same_node(one: &Expr, other: &Expr) -> bool {
    match (one, other) {
        (
            Expr::FunctionCall { name, args },
            Expr::FunctionCall {
                name: other_name,
                args: other_args,
            },
        ) => name == other_name && args.len() == other_args.len(),
        (
            Expr::CallResult { args, .. },
            Expr::CallResult {
                args: other_args, ..
            },
        ) => args.len() == other_args.len(),
        (Expr::BinaryOp { op, .. }, Expr::BinaryOp { op: other_op, .. })
        | (Expr::UnaryOp { op, .. }, Expr::UnaryOp { op: other_op, .. }) => op == other_op,
        (Expr::ArrayIndex { .. }, Expr::ArrayIndex { .. })
        | (Expr::Range { .. }, Expr::Range { .. }) => true,
        _ => false,
    }
}

/// Line up a later row (`rest`) with the first row (`first`).
///
/// With `row` `None`, the Forge formula: each part of `rest` that reads the
/// rows above becomes `PREV(that part, the first row's part, back)`. With
/// `Some(row)`, the formula that the column gives in that row (from 1), with
/// a marker for each cell it reads from a row above, as the row is read back.
fn line_up(rest: &Expr, first: &Expr, row: Option<usize>) -> Option<Expr> {
    let mut found = Vec::new();
    backs(rest, &mut found);
    if found.is_empty() {
        return (rest == first).then(|| rest.clone());
    }
    // The same calculation in both, part by part
    if marker_back(rest).is_none() && same_node(rest, first) {
        let parts: Option<Vec<Expr>> = children(rest)
            .into_iter()
            .zip(children(first))
            .map(|(r, f)| line_up(r, f, row))
            .collect();
        if let Some(parts) = parts {
            return Some(rebuilt(rest, parts));
        }
    }
    // This whole part is one PREV: every cell it reads is the same rows up
    let back = found[0];
    if found.iter().any(|b| *b != back) || !reads_only_above(rest) {
        return None;
    }
    Some(match row {
        None => {
            let mut args = vec![unmarked(rest), first.clone()];
            if back > 1 {
                #[allow(clippy::cast_precision_loss)] // a row count
                args.push(Expr::Number(back as f64));
            }
            Expr::FunctionCall {
                name: "PREV".to_string(),
                args,
            }
        },
        Some(row) if row <= back => first.clone(),
        Some(_) => rest.clone(),
    })
}

/// The Forge formula of a column whose rows, read back with markers for the
/// rows above, are `rows` (the first data row first), when some of them
/// differ: `Ok` with the formula, or `Err` with the index of the first row
/// that is not what any one `PREV` formula gives.
pub(super) fn column_formula(rows: &[String]) -> Result<String, usize> {
    let exprs: Vec<Expr> = rows
        .iter()
        .map(|f| parsed(f))
        .collect::<Option<_>>()
        .ok_or(1usize)?;
    // The later rows, which all read the rows above in the same way
    let start = (1..exprs.len())
        .find(|&k| exprs[k..].iter().all(|e| *e == exprs[k]))
        .ok_or(1usize)?;
    let (first, rest) = (&exprs[0], &exprs[start]);
    let first_wrong = || {
        (2..exprs.len())
            .find(|&i| exprs[i] != exprs[1])
            .unwrap_or(1)
    };
    let formula = line_up(rest, first, None).ok_or_else(first_wrong)?;
    // Kept only if it gives each of the first rows as the workbook has it
    for (i, expr) in exprs.iter().enumerate().take(start) {
        if line_up(rest, first, Some(i + 1)).as_ref() != Some(expr) {
            return Err(first_wrong());
        }
    }
    Ok(format!(
        "={}",
        super::workbook_refs::dates::formula_of(&formula)
    ))
}

#[cfg(test)]
mod tests {
    use super::column_formula;

    fn rows(rows: &[&str]) -> Result<String, usize> {
        column_formula(&rows.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn a_running_balance_is_one_prev() {
        assert_eq!(
            rows(&[
                "=(loan.principal) + x",
                "=(PREV(closing, 0)) + x",
                "=(PREV(closing, 0)) + x"
            ]),
            Ok("=PREV(closing, loan.principal) + x".into())
        );
    }

    #[test]
    fn two_rows_back() {
        assert_eq!(
            rows(&["=(0)", "=(0)", "=(PREV(x, 0, 2))", "=(PREV(x, 0, 2))"]),
            Ok("=PREV(x, 0, 2)".into())
        );
        assert_eq!(
            rows(&[
                "=(1) + (5)",
                "=(PREV(x, 0)) + (5)",
                "=(PREV(x, 0)) + (PREV(x, 0, 2))"
            ]),
            Ok("=PREV(x, 1) + PREV(x, 5, 2)".into())
        );
    }

    #[test]
    fn a_calculated_value_read_from_the_row_above() {
        assert_eq!(
            rows(&["=(0) * 2", "=(PREV(closing, 0) - PREV(x, 0)) * 2"]),
            Ok("=PREV(closing - x, 0) * 2".into())
        );
    }

    #[test]
    fn rows_that_no_prev_gives_are_refused() {
        // The second row is neither the first value nor the row above
        assert_eq!(
            rows(&["=(0)", "=(7)", "=(PREV(x, 0, 2))", "=(PREV(x, 0, 2))"]),
            Err(2)
        );
        assert!(rows(&["=1", "=2", "=3"]).is_err());
        // `amount` is read in the row itself, so this is not one PREV
        assert!(rows(&["=amount", "=PREV(balance, 0) + amount"]).is_err());
        assert!(rows(&["=0", "=PREV(balance, 0) * ROW()"]).is_err());
    }
}
