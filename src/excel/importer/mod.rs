//! Excel importer implementation - Excel (.xlsx) → YAML
//!
//! Reads the layout `forge export` writes (see `workbook_refs`): a sheet whose
//! header is `Name`, `Value` holds scalars, any other sheet is a table. Formulas
//! are translated with every sheet's layout, so references between sheets come
//! back as `table.column` and `group.scalar`. A formula that cannot be
//! expressed in Forge is imported as its value, with a warning.

use super::workbook_refs::{Context, ReverseRefs, SheetKind, WorkbookRefs};
use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;
use calamine::{open_workbook, Data, Reader, Xlsx};
use std::io::{Cursor, Read, Seek};
use std::path::Path;

mod cells_read;
mod layout;
mod reserved;
mod runs;
mod scalars;
mod sheet;
mod table;

use sheet::{cells_of, date_of, Cells, Sheet};

/// The names of the workbook and what each column holds, to read the dates
/// of a formula as Forge reads them (`workbook_refs::dates`): for the main
/// model, and for each included file's own formulas.
#[derive(Default)]
struct Names {
    main: WorkbookRefs,
    own: std::collections::HashMap<String, WorkbookRefs>,
}

impl Names {
    /// The names a formula written on `sheet` reads.
    fn of(&self, refs: &ReverseRefs, sheet: &str) -> &WorkbookRefs {
        refs.sheets
            .get(sheet)
            .and_then(SheetKind::include)
            .and_then(|namespace| self.own.get(namespace))
            .unwrap_or(&self.main)
    }
}

/// Excel importer for converting .xlsx files to Forge models
pub struct ExcelImporter {
    source: Source,
}

/// Where the workbook comes from: a file, or its bytes (a browser has no files)
enum Source {
    Path(std::path::PathBuf),
    Bytes(Vec<u8>),
}

