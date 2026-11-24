//! One worksheet as the importer reads it: its cells, formulas and number
//! formats, and what a column of cells holds.

use super::super::workbook_refs::column_letter;
use super::super::workbook_refs::dates::excel_date;
use crate::core::precision;
use calamine::{Data, Range};

/// One worksheet read from the file.
pub(super) struct Sheet {
    pub(super) name: String,
    pub(super) values: Range<Data>,
    pub(super) formulas: Option<Range<String>>,
    /// Each cell's style, for its number format (ADR-061)
    pub(super) styles: Option<calamine::StyleRange>,
    /// The worksheet row (0-based) of each row of `values`: the rows in
    /// order, unless `layout::tidy` took out titles, blank rows or a total
    pub(super) rows: Vec<u32>,
    /// A total row under a table (`layout::tidy`): its worksheet row and cells
    pub(super) total: Option<(u32, Vec<Data>)>,
    /// Other sheets read its second column one cell at a time, never as a
    /// range (`cells_read::mark`): a sheet of scalars, whatever its header
    pub(super) read_one_at_a_time: bool,
}

impl Sheet {
    /// A worksheet as read, every row of its used range in place.
    pub(super) fn new(
        name: String,
        values: Range<Data>,
        formulas: Option<Range<String>>,
        styles: Option<calamine::StyleRange>,
    ) -> Self {
        let (height, _) = values.get_size();
        let r0 = values.start().map_or(0, |(r, _)| r);
        let rows = (0..height)
            .map(|row| r0.saturating_add(u32::try_from(row).unwrap_or(u32::MAX)))
            .collect();
        Self {
            name,
            values,
            formulas,
            styles,
            rows,
            total: None,
            read_one_at_a_time: false,
        }
    }

    /// Each formula of the sheet, with the worksheet row (1-based, as Excel
    /// numbers it) of its cell.
    pub(super) fn formula_cells(&self) -> impl Iterator<Item = (u32, &String)> {
        self.formulas.iter().flat_map(|formulas| {
            let r0 = formulas.start().map_or(0, |(r, _)| r);
            formulas.used_cells().map(move |(row, _, formula)| {
                let row = u32::try_from(row).unwrap_or(u32::MAX);
                (r0.saturating_add(row).saturating_add(1), formula)
            })
        })
    }

    /// The position in the worksheet of the cell at a 0-based `row` and `col`
    /// of the used range of values.
    pub(super) fn at(&self, row: usize, col: usize) -> (u32, u32) {
        let c0 = self.values.start().map_or(0, |(_, c)| c);
        (
            self.rows.get(row).copied().unwrap_or(u32::MAX),
            c0.saturating_add(u32::try_from(col).unwrap_or(u32::MAX)),
        )
    }

    /// The number-format code of a cell, by 0-based position relative to the
    /// used range of values.
    pub(super) fn format_code(&self, row: usize, col: usize) -> Option<String> {
        let style = self.styles.as_ref()?.get(self.at(row, col))?;
        style.number_format.as_ref().map(|f| f.format_code.clone())
    }

