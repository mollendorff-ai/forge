//! `LET`, a `LAMBDA` called where it is written, and `COUNTUNIQUE`, written
//! as Excel formulas that compute what Forge computes.
//!
//! - `LET(x, 10, x * x)` is `LET(_xlpm.x, 10, _xlpm.x * _xlpm.x)`: Excel
//!   stores the names of a LET with the prefix `_xlpm.`.
//! - `LAMBDA(x, y, x * y)(6, 7)` is written as `LET(_xlpm.x, 6, _xlpm.y, 7,
//!   _xlpm.x * _xlpm.y)`. The two mean the same: the values are worked out
//!   first, outside the LAMBDA, then the body with the names bound to them.
//!   (A value never sees a parameter bound before it, because every name
//!   gets an Excel name no other name of the formula has.) Excel 2021 and
//!   `LibreOffice` 24.8 have LET; only Excel 365 has LAMBDA, and no engine
//!   Forge can check against.
//! - `COUNTUNIQUE(t.c)` counts distinct values as Forge does: exact text for a
//!   text column (`EXACT`, since Excel's `UNIQUE` ignores case), the number
//!   for a number column (`ROWS(UNIQUE(..))`).
//!
//! A LET or LAMBDA call that is a whole value, calculation or body of another
//! is merged into it: `LET(x, LET(a, 5, a * 2), x + 1)` is written `LET(a, 5,
//! x, a * 2, x + 1)`. That is the same calculation, since every name is
//! unique in the formula, and `LibreOffice` (26.2) gives #NAME? for a LET inside
//! a LET, so the merged form is the one an engine can check.
//!
//! A name Excel would refuse (`r`, `c`, `x1`, `r2c3`) or one already used by
//! another LET or LAMBDA of the formula, or one that is also the name of a
//! value of the model the formula reads (which `forge import` would read back
//! as the local name), gets `_` (then `_2`, `_3`...) added in Excel.

use super::{matching_paren, split_arguments, Context, Token, WorkbookRefs};
use std::cell::RefCell;
use std::rc::Rc;

/// The functions this module writes.
pub(super) const HANDLED: [&str; 3] = ["LET", "LAMBDA", "COUNTUNIQUE"];

/// Names bound by the enclosing LETs and LAMBDAs: (Forge name, Excel name),
/// innermost last, and the Excel names of every LET and LAMBDA of the formula
/// so far, which a new one may not reuse.
#[derive(Debug, Clone, Default)]
pub(super) struct Locals {
    visible: Vec<(String, String)>,
    taken: Rc<RefCell<Vec<String>>>,
}

impl Locals {
    /// No names bound yet; none may take one of `names`.
    pub(super) fn avoiding<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let taken = names.into_iter().map(str::to_ascii_lowercase).collect();
        Self {
            visible: Vec::new(),
            taken: Rc::new(RefCell::new(taken)),
        }
    }

    /// The Excel name of a bound name, the innermost binding first.
    pub(super) fn get(&self, name: &str) -> Option<&str> {
        self.visible
            .iter()
            .rev()
            .find(|(forge, _)| forge == name)
            .map(|(_, excel)| excel.as_str())
    }

    /// Bind `name`, giving it an Excel name Excel accepts and that no other
    /// LET or LAMBDA of the formula has (Excel names ignore case).
    fn bind(&mut self, name: &str) -> String {
        let base = if name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            name
        } else {
            "v"
        };
        let mut taken = self.taken.borrow_mut();
        let excel = [base.to_string(), format!("{base}_")]
            .into_iter()
            .chain((2..u32::MAX).map(|k| format!("{base}_{k}")))
            .find(|c| excel_accepts(c) && !taken.contains(&c.to_ascii_lowercase()))
            .unwrap_or_else(|| base.to_string());
        taken.push(excel.to_ascii_lowercase());
        self.visible.push((name.to_string(), excel.clone()));
        excel
    }
}

/// A LET in Excel: `_xlpm.name, value` pairs, then the calculation.
struct LetForm {
    pairs: Vec<String>,
    calculation: String,
}

impl LetForm {
    fn text(self) -> String {
        if self.pairs.is_empty() {
            format!("({})", self.calculation)
        } else {
            format!("LET({}, {})", self.pairs.join(", "), self.calculation)
        }
    }
}

