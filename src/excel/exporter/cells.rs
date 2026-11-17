//! Writing a cell: its value or formula, its number format, the cached
//! result a viewer shows, a table's header and the note a value's metadata
//! becomes.

use super::ExcelExporter;
use crate::error::{ForgeError, ForgeResult};
use crate::excel::text_results::{self, TextCell, TextResult};
use crate::excel::workbook_refs::dates::excel_serial;
use crate::types::{Column, ColumnValue, Metadata, Table};
use rust_xlsxwriter::{Format, Formula, Note, Worksheet};
use std::collections::HashMap;

impl ExcelExporter {
    /// Write a scalar's value: a date as a date cell (ADR-060).
    pub(super) fn write_scalar_value(
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: u32,
        var: &crate::types::Variable,
    ) -> ForgeResult<()> {
        let Some(value) = var.value else {
            return Ok(());
        };
        let written = match Self::scalar_format(var) {
            Some(format) => worksheet.write_number_with_format(row, 1, value, &format),
            None => worksheet.write_number(row, 1, value),
        };
        written
            .map(|_| ())
            .map_err(|e| ForgeError::Export(format!("Failed to write scalar value: {e}")))
    }

    /// Write a scalar's formula, with its cached value: a date as a date cell.
    pub(super) fn write_scalar_formula(
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: u32,
        var: &crate::types::Variable,
        translated: &crate::excel::workbook_refs::Translated,
    ) -> ForgeResult<()> {
        let mut formula = Formula::new(&translated.formula);
        if let Some(value) = var.value {
            formula = formula.set_result(value.to_string());
        }
        let written = match (Self::scalar_format(var), translated.has_range) {
            (Some(format), true) => {
                worksheet.write_dynamic_formula_with_format(row, 1, formula, &format)
            },
            (Some(format), false) => worksheet.write_formula_with_format(row, 1, formula, &format),
            (None, true) => worksheet.write_dynamic_formula(row, 1, formula),
            (None, false) => worksheet.write_formula(row, 1, formula),
        };
        written
            .map(|_| ())
            .map_err(|e| ForgeError::Export(format!("Failed to write scalar formula: {e}")))
    }

    /// The cell format of a scalar: its `format:` (ADR-061), else a date's
    /// (ADR-060), else none.
    pub(super) fn scalar_format(var: &crate::types::Variable) -> Option<Format> {
        match (&var.metadata.format, var.is_date) {
            (Some(code), _) => Some(Format::new().set_num_format(code)),
            (None, true) => Some(Self::date_format()),
            (None, false) => None,
        }
    }

    /// The format of a date cell: the day number shown as Forge writes dates.
    pub(super) fn date_format() -> Format {
        Format::new().set_num_format("yyyy-mm-dd")
    }

