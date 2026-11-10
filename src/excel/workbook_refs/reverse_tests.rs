//! Tests of `ReverseRefs`: Excel formulas read back as Forge formulas.

use super::*;

/// The layout `WorkbookRefs::new` produces, read back.
fn reverse(refs: &WorkbookRefs) -> ReverseRefs {
    let mut sheets = HashMap::new();
    for (table, columns) in &refs.columns {
        let columns = columns
            .iter()
            .map(|(n, l)| (l.clone(), n.clone()))
            .collect();
        sheets.insert(
            table.clone(),
            SheetKind::Table {
                name: table.clone(),
                columns,
                include: None,
            },
        );
    }
    for (path, (sheet, row)) in &refs.scalars {
        let entry = sheets
            .entry(sheet.clone())
            .or_insert_with(|| SheetKind::Scalars {
                rows: HashMap::new(),
                include: None,
            });
        if let SheetKind::Scalars { rows, .. } = entry {
            rows.insert(*row, path.clone());
        }
    }
    ReverseRefs { sheets }
}

fn layout() -> WorkbookRefs {
    WorkbookRefs::new(
        [
            // In layout order: profit (A), rev (B), year (C); a.base (row 2), a.rate (row 3)
            (
                "p",
                vec!["profit".to_string(), "rev".to_string(), "year".to_string()],
                3,
            ),
            ("q", vec!["x".to_string()], 3),
        ],
        ["a.base", "a.rate", "v.npv", "tax"],
    )
}

#[test]
fn row_formulas_round_trip() {
    let refs = layout();
    let back = reverse(&refs);
    for f in [
        "=a.base * (1 + a.rate) ^ year",
        "=rev - q.x",
        "=IF(rev > 0, \"up\", FALSE)",
        "=-rev + 1.5",
        "=MATCH(rev, q.x, 0)",
        "=rev * tax",
    ] {
        let xl = refs
            .to_excel(f, Context::Row { table: "p", row: 2 })
            .unwrap();
        assert_eq!(
            back.to_forge(&xl.formula, "p", 2).unwrap(),
            f,
            "{}",
            xl.formula
        );
    }
}

#[test]
fn scalar_formulas_round_trip() {
    let refs = layout();
    let back = reverse(&refs);
    for f in [
        "=NPV(a.rate, p.profit)",
        "=v.npv - a.base",
        "=SUM(p.rev) * tax",
    ] {
        let xl = refs
            .to_excel(f, Context::Scalar { group: Some("v") })
            .unwrap();
        assert_eq!(
            back.to_forge(&xl.formula, "v", 0).unwrap(),
            f,
            "{}",
            xl.formula
        );
    }
}

/// `@costs.x` is a cell on the include's sheet and comes back as
/// `@costs.x`; the include's own formulas come back in its own names.
#[test]
fn include_references_round_trip() {
    let mut refs = layout();
    refs.add_include("costs", ["labour.wage", "labour.loaded"]);
    let mut back = reverse(&refs);
    let xl = refs
        .to_excel(
            "=a.base * @costs.labour.wage",
            Context::Scalar { group: Some("v") },
        )
        .unwrap();
    assert_eq!(xl.formula, "='a'!$B$2 * 'costs.Scalars'!$B$2");
    let xl_row = refs
        .to_excel(
            "=rev * @costs.labour.loaded",
            Context::Row { table: "p", row: 3 },
        )
        .unwrap();
    assert_eq!(xl_row.formula, "=B3 * 'costs.Scalars'!$B$3");
    // A bare last segment does not reach into an included file
    assert!(refs
        .to_excel("=wage", Context::Scalar { group: None })
        .unwrap_err()
        .contains("unknown name"));

    let own = WorkbookRefs::for_include(
        "costs",
        ["labour.wage", "labour.loaded"],
        std::iter::empty(),
    );
    let xl_own = own
        .to_excel(
            "=wage * 1.15",
            Context::Scalar {
                group: Some("labour"),
            },
        )
        .unwrap();
    assert_eq!(xl_own.formula, "='costs.Scalars'!$B$2 * 1.15");

    // As the importer reads the sheet: the include's own paths
    back.sheets.insert(
        "costs.Scalars".to_string(),
        SheetKind::Scalars {
            rows: HashMap::from([
                (2, "labour.wage".to_string()),
                (3, "labour.loaded".to_string()),
            ]),
            include: Some("costs".to_string()),
        },
    );
    assert_eq!(
        back.to_forge(&xl.formula, "v", 0).unwrap(),
        "=a.base * @costs.labour.wage"
    );
    assert_eq!(
        back.to_forge(&xl_row.formula, "p", 3).unwrap(),
        "=rev * @costs.labour.loaded"
    );
    assert_eq!(
        back.to_forge(&xl_own.formula, "costs.Scalars", 0).unwrap(),
        "=labour.wage * 1.15"
    );
    assert!(back
        .to_forge("='a'!$B$2 * 2", "costs.Scalars", 0)
        .unwrap_err()
        .contains("outside itself"));
}

#[test]
fn what_cannot_come_back() {
    let back = reverse(&layout());
    assert!(back
        .to_forge("=C3 - B2", "p", 2)
        .unwrap_err()
        .contains("another row"));
    assert!(back
        .to_forge("='p'!$A$2", "v", 0)
        .unwrap_err()
        .contains("single table cell"));
    assert!(back
        .to_forge("=MyRange * 2", "p", 2)
        .unwrap_err()
        .contains("unsupported name"));
    assert!(back
        .to_forge("=A2:B4", "p", 2)
        .unwrap_err()
        .contains("several columns"));
}

#[test]
fn excel_prefixes_and_quotes() {
    let back = reverse(&layout());
    assert_eq!(
        back.to_forge("=_xlfn.STDEV.S('p'!$A$2:$A$4)", "v", 0)
            .unwrap(),
        "=STDEV.S(p.profit)"
    );
    assert_eq!(
        back.to_forge("=\"a\"\"b\"", "v", 0).unwrap_err(),
        "text with a double quote"
    );
}
