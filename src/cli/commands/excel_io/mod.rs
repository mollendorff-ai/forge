//! Excel export and import commands

mod export;
mod import;

pub use export::{export_bytes_core, export_core, ExportBytes};
pub use import::{import_core, import_text_core, ImportText};
// The commands themselves, and base64, belong to the command line
#[cfg(feature = "cli")]
pub use {export::export, export::export_buffer_core, import::import};

#[cfg(test)]
mod round_trip_tests {
    use super::*;
    use crate::core::ArrayCalculator;
    use crate::error::ForgeResult;
    use crate::excel::ExcelExporter;
    use crate::parser;
    use crate::types::{ColumnValue, ParsedModel};
    use std::fs;
    use std::path::Path;

    fn calculate(model: ParsedModel) -> ForgeResult<ParsedModel> {
        ArrayCalculator::new(model).calculate_all()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    /// Export `path`, import the workbook, and check every calculated number
    /// against the original's: a difference fails the test. `Err` only for a
    /// model that does not calculate on its own.
    fn round_trip(path: &Path) -> Result<(), String> {
        let original = calculate(parser::parse_model(path).unwrap())
            .map_err(|e| format!("original does not calculate: {e}"))?;
        let dir = tempfile::TempDir::new().unwrap();
        let xlsx = dir.path().join("model.xlsx");
        let yaml = dir.path().join("model.yaml");
        ExcelExporter::new(original.clone()).export(&xlsx).unwrap();
        import_core(&xlsx, &yaml, false, false).unwrap();
        let back = calculate(parser::parse_model(&yaml).unwrap()).unwrap();

        for (name, a) in original
            .scalars
            .iter()
            .filter_map(|(name, var)| var.value.map(|a| (name, a)))
        {
            // Import may lower-case a name, and brings a network's results
            // back under another group, since a model's bayesian_network is
            // its network
            let wanted = name.strip_prefix("bayesian_network.").map_or_else(
                || name.to_lowercase(),
                |rest| format!("network_results.{rest}"),
            );
            let found = back
                .scalars
                .iter()
                .find(|(k, _)| k.to_lowercase() == wanted)
                .map(|(_, v)| v.value);
            assert!(
                found.flatten().is_some_and(|b| close(a, b)),
                "{path:?}: {name}: {a} vs {found:?}"
            );
        }
        for (name, table) in &original.tables {
            for (col, column) in &table.columns {
                let Some(a) = column.values.numbers() else {
                    continue;
                };
                let found = back
                    .tables
                    .get(&name.to_lowercase())
                    .and_then(|t| t.columns.get(col))
                    .and_then(|c| c.values.numbers());
                assert!(
                    found.as_ref().is_some_and(|b| a.len() == b.len()
                        && a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y))),
                    "{path:?}: {name}.{col}: {a:?} vs {found:?}"
                );
            }
        }
        Ok(())
    }

    /// Columns and scalars keep the order of the model file through calculate,
    /// export and import, and the numbers survive. The file order here is
    /// deliberately not alphabetical, and mixes data and formula columns.
    #[test]
    fn model_order_survives_calculate_export_and_import() {
        let dir = tempfile::TempDir::new().unwrap();
        let model = dir.path().join("order.yaml");
        fs::write(
            &model,
            r#"_forge_version: "5.0.0"
t:
  zeta: [1, 2, 3]
  alpha: "=zeta * 2"
  mid: [10, 20, 30]
  beta: "=alpha + mid"
g:
  z_last:
    value: 5
  a_first:
    value: null
    formula: "=g.z_last * 3"
"#,
        )
        .unwrap();

        let calculated = calculate(parser::parse_model(&model).unwrap()).unwrap();
        let order = |m: &ParsedModel| -> (Vec<String>, Vec<String>) {
            let columns = m.tables["t"].column_names().into_iter().cloned().collect();
            let scalars = m.scalars.keys().cloned().collect();
            (columns, scalars)
        };
        let expected_columns = ["zeta", "alpha", "mid", "beta"].map(String::from).to_vec();
        let expected_scalars = ["g.z_last", "g.a_first"].map(String::from).to_vec();
        assert_eq!(
            order(&calculated),
            (expected_columns.clone(), expected_scalars.clone())
        );

        let xlsx = dir.path().join("order.xlsx");
        let back_yaml = dir.path().join("back.yaml");
        ExcelExporter::new(calculated).export(&xlsx).unwrap();
        import_core(&xlsx, &back_yaml, false, false).unwrap();
        let back = calculate(parser::parse_model(&back_yaml).unwrap()).unwrap();
        assert_eq!(order(&back), (expected_columns, expected_scalars));
        assert_eq!(
            back.tables["t"].columns["beta"].values,
            ColumnValue::Number(vec![12.0, 24.0, 36.0])
        );
        assert_eq!(back.scalars["g.a_first"].value, Some(15.0));

        round_trip(&model).unwrap();
    }

    /// The shipped examples that calculate on their own come back from Excel
    /// with the same numbers.
    #[test]
    fn examples_round_trip_through_excel() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for example in [
            "scenarios",
            "variance",
            "breakeven",
            "quota_forecast",
            "bagel/bagel",
            "bagel/actual",
            "bagel/costs",
        ] {
            let file = root.join(format!("examples/{example}.yaml"));
            round_trip(&file).unwrap();
        }
    }

    /// T1-EXPORT-INCLUDES: a value from an included file is a cell reference
    /// in Excel, the included file's own formula is a formula on its sheet,
    /// and import gives back the included file and `@costs` references.
    #[test]
    fn included_values_stay_live_and_come_back_as_an_include() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let costs = "_forge_version: \"5.1.0\"\n# kept as written\nlabour:\n  wage: 18.5\n  load: 0.15\n  loaded:\n    formula: \"=wage * (1 + load)\"\n";
        fs::write(dir.path().join("costs.yaml"), costs).unwrap();
        let main = dir.path().join("main.yaml");
        fs::write(
            &main,
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\nshop:\n  hours: 100\n  pay:\n    formula: \"=hours * @costs.labour.loaded\"\n",
        )
        .unwrap();
        let original = calculate(parser::parse_model(&main).unwrap()).unwrap();
        let xlsx = dir.path().join("main.xlsx");
        let warnings = ExcelExporter::new(original.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let shop = wb.worksheet_formula("shop").unwrap();
        assert_eq!(
            shop.get_value((2, 1)).map(String::as_str),
            Some("$B$2 * 'costs.Scalars'!$B$4"),
            "pay reads the include's cell"
        );
        let inc = wb.worksheet_formula("costs.Scalars").unwrap();
        assert_eq!(
            inc.get_value((3, 1)).map(String::as_str),
            Some("'costs.Scalars'!$B$2 * (1 + 'costs.Scalars'!$B$3)"),
            "the include's own formula is a formula"
        );

        // Into another folder: the included file is written, and read by `@costs`
        let other = tempfile::TempDir::new().unwrap();
        let back = other.path().join("main.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        assert!(
            text.contains("_includes:\n  - file: costs.yaml\n    as: costs\n"),
            "{text}"
        );
        assert!(
            text.contains("=shop.hours * @costs.labour.loaded"),
            "{text}"
        );
        let included = fs::read_to_string(other.path().join("costs.yaml")).unwrap();
        assert!(
            included.contains("labour:\n  wage: 18.5\n  load: 0.15\n  loaded:\n    formula: =labour.wage * (1 + labour.load)"),
            "{included}"
        );
        let calculated = calculate(parser::parse_model(&back).unwrap()).unwrap();
        assert_eq!(
            calculated.scalars["shop.pay"].value,
            original.scalars["shop.pay"].value
        );

        // Beside the original: it means the same, so it stays as written
        import_core(&xlsx, &dir.path().join("back.yaml"), false, false).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("costs.yaml")).unwrap(),
            costs
        );

        // Beside a file with other values: refused, nothing written
        let changed = tempfile::TempDir::new().unwrap();
        fs::write(
            changed.path().join("costs.yaml"),
            costs.replace("18.5", "19"),
        )
        .unwrap();
        let err = import_core(&xlsx, &changed.path().join("main.yaml"), false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("hold other values"), "{err}");
        assert!(
            !changed.path().join("main.yaml").exists(),
            "nothing may be written"
        );

        // Split files: the part that reads `@costs` lists the include
        let split = other.path().join("split");
        import_core(&xlsx, &split, true, false).unwrap();
        let scalars = fs::read_to_string(split.join("scalars.yaml")).unwrap();
        assert!(scalars.contains("_includes:"), "{scalars}");
        let part = calculate(parser::parse_model(&split.join("scalars.yaml")).unwrap()).unwrap();
        assert_eq!(
            part.scalars["shop.pay"].value,
            original.scalars["shop.pay"].value
        );
    }

    /// A column shorter than its table is its own range: with the table's
    /// rows, `ROWS(d.short)` was 5 in Excel and 2 in Forge, and `INDEX` past
    /// the column's end read an empty cell.
    #[test]
    fn a_short_column_is_exported_as_its_own_range() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(
            &path,
            "_forge_version: \"5.1.0\"\nd:\n  long: [1, 2, 3, 4, 5]\n  short: [7, 8]\ns:\n  rows:\n    formula: \"=ROWS(d.short)\"\n  last:\n    formula: \"=INDEX(d.short, ROWS(d.short))\"\n",
        )
        .unwrap();
        let model = calculate(parser::parse_model(&path).unwrap()).unwrap();
        assert_eq!(model.scalars["s.rows"].value, Some(2.0));
        let xlsx = dir.path().join("m.xlsx");
        ExcelExporter::new(model)
            .export_with_warnings(&xlsx)
            .unwrap();
        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let s = wb.worksheet_formula("s").unwrap();
        assert_eq!(
            s.get_value((1, 1)).map(String::as_str),
            Some("ROWS('d'!$B$2:$B$3)")
        );
        assert_eq!(
            s.get_value((2, 1)).map(String::as_str),
            Some("INDEX('d'!$B$2:$B$3, ROWS('d'!$B$2:$B$3))")
        );
    }

    /// T2-EXPORT-LET: LET, a LAMBDA called at once, COUNTUNIQUE of a column
    /// and TODAY are Excel formulas, and import reads them back to formulas
    /// that calculate the same. (forge-e2e-gnumeric recalculates each kind in
    /// `LibreOffice`.)
    #[test]
    fn let_lambda_countunique_and_today_are_formulas_both_ways() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(
            &path,
            concat!(
                "_forge_version: \"5.1.0\"\n",
                "t:\n  name: [\"a\", \"A\", \"b\", \"a\"]\n  n: [1, 2, 2, 3]\n  year: \"=YEAR(TODAY()) + n\"\n",
                "x: 100\n",
                "s:\n",
                "  let:\n    formula: \"=LET(a, 2, b, a * 3, a + b)\"\n",
                "  lambda:\n    formula: \"=LAMBDA(x, r, x * r + 1)(4, x)\"\n",
                "  nested:\n    formula: \"=LET(v, LET(a, 5, a * 2), v + x)\"\n",
                "  text:\n    formula: \"=COUNTUNIQUE(t.name)\"\n",
                "  numbers:\n    formula: \"=COUNTUNIQUE(t.n)\"\n",
                "  today:\n    formula: \"=TODAY() - TODAY()\"\n",
            ),
        )
        .unwrap();
        let model = calculate(parser::parse_model(&path).unwrap()).unwrap();
        let xlsx = dir.path().join("m.xlsx");
        let warnings = ExcelExporter::new(model.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let back = dir.path().join("back.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        for formula in [
            "=LET(a, 2, b, a * 3, a + b)",
            "=LET(x_, 4, r_, x, x_ * r_ + 1)",
            "=LET(a, 5, v, a * 2, v + x)",
            "=COUNTUNIQUE(t.name)",
            "=COUNTUNIQUE(t.n)",
            "=YEAR(TODAY()) + n",
        ] {
            assert!(text.contains(formula), "{formula} in {text}");
        }
        let again = calculate(parser::parse_model(&back).unwrap()).unwrap();
        for (name, want) in [
            ("s.let", 8.0),
            ("s.lambda", 401.0),
            ("s.nested", 110.0),
            ("s.text", 3.0),
            ("s.numbers", 3.0),
            ("s.today", 0.0),
        ] {
            assert_eq!(model.scalars[name].value, Some(want), "{name}");
            assert_eq!(again.scalars[name].value, Some(want), "{name} after import");
        }
    }

    fn column(model: &crate::types::ParsedModel, table: &str, column: &str) -> Vec<f64> {
        let values = &model.tables[table].columns[column].values;
        values.numbers().unwrap().into_owned()
    }

    /// T2-TIME: a PREV column is cells that read the row above, a period
    /// column is its dates, and import reads the column back as one formula
    /// that still reads the row above.
    #[test]
    fn a_column_that_reads_the_row_above_survives_export_and_import() {
        use calamine::{open_workbook, DataType, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(
            &path,
            concat!(
                "_forge_version: \"5.2.0\"\n",
                "loan:\n  principal: 1000\n",
                "schedule:\n",
                "  month: {start: 2026-12, count: 4, unit: month}\n",
                "  number: \"=ROW()\"\n",
                "  opening: \"=PREV(closing, loan.principal)\"\n",
                "  interest: \"=opening * 0.01\"\n",
                "  closing: \"=opening + interest - 300\"\n",
            ),
        )
        .unwrap();
        let model = calculate(parser::parse_model(&path).unwrap()).unwrap();
        // 1000 -> 710 -> 417.10 -> 121.271 -> -177.51629
        let closing = column(&model, "schedule", "closing");
        assert!((closing[3] + 177.516_29).abs() < 1e-9, "{closing:?}");
        let xlsx = dir.path().join("m.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let sheet = wb.worksheet_formula("schedule").unwrap();
        let cell = |row, col| sheet.get_value((row, col)).map(String::as_str);
        assert_eq!(cell(1, 1), Some("(ROW() - 1)"));
        assert_eq!(cell(1, 2), Some("('loan'!$B$2)"));
        assert_eq!(cell(2, 2), Some("(E2)"));
        assert_eq!(cell(4, 2), Some("(E4)"));
        // A period column is dates in the workbook: the day number of
        // 2027-01-01, shown as a date (ADR-054)
        let values = wb.worksheet_range("schedule").unwrap();
        let date = values.get_value((2, 0)).unwrap().get_datetime().unwrap();
        assert!((date.as_f64() - 46388.0).abs() < 1e-9, "{date:?}");
        assert_eq!(date.to_ymd_hms_milli(), (2027, 1, 1, 0, 0, 0, 0));

        let back = dir.path().join("back.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        for formula in [
            "number: =ROW()",
            // The column only reads the row above: PREV, as it was written
            "opening: =PREV(closing, loan.principal)",
            "closing: =opening + interest - 300",
        ] {
            assert!(text.contains(formula), "{formula} in {text}");
        }
        // The months come back as the dates they are
        assert!(text.contains("- 2026-12-01\n  - 2027-01-01"), "{text}");
        // Still a model: another principal gives another schedule
        let changed = text.replace("principal: 1000", "principal: 2000");
        fs::write(&back, changed).unwrap();
        let again = calculate(parser::parse_model(&back).unwrap()).unwrap();
        // 2000 -> 1720 -> 1437.2
        assert!((column(&again, "schedule", "closing")[1] - 1437.2).abs() < 1e-9);
    }

    /// `forge import` took the formula of the first data row for the whole
    /// column, so a running balance (100, 150, 175) came back as `=amount`
    /// (100, 50, 25), and Excel's `ROW()` and `COLUMN()` were read as
    /// Forge's, which count differently.
    #[test]
    fn import_reads_every_row_of_a_formula_column() {
        use rust_xlsxwriter::{Formula, Workbook};
        let dir = tempfile::TempDir::new().unwrap();
        let xlsx = dir.path().join("ledger.xlsx");
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet().set_name("ledger").unwrap();
        for (col, name) in ["amount", "balance", "double", "odd", "row", "column"]
            .iter()
            .enumerate()
        {
            sheet
                .write_string(0, u16::try_from(col).unwrap(), *name)
                .unwrap();
        }
        let amounts = [100.0, 50.0, 25.0];
        let mut balance = 0.0;
        for (i, amount) in amounts.iter().enumerate() {
            let row = u32::try_from(i).unwrap() + 1;
            let r = row + 1; // Excel row
            balance += amount;
            sheet.write_number(row, 0, *amount).unwrap();
            let cells = [
                (
                    if i == 0 {
                        "=A2".to_string()
                    } else {
                        format!("=B{}+A{r}", r - 1)
                    },
                    balance,
                ),
                (format!("=A{r}*2"), amount * 2.0),
                // A different formula in every row
                (format!("=A{r}*{}", i + 1), amount * f64::from(row)),
                (format!("=ROW()*A{r}"), f64::from(r) * amount),
                (format!("=COLUMN()+A{r}"), 6.0 + amount),
            ];
            for (col, (formula, result)) in cells.iter().enumerate() {
                let formula = Formula::new(formula).set_result(result.to_string());
                sheet
                    .write_formula(row, u16::try_from(col).unwrap() + 1, formula)
                    .unwrap();
            }
        }
        workbook.save(&xlsx).unwrap();

        let back = dir.path().join("ledger.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        assert!(
            text.contains("balance: =IF(ROW() = 1, amount, PREV(balance, 0) + amount)"),
            "{text}"
        );
        assert!(text.contains("double: =amount * 2"), "{text}");
        assert!(text.contains("row: =(ROW() + 1) * amount"), "{text}");
        let model = calculate(parser::parse_model(&back).unwrap()).unwrap();
        assert_eq!(column(&model, "ledger", "balance"), [100.0, 150.0, 175.0]);
        assert_eq!(column(&model, "ledger", "double"), [200.0, 100.0, 50.0]);
        // Kept as the workbook's values, not as the first row's formula
        assert_eq!(column(&model, "ledger", "odd"), [100.0, 100.0, 75.0]);
        assert_eq!(column(&model, "ledger", "row"), [200.0, 150.0, 100.0]);
        assert_eq!(column(&model, "ledger", "column"), [106.0, 56.0, 31.0]);
    }

    /// T2-INCLUDE-COLUMNS: a table of an included file is a sheet with its
    /// formulas, a formula that reads one of its columns reads that sheet
    /// (the same row in a row formula, the whole column in an aggregate), and
    /// import puts the table back in the included file.
    #[test]
    fn included_tables_stay_live_and_come_back_in_the_include() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let prices = "_forge_version: \"5.1.0\"\ntax:\n  rate: 0.15\nlist:\n  per_kg: [1.15, 9.40, 0.80]\n  with_tax: \"=per_kg * (1 + tax.rate)\"\n";
        fs::write(dir.path().join("prices.yaml"), prices).unwrap();
        let main = dir.path().join("main.yaml");
        fs::write(
            &main,
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: prices.yaml\n    as: prices\norder:\n  kg: [120, 6, 3]\n  cost: \"=kg * @prices.list.with_tax\"\ntotal:\n  dearest:\n    formula: \"=MAX(@prices.list.per_kg)\"\n",
        )
        .unwrap();
        let original = calculate(parser::parse_model(&main).unwrap()).unwrap();
        let xlsx = dir.path().join("main.xlsx");
        let warnings = ExcelExporter::new(original.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let order = wb.worksheet_formula("order").unwrap();
        assert_eq!(
            order.get_value((1, 1)).map(String::as_str),
            Some("A2 * 'prices.list'!B2"),
            "the same row of the included table"
        );
        let list = wb.worksheet_formula("prices.list").unwrap();
        assert_eq!(
            list.get_value((1, 1)).map(String::as_str),
            Some("A2 * (1 + 'prices.Scalars'!$B$2)"),
            "the included table's own formula"
        );
        let total = wb.worksheet_formula("total").unwrap();
        assert_eq!(
            total.get_value((1, 1)).map(String::as_str),
            Some("MAX('prices.list'!$A$2:$A$4)")
        );

        let other = tempfile::TempDir::new().unwrap();
        let back = other.path().join("main.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        assert!(text.contains("cost: =kg * @prices.list.with_tax"), "{text}");
        assert!(text.contains("=MAX(@prices.list.per_kg)"), "{text}");
        assert!(
            !text.contains("per_kg:"),
            "the table stays in the include: {text}"
        );
        let included = fs::read_to_string(other.path().join("prices.yaml")).unwrap();
        assert!(
            included.contains("with_tax: =per_kg * (1 + tax.rate)"),
            "{included}"
        );
        let calculated = calculate(parser::parse_model(&back).unwrap()).unwrap();
        assert_eq!(
            calculated.tables["order"].columns["cost"].values,
            original.tables["order"].columns["cost"].values
        );
        assert_eq!(
            calculated.scalars["total.dearest"].value,
            original.scalars["total.dearest"].value
        );

        // Beside the original, which means the same: kept as written
        import_core(&xlsx, &dir.path().join("back.yaml"), false, false).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("prices.yaml")).unwrap(),
            prices
        );
        // Beside one whose table holds another price: refused
        let changed = tempfile::TempDir::new().unwrap();
        fs::write(
            changed.path().join("prices.yaml"),
            prices.replace("9.40", "9.50"),
        )
        .unwrap();
        let err = import_core(&xlsx, &changed.path().join("main.yaml"), false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("hold other values"), "{err}");
    }

    /// ADR-052: `--multi-doc` gives the same numbers as one file, and
    /// `--split-files` refuses a model whose parts refer to each other.
    #[test]
    fn multi_doc_import_calculates_like_one_file_and_split_refuses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model =
            calculate(parser::parse_model(&root.join("examples/bagel/bagel.yaml")).unwrap())
                .unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let xlsx = dir.path().join("m.xlsx");
        ExcelExporter::new(model).export(&xlsx).unwrap();

        let single = dir.path().join("single.yaml");
        let multi = dir.path().join("multi.yaml");
        import_core(&xlsx, &single, false, false).unwrap();
        import_core(&xlsx, &multi, false, true).unwrap();
        let a = calculate(parser::parse_model(&single).unwrap()).unwrap();
        let b = calculate(parser::parse_model(&multi).unwrap()).unwrap();
        assert_eq!(a.scalars.len(), b.scalars.len());
        for (name, var) in &a.scalars {
            assert_eq!(var.value, b.scalars[name].value, "{name}");
        }
        for (name, table) in &a.tables {
            assert_eq!(
                format!("{:?}", table.columns),
                format!("{:?}", b.tables[name].columns),
                "{name}"
            );
        }

        let split = dir.path().join("split");
        let err = import_core(&xlsx, &split, true, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--split-files would break"), "{err}");
        assert!(!split.exists(), "nothing may be written");
    }

    /// `forge import` writes model format 5.1.0, without nulls (ADR-051); the
    /// bagel has number formats, so 5.4.0 (ADR-061).
    #[test]
    fn imported_examples_are_short_form() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for example in ["variance", "breakeven", "bagel/bagel"] {
            let model = calculate(
                parser::parse_model(&root.join(format!("examples/{example}.yaml"))).unwrap(),
            )
            .unwrap();
            let dir = tempfile::TempDir::new().unwrap();
            let xlsx = dir.path().join("m.xlsx");
            let yaml = dir.path().join("m.yaml");
            ExcelExporter::new(model).export(&xlsx).unwrap();
            import_core(&xlsx, &yaml, false, false).unwrap();
            let text = fs::read_to_string(&yaml).unwrap();
            let version = ["5.1.0", "5.4.0"][usize::from(example == "bagel/bagel")];
            assert!(
                text.starts_with(&format!("_forge_version: \"{version}\"")),
                "{example}"
            );
            assert!(!text.contains("null"), "{example}:\n{text}");
            assert!(
                !text.contains(".0\n"),
                "{example}: whole numbers written as floats\n{text}"
            );
        }
    }

    /// Every YAML model under `examples/` goes through Excel and comes back
    /// with the same numbers, except the ones that do not calculate on their
    /// own: each of those draws from an `MC.` distribution, which only
    /// `forge simulate` evaluates (directly, or in an included file), so there
    /// is no number to carry through the workbook.
    #[test]
    fn round_trip_corpus() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        let mut files = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for path in fs::read_dir(dir)
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.path())
            {
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|x| x == "yaml") {
                    files.push(path);
                }
            }
        }
        files.sort();
        let results: Vec<(String, Result<(), String>)> = files
            .iter()
            .map(|f| {
                let name = f.strip_prefix(&root).unwrap().display().to_string();
                (name, round_trip(f))
            })
            .collect();

        let skipped: Vec<(&str, &str)> = results
            .iter()
            .filter_map(|(n, r)| r.as_ref().err().map(|e| (n.as_str(), e.as_str())))
            .collect();
        let names: Vec<&str> = skipped.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            [
                "acquisition/deal_risk.yaml",
                "bagel/bagel_simulate.yaml",
                "bagel/costs_risk.yaml",
                "microgrid/microgrid_risk.yaml",
                "monte-carlo.yaml",
                "realestate/realestate_flex.yaml",
                "realestate/realestate_risk.yaml",
                "vineyard/vineyard_independent.yaml",
                "vineyard/vineyard_risk.yaml",
            ]
        );
        for (name, error) in &skipped {
            assert!(
                error.starts_with("original does not calculate"),
                "{name}: {error}"
            );
            let text = fs::read_to_string(root.join(name)).unwrap();
            assert!(text.contains("MC."), "{name} has no MC. distribution");
        }
        // The rest, 56 models (the guide's two among them), all round-trip
        // with no difference. The ten
        // in games/, bagel/price_war.yaml, bagel/street_fund.yaml and
        // acquisition/deal_game.yaml come
        // back as their results, numbers as of the export: the export warns
        // that the games themselves are not written. So does the network of
        // expected_loss.yaml, under group network_results
        assert_eq!(results.len() - skipped.len(), 56);
    }
}

