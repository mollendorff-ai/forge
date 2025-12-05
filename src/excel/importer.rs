//! Excel importer implementation - Excel (.xlsx) → YAML

use crate::error::{ForgeError, ForgeResult};
use crate::excel::reverse_formula_translator::ReverseFormulaTranslator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
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
        let mut model = ParsedModel::new();

        // Get all sheet names
        let sheet_names = workbook.sheet_names().to_vec();

        // Process each sheet
        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                self.process_sheet(&sheet_name, &range, &mut workbook, &mut model)?;
            }
        }

        Ok(model)
    }

    /// Process a single worksheet
    fn process_sheet(
        &self,
        sheet_name: &str,
        range: &Range<Data>,
        workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>,
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

        // Get formula range for this sheet
        let formula_range = workbook.worksheet_formula(sheet_name).ok();

        // Process as regular table
        self.process_table_sheet(sheet_name, range, formula_range.as_ref(), model)
    }

    /// Process a regular table sheet
    fn process_table_sheet(
        &self,
        sheet_name: &str,
        range: &Range<Data>,
        formula_range: Option<&Range<String>>,
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
                    Data::String(s) => s.clone(),
                    Data::Int(i) => i.to_string(),
                    Data::Float(f) => f.to_string(),
                    _ => format!("col_{}", col),
                };
                column_names.push(name);
            } else {
                column_names.push(format!("col_{}", col));
            }
        }

        // Read data rows and detect column types
        let mut columns_data: HashMap<String, Vec<Data>> = HashMap::new();
        for col_name in &column_names {
            columns_data.insert(col_name.clone(), Vec::new());
        }

        // Collect all data (skip header row)
        for row in 1..height {
            for (col, col_name) in column_names.iter().enumerate().take(width) {
                if let Some(cell) = range.get((row, col)) {
                    columns_data.get_mut(col_name).unwrap().push(cell.clone());
                } else {
                    // Empty cell - use default based on column type
                    columns_data.get_mut(col_name).unwrap().push(Data::Empty);
                }
            }
        }

        // Create table
        let table_name = self.sanitize_table_name(sheet_name);
        let mut table = Table::new(table_name.clone());

        // Build column map for formula translation (A → revenue, B → cogs, etc.)
        let mut column_map = HashMap::new();
        for (idx, col_name) in column_names.iter().enumerate() {
            let excel_col = self.number_to_column_letter(idx);
            column_map.insert(excel_col, col_name.clone());
        }

        // Create reverse formula translator
        let translator = ReverseFormulaTranslator::new(column_map);

        // Convert columns to YAML format
        for (col_idx, col_name) in column_names.iter().enumerate() {
            // Check if this column has formulas (check first data row in formula_range)
            let has_formula = if let Some(formulas) = formula_range {
                // Row 1 (first data row) = index 1 in the formula range
                if let Some(formula_cell) = formulas.get((1, col_idx)) {
                    !formula_cell.is_empty()
                } else {
                    false
                }
            } else {
                false
            };

            if has_formula {
                // This is a calculated column - extract formula from first data row
                if let Some(formulas) = formula_range {
                    if let Some(formula) = formulas.get((1, col_idx)) {
                        if !formula.is_empty() {
                            // Add leading = if not present (calamine strips it)
                            let formula_with_equals = if formula.starts_with('=') {
                                formula.clone()
                            } else {
                                format!("={}", formula)
                            };

                            // Translate Excel formula to YAML syntax
                            let yaml_formula = translator.translate(&formula_with_equals)?;
                            table.add_row_formula(col_name.clone(), yaml_formula);
                            // Skip this column - don't add as data
                            continue;
                        }
                    }
                }
            }

            // Regular data column - convert to ColumnValue
            let data = &columns_data[col_name];
            // Skip if all data is empty (formula columns may show as empty/zero values)
            if data.iter().all(|cell| matches!(cell, Data::Empty)) {
                continue;
            }
            let column_value = self.convert_to_column_value(data)?;
            table.add_column(Column::new(col_name.clone(), column_value));
        }

        model.add_table(table);
        Ok(())
    }

    /// Process the "Scalars" sheet (if present)
    fn process_scalars_sheet(
        &self,
        range: &Range<Data>,
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
                    Data::Float(f) => Some(*f),
                    Data::Int(i) => Some(*i as f64),
                    _ => None,
                }
            } else {
                None
            };

            let formula = if let Some(cell) = range.get((row, 2)) {
                match cell {
                    Data::String(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            };

            // Create variable
            let variable = Variable::new(name.clone(), value, formula);
            model.add_scalar(name, variable);
        }

        Ok(())
    }

    /// Convert Excel Data array to ColumnValue
    fn convert_to_column_value(&self, data: &[Data]) -> ForgeResult<ColumnValue> {
        // Detect column type from first non-empty cell
        let first_type = data
            .iter()
            .find(|cell| !matches!(cell, Data::Empty))
            .ok_or_else(|| ForgeError::Import("Column has no data".to_string()))?;

        match first_type {
            Data::Float(_) | Data::Int(_) => {
                // Number column
                let numbers: Vec<f64> = data
                    .iter()
                    .map(|cell| match cell {
                        Data::Float(f) => *f,
                        Data::Int(i) => *i as f64,
                        Data::Empty => 0.0, // Default for empty cells
                        _ => 0.0,
                    })
                    .collect();
                Ok(ColumnValue::Number(numbers))
            }
            Data::String(_) => {
                // Text column
                let texts: Vec<String> = data.iter().map(|cell| cell.to_string()).collect();
                Ok(ColumnValue::Text(texts))
            }
            Data::Bool(_) => {
                // Boolean column
                let bools: Vec<bool> = data
                    .iter()
                    .map(|cell| match cell {
                        Data::Bool(b) => *b,
                        Data::Empty => false,
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

    /// Convert column index to Excel column letter (0→A, 1→B, 25→Z, 26→AA, etc.)
    fn number_to_column_letter(&self, n: usize) -> String {
        let mut result = String::new();
        let mut num = n;

        loop {
            let remainder = num % 26;
            result.insert(0, (b'A' + remainder as u8) as char);
            if num < 26 {
                break;
            }
            num = num / 26 - 1;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_importer() -> ExcelImporter {
        ExcelImporter::new(PathBuf::from("test.xlsx"))
    }

    #[test]
    fn test_number_to_column_letter() {
        let importer = create_test_importer();

        // Single letters
        assert_eq!(importer.number_to_column_letter(0), "A");
        assert_eq!(importer.number_to_column_letter(1), "B");
        assert_eq!(importer.number_to_column_letter(25), "Z");

        // Double letters
        assert_eq!(importer.number_to_column_letter(26), "AA");
        assert_eq!(importer.number_to_column_letter(27), "AB");
        assert_eq!(importer.number_to_column_letter(51), "AZ");
        assert_eq!(importer.number_to_column_letter(52), "BA");

        // Triple letters
        assert_eq!(importer.number_to_column_letter(702), "AAA");
    }

    #[test]
    fn test_sanitize_table_name() {
        let importer = create_test_importer();

        assert_eq!(importer.sanitize_table_name("Sheet1"), "sheet1");
        assert_eq!(
            importer.sanitize_table_name("P&L Statement"),
            "pandl_statement"
        );
        assert_eq!(importer.sanitize_table_name("Revenue-2025"), "revenue_2025");
        assert_eq!(
            importer.sanitize_table_name("Special@#$Chars"),
            "specialchars"
        );
    }

    #[test]
    fn test_convert_to_column_value_numbers() {
        let importer = create_test_importer();
        let data = vec![
            Data::Float(100.0),
            Data::Float(200.0),
            Data::Int(300),
            Data::Empty,
        ];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_eq!(nums[0], 100.0);
                assert_eq!(nums[1], 200.0);
                assert_eq!(nums[2], 300.0);
                assert_eq!(nums[3], 0.0); // Empty → 0.0
            }
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_text() {
        let importer = create_test_importer();
        let data = vec![
            Data::String("Apple".to_string()),
            Data::String("Banana".to_string()),
            Data::Empty,
        ];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts.len(), 3);
                assert_eq!(texts[0], "Apple");
                assert_eq!(texts[1], "Banana");
                assert_eq!(texts[2], ""); // Empty → empty string
            }
            _ => panic!("Expected Text column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_boolean() {
        let importer = create_test_importer();
        let data = vec![Data::Bool(true), Data::Bool(false), Data::Empty];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools.len(), 3);
                assert!(bools[0]);
                assert!(!bools[1]);
                assert!(!bools[2]); // Empty → false
            }
            _ => panic!("Expected Boolean column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_empty() {
        let importer = create_test_importer();
        let data = vec![Data::Empty, Data::Empty];

        // Should return error - no data to detect type
        let result = importer.convert_to_column_value(&data);
        assert!(result.is_err());
    }

    // =========================================================================
    // Import Integration Tests
    // =========================================================================

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

        // Create a model and export it
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

        // Now import it
        let importer = ExcelImporter::new(&excel_path);
        let imported = importer.import().unwrap();

        assert!(imported.tables.contains_key("sales"));
        let table = imported.tables.get("sales").unwrap();
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
        let imported = importer.import().unwrap();

        assert!(imported.tables.contains_key("products"));
        let table = imported.tables.get("products").unwrap();
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
        let imported = importer.import().unwrap();

        assert!(imported.tables.contains_key("revenue"));
        assert!(imported.tables.contains_key("costs"));
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
        let imported = importer.import().unwrap();

        // This test verifies the import process completes successfully
        // The import creates a valid ParsedModel - just verify it exists
        let _ = &imported; // imported is a valid ParsedModel
    }

    #[test]
    fn test_importer_new_stores_path() {
        let path = std::path::Path::new("/some/path/file.xlsx");
        let importer = ExcelImporter::new(path);
        // Just verify it compiles and doesn't panic
        assert!(!importer.path.to_str().unwrap().is_empty());
    }

    #[test]
    fn test_convert_to_column_value_mixed_numeric() {
        let importer = create_test_importer();

        // Mix of Float and Int should all convert to numbers
        let data = vec![
            Data::Float(1.5),
            Data::Int(2),
            Data::Float(3.0),
            Data::Int(4),
        ];

        let result = importer.convert_to_column_value(&data).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_eq!(nums[0], 1.5);
                assert_eq!(nums[1], 2.0);
                assert_eq!(nums[2], 3.0);
                assert_eq!(nums[3], 4.0);
            }
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_sanitize_table_name_spaces() {
        let importer = create_test_importer();

        assert_eq!(importer.sanitize_table_name("My Sheet"), "my_sheet");
        assert_eq!(
            importer.sanitize_table_name("Financial Data 2024"),
            "financial_data_2024"
        );
        assert_eq!(
            importer.sanitize_table_name("   trimmed   "),
            "___trimmed___"
        );
    }

    #[test]
    fn test_number_to_column_letter_extended() {
        let importer = create_test_importer();

        // Additional edge cases
        assert_eq!(importer.number_to_column_letter(703), "AAB");
        assert_eq!(importer.number_to_column_letter(704), "AAC");

        // Large column numbers
        assert_eq!(importer.number_to_column_letter(16383), "XFD"); // Max Excel column
    }
}
