//! Excel importer implementation - Excel (.xlsx) → YAML
//!
//! Reads the layout `forge export` writes (see `workbook_refs`): a sheet whose
//! header is `Name`, `Value` holds scalars, any other sheet is a table. Formulas
//! are translated with every sheet's layout, so references between sheets come
//! back as `table.column` and `group.scalar`. A formula that cannot be
//! expressed in Forge is imported as its value, with a warning.

use super::workbook_refs::dates::{excel_date, Holds};
use super::workbook_refs::{column_letter, Context, ReverseRefs, SheetKind, WorkbookRefs};
use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::io::{Cursor, Read, Seek};
use std::path::Path;

/// One worksheet read from the file.
struct Sheet {
    name: String,
    values: Range<Data>,
    formulas: Option<Range<String>>,
}

impl Sheet {
    /// Cell text by 0-based position relative to the used range of values.
    fn header(&self, col: usize) -> Option<String> {
        match self.values.get((0, col))? {
            Data::String(s) => Some(s.clone()),
            Data::Int(i) => Some(i.to_string()),
            Data::Float(f) => Some(f.to_string()),
            _ => None,
        }
    }

    /// `Name` / `Value` (optionally `Formula`) header: a scalar sheet.
    fn is_scalar_sheet(&self) -> bool {
        let (_, width) = self.values.get_size();
        let name = self
            .header(0)
            .is_some_and(|h| h.eq_ignore_ascii_case("name"));
        let value = self
            .header(1)
            .is_some_and(|h| h.eq_ignore_ascii_case("value"));
        let third = width < 3
            || self
                .header(2)
                .is_some_and(|h| h.eq_ignore_ascii_case("formula"));
        name && value && third
    }

    /// Formula of the cell at a 0-based position relative to the value range.
    fn formula(&self, row: usize, col: usize) -> Option<String> {
        let (start_row, start_col) = self.values.start()?;
        let r = start_row + u32::try_from(row).ok()?;
        let c = start_col + u32::try_from(col).ok()?;
        let f = self.formulas.as_ref()?.get_value((r, c))?;
        if f.is_empty() {
            None
        } else if f.starts_with('=') {
            Some(f.clone())
        } else {
            Some(format!("={f}"))
        }
    }

    /// Excel row number (1-based) of a 0-based row in the value range.
    fn excel_row(&self, row: usize) -> u32 {
        let start = self.values.start().map_or(0, |(r, _)| r);
        start + u32::try_from(row).unwrap_or(0) + 1
    }

    /// Excel column letter of a 0-based column in the value range.
    fn letter(&self, col: usize) -> String {
        let start = self.values.start().map_or(0, |(_, c)| c as usize);
        column_letter(start + col)
    }
}

/// What the cells of a column hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cells {
    /// Dates: every cell a whole day Excel shows as a date
    Dates,
    Numbers,
    Text,
    Other,
}

/// The date a cell shows, as Forge writes it (`2026-01-31`): a cell with a
/// date format and no time of day, from 1900-03-01 on.
fn date_of(cell: &Data) -> Option<String> {
    let Data::DateTime(value) = cell else {
        return None;
    };
    if !value.is_datetime() {
        return None;
    }
    let (year, month, day, hour, minute, second, milli) = value.to_ymd_hms_milli();
    if (hour, minute, second, milli) != (0, 0, 0, 0) {
        return None;
    }
    let text = format!("{year:04}-{month:02}-{day:02}");
    excel_date(&text).map(|_| text)
}

/// The number a cell holds; a date or a time as Excel's day number.
#[allow(clippy::cast_precision_loss)] // Excel integers fit in f64
fn number_of(cell: &Data) -> Option<f64> {
    match cell {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::DateTime(value) => Some(value.as_f64()),
        _ => None,
    }
}

