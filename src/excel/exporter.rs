//! Excel exporter implementation

use super::text_results::{self, TextCell, TextResult};
use super::workbook_refs::dates::{excel_serial, Holds};
use super::workbook_refs::{
    include_sheet, split_scalar_path, Context, WorkbookRefs, UNGROUPED_SHEET,
};
use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, Metadata, ParsedModel, Table};
use rust_xlsxwriter::{Format, Formula, Note, Workbook, Worksheet};
use std::collections::HashMap;
use std::path::Path;

/// Excel exporter for v1.0.0 array models and v5.0.0 scalar models
pub struct ExcelExporter {
    model: ParsedModel,
    /// Sheet layout shared by every formula: where each column and scalar lives
    refs: WorkbookRefs,
    /// Included files by namespace, in name order: the calculated model and
    /// the layout its own formulas are written with
    includes: Vec<(String, ParsedModel, WorkbookRefs)>,
}

impl ExcelExporter {
    /// Create a new Excel exporter.
    ///
    /// Pass a calculated model if you have one: a formula that cannot be written
    /// as an Excel formula is exported as its calculated value.
    #[must_use]
    pub fn new(model: ParsedModel) -> Self {
        // Columns in model order: the letters must match what export_table writes.
        let tables = model.tables.iter().map(|(name, table)| {
            let columns: Vec<String> = table.column_names().into_iter().cloned().collect();
            (name.as_str(), columns, Self::row_count(table))
        });
        let mut refs = WorkbookRefs::new(tables, model.scalars.keys().map(String::as_str));
        for (name, table) in &model.tables {
            refs.add_column_types(name, Self::column_types(table));
            refs.add_date_columns(name, Self::date_columns(table));
            refs.add_mixed_columns(name, Self::mixed_columns(table));
            refs.add_column_lengths(name, Self::column_lengths(table));
        }

        // `resolved_includes` is a HashMap, so sort the namespaces: with two or
        // more includes the sheet order would otherwise change between runs.
        let mut namespaces: Vec<&String> = model.resolved_includes.keys().collect();
        namespaces.sort();
        let includes = namespaces
            .into_iter()
            .map(|namespace| {
                let resolved = &model.resolved_includes[namespace];
                // Calculate the included model, so a value it defines by formula
                // is cached. If it does not calculate, write what the file
                // holds; the main model's calculation reports that error.
                let included = crate::core::ArrayCalculator::new(resolved.model.clone())
                    .calculate_all()
                    .unwrap_or_else(|_| resolved.model.clone());
                let paths = || included.scalars.keys().map(String::as_str);
                let tables = || {
                    included.tables.iter().map(|(name, table)| {
                        let columns: Vec<String> =
                            table.column_names().into_iter().cloned().collect();
                        (name.clone(), columns, Self::row_count(table))
                    })
                };
                // `@costs.labour.wage` in the main model is a cell on the
                // include's sheet, and `@costs.prices.flour` a column of the
                // include's table, so a value changed there flows through.
                refs.add_include(namespace, paths());
                refs.add_include_tables(namespace, tables());
                let mut own = WorkbookRefs::for_include(namespace, paths(), tables());
                for (name, table) in &included.tables {
                    let key = format!("@{namespace}.{name}");
                    refs.add_column_types(&key, Self::column_types(table));
                    refs.add_date_columns(&key, Self::date_columns(table));
                    refs.add_mixed_columns(&key, Self::mixed_columns(table));
                    refs.add_column_lengths(&key, Self::column_lengths(table));
                    own.add_column_types(name, Self::column_types(table));
                    own.add_date_columns(name, Self::date_columns(table));
                    own.add_mixed_columns(name, Self::mixed_columns(table));
                    own.add_column_lengths(name, Self::column_lengths(table));
                }
                (namespace.clone(), included, own)
            })
            .collect();
        Self {
            model,
            refs,
            includes,
        }
    }

