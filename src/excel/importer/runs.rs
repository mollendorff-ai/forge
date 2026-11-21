//! A column whose rows do not all hold one formula: two formulas, one from
//! July on, or a number typed in over the formula in one row. A Forge row
//! formula is one formula for the whole column, so it is imported as the one
//! formula that picks by row, `IF(ROW() <= 6, a, b)` or `IF(ROW() = 10,
//! 21000, a)`, which calculates what every row held and keeps a typed-in
//! number in sight, with a warning that names the rows.

use super::sheet::Sheet;
use super::ExcelImporter;
use crate::excel::workbook_refs::{Context, ReverseRefs, WorkbookRefs};
use calamine::Data;

/// Rows that hold the same formula or number, one after another: the first
/// and last table row (from 1), and what they hold, as Forge reads it.
type Run = (usize, usize, String);

/// The most typed-in numbers or other formulas kept in one formula.
const MOST_EXCEPTIONS: usize = 3;
/// The most runs of rows kept in one formula when they are longer.
const MOST_RUNS: usize = 3;

impl ExcelImporter {
    /// The one formula for a column whose rows hold formulas and numbers
    /// that differ, and the warning that says so; the reason when there is
    /// none (a row reads another row, holds text, or the rows differ too much).
    pub(super) fn by_rows(
        sheet: &Sheet,
        col: usize,
        read: (&ReverseRefs, &WorkbookRefs),
        table_name: &str,
        holds_dates: bool,
    ) -> Result<(String, String), String> {
        let (refs, names) = read;
        let (height, _) = sheet.values.get_size();
        let mut runs: Vec<Run> = Vec::new();
        for row in 1..height {
            let piece = match (sheet.formula(row, col), sheet.values.get((row, col))) {
                (Some(excel), _) => refs.to_forge(&excel, &sheet.name, sheet.excel_row(row))?,
                (None, Some(Data::Float(n))) => format!("={n}"),
                _ => {
                    return Err(format!(
                        "row {} holds neither a formula nor a number",
                        sheet.excel_row(row)
                    ))
                },
            };
            let piece = piece.trim_start_matches('=').to_string();
            match runs.last_mut() {
                Some(run) if run.2 == piece => run.1 = row,
                _ => runs.push((row, row, piece)),
            }
        }
        let (formula, rows) = one_formula(&runs, |row| sheet.excel_row(row))?;
        let holds = Self::holds(sheet, col, holds_dates);
        let context = Context::Row {
            table: table_name,
            row: 2,
        };
        let formula = names.read_dates(&formula, context, holds)?;
        let warning = format!(
            "{table_name}.{}: its rows hold different formulas or numbers typed in ({rows}), \
             so it is imported as one formula that picks by row: {formula}",
            sheet.column_name(col)
        );
        Ok((formula, warning))
    }
}