fn cells_of<'d>(cells: impl IntoIterator<Item = &'d Data>) -> Cells {
    let mut found = None;
    for cell in cells {
        let kind = match cell {
            Data::Empty => continue,
            _ if date_of(cell).is_some() => Cells::Dates,
            Data::Float(_) | Data::Int(_) | Data::DateTime(_) => Cells::Numbers,
            Data::String(_) => Cells::Text,
            _ => Cells::Other,
        };
        found = Some(match (found, kind) {
            (None, kind) => kind,
            (Some(same), kind) if same == kind => same,
            // Dates among other numbers are numbers: their day numbers
            (Some(Cells::Dates | Cells::Numbers), Cells::Dates | Cells::Numbers) => Cells::Numbers,
            _ => Cells::Other,
        });
    }
    found.unwrap_or(Cells::Other)
}

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
        let sheets = match &self.source {
            Source::Path(path) => Self::sheets(open_workbook(path).map_err(fail)?),
            Source::Bytes(bytes) => {
                Self::sheets(Xlsx::new(Cursor::new(bytes.as_slice())).map_err(fail)?)
            },
        };

        let refs = Self::layout(&sheets);
        let names = Self::names(&sheets, &refs);
        let read = (&refs, &names);
        let namespaces = Self::namespaces(&sheets);
        let mut model = ParsedModel::new();
        let mut warnings = Vec::new();
        for sheet in &sheets {
            if let Some(namespace) = Self::include_namespace(sheet) {
                Self::import_include(sheet, &namespace, read, &mut model, &mut warnings);
            } else if sheet.is_scalar_sheet() {
                let group = Self::scalar_group(&sheet.name);
                Self::import_scalars(sheet, group.as_deref(), read, &mut model, &mut warnings);
            } else if let Some((namespace, _)) = Self::include_table(&sheet.name, &namespaces) {
                // A table of an included file goes into that file
                let included = Self::included_model(&mut model, &namespace);
                Self::import_table(sheet, read, included, &mut warnings)?;
            } else {
                Self::import_table(sheet, read, &mut model, &mut warnings)?;
            }
        }
        Ok((model, warnings))
    }

    /// Every worksheet with something in it.
    fn sheets<R: Read + Seek>(mut workbook: Xlsx<R>) -> Vec<Sheet> {
        let mut sheets = Vec::new();
        for name in workbook.sheet_names() {
            let Ok(values) = workbook.worksheet_range(&name) else {
                continue;
            };
            if values.is_empty() {
                continue;
            }
            let formulas = workbook.worksheet_formula(&name).ok();
            sheets.push(Sheet {
                name,
                values,
                formulas,
            });
        }
        sheets
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
            match refs.sheets.get(&sheet.name) {
                Some(SheetKind::Table { name, include, .. }) => {
                    let (_, width) = sheet.values.get_size();
                    let columns: Vec<String> = (0..width)
                        .map(|col| sheet.header(col).unwrap_or_else(|| format!("col_{col}")))
                        .collect();
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
                            match cells_of(Self::column_cells(sheet, col)) {
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
                Some(SheetKind::Scalars { rows, include }) => {
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
                None => {},
            }
        }
        Self::settle_dates(&mut names, dated, refs);
        names
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
                            let name = sheet.header(col).unwrap_or_else(|| format!("col_{col}"));
                            (sheet.letter(col), name)
                        })
                        .collect();
                    let (include, name) = match Self::include_table(&sheet.name, &namespaces) {
                        Some((ns, table)) => (Some(ns), table),
                        None => (None, Self::sanitize_table_name(&sheet.name)),
                    };
                    SheetKind::Table {
                        name,
                        columns,
                        include,
                    }
                },
            };
            refs.sheets.insert(sheet.name.clone(), kind);
        }
        refs
    }

    /// Excel row and name of each scalar on a `Name` / `Value` sheet.
    fn scalar_names(sheet: &Sheet) -> impl Iterator<Item = (u32, String)> + '_ {
        let (height, _) = sheet.values.get_size();
        (1..height).filter_map(|row| {
            let name = sheet.values.get((row, 0))?.to_string();
            (!name.is_empty()).then(|| (sheet.excel_row(row), name))
        })
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

    /// An included file's sheet: its scalars, under their own paths, become
    /// that file's model, read by the main model as `@{namespace}.{path}`.
    fn import_include(
        sheet: &Sheet,
        namespace: &str,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let mut sheet_model = ParsedModel::new();
        Self::import_scalars(sheet, None, read, &mut sheet_model, warnings);
        let prefix = format!("{namespace}.");
        let included = Self::included_model(model, namespace);
        for (name, mut var) in sheet_model.scalars {
            let path = name[prefix.len()..].to_string();
            var.path.clone_from(&path);
            included.add_scalar(path, var);
        }
    }

    /// A `Name` / `Value` sheet: one scalar per row, named `{group}.{name}`.
    fn import_scalars(
        sheet: &Sheet,
        group: Option<&str>,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let (refs, names) = read;
        let (height, _) = sheet.values.get_size();
        for row in 1..height {
            let Some(name) = sheet.values.get((row, 0)).map(ToString::to_string) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let path = group
                .as_ref()
                .map_or_else(|| name.clone(), |g| format!("{g}.{name}"));

            // A scalar is a number: a date is the day number its cell holds
            let value = sheet.values.get((row, 1)).and_then(|cell| match cell {
                Data::Bool(b) => Some(f64::from(u8::from(*b))),
                other => number_of(other),
            });

            let formula = sheet.formula(row, 1).map_or_else(
                // Older workbooks: Forge formula as text in a third column
                || match sheet.values.get((row, 2)) {
                    Some(Data::String(f)) if !f.is_empty() => Some(f.clone()),
                    _ => None,
                },
                |excel| match refs
                    .to_forge(&excel, &sheet.name, sheet.excel_row(row))
                    .and_then(|forge| {
                        names.of(refs, &sheet.name).read_dates(
                            &forge,
                            Context::Scalar { group },
                            Holds::Other,
                        )
                    }) {
                    Ok(forge) => Some(forge),
                    Err(reason) => {
                        warnings.push(format!("{path}: imported as a value ({reason}): {excel}"));
                        None
                    },
                },
            );

            model.add_scalar(path.clone(), Variable::new(path, value, formula));
        }
    }

    /// Any other sheet: a table, header in the first row.
    fn import_table(
        sheet: &Sheet,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        let (refs, names) = read;
        let names = names.of(refs, &sheet.name);
        let (height, width) = sheet.values.get_size();
        if height < 2 {
            return Ok(()); // header only
        }
        let table_name = match refs.sheets.get(&sheet.name) {
            Some(SheetKind::Table { name, .. }) => name.clone(),
            _ => Self::sanitize_table_name(&sheet.name),
        };
        let mut table = Table::new(table_name.clone());

        for col in 0..width {
            let col_name = sheet.header(col).unwrap_or_else(|| format!("col_{col}"));
            // Whether the column is one of dates in Forge (`Self::names`)
            let holds_dates = names
                .date_columns
                .contains(&format!("{table_name}.{col_name}"));

            // A formula in the first data row makes this a calculated column,
            // if every row holds that same formula: a Forge row formula is one
            // formula for the whole column
            if let Some(excel) = sheet.formula(1, col) {
                let whole_column =
                    Self::row_formula(sheet, col, (refs, names), &table_name, holds_dates, &excel);
                // `forge calculate` refuses an aggregate in a row formula, so a
                // column that reads its own total (a share of the total,
                // `=A2/SUM($A$2:$A$5)`) is imported as the values it shows
                let whole_column = whole_column.and_then(|forge| {
                    if crate::core::array_calculator::ArrayCalculator::is_aggregation_formula(
                        &forge,
                    ) {
                        Err("it reads a total of its own column, which a Forge column \
                             formula cannot; put the total in a scalar"
                            .to_string())
                    } else {
                        Ok(forge)
                    }
                });
                match whole_column {
                    Ok(forge) => {
                        table.add_row_formula(col_name, forge);
                        continue;
                    },
                    Err(reason) => warnings.push(format!(
                        "{table_name}.{col_name}: imported as values ({reason}): {excel}"
                    )),
                }
            }

            let mut data: Vec<Data> = (1..height)
                .map(|row| sheet.values.get((row, col)).cloned().unwrap_or(Data::Empty))
                .collect();
            // Columns may differ in length: a shorter one ends in empty cells
            while matches!(data.last(), Some(Data::Empty)) {
                data.pop();
            }
            if data.is_empty() {
                continue;
            }
            let column_value = if cells_of(&data) == Cells::Dates && !holds_dates {
                // Shown as dates, but numbers to the formulas that read them
                ColumnValue::Number(data.iter().map(|c| number_of(c).unwrap_or(0.0)).collect())
            } else {
                Self::convert_to_column_value(&data)?
            };
            if matches!(column_value, ColumnValue::Number(_)) {
                // Text, a boolean or an error among numbers is 0 in Forge
                let not_numbers: Vec<String> = data
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| !matches!(cell, Data::Empty) && number_of(cell).is_none())
                    .map(|(i, cell)| {
                        format!("{}{} ({cell})", sheet.letter(col), sheet.excel_row(i + 1))
                    })
                    .collect();
                if !not_numbers.is_empty() {
                    let shown = not_numbers.iter().take(5).cloned().collect::<Vec<_>>();
                    let more = not_numbers.len() - shown.len();
                    warnings.push(format!(
                        "{table_name}.{col_name}: a column of numbers holds text or another \
                         non-number in {} imported as 0: {}{}",
                        if not_numbers.len() == 1 {
                            "1 cell,"
                        } else {
                            "several cells,"
                        },
                        shown.join(", "),
                        if more > 0 {
                            format!(" and {more} more")
                        } else {
                            String::new()
                        }
                    ));
                }
            }
            // Forge reads a column that starts with a date as a column of
            // dates, and rejects any other text in it
            if let ColumnValue::Text(texts) = &column_value {
                let dated = |t: &String| crate::parser::is_valid_date_format(t);
                if texts.first().is_some_and(dated) && !texts.iter().all(dated) {
                    warnings.push(format!(
                        "{table_name}.{col_name}: starts with a date and holds other text; Forge \
                         reads a column that starts with a date as dates, so `forge calculate` \
                         rejects this file until the column is edited"
                    ));
                }
            }
            table.add_column(Column::new(col_name, column_value));
        }

        model.add_table(table);
        Ok(())
    }

    /// The Forge row formula of a column whose first data row holds the
    /// formula `excel`: the one formula every row holds, or, for a column
    /// whose first row differs from the rest (a running balance), one that
    /// reads the row above.
    fn row_formula(
        sheet: &Sheet,
        col: usize,
        read: (&ReverseRefs, &WorkbookRefs),
        table_name: &str,
        holds_dates: bool,
        excel: &str,
    ) -> Result<String, String> {
        let (refs, names) = read;
        let (height, _) = sheet.values.get_size();
        refs
            .to_forge(excel, &sheet.name, sheet.excel_row(1))
            .and_then(|forge| {
                let other = (2..height).find(|&row| {
                    sheet.formula(row, col).map(|excel| {
                        refs.to_forge(&excel, &sheet.name, sheet.excel_row(row))
                    }) != Some(Ok(forge.clone()))
                });
                if other.is_none() {
                    return Ok(forge);
                }
                // The first rows differ from the rest, as in a running
                // balance: a PREV, whose first value is in those rows
                let rows: Vec<String> = (1..height)
                    .map(|row| {
                        sheet
                            .formula(row, col)
                            .ok_or_else(|| {
                                format!("row {} holds no formula", sheet.excel_row(row))
                            })
                            .and_then(|excel| {
                                refs.to_forge_reading_above(
                                    &excel,
                                    &sheet.name,
                                    sheet.excel_row(row),
                                )
                            })
                    })
                    .collect::<Result<_, _>>()?;
                match super::previous_rows::column_formula(&rows) {
                    Ok(formula) => Ok(formula),
                    // Only the first row differs: correct, if not as written
                    Err(_) if rows[1..].iter().all(|r| *r == rows[1]) => Ok(format!(
                        "=IF(ROW() = 1, {}, {})",
                        forge.trim_start_matches('='),
                        rows[1].trim_start_matches('=')
                    )),
                    Err(index) => Err(format!(
                        "row {} holds another formula than the rows above it, and a \
                         Forge row formula is one formula for the whole column",
                        sheet.excel_row(index + 1)
                    )),
                }
            })
            // A date is read as Forge reads it, or the formula is not
            // the same calculation in Forge (ADR-054)
            .and_then(|forge| {
                let ctx = Context::Row {
                    table: table_name,
                    row: 2,
                };
                let holds = match cells_of(Self::column_cells(sheet, col)) {
                    Cells::Dates if holds_dates => Holds::Dates,
                    Cells::Dates | Cells::Numbers => Holds::Numbers,
                    _ => Holds::Other,
                };
                names.read_dates(&forge, ctx, holds)
            })
    }

    /// Convert Excel Data array to `ColumnValue`
    fn convert_to_column_value(data: &[Data]) -> ForgeResult<ColumnValue> {
        // A column of dates: every cell a whole day shown as a date (ADR-054)
        if cells_of(data) == Cells::Dates && data.iter().all(|c| !matches!(c, Data::Empty)) {
            return Ok(ColumnValue::Date(data.iter().filter_map(date_of).collect()));
        }

        // Detect column type from first non-empty cell
        let first_type = data
            .iter()
            .find(|cell| !matches!(cell, Data::Empty))
            .ok_or_else(|| ForgeError::Import("Column has no data".to_string()))?;

        match first_type {
            Data::Float(_) | Data::Int(_) | Data::DateTime(_) => {
                // Number column: dates with a time of day, times and durations
                // are Excel's day numbers
                #[allow(clippy::cast_precision_loss)]
                // Excel integer values fit within f64 precision
                let numbers: Vec<f64> = data
                    .iter()
                    // An empty cell, or one that holds text, is 0; a date
                    // or a time is its day number
                    .map(|cell| number_of(cell).unwrap_or(0.0))
                    .collect();
                Ok(ColumnValue::Number(numbers))
            },
            Data::String(_) => {
                // Text column
                let texts: Vec<String> =
                    data.iter().map(std::string::ToString::to_string).collect();
                Ok(ColumnValue::Text(texts))
            },
            Data::Bool(_) => {
                // Boolean column
                let bools: Vec<bool> = data
                    .iter()
                    .map(|cell| matches!(cell, Data::Bool(true)))
                    .collect();
                Ok(ColumnValue::Boolean(bools))
            },
            _ => {
                // Default to text
                let texts: Vec<String> =
                    data.iter().map(std::string::ToString::to_string).collect();
                Ok(ColumnValue::Text(texts))
            },
        }
    }

    /// Sanitize sheet name to valid YAML key
    fn sanitize_table_name(sheet_name: &str) -> String {
        sheet_name
            .to_lowercase()
            .replace(' ', "_")
            .replace('&', "and")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }
}