    /// The `format:` of the cells at `rows` of column `col`, as `forge import`
    /// writes it (ADR-061): their one code, if they share one that Forge
    /// reads and that is not `General` (nor, on dates, the `yyyy-mm-dd` Forge
    /// writes for a date without a format). A code Forge does not read, or
    /// cells with different codes, give a warning and no format.
    pub(super) fn imported_format(
        &self,
        place: &str,
        cells: &[(usize, usize)],
        is_date: bool,
        warnings: &mut Vec<String>,
    ) -> Option<String> {
        use crate::core::array_calculator::evaluator::text::number_format::{
            check_format, FormatKind,
        };
        let mut codes: Vec<String> = Vec::new();
        for &(row, col) in cells {
            let code = self
                .format_code(row, col)
                .unwrap_or_else(|| "General".to_string());
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
        match codes.as_slice() {
            [] => None,
            [code] => {
                if is_date && code == "yyyy-mm-dd" {
                    return None;
                }
                match check_format(code) {
                    Ok(FormatKind::General) => None,
                    Ok(FormatKind::Date) if !is_date => None,
                    Ok(FormatKind::Number) if is_date => None,
                    Ok(_) => Some(code.clone()),
                    Err(why) => {
                        warnings.push(format!(
                            "{place}: the number format \"{code}\" is not imported: {}",
                            why.trim_start_matches("TEXT: ")
                        ));
                        None
                    },
                }
            },
            several => {
                warnings.push(format!(
                    "{place}: its cells have different number formats ({}), so none is imported",
                    several.join(", ")
                ));
                None
            },
        }
    }
    /// Cell text by 0-based position relative to the used range of values.
    pub(super) fn header(&self, col: usize) -> Option<String> {
        match self.values.get((0, col))? {
            Data::String(s) => Some(s.clone()),
            Data::Int(i) => Some(i.to_string()),
            Data::Float(f) => Some(f.to_string()),
            _ => None,
        }
    }

    /// `Name` / `Value` (optionally `Formula`) header: a scalar sheet.
    pub(super) fn is_scalar_sheet(&self) -> bool {
        let (_, width) = self.values.get_size();
        let name = self
            .header(0)
            .is_some_and(|h| h.eq_ignore_ascii_case("name"));
        let value = self
            .header(1)
            .is_some_and(|h| h.eq_ignore_ascii_case("value"));
        let third = width < 3
            || self
                .header(2)
                .is_some_and(|h| h.eq_ignore_ascii_case("formula"));
        (name && value && third) || self.is_label_sheet()
    }

    /// Formula of the cell at a 0-based position relative to the value range.
    pub(super) fn formula(&self, row: usize, col: usize) -> Option<String> {
        self.formula_at(self.at(row, col))
    }

    /// Formula of the cell at a position in the worksheet.
    pub(super) fn formula_at(&self, at: (u32, u32)) -> Option<String> {
        let f = self.formulas.as_ref()?.get_value(at)?;
        if f.is_empty() {
            None
        } else if f.starts_with('=') {
            Some(f.clone())
        } else {
            Some(format!("={f}"))
        }
    }

    /// Excel row number (1-based) of a 0-based row in the value range.
    pub(super) fn excel_row(&self, row: usize) -> u32 {
        self.at(row, 0).0.saturating_add(1)
    }

    /// Excel column letter of a 0-based column in the value range.
    /// The Excel rows of a table's data: every row below the header.
    pub(super) fn data_rows(&self) -> Vec<u32> {
        let (height, _) = self.values.get_size();
        (1..height).map(|row| self.excel_row(row)).collect()
    }

    /// The Excel row of each column's last data cell with a value or a
    /// formula in it, by column letter, for a column that ends above the
    /// table's last row (a shorter column, ADR-059).
    pub(super) fn column_ends(&self) -> std::collections::HashMap<String, u32> {
        let (height, width) = self.values.get_size();
        (0..width)
            .filter_map(|col| {
                let filled = |row: &usize| {
                    self.formula(*row, col).is_some()
                        || !matches!(self.values.get((*row, col)), None | Some(Data::Empty))
                };
                let last = (1..height).rev().find(filled)?;
                (last + 1 < height).then(|| (self.letter(col), self.excel_row(last)))
            })
            .collect()
    }

    pub(super) fn letter(&self, col: usize) -> String {
        let start = self.values.start().map_or(0, |(_, c)| c as usize);
        column_letter(start + col)
    }
}

/// The cells, with a number that is not finite (NaN or an infinity) as Excel's
/// `#NUM!`. Excel never writes one, but a workbook an older Forge exported can
/// hold a cached `NaN`, which was imported as `.nan`: now it is imported as an
/// error cell is, and a model holds no number that is not finite.
pub(super) fn finite_cells(mut values: Range<Data>) -> Range<Data> {
    let (height, width) = values.get_size();
    for row in 0..height {
        for col in 0..width {
            let cell = &mut values[(row, col)];
            if number_of(cell).is_some_and(|n| !n.is_finite()) {
                *cell = Data::Error(calamine::CellErrorType::Num);
            }
        }
    }
    values
}

/// What the cells of a column hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cells {
    /// Dates: every cell a whole day Excel shows as a date
    Dates,
    Numbers,
    Text,
    Other,
}

/// The date a cell shows, as Forge writes it (`2026-01-31`): a cell with a
/// date format and no time of day, from 1900-03-01 on.
pub(super) fn date_of(cell: &Data) -> Option<String> {
    let Data::DateTime(value) = cell else {
        return None;
    };
    if !value.is_datetime() {
        return None;
    }
    let (year, month, day, hour, minute, second, milli) = value.to_ymd_hms_milli();
    if (hour, minute, second, milli) != (0, 0, 0, 0) {
        return None;
    }
    let text = format!("{year:04}-{month:02}-{day:02}");
    excel_date(&text).map(|_| text)
}

/// The number a cell holds; a date or a time as Excel's day number.
pub(super) fn number_of(cell: &Data) -> Option<f64> {
    match cell {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(precision::whole(*i)),
        Data::DateTime(value) => Some(value.as_f64()),
        _ => None,
    }
}

