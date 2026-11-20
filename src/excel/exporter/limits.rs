//! What a worksheet can hold. A table or a group of scalars that does not fit
//! is refused before any of it is written, so that no cell write can fail for
//! the cell's place and the writers need not handle it.

use crate::error::{ForgeError, ForgeResult};

/// Excel's rows in a worksheet, the header's included.
pub(super) const MAX_ROWS: usize = 1_048_576;

/// Excel's columns in a worksheet.
pub(super) const MAX_COLUMNS: usize = 16_384;

/// Why a cell write cannot fail: its sheet passed [`check_size`], and the
/// writer refuses a cell only for its place or the length of its text.
pub(super) const CHECKED: &str = "the sheet's size was checked, so the cell is inside it";

/// Refuse a sheet of `rows` rows (the header's included) and `columns`
/// columns that does not fit in a worksheet of `max_rows` rows.
pub(super) fn check_size(
    sheet: &str,
    (rows, columns): (usize, usize),
    max_rows: usize,
) -> ForgeResult<()> {
    if rows > max_rows {
        return Err(ForgeError::Export(format!(
            "{sheet}: {rows} rows, the header's included, do not fit in an Excel worksheet, which holds {max_rows}"
        )));
    }
    if columns > MAX_COLUMNS {
        return Err(ForgeError::Export(format!(
            "{sheet}: {columns} columns do not fit in an Excel worksheet, which holds {MAX_COLUMNS}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_size, MAX_COLUMNS, MAX_ROWS};

    #[test]
    fn a_sheet_as_big_as_excel_allows_is_accepted_and_one_more_is_refused() {
        assert!(check_size("t", (MAX_ROWS, MAX_COLUMNS), MAX_ROWS).is_ok());
        assert_eq!(
            check_size("t", (MAX_ROWS + 1, 1), MAX_ROWS)
                .unwrap_err()
                .to_string(),
            "Excel export error: t: 1048577 rows, the header's included, do not fit in an Excel worksheet, which holds 1048576"
        );
        assert_eq!(
            check_size("t", (1, MAX_COLUMNS + 1), MAX_ROWS)
                .unwrap_err()
                .to_string(),
            "Excel export error: t: 16385 columns do not fit in an Excel worksheet, which holds 16384"
        );
    }

    #[test]
    fn the_row_limit_is_a_parameter() {
        assert!(check_size("s", (4, 2), 4).is_ok());
        assert!(check_size("s", (5, 2), 4).is_err());
    }
}
