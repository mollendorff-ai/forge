//! Excel's `COLUMN` read back as Forge's, where the two give the same
//! number: `COLUMN()` in a table's row, and `COLUMN` of the header cell of a
//! table's column, which is what `forge export` writes (`column.rs`). Any
//! other `COLUMN` (of another cell, of a range, in a scalar's cell, or on a
//! sheet whose letters Forge numbers otherwise) is not read.

use super::{Cell, ReverseRefs, SheetKind, XlToken};

impl ReverseRefs {
    /// When `tokens` start with a `COLUMN` call Forge numbers as Excel does,
    /// read on `current` (sheet `sheet`): its Forge text and how many tokens
    /// after the name it takes up.
    pub(super) fn column_back(
        &self,
        current: &SheetKind,
        tokens: &[XlToken],
        sheet: &str,
    ) -> Option<(String, usize)> {
        let [XlToken::Function(function), XlToken::Open, rest @ ..] = tokens else {
            return None;
        };
        // An included file reads no table of the main model
        if !function.eq_ignore_ascii_case("COLUMN") || current.include().is_some() {
            return None;
        }
        match rest {
            [XlToken::Close, ..] => numbered(current).then(|| ("COLUMN()".to_string(), 2)),
            [XlToken::Reference(target, cell, None), XlToken::Close, ..] => {
                let kind = self.sheets.get(target.as_deref().unwrap_or(sheet))?;
                let (table, column) = header_column(kind, cell)?;
                let here = matches!(current, SheetKind::Table { name, include: None, .. } if name == table);
                let name = if here {
                    column.clone()
                } else {
                    format!("{table}.{column}")
                };
                Some((format!("COLUMN({name})"), 3))
            },
            _ => None,
        }
    }
}

/// Whether `kind` is a table of the model with rows whose letters Forge
/// numbers as Excel does. An included file's table is laid out on its own
/// sheet, and the main model gives it no number.
const fn numbered(kind: &SheetKind) -> bool {
    matches!(
        kind,
        SheetKind::Table { include: None, numbered: true, rows, .. } if !rows.is_empty()
    )
}

