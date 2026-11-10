//! Excel exporter implementation

use super::workbook_refs::{
    include_sheet, split_scalar_path, Context, WorkbookRefs, UNGROUPED_SHEET,
};
use crate::error::{ForgeError, ForgeResult};
use crate::types::{ColumnValue, Metadata, ParsedModel, Table};
use rust_xlsxwriter::{Formula, Note, Workbook, Worksheet};
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
                    refs.add_column_lengths(&key, Self::column_lengths(table));
                    own.add_column_types(name, Self::column_types(table));
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
                super::super::types::ColumnValue::Text(_) => Some((name.as_str(), true)),
                _ => None,
            })
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

    /// Forge's calculated value as a cached formula result.
    fn cached_result(values: &ColumnValue, index: usize) -> Option<String> {
        match values {
            ColumnValue::Number(v) => v.get(index).map(ToString::to_string),
            ColumnValue::Text(v) | ColumnValue::Date(v) => v.get(index).cloned(),
            ColumnValue::Boolean(v) => v
                .get(index)
                .map(|b| if *b { "TRUE" } else { "FALSE" }.to_string()),
        }
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
        let (mut workbook, warnings) = self.build_workbook()?;
        workbook
            .save(output_path)
            .map_err(|e| ForgeError::IO(format!("Failed to save Excel file: {e}")))?;
        Ok(warnings)
    }

    /// Export the model to an in-memory Excel buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be serialized.
    pub fn export_to_buffer(&self) -> ForgeResult<Vec<u8>> {
        let (mut workbook, _warnings) = self.build_workbook()?;
        workbook
            .save_to_buffer()
            .map_err(|e| ForgeError::IO(format!("Failed to create Excel buffer: {e}")))
    }

    fn build_workbook(&self) -> ForgeResult<(Workbook, Vec<String>)> {
        let mut workbook = Workbook::new();
        let mut warnings = Vec::new();

        // Tables in model order, which is the same on every run
        for (table_name, table) in &self.model.tables {
            Self::export_table(
                &mut workbook,
                &self.refs,
                (table_name, table_name),
                table,
                &mut warnings,
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
                    &mut warnings,
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

        Ok((workbook, warnings))
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
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        let (sheet_name, table_name) = names;
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

        // Formula columns that Excel cannot express are written as values.
        let mut untranslatable: HashMap<&str, String> = HashMap::new();
        for (name, formula) in &table.row_formulas {
            let ctx = Context::Row {
                table: table_name,
                row: 2,
            };
            // Forge's TODAY is text in a table column, Excel's a date serial
            let today_text = crate::excel::workbook_refs::calls_function(formula, "TODAY")
                && table.columns.get(name).is_some_and(|c| {
                    !matches!(c.values, super::super::types::ColumnValue::Number(_))
                });
            let translated = if today_text {
                Err("TODAY gives text in a table column, and a date serial in Excel".to_string())
            } else {
                refs.to_excel(formula, ctx).map(|_| ())
            };
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
                    let translated = refs
                        .to_excel(formula, ctx)
                        .map_err(|e| ForgeError::Export(format!("{sheet_name}.{col_name}: {e}")))?;
                    let mut excel_formula = Formula::new(&translated.formula);
                    // Cache Forge's result, so viewers that do not recalculate show it
                    if let Some(result) = table
                        .columns
                        .get(col_name)
                        .and_then(|c| Self::cached_result(&c.values, row_idx))
                    {
                        excel_formula = excel_formula.set_result(result);
                    }
                    worksheet
                        .write_formula(excel_row - 1, col, excel_formula)
                        .map_err(|e| ForgeError::Export(format!("Failed to write formula: {e}")))?;
                } else if let Some(column) = table.columns.get(col_name) {
                    // Data value, or the calculated value of an untranslatable formula
                    Self::write_cell_value(worksheet, excel_row - 1, col, &column.values, row_idx)?;
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

    /// Write a single cell value based on column type
    fn write_cell_value(
        worksheet: &mut Worksheet,
        row: u32,
        col: u16,
        values: &ColumnValue,
        index: usize,
    ) -> ForgeResult<()> {
        match values {
            ColumnValue::Number(nums) => {
                if let Some(&value) = nums.get(index) {
                    worksheet
                        .write_number(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write number: {e}")))?;
                }
            },
            ColumnValue::Text(texts) => {
                if let Some(value) = texts.get(index) {
                    worksheet
                        .write_string(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write text: {e}")))?;
                }
            },
            ColumnValue::Date(dates) => {
                if let Some(value) = dates.get(index) {
                    worksheet
                        .write_string(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write date: {e}")))?;
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
mod tests {
    use super::*;
    use crate::types::{Column, Variable};

    // =========================================================================
    // Metadata Note Formatting Tests
    // =========================================================================

    #[test]
    fn test_format_metadata_note_empty() {
        let metadata = Metadata::default();
        assert!(ExcelExporter::format_metadata_note(&metadata).is_none());
    }

    #[test]
    fn test_format_metadata_note_with_unit() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            ..Default::default()
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Unit: CAD"));
    }

    #[test]
    fn test_format_metadata_note_with_notes() {
        let metadata = Metadata {
            notes: Some("Revenue projection".to_string()),
            ..Default::default()
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Notes: Revenue projection"));
    }

    #[test]
    fn test_format_metadata_note_with_source() {
        let metadata = Metadata {
            source: Some("data.yaml".to_string()),
            ..Default::default()
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Source: data.yaml"));
    }

    #[test]
    fn test_format_metadata_note_with_validation_status() {
        let metadata = Metadata {
            validation_status: Some("VALIDATED".to_string()),
            ..Default::default()
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Status: VALIDATED"));
    }

    #[test]
    fn test_format_metadata_note_with_last_updated() {
        let metadata = Metadata {
            last_updated: Some("2025-01-01".to_string()),
            ..Default::default()
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Updated: 2025-01-01"));
    }

    #[test]
    fn test_format_metadata_note_multiple_fields() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            notes: Some("Important".to_string()),
            source: Some("finance.yaml".to_string()),
            validation_status: Some("PROJECTED".to_string()),
            last_updated: Some("2025-11-26".to_string()),
            better: None,
        };
        let note = ExcelExporter::format_metadata_note(&metadata).unwrap();
        assert!(note.contains("Unit: CAD"));
        assert!(note.contains("Notes: Important"));
        assert!(note.contains("Source: finance.yaml"));
        assert!(note.contains("Status: PROJECTED"));
        assert!(note.contains("Updated: 2025-11-26"));
        // Check newlines
        assert!(note.contains('\n'));
    }

    // =========================================================================
    // ExcelExporter Construction Tests
    // =========================================================================

    #[test]
    fn test_exporter_new_empty_model() {
        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);
        assert!(exporter.refs.columns.is_empty());
        assert!(exporter.refs.rows.is_empty());
    }

    #[test]
    fn test_exporter_new_with_table() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        assert!(exporter.refs.columns.contains_key("sales"));
        assert_eq!(exporter.refs.rows.get("sales"), Some(&3));
    }

    #[test]
    fn test_exporter_new_with_row_formula() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("calc".to_string());
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0]),
        ));
        table.add_row_formula("total".to_string(), "=SUM(amount)".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        // Column map should include both data column and formula column
        let col_map = exporter.refs.columns.get("calc").unwrap();
        assert!(col_map.contains_key("amount"));
        assert!(col_map.contains_key("total"));
    }

    #[test]
    fn test_exporter_new_multiple_tables() {
        let mut model = ParsedModel::new();

        let mut table1 = Table::new("sales".to_string());
        table1.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        model.add_table(table1);

        let mut table2 = Table::new("costs".to_string());
        table2.add_column(Column::new(
            "expense".to_string(),
            ColumnValue::Number(vec![50.0, 75.0, 100.0]),
        ));
        model.add_table(table2);

        let exporter = ExcelExporter::new(model);

        assert!(exporter.refs.columns.contains_key("sales"));
        assert!(exporter.refs.columns.contains_key("costs"));
        assert_eq!(exporter.refs.rows.get("sales"), Some(&2));
        assert_eq!(exporter.refs.rows.get("costs"), Some(&3));
    }

    #[test]
    fn test_exporter_column_maps_keep_model_order() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "zebra".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        table.add_column(Column::new(
            "alpha".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        table.add_column(Column::new(
            "beta".to_string(),
            ColumnValue::Number(vec![3.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);
        let col_map = exporter.refs.columns.get("data").unwrap();

        // zebra -> A, alpha -> B, beta -> C (the order the columns were added)
        assert_eq!(col_map.get("zebra"), Some(&"A".to_string()));
        assert_eq!(col_map.get("alpha"), Some(&"B".to_string()));
        assert_eq!(col_map.get("beta"), Some(&"C".to_string()));
    }

    #[test]
    fn test_exporter_empty_table() {
        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);
        assert_eq!(exporter.refs.rows.get("empty"), Some(&0));
    }

    // =========================================================================
    // Excel Export Tests (File I/O)
    // =========================================================================

    #[test]
    fn test_export_empty_model() {
        use tempfile::TempDir;

        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("empty.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_single_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("sales.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // File should have non-zero size
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_export_with_text_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("names.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_date_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("timeline".to_string());
        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2024-01-01".to_string(),
                "2024-02-01".to_string(),
                "2024-03-01".to_string(),
            ]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("dates.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_boolean_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("flags".to_string());
        table.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false, true]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("flags.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_row_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("calc".to_string());
        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![2.0, 3.0, 4.0]),
        ));
        table.add_row_formula("total".to_string(), "=price * quantity".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("calculated.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_scalars() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.add_scalar(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.15), None),
        );
        model.add_scalar(
            "profit".to_string(),
            Variable::new(
                "profit".to_string(),
                Some(50000.0),
                Some("=revenue - costs".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_multiple_tables() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        let mut revenue_table = Table::new("revenue".to_string());
        revenue_table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 3000.0]),
        ));
        model.add_table(revenue_table);

        let mut costs_table = Table::new("costs".to_string());
        costs_table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![500.0, 750.0, 1000.0]),
        ));
        model.add_table(costs_table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("multi.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_metadata() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            notes: Some("Revenue data".to_string()),
            source: Some("finance.yaml".to_string()),
            validation_status: Some("VALIDATED".to_string()),
            last_updated: Some("2024-01-01".to_string()),
            better: None,
        };

        table.add_column(Column::with_metadata(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
            metadata,
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("metadata.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_mixed_column_types() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("mixed".to_string());

        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        table.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec!["A".to_string(), "B".to_string()]),
        ));
        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-02-01".to_string()]),
        ));
        table.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false]),
        ));

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("mixed.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_to_nonexistent_directory_fails() {
        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);

        let output_path = std::path::Path::new("/nonexistent/dir/output.xlsx");

        let result = exporter.export(output_path);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional coverage tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_export_with_row_formulas() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));

        // Add a formula column
        table
            .row_formulas
            .insert("total".to_string(), "=price * quantity".to_string());

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("formulas.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_scalars_with_formulas() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add input scalar (no formula)
        model.scalars.insert(
            "inputs.rate".to_string(),
            crate::types::Variable::new("inputs.rate".to_string(), Some(0.05), None),
        );

        // Add output scalar (with formula)
        model.scalars.insert(
            "outputs.result".to_string(),
            crate::types::Variable::new(
                "outputs.result".to_string(),
                Some(500.0),
                Some("=inputs.rate * 10000".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_aggregations() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add a table
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        // Add aggregation
        model
            .aggregations
            .insert("total_sales".to_string(), "=SUM(sales.amount)".to_string());

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("aggregations.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_empty_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        // Don't add any columns
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("empty.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_large_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("large".to_string());

        // Create 1000 row table
        let values: Vec<f64> = (0..1000).map(f64::from).collect();
        table.add_column(Column::new("id".to_string(), ColumnValue::Number(values)));

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("large.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_cross_table_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // First table
        let mut revenue = Table::new("revenue".to_string());
        revenue.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0]),
        ));
        model.add_table(revenue);

        // Second table referencing first
        let mut profit = Table::new("profit".to_string());
        profit.add_column(Column::new(
            "margin".to_string(),
            ColumnValue::Number(vec![0.2, 0.3]),
        ));
        profit
            .row_formulas
            .insert("amount".to_string(), "=revenue.amount * margin".to_string());
        model.add_table(profit);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("cross_table.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_all_metadata_fields() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("complete".to_string());

        // Full metadata
        let metadata = Metadata {
            unit: Some("USD".to_string()),
            notes: Some("Complete metadata test".to_string()),
            source: Some("test.yaml".to_string()),
            validation_status: Some("PENDING".to_string()),
            last_updated: Some("2025-12-04".to_string()),
            better: None,
        };

        table.add_column(Column::with_metadata(
            "value".to_string(),
            ColumnValue::Number(vec![42.0]),
            metadata,
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("full_metadata.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Grouped Scalar Export Tests (v5.0.0 scalar models)
    // =========================================================================

    #[test]
    fn test_export_grouped_scalars() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add scalars in different groups (simulating v5.0.0 scalar model)
        // Group: utilities
        model.scalars.insert(
            "utilities.extinction".to_string(),
            Variable::new("utilities.extinction".to_string(), Some(0.0), None),
        );
        model.scalars.insert(
            "utilities.flourishing".to_string(),
            Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
        );

        // Group: scenario_probs
        model.scalars.insert(
            "scenario_probs.p_unaligned".to_string(),
            Variable::new("scenario_probs.p_unaligned".to_string(), Some(0.35), None),
        );
        model.scalars.insert(
            "scenario_probs.p_aligned".to_string(),
            Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
        );

        // Group: analysis (with formula referencing other groups)
        model.scalars.insert(
            "analysis.expected_value".to_string(),
            Variable::new(
                "analysis.expected_value".to_string(),
                Some(25.0),
                Some("=scenario_probs.p_aligned * utilities.flourishing".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        // Verify scalar locations were built correctly
        assert!(exporter.refs.scalars.contains_key("utilities.extinction"));
        assert!(exporter
            .refs
            .scalars
            .contains_key("scenario_probs.p_unaligned"));
        assert!(exporter
            .refs
            .scalars
            .contains_key("analysis.expected_value"));

        // Verify worksheet assignments
        let util_loc = exporter.refs.scalars.get("utilities.extinction").unwrap();
        assert_eq!(util_loc.0, "utilities");

        let prob_loc = exporter
            .refs
            .scalars
            .get("scenario_probs.p_unaligned")
            .unwrap();
        assert_eq!(prob_loc.0, "scenario_probs");

        let analysis_loc = exporter
            .refs
            .scalars
            .get("analysis.expected_value")
            .unwrap();
        assert_eq!(analysis_loc.0, "analysis");

        // Export and verify file creation
        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("grouped_scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_scalars_without_dots() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Scalars without dots should go to "Scalars" worksheet
        model.scalars.insert(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.15), None),
        );
        model.scalars.insert(
            "discount_rate".to_string(),
            Variable::new("discount_rate".to_string(), Some(0.10), None),
        );

        let exporter = ExcelExporter::new(model);

        // Verify they go to "Scalars" worksheet
        let tax_loc = exporter.refs.scalars.get("tax_rate").unwrap();
        assert_eq!(tax_loc.0, "Scalars");

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("ungrouped_scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_warns_when_scenarios_are_not_exported() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.scalars.insert(
            "rate".to_string(),
            Variable::new("rate".to_string(), Some(0.1), None),
        );
        for name in ["bull", "base"] {
            model.scenarios.insert(
                name.to_string(),
                crate::types::Scenario {
                    overrides: std::collections::HashMap::from([("rate".to_string(), 0.2)]),
                },
            );
        }

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("with_scenarios.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&path)
            .unwrap();

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with("scenarios: not exported (base, bull)"),
            "{}",
            warnings[0]
        );
    }

    #[test]
    fn test_export_without_scenarios_has_no_scenario_warning() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.scalars.insert(
            "rate".to_string(),
            Variable::new("rate".to_string(), Some(0.1), None),
        );

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("no_scenarios.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&path)
            .unwrap();

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn test_translate_grouped_scalar_formula() {
        use super::super::workbook_refs::Context;
        let mut model = ParsedModel::new();
        model.scalars.insert(
            "utilities.flourishing".to_string(),
            Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
        );
        model.scalars.insert(
            "scenario_probs.p_aligned".to_string(),
            Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
        );
        let exporter = ExcelExporter::new(model);
        let t = exporter
            .refs
            .to_excel(
                "=scenario_probs.p_aligned * utilities.flourishing",
                Context::Scalar {
                    group: Some("analysis"),
                },
            )
            .unwrap();
        assert_eq!(t.formula, "='scenario_probs'!$B$2 * 'utilities'!$B$2");
    }

    #[test]
    fn test_export_scalar_with_mc_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Scalar with Monte Carlo formula (not supported in Excel)
        model.scalars.insert(
            "inputs.probability".to_string(),
            Variable::new(
                "inputs.probability".to_string(),
                Some(0.35), // Calculated value
                Some("=MC.Triangular(0.25, 0.35, 0.45)".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("mc_scalar.xlsx");

        // Should succeed - MC formulas are written as values with formula in comment
        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }
}