impl ExcelImporter {
    /// Create a new Excel importer
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            source: Source::Path(path.as_ref().to_path_buf()),
        }
    }

    /// An importer for a workbook held in memory, the bytes of an .xlsx file.
    #[must_use]
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            source: Source::Bytes(bytes),
        }
    }

    /// Import Excel file to `ParsedModel`
    ///
    /// # Errors
    ///
    /// Returns an error if the Excel file cannot be opened or parsed.
    pub fn import(&self) -> ForgeResult<ParsedModel> {
        self.import_with_warnings().map(|(model, _)| model)
    }

    /// Import, and return one warning per formula that was imported as values.
    ///
    /// # Errors
    ///
    /// Returns an error if the Excel file cannot be opened or parsed.
    pub fn import_with_warnings(&self) -> ForgeResult<(ParsedModel, Vec<String>)> {
        let fail =
            |e: calamine::XlsxError| ForgeError::IO(format!("Failed to open Excel file: {e}"));
        let (mut sheets, defined) = match &self.source {
            Source::Path(path) => Self::sheets(open_workbook(path).map_err(fail)?),
            Source::Bytes(bytes) => {
                Self::sheets(Xlsx::new(Cursor::new(bytes.as_slice())).map_err(fail)?)
            },
        };

        // A workbook's names stand for what they name (`_xlnm.` names are Excel's own)
        let workbook_names: std::collections::HashMap<String, String> = defined
            .into_iter()
            .filter(|(name, _)| !name.starts_with("_xlnm."))
            .map(|(name, target)| (name.to_lowercase(), target))
            .collect();
        let reader = ReverseRefs {
            names: workbook_names.clone(),
            ..ReverseRefs::default()
        };
        cells_read::mark(&mut sheets, &reader);
        let mut refs = Self::layout(&sheets);
        refs.names = workbook_names;
        let names = Self::names(&sheets, &refs);
        let read = (&refs, &names);
        let namespaces = Self::namespaces(&sheets);
        let mut model = ParsedModel::new();
        let mut warnings = Vec::new();
        for sheet in &sheets {
            if sheet.holds_data(0) {
                warnings.push(format!(
                    "{}: its first row holds a value or a formula, and is read as the header, \
                     so that row is not in the model; give the sheet a header or a title row",
                    sheet.name
                ));
            }
            if let Some(namespace) = Self::include_namespace(sheet) {
                Self::import_include(sheet, &namespace, read, &mut model, &mut warnings);
            } else if sheet.is_scalar_sheet() {
                let group = Self::scalar_group(&sheet.name);
                Self::import_scalars(sheet, group.as_deref(), read, &mut model, &mut warnings);
            } else if let Some((namespace, _)) = Self::include_table(&sheet.name, &namespaces) {
                // A table of an included file goes into that file
                let included = Self::included_model(&mut model, &namespace);
                Self::import_table(sheet, read, included, &mut warnings);
            } else {
                Self::import_table(sheet, read, &mut model, &mut warnings);
            }
        }
        reserved::rename_network_results(&mut model, &mut warnings);
        Ok((model, warnings))
    }

    /// Every worksheet with something in it, and the workbook's names.
    fn sheets<R: Read + Seek>(mut workbook: Xlsx<R>) -> (Vec<Sheet>, Vec<(String, String)>) {
        let mut sheets = Vec::new();
        for name in workbook.sheet_names() {
            // A sheet that cannot be read is skipped, as an empty one is
            let values = sheet::finite_cells(workbook.worksheet_range(&name).unwrap_or_default());
            if values.is_empty() {
                continue;
            }
            let formulas = workbook.worksheet_formula(&name).ok();
            let styles = workbook.worksheet_style(&name).ok();
            let merged = workbook
                .merge_cells_by_sheet_name(&name)
                .unwrap_or_default();
            let sheet = Sheet::new(name, values, formulas, styles);
            sheets.push(layout::tidy(sheet, &merged));
        }
        (sheets, workbook.defined_names().to_vec())
    }

    /// The cells of a table's column, below the header.
    fn column_cells(sheet: &Sheet, col: usize) -> impl Iterator<Item = &Data> {
        let (height, _) = sheet.values.get_size();
        (1..height).filter_map(move |row| sheet.values.get((row, col)))
    }

    /// The names of the workbook, and which columns hold dates, text or
    /// numbers (ADR-054).
    ///
    /// A column Excel shows as dates is a column of dates in Forge if its
    /// cells are values, or its formula gives a date (`EDATE(d, 1)`, `IF(c,
    /// d, e)`). A formula such as `d + 30` gives a number in Forge, the day
    /// number the cell holds, so that column is one of numbers.
    fn names(sheets: &[Sheet], refs: &ReverseRefs) -> Names {
        let mut names = Names::default();
        // (the names it is in, table key there, column, sheet, column index)
        let mut dated: Vec<(Option<String>, String, String, &Sheet, usize)> = Vec::new();
        for sheet in sheets {
            // `layout` has a kind for every sheet
            match &refs.sheets[&sheet.name] {
                SheetKind::Table { name, include, .. } => {
                    let (_, width) = sheet.values.get_size();
                    let columns: Vec<String> =
                        (0..width).map(|col| sheet.column_name(col)).collect();
                    let letters = |columns: &[String]| {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(i, column)| (column.clone(), sheet.letter(i)))
                            .collect()
                    };
                    // The main model reads an included table as `@ns.table`;
                    // the included file's own formulas read it as `table`
                    let mut places: Vec<(Option<String>, String)> = Vec::new();
                    match include {
                        Some(namespace) => {
                            places.push((None, format!("@{namespace}.{name}")));
                            places.push((Some(namespace.clone()), name.clone()));
                        },
                        None => places.push((None, name.clone())),
                    }
                    for (place, key) in places {
                        let target = match &place {
                            Some(namespace) => names.own.entry(namespace.clone()).or_default(),
                            None => &mut names.main,
                        };
                        target.columns.insert(key.clone(), letters(&columns));
                        for (col, column) in columns.iter().enumerate() {
                            match cells_of(&mut Self::column_cells(sheet, col)) {
                                Cells::Dates => {
                                    target.add_date_columns(&key, [column.as_str()]);
                                    if sheet.formula(1, col).is_some() {
                                        dated.push((
                                            place.clone(),
                                            key.clone(),
                                            column.clone(),
                                            sheet,
                                            col,
                                        ));
                                    }
                                },
                                Cells::Numbers => {
                                    target.add_column_types(&key, [(column.as_str(), false)]);
                                },
                                Cells::Text => {
                                    target.add_column_types(&key, [(column.as_str(), true)]);
                                    // Text with a digit: Forge reads a number
                                    // from it its own way (a date, "5")
                                    let dated = Self::column_cells(sheet, col).any(|cell| {
                                        matches!(cell, Data::String(text)
                                            if text.bytes().any(|b| b.is_ascii_digit()))
                                    });
                                    if dated {
                                        target.add_mixed_columns(&key, [column.as_str()]);
                                    }
                                },
                                Cells::Other => {},
                            }
                        }
                    }
                },
                SheetKind::Scalars { rows, include } => {
                    for path in rows.values() {
                        let cell = (sheet.name.clone(), 0);
                        match include {
                            Some(namespace) => {
                                names
                                    .main
                                    .scalars
                                    .insert(format!("@{namespace}.{path}"), cell.clone());
                                names
                                    .own
                                    .entry(namespace.clone())
                                    .or_default()
                                    .scalars
                                    .insert(path.clone(), cell);
                            },
                            None => {
                                names.main.scalars.insert(path.clone(), cell);
                            },
                        }
                    }
                },
            }
        }
        Self::settle_dates(&mut names, dated, refs);
        Self::settle_date_scalars(&mut names, sheets, refs);
        names
    }

    /// Which scalars are dates (ADR-060): a date cell with no formula, and a
    /// formula whose value is a date in Forge, whatever the cell's format
    /// (a formula reading a date scalar may make another one a date).
    fn settle_date_scalars(names: &mut Names, sheets: &[Sheet], refs: &ReverseRefs) {
        // (namespace, path, sheet, data row)
        let mut formulas: Vec<(Option<String>, String, &Sheet, usize)> = Vec::new();
        let mut dates: Vec<(Option<String>, String)> = Vec::new();
        for sheet in sheets {
            let Some(SheetKind::Scalars { rows, include }) = refs.sheets.get(&sheet.name) else {
                continue;
            };
            let (height, _) = sheet.values.get_size();
            for row in 1..height {
                let Some(path) = rows.get(&sheet.excel_row(row)) else {
                    continue;
                };
                if sheet.formula(row, 1).is_some() {
                    formulas.push((include.clone(), path.clone(), sheet, row));
                } else if sheet.values.get((row, 1)).and_then(date_of).is_some() {
                    dates.push((include.clone(), path.clone()));
                }
            }
        }
        loop {
            for (namespace, path) in std::mem::take(&mut dates) {
                match namespace {
                    Some(namespace) => {
                        names
                            .main
                            .date_scalars
                            .insert(format!("@{namespace}.{path}"));
                        names
                            .own
                            .entry(namespace)
                            .or_default()
                            .date_scalars
                            .insert(path);
                    },
                    None => {
                        names.main.date_scalars.insert(path);
                    },
                }
            }
            let found = formulas.iter().position(|(_, path, sheet, row)| {
                let group = Self::scalar_group(&sheet.name);
                let group = match refs.sheets.get(&sheet.name).and_then(SheetKind::include) {
                    Some(_) => path.rsplit_once('.').map(|(g, _)| g.to_string()),
                    None => group,
                };
                sheet
                    .formula(*row, 1)
                    .and_then(|excel| {
                        refs.to_forge(&excel, &sheet.name, sheet.excel_row(*row))
                            .ok()
                    })
                    .is_some_and(|forge| {
                        names.of(refs, &sheet.name).reads_as_a_date(
                            &forge,
                            Context::Scalar {
                                group: group.as_deref(),
                            },
                        )
                    })
            });
            let Some(index) = found else { break };
            let (namespace, path, ..) = formulas.remove(index);
            dates.push((namespace, path));
        }
    }

    /// A formula column shown as dates whose formula gives a number is a
    /// column of numbers, which may make another column one too: settle
    /// which of the `dated` columns are columns of dates.
    fn settle_dates(
        names: &mut Names,
        mut dated: Vec<(Option<String>, String, String, &Sheet, usize)>,
        refs: &ReverseRefs,
    ) {
        loop {
            let demoted = dated.iter().position(|(place, key, column, sheet, col)| {
                let target = place
                    .as_ref()
                    .and_then(|namespace| names.own.get(namespace))
                    .unwrap_or(&names.main);
                let table = key.as_str();
                let is_date = sheet
                    .formula(1, *col)
                    .and_then(|excel| refs.to_forge(&excel, &sheet.name, sheet.excel_row(1)).ok())
                    .is_none_or(|forge| {
                        target.reads_as_a_date(&forge, Context::Row { table, row: 2 })
                    });
                !is_date && target.date_columns.contains(&format!("{key}.{column}"))
            });
            let Some(index) = demoted else { break };
            let (place, key, column, ..) = dated.remove(index);
            let target = match &place {
                Some(namespace) => names.own.entry(namespace.clone()).or_default(),
                None => &mut names.main,
            };
            target.date_columns.remove(&format!("{key}.{column}"));
            target.add_column_types(&key, [(column.as_str(), false)]);
        }
    }

    /// Scalar group for a sheet name; the `Scalars` sheet holds ungrouped ones.
    fn scalar_group(sheet_name: &str) -> Option<String> {
        if sheet_name.eq_ignore_ascii_case(super::workbook_refs::UNGROUPED_SHEET) {
            None
        } else {
            Some(Self::sanitize_table_name(sheet_name))
        }
    }

    /// Where every column and scalar is, for translating formulas.
    fn layout(sheets: &[Sheet]) -> ReverseRefs {
        let mut refs = ReverseRefs::default();
        let namespaces = Self::namespaces(sheets);
        for sheet in sheets {
            let (_, width) = sheet.values.get_size();
            let kind = match Self::include_namespace(sheet) {
                Some(namespace) => {
                    let prefix = format!("{namespace}.");
                    let rows = Self::scalar_names(sheet)
                        .map(|(row, name)| (row, name[prefix.len()..].to_string()))
                        .collect();
                    SheetKind::Scalars {
                        rows,
                        include: Some(namespace),
                    }
                },
                None if sheet.is_scalar_sheet() => {
                    let group = Self::scalar_group(&sheet.name);
                    let rows = Self::scalar_names(sheet)
                        .map(|(row, name)| {
                            let path = group
                                .as_ref()
                                .map_or_else(|| name.clone(), |g| format!("{g}.{name}"));
                            (row, path)
                        })
                        .collect();
                    SheetKind::Scalars {
                        rows,
                        include: None,
                    }
                },
                None => {
                    let columns = (0..width)
                        .map(|col| {
                            let name = sheet.column_name(col);
                            (sheet.letter(col), name)
                        })
                        .collect();
                    let (include, name) = match Self::include_table(&sheet.name, &namespaces) {
                        Some((ns, table)) => (Some(ns), table),
                        None => (None, Self::sanitize_table_name(&sheet.name)),
                    };
                    SheetKind::Table {
                        columns,
                        include,
                        rows: sheet.data_rows(),
                        ends: sheet.column_ends(),
                        total: sheet.total_group(&name),
                        name,
                    }
                },
            };
            refs.sheets.insert(sheet.name.clone(), kind);
        }
        refs
    }

    /// Excel row and name of each scalar on a `Name` / `Value` sheet.
    fn scalar_names(sheet: &Sheet) -> impl Iterator<Item = (u32, String)> + '_ {
        sheet
            .labels()
            .into_iter()
            .map(|(row, name)| (sheet.excel_row(row), name))
    }

    /// The namespaces of the included files whose sheets the workbook has.
    fn namespaces(sheets: &[Sheet]) -> Vec<String> {
        sheets.iter().filter_map(Self::include_namespace).collect()
    }

    /// For a sheet `{namespace}.{table}` of an included file whose
    /// `{namespace}.Scalars` sheet the workbook has: the namespace and the
    /// table's name in that file.
    fn include_table(sheet_name: &str, namespaces: &[String]) -> Option<(String, String)> {
        let (namespace, table) = sheet_name.split_once('.')?;
        (table != "Scalars" && !table.is_empty() && namespaces.iter().any(|n| n == namespace))
            .then(|| (namespace.to_string(), Self::sanitize_table_name(table)))
    }

    /// The model of the included file `namespace`, created the first time.
    fn included_model<'m>(model: &'m mut ParsedModel, namespace: &str) -> &'m mut ParsedModel {
        if !model.resolved_includes.contains_key(namespace) {
            let file = format!("{namespace}.yaml");
            let include = crate::types::Include::new(file.clone(), namespace.to_string());
            model.add_include(include.clone());
            model.resolved_includes.insert(
                namespace.to_string(),
                crate::types::ResolvedInclude {
                    include,
                    resolved_path: std::path::PathBuf::from(file),
                    model: ParsedModel::new(),
                },
            );
        }
        &mut model
            .resolved_includes
            .get_mut(namespace)
            .expect("inserted above")
            .model
    }

    /// The namespace of a sheet `forge export` writes for an included file:
    /// named `{namespace}.Scalars`, every name starting with `{namespace}.`.
    fn include_namespace(sheet: &Sheet) -> Option<String> {
        let namespace = sheet.name.strip_suffix(".Scalars")?;
        let valid = !namespace.is_empty()
            && namespace
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid || !sheet.is_scalar_sheet() {
            return None;
        }
        // Every name starts with `{namespace}.`; no name at all is an
        // included file with tables only
        let prefix = format!("{namespace}.");
        let all_prefixed = Self::scalar_names(sheet)
            .all(|(_, n)| n.len() > prefix.len() && n.starts_with(&prefix));
        all_prefixed.then(|| namespace.to_string())
    }
}

