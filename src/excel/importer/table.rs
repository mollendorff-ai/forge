//! A sheet that is a table: its columns of values, and its columns of
//! formulas as row formulas.

use super::sheet::{cells_of, date_of, number_of, Cells, Sheet};
use super::{ExcelImporter, Names};
use crate::excel::workbook_refs::dates::Holds;
use crate::excel::workbook_refs::{Context, ReverseRefs, SheetKind, WorkbookRefs};
use crate::types::{Column, ColumnValue, ParsedModel, Table};
use calamine::Data;

impl ExcelImporter {
    /// Any other sheet: a table, header in the first row.
    pub(super) fn import_table(
        sheet: &Sheet,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let (refs, names_of_all) = read;
        let names = names_of_all.of(refs, &sheet.name);
        let (height, width) = sheet.values.get_size();
        if height < 2 {
            return; // header only
        }
        let table_name = match refs.sheets.get(&sheet.name) {
            Some(SheetKind::Table { name, .. }) => name.clone(),
            _ => Self::sanitize_table_name(&sheet.name),
        };
        let mut table = Table::new(table_name.clone());
        // The rows the sheet's formulas run down to: a column of numbers is as
        // long as that, its last cells empty (ADR-059). A sheet without
        // formulas keeps its shorter columns short.
        let formula_rows = Self::formula_rows(sheet, height, width);

        for col in 0..width {
            let col_name = sheet.column_name(col);
            // Whether the column is one of dates in Forge (`Self::names`)
            let holds_dates = names
                .date_columns
                .contains(&format!("{table_name}.{col_name}"));

            // A formula in the first data row makes this a calculated column,
            // if every row holds that same formula: a Forge row formula is one
            // formula for the whole column. A formula in a later row only, or
            // rows that differ: one formula that picks by row, with a warning
            if let Some(excel) = (1..height).find_map(|row| sheet.formula(row, col)) {
                let column = (col, table_name.as_str(), holds_dates);
                let whole_column =
                    Self::formula_column(sheet, column, (refs, names), &excel, warnings);
                match whole_column {
                    Ok(forge) => {
                        let place = format!("{table_name}.{col_name}");
                        let cells: Vec<(usize, usize)> = (1..height)
                            .filter(|row| sheet.formula(*row, col).is_some())
                            .map(|row| (row, col))
                            .collect();
                        let format = sheet.imported_format(&place, &cells, holds_dates, warnings);
                        if let Some(format) = format {
                            table.formula_metadata.insert(
                                col_name.clone(),
                                crate::types::Metadata {
                                    format: Some(format),
                                    ..Default::default()
                                },
                            );
                        }
                        table.add_row_formula(col_name, forge);
                        continue;
                    },
                    Err(reason) => warnings.push(format!(
                        "{table_name}.{col_name}: imported as values ({reason}): {excel}"
                    )),
                }
            }

            let mut data: Vec<Data> = (1..height)
                .map(|row| sheet.values.get((row, col)).cloned().unwrap_or(Data::Empty))
                .collect();
            // Columns may differ in length: a shorter one ends in empty cells
            while matches!(data.last(), Some(Data::Empty)) {
                data.pop();
            }
            Self::pad_numbers(&mut data, formula_rows);
            if data.is_empty() {
                // A column with a name and no value: a column of empty cells
                // as long as the sheet's data (ADR-059)
                if sheet.header(col).is_some() && height > 1 {
                    table.add_column(Column::new(
                        col_name,
                        ColumnValue::Blanks(vec![None; height - 1]),
                    ));
                }
                continue;
            }
            let column_value = if cells_of(&mut data.iter()) == Cells::Dates && !holds_dates {
                // Shown as dates, but numbers to the formulas that read them
                ColumnValue::Number(data.iter().map(|c| number_of(c).unwrap_or(0.0)).collect())
            } else {
                Self::convert_to_column_value(&data)
            };
            // An empty cell among numbers stays empty (ADR-059)
            let column_value = Self::with_empty_cells(column_value, &data);
            if matches!(
                column_value,
                ColumnValue::Number(_) | ColumnValue::Blanks(_)
            ) {
                Self::warn_not_numbers(sheet, col, (&table_name, &col_name), &data, warnings);
            }
            // Forge reads a column that starts with a date as a column of
            // dates, and rejects any other text in it
            if let ColumnValue::Text(texts) = &column_value {
                let dated = |t: &String| crate::parser::is_valid_date_format(t);
                if texts.first().is_some_and(dated) && !texts.iter().all(dated) {
                    warnings.push(format!(
                        "{table_name}.{col_name}: starts with a date and holds other text; Forge \
                         reads a column that starts with a date as dates, so `forge calculate` \
                         rejects this file until the column is edited"
                    ));
                }
            }
            // The number format its cells share (ADR-061)
            let is_date = matches!(column_value, ColumnValue::Date(_));
            let cells = (0..data.len())
                .filter(|i| !matches!(data[*i], Data::Empty))
                .map(|i| (i + 1, col));
            let place = format!("{table_name}.{col_name}");
            let format =
                sheet.imported_format(&place, &cells.collect::<Vec<_>>(), is_date, warnings);
            let mut column = Column::new(col_name, column_value);
            column.metadata.format = format;
            table.add_column(column);
        }

        model.add_table(table);
        Self::import_total(sheet, (refs, names_of_all), model, warnings);
    }

