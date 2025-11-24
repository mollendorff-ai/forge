//! Excel exporter implementation

use crate::error::{ForgeError, ForgeResult};
use crate::types::{ColumnValue, ParsedModel, Table};
use rust_xlsxwriter::{Formula, Workbook, Worksheet};
use std::collections::HashMap;
use std::path::Path;

/// Excel exporter for v1.0.0 array models
pub struct ExcelExporter {
    model: ParsedModel,
}

impl ExcelExporter {
    /// Create a new Excel exporter
    pub fn new(model: ParsedModel) -> Self {
        Self { model }
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

        // Build column name → Excel column letter mapping
        let column_map: HashMap<String, String> = column_names
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let col_letter = super::FormulaTranslator::column_index_to_letter(idx);
                (name.clone(), col_letter)
            })
            .collect();

        // Create formula translator
        let translator = super::FormulaTranslator::new(column_map);

        // Write header row (row 0)
        for (col_idx, col_name) in column_names.iter().enumerate() {
            worksheet
                .write_string(0, col_idx as u16, col_name)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;
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
                        .map_err(|e| ForgeError::Export(format!("Failed to write formula: {}", e)))?;
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
                    worksheet
                        .write_number(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write number: {}", e)))?;
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
                    worksheet
                        .write_boolean(row, col, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write boolean: {}", e)))?;
                }
            }
        }
        Ok(())
    }

    /// Export scalars to a dedicated "Scalars" worksheet
    fn export_scalars(&self, workbook: &mut Workbook) -> ForgeResult<()> {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name("Scalars")
            .map_err(|e| ForgeError::Export(format!("Failed to set Scalars worksheet name: {}", e)))?;

        // Write header row
        worksheet
            .write_string(0, 0, "Name")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;
        worksheet
            .write_string(0, 1, "Value")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;
        worksheet
            .write_string(0, 2, "Formula")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {}", e)))?;

        // Write scalars (sorted by name for deterministic output)
        let mut scalar_names: Vec<&String> = self.model.scalars.keys().collect();
        scalar_names.sort();

        for (idx, name) in scalar_names.iter().enumerate() {
            let row = (idx + 1) as u32; // +1 for header row

            if let Some(var) = self.model.scalars.get(*name) {
                // Write name
                worksheet
                    .write_string(row, 0, *name)
                    .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {}", e)))?;

                // Write value (if present)
                if let Some(value) = var.value {
                    worksheet
                        .write_number(row, 1, value)
                        .map_err(|e| ForgeError::Export(format!("Failed to write scalar value: {}", e)))?;
                }

                // Write formula (if present) - Phase 3.4 will translate these
                if let Some(formula) = &var.formula {
                    worksheet
                        .write_string(row, 2, formula)
                        .map_err(|e| ForgeError::Export(format!("Failed to write scalar formula: {}", e)))?;
                }
            }
        }

        Ok(())
    }
}