/// The `IF(ROW() ...)` formula that gives each run's value in its rows, and
/// the rows that differ in words. When more than half the rows hold one
/// formula and a few single rows something else (a number typed over the
/// formula), those rows are named one by one; otherwise each run, of two
/// rows at least, but the last ends at a row. Anything else is not one
/// formula: a column of numbers with a formula here and there, or a
/// formula that changes from row to row.
fn one_formula(runs: &[Run], excel_row: impl Fn(usize) -> u32) -> Result<(String, String), String> {
    let rows_of = |piece: &str| -> usize {
        runs.iter()
            .filter(|run| run.2 == piece)
            .map(|(first, last, _)| last - first + 1)
            .sum()
    };
    let common = runs
        .iter()
        .max_by_key(|run| rows_of(&run.2))
        .map(|run| run.2.clone())
        .unwrap_or_default();
    let odd: Vec<&Run> = runs.iter().filter(|run| run.2 != common).collect();
    let all_rows = runs.last().map_or(0, |run| run.1);
    let a_formula = common.parse::<f64>().is_err() && rows_of(&common) * 2 > all_rows;
    if a_formula && odd.len() <= MOST_EXCEPTIONS && odd.iter().all(|(first, last, _)| first == last)
    {
        let formula = odd
            .iter()
            .rev()
            .fold(common.clone(), |rest, (row, _, piece)| {
                format!("IF(ROW() = {row}, {piece}, {rest})")
            });
        let rows = odd
            .iter()
            .map(|(row, _, piece)| format!("row {} holds {piece}", excel_row(*row)))
            .collect::<Vec<_>>()
            .join(", ");
        return Ok((
            format!("={formula}"),
            format!("{rows}, the others {common}"),
        ));
    }
    if runs.len() > MOST_RUNS || runs.iter().any(|(first, last, _)| first == last) {
        return Err(format!(
            "its rows hold {} runs of different formulas or numbers, and a Forge row \
             formula is one formula for the whole column",
            runs.len()
        ));
    }
    let (last, before) = runs.split_last().unwrap_or((&runs[0], &[]));
    let formula = before
        .iter()
        .rev()
        .fold(last.2.clone(), |rest, (_, end, piece)| {
            format!("IF(ROW() <= {end}, {piece}, {rest})")
        });
    let rows = runs
        .iter()
        .map(|(first, last, piece)| {
            format!(
                "rows {} to {}: {piece}",
                excel_row(*first),
                excel_row(*last)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Ok((format!("={formula}"), rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(first: usize, last: usize, piece: &str) -> Run {
        (first, last, piece.to_string())
    }

    /// Excel rows are table rows plus 3, as under a header in row 3.
    fn formula(runs: &[Run]) -> Result<(String, String), String> {
        one_formula(runs, |row| u32::try_from(row).unwrap() + 3)
    }

    #[test]
    fn a_number_typed_over_the_formula_is_kept_in_its_row() {
        let (f, rows) = formula(&[
            run(1, 9, "a.staff"),
            run(10, 10, "21000"),
            run(11, 12, "a.staff"),
        ])
        .unwrap();
        assert_eq!(f, "=IF(ROW() = 10, 21000, a.staff)");
        assert_eq!(rows, "row 13 holds 21000, the others a.staff");
        let (f, _) = formula(&[run(1, 1, "5"), run(2, 6, "x"), run(7, 7, "6")]).unwrap();
        assert_eq!(f, "=IF(ROW() = 1, 5, IF(ROW() = 7, 6, x))");
    }

    #[test]
    fn runs_of_rows_end_at_a_row() {
        let (f, rows) = formula(&[run(1, 6, "p"), run(7, 12, "p * 1.04")]).unwrap();
        assert_eq!(f, "=IF(ROW() <= 6, p, p * 1.04)");
        assert_eq!(rows, "rows 4 to 9: p; rows 10 to 15: p * 1.04");
        let (f, _) = formula(&[run(1, 4, "a"), run(5, 8, "b"), run(9, 12, "c")]).unwrap();
        assert_eq!(f, "=IF(ROW() <= 4, a, IF(ROW() <= 8, b, c))");
    }

    #[test]
    fn too_many_runs_are_not_one_formula() {
        let runs = [
            run(1, 2, "a"),
            run(3, 4, "b"),
            run(5, 6, "a"),
            run(7, 8, "b"),
        ];
        assert!(formula(&runs).unwrap_err().contains("4 runs"));
        let typed: Vec<Run> = (1..=9)
            .map(|row| run(row, row, if row % 2 == 0 { "x" } else { "y" }))
            .collect();
        assert!(formula(&typed).is_err());
        // Numbers with a formula here and there, and a formula row by row
        let numbers = [run(1, 1, "1 + 1"), run(2, 2, "2 + 2"), run(3, 6, "5")];
        assert!(formula(&numbers).is_err());
        let each = [
            run(1, 1, "s + 30"),
            run(2, 2, "s + 31"),
            run(3, 3, "s + 32"),
        ];
        assert!(formula(&each).unwrap_err().contains("3 runs"));
    }

    fn import(mut book: rust_xlsxwriter::Workbook) -> (crate::types::ParsedModel, Vec<String>) {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::from_bytes(bytes)
            .import_with_warnings()
            .unwrap()
    }

    /// A number typed into the first row of a column of formulas: one
    /// formula that keeps it. With text there instead, the column's values.
    #[test]
    fn a_first_row_typed_in_and_a_row_of_text() {
        use rust_xlsxwriter::{Formula, Workbook};
        let mut book = Workbook::new();
        let t = book.add_worksheet().set_name("t").unwrap();
        t.write_string(0, 0, "a").unwrap();
        t.write_string(0, 1, "b").unwrap();
        t.write_string(0, 2, "c").unwrap();
        for row in 1..6 {
            t.write_number(row, 0, f64::from(row)).unwrap();
            if row == 1 {
                t.write_number(row, 1, 5.0).unwrap();
                t.write_string(row, 2, "n/a").unwrap();
            } else {
                let twice = format!("=A{}*2", row + 1);
                let result = format!("{}", f64::from(row) * 2.0);
                t.write_formula(row, 1, Formula::new(&twice).set_result(&result))
                    .unwrap();
                t.write_formula(row, 2, Formula::new(&twice).set_result(&result))
                    .unwrap();
            }
        }
        let (model, warnings) = import(book);
        let table = &model.tables["t"];
        assert_eq!(table.row_formulas["b"], "=IF(ROW() = 1, 5, a * 2)");
        assert!(
            warnings[0].contains("t.b: its rows hold different formulas"),
            "{warnings:?}"
        );
        assert!(
            warnings[0].contains("row 2 holds 5, the others a * 2"),
            "{warnings:?}"
        );
        assert!(!table.row_formulas.contains_key("c"));
        assert!(
            warnings[1].starts_with("t.c: imported as values (row 2 holds no formula)"),
            "{warnings:?}"
        );
    }
}
