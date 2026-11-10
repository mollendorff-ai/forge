use super::*;

/// Columns and scalars are listed in the order the layout uses:
/// revenue (A), `total_profit` (B), year (C); valuation.net (row 2), npv (row 3).
fn refs() -> WorkbookRefs {
    WorkbookRefs::new(
        [
            (
                "projections",
                vec![
                    "revenue".to_string(),
                    "total_profit".to_string(),
                    "year".to_string(),
                ],
                3,
            ),
            ("other", vec!["x".to_string()], 3),
        ],
        [
            "assumptions.base_revenue",
            "assumptions.growth",
            "valuation.net",
            "valuation.npv",
            "tax",
        ],
    )
}

#[test]
fn layout_keeps_the_order_given() {
    // Model order, not alphabetical: year (A), revenue (B), year again ignored;
    // b.second before b.first gives rows 2 and 3.
    let refs = WorkbookRefs::new(
        [(
            "t",
            vec![
                "year".to_string(),
                "revenue".to_string(),
                "year".to_string(),
            ],
            3,
        )],
        ["b.second", "b.first"],
    );
    let columns = &refs.columns["t"];
    assert_eq!(columns["year"], "A");
    assert_eq!(columns["revenue"], "B");
    assert_eq!(columns.len(), 2);
    assert_eq!(refs.scalars["b.second"], ("b".to_string(), 2));
    assert_eq!(refs.scalars["b.first"], ("b".to_string(), 3));
}

fn row(f: &str) -> Result<String, String> {
    refs()
        .to_excel(
            f,
            Context::Row {
                table: "projections",
                row: 3,
            },
        )
        .map(|t| t.formula)
}

fn scalar(f: &str, group: Option<&str>) -> Result<Translated, String> {
    refs().to_excel(f, Context::Scalar { group })
}

#[test]
fn column_letters() {
    assert_eq!(column_letter(0), "A");
    assert_eq!(column_letter(25), "Z");
    assert_eq!(column_letter(26), "AA");
    assert_eq!(column_letter(701), "ZZ");
    assert_eq!(column_letter(702), "AAA");
}

#[test]
fn row_formula_columns_named_like_functions() {
    // `year` is a column here, not YEAR(); the fixture lists
    // revenue (A), total_profit (B), year (C).
    assert_eq!(
        row("=assumptions.base_revenue * (1 + assumptions.growth) ^ year").unwrap(),
        "='assumptions'!$B$2 * (1 + 'assumptions'!$B$3)^C3"
    );
}

#[test]
fn row_formula_other_table_is_same_row() {
    assert_eq!(row("=revenue - other.x").unwrap(), "=A3 - 'other'!A3");
}

#[test]
fn row_formula_bare_scalar_name() {
    assert_eq!(row("=revenue * tax").unwrap(), "=A3 * 'Scalars'!$B$2");
    assert_eq!(
        row("=revenue * growth").unwrap(),
        "=A3 * 'assumptions'!$B$3"
    );
}

#[test]
fn unary_minus_and_literals() {
    assert_eq!(row("=-revenue + 1.5").unwrap(), "=-A3 + 1.5");
    assert_eq!(
        row("=IF(revenue > 0, \"up\", FALSE)").unwrap(),
        "=IF(A3 > 0, \"up\", FALSE)"
    );
}

#[test]
fn scalar_formula_ranges_and_own_group() {
    let t = scalar(
        "=NPV(assumptions.growth, projections.total_profit)",
        Some("valuation"),
    )
    .unwrap();
    assert_eq!(
        t.formula,
        "=NPV('assumptions'!$B$3, 'projections'!$B$2:$B$4)"
    );
    assert!(t.has_range);

    // `npv` is in the formula's own group: same sheet, no prefix.
    let t = scalar("=npv - assumptions.base_revenue", Some("valuation")).unwrap();
    assert_eq!(t.formula, "=$B$3 - 'assumptions'!$B$2");
    assert!(!t.has_range);
}

#[test]
fn irr_only_when_excel_means_the_same() {
    assert_eq!(
        scalar("=IRR(projections.total_profit)", Some("valuation"))
            .unwrap()
            .formula,
        "=IRR('projections'!$B$2:$B$4)"
    );
    assert!(scalar("=IRR(projections.total_profit, 0.1)", Some("valuation")).is_ok());
    let err = scalar(
        "=IRR(-assumptions.base_revenue, projections.total_profit)",
        Some("valuation"),
    )
    .unwrap_err();
    assert!(err.contains("guess"), "{err}");
}