    /// The one Forge formula of column `col` (with its table and whether it
    /// holds dates), whose rows hold `excel` and maybe other formulas: the
    /// row formula every row holds, or one that picks by row (`by_rows`,
    /// which adds its warning); the reason when there is none.
    fn formula_column(
        sheet: &Sheet,
        (col, table_name, holds_dates): (usize, &str, bool),
        read: (&ReverseRefs, &WorkbookRefs),
        excel: &str,
        warnings: &mut Vec<String>,
    ) -> Result<String, String> {
        let whole_column = match sheet.formula(1, col) {
            Some(_) => Self::row_formula(sheet, col, read, table_name, holds_dates, excel),
            None => Err(format!("row {} holds no formula", sheet.excel_row(1))),
        };
        whole_column.or_else(|reason| {
            Self::by_rows(sheet, col, read, table_name, holds_dates)
                .map(|(forge, warning)| {
                    warnings.push(warning);
                    forge
                })
                .map_err(|_| reason)
        })
    }

    /// The Forge row formula of a column whose first data row holds the
    /// formula `excel`: the one formula every row holds, or, for a column
    /// whose first row differs from the rest (a running balance), one that
    /// reads the row above.
    pub(super) fn row_formula(
        sheet: &Sheet,
        col: usize,
        read: (&ReverseRefs, &WorkbookRefs),
        table_name: &str,
        holds_dates: bool,
        excel: &str,
    ) -> Result<String, String> {
        let (refs, names) = read;
        let (height, _) = sheet.values.get_size();
        refs
            .to_forge(excel, &sheet.name, sheet.excel_row(1))
            .and_then(|forge| {
                let other = (2..height).find(|&row| {
                    sheet.formula(row, col).map(|excel| {
                        refs.to_forge(&excel, &sheet.name, sheet.excel_row(row))
                    }) != Some(Ok(forge.clone()))
                });
                if other.is_none() {
                    return Ok(forge);
                }
                // The first rows differ from the rest, as in a running
                // balance: a PREV, whose first value is in those rows
                let rows: Vec<String> = (1..height)
                    .map(|row| {
                        sheet
                            .formula(row, col)
                            .ok_or_else(|| {
                                format!("row {} holds no formula", sheet.excel_row(row))
                            })
                            .and_then(|excel| {
                                refs.to_forge_reading_above(
                                    &excel,
                                    &sheet.name,
                                    sheet.excel_row(row),
                                )
                            })
                    })
                    .collect::<Result<_, _>>()?;
                match crate::excel::previous_rows::column_formula(&rows) {
                    Ok(formula) => Ok(formula),
                    // Only the first row differs: correct, if not as written
                    Err(_) if rows[1..].iter().all(|r| *r == rows[1]) => Ok(format!(
                        "=IF(ROW() = 1, {}, {})",
                        forge.trim_start_matches('='),
                        rows[1].trim_start_matches('=')
                    )),
                    Err(index) => Err(format!(
                        "row {} holds another formula than the rows above it, and a \
                         Forge row formula is one formula for the whole column",
                        sheet.excel_row(index + 1)
                    )),
                }
            })
            // A date is read as Forge reads it, or the formula is not
            // the same calculation in Forge (ADR-054)
            .and_then(|forge| {
                let ctx = Context::Row {
                    table: table_name,
                    row: 2,
                };
                names.read_dates(&forge, ctx, Self::holds(sheet, col, holds_dates))
            })
    }