#[cfg(test)]
// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_table_name() {
        assert_eq!(ExcelImporter::sanitize_table_name("Sheet1"), "sheet1");
        assert_eq!(
            ExcelImporter::sanitize_table_name("P&L Statement"),
            "pandl_statement"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("Revenue-2025"),
            "revenue_2025"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("Special@#$Chars"),
            "specialchars"
        );
    }

    #[test]
    fn test_convert_to_column_value_numbers() {
        let data = vec![
            Data::Float(100.0),
            Data::Float(200.0),
            Data::Int(300),
            Data::Empty,
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_eq!(nums[0], 100.0);
                assert_eq!(nums[1], 200.0);
                assert_eq!(nums[2], 300.0);
                assert_eq!(nums[3], 0.0);
            },
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_text() {
        let data = vec![
            Data::String("Apple".to_string()),
            Data::String("Banana".to_string()),
            Data::Empty,
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts.len(), 3);
                assert_eq!(texts[0], "Apple");
                assert_eq!(texts[1], "Banana");
                assert_eq!(texts[2], "");
            },
            _ => panic!("Expected Text column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_boolean() {
        let data = vec![Data::Bool(true), Data::Bool(false), Data::Empty];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools.len(), 3);
                assert!(bools[0]);
                assert!(!bools[1]);
                assert!(!bools[2]);
            },
            _ => panic!("Expected Boolean column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_empty() {
        let data = vec![Data::Empty, Data::Empty];
        let result = ExcelImporter::convert_to_column_value(&data);
        assert!(result.is_err());
    }

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
    fn test_convert_to_column_value_mixed_numeric() {
        let data = vec![
            Data::Float(1.5),
            Data::Int(2),
            Data::Float(3.0),
            Data::Int(4),
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_eq!(nums[0], 1.5);
                assert_eq!(nums[1], 2.0);
                assert_eq!(nums[2], 3.0);
                assert_eq!(nums[3], 4.0);
            },
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_sanitize_table_name_spaces() {
        assert_eq!(ExcelImporter::sanitize_table_name("My Sheet"), "my_sheet");
        assert_eq!(
            ExcelImporter::sanitize_table_name("Financial Data 2024"),
            "financial_data_2024"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("   trimmed   "),
            "___trimmed___"
        );
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
}
