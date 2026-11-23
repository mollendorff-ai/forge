//! `PREV` and `ROW()` in the exported workbook (ADR-053).
//!
//! A row formula is written once per row, so `PREV(closing, loan.principal)`
//! is simply another cell in each row: the value of `first` in the rows that
//! have no row far enough up, and `value` read in the earlier row in every
//! other row. In Excel the column reads the cell above, as a hand-built
//! schedule does.
//!
//! Forge's `ROW()` is the number of the table's row, from 1; Excel's is the
//! sheet's row, and the table starts in row 2. So `ROW()` is written as
//! `(ROW() - 1)`, less the rows back when it is inside a `PREV`, and an Excel
//! `ROW()` is read back as `(ROW() + 1)`.

use super::local_names::Locals;
use super::{matching_paren, split_arguments, Context, Token, WorkbookRefs};
use num_traits::ToPrimitive;

impl WorkbookRefs {
    /// Translate `PREV(value, first, [back])` whose name is `tokens[at]`.
    /// Returns the Excel text and the index of the first token after it.
    pub(super) fn translate_prev(
        &self,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        calls: &[super::Call],
        locals: &Locals,
        has_range: &mut bool,
    ) -> Result<(String, usize), String> {
        let close = matching_paren(tokens, at + 1).ok_or("PREV: missing )")?;
        let args = split_arguments(&tokens[at + 2..close]);
        if !(2..=3).contains(&args.len()) || args.iter().any(|arg| arg.is_empty()) {
            return Err("PREV takes a value, what to use in the first row, and optionally how many rows back".into());
        }
        let Context::Row { table, row } = ctx else {
            return Err("PREV works only in a row formula".into());
        };
        // A whole number of rows back, from 1 to 999,999
        let rows_back = |arg: &[Token]| match arg {
            [Token::Number(n)] if n.fract() == 0.0 => {
                n.to_u32().filter(|b| (1..1_000_000).contains(b))
            },
            _ => None,
        };
        let back = match args.get(2).map(|arg| rows_back(arg)) {
            None => 1,
            Some(Some(back)) => back,
            Some(None) => {
                return Err(
                    "PREV with a calculated number of rows back has no Excel form Forge writes"
                        .into(),
                )
            },
        };
        // Both are translated in every row, so a part Excel cannot express is
        // found in the first row, where the exporter decides how to write
        // the column
        let first = self.translate(args[1], ctx, calls, locals, has_range)?;
        let has_row_above = row >= 2 + back;
        let earlier = Context::Row {
            table,
            row: if has_row_above { row - back } else { row },
        };
        let value = self.translate(args[0], earlier, calls, &locals.lagged(back), has_range)?;
        let text = if has_row_above { value } else { first };
        Ok((format!("({text})"), close + 1))
    }

    /// Translate `ROW()` whose name is `tokens[at]`: the table's row number.
    pub(super) fn translate_row(
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        locals: &Locals,
    ) -> Result<(String, usize), String> {
        if !matches!(tokens.get(at + 2), Some(Token::CloseParen)) {
            return Err("ROW of a reference has no Excel equivalent".into());
        }
        if !matches!(ctx, Context::Row { .. }) {
            return Err("ROW() outside a row formula has no Excel equivalent".into());
        }
        Ok((format!("(ROW() - {})", 1 + locals.lag()), at + 3))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ReverseRefs, SheetKind};
    use super::*;
    use std::collections::HashMap;

    /// A table `t` (x, closing, opening in columns A, B, C) and a scalar `loan.principal`.
    fn refs() -> WorkbookRefs {
        let columns = ["x", "closing", "opening"].map(String::from).to_vec();
        WorkbookRefs::new([("t", columns, 4)], ["loan.principal"])
    }

    fn row(formula: &str, row: u32) -> Result<String, String> {
        refs()
            .to_excel(formula, Context::Row { table: "t", row })
            .map(|t| t.formula)
    }

    #[test]
    fn the_rows_back_are_a_whole_number_from_1_to_999999() {
        assert!(row("=PREV(closing, 0, 999999)", 2).is_ok());
        assert!(row("=PREV(closing, 0, 1)", 3).is_ok());
        for back in ["1000000", "0", "2.5", "-1", "4294967297"] {
            let err = row(&format!("=PREV(closing, 0, {back})"), 2).unwrap_err();
            assert!(
                err.contains("calculated number of rows back"),
                "{back}: {err}"
            );
        }
    }

