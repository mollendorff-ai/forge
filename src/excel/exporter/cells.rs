//! Writing a cell: its value or formula, its number format, the cached
//! result a viewer shows, a table's header and the note a value's metadata
//! becomes.

use super::limits::CHECKED;
use super::ExcelExporter;
use crate::error::{ForgeError, ForgeResult};
use crate::excel::text_results::{self, TextCell, TextResult};
use crate::excel::workbook_refs::dates::excel_serial;
use crate::types::{Column, ColumnValue, Metadata, Table};
use rust_xlsxwriter::{Format, Formula, Note, Worksheet};
use std::collections::HashMap;

impl ExcelExporter {
    /// The `Name` and `Value` headers of a sheet of scalars.
    pub(super) fn write_name_value_header(worksheet: &mut Worksheet) {
        for (col, header) in [(0, "Name"), (1, "Value")] {
            worksheet.write_string(0, col, header).expect(CHECKED);
        }
    }

    /// Write a scalar's value: a date as a date cell (ADR-060).
    pub(super) fn write_scalar_value(
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: u32,
        var: &crate::types::Variable,
    ) {
        let Some(value) = var.value else {
            return;
        };
        match Self::scalar_format(var) {
            Some(format) => worksheet.write_number_with_format(row, 1, value, &format),
            None => worksheet.write_number(row, 1, value),
        }
        .expect(CHECKED);
    }

    /// Write a scalar's formula, with its cached value: a date as a date cell.
    pub(super) fn write_scalar_formula(
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: u32,
        var: &crate::types::Variable,
        translated: &crate::excel::workbook_refs::Translated,
    ) {
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
        written.expect(CHECKED);
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
        // The sheet's size was checked: a header fails only for its length
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
            .expect(CHECKED);
        };
        match values {
            ColumnValue::Number(nums) => {
                if let Some(&value) = nums.get(index) {
                    number(worksheet, value);
                }
            },
            // An empty cell is left empty (ADR-059)
            ColumnValue::Blanks(nums) => {
                if let Some(Some(value)) = nums.get(index) {
                    number(worksheet, *value);
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
                                .expect(CHECKED);
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
                    worksheet.write_boolean(row, col, value).expect(CHECKED);
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

    #[test]
    fn a_cached_result_is_what_the_column_holds_at_that_row() {
        let numbers = ColumnValue::Number(vec![1.5, 2.0]);
        let blanks = ColumnValue::Blanks(vec![Some(3.0), None]);
        let text = ColumnValue::Text(vec!["a".to_string()]);
        let dates = ColumnValue::Date(vec!["2026-10-31".to_string(), "soon".to_string()]);
        let bools = ColumnValue::Boolean(vec![true, false]);
        let cached = |values: &ColumnValue, index, is_date| {
            ExcelExporter::cached_result(values, index, is_date)
        };
        assert_eq!(cached(&numbers, 0, false).as_deref(), Some("1.5"));
        assert_eq!(cached(&numbers, 2, false), None);
        assert_eq!(cached(&blanks, 0, false).as_deref(), Some("3"));
        assert_eq!(cached(&blanks, 1, false), None);
        assert_eq!(cached(&text, 0, false).as_deref(), Some("a"));
        assert_eq!(cached(&text, 1, false), None);
        assert_eq!(cached(&bools, 0, false).as_deref(), Some("TRUE"));
        assert_eq!(cached(&bools, 1, false).as_deref(), Some("FALSE"));
        // A date of a column of dates is its day number, and text that is no date has none
        assert_eq!(cached(&dates, 0, true).as_deref(), Some("46326"));
        assert_eq!(cached(&dates, 1, true), None);
        assert_eq!(cached(&dates, 0, false).as_deref(), Some("2026-10-31"));
        // A column that is not text is read as itself, dates or not
        assert_eq!(cached(&numbers, 0, true).as_deref(), Some("1.5"));
    }

    /// The cells of a sheet that `fill` wrote, read back from the saved workbook
    fn read_back(fill: impl FnOnce(&mut Worksheet)) -> calamine::Range<calamine::Data> {
        use calamine::Reader;
        let mut workbook = rust_xlsxwriter::Workbook::new();
        fill(workbook.add_worksheet());
        let bytes = workbook.save_to_buffer().unwrap();
        let mut book = calamine::Xlsx::new(std::io::Cursor::new(bytes)).unwrap();
        book.worksheet_range("Sheet1").unwrap()
    }

    #[test]
    fn a_formula_keeps_no_result_when_the_column_has_none_at_that_row() {
        let column = Column::new("a".to_string(), ColumnValue::Number(vec![1.0]));
        let mut text_results = Vec::new();
        let range = read_back(|sheet| {
            for (index, (column, row)) in [(Some(&column), 0), (Some(&column), 5), (None, 0)]
                .into_iter()
                .enumerate()
            {
                let formula = ExcelExporter::with_cached_result(
                    Formula::new("=A2"),
                    column,
                    (row, false),
                    (
                        TextCell {
                            sheet: 0,
                            position: (0, 0),
                        },
                        &mut text_results,
                    ),
                );
                sheet
                    .write_formula(u32::try_from(index).unwrap(), 0, formula)
                    .unwrap();
            }
        });
        // The result is cached for the first, and the others keep the writer's 0
        let cached: Vec<Option<&calamine::Data>> =
            (0..3).map(|row| range.get_value((row, 0))).collect();
        assert_eq!(
            cached,
            [
                Some(&calamine::Data::Float(1.0)),
                Some(&calamine::Data::Float(0.0)),
                Some(&calamine::Data::Float(0.0))
            ]
        );
        assert!(text_results.is_empty());
    }

    #[test]
    fn a_cell_is_written_only_where_the_column_has_a_value() {
        let write = |sheet: &mut Worksheet, values: &ColumnValue, index, is_date| {
            ExcelExporter::write_cell_value(sheet, (0, 0), values, index, (is_date, None)).unwrap();
        };
        let blanks = ColumnValue::Blanks(vec![Some(3.0), None]);
        let dates = ColumnValue::Date(vec!["2026-10-31".to_string(), "soon".to_string()]);
        let bools = ColumnValue::Boolean(vec![true]);
        let numbers = ColumnValue::Number(vec![1.0]);
        // A cell past the end of its column is empty, and so are a blank and text that is no date
        let range = read_back(|sheet| {
            write(sheet, &blanks, 1, false);
            write(sheet, &dates, 1, true);
            write(sheet, &dates, 2, true);
            write(sheet, &bools, 1, false);
            write(sheet, &numbers, 1, false);
        });
        assert!(range.is_empty());
        let value = |values: &ColumnValue, is_date| {
            read_back(|sheet| write(sheet, values, 0, is_date))
                .get_value((0, 0))
                .cloned()
        };
        assert_eq!(value(&blanks, false), Some(calamine::Data::Float(3.0)));
        assert_eq!(value(&bools, false), Some(calamine::Data::Bool(true)));
        assert_eq!(value(&dates, true).unwrap().to_string(), "46326");
    }

    #[test]
    fn a_header_note_that_is_too_long_is_refused() {
        let mut table = Table::new("t".to_string());
        table.add_column(Column::with_metadata(
            "a".to_string(),
            ColumnValue::Number(vec![1.0]),
            Metadata {
                notes: Some("x".repeat(33_000)),
                ..Metadata::default()
            },
        ));
        let names = vec!["a".to_string()];
        let err = ExcelExporter::write_header(&mut Worksheet::new(), &table, &names)
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "Excel export error: Failed to add note: String exceeds Excel's limit of 32,767 characters."
        );
    }
}