    /// Whether each column of numbers or text holds text (`COUNTUNIQUE` is
    /// written differently for each).
    fn column_types(table: &Table) -> impl Iterator<Item = (&str, bool)> {
        table
            .columns
            .iter()
            .filter_map(|(name, column)| match column.values {
                super::super::types::ColumnValue::Number(_) => Some((name.as_str(), false)),
                // Months (`2026-01`) and other dates Excel cannot hold are
                // text in the workbook; a column of dates is marked after
                super::super::types::ColumnValue::Text(_)
                | super::super::types::ColumnValue::Date(_) => Some((name.as_str(), true)),
                super::super::types::ColumnValue::Boolean(_) => None,
            })
    }

    /// Whether a column is written as dates: every value is a date Excel can
    /// hold (`2026-01-31`, from 1900-03-01 on). A column of months
    /// (`2026-01`), an earlier date or any other text keeps the column text.
    fn holds_dates(values: &ColumnValue) -> bool {
        match values {
            ColumnValue::Date(v) | ColumnValue::Text(v) => {
                !v.is_empty() && v.iter().all(|text| excel_serial(text).is_some())
            },
            _ => false,
        }
    }

    /// The columns of a table written as dates (ADR-054).
    fn date_columns(table: &Table) -> impl Iterator<Item = &str> {
        table
            .columns
            .iter()
            .filter(|(_, column)| Self::holds_dates(&column.values))
            .map(|(name, _)| name.as_str())
    }

    /// The columns of a table that stay text and have a digit in some row:
    /// `n/a` among dates, a date before 1900-03-01, a month (`2026-01`), a
    /// number written as text (`"5"`). Forge reads a number from such text
    /// its own way, and Excel its own: a formula that reads one is written
    /// as values (ADR-054). A column of words reads the same in both.
    fn mixed_columns(table: &Table) -> impl Iterator<Item = &str> {
        table
            .columns
            .iter()
            .filter(|(_, column)| match &column.values {
                ColumnValue::Date(v) | ColumnValue::Text(v) => {
                    !Self::holds_dates(&column.values)
                        && v.iter()
                            .any(|text| text.bytes().any(|b| b.is_ascii_digit()))
                },
                _ => false,
            })
            .map(|(name, _)| name.as_str())
    }

    /// What a column holds, for a formula that gives it.
    fn holds(table: &Table, column: &str, dates: &std::collections::HashSet<&str>) -> Holds {
        if dates.contains(column) {
            return Holds::Dates;
        }
        match table.columns.get(column).map(|c| &c.values) {
            Some(values) if values.is_empty() => Holds::Unknown,
            None => Holds::Unknown,
            Some(ColumnValue::Number(_)) => Holds::Numbers,
            Some(_) => Holds::Other,
        }
    }

    /// The format of a date cell: the day number shown as Forge writes dates.
    fn date_format() -> Format {
        Format::new().set_num_format("yyyy-mm-dd")
    }

    /// The number of values of each column.
    fn column_lengths(table: &Table) -> impl Iterator<Item = (&str, usize)> {
        table
            .columns
            .iter()
            .map(|(name, column)| (name.as_str(), column.len()))
    }

    /// Number of data rows: the longest column (columns may differ in length).
    fn row_count(table: &Table) -> usize {
        table
            .columns
            .values()
            .map(super::super::types::Column::len)
            .max()
            .unwrap_or(0)
    }

    /// Forge's calculated value as a cached formula result; a date of a
    /// column of dates as its day number, which is what the cell holds.
    fn cached_result(values: &ColumnValue, index: usize, is_date: bool) -> Option<String> {
        if is_date {
            if let ColumnValue::Text(v) | ColumnValue::Date(v) = values {
                return v
                    .get(index)
                    .and_then(|text| excel_serial(text))
                    .map(|serial| format!("{serial:.0}"));
            }
        }
        match values {
            ColumnValue::Number(v) => v.get(index).map(ToString::to_string),
            ColumnValue::Text(v) | ColumnValue::Date(v) => v.get(index).cloned(),
            ColumnValue::Boolean(v) => v
                .get(index)
                .map(|b| if *b { "TRUE" } else { "FALSE" }.to_string()),
        }
    }