pub(super) fn cells_of(cells: &mut dyn Iterator<Item = &Data>) -> Cells {
    let mut found = None;
    for cell in cells {
        let kind = match cell {
            Data::Empty => continue,
            _ if date_of(cell).is_some() => Cells::Dates,
            Data::Float(_) | Data::Int(_) | Data::DateTime(_) => Cells::Numbers,
            Data::String(_) => Cells::Text,
            _ => Cells::Other,
        };
        found = Some(match (found, kind) {
            (None, kind) => kind,
            (Some(same), kind) if same == kind => same,
            // Dates among other numbers are numbers: their day numbers
            (Some(Cells::Dates | Cells::Numbers), Cells::Dates | Cells::Numbers) => Cells::Numbers,
            _ => Cells::Other,
        });
    }
    found.unwrap_or(Cells::Other)
}

#[cfg(test)]
mod tests {
    use super::super::ExcelImporter;
    use super::*;
    use calamine::{ExcelDateTime, ExcelDateTimeType, Reader, Xlsx};
    use rust_xlsxwriter::{Format, Workbook};
    use std::io::Cursor;

    /// The sheets of a workbook as the importer reads them
    fn read(mut book: Workbook) -> Vec<Sheet> {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::sheets(Xlsx::new(Cursor::new(bytes)).unwrap()).0
    }

    /// A sheet built by hand: `cells` from A1, and no formulas or styles
    /// A cell that holds NaN or an infinity, which only a workbook Forge
    /// exported can, becomes Excel's #NUM!; every other cell is kept
    #[test]
    fn a_number_that_is_not_finite_is_num() {
        let cells = [
            Data::Float(f64::NAN),
            Data::Float(f64::INFINITY),
            Data::Float(1.5),
            Data::String("NaN".to_string()),
        ];
        let sheet = by_hand(&[&cells]);
        let num = Data::Error(calamine::CellErrorType::Num);
        let finite = finite_cells(sheet.values);
        let read: Vec<Data> = finite.cells().map(|(_, _, cell)| cell.clone()).collect();
        assert_eq!(
            read,
            [
                num.clone(),
                num,
                Data::Float(1.5),
                Data::String("NaN".to_string())
            ]
        );
    }

