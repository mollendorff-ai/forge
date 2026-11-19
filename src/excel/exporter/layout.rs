//! What the model holds, as the workbook needs it: which columns are dates
//! or mixed, which scalars are dates, and how many rows each table has.

use super::ExcelExporter;
use crate::excel::workbook_refs::dates::{excel_serial, Holds};
use crate::types::{ColumnValue, ParsedModel, Table};

impl ExcelExporter {
    /// Whether each column of numbers or text holds text (`COUNTUNIQUE` is
    /// written differently for each).
    pub(super) fn column_types(table: &Table) -> impl Iterator<Item = (&str, bool)> {
        table
            .columns
            .iter()
            .filter_map(|(name, column)| match column.values {
                crate::types::ColumnValue::Number(_) | crate::types::ColumnValue::Blanks(_) => {
                    Some((name.as_str(), false))
                },
                // Months (`2026-01`) and other dates Excel cannot hold are
                // text in the workbook; a column of dates is marked after
                crate::types::ColumnValue::Text(_) | crate::types::ColumnValue::Date(_) => {
                    Some((name.as_str(), true))
                },
                crate::types::ColumnValue::Boolean(_) => None,
            })
    }

    /// Whether a column is written as dates: every value is a date Excel can
    /// hold (`2026-01-31`, from 1900-03-01 on). A column of months
    /// (`2026-01`), an earlier date or any other text keeps the column text.
    pub(super) fn holds_dates(values: &ColumnValue) -> bool {
        match values {
            ColumnValue::Date(v) | ColumnValue::Text(v) => {
                !v.is_empty() && v.iter().all(|text| excel_serial(text).is_some())
            },
            _ => false,
        }
    }

    /// The columns of a table written as dates (ADR-054).
    pub(super) fn date_columns(table: &Table) -> impl Iterator<Item = &str> {
        table
            .columns
            .iter()
            .filter(|(_, column)| Self::holds_dates(&column.values))
            .map(|(name, _)| name.as_str())
    }

    /// The columns of a table that stay text and have a digit in some row:
    /// `n/a` among dates, a date before 1900-03-01, a month (`2026-01`), a
    /// number written as text (`"5"`). Forge reads a number from such text
    /// its own way, and Excel its own: a formula that reads one is written
    /// as values (ADR-054). A column of words reads the same in both.
    pub(super) fn mixed_columns(table: &Table) -> impl Iterator<Item = &str> {
        table
            .columns
            .iter()
            .filter(|(_, column)| match &column.values {
                ColumnValue::Date(v) | ColumnValue::Text(v) => {
                    !Self::holds_dates(&column.values)
                        && v.iter()
                            .any(|text| text.bytes().any(|b| b.is_ascii_digit()))
                },
                _ => false,
            })
            .map(|(name, _)| name.as_str())
    }

    /// What a column holds, for a formula that gives it.
    pub(super) fn holds(
        table: &Table,
        column: &str,
        dates: &std::collections::HashSet<&str>,
    ) -> Holds {
        if dates.contains(column) {
            return Holds::Dates;
        }
        match table.columns.get(column).map(|c| &c.values) {
            Some(values) if values.is_empty() => Holds::Unknown,
            None => Holds::Unknown,
            Some(ColumnValue::Number(_) | ColumnValue::Blanks(_)) => Holds::Numbers,
            Some(_) => Holds::Other,
        }
    }

    /// The scalars whose value is a date (ADR-060).
    pub(super) fn date_scalars(model: &ParsedModel) -> impl Iterator<Item = &str> {
        model
            .scalars
            .iter()
            .filter(|(_, var)| var.is_date)
            .map(|(path, _)| path.as_str())
    }

    /// The number of values of each column.
    pub(super) fn column_lengths(table: &Table) -> impl Iterator<Item = (&str, usize)> {
        table
            .columns
            .iter()
            .map(|(name, column)| (name.as_str(), column.len()))
    }

    /// Number of data rows: the longest column (columns may differ in length).
    pub(super) fn row_count(table: &Table) -> usize {
        table
            .columns
            .values()
            .map(crate::types::Column::len)
            .max()
            .unwrap_or(0)
    }
}