/// examples/inherited-budget (roadmap T1-EXAMPLE-EXCEL-ROUNDTRIP): a workbook
/// Forge did not write, with titles, merged headings, a blank row, a total
/// row, a named range and a number typed in, imported, audited and changed.
#[cfg(test)]
mod inherited_budget {
    use super::*;
    use crate::cli::commands::results::AuditDep;
    use crate::cli::commands::{audit_core, calculate_core};
    use calamine::{open_workbook, DataType, Reader, Xlsx};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn example(file: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/inherited-budget")
            .join(file)
    }

    /// The value `LibreOffice` stored in a cell of the workbook (row and
    /// column from 0).
    fn stored(
        book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
        sheet: &str,
        at: (u32, u32),
    ) -> f64 {
        let range = book.worksheet_range(sheet).unwrap();
        range.get_value(at).and_then(DataType::as_f64).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    /// Every number the workbook calculated, the months, the total row and
    /// the summary, is what the imported model calculates; and the import
    /// names the three things it did not find as Forge holds them.
    #[test]
    fn the_imported_workbook_calculates_to_its_own_values() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("budget.yaml");
        let imported = import_core(&example("budget.xlsx"), &yaml, false, false).unwrap();
        let warnings = &imported.warnings;
        assert_eq!(warnings.len(), 3, "{warnings:#?}");
        assert!(warnings[0].starts_with("monthly.price: its rows hold different formulas"));
        assert!(warnings[0].ends_with(
            "=IF(ROW() <= 6, assumptions.price, assumptions.price * (1 + assumptions.price_rise_from_july))"
        ));
        assert!(warnings[1].contains("row 14 holds 21000, the others assumptions.staff"));
        assert!(warnings[1].ends_with("=IF(ROW() = 10, 21000, assumptions.staff)"));
        assert!(warnings[2].starts_with("summary.h2_revenue_over_h1: imported as a value"));

        let text = fs::read_to_string(&yaml).unwrap();
        for line in [
            "volume_kg:\n    formula: =IF(ROW() = 1, assumptions.opening_volume, PREV(volume_kg, 0) * (1 + assumptions.volume_growth))",
            "revenue:\n    formula: =SUM(monthly.revenue)",
            "price:\n    formula: =monthly_total.revenue / monthly_total.volume_kg",
            "revenue:\n    formula: =monthly_total.revenue",
            "    unit: kg a month\n    notes: January, from the 2026 run rate",
        ] {
            assert!(text.contains(line), "{line}\n{text}");
        }

        let model = calculate_core(&yaml, true, None).unwrap();
        let mut book: Xlsx<_> = open_workbook(example("budget.xlsx")).unwrap();
        let columns = [
            "volume_kg",
            "price",
            "revenue",
            "cogs",
            "gross_margin",
            "rent",
            "staff",
            "ebitda",
        ];
        let months: Vec<u32> = (3..9).chain(10..16).collect();
        for (i, column) in columns.iter().enumerate() {
            let col = u32::try_from(i).unwrap() + 1;
            let forge: Vec<f64> = model.tables["monthly"].values[*column]
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(forge.len(), 12, "{column}");
            for (value, row) in forge.iter().zip(&months) {
                let cell = stored(&mut book, "Monthly", (*row, col));
                assert!(close(*value, cell), "{column} {row}: {value} vs {cell}");
            }
            let total = model.scalars[&format!("monthly_total.{column}")].unwrap();
            let cell = stored(&mut book, "Monthly", (16, col));
            assert!(close(total, cell), "total {column}: {total} vs {cell}");
        }
        let summary = [
            "revenue",
            "gross_margin",
            "gross_margin_pct",
            "ebitda",
            "tax",
            "net_income",
            "h2_revenue_over_h1",
        ];
        for (i, name) in summary.iter().enumerate() {
            let value = model.scalars[&format!("summary.{name}")].unwrap();
            let cell = stored(&mut book, "Summary", (u32::try_from(i).unwrap() + 2, 1));
            assert!(close(value, cell), "{name}: {value} vs {cell}");
        }
        assert!(close(
            model.scalars["summary.net_income"].unwrap(),
            656_435.464_438_365
        ));
    }