#[test]
fn untranslatable_formulas_say_why() {
    assert!(row("=MC.Normal(1, 2)")
        .unwrap_err()
        .contains("no Excel equivalent"));
    assert!(row("=revenue[0]").unwrap_err().contains("indexing"));
    assert!(row("=missing * 2").unwrap_err().contains("unknown name"));
}

#[test]
fn lookup_arguments_are_whole_columns_in_row_formulas() {
    // MATCH reads its second argument as the whole column; the first is per row.
    assert_eq!(
        row("=MATCH(revenue, other.x, 0)").unwrap(),
        "=MATCH(A3, 'other'!$A$2:$A$4, 0)"
    );
    assert_eq!(
        row("=INDEX(total_profit, 2) + year").unwrap(),
        "=INDEX($B$2:$B$4, 2) + C3"
    );
}

#[test]
fn functions_that_differ_in_excel_are_not_translated() {
    for f in [
        "=VLOOKUP(1, revenue, 2)",
        "=NOW()",
        "=ROW(revenue)",
        "=COLUMN()",
    ] {
        assert!(row(f).unwrap_err().contains("no Excel equivalent"), "{f}");
    }
    // COUNTUNIQUE has an Excel form only in a scalar (local_names)
    assert!(row("=COUNTUNIQUE(revenue)")
        .unwrap_err()
        .contains("row formula"));
}

/// Forge names Excel spells otherwise, functions Excel stores with a
/// prefix, and `COLUMNS(table)`.
#[test]
fn functions_with_other_names_in_excel() {
    for (forge, excel) in [
        ("=POW(revenue, 2)", "=POWER(A3, 2)"),
        ("=AVG(revenue, 1)", "=AVERAGE(A3, 1)"),
        ("=ROUND(E() * revenue, 2)", "=ROUND(EXP(1) * A3, 2)"),
        ("=LN(E())", "=LN(EXP(1))"),
    ] {
        assert_eq!(row(forge).unwrap(), excel, "{forge}");
    }
    assert!(row("=E(1)").unwrap_err().contains("no arguments"));
    for (forge, excel) in [
        (
            "=STDEV.S(projections.revenue)",
            "=STDEV.S('projections'!$A$2:$A$4)",
        ),
        (
            "=RANK.EQ(assumptions.growth, projections.revenue, 1)",
            "=RANK.EQ('assumptions'!$B$3, 'projections'!$A$2:$A$4, 1)",
        ),
        ("=COLUMNS(projections)", "=COLUMNS('projections'!$A$2:$C$4)"),
        ("=RANDBETWEEN(1, 6)", "=RANDBETWEEN(1, 6)"),
    ] {
        assert_eq!(
            scalar(forge, Some("valuation")).unwrap().formula,
            excel,
            "{forge}"
        );
    }
}

/// Forge's own functions become the Excel expression that computes the
/// same (forge-e2e-gnumeric recalculates each one in Gnumeric).
#[test]
fn forge_functions_as_excel_expressions() {
    for (forge, excel) in [
        ("=VARIANCE(revenue, 100)", "=((A3) - (100))"),
        ("=VARIANCE_PCT(revenue, year)", "=(((A3) - (C3)) / ABS(C3))"),
        (
            "=BREAKEVEN_UNITS(1000, revenue, 20)",
            "=IF(((A3) - (20)) <= 0, NA(), (1000) / ((A3) - (20)))",
        ),
        (
            "=BREAKEVEN_REVENUE(1000, 0.4)",
            "=IF(OR((0.4) <= 0, (0.4) > 1), NA(), (1000) / (0.4))",
        ),
        (
            "=VARIANCE_STATUS(revenue, 100, \"cost\")",
            "=IF((100) = 0, SIGN(A3), IF(ABS((((A3) - (100)) / ABS(100))) <= 0.01, 0, IF((((A3) - (100)) / ABS(100)) > 0, -1, 1)))",
        ),
        (
            "=ROUND(VARIANCE(MAX(revenue, 1), 2) * 3, 0)",
            "=ROUND(((MAX(A3, 1)) - (2)) * 3, 0)",
        ),
    ] {
        assert_eq!(row(forge).unwrap(), excel, "{forge}");
    }
    assert!(row("=VARIANCE(revenue)").unwrap_err().contains("takes 2"));
    assert!(row("=VARIANCE_STATUS(revenue, 100, year)")
        .unwrap_err()
        .contains("number or"));
}

#[test]
fn sheet_names_with_quotes() {
    assert_eq!(sheet_prefix("Bob's"), "'Bob''s'!");
}