    /// The formula with Forge's result cached in it. A text result that the
    /// writer would read as a number, a boolean or an error (`010`, `TRUE`) is
    /// noted in `text_results`, to be marked as text after the save.
    fn with_cached_result(
        formula: Formula,
        column: Option<&Column>,
        (row_idx, is_date): (usize, bool),
        (cell, text_results): (TextCell, &mut Vec<TextResult>),
    ) -> Formula {
        let Some(column) = column else {
            return formula;
        };
        let Some(result) = Self::cached_result(&column.values, row_idx, is_date) else {
            return formula;
        };
        let is_text = matches!(column.values, ColumnValue::Text(_) | ColumnValue::Date(_));
        if is_text && !is_date && text_results::looks_like_another_type(&result) {
            text_results.push(TextResult {
                sheet: cell.sheet,
                position: cell.position,
                text: result.clone(),
            });
        }
        formula.set_result(result)
    }

    /// Export the model to an Excel .xlsx file
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be saved to the specified path.
    pub fn export(&self, output_path: &Path) -> ForgeResult<()> {
        self.export_with_warnings(output_path).map(|_| ())
    }

    /// Export to a file and return one warning per formula that was written as
    /// its value because Excel cannot express it.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be saved to the specified path.
    pub fn export_with_warnings(&self, output_path: &Path) -> ForgeResult<Vec<String>> {
        let (bytes, warnings) = self.export_to_buffer_with_warnings()?;
        std::fs::write(output_path, bytes)
            .map_err(|e| ForgeError::IO(format!("Failed to save Excel file: {e}")))?;
        Ok(warnings)
    }

    /// Export the model to an in-memory Excel buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be serialized.
    pub fn export_to_buffer(&self) -> ForgeResult<Vec<u8>> {
        self.export_to_buffer_with_warnings()
            .map(|(bytes, _)| bytes)
    }

    /// Export to memory and return the warnings [`Self::export_with_warnings`] gives.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be serialized.
    pub fn export_to_buffer_with_warnings(&self) -> ForgeResult<(Vec<u8>, Vec<String>)> {
        let (mut workbook, warnings, text_results) = self.build_workbook()?;
        let bytes = workbook
            .save_to_buffer()
            .map_err(|e| ForgeError::IO(format!("Failed to create Excel buffer: {e}")))?;
        Ok((text_results::mark_as_text(bytes, &text_results)?, warnings))
    }

    fn build_workbook(&self) -> ForgeResult<(Workbook, Vec<String>, Vec<TextResult>)> {
        let mut workbook = Workbook::new();
        let mut warnings = Vec::new();
        let mut text_results = Vec::new();

        // Tables in model order, which is the same on every run
        for (table_name, table) in &self.model.tables {
            Self::export_table(
                &mut workbook,
                &self.refs,
                (table_name, table_name),
                table,
                (&mut warnings, &mut text_results),
            )?;
        }

        if !self.model.scalars.is_empty() {
            self.export_scalars(&mut workbook, &mut warnings)?;
        }

        // Included files: tables on `{namespace}.{table}` sheets and scalars
        // on a `{namespace}.Scalars` sheet, each with its formulas in the
        // included file's own names.
        for (namespace, included, own) in &self.includes {
            for (table_name, table) in &included.tables {
                let sheet = format!("{namespace}.{table_name}");
                Self::export_table(
                    &mut workbook,
                    own,
                    (&sheet, table_name),
                    table,
                    (&mut warnings, &mut text_results),
                )?;
            }
            // Written even without scalars: it is how an import knows the
            // `{namespace}.{table}` sheets belong to an included file
            if !included.scalars.is_empty() || !included.tables.is_empty() {
                Self::export_namespaced_scalars(
                    &mut workbook,
                    namespace,
                    included,
                    own,
                    &mut warnings,
                )?;
            }
        }

        // Excel has no counterpart for scenarios (named sets of input overrides),
        // so they are not written. Say so, instead of dropping them silently.
        if !self.model.scenarios.is_empty() {
            let mut names: Vec<&str> = self.model.scenarios.keys().map(String::as_str).collect();
            names.sort_unstable();
            warnings.push(format!(
                "scenarios: not exported ({}); Excel has no equivalent, so keep the YAML file as the source of truth",
                names.join(", ")
            ));
        }

        Ok((workbook, warnings, text_results))
    }