#[cfg(test)]
// Financial math: exact float comparison validated against Excel/Gnumeric/R
mod tests {
    use super::*;
    use crate::types::{Column, ColumnValue, Table, Variable};

    #[test]
    fn test_import_nonexistent_file_fails() {
        let importer = ExcelImporter::new("/nonexistent/file.xlsx");
        let result = importer.import();
        assert!(result.is_err());
    }

    #[test]
    fn test_import_simple_excel_file() {
        use crate::excel::exporter::ExcelExporter;
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        let dir = TempDir::new().unwrap();
        let excel_path = dir.path().join("test.xlsx");
        let exporter = ExcelExporter::new(model);
        exporter.export(&excel_path).unwrap();

        let importer = ExcelImporter::new(&excel_path);
        let result = importer.import().unwrap();
        assert!(result.tables.contains_key("sales"));
        let table = result.tables.get("sales").unwrap();
        assert!(table.columns.contains_key("revenue"));
    }

    #[test]
    fn test_import_with_text_column() {
        use crate::excel::exporter::ExcelExporter;
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("products".to_string());
        table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        model.add_table(table);

        let dir = TempDir::new().unwrap();
        let excel_path = dir.path().join("test_text.xlsx");
        let exporter = ExcelExporter::new(model);
        exporter.export(&excel_path).unwrap();

        let importer = ExcelImporter::new(&excel_path);
        let result = importer.import().unwrap();
        assert!(result.tables.contains_key("products"));
        let table = result.tables.get("products").unwrap();
        assert!(table.columns.contains_key("name"));
    }