    fn find<'a>(deps: &'a [AuditDep], name: &str) -> Option<&'a AuditDep> {
        deps.iter().find_map(|d| {
            (d.name == name)
                .then_some(d)
                .or_else(|| find(&d.children, name))
        })
    }

    /// `forge audit` of net income on the import goes down to October's
    /// typed number: the staff column's formula holds it.
    #[test]
    fn audit_follows_net_income_to_the_typed_number() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("budget.yaml");
        import_core(&example("budget.xlsx"), &yaml, false, false).unwrap();
        let audit = audit_core(&yaml, "summary.net_income").unwrap();
        assert!(audit.is_valid);
        assert_eq!(
            audit.formula.as_deref(),
            Some("=summary.ebitda - summary.tax")
        );
        let staff = find(&audit.dependencies, "monthly.staff").unwrap();
        assert_eq!(
            staff.formula.as_deref(),
            Some("=IF(ROW() = 10, 21000, assumptions.staff)")
        );
        assert_eq!(staff.children[0].name, "assumptions.staff");
        assert_eq!(staff.children[0].value, Some(21_000.0));
        let total = find(&audit.dependencies, "monthly_total.ebitda").unwrap();
        assert_eq!(total.formula.as_deref(), Some("=SUM(monthly.ebitda)"));
    }

    /// The pay rise in budget.yaml, with October reading the assumption,
    /// and as it would have been with October still typed in.
    #[test]
    fn the_pay_rise_with_and_without_october_typed_in() {
        let budget = calculate_core(&example("budget.yaml"), true, None).unwrap();
        let net = budget.scalars["summary.net_income"].unwrap();
        assert!(close(net, 642_935.464_438_365), "{net}");
        let h2 = budget.scalars["summary.h2_revenue_over_h1"].unwrap();
        assert!(close(h2, 0.137_180_994_500_346), "{h2}");

        let dir = tempfile::tempdir().unwrap();
        let trap = dir.path().join("trap.yaml");
        let text = fs::read_to_string(example("budget.yaml")).unwrap();
        let typed = text.replace(
            "formula: =assumptions.staff\n",
            "formula: =IF(ROW() = 10, 21000, assumptions.staff)\n",
        );
        assert_ne!(typed, text);
        fs::write(&trap, typed).unwrap();
        let trapped = calculate_core(&trap, true, None).unwrap();
        let net_trapped = trapped.scalars["summary.net_income"].unwrap();
        assert!(close(net_trapped, 644_060.464_438_365), "{net_trapped}");
        assert!(close(net_trapped - net, 1_125.0));
    }
}