    /// What the cells of column `col` hold, as a formula over them reads
    /// them: dates only in a column of dates in Forge (`Self::names`).
    pub(super) fn holds(sheet: &Sheet, col: usize, holds_dates: bool) -> Holds {
        match cells_of(&mut Self::column_cells(sheet, col)) {
            Cells::Dates if holds_dates => Holds::Dates,
            Cells::Dates | Cells::Numbers => Holds::Numbers,
            _ => Holds::Other,
        }
    }

    /// The last data row of `sheet` that holds a formula (0 if none does)
    pub(super) fn formula_rows(sheet: &Sheet, height: usize, width: usize) -> usize {
        (1..height)
            .rev()
            .find(|&row| (0..width).any(|col| sheet.formula(row, col).is_some()))
            .unwrap_or(0)
    }

    /// A column of numbers beside formulas that run further down: its last
    /// cells are empty cells, as a month not closed yet (ADR-059)
    pub(super) fn pad_numbers(data: &mut Vec<Data>, formula_rows: usize) {
        let numbers = data.iter().any(|c| number_of(c).is_some())
            && data
                .iter()
                .all(|c| matches!(c, Data::Empty) || number_of(c).is_some());
        if numbers && data.len() < formula_rows {
            data.resize(formula_rows, Data::Empty);
        }
    }

    /// A column of numbers whose cells include an empty one, as a column with
    /// empty cells (ADR-059); any other column as it is.
    pub(super) fn with_empty_cells(values: ColumnValue, data: &[Data]) -> ColumnValue {
        match values {
            ColumnValue::Number(numbers) if data.iter().any(|c| matches!(c, Data::Empty)) => {
                ColumnValue::Blanks(
                    numbers
                        .into_iter()
                        .zip(data)
                        .map(|(n, cell)| (!matches!(cell, Data::Empty)).then_some(n))
                        .collect(),
                )
            },
            other => other,
        }
    }

    /// Convert Excel Data array to `ColumnValue`
    pub(super) fn convert_to_column_value(data: &[Data]) -> ColumnValue {
        // A column of dates: every cell a whole day shown as a date (ADR-054)
        if cells_of(&mut data.iter()) == Cells::Dates
            && data.iter().all(|c| !matches!(c, Data::Empty))
        {
            return ColumnValue::Date(data.iter().filter_map(date_of).collect());
        }
        // The column's type is that of its first cell with something in it
        match data.iter().find(|cell| !matches!(cell, Data::Empty)) {
            // Number column: dates with a time of day, times and durations are
            // Excel's day numbers; an empty cell, or one that holds text, is 0
            Some(Data::Float(_) | Data::Int(_) | Data::DateTime(_)) => ColumnValue::Number(
                data.iter()
                    .map(|cell| number_of(cell).unwrap_or(0.0))
                    .collect(),
            ),
            Some(Data::Bool(_)) => ColumnValue::Boolean(
                data.iter()
                    .map(|cell| matches!(cell, Data::Bool(true)))
                    .collect(),
            ),
            // Nothing at all: a column of empty cells (ADR-059)
            None => ColumnValue::Blanks(vec![None; data.len()]),
            // Text, or anything else (an error), as text
            Some(_) => ColumnValue::Text(data.iter().map(ToString::to_string).collect()),
        }
    }