/// Where the LET or LAMBDA call starting at `tokens[at]` ends (the index after
/// it), if it is one.
fn let_end(tokens: &[Token], at: usize) -> Option<usize> {
    let Some(Token::Identifier(word)) = tokens.get(at) else {
        return None;
    };
    let close = matching_paren(tokens, at + 1)?;
    match word.to_uppercase().as_str() {
        "LET" => Some(close + 1),
        "LAMBDA" if matches!(tokens.get(close + 1), Some(Token::OpenParen)) => {
            matching_paren(tokens, close + 1).map(|end| end + 1)
        },
        _ => None,
    }
}

/// Whether Excel accepts `name` as the name of a LET or LAMBDA: it starts with
/// a letter or `_`, and is not `r`, `c`, `TRUE`, `FALSE` or a cell reference
/// in A1 style (`x1`, `abc12`, refused up to three letters even past column
/// XFD) or R1C1 style (`r2`, `c3`, `r1c1`).
fn excel_accepts(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if ["r", "c", "true", "false"].contains(&lower.as_str()) {
        return false;
    }
    if !lower.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        || !lower
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return false;
    }
    let letters = lower.chars().take_while(char::is_ascii_alphabetic).count();
    let digits = &lower[letters..];
    let a1 = (1..=3).contains(&letters)
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit());
    !a1 && !is_r1c1(&lower)
}

/// `r`, `c`, `r5`, `c5`, `rc`, `r1c1`... (lower case).
fn is_r1c1(lower: &str) -> bool {
    let mut rest = lower;
    let mut any = false;
    for axis in ['r', 'c'] {
        if let Some(after) = rest.strip_prefix(axis) {
            rest = after.trim_start_matches(|c: char| c.is_ascii_digit());
            any = true;
        }
    }
    any && rest.is_empty()
}

/// The name in a LET or LAMBDA name position.
fn single_name<'t>(tokens: &'t [Token], function: &str) -> Result<&'t str, String> {
    match tokens {
        [Token::Identifier(name)] => Ok(name),
        _ => Err(format!("{function}: a name must be a single word")),
    }
}

impl WorkbookRefs {
    /// Translate the call of one of [`HANDLED`] whose name is `tokens[at]`.
    /// Returns the Excel text and the index of the first token after it.
    pub(super) fn translate_special(
        &self,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        calls: &[(Option<String>, usize)],
        locals: &Locals,
        has_range: &mut bool,
    ) -> Result<(String, usize), String> {
        let Some(Token::Identifier(word)) = tokens.get(at) else {
            return Err("expected a function name".to_string());
        };
        if word.eq_ignore_ascii_case("COUNTUNIQUE") {
            return self.count_unique(tokens, at, ctx, has_range, locals);
        }
        let (form, end) = self.let_form(tokens, at, ctx, calls, locals, has_range)?;
        Ok((form.text(), end))
    }

    /// Translate `arg`, merging it into `pairs` when it is a whole LET or
    /// LAMBDA call: its pairs are added and its calculation returned.
    #[allow(clippy::too_many_arguments)] // the translator's state, passed down
    fn merged(
        &self,
        arg: &[Token],
        pairs: &mut Vec<String>,
        ctx: Context<'_>,
        calls: &[(Option<String>, usize)],
        scope: &Locals,
        has_range: &mut bool,
    ) -> Result<String, String> {
        if arg.is_empty() {
            return Err("an argument is empty".to_string());
        }
        if let_end(arg, 0) == Some(arg.len()) {
            let (inner, _) = self.let_form(arg, 0, ctx, calls, scope, has_range)?;
            pairs.extend(inner.pairs);
            return Ok(inner.calculation);
        }
        self.translate(arg, ctx, calls, scope, has_range)
    }