    #[test]
    fn test_import_multiple_tables() {
        use crate::excel::exporter::ExcelExporter;
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table1 = Table::new("revenue".to_string());
        table1.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0]),
        ));
        model.add_table(table1);
        let mut table2 = Table::new("costs".to_string());
        table2.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![500.0, 750.0]),
        ));
        model.add_table(table2);

        let dir = TempDir::new().unwrap();
        let excel_path = dir.path().join("multi.xlsx");
        let exporter = ExcelExporter::new(model);
        exporter.export(&excel_path).unwrap();

        let importer = ExcelImporter::new(&excel_path);
        let result = importer.import().unwrap();
        assert!(result.tables.contains_key("revenue"));
        assert!(result.tables.contains_key("costs"));
    }

    #[test]
    fn test_import_with_scalars() {
        use crate::excel::exporter::ExcelExporter;
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.add_scalar(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.15), None),
        );

        let dir = TempDir::new().unwrap();
        let excel_path = dir.path().join("scalars.xlsx");
        let exporter = ExcelExporter::new(model);
        exporter.export(&excel_path).unwrap();

        let importer = ExcelImporter::new(&excel_path);
        let result = importer.import().unwrap();
        let _ = &result;
    }

    #[test]
    fn test_importer_new_stores_path() {
        let path = std::path::Path::new("/some/path/file.xlsx");
        let importer = ExcelImporter::new(path);
        assert!(matches!(&importer.source, Source::Path(p) if p == path));
    }

    #[test]
    fn test_importer_reads_bytes_as_it_reads_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let xlsx = dir.path().join("bagel.xlsx");
        let bagel =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bagel/bagel.yaml");
        crate::excel::ExcelExporter::new(crate::parser::parse_model(&bagel).unwrap())
            .export(&xlsx)
            .unwrap();
        let (from_file, file_warnings) = ExcelImporter::new(&xlsx).import_with_warnings().unwrap();
        let (from_bytes, byte_warnings) = ExcelImporter::from_bytes(std::fs::read(&xlsx).unwrap())
            .import_with_warnings()
            .unwrap();
        assert_eq!(file_warnings, byte_warnings);
        assert_eq!(
            serde_json::to_string(&from_file.scalars).unwrap(),
            serde_json::to_string(&from_bytes.scalars).unwrap()
        );
        assert_eq!(
            serde_json::to_string(&from_file.tables).unwrap(),
            serde_json::to_string(&from_bytes.tables).unwrap()
        );
        assert!(ExcelImporter::from_bytes(b"not a workbook".to_vec())
            .import()
            .is_err());
    }

    #[test]
    fn test_import_keeps_row_formulas_on_their_own_columns() {
        use crate::excel::exporter::ExcelExporter;
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        table.add_column(Column::new(
            "cogs".to_string(),
            ColumnValue::Number(vec![40.0, 90.0, 120.0]),
        ));
        table.add_row_formula("profit".to_string(), "=revenue - cogs".to_string());
        table.add_row_formula("margin".to_string(), "=profit / revenue".to_string());
        model.add_table(table);

        let dir = TempDir::new().unwrap();
        let excel_path = dir.path().join("formulas.xlsx");
        ExcelExporter::new(model).export(&excel_path).unwrap();

        let imported = ExcelImporter::new(&excel_path).import().unwrap();
        let table = &imported.tables["sales"];

        // Data columns stay data, formula columns stay formulas, with names restored.
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns.contains_key("revenue"));
        assert!(table.columns.contains_key("cogs"));
        assert_eq!(table.row_formulas["profit"], "=revenue - cogs");
        assert_eq!(table.row_formulas["margin"], "=profit / revenue");
    }

    mod import_gaps {
        //! `forge import` says what it cannot keep (roadmap T1-IMPORT-MIXED), and
        //! keeps a share of the total as a formula since ADR-058 (T1-IMPORT-AGGREGATE
        //! imported it as values).
        //!
        //! The numbers are worked by hand: shares of 10, 20, 30 and 40 over their
        //! total of 100 are 0.1, 0.2, 0.3 and 0.4.

        use crate::cli::{calculate_core, import_core};
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
        fn a_share_of_the_total_imports_as_a_formula() {
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
            assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
            let yaml = std::fs::read_to_string(&out).unwrap();
            assert!(yaml.contains("share: =n / SUM(n)"), "{yaml}");

            // The model calculates, to the workbook's values
            let result = calculate_core(&out, true, None).unwrap();
            let share: Vec<f64> = result.tables["shares"].values["share"]
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(share, [0.1, 0.2, 0.3, 0.4]);
        }

        /// A range over part of a column is not the column: `SUM(A2:A3)` of 10,
        /// 20, 30 and 40 is 30, and a running total `SUM($A$2:A3)` is 30 in
        /// the second row. Both came in as the sum of the whole column, 100.
        #[test]
        fn a_range_over_part_of_a_column_is_not_the_column() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("part.xlsx");
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet().set_name("sales").unwrap();
            sheet.write_string(0, 0, "amount").unwrap();
            sheet.write_string(0, 1, "so_far").unwrap();
            let mut so_far = 0.0;
            for (i, n) in [10.0, 20.0, 30.0, 40.0].iter().enumerate() {
                let row = u32::try_from(i).unwrap() + 1;
                so_far += n;
                sheet.write_number(row, 0, *n).unwrap();
                let running = Formula::new(format!("=SUM($A$2:A{})", row + 1))
                    .set_result(format!("{so_far}"));
                sheet.write_formula(row, 1, running).unwrap();
            }
            let scalars = workbook.add_worksheet().set_name("Scalars").unwrap();
            scalars.write_string(0, 0, "Name").unwrap();
            scalars.write_string(0, 1, "Value").unwrap();
            scalars.write_string(1, 0, "first_two").unwrap();
            let first_two = Formula::new("=SUM(sales!A2:A3)").set_result("30");
            scalars.write_formula(1, 1, first_two).unwrap();
            scalars.write_string(2, 0, "all").unwrap();
            let all = Formula::new("=SUM(sales!A2:A5)").set_result("100");
            scalars.write_formula(2, 1, all).unwrap();
            workbook.save(&xlsx).unwrap();

            let out = dir.path().join("part.yaml");
            let imported = import_core(&xlsx, &out, false, false).unwrap();
            let warnings = imported.warnings.join("\n");
            assert!(
                warnings.contains("sales.so_far: imported as values"),
                "{warnings}"
            );
            assert!(
                warnings.contains("first_two: imported as a value"),
                "{warnings}"
            );
            assert!(warnings.contains("part of a column"), "{warnings}");
            let result = calculate_core(&out, true, None).unwrap();
            assert_eq!(result.scalars["first_two"], Some(30.0));
            // The whole column is still the column
            assert_eq!(result.scalars["all"], Some(100.0));
            let yaml = std::fs::read_to_string(&out).unwrap();
            assert!(yaml.contains("formula: =SUM(sales.amount)"), "{yaml}");
            let so_far: Vec<f64> = result.tables["sales"].values["so_far"]
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(so_far, [10.0, 30.0, 60.0, 100.0]);
        }

        /// ADR-059: an empty cell among numbers is imported as an empty cell (null,
        /// model format 5.3.0), so AVERAGE of 10, empty, 30, empty, 0 is 40/3, as in
        /// Excel (it was 8 when the empty cells came in as 0).
        #[test]
        fn an_empty_cell_among_numbers_stays_empty() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("gaps.xlsx");
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet().set_name("gaps").unwrap();
            sheet.write_string(0, 0, "x").unwrap();
            sheet.write_number(1, 0, 10.0).unwrap();
            sheet.write_number(3, 0, 30.0).unwrap();
            sheet.write_number(5, 0, 0.0).unwrap();
            sheet.write_string(0, 1, "nothing").unwrap();
            workbook.save(&xlsx).unwrap();

            let out = dir.path().join("gaps.yaml");
            let imported = import_core(&xlsx, &out, false, false).unwrap();
            assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
            let yaml = std::fs::read_to_string(&out).unwrap();
            assert!(yaml.starts_with("_forge_version: \"5.3.0\""), "{yaml}");

            // The model reads the empty cells as Excel does
            let model = format!(
                "{yaml}\ns:\n  average:\n    formula: \"=AVERAGE(gaps.x)\"\n  empty:\n    formula: \"=COUNTBLANK(gaps.x)\"\n  none:\n    formula: \"=COUNTBLANK(gaps.nothing)\"\n"
            );
            std::fs::write(&out, model).unwrap();
            let result = calculate_core(&out, true, None).unwrap();
            assert!((result.scalars["s.average"].unwrap() - 40.0 / 3.0).abs() < 1e-12);
            assert_eq!(result.scalars["s.empty"], Some(2.0));
            // A column with a name and no value is a column of empty cells
            assert_eq!(result.scalars["s.none"], Some(5.0));
        }
    }

    mod not_finite {
        //! A workbook an older Forge exported can hold a formula whose cached
        //! result is `NaN` (`=1E309 - 1E309` was written as `inf - inf`). It
        //! was imported as `.nan`, which no model may hold; it is imported as
        //! Excel's #NUM! is, and the model reads.

        use crate::cli::import_core;
        use rust_xlsxwriter::{Formula, Workbook};

        #[test]
        fn a_cached_nan_is_imported_as_an_error_cell() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("old.xlsx");
            let mut workbook = Workbook::new();
            let scalars = workbook.add_worksheet().set_name("out").unwrap();
            scalars.write_string(0, 0, "Name").unwrap();
            scalars.write_string(0, 1, "Value").unwrap();
            scalars.write_string(1, 0, "nanres").unwrap();
            scalars
                .write_formula(1, 1, Formula::new("=inf - inf").set_result("NaN"))
                .unwrap();
            let table = workbook.add_worksheet().set_name("t").unwrap();
            table.write_string(0, 0, "x").unwrap();
            table.write_number(1, 0, 3.0).unwrap();
            table
                .write_formula(2, 0, Formula::new("=A2 * 1E308 * 10").set_result("NaN"))
                .unwrap();
            workbook.save(&xlsx).unwrap();

            let out = dir.path().join("old.yaml");
            let imported = import_core(&xlsx, &out, false, false).unwrap();
            let yaml = std::fs::read_to_string(&out).unwrap();
            assert!(!yaml.contains(".nan") && !yaml.contains(".inf"), "{yaml}");
            // The cell in the column of numbers is named, as any error cell is
            assert!(
                imported
                    .warnings
                    .iter()
                    .any(|w| w.contains("t.x") && w.contains("A3 (#NUM!)")),
                "{:?}",
                imported.warnings
            );
            // and the model it wrote reads
            crate::parser::parse_model(&out).unwrap();
        }
    }

    mod layout {
        //! Which sheets are what: chart sheets, sheets that look like an
        //! included file's and are not, and the dates of an included table.

        use super::super::ExcelImporter;
        use crate::types::ParsedModel;
        use rust_xlsxwriter::{Chart, ChartType, Format, Formula, Workbook, Worksheet};

        fn import(mut book: Workbook) -> ParsedModel {
            let bytes = book.save_to_buffer().unwrap();
            ExcelImporter::from_bytes(bytes).import().unwrap()
        }

        fn scalar_sheet<'a>(book: &'a mut Workbook, name: &str) -> &'a mut Worksheet {
            let sheet = book.add_worksheet().set_name(name).unwrap();
            sheet.write_string(0, 0, "Name").unwrap();
            sheet.write_string(0, 1, "Value").unwrap();
            sheet
        }

        #[test]
        fn a_chart_sheet_is_not_a_table() {
            let mut book = Workbook::new();
            let t = book.add_worksheet().set_name("t").unwrap();
            t.write_string(0, 0, "n").unwrap();
            t.write_number(1, 0, 1.0).unwrap();
            t.write_number(2, 0, 2.0).unwrap();
            let mut chart = Chart::new(ChartType::Column);
            chart.add_series().set_values("t!$A$2:$A$3");
            book.add_chartsheet()
                .set_name("chart")
                .unwrap()
                .insert_chart(0, 0, &chart)
                .unwrap();
            let model = import(book);
            let names: Vec<&String> = model.tables.keys().collect();
            assert_eq!(names, ["t"]);
        }

        #[test]
        fn a_sheet_named_like_an_included_files_is_one_only_when_it_is() {
            let mut book = Workbook::new();
            // Not a namespace Forge writes
            let s = scalar_sheet(&mut book, "my-ns.Scalars");
            s.write_string(1, 0, "my-ns.x").unwrap();
            s.write_number(1, 1, 1.0).unwrap();
            // Not a Name / Value sheet
            let t = book.add_worksheet().set_name("ns.Scalars").unwrap();
            t.write_string(0, 0, "a").unwrap();
            t.write_number(1, 0, 2.0).unwrap();
            let model = import(book);
            assert_eq!(model.resolved_includes.keys().count(), 0);
            let scalars: Vec<&String> = model.scalars.keys().collect();
            let tables: Vec<&String> = model.tables.keys().collect();
            assert_eq!(scalars, ["my_nsscalars.my-ns.x"]);
            assert_eq!(tables, ["nsscalars"]);
        }

        #[test]
        fn an_included_tables_date_columns_are_settled_in_its_own_file() {
            let mut book = Workbook::new();
            let date = Format::new().set_num_format("yyyy-mm-dd");
            let c = scalar_sheet(&mut book, "costs.Scalars");
            c.write_string(1, 0, "costs.rate").unwrap();
            c.write_number(1, 1, 1.0).unwrap();
            let p = book.add_worksheet().set_name("costs.prices").unwrap();
            for (col, name) in ["start", "next", "plus"].into_iter().enumerate() {
                p.write_string(0, u16::try_from(col).unwrap(), name)
                    .unwrap();
            }
            for row in 1..=2 {
                let r = row + 1;
                p.write_number_with_format(row, 0, 46_023.0, &date).unwrap();
                let next = Formula::new(format!("=EDATE(A{r},1)")).set_result("46054");
                p.write_formula_with_format(row, 1, next, &date).unwrap();
                let plus = Formula::new(format!("=A{r}+30")).set_result("46053");
                p.write_formula_with_format(row, 2, plus, &date).unwrap();
            }
            let model = import(book);
            let prices = &model.resolved_includes["costs"].model.tables["prices"];
            assert_eq!(prices.row_formulas["next"], "=EDATE(start, 1)");
            assert_eq!(prices.row_formulas["plus"], "=start + 30");
            // A day plus 30 is a day number, so its date format is not kept
            assert!(!prices.formula_metadata.contains_key("plus"));
        }
    }
}
