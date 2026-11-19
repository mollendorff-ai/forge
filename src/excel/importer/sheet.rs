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
}

impl Sheet {
    /// The number-format code of a cell, by 0-based position relative to the
    /// used range of values.
    pub(super) fn format_code(&self, row: usize, col: usize) -> Option<String> {
        let (r0, c0) = self.values.start()?;
        let pos = (r0 + u32::try_from(row).ok()?, c0 + u32::try_from(col).ok()?);
        let style = self.styles.as_ref()?.get(pos)?;
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
        cells: impl IntoIterator<Item = (usize, usize)>,
        is_date: bool,
        warnings: &mut Vec<String>,
    ) -> Option<String> {
        use crate::core::array_calculator::evaluator::text::number_format::{
            check_format, FormatKind,
        };
        let mut codes: Vec<String> = Vec::new();
        for (row, col) in cells {
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
                if code.eq_ignore_ascii_case("general") || (is_date && code == "yyyy-mm-dd") {
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
                let general = several.iter().all(|c| c.eq_ignore_ascii_case("general"));
                if !general {
                    warnings.push(format!(
                        "{place}: its cells have different number formats ({}), so none is \
                         imported",
                        several.join(", ")
                    ));
                }
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
        name && value && third
    }

    /// Formula of the cell at a 0-based position relative to the value range.
    pub(super) fn formula(&self, row: usize, col: usize) -> Option<String> {
        let (start_row, start_col) = self.values.start()?;
        let r = start_row + u32::try_from(row).ok()?;
        let c = start_col + u32::try_from(col).ok()?;
        let f = self.formulas.as_ref()?.get_value((r, c))?;
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
        let start = self.values.start().map_or(0, |(r, _)| r);
        start + u32::try_from(row).unwrap_or(0) + 1
    }

    /// Excel column letter of a 0-based column in the value range.
    pub(super) fn letter(&self, col: usize) -> String {
        let start = self.values.start().map_or(0, |(_, c)| c as usize);
        column_letter(start + col)
    }
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

pub(super) fn cells_of<'d>(cells: impl IntoIterator<Item = &'d Data>) -> Cells {
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
