//! Excel exporter implementation

use crate::error::{ForgeError, ForgeResult};
use crate::types::{ColumnValue, Metadata, ParsedModel, Table};
use rust_xlsxwriter::{Formula, Note, Workbook, Worksheet};
use std::collections::HashMap;
use std::path::Path;

/// Excel exporter for v1.0.0 array models
pub struct ExcelExporter {
    model: ParsedModel,
    /// Global mapping: table_name -> (column_name -> column_letter)
    table_column_maps: HashMap<String, HashMap<String, String>>,
    /// Global mapping: table_name -> row_count
    table_row_counts: HashMap<String, usize>,
}

impl ExcelExporter {
    /// Create a new Excel exporter
    pub fn new(model: ParsedModel) -> Self {
        // Build global column mappings for all tables
        let mut table_column_maps = HashMap::new();
        let mut table_row_counts = HashMap::new();

        for (table_name, table) in &model.tables {
            let mut column_names: Vec<String> = Vec::new();

            // Add data columns
            for name in table.columns.keys() {
                column_names.push(name.clone());
            }

            // Add formula columns
            for name in table.row_formulas.keys() {
                if !column_names.contains(name) {
                    column_names.push(name.clone());
                }
            }

            column_names.sort(); // Alphabetical order

            // Build column name → letter mapping
            let column_map: HashMap<String, String> = column_names
                .iter()
                .enumerate()
                .map(|(idx, name)| {
                    let col_letter = super::FormulaTranslator::column_index_to_letter(idx);
                    (name.clone(), col_letter)
                })
                .collect();

            // Get row count
            let row_count = table
                .columns
                .values()
                .next()
                .map(|col| col.len())
                .unwrap_or(0);

            table_column_maps.insert(table_name.clone(), column_map);
            table_row_counts.insert(table_name.clone(), row_count);
        }

        Self {
            model,
            table_column_maps,
            table_row_counts,
        }
    }

    /// Export the model to an Excel .xlsx file
    pub fn export(&self, output_path: &Path) -> ForgeResult<()> {
        let mut workbook = Workbook::new();

        // Export each table as a separate worksheet
        for (table_name, table) in &self.model.tables {
            self.export_table(&mut workbook, table_name, table)?;
        }

        // Export scalars to dedicated worksheet (if any)
        if !self.model.scalars.is_empty() {
            self.export_scalars(&mut workbook)?;
        }

        // Export included files' tables with namespace prefix (v4.4.2)
        for (namespace, resolved) in &self.model.resolved_includes {
            // Export tables from included file
            for (table_name, table) in &resolved.model.tables {
                let prefixed_name = format!("{}.{}", namespace, table_name);
                self.export_table(&mut workbook, &prefixed_name, table)?;
            }

            // Export scalars from included file with namespace prefix
            if !resolved.model.scalars.is_empty() {
                self.export_namespaced_scalars(&mut workbook, namespace, &resolved.model)?;
            }
        }

        // Save workbook to file
        workbook
            .save(output_path)
            .map_err(|e| ForgeError::IO(format!("Failed to save Excel file: {}", e)))?;

        Ok(())
    }

