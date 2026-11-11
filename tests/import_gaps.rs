//! `forge import` says what it cannot keep (roadmap T1-IMPORT-MIXED and
//! T1-IMPORT-AGGREGATE). Both used to import without a word.
//!
//! The numbers are worked by hand: shares of 10, 20, 30 and 40 over their
//! total of 100 are 0.1, 0.2, 0.3 and 0.4.

use mollendorff_forge::cli::{calculate_core, import_core};
use rust_xlsxwriter::{Formula, Workbook};

#[test]
fn text_in_a_column_of_numbers_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx = dir.path().join("orders.xlsx");
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("orders").unwrap();
    sheet.write_string(0, 0, "amount").unwrap();
    sheet.write_number(1, 0, 10.0).unwrap();
    sheet.write_string(2, 0, "n/a").unwrap();
    sheet.write_number(3, 0, 30.0).unwrap();
    sheet.write_string(4, 0, "see note").unwrap();
    sheet.write_number(5, 0, 50.0).unwrap();
    workbook.save(&xlsx).unwrap();

    let out = dir.path().join("orders.yaml");
    let imported = import_core(&xlsx, &out, false, false).unwrap();
    assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
    let warning = &imported.warnings[0];
    for expected in ["orders.amount", "A3 (n/a)", "A5 (see note)", "as 0"] {
        assert!(warning.contains(expected), "{warning}");
    }
    assert!(!warning.contains("A2"), "{warning}");
}

#[test]
fn a_share_of_the_total_imports_as_values_that_calculate() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx = dir.path().join("shares.xlsx");
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("shares").unwrap();
    sheet.write_string(0, 0, "n").unwrap();
    sheet.write_string(0, 1, "share").unwrap();
    for (i, n) in [10.0, 20.0, 30.0, 40.0].iter().enumerate() {
        let row = u32::try_from(i).unwrap() + 1;
        sheet.write_number(row, 0, *n).unwrap();
        let share = Formula::new(format!("=A{}/SUM($A$2:$A$5)", row + 1))
            .set_result(format!("{}", n / 100.0));
        sheet.write_formula(row, 1, share).unwrap();
    }
    workbook.save(&xlsx).unwrap();

    let out = dir.path().join("shares.yaml");
    let imported = import_core(&xlsx, &out, false, false).unwrap();
    assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
    assert!(
        imported.warnings[0].contains("shares.share")
            && imported.warnings[0].contains("imported as values"),
        "{:?}",
        imported.warnings
    );

    // The model calculates, to the workbook's values
    let result = calculate_core(&out, true, None).unwrap();
    let share: Vec<f64> = result.tables["shares"].values["share"]
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(share, [0.1, 0.2, 0.3, 0.4]);
}
