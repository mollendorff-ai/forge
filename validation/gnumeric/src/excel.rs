//! Excel helpers for E2E testing.
//!
//! Provides reading of Excel files to verify exports.

#![allow(dead_code)]

use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::{Formula, Workbook, XlsxError};

/// Creates a test Excel file with scalars for import testing.
///
/// # Errors
///
/// Returns an error if the workbook cannot be created or saved.
pub fn create_test_scalars_xlsx(path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Scalars")?;

    sheet.write(0, 0, "Name")?;
    sheet.write(0, 1, "Value")?;

    sheet.write(1, 0, "revenue")?;
    sheet.write(1, 1, 100_000.0)?;

    sheet.write(2, 0, "costs")?;
    sheet.write(2, 1, 40_000.0)?;

    sheet.write(3, 0, "profit")?;
    sheet.write_formula(3, 1, Formula::new("=B2-B3"))?;

    workbook.save(path)?;
    Ok(())
}

/// Cell value from an Excel file.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(String),
}

impl CellValue {
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl From<&Data> for CellValue {
    #[allow(clippy::cast_precision_loss)]
    fn from(dt: &Data) -> Self {
        match dt {
            Data::Empty => Self::Empty,
            Data::Int(i) => Self::Number(*i as f64),
            Data::Float(f) => Self::Number(*f),
            Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => Self::Text(s.clone()),
            Data::Bool(b) => Self::Bool(*b),
            Data::Error(e) => Self::Error(format!("{e:?}")),
            Data::DateTime(dt) => Self::Number(dt.as_f64()),
        }
    }
}

/// Sheet data from an Excel file.
pub type SheetData = Vec<(String, Vec<Vec<CellValue>>)>;

/// Reads an Excel file and returns sheet data.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or a sheet cannot be read.
pub fn read_xlsx(path: &Path) -> Result<SheetData, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("Failed to open Excel file: {e}"))?;

    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::new();

    for name in sheet_names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| format!("Failed to read sheet {name}: {e}"))?;

        let mut rows = Vec::new();
        for row in range.rows() {
            let cells: Vec<CellValue> = row.iter().map(CellValue::from).collect();
            rows.push(cells);
        }
        sheets.push((name, rows));
    }

    Ok(sheets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_value_as_number() {
        let num = CellValue::Number(42.0);
        assert_eq!(num.as_number(), Some(42.0));
    }

    #[test]
    fn cell_value_as_text() {
        let text = CellValue::Text("hello".to_string());
        assert_eq!(text.as_text(), Some("hello"));
    }
}