    /// Export a single table to a worksheet
    fn export_table(
        &self,
        workbook: &mut Workbook,
        table_name: &str,
        table: &Table,
    ) -> ForgeResult<()> {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(table_name)
            .map_err(|e| ForgeError::Export(format!("Failed to set worksheet name: {}", e)))?;

        // Get column names in a deterministic order (data + formula columns)
        let mut column_names: Vec<String> = Vec::new();

        // Add data columns
        for name in table.columns.keys() {
            column_names.push(name.clone());
        }

        // Add formula columns
        for name in table.row_formulas.keys() {
            if !column_names.contains(name) {
                column_names.push(name.clone());
            }
        }

        column_names.sort(); // Alphabetical order for now

        // Get the column map for this table (already built in new())
        let column_map = self
            .table_column_maps
            .get(table_name)
            .cloned()
            .unwrap_or_default();

        // Create formula translator with global table knowledge
        let translator = super::FormulaTranslator::new_with_tables(
            column_map,
            self.table_column_maps.clone(),
            self.table_row_counts.clone(),
        );

        // Write header row (row 0) with metadata as notes (v4.0)
        for (col_idx, col_name) in column_names.iter().enumerate() {
            worksheet
                .write_string(0, col_idx as u16, col_name)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;

            // Add metadata as cell note if column has metadata (v4.0)
            if let Some(column) = table.columns.get(col_name) {
                if let Some(note_text) = Self::format_metadata_note(&column.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet
                        .insert_note(0, col_idx as u16, &note)
                        .map_err(|e| ForgeError::Export(format!("Failed to add note: {}", e)))?;
                }
            }
        }

        // Get row count from first data column
        let row_count = table
            .columns
            .values()
            .next()
            .map(|col| col.len())
            .unwrap_or(0);

        // Write data rows (starting at row 1)
        for row_idx in 0..row_count {
            let excel_row = (row_idx + 1) as u32 + 1; // +1 for header row, +1 for Excel 1-indexing = row 2 for first data row

            for (col_idx, col_name) in column_names.iter().enumerate() {
                // Check if this is a calculated column (has formula)
                if let Some(formula) = table.row_formulas.get(col_name) {
                    // Translate and write formula
                    let excel_formula = translator.translate_row_formula(formula, excel_row)?;
                    worksheet
                        .write_formula(excel_row - 1, col_idx as u16, Formula::new(&excel_formula))
                        .map_err(|e| {
                            ForgeError::Export(format!("Failed to write formula: {}", e))
                        })?;
                } else if let Some(column) = table.columns.get(col_name) {
                    // Write data value
                    self.write_cell_value(
                        worksheet,
                        excel_row - 1, // Excel row is 1-indexed, worksheet API is 0-indexed
                        col_idx as u16,
                        &column.values,
                        row_idx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Write a single cell value based on column type
    fn write_cell_value(
        &self,
        worksheet: &mut Worksheet,
        row: u32,
        col: u16,
        values: &ColumnValue,
        index: usize,
    ) -> ForgeResult<()> {
        match values {
            ColumnValue::Number(nums) => {
                if let Some(&value) = nums.get(index) {
                    worksheet.write_number(row, col, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write number: {}", e))
                    })?;
                }
            }
            ColumnValue::Text(texts) => {
                if let Some(value) = texts.get(index) {
                    worksheet
                        .write_string(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write text: {}", e)))?;
                }
            }
            ColumnValue::Date(dates) => {
                if let Some(value) = dates.get(index) {
                    worksheet
                        .write_string(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write date: {}", e)))?;
                }
            }
            ColumnValue::Boolean(bools) => {
                if let Some(&value) = bools.get(index) {
                    worksheet.write_boolean(row, col, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write boolean: {}", e))
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Export scalars to a dedicated "Scalars" worksheet
    fn export_scalars(&self, workbook: &mut Workbook) -> ForgeResult<()> {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Scalars").map_err(|e| {
            ForgeError::Export(format!("Failed to set Scalars worksheet name: {}", e))
        })?;

        // Create formula translator with global table knowledge
        let translator = super::FormulaTranslator::new_with_tables(
            HashMap::new(), // No local columns for scalars
            self.table_column_maps.clone(),
            self.table_row_counts.clone(),
        );

        // Write header row
        worksheet
            .write_string(0, 0, "Name")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;
        worksheet
            .write_string(0, 1, "Value")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;

        // Write scalars (sorted by name for deterministic output)
        let mut scalar_names: Vec<&String> = self.model.scalars.keys().collect();
        scalar_names.sort();

        // Build a map of scalar names to their row numbers for inter-scalar references
        let scalar_row_map: HashMap<String, u32> = scalar_names
            .iter()
            .enumerate()
            .map(|(idx, name)| ((*name).clone(), (idx + 1) as u32 + 1)) // +1 for header, +1 for Excel 1-indexing
            .collect();

        for (idx, name) in scalar_names.iter().enumerate() {
            let row = (idx + 1) as u32; // +1 for header row

            if let Some(var) = self.model.scalars.get(*name) {
                // Write name
                worksheet.write_string(row, 0, *name).map_err(|e| {
                    ForgeError::Export(format!("Failed to write scalar name: {}", e))
                })?;

                // Write formula or value
                if let Some(formula) = &var.formula {
                    // Translate and write as actual Excel formula
                    match translator.translate_scalar_formula(formula, &scalar_row_map) {
                        Ok(excel_formula) => {
                            worksheet
                                .write_formula(row, 1, Formula::new(&excel_formula))
                                .map_err(|e| {
                                    ForgeError::Export(format!(
                                        "Failed to write scalar formula: {}",
                                        e
                                    ))
                                })?;
                        }
                        Err(_) => {
                            // Fallback: write calculated value if formula translation fails
                            if let Some(value) = var.value {
                                worksheet.write_number(row, 1, value).map_err(|e| {
                                    ForgeError::Export(format!(
                                        "Failed to write scalar value: {}",
                                        e
                                    ))
                                })?;
                            }
                        }
                    }
                } else if let Some(value) = var.value {
                    // No formula, just write the value
                    worksheet.write_number(row, 1, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar value: {}", e))
                    })?;
                }

                // Add metadata as cell note for scalars (v4.0)
                if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet.insert_note(row, 1, &note).map_err(|e| {
                        ForgeError::Export(format!("Failed to add scalar note: {}", e))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Export scalars from an included file with namespace prefix (v4.4.2)
    fn export_namespaced_scalars(
        &self,
        workbook: &mut Workbook,
        namespace: &str,
        included_model: &ParsedModel,
    ) -> ForgeResult<()> {
        let sheet_name = format!("{}.Scalars", namespace);
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name).map_err(|e| {
            ForgeError::Export(format!(
                "Failed to set {} worksheet name: {}",
                sheet_name, e
            ))
        })?;

        // Write header row
        worksheet
            .write_string(0, 0, "Name")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;
        worksheet
            .write_string(0, 1, "Value")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;

        // Write scalars (sorted by name)
        let mut scalar_names: Vec<&String> = included_model.scalars.keys().collect();
        scalar_names.sort();

        for (idx, name) in scalar_names.iter().enumerate() {
            let row = (idx + 1) as u32;

            if let Some(var) = included_model.scalars.get(*name) {
                // Write name with namespace prefix
                let prefixed_name = format!("{}.{}", namespace, name);
                worksheet
                    .write_string(row, 0, &prefixed_name)
                    .map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar name: {}", e))
                    })?;

                // Write value (formulas not translated for included scalars yet)
                if let Some(value) = var.value {
                    worksheet.write_number(row, 1, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar value: {}", e))
                    })?;
                }

                // Add metadata as cell note
                if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet.insert_note(row, 1, &note).map_err(|e| {
                        ForgeError::Export(format!("Failed to add scalar note: {}", e))
                    })?;
                }
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
            parts.push(format!("Unit: {}", unit));
        }
        if let Some(notes) = &metadata.notes {
            parts.push(format!("Notes: {}", notes));
        }
        if let Some(source) = &metadata.source {
            parts.push(format!("Source: {}", source));
        }
        if let Some(status) = &metadata.validation_status {
            parts.push(format!("Status: {}", status));
        }
        if let Some(updated) = &metadata.last_updated {
            parts.push(format!("Updated: {}", updated));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }
}