    fn by_hand(cells: &[&[Data]]) -> Sheet {
        let rows = u32::try_from(cells.len()).unwrap();
        let cols = u32::try_from(cells[0].len()).unwrap();
        let mut values = Range::new((0, 0), (rows - 1, cols - 1));
        for (r, row) in cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                values.set_value(
                    (u32::try_from(r).unwrap(), u32::try_from(c).unwrap()),
                    cell.clone(),
                );
            }
        }
        Sheet::new("hand".to_string(), values, None, None)
    }

    #[test]
    fn a_column_of_one_number_format_keeps_it_and_mixed_formats_are_named() {
        let mut book = Workbook::new();
        let sheet = book.add_worksheet().set_name("t").unwrap();
        let (money, share, text) = (
            Format::new().set_num_format("#,##0.00"),
            Format::new().set_num_format("0.0%"),
            Format::new().set_num_format("@"),
        );
        sheet.write_string(0, 0, "a").unwrap();
        sheet.write_number_with_format(1, 0, 1.0, &money).unwrap();
        sheet.write_number_with_format(2, 0, 2.0, &money).unwrap();
        sheet.write_number_with_format(3, 0, 3.0, &share).unwrap();
        sheet.write_number_with_format(4, 0, 4.0, &text).unwrap();
        sheet.write_number(5, 0, 5.0).unwrap();
        let sheets = read(book);
        let t = &sheets[0];
        let mut warnings = Vec::new();
        let format = |rows: &[usize], is_date: bool, warnings: &mut Vec<String>| {
            t.imported_format(
                "t.a",
                &rows.iter().map(|r| (*r, 0)).collect::<Vec<_>>(),
                is_date,
                warnings,
            )
        };
        assert_eq!(
            format(&[1, 2], false, &mut warnings).as_deref(),
            Some("#,##0.00")
        );
        // A number format on a cell Forge reads as a date is not a date's format
        assert_eq!(format(&[1], true, &mut warnings), None);
        // No format at all
        assert_eq!(format(&[5], false, &mut warnings), None);
        assert_eq!(format(&[], false, &mut warnings), None);
        assert!(warnings.is_empty(), "{warnings:?}");
        // Two formats in one column: neither, said
        assert_eq!(format(&[1, 3], false, &mut warnings), None);
        assert!(
            warnings[0].contains("t.a: its cells have different number formats (#,##0.00, 0.0%)"),
            "{warnings:?}"
        );
        // A format for text: not imported, said
        assert_eq!(format(&[4], false, &mut warnings), None);
        assert!(
            warnings[1].contains("the number format \"@\" is not imported"),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_date_format_on_a_number_and_the_default_date_format_are_not_kept() {
        let mut book = Workbook::new();
        let sheet = book.add_worksheet().set_name("t").unwrap();
        sheet.write_string(0, 0, "d").unwrap();
        let date = Format::new().set_num_format("d mmm yyyy");
        let plain = Format::new().set_num_format("yyyy-mm-dd");
        sheet
            .write_number_with_format(1, 0, 46_023.0, &date)
            .unwrap();
        sheet
            .write_number_with_format(2, 0, 46_023.0, &plain)
            .unwrap();
        let sheets = read(book);
        let mut warnings = Vec::new();
        let t = &sheets[0];
        assert_eq!(
            t.imported_format("t.d", &[(1, 0)], false, &mut warnings),
            None
        );
        assert_eq!(
            t.imported_format("t.d", &[(1, 0)], true, &mut warnings)
                .as_deref(),
            Some("d mmm yyyy")
        );
        assert_eq!(
            t.imported_format("t.d", &[(2, 0)], true, &mut warnings),
            None
        );
    }

    #[test]
    fn a_sheet_without_styles_or_formulas_has_neither() {
        let sheet = by_hand(&[&[Data::String("a".to_string())], &[Data::Float(1.0)]]);
        assert_eq!(sheet.format_code(1, 0), None);
        assert_eq!(sheet.formula(1, 0), None);
        let mut warnings = Vec::new();
        assert_eq!(
            sheet.imported_format("x", &[(1, 0)], false, &mut warnings),
            None
        );
    }

    #[test]
    fn a_header_is_text_or_a_number_and_nothing_else() {
        let sheet = by_hand(&[&[
            Data::String("name".to_string()),
            Data::Int(2026),
            Data::Float(1.5),
            Data::Bool(true),
        ]]);
        assert_eq!(sheet.header(0).as_deref(), Some("name"));
        assert_eq!(sheet.header(1).as_deref(), Some("2026"));
        assert_eq!(sheet.header(2).as_deref(), Some("1.5"));
        assert_eq!(sheet.header(3), None);
        assert_eq!(sheet.header(9), None);
    }

    #[test]
    fn a_formula_is_written_with_its_equals_sign_once() {
        let mut sheet = by_hand(&[&[Data::Float(1.0), Data::Float(2.0), Data::Float(3.0)]]);
        let mut formulas = Range::new((0, 0), (0, 2));
        formulas.set_value((0, 0), "A2+1".to_string());
        formulas.set_value((0, 1), "=B2*2".to_string());
        formulas.set_value((0, 2), String::new());
        sheet.formulas = Some(formulas);
        assert_eq!(sheet.formula(0, 0).as_deref(), Some("=A2+1"));
        assert_eq!(sheet.formula(0, 1).as_deref(), Some("=B2*2"));
        assert_eq!(sheet.formula(0, 2), None);
    }

    #[test]
    fn a_date_is_a_whole_day_shown_as_a_date() {
        let day = |value, kind| Data::DateTime(ExcelDateTime::new(value, kind, false));
        assert_eq!(
            date_of(&day(46_023.0, ExcelDateTimeType::DateTime)).as_deref(),
            Some("2026-01-01")
        );
        // A time of day, a duration, a number: not a date
        assert_eq!(date_of(&day(46_023.5, ExcelDateTimeType::DateTime)), None);
        assert_eq!(date_of(&day(1.0, ExcelDateTimeType::TimeDelta)), None);
        assert_eq!(date_of(&Data::Float(46_023.0)), None);
        // Dates among numbers make a column of numbers; among text, of neither
        let date = day(46_023.0, ExcelDateTimeType::DateTime);
        assert_eq!(
            cells_of(&mut [date.clone(), Data::Float(1.0)].iter()),
            Cells::Numbers
        );
        assert_eq!(
            cells_of(&mut [Data::Float(1.0), date.clone()].iter()),
            Cells::Numbers
        );
        assert_eq!(
            cells_of(&mut [date.clone(), Data::Empty, date.clone()].iter()),
            Cells::Dates
        );
        assert_eq!(
            cells_of(&mut [date, Data::String("x".to_string())].iter()),
            Cells::Other
        );
        assert_eq!(
            cells_of(&mut std::iter::once(&Data::Bool(true))),
            Cells::Other
        );
        assert_eq!(cells_of(&mut std::iter::empty()), Cells::Other);
        assert_eq!(number_of(&Data::Int(3)), Some(3.0));
        assert_eq!(number_of(&Data::Bool(true)), None);
    }
}