#[cfg(test)]
mod printed_from_the_result {
    //! What `forge export` and `forge import` print is in their cores'
    //! results (T1-CLI-FROM-CORES).

    use crate::cli::commands::{export_core, import_core};
    use std::path::Path;

    #[test]
    fn export_names_why_a_workbook_was_not_checked() {
        let dir = tempfile::tempdir().unwrap();
        let failing = dir.path().join("f.yaml");
        std::fs::write(
            &failing,
            "_forge_version: \"5.2.0\"\nr:\n  missing: {formula: \"=MATCH(99, 1, 0)\"}\n",
        )
        .unwrap();
        let result = export_core(&failing, &dir.path().join("f.xlsx")).unwrap();
        let reason = result.not_checked.clone().unwrap();
        assert_eq!(result.warnings[0], reason);
        let fine = export_core(
            Path::new("examples/quota_forecast.yaml"),
            &dir.path().join("b.xlsx"),
        )
        .unwrap();
        assert!(fine.not_checked.is_none());
        let json = serde_json::to_value(&fine).unwrap();
        assert!(json.get("not_checked").is_none(), "{json}");
    }

    #[test]
    fn import_gives_each_table_the_formulas_kept_and_the_files_written() {
        let dir = tempfile::tempdir().unwrap();
        let workbook = dir.path().join("b.xlsx");
        export_core(Path::new("examples/bagel/bagel.yaml"), &workbook).unwrap();
        let single = import_core(&workbook, &dir.path().join("b.yaml"), false, false).unwrap();
        // As the file it wrote reads: the data columns, the formulas apart
        let written = crate::parser::parse_model(&dir.path().join("b.yaml")).unwrap();
        let shapes: Vec<(&str, usize, usize)> = single
            .tables
            .iter()
            .map(|t| (t.name.as_str(), t.columns, t.rows))
            .collect();
        let expected: Vec<(&str, usize, usize)> = written
            .tables
            .values()
            .map(|t| (t.name.as_str(), t.columns.len(), t.row_count()))
            .collect();
        assert_eq!(shapes, expected);
        assert_eq!(shapes[0].0, "months");
        assert!(single.formulas_kept > 0);
        assert_eq!(single.files_written, Vec::<String>::new());
        // A model whose parts do not refer to each other can be split
        let plain = dir.path().join("q.xlsx");
        export_core(Path::new("examples/quota_forecast.yaml"), &plain).unwrap();
        let split = import_core(&plain, &dir.path().join("split"), true, false).unwrap();
        assert_ne!(split.files_written, Vec::<String>::new());
        assert!(split.files_written.iter().all(|f| Path::new(f).exists()));
    }
}
