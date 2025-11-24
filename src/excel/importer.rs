//! Excel importer implementation - Excel (.xlsx) → YAML

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ForgeVersion, ParsedModel, Table, Variable};
use calamine::{open_workbook, DataType, Range, Reader, Xlsx};
use std::collections::HashMap;
use std::path::Path;

/// Excel importer for converting .xlsx files to v1.0.0 YAML models
pub struct ExcelImporter {
    path: std::path::PathBuf,
}

impl ExcelImporter {
    /// Create a new Excel importer
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Import Excel file to ParsedModel
    pub fn import(&self) -> ForgeResult<ParsedModel> {
        // Open Excel workbook
        let mut workbook: Xlsx<_> = open_workbook(&self.path)
            .map_err(|e| ForgeError::IO(format!("Failed to open Excel file: {}", e)))?;

        // Create model
        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        // Get all sheet names
        let sheet_names = workbook.sheet_names().to_vec();

        // Process each sheet
        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                self.process_sheet(&sheet_name, &range, &mut model)?;
            }
        }

        Ok(model)
    }

    /// Process a single worksheet
    fn process_sheet(
        &self,
        sheet_name: &str,
        range: &Range<DataType>,
        model: &mut ParsedModel,
    ) -> ForgeResult<()> {
        // Check if sheet is empty
        if range.is_empty() {
            return Ok(()); // Skip empty sheets
        }

        // Check if this is a "Scalars" sheet (special handling)
        if sheet_name.to_lowercase() == "scalars" {
            return self.process_scalars_sheet(range, model);
        }

        // Process as regular table
        self.process_table_sheet(sheet_name, range, model)
    }

    /// Process a regular table sheet
    fn process_table_sheet(
        &self,
        sheet_name: &str,
        range: &Range<DataType>,
        model: &mut ParsedModel,
    ) -> ForgeResult<()> {
        let (height, width) = range.get_size();

        if height < 2 {
            // Need at least header + 1 data row
            return Ok(()); // Skip sheets with insufficient data
        }

        // Read header row (row 0)
        let mut column_names: Vec<String> = Vec::new();
        for col in 0..width {
            if let Some(cell) = range.get((0, col)) {
                let name = match cell {
                    DataType::String(s) => s.clone(),
                    DataType::Int(i) => i.to_string(),
                    DataType::Float(f) => f.to_string(),
                    _ => format!("col_{}", col),
                };
                column_names.push(name);
            } else {
                column_names.push(format!("col_{}", col));
            }
        }

        // Read data rows and detect column types
        let mut columns_data: HashMap<String, Vec<DataType>> = HashMap::new();
        for col_name in &column_names {
            columns_data.insert(col_name.clone(), Vec::new());
        }

        // Collect all data (skip header row)
        for row in 1..height {
            for col in 0..width {
                let col_name = &column_names[col];
                if let Some(cell) = range.get((row, col)) {
                    columns_data.get_mut(col_name).unwrap().push(cell.clone());
                } else {
                    // Empty cell - use default based on column type
                    columns_data
                        .get_mut(col_name)
                        .unwrap()
                        .push(DataType::Empty);
                }
            }
        }

        // Create table
        let table_name = self.sanitize_table_name(sheet_name);
        let mut table = Table::new(table_name.clone());

        // Convert columns to YAML format
        for col_name in &column_names {
            let data = &columns_data[col_name];

            // Check if column contains formulas
            let has_formulas = data.iter().any(|cell| matches!(cell, DataType::String(s) if s.starts_with('=')));

            if has_formulas {
                // This is a calculated column - extract formula
                if let Some(DataType::String(formula)) = data.first() {
                    if formula.starts_with('=') {
                        // TODO: Translate Excel formula to YAML syntax (Phase 4.3)
                        // For now, store as-is
                        table.add_row_formula(col_name.clone(), formula.clone());
                        continue;
                    }
                }
            }

            // Regular data column - convert to ColumnValue
            let column_value = self.convert_to_column_value(data)?;
            table.add_column(Column::new(col_name.clone(), column_value));
        }

        model.add_table(table);
        Ok(())
    }

    /// Process the "Scalars" sheet (if present)
    fn process_scalars_sheet(
        &self,
        range: &Range<DataType>,
        model: &mut ParsedModel,
    ) -> ForgeResult<()> {
        let (height, _width) = range.get_size();

        // Skip header row, process data rows
        for row in 1..height {
            // Column 0: Name
            // Column 1: Value
            // Column 2: Formula (optional)

            let name = if let Some(cell) = range.get((row, 0)) {
                cell.to_string()
            } else {
                continue; // Skip row without name
            };

            let value = if let Some(cell) = range.get((row, 1)) {
                match cell {
                    DataType::Float(f) => Some(*f),
                    DataType::Int(i) => Some(*i as f64),
                    _ => None,
                }
            } else {
                None
            };

            let formula = if let Some(cell) = range.get((row, 2)) {
                match cell {
                    DataType::String(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            };

            // Create variable
            let variable = Variable {
                value,
                formula,
                alias: None,
                path: None,
            };
            model.add_scalar(name, variable);
        }

        Ok(())
    }

    /// Convert Excel DataType array to ColumnValue
    fn convert_to_column_value(&self, data: &[DataType]) -> ForgeResult<ColumnValue> {
        // Detect column type from first non-empty cell
        let first_type = data
            .iter()
            .find(|cell| !matches!(cell, DataType::Empty))
            .ok_or_else(|| ForgeError::Import("Column has no data".to_string()))?;

        match first_type {
            DataType::Float(_) | DataType::Int(_) => {
                // Number column
                let numbers: Vec<f64> = data
                    .iter()
                    .map(|cell| match cell {
                        DataType::Float(f) => *f,
                        DataType::Int(i) => *i as f64,
                        DataType::Empty => 0.0, // Default for empty cells
                        _ => 0.0,
                    })
                    .collect();
                Ok(ColumnValue::Number(numbers))
            }
            DataType::String(_) => {
                // Text column
                let texts: Vec<String> = data.iter().map(|cell| cell.to_string()).collect();
                Ok(ColumnValue::Text(texts))
            }
            DataType::Bool(_) => {
                // Boolean column
                let bools: Vec<bool> = data
                    .iter()
                    .map(|cell| match cell {
                        DataType::Bool(b) => *b,
                        DataType::Empty => false,
                        _ => false,
                    })
                    .collect();
                Ok(ColumnValue::Boolean(bools))
            }
            _ => {
                // Default to text
                let texts: Vec<String> = data.iter().map(|cell| cell.to_string()).collect();
                Ok(ColumnValue::Text(texts))
            }
        }
    }

    /// Sanitize sheet name to valid YAML key
    fn sanitize_table_name(&self, sheet_name: &str) -> String {
        sheet_name
            .to_lowercase()
            .replace(' ', "_")
            .replace("&", "and")
            .replace("-", "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }
}
