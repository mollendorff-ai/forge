//! Which sheets the other sheets read one cell at a time (roadmap
//! T1-IMPORT-OTHER-LANGUAGES).
//!
//! A sheet of labels and values is a sheet of scalars when `Value` is over
//! its values, but a workbook written in French says `Valeur`, one in Chinese
//! `数值`, and there are as many words as languages. How the other sheets read
//! it says the same in every language: a driver is read one cell at a time
//! (`Hypothèses!$B$3`), a table as a range (`SUM(Ventes!C2:C13)`) or row by
//! row beside its own rows (`Prix!B2` from row 2).

use super::super::workbook_refs::ReverseRefs;
use super::sheet::Sheet;

/// Mark each sheet whose second column, its values beside the labels, the
/// other sheets read only one cell at a time: at least once, never as part
/// of a range of the sheet, and never from the row it reads unless that row
/// is fixed (`B2` from row 2 is how a table reads the row beside its own;
/// `$B$2` is a driver, from any row). Whether it is a sheet of scalars is then
/// decided by its labels, as for any sheet (`Sheet::is_label_sheet`).
pub(super) fn mark(sheets: &mut [Sheet], refs: &ReverseRefs) {
    let reads: Vec<_> = sheets
        .iter()
        .enumerate()
        .flat_map(|(from, sheet)| {
            sheet
                .formula_cells()
                .flat_map(move |(row, formula)| {
                    refs.reads(formula)
                        .into_iter()
                        .map(move |read| (from, row, read))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for (at, sheet) in sheets.iter_mut().enumerate() {
        let values = sheet.letter(1);
        let mut one_at_a_time = false;
        let mut as_a_table = false;
        for (from, row, read) in &reads {
            if *from == at || read.sheet.as_deref() != Some(sheet.name.as_str()) {
                continue;
            }
            if read.range || (read.row == *row && !read.fixed_row) {
                as_a_table = true;
            } else if read.column == values {
                one_at_a_time = true;
            }
        }
        sheet.read_one_at_a_time = one_at_a_time && !as_a_table;
    }
}

#[cfg(test)]
mod tests {
    use super::super::ExcelImporter;
    use crate::types::{ColumnValue, ParsedModel};
    use rust_xlsxwriter::{Formula, Workbook, Worksheet};

    fn import(mut book: Workbook) -> (ParsedModel, Vec<String>) {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::from_bytes(bytes)
            .import_with_warnings()
            .unwrap()
    }

    /// A sheet of drivers: a header of three words, then a label, a number
    /// and a unit on each row
    fn drivers(sheet: &mut Worksheet, header: [&str; 3], rows: &[(&str, f64, &str)]) {
        for (col, text) in (0..).zip(header) {
            sheet.write_string(0, col, text).unwrap();
        }
        for (row, (label, value, unit)) in (1..).zip(rows) {
            sheet.write_string(row, 0, *label).unwrap();
            sheet.write_number(row, 1, *value).unwrap();
            sheet.write_string(row, 2, *unit).unwrap();
        }
    }

    /// A table of three months whose two columns read the drivers' cells (`#`
    /// in `sales` is the row's own number)
    fn months(
        sheet: &mut Worksheet,
        header: [&str; 3],
        volume: &str,
        sales: &str,
        results: (f64, f64),
    ) {
        for (col, text) in (0..).zip(header) {
            sheet.write_string(0, col, text).unwrap();
        }
        for row in 1..=3 {
            sheet.write_number(row, 0, f64::from(row)).unwrap();
            let v = Formula::new(volume).set_result(results.0.to_string());
            sheet.write_formula(row, 1, v).unwrap();
            let s = Formula::new(sales.replace('#', &(row + 1).to_string()))
                .set_result(results.1.to_string());
            sheet.write_formula(row, 2, s).unwrap();
        }
    }

    fn formula(model: &ParsedModel, table: &str, column: &str) -> String {
        model.tables[table].row_formulas[column].clone()
    }

    /// A French workbook: `Hypothèse`, `Valeur`, `Unité` over the drivers,
    /// read cell by cell by the months
    #[test]
    fn a_french_drivers_sheet_is_a_sheet_of_scalars() {
        let mut book = Workbook::new();
        let hyp = book.add_worksheet().set_name("Hypothèses").unwrap();
        drivers(
            hyp,
            ["Hypothèse", "Valeur", "Unité"],
            &[
                ("Prix unitaire", 12.5, "€"),
                ("Volume mensuel", 400.0, "unités"),
            ],
        );
        let ventes = book.add_worksheet().set_name("Ventes").unwrap();
        months(
            ventes,
            ["Mois", "Volume", "Chiffre d'affaires"],
            "=Hypothèses!$B$3",
            "=B#*Hypothèses!$B$2",
            (400.0, 5000.0),
        );
        let (model, warnings) = import(book);

        let price = &model.scalars["hypothèses.prix_unitaire"];
        assert_eq!(price.value, Some(12.5));
        assert_eq!(
            model.scalars["hypothèses.volume_mensuel"].value,
            Some(400.0)
        );
        assert!(!model.tables.contains_key("hypothèses"));
        assert_eq!(
            formula(&model, "ventes", "volume"),
            "=hypothèses.volume_mensuel"
        );
        assert_eq!(
            formula(&model, "ventes", "chiffre_d_affaires"),
            "=volume * hypothèses.prix_unitaire"
        );
        // No formula came in as a value, and the unit's column, headed in
        // French, is kept with the units
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(price.metadata.unit.as_deref(), Some("€"));
        assert_eq!(
            model.scalars["hypothèses.volume_mensuel"]
                .metadata
                .unit
                .as_deref(),
            Some("unités")
        );
    }

    /// The same in Chinese: `项目`, `数值`, `单位`, the drivers `单价` and `月销量`
    #[test]
    fn a_chinese_drivers_sheet_is_a_sheet_of_scalars() {
        let mut book = Workbook::new();
        let assumptions = book.add_worksheet().set_name("假设").unwrap();
        drivers(
            assumptions,
            ["项目", "数值", "单位"],
            &[("单价", 12.5, "元"), ("月销量", 400.0, "件")],
        );
        let sales = book.add_worksheet().set_name("销售").unwrap();
        months(
            sales,
            ["月份", "销量", "收入"],
            "=假设!$B$3",
            "=B#*假设!$B$2",
            (400.0, 5000.0),
        );
        let (model, warnings) = import(book);

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            model.scalars["假设.单价"].metadata.unit.as_deref(),
            Some("元")
        );
        assert_eq!(model.scalars["假设.单价"].value, Some(12.5));
        assert_eq!(model.scalars["假设.月销量"].value, Some(400.0));
        assert_eq!(formula(&model, "销售", "销量"), "=假设.月销量");
        assert_eq!(formula(&model, "销售", "收入"), "=销量 * 假设.单价");
    }

    /// Read as a range (a lookup, or a sum beside a cell read alone), or row
    /// by row beside a table's own rows, or by no other sheet, a sheet of labels and numbers headed `Prix`
    /// stays a table, as it was
    #[test]
    fn a_sheet_read_as_a_range_beside_its_rows_or_not_at_all_stays_a_table() {
        let cases: [(&str, &str, f64); 5] = [
            ("=VLOOKUP(\"b\", Tarifs!$A$2:$B$4, 2, FALSE)", "=A#", 2.0),
            // A range wins over a cell read alone
            ("=Tarifs!$B$3 + SUM(Tarifs!B2:B4)", "=A#", 8.0),
            ("=Tarifs!B#", "=A#", 1.0),
            ("=1", "=A#", 1.0),
            // Its own cells, and another column of it, say nothing
            ("=Tarifs!$A$2", "=A#", 0.0),
        ];
        for (read, other, result) in cases {
            let mut book = Workbook::new();
            let tarifs = book.add_worksheet().set_name("Tarifs").unwrap();
            drivers(
                tarifs,
                ["Article", "Prix", "Note"],
                &[("a", 1.0, ""), ("b", 2.0, ""), ("c", 3.0, "")],
            );
            let own = Formula::new("=$B$2*2").set_result("2");
            tarifs.write_formula(1, 3, own).unwrap();
            let lignes = book.add_worksheet().set_name("Lignes").unwrap();
            months(lignes, ["n", "lu", "autre"], read, other, (result, 0.0));
            let (model, _) = import(book);
            assert!(model.tables.contains_key("tarifs"), "{read}");
            assert!(model.scalars.is_empty(), "{read}");
            assert_eq!(
                model.tables["tarifs"].columns["Prix"].values,
                ColumnValue::Number(vec![1.0, 2.0, 3.0])
            );
        }
    }
}