    /// Forge's calculated value as a cached formula result; a date of a
    /// column of dates as its day number, which is what the cell holds.
    pub(super) fn cached_result(
        values: &ColumnValue,
        index: usize,
        is_date: bool,
    ) -> Option<String> {
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
            ColumnValue::Blanks(v) => v.get(index).copied().flatten().map(|n| n.to_string()),
            ColumnValue::Text(v) | ColumnValue::Date(v) => v.get(index).cloned(),
            ColumnValue::Boolean(v) => v
                .get(index)
                .map(|b| if *b { "TRUE" } else { "FALSE" }.to_string()),
        }
    }

    /// The formula with Forge's result cached in it. A text result that the
    /// writer would read as a number, a boolean or an error (`010`, `TRUE`) is
    /// noted in `text_results`, to be marked as text after the save.
    pub(super) fn with_cached_result(
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

    /// The header row (row 0), with each column's metadata as a note (v4.0),
    /// a formula column's too (ADR-061).
    pub(super) fn write_header(
        worksheet: &mut Worksheet,
        table: &Table,
        column_names: &[String],
    ) -> ForgeResult<()> {
        // Past Excel's last column the writer refuses the cell, long before the counter ends
        for (col, col_name) in (0u16..=u16::MAX).zip(column_names) {
            worksheet
                .write_string(0, col, col_name)
                .map_err(|e| ForgeError::Export(format!("Failed to write header: {e}")))?;
            if let Some(note_text) = table
                .metadata(col_name)
                .and_then(Self::format_metadata_note)
            {
                let note = Note::new(note_text).set_author("Forge");
                worksheet
                    .insert_note(0, col, &note)
                    .map_err(|e| ForgeError::Export(format!("Failed to add note: {e}")))?;
            }
        }
        Ok(())
    }

    /// Each column's `format:`, as the cell format its cells are written
    /// with (ADR-061).
    pub(super) fn column_formats<'t>(
        table: &Table,
        column_names: &'t [String],
    ) -> HashMap<&'t str, Format> {
        column_names
            .iter()
            .filter_map(|name| {
                let code = table.format_of(name)?;
                Some((name.as_str(), Format::new().set_num_format(code)))
            })
            .collect()
    }

    /// Write a single cell value based on column type; `is_date` says the
    /// column is written as dates, and `format` is its `format:` (ADR-061).
    pub(super) fn write_cell_value(
        worksheet: &mut Worksheet,
        cell: (u32, u16),
        values: &ColumnValue,
        index: usize,
        (is_date, format): (bool, Option<&Format>),
    ) -> ForgeResult<()> {
        let (row, col) = cell;
        let number = |worksheet: &mut Worksheet, value: f64| {
            match format {
                Some(format) => worksheet.write_number_with_format(row, col, value, format),
                None => worksheet.write_number(row, col, value),
            }
            .map(|_| ())
            .map_err(|e| ForgeError::Export(format!("Failed to write number: {e}")))
        };
        match values {
            ColumnValue::Number(nums) => {
                if let Some(&value) = nums.get(index) {
                    number(worksheet, value)?;
                }
            },
            // An empty cell is left empty (ADR-059)
            ColumnValue::Blanks(nums) => {
                if let Some(Some(value)) = nums.get(index) {
                    number(worksheet, *value)?;
                }
            },
            // A column of dates is written as dates: the day number, shown
            // as Forge writes it (ADR-054). Any other text is text.
            ColumnValue::Text(texts) | ColumnValue::Date(texts) => {
                if let Some(value) = texts.get(index) {
                    if is_date {
                        if let Some(serial) = excel_serial(value) {
                            let date_format = Self::date_format();
                            worksheet
                                .write_number_with_format(
                                    row,
                                    col,
                                    serial,
                                    format.unwrap_or(&date_format),
                                )
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

    /// Format metadata fields as a note text for Excel cell comments (v4.0)
    /// Returns None if metadata is empty
    pub(super) fn format_metadata_note(metadata: &Metadata) -> Option<String> {
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
    use crate::types::Metadata;

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
            format: None,
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

    mod cached_text {
        //! A formula that gives text keeps that text in the workbook (roadmap
        //! T3-CACHED-TEXT).
        //!
        //! The workbook writer typed a cached result by reading it, so `010` was
        //! stored as the number 10 and `TRUE` as a boolean. A viewer that does not
        //! recalculate (Quick Look, iOS Files) then showed 10 and TRUE for the text
        //! `010` and `TRUE`.

        use crate::cli::export_core;
        use calamine::{open_workbook, Data, Reader, Xlsx};

        const MODEL: &str = r#"_forge_version: "5.1.0"
t:
  n: [1, 25, 40]
  padded: "=TEXT(n, \"000\")"
  word: "=IF(n > 20, \"TRUE\", \"FALSE\")"
  exponent: "=\"1e\" & n"
  plain: "=IF(n > 20, \"high\", \"low\")"
  flag: "=n > 20"
  double: "=n * 2"
"#;

        fn cells(column: u32) -> Vec<Data> {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("m.yaml");
            std::fs::write(&model, MODEL).unwrap();
            let xlsx = dir.path().join("m.xlsx");
            export_core(&model, &xlsx).unwrap();
            let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
            let range = book.worksheet_range("t").unwrap();
            (1..=3)
                .map(|row| range.get_value((row, column)).cloned().unwrap())
                .collect()
        }

        fn text(values: &[&str]) -> Vec<Data> {
            values.iter().map(|v| Data::String((*v).into())).collect()
        }

        #[test]
        fn text_that_looks_like_a_number_is_cached_as_text() {
            assert_eq!(cells(1), text(&["001", "025", "040"]));
            assert_eq!(cells(3), text(&["1e1", "1e25", "1e40"]));
        }

        #[test]
        fn the_words_true_and_false_are_cached_as_text() {
            assert_eq!(cells(2), text(&["FALSE", "TRUE", "TRUE"]));
        }

        #[test]
        fn other_results_keep_their_type() {
            assert_eq!(cells(4), text(&["low", "high", "high"]));
            assert_eq!(
                cells(5),
                [Data::Bool(false), Data::Bool(true), Data::Bool(true)]
            );
            assert_eq!(
                cells(6),
                [Data::Float(2.0), Data::Float(50.0), Data::Float(80.0)]
            );
        }

        #[test]
        fn the_formulas_are_still_there() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("m.yaml");
            std::fs::write(&model, MODEL).unwrap();
            let xlsx = dir.path().join("m.xlsx");
            export_core(&model, &xlsx).unwrap();
            let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
            let formulas = book.worksheet_formula("t").unwrap();
            assert!(formulas
                .get_value((1, 1))
                .is_some_and(|f| f.contains("TEXT(")));
        }
    }

    mod text_numbers {
        //! Text that looks like a number, in the workbook (roadmap T1-TEXT-NUMBERS,
        //! ADR-054 decision 6).
        //!
        //! Measured on `s: ["5", "apple", "2", "x"]` with the formulas written live,
        //! as Forge 10.0.0-rc.12 writes them: `SUM(t.s)` is 7 in Forge and 0 in
        //! Gnumeric and `LibreOffice`, `COUNTIF(t.s, ">3")` 1 and 0, and
        //! `IFERROR(IF(s > 3, 1, 0), -1)` is 1, -1, 0, -1 in Forge and 1 in every row
        //! in both (any text is greater than any number there).

        use crate::cli::export_core;
        use calamine::{open_workbook, Data, Reader, Xlsx};

        const MODEL: &str = r#"_forge_version: "5.1.0"
t:
  s: ["5", "apple", "2", "x"]
  region: ["north", "south", "east", "west"]
  n: [1, 2, 3, 4]
  twice: "=IFERROR(s * 2, -1)"
  above: "=IFERROR(IF(s > 3, 1, 0), -1)"
  is_five: "=IF(s = \"5\", 1, 0)"
  size: "=LEN(s)"
  label: "=region & \"-\" & s"
r:
  total: {formula: "=SUM(t.s)"}
  over_three: {formula: "=COUNTIF(t.s, \">3\")"}
  apples: {formula: "=COUNTIF(t.s, \"apple\")"}
  filled: {formula: "=COUNTA(t.s)"}
  words: {formula: "=SUMPRODUCT(t.region, t.n)"}
"#;

        #[test]
        fn text_with_digits_read_as_a_number_is_written_as_values() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("m.yaml");
            std::fs::write(&model, MODEL).unwrap();
            let xlsx = dir.path().join("m.xlsx");
            let warnings = export_core(&model, &xlsx).unwrap().warnings;
            let names: Vec<&str> = warnings
                .iter()
                .filter_map(|w| w.split_once(':').map(|(name, _)| name.trim()))
                .collect();
            assert_eq!(
                names,
                ["t.twice", "t.above", "r.total", "r.over_three"],
                "{warnings:?}"
            );

            let mut workbook: Xlsx<_> = open_workbook(&xlsx).unwrap();
            let t = workbook.worksheet_formula("t").unwrap();
            let r = workbook.worksheet_formula("r").unwrap();
            let values = workbook.worksheet_range("r").unwrap();
            // Read as text: the same in both, so live (columns F is_five, G size,
            // H label; r rows 4 to 6)
            assert_eq!(t.get_value((1, 5)).unwrap(), "IF(A2 = \"5\", 1, 0)");
            assert_eq!(t.get_value((1, 6)).unwrap(), "LEN(A2)");
            assert_eq!(t.get_value((1, 7)).unwrap(), "B2 & \"-\" & A2");
            assert_eq!(
                r.get_value((3, 1)).unwrap(),
                "COUNTIF('t'!$A$2:$A$5, \"apple\")"
            );
            assert_eq!(r.get_value((4, 1)).unwrap(), "COUNTA('t'!$A$2:$A$5)");
            // Words read as numbers are nothing in both
            assert_eq!(
                r.get_value((5, 1)).unwrap(),
                "SUMPRODUCT('t'!$B$2:$B$5, 't'!$C$2:$C$5)"
            );
            // Forge's values, with no formula
            assert_eq!(r.get_value((1, 1)).map_or("", String::as_str), "");
            assert_eq!(values.get_value((1, 1)), Some(&Data::Float(7.0)));
            assert_eq!(values.get_value((2, 1)), Some(&Data::Float(1.0)));
        }
    }
}