/// The table and column whose header `cell` is: the row above the table's
/// first data row.
fn header_column<'a>(kind: &'a SheetKind, cell: &Cell) -> Option<(&'a String, &'a String)> {
    let SheetKind::Table {
        name,
        columns,
        rows,
        ..
    } = kind
    else {
        return None;
    };
    // A sheet's first row is 1, so its first data row is at least 2
    let header = rows.first() == Some(&cell.row.saturating_add(1));
    (numbered(kind) && header)
        .then(|| columns.get(&cell.column).map(|column| (name, column)))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::workbook_refs::{Context, WorkbookRefs};
    use std::collections::HashMap;

    /// A table as `forge export` lays it out: header in row 1, data rows 2
    /// to 4, columns from A in the order of `names`
    fn table(name: &str, names: &[&str]) -> SheetKind {
        table_of(name, names, (vec![2, 3, 4], None, true))
    }

    /// A table with its own data rows, namespace and whether it is numbered
    fn table_of(
        name: &str,
        names: &[&str],
        (rows, include, numbered): (Vec<u32>, Option<&str>, bool),
    ) -> SheetKind {
        SheetKind::Table {
            name: name.to_string(),
            columns: names
                .iter()
                .zip(["A", "B", "C"])
                .map(|(n, l)| (l.to_string(), (*n).to_string()))
                .collect(),
            include: include.map(str::to_string),
            rows,
            ends: HashMap::new(),
            total: None,
            numbered,
        }
    }

    /// Table `t` (x, y, z), table `u` (p, q) and the scalar sheet `s`, both
    /// ways round
    fn layout() -> (WorkbookRefs, ReverseRefs) {
        let t = ["x", "y", "z"].map(String::from).to_vec();
        let u = ["p", "q"].map(String::from).to_vec();
        let refs = WorkbookRefs::new([("t", t, 3), ("u", u, 3)], ["s.n"]);
        let scalars = SheetKind::Scalars {
            rows: HashMap::from([(2, "s.n".to_string())]),
            include: None,
        };
        let back = ReverseRefs {
            sheets: HashMap::from([
                ("t".to_string(), table("t", &["x", "y", "z"])),
                ("u".to_string(), table("u", &["p", "q"])),
                ("s".to_string(), scalars),
            ]),
            names: HashMap::new(),
        };
        (refs, back)
    }

    /// What `forge export` writes for COLUMN comes back as it was, or as a
    /// name Forge numbers the same (`t.y` in a row of `t` is `y`)
    #[test]
    fn column_round_trips() {
        let (refs, back) = layout();
        let row = Context::Row { table: "t", row: 2 };
        for (forge, again) in [
            ("=COLUMN() * 10", "=COLUMN() * 10"),
            ("=COLUMN(z)", "=COLUMN(z)"),
            ("=COLUMN(u.q) + x", "=COLUMN(u.q) + x"),
            ("=COLUMN(t.y)", "=COLUMN(y)"),
        ] {
            let excel = refs.to_excel(forge, row).unwrap().formula;
            assert_eq!(back.to_forge(&excel, "t", 2).unwrap(), again, "{excel}");
        }
        let scalar = Context::Scalar { group: Some("s") };
        for forge in ["=COLUMN(t.z)", "=SUM(COLUMN(u.p), 1)"] {
            let excel = refs.to_excel(forge, scalar).unwrap().formula;
            assert_eq!(back.to_forge(&excel, "s", 0).unwrap(), forge, "{excel}");
        }
        // Without `$`, in lower case: the same cell
        assert_eq!(back.to_forge("=column(B1)", "u", 3).unwrap(), "=COLUMN(q)");
    }

    /// Any other COLUMN is Excel's own, and imported as a value
    #[test]
    fn any_other_column_is_refused() {
        let (_, back) = layout();
        let refused = |formula: &str, sheet: &str| {
            let err = back.to_forge(formula, sheet, 2).unwrap_err();
            assert!(
                err.contains("COLUMN works differently in Forge"),
                "{formula}: {err}"
            );
        };
        for formula in [
            "=COLUMN(C2)",
            "=COLUMN($C$1:$C$1)",
            "=COLUMN(D1)",
            "=COLUMN(A1, B1)",
            "=COLUMN(1)",
            "=COLUMN('s'!$B$2)",
            "=COLUMN(",
            "=COLUMN('nosheet'!$A$1)",
        ] {
            refused(formula, "t");
        }
        refused("=COLUMN()", "s");
        refused("=COLUMN('t'!$C$2)", "s");
        let total = back.total_to_forge("=COLUMN()", "t").unwrap_err();
        assert!(total.contains("COLUMN works differently"), "{total}");
    }

    /// A table whose letters Forge numbers otherwise, one with no rows, and
    /// an included file's tables have no COLUMN in Forge
    #[test]
    fn a_table_forge_numbers_otherwise_has_no_column() {
        let (_, mut back) = layout();
        let names = ["a", "b"];
        let sheets = [
            ("w", table_of("w", &names, (vec![2, 3], None, false))),
            ("e", table_of("e", &names, (Vec::new(), None, true))),
            (
                "inc.w",
                table_of("w", &names, (vec![2, 3], Some("inc"), true)),
            ),
        ];
        for (name, kind) in sheets {
            back.sheets.insert(name.to_string(), kind);
            for (formula, on) in [
                ("=COLUMN()".to_string(), name),
                ("=COLUMN($B$1)".to_string(), name),
                (format!("=COLUMN('{name}'!$B$1)"), "s"),
            ] {
                let err = back.to_forge(&formula, on, 2).unwrap_err();
                assert!(err.contains("works differently"), "{name} {formula}: {err}");
            }
        }
        // An included file reads no table of the main model
        let err = back.to_forge("=COLUMN('t'!$A$1)", "inc.w", 2).unwrap_err();
        assert!(err.contains("works differently"), "{err}");
    }
}