    /// Export a single table to a worksheet
    #[allow(clippy::cast_possible_truncation)] // column count and row count are within u16/u32 Excel limits
    /// Write a table to its sheet. `names` is (the sheet, the table's name in
    /// `refs`); they differ for a table of an included file.
    fn export_table(
        workbook: &mut Workbook,
        refs: &WorkbookRefs,
        names: (&str, &str),
        table: &Table,
        out: (&mut Vec<String>, &mut Vec<TextResult>),
    ) -> ForgeResult<()> {
        let (sheet_name, table_name) = names;
        let (warnings, text_results) = out;
        let sheet = workbook.worksheets().len();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(sheet_name)
            .map_err(|e| ForgeError::Export(format!("Failed to set worksheet name: {e}")))?;

        // Data and formula columns in model order (the same order as `self.refs`)
        let column_names: Vec<String> = table.column_names().into_iter().cloned().collect();

        // Write header row (row 0) with metadata as notes (v4.0)
        for (col_idx, col_name) in column_names.iter().enumerate() {
            worksheet
                .write_string(0, col_idx as u16, col_name)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;

            // Add metadata as cell note if column has metadata (v4.0)
            if let Some(column) = table.columns.get(col_name) {
                if let Some(note_text) = Self::format_metadata_note(&column.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet
                        .insert_note(0, col_idx as u16, &note)
                        .map_err(|e| ForgeError::Export(format!("Failed to add note: {e}")))?;
                }
            }
        }

        let row_count = Self::row_count(table);

        // The columns written as dates, found once (not once per cell)
        let date_columns: std::collections::HashSet<&str> = Self::date_columns(table).collect();

        // Formula columns that Excel cannot express are written as values.
        let mut untranslatable: HashMap<&str, String> = HashMap::new();
        for (name, formula) in &table.row_formulas {
            let ctx = Context::Row {
                table: table_name,
                row: 2,
            };
            // A column of dates must hold dates in the workbook too
            let holds = Self::holds(table, name, &date_columns);
            let translated = refs.to_excel_as(formula, ctx, holds).map(|_| ());
            if let Err(reason) = translated {
                warnings.push(format!(
                    "{sheet_name}.{name}: written as values ({reason}): {formula}"
                ));
                untranslatable.insert(name.as_str(), reason);
            }
        }

        // Write data rows (starting at row 1)
        for row_idx in 0..row_count {
            let excel_row = (row_idx + 1) as u32 + 1; // Excel row of this data row (header is row 1)

            for (col_idx, col_name) in column_names.iter().enumerate() {
                let col = col_idx as u16;
                let formula = table.row_formulas.get(col_name);
                if let (Some(formula), None) = (formula, untranslatable.get(col_name.as_str())) {
                    let ctx = Context::Row {
                        table: table_name,
                        row: excel_row,
                    };
                    let holds = Self::holds(table, col_name, &date_columns);
                    let translated = refs
                        .to_excel_as(formula, ctx, holds)
                        .map_err(|e| ForgeError::Export(format!("{sheet_name}.{col_name}: {e}")))?;
                    // Cache Forge's result, so viewers that do not recalculate show it
                    let excel_formula = Self::with_cached_result(
                        Formula::new(&translated.formula),
                        table.columns.get(col_name),
                        (row_idx, translated.is_date),
                        (
                            TextCell {
                                sheet,
                                position: (excel_row - 1, col),
                            },
                            text_results,
                        ),
                    );
                    if translated.is_date {
                        worksheet.write_formula_with_format(
                            excel_row - 1,
                            col,
                            excel_formula,
                            &Self::date_format(),
                        )
                    } else {
                        worksheet.write_formula(excel_row - 1, col, excel_formula)
                    }
                    .map_err(|e| ForgeError::Export(format!("Failed to write formula: {e}")))?;
                } else if let Some(column) = table.columns.get(col_name) {
                    // Data value, or the calculated value of an untranslatable formula
                    Self::write_cell_value(
                        worksheet,
                        (excel_row - 1, col),
                        &column.values,
                        row_idx,
                        date_columns.contains(col_name.as_str()),
                    )?;
                }
                if row_idx == 0 {
                    if let (Some(formula), Some(reason)) =
                        (formula, untranslatable.get(col_name.as_str()))
                    {
                        let note = Note::new(format!(
                            "Forge formula: {formula}\nWritten as values: {reason}"
                        ))
                        .set_author("Forge");
                        worksheet
                            .insert_note(excel_row - 1, col, &note)
                            .map_err(|e| ForgeError::Export(format!("Failed to add note: {e}")))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Write a single cell value based on column type; `is_date` says the
    /// column is written as dates.
    fn write_cell_value(
        worksheet: &mut Worksheet,
        cell: (u32, u16),
        values: &ColumnValue,
        index: usize,
        is_date: bool,
    ) -> ForgeResult<()> {
        let (row, col) = cell;
        match values {
            ColumnValue::Number(nums) => {
                if let Some(&value) = nums.get(index) {
                    worksheet
                        .write_number(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write number: {e}")))?;
                }
            },
            // A column of dates is written as dates: the day number, shown
            // as Forge writes it (ADR-054). Any other text is text.
            ColumnValue::Text(texts) | ColumnValue::Date(texts) => {
                if let Some(value) = texts.get(index) {
                    if is_date {
                        if let Some(serial) = excel_serial(value) {
                            worksheet
                                .write_number_with_format(row, col, serial, &Self::date_format())
                                .map_err(|e| {
                                    ForgeError::Export(format!("Failed to write date: {e}"))
                                })?;
                        }
                    } else {
                        worksheet.write_string(row, col, value).map_err(|e| {
                            ForgeError::Export(format!("Failed to write text: {e}"))
                        })?;
                    }
                }
            },
            ColumnValue::Boolean(bools) => {
                if let Some(&value) = bools.get(index) {
                    worksheet
                        .write_boolean(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write boolean: {e}")))?;
                }
            },
        }
        Ok(())
    }

    /// Export scalars to grouped worksheets (one per prefix group)
    /// e.g., "utilities.extinction", "utilities.flourishing" -> worksheet "utilities";
    /// scalars without a group go to the "Scalars" worksheet.
    fn export_scalars(
        &self,
        workbook: &mut Workbook,
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        // Groups in the order of their first scalar, scalars in model order
        // (the same rows as `self.refs`).
        let mut groups: indexmap::IndexMap<Option<&str>, Vec<&String>> = indexmap::IndexMap::new();
        for path in self.model.scalars.keys() {
            groups
                .entry(split_scalar_path(path).0)
                .or_default()
                .push(path);
        }
        for (group, paths) in &groups {
            self.export_scalar_group(workbook, *group, paths, warnings)?;
        }
        Ok(())
    }

    /// Export one group of scalars to a worksheet: `Name` in A, `Value` in B
    fn export_scalar_group(
        &self,
        workbook: &mut Workbook,
        group: Option<&str>,
        paths: &[&String],
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        let sheet = group.unwrap_or(UNGROUPED_SHEET);
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(sheet).map_err(|e| {
            ForgeError::Export(format!("Failed to set worksheet name '{sheet}': {e}"))
        })?;
        for (col, header) in [(0, "Name"), (1, "Value")] {
            worksheet
                .write_string(0, col, header)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;
        }

        for path in paths {
            let var = &self.model.scalars[*path];
            let row = self.refs.scalars[path.as_str()].1 - 1; // 0-based worksheet row
            worksheet
                .write_string(row, 0, split_scalar_path(path).1)
                .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {e}")))?;

            let mut notes: Vec<String> = Vec::new();
            let translated = var
                .formula
                .as_ref()
                .map(|f| (f, self.refs.to_excel(f, Context::Scalar { group })));
            match translated {
                Some((_, Ok(t))) => {
                    let mut formula = Formula::new(&t.formula);
                    if let Some(value) = var.value {
                        formula = formula.set_result(value.to_string());
                    }
                    let written = if t.has_range {
                        worksheet.write_dynamic_formula(row, 1, formula)
                    } else {
                        worksheet.write_formula(row, 1, formula)
                    };
                    written.map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar formula: {e}"))
                    })?;
                },
                Some((formula, Err(reason))) => {
                    warnings.push(format!("{path}: written as a value ({reason}): {formula}"));
                    notes.push(format!(
                        "Forge formula: {formula}\nWritten as a value: {reason}"
                    ));
                    if let Some(value) = var.value {
                        worksheet.write_number(row, 1, value).map_err(|e| {
                            ForgeError::Export(format!("Failed to write scalar value: {e}"))
                        })?;
                    }
                },
                None => {
                    if let Some(value) = var.value {
                        worksheet.write_number(row, 1, value).map_err(|e| {
                            ForgeError::Export(format!("Failed to write scalar value: {e}"))
                        })?;
                    }
                },
            }

            if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                notes.push(note_text);
            }
            if !notes.is_empty() {
                let note = Note::new(notes.join("\n\n")).set_author("Forge");
                worksheet
                    .insert_note(row, 1, &note)
                    .map_err(|e| ForgeError::Export(format!("Failed to add scalar note: {e}")))?;
            }
        }

        Ok(())
    }

    /// Export an included file's scalars to its `{namespace}.Scalars` sheet:
    /// `{namespace}.{path}` in A, the value or formula in B, in the included
    /// model's order (the rows `WorkbookRefs::add_include` points at).
    fn export_namespaced_scalars(
        workbook: &mut Workbook,
        namespace: &str,
        included_model: &ParsedModel,
        own: &WorkbookRefs,
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        let sheet_name = include_sheet(namespace);
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name).map_err(|e| {
            ForgeError::Export(format!("Failed to set {sheet_name} worksheet name: {e}"))
        })?;
        for (col, header) in [(0, "Name"), (1, "Value")] {
            worksheet
                .write_string(0, col, header)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;
        }

        for (path, var) in &included_model.scalars {
            let row = own.scalars[path.as_str()].1 - 1; // 0-based worksheet row
            worksheet
                .write_string(row, 0, format!("{namespace}.{path}"))
                .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {e}")))?;

            let mut notes: Vec<String> = Vec::new();
            // The included file's own formulas resolve inside that file
            let group = split_scalar_path(path).0;
            let translated = var
                .formula
                .as_ref()
                .map(|f| (f, own.to_excel(f, Context::Scalar { group })));
            match translated {
                Some((_, Ok(t))) => {
                    let mut formula = Formula::new(&t.formula);
                    if let Some(value) = var.value {
                        formula = formula.set_result(value.to_string());
                    }
                    let written = if t.has_range {
                        worksheet.write_dynamic_formula(row, 1, formula)
                    } else {
                        worksheet.write_formula(row, 1, formula)
                    };
                    written.map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar formula: {e}"))
                    })?;
                },
                other => {
                    if let Some((formula, Err(reason))) = other {
                        warnings.push(format!(
                            "@{namespace}.{path}: written as a value ({reason}): {formula}"
                        ));
                        notes.push(format!(
                            "Forge formula: {formula}\nWritten as a value: {reason}"
                        ));
                    }
                    if let Some(value) = var.value {
                        worksheet.write_number(row, 1, value).map_err(|e| {
                            ForgeError::Export(format!("Failed to write scalar value: {e}"))
                        })?;
                    }
                },
            }

            if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                notes.push(note_text);
            }
            if !notes.is_empty() {
                let note = Note::new(notes.join("\n\n")).set_author("Forge");
                worksheet
                    .insert_note(row, 1, &note)
                    .map_err(|e| ForgeError::Export(format!("Failed to add scalar note: {e}")))?;
            }
        }

        Ok(())
    }

    /// Format metadata fields as a note text for Excel cell comments (v4.0)
    /// Returns None if metadata is empty
    fn format_metadata_note(metadata: &Metadata) -> Option<String> {
        if metadata.is_empty() {
            return None;
        }

        let mut parts = Vec::new();

        if let Some(unit) = &metadata.unit {
            parts.push(format!("Unit: {unit}"));
        }
        if let Some(notes) = &metadata.notes {
            parts.push(format!("Notes: {notes}"));
        }
        if let Some(source) = &metadata.source {
            parts.push(format!("Source: {source}"));
        }
        if let Some(status) = &metadata.validation_status {
            parts.push(format!("Status: {status}"));
        }
        if let Some(updated) = &metadata.last_updated {
            parts.push(format!("Updated: {updated}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }
}

#[cfg(test)]
#[path = "exporter_tests.rs"]
mod tests;