    #[test]
    fn prev_without_its_closing_paren_is_refused() {
        assert_eq!(row("=PREV(closing, 0", 2).unwrap_err(), "PREV: missing )");
    }

    #[test]
    fn prev_is_the_first_value_then_the_cell_above() {
        let formula = "=PREV(closing, loan.principal) + x";
        assert_eq!(row(formula, 2).unwrap(), "=('loan'!$B$2) + A2");
        assert_eq!(row(formula, 3).unwrap(), "=(B2) + A3");
        assert_eq!(row(formula, 5).unwrap(), "=(B4) + A5");
        // An expression is read in the row above as a whole
        assert_eq!(
            row("=PREV(closing * 2 - x, 0)", 4).unwrap(),
            "=(B3 * 2 - A3)"
        );
        // Two rows back: the first two rows have no such row
        assert_eq!(row("=PREV(x, -1, 2)", 3).unwrap(), "=(-1)");
        assert_eq!(row("=PREV(x, -1, 2)", 4).unwrap(), "=(A2)");
        assert_eq!(row("=PREV(PREV(x, 7), 9)", 4).unwrap(), "=((A2))");
        assert_eq!(row("=PREV(PREV(x, 7), 9)", 3).unwrap(), "=((7))");
    }

    #[test]
    fn row_is_the_number_of_the_table_row() {
        assert_eq!(row("=ROW() * x", 2).unwrap(), "=(ROW() - 1) * A2");
        assert_eq!(row("=PREV(ROW(), 0)", 3).unwrap(), "=((ROW() - 2))");
        assert!(row("=ROW(x)", 2)
            .unwrap_err()
            .contains("no Excel equivalent"));
        let scalar = refs().to_excel("=ROW()", Context::Scalar { group: None });
        assert!(scalar.unwrap_err().contains("outside a row formula"));
    }

    #[test]
    fn what_prev_cannot_be_written_as() {
        for (formula, why) in [
            ("=PREV(x)", "PREV takes a value"),
            ("=PREV(x, 0, x)", "calculated number of rows back"),
            ("=PREV(x, 0, 0)", "calculated number of rows back"),
            // A part Excel cannot express is found in every row
            ("=PREV(NOW(), 0)", "no Excel equivalent"),
            ("=PREV(x, NOW())", "no Excel equivalent"),
        ] {
            for at in [2, 3] {
                assert!(
                    row(formula, at).unwrap_err().contains(why),
                    "{formula} in row {at}"
                );
            }
        }
        let scalar = refs().to_excel("=PREV(1, 2)", Context::Scalar { group: None });
        assert!(scalar.unwrap_err().contains("only in a row formula"));
    }

    #[test]
    fn import_reads_the_row_above_and_excels_row() {
        let letters = [("A", "x"), ("B", "closing"), ("C", "opening")];
        let back = ReverseRefs {
            sheets: HashMap::from([(
                "t".to_string(),
                SheetKind::Table {
                    name: "t".to_string(),
                    columns: letters
                        .iter()
                        .map(|(l, n)| ((*l).to_string(), (*n).to_string()))
                        .collect(),
                    include: None,
                    rows: (2..=5).collect(),
                    ends: HashMap::new(),
                    total: None,
                },
            )]),
            names: HashMap::new(),
        };
        assert_eq!(
            back.to_forge("=(ROW() - 1) * A3", "t", 3).unwrap(),
            "=ROW() * x"
        );
        assert_eq!(
            back.to_forge("=ROW() * A3", "t", 3).unwrap(),
            "=(ROW() + 1) * x"
        );
        assert!(back
            .to_forge("=B2 + A3", "t", 3)
            .unwrap_err()
            .contains("another row"));
        assert_eq!(
            back.to_forge_reading_above("=B2 + A3", "t", 3).unwrap(),
            "=PREV(closing, 0) + x"
        );
        // Two data rows up, and the header row, which is not data
        assert_eq!(
            back.to_forge_reading_above("=B2 + A4", "t", 4).unwrap(),
            "=PREV(closing, 0, 2) + x"
        );
        assert!(back.to_forge_reading_above("=B1 + A3", "t", 3).is_err());
        for different in [
            "=COLUMN()",
            "=ROW(A3)",
            "=VLOOKUP(1, A2:A5, 1)",
            "=NOW()",
            "=UNIQUE(A2:A5)",
        ] {
            let err = back.to_forge(different, "t", 3).unwrap_err();
            assert!(
                err.contains("works differently in Forge"),
                "{different}: {err}"
            );
        }
    }
}