    /// `LET(...)` or `LAMBDA(...)(...)` at `tokens[at]`, as one Excel LET.
    fn let_form(
        &self,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        calls: &[(Option<String>, usize)],
        locals: &Locals,
        has_range: &mut bool,
    ) -> Result<(LetForm, usize), String> {
        let Some(Token::Identifier(word)) = tokens.get(at) else {
            return Err("expected a function name".to_string());
        };
        let close = matching_paren(tokens, at + 1).ok_or_else(|| format!("{word}: missing )"))?;
        let inner = &tokens[at + 2..close];
        let mut scope = locals.clone();
        let mut pairs = Vec::new();
        if word.eq_ignore_ascii_case("LET") {
            let args = split_arguments(inner);
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err("LET takes pairs of a name and a value, then a calculation".into());
            }
            let (calculation, bindings) = args.split_last().unwrap_or((&inner, &[]));
            for pair in bindings.chunks(2) {
                let name = single_name(pair[0], "LET")?;
                // A value sees the names bound before it
                let value = self.merged(pair[1], &mut pairs, ctx, calls, &scope, has_range)?;
                pairs.push(format!("_xlpm.{}", scope.bind(name)));
                pairs.push(value);
            }
            let calculation =
                self.merged(calculation, &mut pairs, ctx, calls, &scope, has_range)?;
            return Ok((LetForm { pairs, calculation }, close + 1));
        }
        // LAMBDA(params..., body)(values...)
        if !matches!(tokens.get(close + 1), Some(Token::OpenParen)) {
            return Err(
                "a LAMBDA that is not called where it is written has no Excel form Forge writes"
                    .into(),
            );
        }
        let end = matching_paren(tokens, close + 1).ok_or("LAMBDA call: missing )")?;
        if matches!(tokens.get(end + 1), Some(Token::OpenParen)) {
            return Err(
                "calling the result of a LAMBDA call has no Excel form Forge writes".into(),
            );
        }
        let definition = split_arguments(inner);
        let Some((body, params)) = definition.split_last() else {
            return Err("LAMBDA needs a body".into());
        };
        let given = &tokens[close + 2..end];
        let args = if given.is_empty() {
            Vec::new()
        } else {
            split_arguments(given)
        };
        if args.len() != params.len() {
            return Err(format!(
                "LAMBDA has {} parameters and is called with {} values",
                params.len(),
                args.len()
            ));
        }
        // The values are worked out outside the LAMBDA, before any parameter
        // is bound
        let values = args
            .iter()
            .map(|arg| {
                if arg.is_empty() {
                    return Err("LAMBDA call: a value is empty".to_string());
                }
                self.translate(arg, ctx, calls, locals, has_range)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (param, value) in params.iter().zip(values) {
            let name = single_name(param, "LAMBDA")?;
            pairs.push(format!("_xlpm.{}", scope.bind(name)));
            pairs.push(value);
        }
        let calculation = self.merged(body, &mut pairs, ctx, calls, &scope, has_range)?;
        Ok((LetForm { pairs, calculation }, end + 1))
    }

    /// `COUNTUNIQUE(column)` in a scalar. A row formula is a plain formula in
    /// each cell, where Excel's TRANSPOSE and UNIQUE need an array formula.
    fn count_unique(
        &self,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        has_range: &mut bool,
        locals: &Locals,
    ) -> Result<(String, usize), String> {
        if matches!(ctx, Context::Row { .. }) {
            return Err("COUNTUNIQUE in a row formula has no Excel form Forge writes".into());
        }
        let close = matching_paren(tokens, at + 1).ok_or("COUNTUNIQUE: missing )")?;
        let column = match &tokens[at + 2..close] {
            [Token::Identifier(name)] if locals.get(name).is_none() => name,
            _ => {
                return Err(
                    "COUNTUNIQUE of anything but one column has no Excel form Forge writes".into(),
                )
            },
        };
        let is_text = self
            .column_key(column, ctx)
            .and_then(|key| self.text_columns.get(&key).copied())
            .ok_or_else(|| format!("COUNTUNIQUE: '{column}' is not a column of numbers or text"))?;
        let mut range = false;
        let r = self.reference(column, ctx, true, &mut range)?;
        if !range {
            return Err(format!("COUNTUNIQUE: '{column}' is not a column"));
        }
        *has_range = true;
        let text = if is_text {
            // Each value counts 1 / (how many values are exactly it)
            format!("SUMPRODUCT(1 / MMULT(--EXACT({r}, TRANSPOSE({r})), ROW({r})^0))")
        } else {
            format!("ROWS(UNIQUE({r}))")
        };
        Ok((text, close + 1))
    }

    /// The `text_columns` key of a column reference, resolved as `reference`
    /// resolves it.
    fn column_key(&self, word: &str, ctx: Context<'_>) -> Option<String> {
        let has = |table: &str, column: &str| {
            self.columns
                .get(table)
                .is_some_and(|c| c.contains_key(column))
        };
        if let Context::Row { table, .. } = ctx {
            if has(table, word) {
                return Some(format!("{table}.{word}"));
            }
        }
        let (table, column) = word.rsplit_once('.')?;
        if word.starts_with('@') || !table.contains('.') {
            return has(table, column).then(|| word.to_string());
        }
        None
    }
}

/// `forge import`: the two Excel forms `COUNTUNIQUE` is exported as, read
/// back as `COUNTUNIQUE` (Forge has no `MMULT`).
pub(super) fn restore_count_unique(formula: &str) -> String {
    let mut out = formula.to_string();
    for (start, middle) in [
        ("SUMPRODUCT(1 / MMULT(--EXACT(", ", TRANSPOSE("),
        ("ROWS(UNIQUE(", "))"),
    ] {
        let mut from = 0;
        while let Some(found) = out[from..].find(start).map(|p| p + from) {
            let rest = &out[found + start.len()..];
            let column = rest.split(middle).next().unwrap_or_default();
            let whole = if middle == "))" {
                format!("{start}{column}))")
            } else {
                format!("{start}{column}, TRANSPOSE({column})), ROW({column}) ^ 0))")
            };
            let is_name = !column.is_empty()
                && column
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '@'));
            if is_name && out[found..].starts_with(&whole) {
                out.replace_range(
                    found..found + whole.len(),
                    &format!("COUNTUNIQUE({column})"),
                );
            }
            from = found + 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{ReverseRefs, SheetKind};
    use super::*;
    use std::collections::HashMap;

    /// A table `t` (text `name`, numbers `n`), a scalar `x` and `g.y`.
    fn refs() -> WorkbookRefs {
        let mut refs = WorkbookRefs::new(
            [("t", vec!["name".to_string(), "n".to_string()], 3)],
            ["x", "g.y"],
        );
        refs.add_column_types("t", [("name", true), ("n", false)]);
        refs
    }

    fn scalar(formula: &str) -> Result<String, String> {
        refs()
            .to_excel(formula, Context::Scalar { group: Some("g") })
            .map(|t| t.formula)
    }

    #[test]
    fn let_names_get_the_xlpm_prefix() {
        assert_eq!(
            scalar("=LET(a, 10, b, a * 2, b + y)").unwrap(),
            "=LET(_xlpm.a, 10, _xlpm.b, _xlpm.a * 2, _xlpm.b + $B$2)"
        );
    }

    #[test]
    fn a_lambda_called_at_once_is_a_let() {
        assert_eq!(
            scalar("=ROUND(LAMBDA(p, q, p / q)(1, 3), 2)").unwrap(),
            "=ROUND(LET(_xlpm.p, 1, _xlpm.q, 3, _xlpm.p / _xlpm.q), 2)"
        );
        assert_eq!(scalar("=LAMBDA(42)()").unwrap(), "=(42)");
        // The values are worked out outside: `x` here is the model's x, so
        // the parameter x gets another Excel name
        assert_eq!(
            scalar("=LAMBDA(x, q, x + q)(1, x)").unwrap(),
            "=LET(_xlpm.x_, 1, _xlpm.q, 'Scalars'!$B$2, _xlpm.x_ + _xlpm.q)"
        );
        for (formula, why) in [
            ("=LAMBDA(a, a * 2)", "not called"),
            (
                "=LAMBDA(a, a * 2)(1, 2)",
                "1 parameters and is called with 2",
            ),
            ("=LAMBDA(a, LAMBDA(b, a + b))(1)(2)", "result of a LAMBDA"),
        ] {
            assert!(scalar(formula).unwrap_err().contains(why), "{formula}");
        }
    }

    #[test]
    fn nested_lets_are_merged_and_names_stay_unique() {
        assert_eq!(
            scalar("=LET(v, LET(a, 5, a * 2), w, 3, v * w)").unwrap(),
            "=LET(_xlpm.a, 5, _xlpm.v, _xlpm.a * 2, _xlpm.w, 3, _xlpm.v * _xlpm.w)"
        );
        assert_eq!(
            scalar("=LAMBDA(k, LET(m, k * 2, m + 1))(10)").unwrap(),
            "=LET(_xlpm.k, 10, _xlpm.m, _xlpm.k * 2, _xlpm.m + 1)"
        );
        assert_eq!(
            scalar("=LET(a, 5, LET(a, 1, a))").unwrap(),
            "=LET(_xlpm.a, 5, _xlpm.a_, 1, _xlpm.a_)"
        );
        // Inside an expression, a LET stays a LET of its own
        assert_eq!(
            scalar("=LET(a, 1, a + LET(b, 2, b))").unwrap(),
            "=LET(_xlpm.a, 1, _xlpm.a + LET(_xlpm.b, 2, _xlpm.b))"
        );
        assert_eq!(
            scalar("=LET(r, 1, c, 2, r + c)").unwrap(),
            "=LET(_xlpm.r_, 1, _xlpm.c_, 2, _xlpm.r_ + _xlpm.c_)"
        );
    }

    #[test]
    fn count_unique_of_a_column() {
        assert_eq!(
            scalar("=COUNTUNIQUE(t.name)").unwrap(),
            "=SUMPRODUCT(1 / MMULT(--EXACT('t'!$A$2:$A$4, TRANSPOSE('t'!$A$2:$A$4)), ROW('t'!$A$2:$A$4)^0))"
        );
        assert_eq!(
            scalar("=COUNTUNIQUE(t.n)").unwrap(),
            "=ROWS(UNIQUE('t'!$B$2:$B$4))"
        );
        assert!(scalar("=COUNTUNIQUE(t.n * 2)")
            .unwrap_err()
            .contains("one column"));
        let row = refs().to_excel("=COUNTUNIQUE(name)", Context::Row { table: "t", row: 2 });
        assert!(row.unwrap_err().contains("row formula"));
    }

    #[test]
    fn import_reads_the_names_back() {
        let back = ReverseRefs {
            sheets: HashMap::from([(
                "g".to_string(),
                SheetKind::Scalars {
                    rows: HashMap::from([(2, "g.y".to_string())]),
                    include: None,
                },
            )]),
        };
        assert_eq!(
            back.to_forge("=_xlfn.LET(_xlpm.a, 10, _xlpm.a * $B$2)", "g", 0)
                .unwrap(),
            "=LET(a, 10, a * g.y)"
        );
    }

    #[test]
    fn excel_refuses_cell_like_names() {
        for bad in [
            "r", "C", "x1", "abc12", "r2", "c3", "R1C1", "rc", "TRUE", "1a", "a-b",
        ] {
            assert!(!excel_accepts(bad), "{bad}");
        }
        for good in [
            "x", "rate", "pmt", "abcd1", "r_", "x1_", "_a", "base", "p", "part",
        ] {
            assert!(excel_accepts(good), "{good}");
        }
    }

    #[test]
    fn a_name_is_renamed_when_excel_refuses_it_or_it_is_taken() {
        let mut locals = Locals::default();
        assert_eq!(locals.bind("x"), "x");
        assert_eq!(locals.bind("X"), "X_");
        assert_eq!(locals.bind("x"), "x_2");
        assert_eq!(locals.bind("r"), "r_");
        assert_eq!(locals.get("x"), Some("x_2"));
        // A sibling scope cannot reuse a name either
        let mut other = Locals {
            visible: Vec::new(),
            taken: locals.taken.clone(),
        };
        assert_eq!(other.bind("x"), "x_3");
    }

    #[test]
    fn count_unique_comes_back_from_both_forms() {
        assert_eq!(
            restore_count_unique(
                "=1 + SUMPRODUCT(1 / MMULT(--EXACT(t.name, TRANSPOSE(t.name)), ROW(t.name) ^ 0))"
            ),
            "=1 + COUNTUNIQUE(t.name)"
        );
        assert_eq!(
            restore_count_unique("=ROWS(UNIQUE(t.n)) * 2"),
            "=COUNTUNIQUE(t.n) * 2"
        );
        // Anything else stays
        assert_eq!(
            restore_count_unique("=ROWS(UNIQUE(t.n + 1))"),
            "=ROWS(UNIQUE(t.n + 1))"
        );
        assert_eq!(
            restore_count_unique(
                "=SUMPRODUCT(1 / MMULT(--EXACT(a.b, TRANSPOSE(a.c)), ROW(a.b) ^ 0))"
            ),
            "=SUMPRODUCT(1 / MMULT(--EXACT(a.b, TRANSPOSE(a.c)), ROW(a.b) ^ 0))"
        );
    }
}
