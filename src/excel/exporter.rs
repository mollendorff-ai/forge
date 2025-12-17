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
                .map_or(0, super::super::types::Column::len);

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
                let prefixed_name = format!("{namespace}.{table_name}");
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
            .map_err(|e| ForgeError::IO(format!("Failed to save Excel file: {e}")))?;

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
            .map_err(|e| ForgeError::Export(format!("Failed to set worksheet name: {e}")))?;

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

        // Get row count from first data column
        let row_count = table
            .columns
            .values()
            .next()
            .map_or(0, super::super::types::Column::len);

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
                        .map_err(|e| ForgeError::Export(format!("Failed to write formula: {e}")))?;
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

    /// Export scalars to a dedicated "Scalars" worksheet
    fn export_scalars(&self, workbook: &mut Workbook) -> ForgeResult<()> {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Scalars").map_err(|e| {
            ForgeError::Export(format!("Failed to set Scalars worksheet name: {e}"))
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
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;
        worksheet
            .write_string(0, 1, "Value")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;

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
                worksheet
                    .write_string(row, 0, *name)
                    .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {e}")))?;

                // Write formula or value
                if let Some(formula) = &var.formula {
                    // Translate and write as actual Excel formula
                    match translator.translate_scalar_formula(formula, &scalar_row_map) {
                        Ok(excel_formula) => {
                            worksheet
                                .write_formula(row, 1, Formula::new(&excel_formula))
                                .map_err(|e| {
                                    ForgeError::Export(format!(
                                        "Failed to write scalar formula: {e}"
                                    ))
                                })?;
                        },
                        Err(_) => {
                            // Fallback: write calculated value if formula translation fails
                            if let Some(value) = var.value {
                                worksheet.write_number(row, 1, value).map_err(|e| {
                                    ForgeError::Export(format!("Failed to write scalar value: {e}"))
                                })?;
                            }
                        },
                    }
                } else if let Some(value) = var.value {
                    // No formula, just write the value
                    worksheet.write_number(row, 1, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar value: {e}"))
                    })?;
                }

                // Add metadata as cell note for scalars (v4.0)
                if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet.insert_note(row, 1, &note).map_err(|e| {
                        ForgeError::Export(format!("Failed to add scalar note: {e}"))
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
        let sheet_name = format!("{namespace}.Scalars");
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name).map_err(|e| {
            ForgeError::Export(format!("Failed to set {sheet_name} worksheet name: {e}"))
        })?;

        // Write header row
        worksheet
            .write_string(0, 0, "Name")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;
        worksheet
            .write_string(0, 1, "Value")
            .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;

        // Write scalars (sorted by name)
        let mut scalar_names: Vec<&String> = included_model.scalars.keys().collect();
        scalar_names.sort();

        for (idx, name) in scalar_names.iter().enumerate() {
            let row = (idx + 1) as u32;

            if let Some(var) = included_model.scalars.get(*name) {
                // Write name with namespace prefix
                let prefixed_name = format!("{namespace}.{name}");
                worksheet
                    .write_string(row, 0, &prefixed_name)
                    .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {e}")))?;

                // Write value (formulas not translated for included scalars yet)
                if let Some(value) = var.value {
                    worksheet.write_number(row, 1, value).map_err(|e| {
                        ForgeError::Export(format!("Failed to write scalar value: {e}"))
                    })?;
                }

                // Add metadata as cell note
                if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                    let note = Note::new(note_text).set_author("Forge");
                    worksheet.insert_note(row, 1, &note).map_err(|e| {
                        ForgeError::Export(format!("Failed to add scalar note: {e}"))
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
        assert!(exporter.table_column_maps.is_empty());
        assert!(exporter.table_row_counts.is_empty());
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

        assert!(exporter.table_column_maps.contains_key("sales"));
        assert_eq!(exporter.table_row_counts.get("sales"), Some(&3));
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
        let col_map = exporter.table_column_maps.get("calc").unwrap();
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

        assert!(exporter.table_column_maps.contains_key("sales"));
        assert!(exporter.table_column_maps.contains_key("costs"));
        assert_eq!(exporter.table_row_counts.get("sales"), Some(&2));
        assert_eq!(exporter.table_row_counts.get("costs"), Some(&3));
    }

    #[test]
    fn test_exporter_column_maps_sorted_alphabetically() {
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
        let col_map = exporter.table_column_maps.get("data").unwrap();

        // alpha -> A, beta -> B, zebra -> C (alphabetical order)
        assert_eq!(col_map.get("alpha"), Some(&"A".to_string()));
        assert_eq!(col_map.get("beta"), Some(&"B".to_string()));
        assert_eq!(col_map.get("zebra"), Some(&"C".to_string()));
    }

    #[test]
    fn test_exporter_empty_table() {
        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);
        assert_eq!(exporter.table_row_counts.get("empty"), Some(&0));
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
}