    /// Sanitize sheet name to valid YAML key
    pub(super) fn sanitize_table_name(sheet_name: &str) -> String {
        sheet_name
            .to_lowercase()
            .replace(' ', "_")
            .replace('&', "and")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{ExcelDateTime, ExcelDateTimeType};
    use rust_xlsxwriter::{Format, Formula, Workbook, Worksheet};

    /// The model and the warnings an import of `book` gives
    fn import(mut book: Workbook) -> (ParsedModel, Vec<String>) {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::from_bytes(bytes)
            .import_with_warnings()
            .unwrap()
    }

    fn sheet<'a>(book: &'a mut Workbook, name: &str) -> &'a mut Worksheet {
        book.add_worksheet().set_name(name).unwrap()
    }

    fn values(model: &ParsedModel, table: &str, column: &str) -> ColumnValue {
        model.tables[table].columns[column].values.clone()
    }

    #[test]
    fn test_sanitize_table_name() {
        assert_eq!(ExcelImporter::sanitize_table_name("Sheet1"), "sheet1");
        assert_eq!(
            ExcelImporter::sanitize_table_name("P&L Statement"),
            "pandl_statement"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("Revenue-2025"),
            "revenue_2025"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("Special@#$Chars"),
            "specialchars"
        );
        assert_eq!(ExcelImporter::sanitize_table_name("My Sheet"), "my_sheet");
        assert_eq!(
            ExcelImporter::sanitize_table_name("   trimmed   "),
            "___trimmed___"
        );
    }

    #[test]
    fn a_column_takes_the_type_of_its_first_cell() {
        let convert = ExcelImporter::convert_to_column_value;
        assert_eq!(
            convert(&[
                Data::Float(100.0),
                Data::Int(300),
                Data::Empty,
                Data::String("x".into())
            ]),
            ColumnValue::Number(vec![100.0, 300.0, 0.0, 0.0])
        );
        assert_eq!(
            convert(&[Data::String("Apple".into()), Data::Empty]),
            ColumnValue::Text(vec!["Apple".to_string(), String::new()])
        );
        assert_eq!(
            convert(&[Data::Bool(true), Data::Bool(false), Data::Empty]),
            ColumnValue::Boolean(vec![true, false, false])
        );
        // An error cell first: the column is text
        assert_eq!(
            convert(&[Data::Error(calamine::CellErrorType::Div0), Data::Float(1.0)]),
            ColumnValue::Text(vec!["#DIV/0!".to_string(), "1".to_string()])
        );
        assert_eq!(
            convert(&[Data::Empty, Data::Empty]),
            ColumnValue::Blanks(vec![None, None])
        );
        let day = Data::DateTime(ExcelDateTime::new(
            46_023.0,
            ExcelDateTimeType::DateTime,
            false,
        ));
        assert_eq!(
            convert(&[day.clone(), day.clone()]),
            ColumnValue::Date(vec!["2026-01-01".to_string(), "2026-01-01".to_string()])
        );
        // A date among empty cells is its day number
        assert_eq!(
            convert(&[day, Data::Empty]),
            ColumnValue::Number(vec![46_023.0, 0.0])
        );
    }

    #[test]
    fn a_sheet_with_only_a_header_has_no_table() {
        let mut book = Workbook::new();
        sheet(&mut book, "h").write_string(0, 0, "a").unwrap();
        let (model, _) = import(book);
        assert!(!model.tables.contains_key("h"));
    }

    #[test]
    fn a_sheet_the_layout_does_not_know_is_named_from_the_sheet() {
        let mut cells = calamine::Range::new((0, 0), (1, 0));
        cells.set_value((0, 0), Data::String("n".to_string()));
        cells.set_value((1, 0), Data::Float(4.0));
        let sheet = Sheet::new("My Sheet".to_string(), cells, None, None);
        let (refs, names) = (ReverseRefs::default(), Names::default());
        let mut model = ParsedModel::new();
        ExcelImporter::import_table(&sheet, (&refs, &names), &mut model, &mut Vec::new());
        assert_eq!(
            values(&model, "my_sheet", "n"),
            ColumnValue::Number(vec![4.0])
        );
    }

    #[test]
    fn a_column_without_a_header_is_named_by_its_place_and_an_empty_one_is_left_out() {
        let mut book = Workbook::new();
        let t = sheet(&mut book, "t");
        t.write_string(0, 0, "a").unwrap();
        t.write_number(1, 0, 1.0).unwrap();
        t.write_string(0, 2, "c").unwrap();
        t.write_number(1, 2, 2.0).unwrap();
        t.write_number(1, 3, 5.0).unwrap();
        let (model, _) = import(book);
        let names: Vec<&String> = model.tables["t"].columns.keys().collect();
        assert_eq!(names, ["a", "c", "col_3"]);
        assert_eq!(values(&model, "t", "col_3"), ColumnValue::Number(vec![5.0]));
    }

    #[test]
    fn text_that_starts_with_a_date_and_goes_on_otherwise_is_warned_about() {
        let mut book = Workbook::new();
        let t = sheet(&mut book, "t");
        t.write_string(0, 0, "when").unwrap();
        t.write_string(1, 0, "2026-01-01").unwrap();
        t.write_string(2, 0, "later").unwrap();
        let (_, warnings) = import(book);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("t.when: starts with a date and holds other text"));
    }

    #[test]
    fn numbers_shown_as_dates_by_formulas_that_differ_come_in_as_day_numbers() {
        let mut book = Workbook::new();
        let date = Format::new().set_num_format("yyyy-mm-dd");
        let t = sheet(&mut book, "t");
        t.write_string(0, 0, "start").unwrap();
        t.write_string(0, 1, "due").unwrap();
        for (i, add) in [30, 31, 32].into_iter().enumerate() {
            let row = u32::try_from(i).unwrap() + 1;
            t.write_number_with_format(row, 0, 46_023.0, &date).unwrap();
            let formula =
                Formula::new(format!("=A{}+{add}", row + 1)).set_result((46_023 + add).to_string());
            t.write_formula_with_format(row, 1, formula, &date).unwrap();
        }
        let (model, warnings) = import(book);
        assert!(
            warnings[0].contains("t.due: imported as values (row 4 holds another formula"),
            "{warnings:?}"
        );
        // A day plus days is a day number in Forge, so not a column of dates
        assert_eq!(
            values(&model, "t", "due"),
            ColumnValue::Number(vec![46_053.0, 46_054.0, 46_055.0])
        );
    }

    #[test]
    fn a_column_whose_later_rows_have_no_formula_is_imported_as_values() {
        let mut book = Workbook::new();
        let t = sheet(&mut book, "t");
        t.write_string(0, 0, "x").unwrap();
        t.write_formula(1, 0, Formula::new("=1+1").set_result("2"))
            .unwrap();
        t.write_formula(2, 0, Formula::new("=2+2").set_result("4"))
            .unwrap();
        t.write_number(3, 0, 5.0).unwrap();
        let (model, warnings) = import(book);
        assert!(
            warnings[0].contains("row 4 holds no formula"),
            "{warnings:?}"
        );
        assert_eq!(
            values(&model, "t", "x"),
            ColumnValue::Number(vec![2.0, 4.0, 5.0])
        );
    }

    #[test]
    fn numbers_beside_longer_formulas_end_in_empty_cells() {
        let mut book = Workbook::new();
        let t = sheet(&mut book, "t");
        t.write_string(0, 0, "actual").unwrap();
        t.write_string(0, 1, "plan").unwrap();
        t.write_number(1, 0, 1.0).unwrap();
        t.write_number(2, 0, 2.0).unwrap();
        for row in 1..=4 {
            t.write_formula(row, 1, Formula::new("=10").set_result("10"))
                .unwrap();
        }
        let (model, warnings) = import(book);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            values(&model, "t", "actual"),
            ColumnValue::Blanks(vec![Some(1.0), Some(2.0), None, None])
        );
    }

    #[test]
    fn non_numbers_in_a_column_of_numbers_are_named_one_or_many() {
        let mut book = Workbook::new();
        let t = sheet(&mut book, "one");
        t.write_string(0, 0, "n").unwrap();
        t.write_number(1, 0, 1.0).unwrap();
        t.write_string(2, 0, "x").unwrap();
        let t = sheet(&mut book, "many");
        t.write_string(0, 0, "n").unwrap();
        t.write_number(1, 0, 1.0).unwrap();
        for row in 2..=8 {
            t.write_string(row, 0, "x").unwrap();
        }
        let (_, warnings) = import(book);
        assert!(
            warnings[0].contains("one.n: a column of numbers holds text"),
            "{warnings:?}"
        );
        assert!(
            warnings[0].contains("in 1 cell, imported as 0: A3 (x)"),
            "{warnings:?}"
        );
        assert!(warnings[1].contains("in several cells,"), "{warnings:?}");
        assert!(warnings[1].ends_with("A7 (x) and 2 more"), "{warnings:?}");
    }
}
