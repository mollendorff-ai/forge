//! A sheet that is a table: its columns of values, and its columns of
//! formulas as row formulas.

use super::sheet::{cells_of, date_of, number_of, Cells, Sheet};
use super::{ExcelImporter, Names};
use crate::error::{ForgeError, ForgeResult};
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
    ) -> ForgeResult<()> {
        let (refs, names) = read;
        let names = names.of(refs, &sheet.name);
        let (height, width) = sheet.values.get_size();
        if height < 2 {
            return Ok(()); // header only
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
            let col_name = sheet.header(col).unwrap_or_else(|| format!("col_{col}"));
            // Whether the column is one of dates in Forge (`Self::names`)
            let holds_dates = names
                .date_columns
                .contains(&format!("{table_name}.{col_name}"));

            // A formula in the first data row makes this a calculated column,
            // if every row holds that same formula: a Forge row formula is one
            // formula for the whole column
            if let Some(excel) = sheet.formula(1, col) {
                let whole_column =
                    Self::row_formula(sheet, col, (refs, names), &table_name, holds_dates, &excel);
                match whole_column {
                    Ok(forge) => {
                        let place = format!("{table_name}.{col_name}");
                        let rows = (1..height).filter(|row| sheet.formula(*row, col).is_some());
                        let format = sheet.imported_format(
                            &place,
                            rows.map(|row| (row, col)),
                            holds_dates,
                            warnings,
                        );
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
            let column_value = if cells_of(&data) == Cells::Dates && !holds_dates {
                // Shown as dates, but numbers to the formulas that read them
                ColumnValue::Number(data.iter().map(|c| number_of(c).unwrap_or(0.0)).collect())
            } else {
                Self::convert_to_column_value(&data)?
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
            let format = sheet.imported_format(&place, cells, is_date, warnings);
            let mut column = Column::new(col_name, column_value);
            column.metadata.format = format;
            table.add_column(column);
        }

        model.add_table(table);
        Ok(())
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
                let holds = match cells_of(Self::column_cells(sheet, col)) {
                    Cells::Dates if holds_dates => Holds::Dates,
                    Cells::Dates | Cells::Numbers => Holds::Numbers,
                    _ => Holds::Other,
                };
                names.read_dates(&forge, ctx, holds)
            })
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
    pub(super) fn convert_to_column_value(data: &[Data]) -> ForgeResult<ColumnValue> {
        // A column of dates: every cell a whole day shown as a date (ADR-054)
        if cells_of(data) == Cells::Dates && data.iter().all(|c| !matches!(c, Data::Empty)) {
            return Ok(ColumnValue::Date(data.iter().filter_map(date_of).collect()));
        }

        // Detect column type from first non-empty cell
        let first_type = data
            .iter()
            .find(|cell| !matches!(cell, Data::Empty))
            .ok_or_else(|| ForgeError::Import("Column has no data".to_string()))?;

        match first_type {
            Data::Float(_) | Data::Int(_) | Data::DateTime(_) => {
                // Number column: dates with a time of day, times and durations
                // are Excel's day numbers
                let numbers: Vec<f64> = data
                    .iter()
                    // An empty cell, or one that holds text, is 0; a date
                    // or a time is its day number
                    .map(|cell| number_of(cell).unwrap_or(0.0))
                    .collect();
                Ok(ColumnValue::Number(numbers))
            },
            Data::String(_) => {
                // Text column
                let texts: Vec<String> =
                    data.iter().map(std::string::ToString::to_string).collect();
                Ok(ColumnValue::Text(texts))
            },
            Data::Bool(_) => {
                // Boolean column
                let bools: Vec<bool> = data
                    .iter()
                    .map(|cell| matches!(cell, Data::Bool(true)))
                    .collect();
                Ok(ColumnValue::Boolean(bools))
            },
            _ => {
                // Default to text
                let texts: Vec<String> =
                    data.iter().map(std::string::ToString::to_string).collect();
                Ok(ColumnValue::Text(texts))
            },
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
    }

    #[test]
    fn test_convert_to_column_value_numbers() {
        let data = vec![
            Data::Float(100.0),
            Data::Float(200.0),
            Data::Int(300),
            Data::Empty,
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_exact!(nums[0], 100.0);
                assert_exact!(nums[1], 200.0);
                assert_exact!(nums[2], 300.0);
                assert_exact!(nums[3], 0.0);
            },
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_text() {
        let data = vec![
            Data::String("Apple".to_string()),
            Data::String("Banana".to_string()),
            Data::Empty,
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts.len(), 3);
                assert_eq!(texts[0], "Apple");
                assert_eq!(texts[1], "Banana");
                assert_eq!(texts[2], "");
            },
            _ => panic!("Expected Text column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_boolean() {
        let data = vec![Data::Bool(true), Data::Bool(false), Data::Empty];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools.len(), 3);
                assert!(bools[0]);
                assert!(!bools[1]);
                assert!(!bools[2]);
            },
            _ => panic!("Expected Boolean column"),
        }
    }

    #[test]
    fn test_convert_to_column_value_empty() {
        let data = vec![Data::Empty, Data::Empty];
        let result = ExcelImporter::convert_to_column_value(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_column_value_mixed_numeric() {
        let data = vec![
            Data::Float(1.5),
            Data::Int(2),
            Data::Float(3.0),
            Data::Int(4),
        ];
        let result = ExcelImporter::convert_to_column_value(&data).unwrap();
        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 4);
                assert_exact!(nums[0], 1.5);
                assert_exact!(nums[1], 2.0);
                assert_exact!(nums[2], 3.0);
                assert_exact!(nums[3], 4.0);
            },
            _ => panic!("Expected Number column"),
        }
    }

    #[test]
    fn test_sanitize_table_name_spaces() {
        assert_eq!(ExcelImporter::sanitize_table_name("My Sheet"), "my_sheet");
        assert_eq!(
            ExcelImporter::sanitize_table_name("Financial Data 2024"),
            "financial_data_2024"
        );
        assert_eq!(
            ExcelImporter::sanitize_table_name("   trimmed   "),
            "___trimmed___"
        );
    }
}
