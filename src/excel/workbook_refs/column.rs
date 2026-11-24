//! `COLUMN` written for Excel, where it means what it means in Forge: a
//! column's number as the export lays its table out, A being 1. Forge
//! numbers a table's columns in model order, the order of the letters
//! [`WorkbookRefs::columns`] gives them, so `COLUMN()` in a row formula is
//! Excel's `COLUMN()` of the cell, and `COLUMN(t.b)` is Excel's `COLUMN` of
//! `b`'s header cell. What Forge refuses (`COLUMN()` in a scalar, `COLUMN`
//! of a scalar, of an expression or of an included file's table) has no
//! Excel equivalent.

use super::{matching_paren, sheet_prefix, split_arguments, Context, Locals, WorkbookRefs};
use crate::core::array_calculator::tokenizer::Token;

impl WorkbookRefs {
    /// Translate `COLUMN(...)` whose name is `tokens[at]`: its Excel text,
    /// and the index of the token after its `)`.
    pub(super) fn translate_column(
        &self,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
        locals: &Locals,
    ) -> Result<(String, usize), String> {
        let close = matching_paren(tokens, at + 1).ok_or("COLUMN: missing )")?;
        let args = split_arguments(&tokens[at + 2..close]);
        let text = match (args.as_slice(), ctx) {
            ([], Context::Row { .. }) => "COLUMN()".to_string(),
            ([], Context::Scalar { .. }) => {
                return Err("COLUMN() outside a row formula has no Excel equivalent".into());
            },
            ([[Token::Identifier(word)]], _) if locals.get(word).is_none() => {
                let header = self
                    .column_header(word, ctx)
                    .ok_or("COLUMN of what is not a table column has no Excel equivalent")?;
                format!("COLUMN({header})")
            },
            _ => return Err("COLUMN of what is not a table column has no Excel equivalent".into()),
        };
        Ok((text, close + 1))
    }

    /// The header cell of the table column `word` names, as Forge reads the
    /// name: a column of the row formula's table, else `table.column` of a
    /// table of the model, unless a scalar has that name. An included file's
    /// table is laid out on its own sheet, and Forge gives it no number.
    fn column_header(&self, word: &str, ctx: Context<'_>) -> Option<String> {
        if let Context::Row { table, .. } = ctx {
            if let Some(letter) = self.columns.get(table).and_then(|c| c.get(word)) {
                return Some(format!("${letter}$1"));
            }
        }
        if word.starts_with('@') || self.scalars.contains_key(word) {
            return None;
        }
        let (table, column) = word.split_once('.')?;
        let letter = self.columns.get(table)?.get(column)?;
        Some(format!("{}${letter}$1", sheet_prefix(self.sheet_of(table))))
    }
}

#[cfg(test)]
mod tests {
    use super::super::Context;
    use super::*;

    /// Table `t` (x, y, z in columns A to C), table `u` (p, q in A and B) and
    /// a scalar `s.n`.
    fn refs() -> WorkbookRefs {
        let t = ["x", "y", "z"].map(String::from).to_vec();
        let u = ["p", "q"].map(String::from).to_vec();
        WorkbookRefs::new([("t", t, 3), ("u", u, 3)], ["s.n"])
    }

    fn row(formula: &str) -> Result<String, String> {
        refs()
            .to_excel(formula, Context::Row { table: "t", row: 2 })
            .map(|t| t.formula)
    }

    fn scalar(formula: &str) -> Result<String, String> {
        refs()
            .to_excel(formula, Context::Scalar { group: Some("s") })
            .map(|t| t.formula)
    }

    /// The cell's own column, and the header cell of the column named: Excel
    /// numbers them as Forge does, since the letters are Forge's order
    #[test]
    fn column_is_written_as_the_column_excel_numbers_the_same() {
        assert_eq!(row("=COLUMN() * 10").unwrap(), "=COLUMN() * 10");
        assert_eq!(row("=COLUMN(z)").unwrap(), "=COLUMN($C$1)");
        assert_eq!(row("=COLUMN(t.y) + x").unwrap(), "=COLUMN('t'!$B$1) + A2");
        assert_eq!(row("=COLUMN(u.q)").unwrap(), "=COLUMN('u'!$B$1)");
        assert_eq!(scalar("=COLUMN(t.z)").unwrap(), "=COLUMN('t'!$C$1)");
        assert_eq!(
            scalar("=SUM(COLUMN(u.p), 1)").unwrap(),
            "=SUM(COLUMN('u'!$A$1), 1)"
        );
    }

    /// What Forge refuses, or numbers otherwise, is not written
    #[test]
    fn what_has_no_column_number_in_forge_is_refused() {
        let refused = "has no Excel equivalent";
        for f in [
            "=COLUMN(s.n)",
            "=COLUMN(n)",
            "=COLUMN(1 + 2)",
            "=COLUMN(x, y)",
            "=COLUMN(t.nope)",
            "=COLUMN(nope.x)",
            "=COLUMN(p)",
        ] {
            assert!(row(f).unwrap_err().contains(refused), "{f}");
        }
        assert!(scalar("=COLUMN()")
            .unwrap_err()
            .contains("outside a row formula"));
        assert!(scalar("=COLUMN(z)").unwrap_err().contains(refused));
        assert_eq!(row("=COLUMN(x").unwrap_err(), "COLUMN: missing )");
        // A LET name is not a column, even where a column has its name
        assert!(row("=LET(x, 1, COLUMN(x))").unwrap_err().contains(refused));
    }

    /// An included file's table is on its own sheet and has no number in Forge
    #[test]
    fn an_included_tables_column_is_refused() {
        let mut refs = refs();
        refs.add_include_tables(
            "costs",
            [("prices".to_string(), vec!["flour".to_string()], 2)],
        );
        let err = refs
            .to_excel(
                "=COLUMN(@costs.prices.flour)",
                Context::Row { table: "t", row: 2 },
            )
            .unwrap_err();
        assert!(err.contains("has no Excel equivalent"), "{err}");
    }
}
