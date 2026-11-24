//! Conditional aggregation functions: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, etc.
//!
//! This module provides Excel-compatible conditional aggregation functions:
//!
//! - **Single criteria**: SUMIF, COUNTIF, AVERAGEIF
//! - **Multiple criteria**: SUMIFS, COUNTIFS, AVERAGEIFS
//! - **Min/Max with criteria**: MINIFS, MAXIFS
//!
//! All functions support comparison operators in criteria strings:
//! `>`, `<`, `>=`, `<=`, `<>`, `=`

mod averageif;
mod countif;
mod minmax;
mod sumif;

use super::{collect_values_as_vec, evaluate, Criterion, EvalContext, EvalError, Expr, Value};

/// Which rows meet every criterion of a `*IFS` function, from its pairs of
/// criteria range and criterion. Every range has `rows` values, as Excel,
/// `LibreOffice` and Gnumeric require: a range of another size is `#VALUE!`
/// (Forge counted the rows past a shorter criteria range as matching)
fn matching_rows(
    name: &str,
    rows: usize,
    pairs: &[Expr],
    ctx: &EvalContext,
) -> Result<Vec<bool>, EvalError> {
    let mut matches = vec![true; rows];
    for (n, pair) in pairs.chunks(2).enumerate() {
        let range = collect_values_as_vec(&pair[0], ctx)?;
        if range.len() != rows {
            return Err(EvalError::new(format!(
                "#VALUE!: {name}: criteria range {} is of size {}, not {rows}",
                n + 1,
                range.len()
            )));
        }
        let criterion = Criterion::new(&evaluate(&pair[1], ctx)?);
        for (matched, value) in matches.iter_mut().zip(range.iter()) {
            *matched &= criterion.matches(value);
        }
    }
    Ok(matches)
}

/// Try to evaluate a conditional aggregation function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SUMIF" => sumif::eval_sumif(args, ctx)?,
        "SUMIFS" => sumif::eval_sumifs(args, ctx)?,

        "COUNTIF" => countif::eval_countif(args, ctx)?,
        "COUNTIFS" => countif::eval_countifs(args, ctx)?,

        "AVERAGEIF" => averageif::eval_averageif(args, ctx)?,
        "AVERAGEIFS" => averageif::eval_averageifs(args, ctx)?,

        "MAXIFS" => minmax::eval_maxifs(args, ctx)?,
        "MINIFS" => minmax::eval_minifs(args, ctx)?,

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::evaluator::tests as h;

    /// Every range of a `*IFS` function has the size of the first: Excel's
    /// help asks for the same size and shape, `LibreOffice`'s
    /// `IterateParametersIfs` gives Err:502 and Gnumeric's
    /// `gnm_criteria_ifs_func` #VALUE! for one of another size. Forge counted
    /// the rows past a shorter criteria range as matching: 60 for the SUMIFS
    /// below, where the only row both criteria see is 1, whose value is 10.
    #[test]
    fn ranges_of_another_size_are_value_errors() {
        let mut both = h::table(&[
            ("k", h::numbers(&[1.0, 2.0, 3.0])),
            ("v", h::numbers(&[10.0, 20.0, 30.0])),
        ]);
        let short = h::table(&[("j", h::numbers(&[1.0]))]);
        both.tables.extend(
            short
                .tables
                .into_values()
                .map(|columns| ("s".to_string(), columns)),
        );
        for (formula, message) in [
            (
                "SUMIFS(t.v, t.k, \">0\", s.j, \">0\")",
                "Eval error: #VALUE!: SUMIFS: criteria range 2 is of size 1, not 3",
            ),
            (
                "AVERAGEIFS(t.v, s.j, \">0\")",
                "Eval error: #VALUE!: AVERAGEIFS: criteria range 1 is of size 1, not 3",
            ),
            (
                "COUNTIFS(t.k, \">0\", s.j, \">0\")",
                "Eval error: #VALUE!: COUNTIFS: criteria range 2 is of size 1, not 3",
            ),
            (
                "MAXIFS(t.v, s.j, \">0\")",
                "Eval error: #VALUE!: MAXIFS: criteria range 1 is of size 1, not 3",
            ),
            (
                "MINIFS(s.j, t.k, \">0\")",
                "Eval error: #VALUE!: MINIFS: criteria range 1 is of size 3, not 1",
            ),
        ] {
            assert_eq!(h::error_in(formula, &both), message, "{formula}");
        }
        // Ranges of one size still match row by row: 20 + 30
        h::is_exactly_in("SUMIFS(t.v, t.k, \">1\", t.v, \"<40\")", &both, 50.0);
    }
}
